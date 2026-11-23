#!/bin/sh
# The gate's test step (`task test`, locally and in CI). AGENTS.md, "The gate", gives the layout and
# the budgets this step holds.
#
#   1. Builds every test target of the workspace once and takes the `ekr` binary of that build for
#      the SDK's tests (`EKR_SDK_TEST_EKR`), so no test builds it again.
#   2. Runs every test under cargo-nextest, one process per test (.config/nextest.toml): a test
#      over 60 s is killed and fails; the run stops at the interim ceiling named there.
#   3. Runs the doc tests, which nextest does not run.
#   4. Prints the ten slowest tests and the run time against the budget of 120 s.
#
# Exits non-zero when a build, a test or a doc test failed.
set -u

budget=120

target=$(cargo metadata --format-version 1 --no-deps --locked |
    sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')
if [ -z "$target" ]; then
    echo "test-step: cargo metadata named no target directory" >&2
    exit 1
fi
work=$target/ekr-test-step
mkdir -p "$work"

started=$(date +%s)
if ! cargo test --workspace --locked --no-run --message-format=json-render-diagnostics \
    >"$work/build.json"; then
    echo "test-step: the test build failed" >&2
    exit 1
fi
built=$(date +%s)

# The binary itself, not the test harness cargo also builds from its source (`"test":true`).
executable=$(grep '"reason":"compiler-artifact"' "$work/build.json" |
    grep '"kind":\["bin"\]' | grep '"name":"ekr","src_path"' | grep '"test":false}' |
    sed -n 's/.*"executable":"\([^"]*\)".*/\1/p' | tail -n 1)
if [ -z "$executable" ] || [ ! -x "$executable" ]; then
    echo "test-step: the test build named no ekr executable" >&2
    exit 1
fi
cp "$executable" "$work/ekr"
EKR_SDK_TEST_EKR=$work/ekr
export EKR_SDK_TEST_EKR

cargo nextest run --workspace --locked --no-fail-fast
tests=$?
cargo test --workspace --locked --doc
docs=$?
finished=$(date +%s)

junit=$target/nextest/default/junit.xml
echo
echo "The ten slowest tests (seconds, binary, test), from $junit:"
if [ -f "$junit" ]; then
    awk '/<testcase / {
        name = ""; binary = ""; time = ""
        if (match($0, / name="[^"]*"/)) name = substr($0, RSTART + 7, RLENGTH - 8)
        if (match($0, / classname="[^"]*"/)) binary = substr($0, RSTART + 12, RLENGTH - 13)
        if (match($0, / time="[^"]*"/)) time = substr($0, RSTART + 7, RLENGTH - 8)
        print time, binary, name
    }' "$junit" | sort -rn | head -n 10
else
    echo "  (no report: the run did not start)"
fi

build_time=$((built - started))
run_time=$((finished - built))
echo
echo "test step: build ${build_time} s; tests and doc tests ${run_time} s against a budget of ${budget} s"
if [ "$run_time" -gt "$budget" ]; then
    echo "test step: over the budget by $((run_time - budget)) s (AGENTS.md, \"The gate\", names what remains)"
fi

if [ "$tests" -ne 0 ] || [ "$docs" -ne 0 ]; then
    echo "test step: failed (nextest exit $tests, doc tests exit $docs)" >&2
    exit 1
fi
