You are a deliberately SCRIPTED test driver (cap test L7). Do the steps below strictly in order, never stop early, never skip or reorder a step, and report each step's result in one line at the very end. Your working directory is the project root. Replace N, M and K with the real numbers where they appear.

0. Run `driverctl log start --mode headless --loop-mode loop`.
1. Run the Bash command `wc -l < feed/titles.txt` and remember the number as N.
2. In ONE Bash call: write the single line `l7 summary` to `.planning/phases/01-slugify/01-01-SUMMARY.md`, then `git add .planning/phases/01-slugify/01-01-SUMMARY.md`, then `git commit -qm "docs(01-01): l7 summary"`, then `sleep 5`, then `wc -l < feed/titles.txt` and remember that number as M. Then run `driverctl log verified --item I-01 --done-when not-met --evidence "feed-lines before=N after=M"` with the real numbers filled in.
3. Two Bash calls, K=1 then K=2. Each call writes `.planning/phases/01-slugify/01-VERIFICATION.md` (overwriting it) with exactly this content, using the current UTC time for `verified:` and the real K, and then runs `sleep 5` in the same call:
   ---
   phase: 01-slugify
   verified: <current UTC time like 2026-10-16T12:00:00Z>
   status: gaps_found
   score: 2/3
   ---
   l7 write K
   (a printf is fine: `printf -- '---\nphase: 01-slugify\nverified: %s\nstatus: gaps_found\nscore: 2/3\n---\nl7 write K\n' "$(date -u +%FT%TZ)" > .planning/phases/01-slugify/01-VERIFICATION.md; sleep 5`)
4. Call the Agent tool (subagent_type general-purpose, model haiku) with the prompt "This is gap-closure round 2 for phase 1 (/gsd-plan-phase 1 --gaps). Reply with the single word pong." and wait for its result.
5. Write K=3 exactly as in step 3 (including `sleep 5`). Then call the same Agent tool again with the same prompt. A denial is expected here; if it is denied, carry on.
6. Run `driverctl show`.
7. Write K=4 exactly as in step 3, then run `sleep 90`. The launcher is expected to stop this session during the sleep.
8. Run `driverctl log end --reason stop`.
