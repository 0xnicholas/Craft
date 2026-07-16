#!/usr/bin/env bash
set -euo pipefail

# Coverage gate for v1 exit criterion (ROADMAP.md §"v1 Exit Criteria").
# Measures craft-kernel and craft-editor line coverage on PRODUCTION code only —
# inline `#[cfg(test)]` modules are excluded so that adding more tests doesn't
# artificially lower the gate.
#
# Usage:
#   scripts/coverage.sh           # default threshold 65
#   scripts/coverage.sh 75        # custom threshold
#   scripts/coverage.sh --html    # write HTML report to target/llvm-cov/html
#   scripts/coverage.sh --json    # emit JSON to stdout (CI consumption)

threshold="${1:-65}"
format="summary"

case "${1:-}" in
    --html) format="html"; shift ;;
    --json) format="json"; shift; threshold="${1:-65}" ;;
esac

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
    echo "error: cargo-llvm-cov not installed. Install with:" >&2
    echo "  cargo install cargo-llvm-cov --locked" >&2
    exit 1
fi

lcov_path="target/llvm-cov/coverage.lcov"
mkdir -p "$(dirname "${lcov_path}")"

cargo llvm-cov --package craft-kernel \
    -p craft-editor \
    --lcov \
    --output-path "${lcov_path}" >/dev/null

python3 - <<PY > "${lcov_path}.filtered"
import re, sys

lcov_input = "${lcov_path}"
src_root = "."

def find_test_ranges(path):
    """Return list of (start_line, end_line) for inline test modules in path."""
    try:
        with open(path) as f:
            lines = f.readlines()
    except FileNotFoundError:
        return []
    ranges = []
    in_test = False
    depth = 0
    test_start = None
    for i, line in enumerate(lines, 1):
        if not in_test:
            if "#[cfg(test)]" in line and i + 1 < len(lines) and re.match(r"^\s*mod\s+tests\s*\{", lines[i]):
                in_test = True
                test_start = i + 1
                depth = 1
            elif re.match(r"^\s*mod\s+tests\s*\{", line):
                in_test = True
                test_start = i
                depth = 1
        else:
            depth += line.count("{") - line.count("}")
            if depth <= 0:
                ranges.append((test_start, i))
                in_test = False
    return ranges

with open(lcov_input) as f:
    current_file = None
    test_ranges = []
    for raw in f:
        line = raw.rstrip("\n")
        if line.startswith("SF:"):
            current_file = line[3:]
            test_ranges = find_test_ranges(current_file)
            print(line)
        elif line.startswith("DA:") and test_ranges:
            m = re.match(r"DA:(\d+),", line)
            if m:
                ln = int(m.group(1))
                if any(s <= ln <= e for s, e in test_ranges):
                    continue
            print(line)
        else:
            print(line)
PY

read line_cover covered total < <(LCOV_PATH="${lcov_path}.filtered" python3 - <<'PY'
import os, re

lcov_path = os.environ["LCOV_PATH"]
src_root = "."

def find_test_ranges(path):
    try:
        with open(path) as f:
            lines = f.readlines()
    except FileNotFoundError:
        return []
    ranges = []
    in_test = False
    depth = 0
    test_start = None
    for i, line in enumerate(lines, 1):
        if not in_test:
            stripped = line.lstrip()
            if stripped.startswith("mod tests") and "{" in stripped:
                in_test = True
                test_start = i
                depth = 1
                for ch in stripped[stripped.index("{")+1:]:
                    if ch == "{":
                        depth += 1
                    elif ch == "}":
                        depth -= 1
                if depth <= 0:
                    ranges.append((test_start, i))
                    in_test = False
        else:
            depth += line.count("{") - line.count("}")
            if depth <= 0:
                ranges.append((test_start, i))
                in_test = False
    return ranges

total_lines = 0
covered_lines = 0

with open(lcov_path) as f:
    current_file = None
    test_ranges = []
    for raw in f:
        line = raw.rstrip()
        if line.startswith("SF:"):
            current_file = line[3:]
            test_ranges = find_test_ranges(current_file)
        elif line.startswith("DA:") and current_file:
            m = re.match(r"DA:(\d+),(\d+)", line)
            if m:
                ln, count = int(m.group(1)), int(m.group(2))
                if any(s <= ln <= e for s, e in test_ranges):
                    continue
                total_lines += 1
                if count > 0:
                    covered_lines += 1

if total_lines == 0:
    print("0.00 0 0")
else:
    pct = 100.0 * covered_lines / total_lines
    print(f"{pct:.2f} {covered_lines} {total_lines}")
PY
)

if [[ "${format}" == "json" ]]; then
    echo "{\"coverage_path\": \"${lcov_path}.filtered\", \"lines\": \"${line_cover:-unknown}\", \"covered\": ${covered:-0}, \"total\": ${total:-0}}"
    exit 0
fi

if [[ -z "${line_cover}" ]]; then
    echo "error: failed to compute coverage" >&2
    exit 2
fi

echo "craft-kernel,craft-editor production line coverage: ${line_cover}% (${covered}/${total} lines)"
echo "threshold: ${threshold}%"

if (( covered < threshold )); then
    echo "FAIL: coverage ${covered}% < threshold ${threshold}%" >&2
    exit 1
fi

echo "PASS"

if [[ "${format}" == "html" ]]; then
    cargo llvm-cov --package craft-kernel -p craft-editor --html
    echo "HTML report: target/llvm-cov/html/index.html"
fi