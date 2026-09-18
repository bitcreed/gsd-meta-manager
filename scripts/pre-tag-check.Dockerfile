# pre-tag-check.Dockerfile — an `ubuntu-latest` LOOKALIKE for
# `./scripts/pre-tag-check.sh --container`.
#
# WHY THIS IMAGE EXISTS. A bare local pre-tag run cannot certify the publish
# job, because the developer machine differs from the runner in exactly the two
# places that have blocked publication: GSD is installed here and absent there,
# and the ambient git is a different version. Both of those tests therefore
# pass locally on precisely the trees CI rejects. This image reproduces the
# runner closely enough that a green run in here is evidence about CI.
#
# EVERY LINE BELOW MIRRORS A MEASURED RUNNER FACT. When one of them stops
# matching `ubuntu-latest`, this image starts lying, so say WHY each line is
# here and not just what it installs.

FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

# `git` from ppa:git-core/ppa rather than the Ubuntu archive, because the
# archive's git is years behind the runner's. The PPA yields git 2.55.0 — the
# version CONFIG_SECTION_CONSTANTS_DERIVED_AGAINST_GIT_VERSION in
# src/envelope/policy.rs records AND the version the runner's ambient git
# carries. That is what lets the version-witness test be validated in here the
# way CI validates it; on a developer machine with an older git it fails, and
# pre-tag-check.sh's reconciliation banner exists to explain that away.
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

# Rust. `stable` with clippy covers gates 3-5; the 1.88.0 pin is here so gate 2
# — the MSRV gate — can actually run in the container instead of SKIPPING.
# 1.88.0 MIRRORS THE `dtolnay/rust-toolchain@1.88.0` REF in release.yml's msrv
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
 && rustup toolchain install 1.88.0 --profile minimal \
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

# `gh` — PREINSTALLED ON ubuntu-latest, from the SAME apt repo and the same
# keyring path GitHub's own runner image uses, which yields gh 2.101.0.
#
# READ THIS BEFORE REMOVING THE LINE TO SAVE A LAYER. The two gh_grammar_probe
# tests in src/envelope/policy.rs pin the grammar of `gh` invocations against
# the gh that is actually present. An image WITHOUT gh manufactures two
# failures that are not runner failures and sends the next investigation down a
# false trail. It already did exactly that once.
RUN curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg \
      -o /usr/share/keyrings/githubcli-archive-keyring.gpg \
 && chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg \
 && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" \
      > /etc/apt/sources.list.d/github-cli.list \
 && apt-get update -qq \
 && apt-get install -y -qq --no-install-recommends gh \
 && rm -rf /var/lib/apt/lists/*
