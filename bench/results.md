# BitIodine memory benchmarks

Peak RSS is measured as `VmHWM` (process high-water mark) read by the binary itself
from `/proc/self/status` — identical to GNU `time -v` "Maximum resident set size".
Corpus: first N block files of the local mainnet blocks dir, `--max-blocks N`.
Corpus start: commit 04897da lineage; benchmark machine: 60 GB RAM, bitcoind stopped,
XOR obfuscation present (xor.dat).

## Baseline (before any optimization, after Phase 0 instrumentation)

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
