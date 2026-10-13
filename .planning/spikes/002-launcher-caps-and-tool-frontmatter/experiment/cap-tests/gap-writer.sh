#!/usr/bin/env bash
# Host-side stand-in for gsd-verifier: rewrites 01-VERIFICATION.md with status gaps_found every N s.
# Usage: gap-writer.sh <project> <count> <interval-s>
P="$1"; N="$2"; I="$3"; F="$P/.planning/phases/01-slugify/01-VERIFICATION.md"
for r in $(seq 1 "$N"); do
  sleep "$I"
  printf -- '---\nphase: 01-slugify\nverified: %s\nstatus: gaps_found\nscore: 2/3\n---\nround %s\n' "$(date -u +%FT%TZ)" "$r" > "$F"
  echo "wrote gaps_found #$r"
done
