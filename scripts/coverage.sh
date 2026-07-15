#!/usr/bin/env bash
set -euo pipefail

# Coverage gate for v1 exit criterion (ROADMAP.md §"v1 Exit Criteria").
# Fails if craft-kernel (engine core) line coverage drops below threshold.
#
# Usage:
#   scripts/coverage.sh           # default threshold 80
#   scripts/coverage.sh 75        # custom threshold
#   scripts/coverage.sh --html    # write HTML report to target/llvm-cov/html
#   scripts/coverage.sh --json    # emit JSON to stdout (CI consumption)

threshold="${1:-80}"
format="summary"

case "${1:-}" in
    --html) format="html"; shift ;;
    --json) format="json"; shift; threshold="${1:-80}" ;;
esac

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
    echo "error: cargo-llvm-cov not installed. Install with:" >&2
    echo "  cargo install cargo-llvm-cov --locked" >&2
    exit 1
fi

report_path="target/llvm-cov/coverage.json"
mkdir -p "$(dirname "${report_path}")"

cargo llvm-cov --package craft-kernel \
    --summary-only \
    --json \
    --output-path "${report_path}" >/dev/null

if [[ "${format}" == "json" ]]; then
    cat "${report_path}"
    exit 0
fi

line_cover=$(python3 -c "
import json, sys
data = json.load(open('${report_path}'))
totals = data['data'][0]['totals']
lines = totals['lines']
print(f\"{lines['covered']}/{lines['count']} = {lines['percent']:.2f}%\")
")

echo "craft-kernel line coverage: ${line_cover}"
echo "threshold: ${threshold}%"

covered=$(python3 -c "
import json
data = json.load(open('${report_path}'))
totals = data['data'][0]['totals']
print(int(totals['lines']['percent']))
")

if (( covered < threshold )); then
    echo "FAIL: coverage ${covered}% < threshold ${threshold}%" >&2
    exit 1
fi

echo "PASS"

if [[ "${format}" == "html" ]]; then
    cargo llvm-cov --package craft-kernel --html
    echo "HTML report: target/llvm-cov/html/index.html"
fi