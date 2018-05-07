use preamble::*;

pub struct DumpTxHashes;

impl<'a> BlockChainVisitor<'a> for DumpTxHashes {
    type BlockItem = u64;
    type TransactionItem = ();
    type OutputItem = ();
    type DoneItem = ();

    fn new() -> Self {
        Self {}
    }

    fn visit_block_begin(&mut self, block: Block<'a>, height: u64) -> Self::BlockItem {
        if height > 480000 && height % 1000 == 0 {
            println!("Block {} - {} transactions", height, block.transactions().unwrap().count);
        }
        height
    }

    fn visit_transaction_begin(&mut self, _block_item: &mut Self::BlockItem) {}

    fn visit_transaction_end(&mut self, tx: Transaction<'a>, block_item: &mut Self::BlockItem, _tx_item: Self::TransactionItem) {
        if *block_item > 480000 && *block_item % 1000 == 0 {
            println!("Transaction {}", tx.txid.to_string());
        }
    }

    fn done(&mut self) -> Result<Self::DoneItem> {
        Ok(())
    }
}
