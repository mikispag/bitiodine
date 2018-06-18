use merkle::MerkleHasher;
use preamble::*;

pub struct MerkleVisitor;

impl<'a> BlockChainVisitor<'a> for MerkleVisitor {
    type BlockItem = MerkleHasher;
    type TransactionItem = ();
    type OutputItem = ();
    type DoneItem = ();

    fn new() -> Self {
        Self {}
    }

    fn visit_block_begin(&mut self, _block: Block<'a>, _height: u64) -> MerkleHasher {
        Default::default()
    }

    fn visit_block_end(&mut self, block: Block<'a>, _height: u64, hasher: MerkleHasher) {
        assert_eq!(block.header().merkle_root(), &hasher.finish().unwrap());
    }

    fn visit_transaction_begin(&mut self, _hasher: &mut MerkleHasher) {}

    fn visit_transaction_end(
        &mut self,
        tx: Transaction<'a>,
        hasher: &mut MerkleHasher,
        _tx_item: (),
    ) {
        hasher.add(Hash::from_data(tx.slice));
    }

    fn done(&mut self) -> Result<Self::DoneItem> {
        Ok(())
    }
}
