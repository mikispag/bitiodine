use crate::block::Block;
use crate::error::Result;
use crate::merkle::MerkleHasher;
use crate::transactions::Transaction;
use crate::visitors::BlockChainVisitor;

#[derive(Default)]
pub struct MerkleVisitor;

impl BlockChainVisitor for MerkleVisitor {
    type BlockItem = MerkleHasher;
    type TransactionItem = ();
    type OutputItem = ();
    type DoneItem = ();

    fn new() -> Self {
        Self
    }

    fn visit_block_begin<'a>(&mut self, _block: Block<'a>, _height: u64) -> MerkleHasher {
        Default::default()
    }

    fn visit_block_end<'a>(&mut self, block: Block<'a>, _height: u64, hasher: MerkleHasher) {
        assert_eq!(block.header().merkle_root(), &hasher.finish().unwrap());
    }

    fn visit_transaction_begin(&mut self, _hasher: &mut MerkleHasher) {}

    fn visit_transaction_end<'a>(
        &mut self,
        tx: Transaction<'a>,
        hasher: &mut MerkleHasher,
        _tx_item: (),
    ) {
        hasher.add(tx.txid);
    }

    fn done(&mut self) -> Result<Self::DoneItem> {
        Ok(())
    }
}
