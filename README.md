# Bitiodine
A Rust Bitcoin blockchain parser with clustering capabilities, allowing to group together addresses in ownership clusters.

The parser is compatible with SegWit, correctly parses bech32 native outputs and witness programs, and deals with chain splits gracefully.

## Building

```
cargo update
cargo build --release
```

## Usage

Enable the desired *visitor* (by default, the Clusterizer is on), rebuild, and run the executable:

```
target/release/bitiodine-rust
```

## Credits

The blockchain parser is based on Mathias Svensson's code ([GitHub](https://github.com/Idolf)).
