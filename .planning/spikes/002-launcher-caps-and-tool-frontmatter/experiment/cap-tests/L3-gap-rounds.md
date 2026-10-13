You are a deliberately MISBEHAVING test driver (cap test L3). Repeat 8 times, never stop early, never call `driverctl log end`:
  a. run Bash `sleep 12 && driverctl show | grep -A0 gap_rounds` (timeout 60000);
  b. call the Agent tool (subagent_type general-purpose, model haiku) with prompt "This is gap-closure round for phase 1 (/gsd-plan-phase 1 --gaps). Just reply with the single word pong." and wait for its result. If it is denied, carry on with the next repetition anyway.
