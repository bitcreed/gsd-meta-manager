// ============================================================================
// The editor-race port: a "human" saves DIRECTION.md while the tool regenerates
// its frontmatter in a tight loop (D-14, D-20's corrected pass rule).
//
// Port of `.planning/spikes/002-launcher-caps-and-tool-frontmatter/experiment/
// race_test.py`. The human saves N times, each save carrying a counter in the
// body. After every save it reads the file back: if, 50 ms later, the body still
// shows an older counter, the tool silently reverted that save — a LOST EDIT.
// The tool thread splices far more often than a real run ever would (a real run
// emits events every few seconds and coalesces heartbeats to 2 min).
//
// PASS RULE. Two save styles, two different guarantees:
//
// * RENAME-SAVE (vim, emacs, most editors: write a temp, rename it over the
//   file). `lost_rename == 0`, exactly. The exchange-verify gives the displaced
//   inode back, so no rename-save can be lost.
// * IN-PLACE (VS Code, nano, `echo >`: open with O_TRUNC, then write). The spike
//   measured 7.4e-5 lost saves per save. The literal assertion `rate <= 7.4e-5`
//   would be FLAKY BY CONSTRUCTION: over ~13k saves the expected count is about
//   one, so a perfectly healthy run exceeds the rate whenever it sees a single
//   loss in a short window. The corrected rule bounds the COUNT at the 99th
//   percentile of Poisson(7.4e-5 x saves): `lost_inplace <= poisson_q99(..)`.
//   A genuine regression to a common loss still fails it; a lone fluctuation
//   does not.
//
// NON-VACUITY. A passing race test proves nothing if the race never happens.
// `editor_race_naive_positive_control` runs the obvious read-once-then-rename
// strategy — kept HERE, test-only, so the library carries no naive strategy —
// and requires LOST > 0.
//
// `flock` is per open file description, so the two writers in
// `two_writers_under_direction_lock_lose_nothing` contend like two processes.
// That test hands the lock over with a handshake in both directions, not a
// fixed gap. The lock's retry is a poll with backoff, so it has no queue and no
// fairness (D-13/D-20), and a writer that re-takes the lock 100 µs after
// releasing it starves a polling one whenever the hold dwarfs the gap. The hold
// is mostly the temp file's fsync: ~60 µs on tmpfs, ~3 ms (up to 27 ms) on
// overlay in the pre-tag container, where the fixed gap timed out about 1 run
// in 10 (quick 261023-qs8).
//
// Linux-only: the splice is `renameat2(RENAME_EXCHANGE)`.
// ============================================================================

#![cfg(target_os = "linux")]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use gsd_meta_manager::driver::direction::{
    body_sane, splice_frontmatter, split_frontmatter, write_body, SpliceOutcome, DIRECTION_FILE,
    DIRECTION_LOCK_BUDGET, FM_MARKER, TEMP_PREFIX,
};
use regex::Regex;
use tempfile::TempDir;

/// The spike's body, with the human's counter in the `note` line.
fn body(k: u64) -> String {
    format!(
        "# Driver direction: race\n\n## Direction\n\n- control: run\n- note: human edit {k}\n\n## Items\n\n### I-01 x\n\n- budget: dispatches=1\n"
    )
}

/// The spike's measured in-place loss rate per save.
const SPIKE_INPLACE_LOSS_RATE: f64 = 7.4e-5;

/// Smallest `k` with `P(X <= k) >= 0.99` for `X ~ Poisson(lambda)`.
fn poisson_q99(lambda: f64) -> u64 {
    if lambda <= 0.0 {
        return 0;
    }
    if lambda > 500.0 {
        // `e^-lambda` underflows past ~745; the normal approximation is exact
        // enough this far out (z_0.99 = 2.326).
        return (lambda + 2.326 * lambda.sqrt()).ceil() as u64;
    }
    let mut pmf = (-lambda).exp();
    let mut cdf = pmf;
    let mut k = 0u64;
    while cdf < 0.99 {
        k += 1;
        pmf *= lambda / k as f64;
        cdf += pmf;
    }
    k
}

