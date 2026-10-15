#!/usr/bin/env bash
# Build the Linux release binary in the Debian 11 image, then package it.
#
#   packaging/linux/build-release.sh      → target/container/release/snipdeck
#                                           and dist/<version>/ (package.sh)
#
# The image comes from Containerfile.release and is built on first use. The
# source is mounted read-only; the target directory stays in
# target/container, beside the host's own, and the host's cargo registry is
# shared so nothing is downloaded twice.
#
# On this machine every build goes through the build queue: CARGO_RUN=derle.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$here"
image=snipdeck-release
podman image exists "$image" ||
    podman build -f packaging/linux/Containerfile.release -t "$image" packaging/linux
mkdir -p target/container "${CARGO_HOME:-$HOME/.cargo}/registry" "${CARGO_HOME:-$HOME/.cargo}/git"
${CARGO_RUN:-} podman run --rm --userns=keep-id \
    -v "$here":/src:ro \
    -v "$here/target/container":/target \
    -v "${CARGO_HOME:-$HOME/.cargo}/registry":/cargo/registry \
    -v "${CARGO_HOME:-$HOME/.cargo}/git":/cargo/git \
    "$image" cargo build --release --locked
packaging/linux/package.sh
