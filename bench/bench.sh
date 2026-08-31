#!/usr/bin/env bash
# Benchmark bitiodine on the first N block files of a blocks directory.
# Records wall time and peak RSS (VmHWM) for the clusterizer and dump-balances
# actions, appending results to bench/results.md.
#
# Usage: ./bench/bench.sh [--blocks-dir DIR] [N]
#   --blocks-dir  path to the bitcoind blocks directory (default: ~/.bitcoin/blocks)
#   N             number of block files to process (default: 500)
#
# Safety: bitcoind must NOT be running against this directory (block files are
# mapped and must not be mutated concurrently).
set -euo pipefail

BLOCKS_DIR="${HOME}/.bitcoin/blocks"
N=500
while [ $# -gt 0 ]; do
    case "$1" in
        --blocks-dir)
            BLOCKS_DIR="$2"
            shift 2
            ;;
        *)
            N="$1"
            shift
            ;;
    esac
done

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RESULTS="$REPO_ROOT/bench/results.md"
cd "$REPO_ROOT"

cargo build --release
BIN="./target/release/bitiodine"

OUT_DIR="$(mktemp -d)"
trap 'rm -rf "$OUT_DIR"' EXIT

TIMESTAMP="$(date '+%Y-%m-%d %H:%M')"
COMMIT="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
HEADER="## $TIMESTAMP — commit $COMMIT — first $N block file(s) from $BLOCKS_DIR"
echo "$HEADER" >>"$RESULTS"
echo "" >>"$RESULTS"
echo "| action | wall time (s) | peak RSS (MiB) |" >>"$RESULTS"
echo "|---|---|---|" >>"$RESULTS"

for action in clusterizer dump-balances; do
    echo "Benchmarking $action on $N block file(s)..."
    START_NS="$(date +%s%N)"
    "$BIN" -b "$BLOCKS_DIR" -a "$action" --max-blocks "$N" -o "$OUT_DIR/$action.csv" \
        2>"$OUT_DIR/$action.log"
    END_NS="$(date +%s%N)"

    WALL_S="$(awk -v s="$START_NS" -v e="$END_NS" 'BEGIN {printf "%.1f", (e - s) / 1e9}')"
    # Peak RSS logged by the binary itself (VmHWM from /proc/self/status, KiB).
    RSS_MIB="$(grep -o 'Peak RSS (VmHWM)[^:]*: [0-9.]* MiB' "$OUT_DIR/$action.log" \
        | tail -1 | grep -o '[0-9.]* MiB' | awk '{print $1}')"
    if [ -z "$RSS_MIB" ]; then
        RSS_MIB="n/a"
    fi

    echo "| $action | $WALL_S | $RSS_MIB |" >>"$RESULTS"
    echo "  $action: ${WALL_S}s, ${RSS_MIB} MiB"
done

echo "" >>"$RESULTS"
echo "Results appended to $RESULTS"
