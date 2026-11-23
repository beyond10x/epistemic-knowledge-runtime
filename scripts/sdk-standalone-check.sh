#!/bin/sh
# Build the SDK the way an application outside this workspace does (story:sdk-standalone-build).
#
# A Cargo workspace of its own depends on crates/ekr-sdk by path. Cargo applies only the root
# workspace's [patch], so nothing this workspace's Cargo.toml selects reaches it: every crate the
# SDK needs must come from the SDK's own declared dependencies, as it does for an application that
# names a release tag. `task sdk-standalone-check` runs this, outside the test step.
#
# Offline: registry crates come from the local cache (`cargo fetch --locked` fills it, as CI does
# before `task check`). The consumer's lock starts as a copy of this workspace's Cargo.lock, so it
# resolves the versions the gate tests and needs no index.
#
#   EKR_SDK_STANDALONE_DIR  where the consumer and its build go
#                           (default: ${CARGO_TARGET_DIR:-target}/sdk-standalone)
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
target=${CARGO_TARGET_DIR:-target}
case "$target" in
/*) ;;
*) target=$root/$target ;;
esac
dir=${EKR_SDK_STANDALONE_DIR:-$target/sdk-standalone}
consumer=$dir/consumer

rm -rf "$consumer"
mkdir -p "$consumer/src"

printf '%s\n' \
    '[package]' \
    'name = "ekr-sdk-standalone-consumer"' \
    'version = "0.0.0"' \
    'edition = "2021"' \
    'publish = false' \
    '' \
    '[dependencies]' \
    "ekr-sdk = { path = \"$root/crates/ekr-sdk\" }" \
    '' \
    '[workspace]' >"$consumer/Cargo.toml"

printf '%s\n' \
    'fn main() {' \
    '    let options = ekr_sdk::session::SessionOptions::default();' \
    '    let _ = std::hint::black_box(options);' \
    '}' >"$consumer/src/main.rs"

cp "$root/Cargo.lock" "$consumer/Cargo.lock"

start=$(date +%s)
status=0
# From the repository root, so rustup takes the toolchain rust-toolchain.toml pins.
CARGO_TARGET_DIR=$dir/target cargo check --offline --manifest-path "$consumer/Cargo.toml" ||
    status=$?
end=$(date +%s)

if [ "$status" -ne 0 ]; then
    echo "sdk-standalone-check: an application outside this workspace cannot build ekr-sdk (cargo exit $status, $((end - start)) s)" >&2
    exit 1
fi
echo "sdk-standalone-check: ekr-sdk builds in a standalone workspace ($((end - start)) s)"
