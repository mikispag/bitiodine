# BitIodine memory benchmarks

Peak RSS is measured as `VmHWM` (process high-water mark) read by the binary itself
from `/proc/self/status` — identical to GNU `time -v` "Maximum resident set size".
Corpus: first N block files of the local mainnet blocks dir, `--max-blocks N`.
Corpus start: commit 04897da lineage; benchmark machine: 60 GB RAM, bitcoind stopped,
XOR obfuscation present (xor.dat).

## Baseline (before any optimization, after Phase 0 instrumentation, commit d5313c3)

Corpus: first 500 block files (XOR-obfuscated), height range roughly 0 to ~250k.
Clusterizer: OutputMap 8,485,479 entries at end of walk; 95,776,936 addresses clustered;
CSV 6.3 GB. Walk ≈ 6.5 min, `done()` + CSV ≈ 6.5 min (double serialization + global sort).

| action | wall time (s) | peak RSS (MiB) |
|---|---|---|
| clusterizer | 821.7 | 23186.8 |
| dump-balances | 538.7 | 14987.0 |

Notes: the CSV String transient built in `done()` and the global sort buffer in
`write_csv` dominate the tail of the clusterizer run; OutputMap+DisjointSet maps
dominate the rest. Peak is at end-of-run, confirming the memory model in PLAN.md.

## Phase 1 — drop CSV double-serialization and sort transients (commit b8b44af)

N=100 corpus. Content verified: sorted CSV sha256 identical to N=100 baseline.

| action | wall (s) | peak RSS (MiB) | N=100 baseline |
|---|---|---|---|
| clusterizer | 67.5 | 2908.2 | 121.5 s / 3491.4 MiB |
| dump-balances | 66.3 | 2974.2 | 68.4 s / 2974.3 MiB |

## Phase 2 — CompactAddress + foldhash (commit b4a9353)

N=100 corpus. Content verified: sorted CSV sha256 identical to baseline.

| action | wall (s) | peak RSS (MiB) | N=100 baseline |
|---|---|---|---|
| clusterizer | 62.8 | 2568.5 | 121.5 s / 3491.4 MiB |
| dump-balances | 58.4 | 2300.0 | 68.4 s / 2974.3 MiB |

## Phase 3 — OutputMap [;1] + jemalloc (commit bec2013)

N=100 corpus. Content verified. [;1] vs [;2]: no RSS regression, savings scale
with entry count (kept). jemalloc default-on, `--no-default-features` builds.

| action | wall (s) | peak RSS (MiB) | Phase 2 |
|---|---|---|---|
| clusterizer | 68.1 | 2536.1 | 62.8 s / 2568.5 MiB |
| dump-balances | 63.3 | 2369.6 | 58.4 s / 2300.0 MiB |

## Phase 4 — XOR scratch buffer (commit 07afbbe)

N=100 corpus. Content verified. Same wall/RSS at this size (XOR COW churn was
page-cache-resident anyway); removes per-file COW dirty-page churn.

| action | wall (s) | peak RSS (MiB) | Phase 3 |
|---|---|---|---|
| clusterizer | 62.7 | 2537.0 | 68.1 s / 2536.1 MiB |
| dump-balances | 58.6 | 2369.4 | 63.3 s / 2369.6 MiB |

## Final (all phases) — full 500-file corpus (commit 07afbbe)

Same 500-file corpus as baseline. Identical end-of-walk OutputMap entry count
(8,485,479), identical clustered address count (95,776,936) and balance count
(7,030,395). Content verified: sorted CSV sha256 identical to baseline for both
actions (clusterizer `9286eca9…`, dump-balances `98dc2b87…`).

| action | wall time (s) | peak RSS (MiB) | baseline (s) | baseline (MiB) | Δ wall | Δ RSS |
|---|---|---|---|---|---|---|
| clusterizer | 459.1 | 14839.9 | 821.7 | 23186.8 | -44% | -36% |
| dump-balances | 410.2 | 11530.1 | 538.7 | 14987.0 | -24% | -23% |

Stop criterion note: the ≤½ peak target was calibrated against the full-chain
corpus (~180M UTXO entries), where the OutputMap whale dominates. On this
500-file corpus the UTXO map is only 8.5M entries (~1.5 GiB at baseline) and the
dominant residual is the 95.8M-entry cluster map (`HashMap<CompactAddress, u32>`
≈ 5-6 GB plus rehash growth transients), which the plan deliberately leaves
intact (no hashed/truncated address keys). Per-entry OutputMap cost did shrink
~175 → ~70-100 B as modeled; at this corpus scale that is ~0.8 GiB of the total
reduction. The transient CSV String (6.3 GB) and sort buffer (1.5 GB) are fully
eliminated — visible directly in the 44% wall-time reduction.

