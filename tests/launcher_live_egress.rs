#![cfg(target_os = "linux")]
// ============================================================================
// LIVE proof of the network-isolated sandbox (Phase 28.1, plan 28.1-10; D-02,
// D-03, D-04, SC1, T-28.1-21, T-28.1-22). Opt-in: the test is `#[ignore]` because
// it needs a `bwrap` that can create network namespaces and `curl`.
//
//   cargo test --test launcher_live_egress -- --ignored --nocapture
//
// Run it OUTSIDE any tool sandbox. A test that cannot run prints a `SKIP:` line and
// returns; a `SKIP:` line is no evidence, it means this host proved nothing.
//
// What it shows: the real `bwrap_argv` profile with the real `gsd-meta-manager
// bridge` as bwrap's command, an in-process `EgressServer` on the run's egress
// socket and a plain-TCP stub standing in for `api.anthropic.com`. From inside:
// only `lo` exists, the hostname is fixed, a request to `$ANTHROPIC_BASE_URL`
// reaches the egress server (200 for `/v1/messages`, 403 for an account path), a
// CONNECT to a host off the allowlist is refused (403), a direct connection to a
// public address fails, and a listener the test opened on the HOST's loopback is
// unreachable, and so are a listener on the host's LAN address, a host abstract
// unix socket and (when the host has it) Xwayland's `@/tmp/.X11-unix/X0`. No
// `docker.sock` or system bus socket is visible under the `/run` tmpfs, `/run`
// lists only the runtime-dir mount point, and no SysV shared-memory segment is
// listed. The host-side controls (the test process connects to the same
// listeners) keep the "unreachable" results from being vacuous.
//
// The probe prints only `KEY=VALUE` lines, never the content of a host file.
// ============================================================================

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io::{Read, Write};
use std::net::UdpSocket;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::linux::net::SocketAddrExt;
use std::os::unix::net::{SocketAddr as UnixSocketAddr, UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use gsd_meta_manager::driverctl::serve::socket_path;
use gsd_meta_manager::launcher::auth::resolve_launch_token;
use gsd_meta_manager::launcher::egress::upstream::Upstream;
use gsd_meta_manager::launcher::egress::{
    socket_path_for, EgressServer, EgressSpec, EgressStats, TokenProvider,
};
use gsd_meta_manager::launcher::home::HomeSetup;
use gsd_meta_manager::launcher::host::detect;
use gsd_meta_manager::launcher::sandbox::{
    bridged_command, bwrap_argv, child_env, egress_env_sets, LaunchPaths, SandboxSpec, BRIDGE_PORT,
    DUMMY_OAUTH_TOKEN,
};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

#[path = "launcher_support/mod.rs"]
mod launcher_support;

/// The wall-clock bound on the sandboxed probe.
const PROBE_BOUND: Duration = Duration::from_secs(90);

/// The in-sandbox probe. `$1` is the port of a listener the test opened on the
/// host's loopback, `$2` the `ip:port` of one on the host's LAN address (`none`
/// when the host has no route), `$3` the name of a host abstract unix socket and
/// `$4` is `x11` when the host also lists Xwayland's abstract socket. Every line
/// is `KEY=VALUE`.
///
/// `ifaces` reads `/proc/net/dev`, not `/sys/class/net`: the sandbox keeps the
/// host's `/sys` (a read-only bind of `/`), and a sysfs mount belongs to the network
/// namespace it was mounted in, so `/sys/class/net` lists the HOST's interfaces
/// even inside a fresh namespace. `/proc` is mounted inside the sandbox and
/// answers for its own namespace.
const PROBE: &str = r#"
host_port=$1; lan=$2; abstract=$3; x11=$4
echo "ifaces=$(tail -n +3 /proc/net/dev | cut -d: -f1 | tr -d ' ' | paste -sd, -)"
echo "hostname=$(cat /proc/sys/kernel/hostname)"
echo "msgs=$(curl -s -o /dev/null -m 10 -w '%{http_code}' -X POST -H 'content-type: application/json' -H 'anthropic-version: 2023-06-01' -d '{}' "$ANTHROPIC_BASE_URL/v1/messages")"
echo "sessions=$(curl -s -o /dev/null -m 10 -w '%{http_code}' "$ANTHROPIC_BASE_URL/v1/code/sessions")"
echo "connect=$(curl -s -o /dev/null -m 10 -w '%{http_connect}' https://example.com)"
curl -s -o /dev/null -m 5 --noproxy '*' https://1.1.1.1; echo "direct_rc=$?"
curl -s -o /dev/null -m 5 --noproxy '*' "http://127.0.0.1:$host_port/"; echo "loopback_rc=$?"
if [ "$lan" != none ]; then
  curl -s -o /dev/null -m 5 --noproxy '*' "http://$lan/"; echo "lan_rc=$?"
else
  echo "lan_rc=skipped"
fi
curl -s -o /dev/null -m 5 --noproxy '*' --abstract-unix-socket "$abstract" http://localhost/; echo "abstract_rc=$?"
if [ "$x11" = x11 ]; then
  curl -s -o /dev/null -m 5 --noproxy '*' --abstract-unix-socket /tmp/.X11-unix/X0 http://localhost/; echo "x11_rc=$?"
else
  echo "x11_rc=skipped"
fi
if [ -S /run/docker.sock ]; then echo "docker_sock=present"; else echo "docker_sock=absent"; fi
if [ -S /run/dbus/system_bus_socket ]; then echo "system_bus=present"; else echo "system_bus=absent"; fi
echo "run_entries=$(ls -A /run | paste -sd, -)"
echo "shm=$(tail -n +2 /proc/sysvipc/shm | wc -l | tr -d ' ')"
"#;

// ----------------------------------------------------------------- the stub

/// A stand-in for `api.anthropic.com`: answers `{}` with 200 to every request.
struct Stub {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Stub {
    fn start() -> Stub {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind the stub");
        listener.set_nonblocking(true).expect("nonblocking");
        let addr = listener.local_addr().expect("addr");
        let stop = Arc::new(AtomicBool::new(false));
        let stop_t = Arc::clone(&stop);
        let thread = std::thread::spawn(move || loop {
            match listener.accept() {
                Ok((stream, _)) => {
                    std::thread::spawn(move || stub_connection(stream));
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if stop_t.load(Ordering::SeqCst) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(2));
                }
                Err(_) => break,
            }
        });
        Stub {
            addr,
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for Stub {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn stub_connection(mut stream: TcpStream) {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let end = loop {
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    };
    let head = String::from_utf8_lossy(&buf[..end]).into_owned();
    let length = head
        .split("\r\n")
        .filter_map(|l| l.split_once(':'))
        .find(|(n, _)| n.trim().eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.trim().parse::<usize>().ok())
        .unwrap_or(0);
    while buf.len() < end + length {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
    let _ =
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}");
}

// ------------------------------------------------------------------ helpers

fn bwrap_netns_runs_here() -> Result<(), String> {
    match Command::new("bwrap")
        .args([
            "--unshare-net",
            "--unshare-ipc",
            "--unshare-uts",
            "--unshare-pid",
            "--ro-bind",
            "/",
            "/",
            "--dev",
            "/dev",
            "--",
            "true",
        ])
        .stdin(Stdio::null())
        .output()
    {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(format!(
            "bwrap cannot create the namespaces here ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        )),
        Err(e) => Err(format!("bwrap is not runnable: {e}")),
    }
}

fn curl_runs_here() -> Result<(), String> {
    match Command::new("curl")
        .arg("--version")
        .stdin(Stdio::null())
        .output()
    {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(format!("curl --version exited {}", out.status)),
        Err(e) => Err(format!("curl is not runnable: {e}")),
    }
}

/// `KEY=VALUE` lines to a map (the last duplicate wins).
fn parse_lines(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter_map(|line| line.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}

/// Run `argv` with exactly `env`, bounded by [`PROBE_BOUND`]; stdout and stderr as text.
fn run_bounded(
    argv: &[OsString],
    env: &BTreeMap<OsString, OsString>,
    cwd: &Path,
) -> (bool, String, String) {
    let (program, rest) = argv.split_first().expect("a program");
    let mut child = Command::new(program)
        .args(rest)
        .env_clear()
        .envs(env)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn bwrap");
    let mut stdout = child.stdout.take().expect("stdout");
    let mut stderr = child.stderr.take().expect("stderr");
    let out_t = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stdout.read_to_string(&mut s);
        s
    });
    let err_t = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr.read_to_string(&mut s);
        s
    });
    let deadline = Instant::now() + PROBE_BOUND;
    let success = loop {
        match child.try_wait().expect("wait") {
            Some(status) => break status.success(),
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                break false;
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    };
    (
        success,
        out_t.join().unwrap_or_default(),
        err_t.join().unwrap_or_default(),
    )
}

/// One sandboxed fixture: project, run dir, config dir, home, the driverctl socket
/// (a dummy listener) and the run's egress socket served by a real `EgressServer`.
struct Rig {
    _scratch: TempDir,
    project: PathBuf,
    spec: SandboxSpec,
    egress_sock: PathBuf,
    egress: Option<gsd_meta_manager::launcher::egress::EgressHandle>,
    _driverctl: UnixListener,
    _stub: Stub,
}

impl Rig {
    /// `None` (with a `SKIP:` line) when this host cannot run the probe.
    fn start(name: &str) -> Option<Rig> {
        if let Err(reason) = bwrap_netns_runs_here() {
            println!("SKIP: {reason}");
            return None;
        }
        if let Err(reason) = curl_runs_here() {
            println!("SKIP: {reason}");
            return None;
        }
        match std::env::var_os("XDG_RUNTIME_DIR") {
            Some(dir) if !dir.is_empty() && Path::new(&dir).is_dir() => {}
            _ => {
                println!("SKIP: XDG_RUNTIME_DIR is not set to a directory");
                return None;
            }
        }
        // bwrap's `--tmpfs /tmp` hides /tmp, so the fixture lives under the target dir.
        let base = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("launcher-live-egress");
        fs::create_dir_all(&base).unwrap();
        let scratch = TempDir::new_in(&base).unwrap();
        let root = scratch.path().canonicalize().unwrap();
        let project = root.join("project");
        let run_dir = project.join(".planning/meta-manager/drivers").join(name);
        let config_dir = root.join("config");
        let home = root.join("home");
        for dir in [
            &run_dir,
            &config_dir.join("bin"),
            &config_dir.join("managed"),
            &home,
        ] {
            fs::create_dir_all(dir).unwrap();
        }
        fs::write(config_dir.join("settings.json"), "{}").unwrap();
        fs::write(run_dir.join("launcher.lock"), "").unwrap();
        fs::write(run_dir.join("DIRECTION.lock"), "").unwrap();

        // `socket_path` creates the private 0700 directory when it is absent.
        let driverctl_sock = socket_path(name, &run_dir).expect("the run's driverctl socket path");
        let _ = fs::remove_file(&driverctl_sock);
        let driverctl = UnixListener::bind(&driverctl_sock).expect("bind the driverctl socket");
        let egress_sock = socket_path_for(&driverctl_sock, &run_dir);
        let _ = fs::remove_file(&egress_sock);
        let egress_listener = UnixListener::bind(&egress_sock).expect("bind the egress socket");

        let stub = Stub::start();
        let egress = EgressServer::start(
            egress_listener,
            EgressSpec {
                allow_hosts: Vec::new(),
                token: TokenProvider::Static("LIVE-EGRESS-REAL-TOKEN-SENTINEL".to_owned()),
                upstream: Upstream::Plain(stub.addr),
                log_path: root.join("egress.log"),
            },
        )
        .expect("start the egress server");

        // The fixture home is a scratch directory, so the allowlist is nearly empty; the
        // gitconfig still comes from the host, as in a launch (D-17).
        let home_setup = HomeSetup::for_host(
            &home,
            Path::new(env!("CARGO_BIN_EXE_gsd-meta-manager")),
            detect().overlay,
            &root,
        )
        .expect("derive the $HOME setup");
        let spec = SandboxSpec::from_host(
            LaunchPaths {
                project: project.clone(),
                run_dir,
                config_dir,
                socket: driverctl_sock,
                egress_socket: egress_sock.clone(),
            },
            &home,
            &home_setup,
        );
        Some(Rig {
            _scratch: scratch,
            project,
            spec,
            egress_sock,
            egress: Some(egress),
            _driverctl: driverctl,
            _stub: stub,
        })
    }

    /// Run `script` (with `args` as `$1...`) inside the profile behind the real bridge.
    fn probe(&self, script: &str, args: &[String]) -> (bool, String, String) {
        let mut inner: Vec<OsString> =
            vec!["sh".into(), "-c".into(), script.into(), "probe".into()];
        inner.extend(args.iter().map(OsString::from));
        let command = bridged_command(
            Path::new(env!("CARGO_BIN_EXE_gsd-meta-manager")),
            BRIDGE_PORT,
            &self.egress_sock,
            &inner,
        );
        let argv = bwrap_argv(&self.spec, &command);
        let env = child_env(std::env::vars_os(), &egress_env_sets(BRIDGE_PORT));
        run_bounded(&argv, &env, &self.project)
    }

    fn stop(&mut self) -> EgressStats {
        self.egress.take().expect("running").stop()
    }
}

/// The host's LAN address: the source address of a UDP socket "connected" to a
/// public address (connecting sends no packet). `None` when the host has no route.
fn host_lan_ip() -> Option<std::net::IpAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("1.1.1.1:53").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_loopback() && !ip.is_unspecified()).then_some(ip)
}

/// Whether the host lists the abstract socket `@name` in `/proc/net/unix`.
fn host_lists_abstract(name: &str) -> bool {
    fs::read_to_string("/proc/net/unix")
        .map(|text| text.lines().any(|l| l.ends_with(&format!(" @{name}"))))
        .unwrap_or(false)
}

#[test]
#[ignore = "needs bwrap with network namespaces and curl; run with -- --ignored --nocapture"]
fn the_network_namespace_leaves_only_the_egress_bridge() {
    let Some(mut rig) = Rig::start("egress281") else {
        return;
    };

    // A listener on the HOST's loopback. Its port is not the bridge's.
    let host_listener = TcpListener::bind("127.0.0.1:0").expect("bind the host loopback listener");
    let host_port = host_listener.local_addr().unwrap().port();
    assert_ne!(
        host_port, BRIDGE_PORT,
        "pick another run: the port collides with the bridge"
    );
    // A listener on the host's LAN address (what claudear-style local agents sit on).
    let lan_listener = host_lan_ip().and_then(|ip| TcpListener::bind((ip, 0)).ok());
    let lan_arg = lan_listener
        .as_ref()
        .map_or("none".to_owned(), |l| l.local_addr().unwrap().to_string());
    // A host abstract unix socket.
    let abstract_name = format!("gsdmm-probe-{}", std::process::id());
    let abstract_listener = UnixListener::bind_addr(
        &UnixSocketAddr::from_abstract_name(abstract_name.as_bytes()).unwrap(),
    )
    .expect("bind the host abstract socket");
    let x11 = host_lists_abstract("/tmp/.X11-unix/X0");

    let (ok, stdout, stderr) = rig.probe(
        PROBE,
        &[
            host_port.to_string(),
            lan_arg.clone(),
            abstract_name.clone(),
            if x11 { "x11" } else { "none" }.to_owned(),
        ],
    );
    println!("LIVE probe exit ok={ok}");
    print!("{stdout}");
    if !stderr.trim().is_empty() {
        println!("LIVE probe stderr: {}", stderr.trim());
    }
    let stats = rig.stop();
    println!(
        "inference_forwarded={} connect_denied={}",
        stats.inference_forwarded, stats.connect_denied
    );

    // The host-side non-vacuity controls: every listener answers a connect from here.
    let loopback_control = TcpStream::connect(("127.0.0.1", host_port)).is_ok();
    let lan_control = lan_listener
        .as_ref()
        .map(|l| TcpStream::connect(l.local_addr().unwrap()).is_ok());
    let abstract_control = UnixStream::connect_addr(
        &UnixSocketAddr::from_abstract_name(abstract_name.as_bytes()).unwrap(),
    )
    .is_ok();
    let x11_control = x11.then(|| {
        UnixStream::connect_addr(&UnixSocketAddr::from_abstract_name(b"/tmp/.X11-unix/X0").unwrap())
            .is_ok()
    });
    let host_docker = Path::new("/run/docker.sock").exists();
    let host_bus = Path::new("/run/dbus/system_bus_socket").exists();
    let host_shm = fs::read_to_string("/proc/sysvipc/shm")
        .map(|t| t.lines().skip(1).count())
        .unwrap_or(0);
    println!("host_loopback_control={loopback_control}");
    println!("host_lan_control={lan_control:?} (lan listener {lan_arg})");
    println!("host_abstract_control={abstract_control}");
    println!("host_x11_control={x11_control:?}");
    println!("host_docker_sock_present={host_docker}");
    println!("host_system_bus_present={host_bus}");
    println!("host_shm_segments={host_shm}");
    drop(abstract_listener);

    let got = parse_lines(&stdout);
    let value = |key: &str| got.get(key).map_or("<missing>", String::as_str).to_owned();
    assert!(ok, "the probe failed: {stderr}");
    assert_eq!(
        value("ifaces"),
        "lo",
        "only the loopback interface exists inside"
    );
    assert_eq!(value("hostname"), "gsdmm-sandbox");
    assert_eq!(
        value("msgs"),
        "200",
        "an inference POST reaches the egress server"
    );
    assert_eq!(
        value("sessions"),
        "403",
        "an account path is refused by the egress server"
    );
    assert_eq!(
        value("connect"),
        "403",
        "a CONNECT off the allowlist is refused"
    );
    assert_ne!(
        value("direct_rc"),
        "0",
        "a direct connection to a public address must fail"
    );
    // curl exit 7 is "failed to connect": the destination does not exist inside, which
    // is a stronger result than a protocol error from something that answered.
    assert_eq!(
        value("loopback_rc"),
        "7",
        "the host's loopback must be unreachable"
    );
    if lan_listener.is_some() {
        assert_eq!(
            value("lan_rc"),
            "7",
            "the host's LAN address must be unreachable"
        );
    }
    assert_eq!(
        value("abstract_rc"),
        "7",
        "a host abstract socket must be unreachable"
    );
    if x11 {
        assert_eq!(
            value("x11_rc"),
            "7",
            "Xwayland's abstract socket must be unreachable"
        );
    }
    assert_eq!(value("docker_sock"), "absent");
    assert_eq!(value("system_bus"), "absent");
    assert_eq!(value("run_entries"), "user");
    assert_eq!(value("shm"), "0");

    assert!(
        loopback_control,
        "the host-side control could not reach its own listener"
    );
    assert_ne!(
        lan_control,
        Some(false),
        "the LAN listener did not answer the host"
    );
    assert!(
        abstract_control,
        "the abstract listener did not answer the host"
    );
    assert_ne!(
        x11_control,
        Some(false),
        "Xwayland's socket did not answer the host"
    );
    assert!(stats.inference_forwarded >= 1);
}

// =========================================================================
// 28.1-07 SC1: a REAL `driver launch` (real systemd-run and bwrap, only `claude`
// faked) reaches the API and the operator's package hosts only through the
// launcher's proxy, with no usable token inside.
// =========================================================================

/// The fake driver of the full launch. It answers `--version`, then probes from
/// inside the sandbox and writes `KEY=VALUE` lines under the (writable) per-run
/// config directory, then ends the run the way a well-mannered driver does.
///
/// `__DUMMY__` and `__MODEL__` are filled in by the test. The `msgs` body asks the
/// current Haiku model for one token, so the request spends well under $0.01.
const FULL_LAUNCH_CLAUDE: &str = r#"#!/bin/sh
if [ "$1" = "--version" ]; then echo "2.1.303 (Claude Code)"; exit 0; fi
OUT="$CLAUDE_CONFIG_DIR/live-egress.txt"
: > "$OUT"
say() { printf '%s=%s\n' "$1" "$2" >> "$OUT"; }
driverctl log start --mode headless --loop-mode loop
BODY='{"model":"__MODEL__","max_tokens":1,"messages":[{"role":"user","content":"Reply with one word."}]}'
say msgs "$(curl -s -o "$CLAUDE_CONFIG_DIR/msgs-body.txt" -m 60 -w '%{http_code}' -X POST "$ANTHROPIC_BASE_URL/v1/messages?beta=true" -H 'anthropic-version: 2023-06-01' -H 'anthropic-beta: oauth-2025-04-20' -H 'content-type: application/json' -d "$BODY")"
say sessions "$(curl -s -o /dev/null -m 20 -w '%{http_code}' "$ANTHROPIC_BASE_URL/v1/code/sessions")"
say crates "$(curl -s -o /dev/null -m 30 -w '%{http_code}' https://index.crates.io/config.json)"
git ls-remote https://github.com/rust-lang/cargo HEAD > /dev/null 2>&1
say git_ls_rc "$?"
say example_connect "$(curl -s -o /dev/null -m 15 -w '%{http_connect}' https://example.com)"
if [ "$CLAUDE_CODE_OAUTH_TOKEN" = "__DUMMY__" ]; then say token_is_dummy yes; else say token_is_dummy no; fi
hashes=""
for f in /proc/[0-9]*/environ; do
  v=$(tr '\0' '\n' < "$f" 2>/dev/null | sed -n 's/^CLAUDE_CODE_OAUTH_TOKEN=//p' | head -n 1)
  if [ -n "$v" ]; then hashes="$hashes$(printf '%s' "$v" | sha256sum | cut -d' ' -f1),"; fi
done
say environ_token_hashes "$hashes"
driverctl show > /dev/null 2>&1
say driverctl_rc "$?"
driverctl log end --reason stop
printf '%s\n' '{"type":"result","subtype":"success"}'
exit 0
"#;

/// The Haiku model id the live request names. The claude-api skill lists
/// `claude-haiku-5-5` as the current Haiku, but a direct host-side request with the
/// same OAuth token answers `429 rate_limit_error` for that id (measured 2026-10-24,
/// no proxy involved) and 200 for `claude-haiku-4-5`, so the request names the
/// latter `[inferred — audit]`.
const LIVE_HAIKU_MODEL: &str = "claude-haiku-4-5";

/// Why the full-launch test cannot run on this host, if it cannot.
fn full_launch_skip_reason() -> Option<String> {
    if let Err(e) = bwrap_netns_runs_here() {
        return Some(e);
    }
    if let Err(e) = curl_runs_here() {
        return Some(e);
    }
    let systemd = Command::new("systemctl")
        .args(["--user", "is-system-running"])
        .stdin(Stdio::null())
        .output();
    match systemd {
        Ok(out)
            if matches!(
                String::from_utf8_lossy(&out.stdout).trim(),
                "running" | "degraded"
            ) => {}
        other => return Some(format!("the user systemd manager is not usable: {other:?}")),
    }
    if std::env::var_os("XDG_RUNTIME_DIR").is_none() {
        return Some("XDG_RUNTIME_DIR is not set".to_owned());
    }
    if let Err(e) = resolve_launch_token(chrono::Utc::now()) {
        return Some(format!("no launch token resolves from the real HOME: {e}"));
    }
    None
}

fn sha_hex(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
#[ignore = "needs bwrap, a user systemd, network and a token; spends under $0.01; run with -- --ignored --nocapture"]
fn a_real_launch_reaches_the_api_only_through_the_proxy() {
    use std::os::unix::fs::PermissionsExt;

    if let Some(reason) = full_launch_skip_reason() {
        println!("SKIP: {reason}");
        return;
    }
    let real_token = resolve_launch_token(chrono::Utc::now())
        .expect("a launch token")
        .value;

    let target_tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("launcher-live");
    let mut fx = launcher_support::Fixture::new_in(&target_tmp);
    fx.runtime_dir = PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR"));
    // Only `claude` is fake: the host's real systemd-run, bwrap and systemctl run.
    for fake in ["systemd-run", "bwrap", "systemctl"] {
        fs::remove_file(fx.shims().join(fake)).expect("remove a fake tool");
    }
    let claude = FULL_LAUNCH_CLAUDE
        .replace("__DUMMY__", DUMMY_OAUTH_TOKEN)
        .replace("__MODEL__", LIVE_HAIKU_MODEL);
    launcher_support::write_executable(&fx.shims(), "claude", &claude);
    // The project must be its own repository with no remote: inside this checkout the
    // run-end push would resolve this repository's `origin`.
    let init = Command::new("git")
        .args(["init", "-q"])
        .arg(fx.project())
        .stdin(Stdio::null())
        .output()
        .expect("git init");
    assert!(init.status.success(), "git init failed");
    fs::write(
        fx.registry(),
        serde_json::json!({
            "version": 2,
            "projects": {launcher_support::ALIAS: {
                "path": fx.project(),
                "added": "2026-01-01T00:00:00Z",
                "driver": {"egress": {"presets": ["rust", "github-read"]}},
            }}
        })
        .to_string(),
    )
    .expect("registry");
    fs::set_permissions(fx.registry(), fs::Permissions::from_mode(0o600)).expect("chmod");

    let mut path = fx.shims().into_os_string();
    path.push(":");
    path.push(std::env::var_os("PATH").unwrap_or_default());
    let mut child = Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"))
        .arg("--config")
        .arg(fx.registry())
        .args([
            "driver",
            "launch",
            launcher_support::ALIAS,
            launcher_support::RUN_ID,
        ])
        .arg("--prompt-file")
        .arg(fx.prompt_file())
        .arg("--foreground")
        // The real HOME, XDG_CONFIG_HOME and XDG_RUNTIME_DIR are inherited so the real
        // token resolves; only the launcher's data root is private.
        .env("PATH", path)
        .env("XDG_DATA_HOME", fx.scratch.path().join("data"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the launcher");
    let deadline = Instant::now() + Duration::from_secs(240);
    let status = loop {
        match child.try_wait().expect("wait for the launcher") {
            Some(status) => break status,
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("the launch did not end within 240 s");
            }
            None => std::thread::sleep(Duration::from_millis(100)),
        }
    };
    let mut launcher_stderr = String::new();
    if let Some(mut e) = child.stderr.take() {
        let _ = e.read_to_string(&mut launcher_stderr);
    }
    println!("LAUNCHER exit={status} stderr=\n{launcher_stderr}");
    assert!(status.success(), "the launch failed: {launcher_stderr}");

    let live = fs::read_to_string(fx.config_dir().join("live-egress.txt")).unwrap_or_default();
    for line in live.lines() {
        println!("{line}");
    }
    let kv = parse_lines(&live);
    let get = |k: &str| kv.get(k).map_or("<absent>", String::as_str).to_owned();
    let msgs_body = fs::read_to_string(fx.config_dir().join("msgs-body.txt")).unwrap_or_default();
    assert_eq!(
        get("msgs"),
        "200",
        "the inference call: {}",
        msgs_body.chars().take(400).collect::<String>()
    );
    assert_eq!(get("sessions"), "403", "an account path is refused");
    assert_eq!(get("crates"), "200", "index.crates.io through CONNECT");
    assert_eq!(get("git_ls_rc"), "0", "git ls-remote through CONNECT");
    assert_eq!(get("example_connect"), "403", "a host off the allowlist");
    assert_eq!(get("token_is_dummy"), "yes");
    assert_eq!(get("driverctl_rc"), "0", "the driverctl socket still works");
    let real_hash = sha_hex(&real_token);
    assert!(
        !get("environ_token_hashes").contains(&real_hash),
        "a process inside holds the real token"
    );

    // What the launcher recorded.
    let state = fx.state_json();
    let auth = state["auth"].as_str().unwrap_or_default().to_owned();
    println!("auth={auth}");
    assert!(
        matches!(auth.as_str(), "setup-token" | "host-token"),
        "{state}"
    );
    assert_eq!(
        state["egress_allow"],
        serde_json::json!([
            "index.crates.io",
            "static.crates.io",
            "crates.io",
            "github.com",
            "codeload.github.com",
            "objects.githubusercontent.com"
        ])
    );
    let requests = state["inference_requests"].as_u64().unwrap_or(0);
    println!("inference_requests={requests}");
    assert!(requests >= 1, "{state}");

    let log = fs::read_to_string(fx.state_dir().join("egress.log")).unwrap_or_default();
    for line in log.lines() {
        println!("egress.log: {line}");
    }
    assert!(log.contains("|inference|POST|/v1/messages|200|"), "{log}");
    assert!(
        log.contains("|connect|CONNECT index.crates.io:443|-|200|"),
        "{log}"
    );
    assert!(
        log.contains("|connect|CONNECT example.com:443|-|403|"),
        "{log}"
    );
    assert!(
        !log.contains(&real_token),
        "the real token is in egress.log"
    );
    assert!(
        !live.contains(&real_token),
        "the real token reached the probe"
    );
}
