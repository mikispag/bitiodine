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
//!
//! The CLI uses disk-backed clustering by default. Library callers can opt into
//! the same behavior with [`cluster_store::ClusterStore`] and
//! [`visitors::ResumableClusterizer`]. [`BlockChain::walk_clusterizer_resumable`]
//! commits each complete block atomically and returns the full selected block
//! count and tip; UTXOs and Union-Find state remain in SQLite. On a later run it
//! resumes matching ancestry, rolls back reorganizations within the retained
//! 144-block undo history, or replays from genesis when that undo has expired.
//! This recovery requires consistent source snapshots and an independently
//! validated tip; it does not follow or operate a live Bitcoin Core node.
//!
//! ```no_run
//! use std::io::Write;
//! use std::path::Path;
//! use bitiodine::{BlockChain, Hash, ParseResult};
//! use bitiodine::cluster_store::ClusterStore;
//! use bitiodine::visitors::ResumableClusterizer;
//!
//! fn resume_clustering(
//!     blocks_dir: &Path,
//!     state_dir: &Path,
//!     core_tip: Hash,
//!     sqlite_bytes: u64,
//!     traversal_bytes: u64,
//!     csv: &mut impl Write,
//! ) -> ParseResult<(u64, Hash)> {
//!     let mut store = ClusterStore::open_with_memory_limit(state_dir, sqlite_bytes)?;
//!     let chain = BlockChain::read_with_memory_limit(blocks_dir, 0, traversal_bytes)?
//!         .with_tip(core_tip);
//!     let result = chain.walk_clusterizer_resumable(
//!         &mut ResumableClusterizer::with_store(&mut store),
//!     )?;
//!     store.write_csv(csv)?;
//!     csv.flush()?;
//!     Ok(result)
//! }
//! ```
//!
//! Library budgets are supplied by the caller. The traversal budget covers file
//! discovery, header indexing, chain selection and block decoding, excluding
//! visitor-owned state and retained output maps. SQLite's heap limit is
//! process-wide; an already lower limit takes precedence. The original
//! [`visitors::Clusterizer`] keeps its growing Union-Find state in memory.
//! Traversal owns its block decoding buffer; parser views borrow slices from it.
//! Persistent clustering streams its working state through SQLite rather than
//! retaining a second full copy in Rust.
//!
//! Constructor migration: [`BlockChainVisitor::new`] requires `Self: Default`.
//! Generic callers of `V::new()` need `V: BlockChainVisitor + Default`; custom
//! visitors can implement [`Default`] or use an explicit constructor. Existing
//! built-in constructors are unchanged. The persistent visitor is constructed
//! through [`visitors::ResumableClusterizer::with_store`], ensuring it has a store.

pub mod address;
pub mod block;
pub mod blockchain;
pub mod buffer_operations;
pub mod bytecode;
pub mod cluster_store;
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
