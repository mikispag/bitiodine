use crate::block::Block;
use crate::error::{ParseResult, Result};
use crate::transactions::{Transaction, TransactionInput, TransactionOutput};

pub mod clusterizer;
pub mod dataoutput_finder;
pub mod donation_finder;
pub mod dump_addresses;
pub mod dump_balances;
pub mod dump_tx_hashes;
pub mod merkle;
pub mod resumable_clusterizer;

pub use clusterizer::Clusterizer;
pub use dataoutput_finder::DataOutputFinder;
pub use donation_finder::DonationFinder;
pub use dump_addresses::DumpAddresses;
pub use dump_balances::DumpBalances;
pub use dump_tx_hashes::DumpTxHashes;
pub use merkle::MerkleVisitor;
pub use resumable_clusterizer::ResumableClusterizer;

/// Receives blocks and transactions in the chain order selected by
/// [`BlockChain::walk`](crate::BlockChain::walk).
///
/// For each block, callbacks run in this order:
///
/// 1. [`visit_block_begin`](Self::visit_block_begin).
/// 2. For each transaction: [`visit_transaction_begin`](Self::visit_transaction_begin),
///    all input callbacks, all output callbacks, all witness callbacks,
///    any replaced-output callbacks, and [`visit_transaction_end`](Self::visit_transaction_end).
/// 3. [`visit_block_end`](Self::visit_block_end).
///
/// Output items returned by [`visit_transaction_output`](Self::visit_transaction_output)
/// are retained until spent or replaced by a duplicate transaction ID. Remaining
/// items are returned by the walk. Borrowed block and transaction data are valid
/// only during their callbacks; retained state must own any needed data.
///
/// Parsing and block-end errors stop traversal without rolling back visitor
/// state or guaranteeing matching end callbacks. The caller must invoke
/// [`done`](Self::done) explicitly after a successful walk.
pub trait BlockChainVisitor {
    /// State created at block begin and consumed at block end.
    type BlockItem;
    /// State created at transaction begin and consumed at transaction end.
    type TransactionItem;
    /// Owned state retained for an output until it is spent or replaced.
    type OutputItem;
    /// Result produced when the caller finalizes the visitor.
    type DoneItem;

    /// Creates a visitor with its initial state.
    fn new() -> Self
    where
        Self: Default,
    {
        Self::default()
    }

    /// Starts a block at its zero-based height in the selected chain.
    fn visit_block_begin<'a>(&mut self, _block: Block<'a>, _height: u64) -> Self::BlockItem;
    /// Finishes a fully parsed block; returning an error stops traversal.
    fn visit_block_end<'a>(
        &mut self,
        _block: Block<'a>,
        _height: u64,
        _block_item: Self::BlockItem,
    ) -> ParseResult<()> {
        Ok(())
    }

    /// Starts a transaction before parsing its fields.
    fn visit_transaction_begin(
        &mut self,
        _block_item: &mut Self::BlockItem,
    ) -> Self::TransactionItem;
    /// Receives an input and consumes its previously retained output item, if any.
    ///
    /// `None` means no item was retained for this outpoint; it does not by itself
    /// indicate an invalid input. Coinbase inputs also have no retained item.
    fn visit_transaction_input<'a>(
        &mut self,
        _txin: TransactionInput<'a>,
        _block_item: &mut Self::BlockItem,
        _tx_item: &mut Self::TransactionItem,
        _output_item: Option<Self::OutputItem>,
    ) {
    }

    /// Receives an output and optionally returns owned state to retain for it.
    fn visit_transaction_output<'a>(
        &mut self,
        _txout: TransactionOutput<'a>,
        _block_item: &mut Self::BlockItem,
        _tx_item: &mut Self::TransactionItem,
    ) -> Option<Self::OutputItem> {
        None
    }

    /// Receives a witness stack element, including empty elements, after outputs.
    ///
    /// Input and item indices are zero-based. Legacy transactions and empty witness
    /// stacks emit no callbacks. Retained witness data must be copied by the visitor.
    fn visit_transaction_witness(
        &mut self,
        _input_index: u64,
        _item_index: u64,
        _item: &[u8],
        _block_item: &mut Self::BlockItem,
        _tx_item: &mut Self::TransactionItem,
    ) {
    }

    /// Consumes an unspent item replaced by a transaction with the same ID.
    ///
    /// Called once for each retained output of the earlier transaction, after
    /// the replacement's output callbacks and before its transaction-end callback.
    /// Visitors maintaining balances should remove the replaced item's value.
    fn visit_output_removed(&mut self, _output_item: Self::OutputItem) {}

    /// Finishes a fully parsed transaction, including its computed transaction ID.
    fn visit_transaction_end<'a>(
        &mut self,
        _tx: Transaction<'a>,
        _block_item: &mut Self::BlockItem,
        _tx_item: Self::TransactionItem,
    ) {
    }

    /// Finalizes the visitor and returns its result; traversal does not call this.
    fn done(&mut self) -> Result<Self::DoneItem>;
}
