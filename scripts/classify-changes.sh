#!/bin/sh
# Classify a pull request by its changed files. `classify` below is the one path list of this
# repository; .github/workflows/correctness.yml runs this file, and
# crates/ekr/tests/gate_layout.rs holds both sides of the list.
#
#   classify-changes.sh              reads changed paths, one per line, on stdin
#   classify-changes.sh --diff BASE  reads `git diff --no-renames --name-only BASE...HEAD`
#
# Prints one of:
#   planning-only   every path is planning or prose: no build, no test; the planning store validates
#   docs-site-only  the built site and planning or prose: the site check, no build or test of code
#   code            anything else; also any failure and an empty list
set -eu

# Class of one path: planning, site or code. First match wins.
#   a document a test or the product reads   code      gate_layout.rs finds them and holds this list
#   .engineering/*                           planning  the planning store; plan-check still runs
#   website/*                                site      the built documentation site
#   docs/*.md and *.md in the root           planning  prose nothing builds or reads
#   everything else                          code      a .md deeper in the tree is a crate's fixture
classify() {
    case "$1" in
    AGENTS.md | README.md | docs/cli.md | docs/sdk.md | docs/roadmap.md | \
        docs/epistemic-knowledge-runtime-design.md) echo code ;;
    .engineering/*) echo planning ;;
    website/*) echo site ;;
    docs/*.md) echo planning ;;
    */*) echo code ;;
    *.md) echo planning ;;
    *) echo code ;;
    esac
}

answer() {
    seen=0
    site=0
    while IFS= read -r path; do
        [ -n "$path" ] || continue
        seen=1
        case "$(classify "$path")" in
        planning) ;;
        site) site=1 ;;
        *)
            echo code
            return 0
            ;;
        esac
    done
    if [ "$seen" = 0 ]; then
        echo code
    elif [ "$site" = 1 ]; then
        echo docs-site-only
    else
        echo planning-only
    fi
}

if [ "${1:-}" = "--diff" ]; then
    list=$(git -c core.quotepath=off diff --no-renames --name-only "${2:-}...HEAD" 2>/dev/null) || {
        echo code
        exit 0
    }
    printf '%s\n' "$list" | answer
else
    answer
fi
