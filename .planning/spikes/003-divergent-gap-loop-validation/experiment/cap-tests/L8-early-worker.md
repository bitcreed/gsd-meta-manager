You are a deliberately SCRIPTED test driver (cap test L8, the Run 1b shape: a worker that ends its turn while its own background agent is still running). Do the steps below strictly in order, never skip or reorder a step, and report each step's result in one line at the very end. Your working directory is the project root.

0. Run `driverctl log start --mode headless --loop-mode loop`.
1. Run `driverctl log dispatched --item I-01 --step "/gsd-plan-phase 1" --round 0 --phase 1`.
2. Call the Agent tool exactly once with subagent_type general-purpose, model haiku and run_in_background true, and this prompt: "Start exactly one Agent call of your own (subagent_type general-purpose, model haiku, run_in_background true) whose prompt tells it to run the Bash command `sleep 45` and then reply with the single word pong. Then reply with the single word waiting and end your turn at once, without waiting for that agent. If you are messaged again later, reply with exactly two lines: `STATUS: done` and `ITEM: I-01`." Remember the worker's agentId from the "Async agent launched" result.
3. End your turn now, saying only `waiting for the worker`.

Standing instructions for every message you receive after step 3:
- If a Stop hook message says that a dispatch has no report, do exactly what it says: call SendMessage with `to` set to the SAME agentId you remembered in step 2 and ask that worker to finish and reply with its STATUS report. Never call the Agent tool again, not even if the worker seems gone.
- When a message that starts with `STATUS:` arrives, run `driverctl log report --item I-01 --status done` with that message as the report text on stdin (a heredoc), then run `driverctl log end --reason stop`, and then stop.
- On any other notification (for example the worker's nested agent finishing with `pong`), end your turn again, saying only `waiting`.
