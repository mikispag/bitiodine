use crate::block::Block;
use crate::error::Result;
use crate::script::HighLevel;
use crate::transactions::TransactionOutput;
use crate::visitors::BlockChainVisitor;

#[derive(Default)]
pub struct DataOutputFinder;

impl BlockChainVisitor for DataOutputFinder {
    type BlockItem = ();
    type TransactionItem = ();
    type OutputItem = String;
    type DoneItem = ();

    fn new() -> Self {
        Self
    }

    fn visit_block_begin<'a>(&mut self, _block: Block<'a>, _height: u64) {}

    fn visit_transaction_begin(&mut self, _hasher: &mut ()) {}

    fn visit_transaction_output<'a>(
        &mut self,
        txout: TransactionOutput<'a>,
        _block_item: &mut (),
        _transaction_item: &mut (),
    ) -> Option<Self::OutputItem> {
        match txout.script.to_highlevel() {
            HighLevel::DataOutput(data) => Some(String::from_utf8_lossy(data).into_owned()),
            _ => None,
        }
    }

    fn done(&mut self) -> Result<Self::DoneItem> {
        Ok(())
    }
}
