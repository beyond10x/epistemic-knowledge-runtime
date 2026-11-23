#!/bin/sh
# The gate's test step (`task test`, locally and in CI). AGENTS.md, "The gate", gives the layout and
# the budgets this step holds.
#
#   1. Builds every test target of the workspace once and takes the `ekr` binary of that build for
#      the SDK's tests (`EKR_SDK_TEST_EKR`), so no test builds it again.
#   2. Starts one headless Chromium for every browser case and names it in `EKR_TEST_BROWSER`; each
#      case opens a browser context of its own in it (crates/ekr/tests/support/shared_browser.rs).
#   3. Runs every test under cargo-nextest, one process per test (.config/nextest.toml): a test
#      over 60 s is killed and fails, and the run is stopped at the step's budget.
#   4. Runs the doc tests, which nextest does not run.
#   5. Prints the ten slowest tests, the browser cases' share and the run time against the budget
#      of 120 s.
#
# Exits non-zero when a build, a test or a doc test failed, or when the tests and doc tests ran
# longer than the budget.
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

# One Chromium for the whole run, with the flags each case's own launch carries: no network for the
# viewer's libraries (the cases serve them), no component update, no background networking.
browser_pid=
stop_browser() {
    if [ -n "$browser_pid" ]; then
        kill "$browser_pid" 2>/dev/null
        wait "$browser_pid" 2>/dev/null
        browser_pid=
    fi
}
trap stop_browser EXIT
trap 'stop_browser; exit 130' INT TERM

browser=${EKR_VIEW_BROWSER:-}
if [ -z "$browser" ]; then
    for shell in "${HOME:-/nonexistent}"/.cache/ms-playwright/chromium_headless_shell-*/chrome-headless-shell-linux64/chrome-headless-shell; do
        if [ -x "$shell" ]; then
            browser=$shell
        fi
    done
fi
if [ -n "$browser" ] && [ -x "$browser" ]; then
    profile=$work/browser-profile
    rm -rf "$profile"
    mkdir -p "$profile"
    nice -n 10 "$browser" \
        --headless \
        --use-angle=swiftshader \
        --enable-unsafe-swiftshader \
        --no-sandbox \
        --no-first-run \
        --disable-component-update \
        --disable-background-networking \
        --disable-extensions \
        "--host-resolver-rules=MAP cdn.jsdelivr.net ~NOTFOUND" \
        --remote-debugging-port=0 \
        "--user-data-dir=$profile" \
        about:blank </dev/null >"$work/browser.log" 2>&1 &
    browser_pid=$!
    port=
    waited=0
    while [ -z "$port" ] && [ "$waited" -lt 30 ]; do
        if [ -s "$profile/DevToolsActivePort" ]; then
            port=$(sed -n '1{/^[0-9][0-9]*$/p;}' "$profile/DevToolsActivePort")
        fi
        if [ -z "$port" ]; then
            if ! kill -0 "$browser_pid" 2>/dev/null; then
                break
            fi
            sleep 1
            waited=$((waited + 1))
        fi
    done
    if [ -z "$port" ]; then
        echo "test-step: the shared Chromium announced no DevTools port; its log:" >&2
        cat "$work/browser.log" >&2
        stop_browser
        exit 1
    fi
    EKR_TEST_BROWSER=127.0.0.1:$port
    export EKR_TEST_BROWSER
    echo "test step: Chromium started once for every browser case (pid $browser_pid, port $port, $browser)"
else
    echo "test step: no headless Chromium (EKR_VIEW_BROWSER unset, none under ~/.cache/ms-playwright); each browser case starts its own or skips"
fi

cargo nextest run --workspace --locked --no-fail-fast
tests=$?
stop_browser
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
    # The browser cases: the six files whose cases drive a browser. Their span runs from the first
    # case's start to the last case's end.
    awk '/<testcase / && / classname="ekr::main"/ &&
        / name="(view_page|search_live|adversary_p_page|adversary_page_stream_1|adversary_viewer_compact|adversary_viewer_follow_ups)::/ {
        time = 0; at = -1
        if (match($0, / time="[^"]*"/)) time = substr($0, RSTART + 7, RLENGTH - 8) + 0
        if (match($0, / timestamp="[^"]*"/)) {
            stamp = substr($0, RSTART + 12, RLENGTH - 13)
            day = substr(stamp, 9, 2) + 0
            split(substr(stamp, 12, 12), clock, ":")
            at = day * 86400 + clock[1] * 3600 + clock[2] * 60 + clock[3]
        }
        cases++; sum += time
        if (at >= 0) {
            if (first == "" || at < first) first = at
            if (last == "" || at + time > last) last = at + time
        }
    }
    END {
        if (cases == 0) { print "browser cases: none ran"; exit }
        if (first == "") span = "unknown"; else span = sprintf("%.1f s", last - first)
        printf "browser cases: %d, %.1f s summed, %s from the first start to the last end\n", cases, sum, span
    }' "$junit"
else
    echo "  (no report: the run did not start)"
fi

build_time=$((built - started))
run_time=$((finished - built))
echo
echo "test step: build ${build_time} s; tests and doc tests ${run_time} s against a budget of ${budget} s"
over=0
if [ "$run_time" -gt "$budget" ]; then
    echo "test step: over the budget by $((run_time - budget)) s (AGENTS.md, \"The gate\")" >&2
    over=1
fi

if [ "$tests" -ne 0 ] || [ "$docs" -ne 0 ] || [ "$over" -ne 0 ]; then
    echo "test step: failed (nextest exit $tests, doc tests exit $docs, over the budget: $over)" >&2
    exit 1
fi
