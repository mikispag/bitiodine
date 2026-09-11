use clap::{Parser, ValueEnum};
use env_logger::Builder;
use log::{error, info, LevelFilter};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use bitiodine::hash::Hash;
use bitiodine::visitors::{
    BlockChainVisitor, Clusterizer, DataOutputFinder, DonationFinder, DumpAddresses, DumpBalances,
    DumpTxHashes, MerkleVisitor,
};
use bitiodine::BlockChain;

#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL_ALLOCATOR: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Group co-spent Bitcoin addresses using an ownership heuristic
    Clusterizer,
    /// Dump all balances and associated addresses
    DumpBalances,
    /// Print all standard and SegWit output addresses
    DumpAddresses,
    /// Print transaction IDs at periodic block heights
    DumpTxHashes,
    /// Find and print OP_RETURN data outputs
    DataoutputFinder,
    /// Print potential donation or non-standard outputs as CSV
    DonationFinder,
    /// Verify the Merkle root of every block
    Merkle,
}

#[derive(Parser, Debug)]
#[command(
    name = "bitiodine",
    version,
    author = "Michele Spagnuolo <miki@miki.it>",
    about = "A high-performance Bitcoin blockchain parser and address clusterizer in Rust."
)]
struct Cli {
    /// Path to the bitcoind blocks directory
    #[arg(short = 'b', long = "blocks-dir", default_value_os_t = default_blocks_dir())]
    blocks_dir: PathBuf,

    /// Path to the output file (used by clusterizer and dump-balances)
    #[arg(short = 'o', long = "output", default_value = "clusters.csv")]
    output: PathBuf,

    /// Stop after processing at most this many block files (0 = no limit)
    #[arg(long, default_value_t = 0)]
    max_blocks: usize,

    /// Select the chain ending at this block hash (e.g. Core getbestblockhash)
    #[arg(long)]
    chain_tip: Option<String>,

    /// Action / visitor to run on the blockchain
    #[arg(short = 'a', long = "action", value_enum, default_value_t = Action::Clusterizer)]
    action: Action,

    /// Sets the level of verbosity (-v for debug, -vv for trace)
    #[arg(short = 'v', action = clap::ArgAction::Count)]
    verbosity: u8,
}

fn default_blocks_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    let data_dir = dirs::data_dir().unwrap_or_default().join("Bitcoin");
    #[cfg(target_os = "macos")]
    let data_dir = dirs::home_dir()
        .unwrap_or_default()
        .join("Library/Application Support/Bitcoin");
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let data_dir = dirs::home_dir().unwrap_or_default().join(".bitcoin");
    data_dir.join("blocks")
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

fn main() -> ExitCode {
    let cli = Cli::parse();
    initialize_logger(cli.verbosity);
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            error!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    info!(
        "Opening Bitcoin blockchain from: {}",
        cli.blocks_dir.display()
    );
    let chain = BlockChain::read(&cli.blocks_dir, cli.max_blocks)?;
    let chain = match cli.chain_tip {
        Some(tip) => chain.with_tip(Hash::from_pretty(&tip)?),
        None => chain,
    };
    if chain.is_empty() {
        return Err(format!(
            "No Bitcoin block files (blk*.dat) found in: {}. Please verify the --blocks-dir path.",
            cli.blocks_dir.display()
        )
        .into());
    }
    info!("Found {} block file(s) to process.", chain.len());

    match cli.action {
        Action::Clusterizer => {
            let mut visitor = Clusterizer::new();
            chain.walk(&mut visitor)?;
            let (count, _) = visitor.done()?;
            info!(
                "Writing {count} clustered addresses to {}",
                cli.output.display()
            );
            let file = File::create(&cli.output)?;
            let mut writer = BufWriter::new(file);
            visitor.write_csv(&mut writer)?;
            writer.flush()?;
            bitiodine::metrics::log_peak_rss("after clusterizer");
            info!("Done!");
        }
        Action::DumpBalances => {
            let mut visitor = DumpBalances::new();
            chain.walk(&mut visitor)?;
            let (count, _) = visitor.done()?;
            info!(
                "Writing {count} address balances to {}",
                cli.output.display()
            );
            let file = File::create(&cli.output)?;
            let mut writer = BufWriter::new(file);
            visitor.write_csv(&mut writer)?;
            writer.flush()?;
            bitiodine::metrics::log_peak_rss("after dump-balances");
            info!("Done!");
        }
        Action::DumpAddresses => {
            let mut visitor = DumpAddresses::new();
            chain.walk(&mut visitor)?;
            visitor.done()?;
        }
        Action::DumpTxHashes => {
            let mut visitor = DumpTxHashes::new();
            chain.walk(&mut visitor)?;
            visitor.done()?;
        }
        Action::DataoutputFinder => {
            let mut visitor = DataOutputFinder::new();
            chain.walk(&mut visitor)?;
            visitor.done()?;
        }
        Action::DonationFinder => {
            let mut visitor = DonationFinder::new();
            chain.walk(&mut visitor)?;
            visitor.done()?;
        }
        Action::Merkle => {
            let mut visitor = MerkleVisitor::new();
            chain.walk(&mut visitor)?;
            visitor.done()?;
            info!("All Merkle roots verified successfully!");
        }
    }
    Ok(())
}
