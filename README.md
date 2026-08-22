# BitIodine

[![CI](https://github.com/mikispag/bitiodine/actions/workflows/ci.yml/badge.svg)](https://github.com/mikispag/bitiodine/actions/workflows/ci.yml)
[![License: MIT/Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Rust: 1.75+](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)

A high-performance, zero-copy Bitcoin blockchain parser and address clusterizer written in Rust.

BitIodine reads raw `blk*.dat` block files produced by `bitcoind` via memory mapping (`mmap`), parses blocks, headers, transactions, and script bytecodes, and provides an extensible **Visitor pattern** to analyze blockchain data, extract address balances, find `OP_RETURN` data outputs, and cluster co-spent addresses using Tarjan's Union-Find algorithm.

---

## Features

- ⚡ **Zero-Copy Memory-Mapped Parsing**: Maps `blk*.dat` files directly into virtual memory for blazing-fast sequential scanning.
- 🔗 **SegWit & Bech32 Native Support**: Parses legacy P2PK/P2PKH/P2SH scripts, SegWit v0 native outputs (P2WPKH, P2WSH), and multisig transactions.
- 👥 **Address Clustering**: Multi-input heuristic clustering with rank-based Union-Find (`DisjointSet`) and iterative path compression.
- 🧩 **Pluggable Visitor Architecture**: Clean `BlockChainVisitor` trait to write custom analyzers, heuristics, and indexers.
- 🛡️ **Modern Pure-Rust Crypto**: Uses standard RustCrypto ecosystem crates (`sha2`, `ripemd`, `bs58`, `hex`) without legacy C dependencies.
- 🖥️ **CLI with Dynamic Action Selection**: Run any visitor directly from the command line without recompiling.

---

## Building

Requires a stable Rust toolchain (1.75 or later).

```bash
cargo build --release
```

The optimized executable will be located at `target/release/bitiodine`.

---

## Usage

```text
A high-performance Bitcoin blockchain parser and address clusterizer in Rust.

Usage: bitiodine [OPTIONS]

Options:
  -b, --blocks-dir <BLOCKS_DIR>  Path to the bitcoind blocks directory [default: ~/.bitcoin/blocks]
  -o, --output <OUTPUT>          Path to the output file [default: clusters.csv]
  -a, --action <ACTION>          Action / visitor to run [default: clusterizer]
                                 [possible values: clusterizer, dump-balances, dump-addresses,
                                  dump-tx-hashes, dataoutput-finder, donation-finder, merkle]
  -v...                          Sets the level of verbosity (-v for debug, -vv for trace)
  -h, --help                     Print help
  -V, --version                  Print version
```

### Available Actions

| Action | Description | Output |
| :--- | :--- | :--- |
| `clusterizer` *(default)* | Groups together co-spent addresses into ownership clusters | CSV (`<address>,<cluster_id>`) |
| `dump-balances` | Calculates unspent balances per address | CSV (`<balance_btc>,<hash160>,<address>`) |
| `dump-addresses` | Extracts all unique recipient addresses across transactions | Stdout |
| `dump-tx-hashes` | Logs transaction IDs at regular block intervals | Stdout |
| `dataoutput-finder` | Discovers and prints `OP_RETURN` arbitrary payload messages | Stdout |
| `donation-finder` | Identifies potential donation or non-standard outputs | Stdout |
| `merkle` | Computes and validates every block's Merkle root against its header | Stdout / Logs |

### Examples

**Run address clusterizer against default bitcoin directory:**
```bash
./target/release/bitiodine -o clusters.csv
```

**Dump address balances from a custom blocks path with debug logging:**
```bash
./target/release/bitiodine -b /var/lib/bitcoind/blocks -a dump-balances -o balances.csv -v
```

**Find OP_RETURN data payloads:**
```bash
./target/release/bitiodine -b /var/lib/bitcoind/blocks -a dataoutput-finder
```

---

## Architecture

BitIodine is structured as a library (`bitiodine_rust`) and a CLI application:

- **`blockchain`**: Handles disk discovery, memory-mapping of `blk*.dat` files, block sequencing, and chain split/reorg resolution.
- **`block` & `header`**: Zero-copy block framing and 80-byte header deserialization (version, previous block hash, Merkle root, timestamp, bits, nonce).
- **`transactions` & `script`**: High-level Bitcoin transaction, input, output, and script bytecode interpreter (P2PK, P2PKH, P2SH, P2WPKH, P2WSH, Multisig, `OP_RETURN`).
- **`hash` & `hash160`**: Type-safe, transparent wrappers for 256-bit and 160-bit Bitcoin hashes with standard little-endian formatting.
- **`visitors`**: Implementation of analysis plugins conforming to `BlockChainVisitor`.

---

## Testing

Run the test suite:

```bash
cargo test
```

Run clippy linter:

```bash
cargo clippy --all-targets -- -D warnings
```

---

## Credits

The blockchain parser architecture is based on research and code originally developed by Michele Spagnuolo ([miki.it](https://miki.it)) and Mathias Svensson.
