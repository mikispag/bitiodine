use crate::block::Block;
use crate::error::{ParseError, ParseResult, Result};
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

    fn visit_block_end<'a>(
        &mut self,
        block: Block<'a>,
        height: u64,
        hasher: MerkleHasher,
    ) -> ParseResult<()> {
        let header = block.header();
        let computed = hasher.finish();
        if computed.as_ref() != Some(header.merkle_root()) {
            let computed = computed.map_or_else(|| "nothing".to_owned(), |root| root.to_string());
            return Err(ParseError::Chain(format!(
                "Merkle root mismatch in block {} at height {height}: header has {}, transactions hash to {computed}",
                header.cur_hash(),
                header.merkle_root(),
            )));
        }
        Ok(())
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
