use preamble::*;

pub mod clusterizer;
pub mod dataoutput_finder;
pub mod donation_finder;
pub mod dump_addresses;
pub mod dump_balances;
pub mod dump_tx_hashes;
pub mod merkle;

pub trait BlockChainVisitor<'a> {
    type BlockItem;
    type TransactionItem;
    type OutputItem;
    type DoneItem;

    fn new() -> Self;

    fn visit_block_begin(&mut self, _block: Block<'a>, _height: u64) -> Self::BlockItem;
    fn visit_block_end(&mut self, _block: Block<'a>, _height: u64, _block_item: Self::BlockItem) {}

    fn visit_transaction_begin(&mut self, _block_item: &mut Self::BlockItem) -> Self::TransactionItem;
    fn visit_transaction_input(&mut self, _txin: TransactionInput<'a>, _block_item: &mut Self::BlockItem, _tx_item: &mut Self::TransactionItem, _output_item: Option<Self::OutputItem>) {}

    fn visit_transaction_output(&mut self, _txout: TransactionOutput<'a>, _block_item: &mut Self::BlockItem, _tx_item: &mut Self::TransactionItem) -> Option<Self::OutputItem> {
        None
    }
    fn visit_transaction_end(&mut self, _tx: Transaction<'a>, _block_item: &mut Self::BlockItem, _tx_item: Self::TransactionItem) {}

    fn done(&mut self) -> Result<Self::DoneItem>;
}
