use crate::block::Block;
use crate::error::Result;
use crate::transactions::Transaction;
use crate::visitors::BlockChainVisitor;

#[derive(Default)]
pub struct DumpTxHashes;

impl BlockChainVisitor for DumpTxHashes {
    type BlockItem = u64;
    type TransactionItem = ();
    type OutputItem = ();
    type DoneItem = ();

    fn new() -> Self {
        Self
    }

    fn visit_block_begin<'a>(&mut self, block: Block<'a>, height: u64) -> Self::BlockItem {
        if height > 480_000 && height % 1000 == 0 {
            println!(
                "Block {} - {} transactions",
                height,
                block.transactions().unwrap().count
            );
        }
        height
    }

    fn visit_transaction_begin(&mut self, _block_item: &mut Self::BlockItem) {}

    fn visit_transaction_end<'a>(
        &mut self,
        tx: Transaction<'a>,
        block_item: &mut Self::BlockItem,
        _tx_item: Self::TransactionItem,
    ) {
        if *block_item > 480_000 && *block_item % 1000 == 0 {
            println!("Transaction {}", tx.txid);
        }
    }

    fn done(&mut self) -> Result<Self::DoneItem> {
        Ok(())
    }
}
