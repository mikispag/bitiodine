# Bitiodine
A Rust Bitcoin blockchain parser with clustering capabilities, allowing to group together addresses in ownership clusters.

The parser is compatible with SegWit, correctly parses bech32 native outputs and witness programs, and deals with chain splits gracefully.

## Building

```
cargo update
cargo build --release
```

## Usage

Enable the desired *visitor* (by default, the Clusterizer is on) in `src/main.rs`, rebuild, and run the executable:

```
target/release/bitiodine-rust
```

```
$ ./bitiodine-rust --help
BitIodine 0.0.2
Michele Spagnuolo <mikispag@gmail.com>
A Rust Bitcoin blockchain parser with clustering capabilities, allowing to group together addresses in ownership
clusters.

USAGE:
    bitiodine-rust [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -v               Sets the level of verbosity
    -V, --version    Prints version information

OPTIONS:
    -b, --blocks-dir <BLOCKS_DIRECTORY_PATH>    Sets the path to the bitcoind blocks directory [default:
                                                /home/$USER/.bitcoin/blocks]
    -o, --output <OUTPUT_FILE>                  Sets the path to the output clusters.csv file [default: clusters.csv]
```

## Credits

The blockchain parser is based on Mathias Svensson's code ([GitHub](https://github.com/Idolf)).
