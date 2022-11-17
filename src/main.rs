#[macro_use]
extern crate arrayref;
extern crate base58;
extern crate bitcoin_bech32;
extern crate byteorder;
extern crate chrono;
extern crate clap;
extern crate crypto;
extern crate memmap;
extern crate rustc_serialize;
extern crate time;
extern crate vec_map;
extern crate void;

extern crate env_logger;
#[macro_use]
extern crate log;

#[macro_use]
mod buffer_operations;

mod address;
mod block;
mod blockchain;
mod bytecode;
mod error;
mod hash;
mod hash160;
mod header;
mod merkle;
mod preamble;
mod script;
mod transactions;
pub mod visitors;

use blockchain::BlockChain;
use clap::{Arg, Command};
use env_logger::Builder;
use log::LevelFilter;
use visitors::clusterizer::Clusterizer;
use visitors::BlockChainVisitor;

use std::fs::File;
use std::io::{LineWriter, Write};

pub use address::Address;
pub use hash::Hash;
pub use header::BlockHeader;
pub use script::HighLevel;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn initialize_logger(level_filter: LevelFilter) {
    Builder::new()
        .filter(None, level_filter)
        .format(|buf, record| {
            writeln!(
                buf,
                "{} - {} - {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.level(),
                record.args()
            )
        })
        .init();
}

fn main() {
    let default_blocks_dir = dirs::home_dir()
        .expect("Unable to get the home directory!")
        .join(".bitcoin")
        .join("blocks")
        .into_os_string()
        .into_string()
        .expect("Unable to build a default bitcoind blocks directory!");

    let matches = Command::new("BitIodine")
        .version(VERSION)
        .author("Michele Spagnuolo <mikispag@gmail.com>")
        .about("A Rust Bitcoin blockchain parser with clustering capabilities, allowing to group together addresses in ownership clusters.")
        .arg(Arg::new("blocks_dir")
            .help("Sets the path to the bitcoind blocks directory")
            .long("blocks-dir")
            .short('b')
            .num_args(1)
            .value_name("BLOCKS_DIRECTORY_PATH")
            .default_value(default_blocks_dir))
        .arg(Arg::new("output")
            .help("Sets the path to the output clusters.csv file")
            .long("output")
            .short('o')
            .num_args(1)
            .value_name("OUTPUT_FILE")
            .default_value("clusters.csv"))
        .arg(Arg::new("v")
            .short('v')
            .num_args(1..)
            .help("Sets the level of verbosity"))
        .get_matches();

    let level_filter: LevelFilter;
    match matches.get_count("v") {
        0 => level_filter = LevelFilter::Info,
        1 => level_filter = LevelFilter::Debug,
        2 | _ => level_filter = LevelFilter::Off,
    }
    initialize_logger(level_filter);

    let chain = unsafe { BlockChain::read(matches.get_one::<String>("blocks_dir").unwrap()) };

    let mut clusterizer_visitor = Clusterizer::new();
    let (_, _, _) = chain.walk(&mut clusterizer_visitor).unwrap();
    let (_clusters_count, visitor_output) =
        clusterizer_visitor.done().expect("Clusterizer failed!");

    let mut writer = LineWriter::new(
        File::create(matches.get_one::<String>("output").unwrap())
            .expect("Unable to create the output file!"),
    );
    writer
        .write_all(visitor_output.as_bytes())
        .expect("Unable to write output file!");
}
