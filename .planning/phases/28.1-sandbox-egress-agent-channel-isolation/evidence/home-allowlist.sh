#!/bin/bash
# Run "$@" inside a bwrap that tmpfs-masks $HOME and ro re-binds an allowlist.
S=/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/110e6e69-666b-4cc0-8133-142b5fa27a61/scratchpad/home
H=$HOME
PROJ=${PROJ:-$S/proj}
CFG=${CFG:-$S/cfg}
args=(bwrap --ro-bind / / --dev /dev --proc /proc --tmpfs /tmp --tmpfs "$H")
ro() { for p in "$@"; do args+=(--ro-bind-try "$p" "$p"); done; }
ro "$H/.local/bin/claude" "$H/.local/share/claude" \
   "$H/.local/share/nvm" "$H/.nvm" \
   "$H/.cargo/bin" "$H/.cargo/registry" "$H/.cargo/git" "$H/.cargo/config.toml" "$H/.cargo/env" "$H/.rustup" \
   "$H/.gitconfig" "$H/.config/git" \
   "$H/.claude/agents" "$H/.claude/commands" "$H/.claude/skills" "$H/.claude/gsd-core" \
   "$H/.claude/get-shit-done" "$H/.claude/hooks" \
   "$H/.gsd" "$H/.gsd-capabilities.json" "$H/.config/rtk"
for p in ${EXTRA_RO:-}; do args+=(--ro-bind "$p" "$p"); done
args+=(--tmpfs "$H/.cache" --tmpfs "$H/.local/share/rtk")
args+=(--bind "$PROJ" "$PROJ" --bind "$CFG" "$CFG")
[ -f "$CFG/settings.json" ] && args+=(--ro-bind "$CFG/settings.json" "$CFG/settings.json")
args+=(--unshare-pid --new-session --die-with-parent --chdir "$PROJ" --)
exec env CLAUDE_CONFIG_DIR="$CFG" "${args[@]}" "$@"
