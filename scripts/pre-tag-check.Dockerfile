# pre-tag-check.Dockerfile — an `ubuntu-24.04` LOOKALIKE for
# `./scripts/pre-tag-check.sh --container`.
#
# WHY THIS IMAGE EXISTS. A bare local pre-tag run cannot certify the publish
# job, because the developer machine differs from the runner in the place that
# has blocked publication: GSD is installed here and absent there, so the
# router-conformance oracle test (which fails by design when the oracle is
# absent) passes locally on precisely the trees CI rejects. This image
# reproduces the runner closely enough that a green run in here is evidence
# about CI. The ambient git and gh are matched for runner fidelity, not because
# any test pins a version.
#
# EVERY LINE BELOW MIRRORS A MEASURED RUNNER FACT. When one of them stops
# matching `ubuntu-24.04`, this image starts lying, so say WHY each line is
# here and not just what it installs.

FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

# `git` from ppa:git-core/ppa rather than the Ubuntu archive, because the
# archive's git is years behind the runner's. The PPA yields git 2.55.0, which
# is the ambient git the ubuntu-24.04 runner image lists (the runner-images
# readme). This is RUNNER FIDELITY: the surviving suites shell out to a real
# git with version-sensitive flags, so the image should run the git the runner
# runs. No test pins a git version any more (Phase 31 retired the witness).
#
# The rest is the ordinary build toolchain plus the small utilities the gates
# and the test suite shell out to.
RUN apt-get update -qq \
 && apt-get install -y -qq --no-install-recommends \
      software-properties-common curl ca-certificates gnupg \
 && add-apt-repository -y ppa:git-core/ppa \
 && apt-get update -qq \
 && apt-get install -y -qq --no-install-recommends \
      git build-essential pkg-config libssl-dev jq unzip xz-utils procps coreutils tar \
 && curl -fsSL https://deb.nodesource.com/setup_24.x | bash - \
 && apt-get install -y -qq --no-install-recommends nodejs \
 && rm -rf /var/lib/apt/lists/*

# Rust. `stable` with clippy covers gates 3-5 and 7 (gate 7's foreign rustup
# targets are added in the LAST layer below); the 1.89.0 pin is here so gate 2
# — the MSRV gate — can actually run in the container instead of SKIPPING.
# 1.89.0 MIRRORS THE `dtolnay/rust-toolchain@1.89.0` REF in release.yml's msrv
# job. If the manifest's rust-version floor moves, BOTH move: this line and
# that ref. Gate 2 derives the floor from cargo metadata, so it will report the
# mismatch as a SKIP rather than silently certifying the wrong floor.
#
# RUSTUP_HOME/CARGO_HOME live under /usr/local and are world-writable because
# the gate run passes the INVOKING user's uid through (see below), which is not
# root and not necessarily uid 1000.
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN curl -fsSL https://sh.rustup.rs | sh -s -- -y --no-modify-path \
      --default-toolchain stable --profile minimal --component clippy \
 && rustup toolchain install 1.89.0 --profile minimal \
 && chmod -R a+w /usr/local/rustup /usr/local/cargo

# A uid-1000 user with a HOME that is DELIBERATELY FREE OF `.claude`.
#
# THE IMAGE MUST NOT BAKE GSD IN. The entire point of the rehearsal is that the
# conformance oracle is genuinely ABSENT until the run's own provisioning step
# — the same scripts/install-conformance-oracle.sh the publish job calls — puts
# it there. An image with GSD pre-installed would go green while telling you
# nothing whatsoever about a runner that has none.
#
# `ubuntu:24.04` ALREADY SHIPS UID 1000 as the user `ubuntu`, so a bare
# `useradd -u 1000` dies with "UID 1000 is not unique". Delete it first.
RUN (userdel -r ubuntu 2>/dev/null || true) \
 && useradd -m -u 1000 -s /bin/bash runner

# 0777 on the HOME, because the run passes `-u $(id -u):$(id -g)` so that files
# written into the bind-mounted repo keep the developer's ownership — and that
# uid is not guaranteed to be 1000 on every machine. Without this, a developer
# whose uid is 1001 gets a HOME they cannot write and the npm install fails.
RUN chmod 0777 /home/runner

# `gh` — PREINSTALLED ON ubuntu-24.04, from the SAME apt repo and the same
# keyring path GitHub's own runner image uses, which yields gh 2.101.0.
#
# READ THIS BEFORE REMOVING THE LINE TO SAVE A LAYER. The launcher's push step
# opens pull requests through `gh`, and the suites that exercise it see the gh
# that is actually present. An image WITHOUT gh differs from the runner in a
# way that can manufacture failures that are not runner failures and send the
# next investigation down a false trail. It already did exactly that once, when
# the (since deleted) policy.rs grammar probes ran against it.
RUN curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg \
      -o /usr/share/keyrings/githubcli-archive-keyring.gpg \
 && chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg \
 && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" \
      > /etc/apt/sources.list.d/github-cli.list \
 && apt-get update -qq \
 && apt-get install -y -qq --no-install-recommends gh \
 && rm -rf /var/lib/apt/lists/*

# The rustup targets gate 7 needs, and ONLY those: this MIRRORS the "Install
# stable toolchain with the target" step of release.yml's cross-target-check job
# (dtolnay/rust-toolchain@stable with `targets: ${{ matrix.target }}`), which
# adds rust-std for each matrix target to stable.
#
# THE LIST IS NOT WRITTEN HERE. `./scripts/pre-tag-check.sh --container` reads it
# from release.yml and passes it in as this build arg, so the image cannot drift
# from the matrix; a copy in this file would be a second source of truth. It is
# also why the layer is LAST: changing the list rebuilds only this layer.
#
# v2.2.0 and v2.3.0 both died in that job on a Windows-only dead-code lint that
# no other gate could see. Without these targets the gate has nothing to lint.
#
# The RUN refuses an empty arg (a bare `docker build` of this file would
# silently produce an image gate 7 then FAILS in), and re-checks the charset
# because the value is deliberately left UNQUOTED so the shell splits it into
# one rustup argument per target. chmod re-applies the toolchain layer's
# a+w: the unprivileged gate-run uid must be able to use what root just wrote.
ARG PRE_TAG_CROSS_TARGETS
RUN if [ -z "$PRE_TAG_CROSS_TARGETS" ]; then \
      echo "PRE_TAG_CROSS_TARGETS is empty: build this image only via ./scripts/pre-tag-check.sh --container, which derives it from release.yml" >&2; \
      exit 1; \
    fi \
 && if printf '%s' "$PRE_TAG_CROSS_TARGETS" | grep -q '[^a-z0-9_. -]'; then \
      echo "PRE_TAG_CROSS_TARGETS may contain only [a-z0-9_.-] and spaces" >&2; \
      exit 1; \
    fi \
 && rustup target add --toolchain stable $PRE_TAG_CROSS_TARGETS \
 && chmod -R a+w /usr/local/rustup
