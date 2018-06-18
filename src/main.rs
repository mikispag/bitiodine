#[macro_use]
extern crate arrayref;
extern crate base58;
extern crate bitcoin_bech32;
extern crate byteorder;
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
use env_logger::Builder;
use log::LevelFilter;
use visitors::clusterizer::Clusterizer;
use visitors::BlockChainVisitor;
//use visitors::dump_balances::DumpBalances;
//use visitors::dump_tx_hashes::DumpTxHashes;

use std::io::Write;

pub use address::Address;
pub use hash::Hash;
pub use header::BlockHeader;
pub use script::HighLevel;

fn initialize_logger() {
    Builder::new()
        .filter(None, LevelFilter::Info)
        .format(|buf, record| {
            let t = time::now();
            writeln!(
                buf,
                "{}.{:04} - {} - {}",
                time::strftime("%Y-%m-%d %H:%M:%S", &t).unwrap(),
                t.tm_nsec / 100_000,
                record.level(),
                record.args()
            )
        })
        .init();
}

fn main() {
    initialize_logger();
    let chain = unsafe { BlockChain::read() };

    /*
    let (_, _, _) = chain
        .walk(&mut visitors::dump_tx_hashes::DumpTxHashes)
        .unwrap();
    */

    let mut clusterizer_visitor = Clusterizer::new();
    let (_, _, _) = chain.walk(&mut clusterizer_visitor).unwrap();
    let _clusters_count = clusterizer_visitor.done();

    /*
    let (_, _, map) = chain.walk(&mut visitors::dataoutput_finder::DataOutputFinder).unwrap();
    for (tx, payload) in map.into_iter() {
        for (tx_id, data) in payload.into_iter() {
            println!("{},{},{}", tx, tx_id, data);
        }
    }
    */

    /*
    let mut balances_visitor = DumpBalances::new();
    let (_, _, _) = chain.walk(&mut balances_visitor).unwrap();
    let _ = balances_visitor.done();
    */
}
