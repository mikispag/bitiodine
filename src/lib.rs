//! Parse Bitcoin Core mainnet block files and analyze a selected chain with visitors.
//!
//! [`BlockChain::read`] discovers consecutive `blkNNNNN.dat` files starting at
//! `blk00000.dat` and reads `xor.dat` when present. Use a stable copy or a stopped
//! node; detected file changes are errors. Reading is safe and fallible.
//!
//! [`BlockChain::walk`] indexes the selected files before visiting blocks in chain
//! order. Supply an independently validated Bitcoin Core tip with
//! [`BlockChain::with_tip`] to select Core's accepted chain. Otherwise, traversal
//! chooses the unique connected candidate with the most header work and rejects
//! ties. Header work selection does not perform consensus validation, and all
//! ancestors of the selected tip must be available in the selected files.
//!
//! ```no_run
//! use std::path::Path;
//! use bitiodine::{BlockChain, BlockChainVisitor, Hash, ParseResult};
//! use bitiodine::visitors::MerkleVisitor;
//!
//! fn verify_merkle_roots(blocks_dir: &Path, core_tip: Hash) -> ParseResult<()> {
//!     // A zero file limit includes every discovered block file.
//!     let chain = BlockChain::read(blocks_dir, 0)?.with_tip(core_tip);
//!     let mut visitor = MerkleVisitor::new();
//!     let (block_count, tip, _retained_outputs) = chain.walk(&mut visitor)?;
//!     visitor.done()?;
//!     println!("Verified {block_count} blocks ending at {tip}");
//!     Ok(())
//! }
//! ```
//!
//! The walk returns the number of visited blocks, selected tip, and unspent output
//! items retained by the visitor. It does not call [`BlockChainVisitor::done`];
//! callers finalize the visitor after a successful walk. Traversal errors can
//! leave visitor state partially updated. See [`BlockChainVisitor`] for callback
//! ordering and output ownership.

pub mod address;
pub mod block;
pub mod blockchain;
pub mod buffer_operations;
pub mod bytecode;
pub mod error;
pub mod hash;
pub mod hash160;
pub mod header;
pub mod merkle;
pub mod metrics;
pub mod script;
pub mod transactions;
pub mod visitors;

pub use address::{Address, CompactAddress};
pub use block::Block;
pub use blockchain::BlockChain;
pub use bytecode::Bytecode;
pub use error::{EofError, ParseError, ParseResult, Result};
pub use hash::{Hash, ZERO_HASH};
pub use hash160::Hash160;
pub use header::BlockHeader;
pub use merkle::MerkleHasher;
pub use script::{ChallengeType, HighLevel, Script};
pub use transactions::{Transaction, TransactionInput, TransactionOutput, Transactions};
pub use visitors::BlockChainVisitor;
