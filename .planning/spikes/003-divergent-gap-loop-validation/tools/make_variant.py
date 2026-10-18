#!/usr/bin/env python3
"""Spike 003 Run-2 variant generator: the v0.4 driver prompt and the run-1 DIRECTION with the prompt-side
gap-round and divergence rules REMOVED, by asserted text removal.

Every target span must occur exactly once in its source; otherwise this exits 1 naming the span, so a
drifted prompt can never silently produce a variant that still carries the rule. Run 2 then relies on
the launcher gap cap alone (deny at limit+1, kill at limit+2).

Outputs (committed for audit):
  experiment/MASTER-PROMPT.run2.md   from MASTER-PROMPT.md
  templates/DIRECTION.run2.md        from templates/DIRECTION.run1.md

Usage: make_variant.py [--check]
  (no flag)  write both files and print a unified diff of each
  --check    regenerate in memory; exit 1 if a span is not present exactly once or a committed file differs
"""
import difflib
import os
import sys

S3 = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

PROMPT_SRC = os.path.join(S3, "MASTER-PROMPT.md")
PROMPT_OUT = os.path.join(S3, "experiment", "MASTER-PROMPT.run2.md")
DIR_SRC = os.path.join(S3, "templates", "DIRECTION.run1.md")
DIR_OUT = os.path.join(S3, "templates", "DIRECTION.run2.md")

PROMPT_SPANS = [
    ("title", "# GSD queue driver: master prompt (draft v0.4)", "# GSD queue driver: master prompt (draft v0.4-run2)"),
    ("v0.4 comment",
     """     v0.3 changes vs v0.2 (spike 003): rule 5 gains gap-round semantics, finding granularity, a
     divergence-over-gap-rounds precedence and a needs-human record on those parks; an answer never
     lifts a stop rule. The worker brief and Loop are unchanged.
     v0.4 changes vs v0.3 (spike 003 / Phase 26.1): the worker brief forbids background nested agents
     and ending a turn while one runs; Loop step 4 treats a worker completion without a `STATUS:`
     report as an early end, answered by SendMessage to the same worker, never a new dispatch. Rule 5
     is unchanged. -->""",
     """     v0.4-run2 (spike 003 run 2 / Phase 26.1): the gap-round and divergence rules are removed on
     purpose; the launcher gap cap is the only gap-round limit. The v0.4 worker brief and Loop step 4
     are kept as in MASTER-PROMPT.md. -->"""),
    ("item-budget gap-rounds mention",
     "   - item budget exhausted: dispatches, minutes (from its first dispatch) or gap-rounds -> `budget`;\n",
     "   - item budget exhausted: dispatches or minutes (from its first dispatch) -> `budget`;\n"),
    ("gap-rounds bullet",
     """   - **gap rounds**: a gap round starts each time the phase's `*-VERIFICATION.md` frontmatter says
     `status: gaps_found`. Two gap-closure rounds per phase at most (or the item's `gap-rounds` if
     lower): when verification reports `gaps_found` for the **third** time, park with `gap-rounds` -
     never dispatch a third closure. `driverctl show` displays the launcher's own count
     (`caps.gap_rounds`), which is authoritative if it differs from yours.
     On a `gap-rounds` or `divergence` park, also run `driverctl log needs-human` with the question a
     human must answer;
""", ""),
    ("divergence bullet",
     """   - **divergence**: two consecutive rounds each report new findings (gaps not a subset of the
     previous round's) -> `divergence`. Also park as `divergence` when a fix for one success
     criterion re-breaks another (the gap list oscillates) - that gap cannot close by more rounds.
     Round semantics: round 0 is the initial execution and its `gaps_found` is the baseline, not a
     round; round k is the k-th gap-closure round; round k has new findings when the `gaps_found`
     after it lists findings absent from the `gaps_found` before it; divergence needs rounds 1 and 2
     both to have new findings, so it can first hold at the third `gaps_found`.
     A finding is one failing case as the verifier lists it under a gap (its reason or missing
     detail: the failing lines, inputs or cases), not only the criterion's name; the same criterion
     failing on cases it did not fail on before is a new finding.
     When divergence holds together with `gap-rounds` or an exhausted gap-rounds budget, park with
     `divergence`;
""", ""),
    ("Loop step 2 gap-closure sentence",
     " Before a gap-closure step, check rule 5 first.", ""),
    ("no-round-limit sentence (appended to rule 5's list)",
     "   Run-level: stop when",
     "   - Gap closure has no round limit in this run: keep closing `gaps_found` until verification passes\n"
     "     or a dispatch is denied.\n"
     "   Run-level: stop when"),
]

DIRECTION_SPANS = [
    ("gap-round standing rule",
     "- Max 2 gap-closure rounds per phase; a third is a park, never a dispatch.\n", ""),
    ("gap-rounds budget key", "gap-rounds=2, ", ""),
]


def apply(text, spans):
    for name, old, new in spans:
        n = text.count(old)
        if n != 1:
            print(f"make_variant: span '{name}' occurs {n} times (need exactly 1)", file=sys.stderr)
            sys.exit(1)
        text = text.replace(old, new)
    return text


def read(path):
    with open(path) as f:
        return f.read()


def main(argv):
    check = argv == ["--check"]
    if argv and not check:
        print(__doc__.split("Usage:")[1].strip().split("\n")[0], file=sys.stderr)
        return 2
    jobs = [(PROMPT_SRC, PROMPT_OUT, PROMPT_SPANS), (DIR_SRC, DIR_OUT, DIRECTION_SPANS)]
    outs = [(src, out, read(src), apply(read(src), spans)) for src, out, spans in jobs]
    rc = 0
    for src, out, old, new in outs:
        if check:
            if not os.path.exists(out) or read(out) != new:
                print(f"make_variant: {os.path.relpath(out, S3)} differs from the regenerated variant", file=sys.stderr)
                rc = 1
        else:
            with open(out, "w") as f:
                f.write(new)
            sys.stdout.writelines(difflib.unified_diff(old.splitlines(True), new.splitlines(True),
                                                       os.path.relpath(src, S3), os.path.relpath(out, S3)))
    if check and rc == 0:
        print("make_variant: check OK")
    return rc


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
