use clap::{Parser, ValueEnum};
use env_logger::Builder;
use log::{error, info, LevelFilter};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use bitiodine_rust::visitors::{
    BlockChainVisitor, Clusterizer, DataOutputFinder, DonationFinder, DumpAddresses, DumpBalances,
    DumpTxHashes, MerkleVisitor,
};
use bitiodine_rust::BlockChain;

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Group together Bitcoin addresses in ownership clusters
    Clusterizer,
    /// Dump all balances and associated addresses
    DumpBalances,
    /// Print all standard and SegWit output addresses
    DumpAddresses,
    /// Print transaction IDs at periodic block heights
    DumpTxHashes,
    /// Find and print OP_RETURN data outputs
    DataoutputFinder,
    /// Find potential donation or non-standard outputs
    DonationFinder,
    /// Verify the Merkle root of every block
    Merkle,
}

#[derive(Parser, Debug)]
#[command(
    name = "bitiodine",
    version,
    author = "Michele Spagnuolo <mikispag@gmail.com>",
    about = "A high-performance Bitcoin blockchain parser and address clusterizer in Rust."
)]
struct Cli {
    /// Path to the bitcoind blocks directory
    #[arg(short = 'b', long = "blocks-dir", default_value_os_t = default_blocks_dir())]
    blocks_dir: PathBuf,

    /// Path to the output file (used by clusterizer and dump-balances)
    #[arg(short = 'o', long = "output", default_value = "clusters.csv")]
    output: PathBuf,

    /// Action / visitor to run on the blockchain
    #[arg(short = 'a', long = "action", value_enum, default_value_t = Action::Clusterizer)]
    action: Action,

    /// Sets the level of verbosity (-v for debug, -vv for trace)
    #[arg(short = 'v', action = clap::ArgAction::Count)]
    verbosity: u8,
}

fn default_blocks_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".bitcoin").join("blocks"))
        .unwrap_or_else(|| PathBuf::from(".bitcoin/blocks"))
}

fn initialize_logger(verbosity: u8) {
    let level_filter = match verbosity {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

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
    let cli = Cli::parse();
    initialize_logger(cli.verbosity);

    info!(
        "Opening Bitcoin blockchain from: {}",
        cli.blocks_dir.display()
    );
    let chain = unsafe { BlockChain::read(&cli.blocks_dir) };
    if chain.is_empty() {
        error!(
            "No Bitcoin block files (blk*.dat) found in: {}. Please verify the --blocks-dir path.",
            cli.blocks_dir.display()
        );
        return;
    }
    info!("Found {} block file(s) to process.", chain.len());

    match cli.action {
        Action::Clusterizer => {
            let mut visitor = Clusterizer::new();
            if let Err(e) = chain.walk(&mut visitor) {
                error!("Error walking blockchain: {e}");
                return;
            }
            let (count, _) = visitor.done().expect("Clusterizer failed!");
            info!(
                "Writing {count} clustered addresses to {}",
                cli.output.display()
            );
            let file = File::create(&cli.output).expect("Unable to create output file!");
            let mut writer = BufWriter::new(file);
            visitor
                .write_csv(&mut writer)
                .expect("Unable to write output file!");
            info!("Done!");
        }
        Action::DumpBalances => {
            let mut visitor = DumpBalances::new();
            if let Err(e) = chain.walk(&mut visitor) {
                error!("Error walking blockchain: {e}");
                return;
            }
            let (count, _) = visitor.done().expect("DumpBalances failed!");
            info!(
                "Writing {count} address balances to {}",
                cli.output.display()
            );
            let file = File::create(&cli.output).expect("Unable to create output file!");
            let mut writer = BufWriter::new(file);
            visitor
                .write_csv(&mut writer)
                .expect("Unable to write output file!");
            info!("Done!");
        }
        Action::DumpAddresses => {
            let mut visitor = DumpAddresses::new();
            if let Err(e) = chain.walk(&mut visitor) {
                error!("Error walking blockchain: {e}");
            }
        }
        Action::DumpTxHashes => {
            let mut visitor = DumpTxHashes::new();
            if let Err(e) = chain.walk(&mut visitor) {
                error!("Error walking blockchain: {e}");
            }
        }
        Action::DataoutputFinder => {
            let mut visitor = DataOutputFinder::new();
            if let Err(e) = chain.walk(&mut visitor) {
                error!("Error walking blockchain: {e}");
            }
        }
        Action::DonationFinder => {
            let mut visitor = DonationFinder::new();
            if let Err(e) = chain.walk(&mut visitor) {
                error!("Error walking blockchain: {e}");
            }
        }
        Action::Merkle => {
            let mut visitor = MerkleVisitor::new();
            if let Err(e) = chain.walk(&mut visitor) {
                error!("Error walking blockchain: {e}");
            } else {
                info!("All Merkle roots verified successfully!");
            }
        }
    }
}