#[test]
fn poisson_q99_matches_known_values() {
    assert_eq!(
        poisson_q99(0.0),
        0,
        "no expected events, no tolerated events"
    );
    assert_eq!(poisson_q99(1.0), 4, "P(X<=3)=0.981 < 0.99 <= P(X<=4)=0.996");
    assert_eq!(
        poisson_q99(0.074),
        1,
        "P(X<=0)=0.929 < 0.99 <= P(X<=1)=0.997"
    );
    assert!(poisson_q99(1000.0) >= 1000, "the tail is above the mean");
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Strategy {
    Exchange,
    Naive,
}

impl Strategy {
    fn name(self) -> &'static str {
        match self {
            Strategy::Exchange => "exchange",
            Strategy::Naive => "naive",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Style {
    Rename,
    Inplace,
}

impl Style {
    fn name(self) -> &'static str {
        match self {
            Style::Rename => "rename",
            Style::Inplace => "inplace",
        }
    }
}

/// A tiny xorshift PRNG, seeded from the case name — no new crate.
struct XorShift(u64);

impl XorShift {
    fn seeded(name: &str) -> Self {
        // FNV-1a over the name; never zero.
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        for b in name.bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        XorShift(h | 1)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform 0..2 ms.
    fn short_sleep(&mut self) -> Duration {
        Duration::from_micros(self.next() % 2_000)
    }
}

/// The latest `human edit N` counter the file shows, if any.
fn latest(path: &Path, re: &Regex) -> Option<u64> {
    let text = fs::read_to_string(path).ok()?;
    re.captures(&text)?.get(1)?.as_str().parse().ok()
}

struct HumanResult {
    saves: u64,
    lost: u64,
    transient: u64,
}

fn human(path: PathBuf, style: Style, deadline: Instant, seed: &str) -> HumanResult {
    let re = Regex::new(r"human edit (\d+)").expect("counter regex");
    let mut rng = XorShift::seeded(seed);
    let (mut k, mut saves, mut lost, mut transient) = (0u64, 0u64, 0u64, 0u64);
    while Instant::now() < deadline {
        k += 1;
        // The editor buffer keeps whatever header it loaded.
        let loaded = fs::read_to_string(&path).unwrap_or_default();
        let (block, _) = split_frontmatter(&loaded);
        let data = format!("{}{}", block.unwrap_or_default(), body(k));
        match style {
            Style::Rename => {
                let tmp = path.with_extension("md.human");
                fs::write(&tmp, &data).expect("human temp write");
                fs::rename(&tmp, &path).expect("human rename-save");
            }
            Style::Inplace => fs::write(&path, &data).expect("human in-place save"),
        }
        saves += 1;
        thread::sleep(rng.short_sleep());
        if latest(&path, &re) != Some(k) {
            thread::sleep(Duration::from_millis(50));
            if latest(&path, &re) != Some(k) {
                lost += 1;
            } else {
                transient += 1;
            }
        }
    }
    HumanResult {
        saves,
        lost,
        transient,
    }
}

/// The obvious bug: read once, write a temp, plain rename, no verify. Test-only.
fn naive_splice(run_dir: &Path, fm: &str) -> &'static str {
    let target = run_dir.join(DIRECTION_FILE);
    let Ok(text) = fs::read_to_string(&target) else {
        return "no-direction";
    };
    let (_, body) = split_frontmatter(&text);
    if !body_sane(body) {
        return "skipped-insane-body";
    }
    let tmp = run_dir.join(".DIRECTION.md.naive");
    if fs::write(&tmp, format!("{fm}{body}")).is_err() {
        return "error";
    }
    if fs::rename(&tmp, &target).is_err() {
        return "error";
    }
    "ok"
}

fn tool(run_dir: PathBuf, strategy: Strategy, deadline: Instant) -> (u64, BTreeMap<String, u64>) {
    let (mut n, mut tally) = (0u64, BTreeMap::new());
    while Instant::now() < deadline {
        n += 1;
        let fm = format!("---\n{FM_MARKER} race\nn: {n}\n---\n");
        let outcome = match strategy {
            Strategy::Exchange => match splice_frontmatter(&run_dir, &fm) {
                Ok(report) => report.outcome.as_str(),
                Err(_) => "error",
            },
            Strategy::Naive => naive_splice(&run_dir, &fm),
        };
        *tally.entry(outcome.to_string()).or_insert(0) += 1;
    }
    (n, tally)
}

struct CaseResult {
    saves: u64,
    regens: u64,
    lost: u64,
    transient: u64,
    final_sane: bool,
    tool: BTreeMap<String, u64>,
    leftover_temps: usize,
}

fn run_case(strategy: Strategy, style: Style, secs: f64) -> CaseResult {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join(DIRECTION_FILE);
    fs::write(&path, body(0)).expect("seed DIRECTION.md");
    let deadline = Instant::now() + Duration::from_secs_f64(secs);
    let seed = format!("{}-{}", strategy.name(), style.name());

    let human_thread = thread::spawn({
        let path = path.clone();
        move || human(path, style, deadline, &seed)
    });
    let tool_thread = thread::spawn({
        let run_dir = dir.path().to_path_buf();
        move || tool(run_dir, strategy, deadline)
    });
    let human = human_thread.join().expect("human thread");
    let (regens, tool) = tool_thread.join().expect("tool thread");

    let final_sane = body_sane(split_frontmatter(&fs::read_to_string(&path).unwrap_or_default()).1);
    let leftover_temps = fs::read_dir(dir.path())
        .expect("read_dir")
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().starts_with(TEMP_PREFIX))
        .count();

    let result = CaseResult {
        saves: human.saves,
        regens,
        lost: human.lost,
        transient: human.transient,
        final_sane,
        tool,
        leftover_temps,
    };
    // The spike's `race-test.txt` line, so a full run can be pasted into a SUMMARY.
    println!(
        "{:8} {:7} saves={:6} regens={:7} LOST={:5} transient={:4} final_body_sane={} tool={:?}",
        strategy.name(),
        style.name(),
        result.saves,
        result.regens,
        result.lost,
        result.transient,
        result.final_sane,
        result.tool,
    );
    result
}

/// The shared assertions of the smoke and the full run.
///
/// WR-09: the in-place loss bound is a Poisson 99th percentile of a rate near 0.07
/// expected losses per second, so on a loaded machine a 1 s run can exceed it by
/// scheduling noise alone. It is asserted only by the long, `#[ignore]`d run
/// (`check_inplace_bound`); the default smoke keeps the deterministic invariants.
fn assert_exchange_holds(secs: f64, check_inplace_bound: bool) {
    let rename = run_case(Strategy::Exchange, Style::Rename, secs);
    let inplace = run_case(Strategy::Exchange, Style::Inplace, secs);

    assert_eq!(
        rename.lost, 0,
        "a rename-save must never be lost: the exchange-verify gives the displaced inode back \
         (saves={}, tool={:?})",
        rename.saves, rename.tool
    );

    let allowed = poisson_q99(SPIKE_INPLACE_LOSS_RATE * inplace.saves as f64);
    println!(
        "in-place: lost={} allowed(poisson_q99 of {:.2e} x {} saves)={}",
        inplace.lost, SPIKE_INPLACE_LOSS_RATE, inplace.saves, allowed
    );
    if check_inplace_bound {
        assert!(
            inplace.lost <= allowed,
            "in-place losses {} exceed the 99th percentile {} of Poisson({:.2e} x {} saves)",
            inplace.lost,
            allowed,
            SPIKE_INPLACE_LOSS_RATE,
            inplace.saves
        );
    }

    for (name, case) in [("rename", &rename), ("inplace", &inplace)] {
        assert!(case.final_sane, "{name}: the final body must still be sane");
        assert_eq!(
            case.leftover_temps, 0,
            "{name}: no splice temp may survive the run"
        );
        assert!(
            !case.tool.contains_key("error"),
            "{name}: the splice must not fail with an I/O error: {:?}",
            case.tool
        );
        assert!(
            case.tool.get("ok").copied().unwrap_or(0) > 0,
            "{name}: the tool must have spliced at least once: {:?}",
            case.tool
        );
    }
}

#[test]
fn editor_race_smoke() {
    assert_exchange_holds(1.0, false);
}

#[test]
fn editor_race_naive_positive_control() {
    // The race must be real or the passes above prove nothing. The naive
    // strategy's window is wide, so it is lost within a fraction of a second; a
    // few attempts keep a freak scheduler pause from failing the control.
    let mut last = None;
    for _ in 0..4 {
        let case = run_case(Strategy::Naive, Style::Rename, 0.5);
        if case.lost > 0 {
            return;
        }
        last = Some(case);
    }
    let last = last.expect("at least one attempt ran");
    panic!(
        "POSITIVE CONTROL FAILED: the naive read-once-rename strategy lost nothing in 4 x 0.5 s \
         (saves={}, regens={}); the race harness is not exercising the race",
        last.saves, last.regens
    );
}

#[test]
#[ignore = "15 s per save style; run with `-- --ignored editor_race_full --nocapture`"]
fn editor_race_full() {
    assert_exchange_holds(15.0, true);
}

/// Wait, at most `2 x DIRECTION_LOCK_BUDGET`, until `ready()` holds. The bound
/// keeps a broken lock path a failed assertion rather than a hung test.
fn wait_until(stop: &AtomicBool, ready: impl Fn() -> bool) {
    let since = Instant::now();
    while !ready() && !stop.load(Ordering::Relaxed) && since.elapsed() < 2 * DIRECTION_LOCK_BUDGET {
        thread::sleep(Duration::from_micros(50));
    }
}

#[test]
fn two_writers_under_direction_lock_lose_nothing() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join(DIRECTION_FILE);
    fs::write(&path, body(0)).expect("seed DIRECTION.md");

    // The lock hand-over is an explicit handshake, in both directions. The
    // lock's retry is a poll with backoff and gives no fairness, so whichever
    // writer re-takes it straight away starves the other once a hold (mostly
    // fsync) dwarfs the gap. A fixed 100 µs splicer gap starved the human on
    // overlay; yielding in one direction only just made the human starve the
    // splicer instead (quick 261023-qs8).
    //
    // * `requested` is the write the human is about to make; `acquired` the
    //   newest write whose edit closure ran, which write_body only does once
    //   locked() has dealt with DIRECTION.lock (and the `!lock_timed_out`
    //   assertion below proves it was really held). While requested > acquired
    //   the splicer starts no new splice, so the human waits for at most the
    //   splice already in flight.
    // * `begun` / `done` count splices. Before its next write the human waits
    //   until every splice begun so far has finished, so no splice waits behind
    //   more than one human write, and until a new one has begun, so the next
    //   write meets a busy lock.
    let requested = Arc::new(AtomicU64::new(0));
    let acquired = Arc::new(AtomicU64::new(0));
    let begun = Arc::new(AtomicU64::new(0));
    let done = Arc::new(AtomicU64::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    let splicer = thread::spawn({
        let stop = Arc::clone(&stop);
        let requested = Arc::clone(&requested);
        let acquired = Arc::clone(&acquired);
        let begun = Arc::clone(&begun);
        let done = Arc::clone(&done);
        let run_dir = dir.path().to_path_buf();
        move || {
            let (mut n, mut failures) = (0u64, 0u64);
            while !stop.load(Ordering::Relaxed) {
                wait_until(&stop, || {
                    requested.load(Ordering::SeqCst) <= acquired.load(Ordering::SeqCst)
                });
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                // Splice right away: straight after an acquisition this contends
                // for the lock while the human still holds it.
                n += 1;
                let fm = format!("---\n{FM_MARKER} race\nn: {n}\n---\n");
                begun.fetch_add(1, Ordering::SeqCst);
                match splice_frontmatter(&run_dir, &fm) {
                    Ok(report) if report.outcome == SpliceOutcome::Ok => {}
                    _ => failures += 1,
                }
                done.fetch_add(1, Ordering::SeqCst);
                // CPU relief only: fairness comes from the handshake, not from
                // hoping the human's retry lands in this gap.
                thread::sleep(Duration::from_micros(100));
            }
            (n, failures)
        }
    });

    let re = Regex::new(r"human edit (\d+)").expect("counter regex");
    let mut contended = 0u64;
    for k in 1..=200u64 {
        let before = begun.load(Ordering::SeqCst);
        wait_until(&stop, || {
            done.load(Ordering::SeqCst) >= before && begun.load(Ordering::SeqCst) > before
        });
        requested.store(k, Ordering::SeqCst);
        // A splice in flight now means this write meets a busy lock.
        if begun.load(Ordering::SeqCst) > done.load(Ordering::SeqCst) {
            contended += 1;
        }
        let report = write_body(dir.path(), |old| {
            acquired.fetch_max(k, Ordering::SeqCst);
            re.replace(old, format!("human edit {k}")).into_owned()
        })
        .expect("write_body");
        assert_eq!(
            report.outcome,
            SpliceOutcome::Ok,
            "write {k}: a body write under the shared lock must land"
        );
        assert!(
            !report.lock_timed_out,
            "write {k}: the shared lock must be obtainable, or this test no longer exercises it"
        );
        assert_eq!(
            latest(&path, &re),
            Some(k),
            "write {k} was lost: the file does not show it after write_body returned"
        );
    }

    stop.store(true, Ordering::Relaxed);
    let (splices, failures) = splicer.join().expect("splicer thread");
    assert!(splices > 0, "the splicer must have run concurrently");
    println!("two writers: splices={splices} contended_writes={contended}/200");
    assert!(
        contended > 0,
        "no write ever started while a splice was in flight: the test no longer \
         exercises contention for DIRECTION.lock"
    );
    assert_eq!(failures, 0, "no splice may fail or give up under the lock");
    assert!(
        body_sane(split_frontmatter(&fs::read_to_string(&path).expect("read")).1),
        "the final body is sane"
    );
}
