use preamble::*;

pub struct DonationFinder;

impl<'a> BlockChainVisitor<'a> for DonationFinder {
    type BlockItem = ();
    type TransactionItem = ();
    type OutputItem = ();
    type DoneItem = ();

    fn new() -> Self {
        Self {}
    }

    fn visit_block_begin(&mut self, _block: Block<'a>, _height: u64) {}

    fn visit_transaction_begin(&mut self, _hasher: &mut ()) {}

    fn visit_transaction_output(
        &mut self,
        txout: TransactionOutput<'a>,
        _block_item: &mut (),
        _transaction_item: &mut (),
    ) -> Option<Self::OutputItem> {
        match txout.script.to_highlevel() {
            HighLevel::Donation | HighLevel::Unknown(..) if txout.value > 0 => Some(()),
            _ => None,
        }
    }

    fn done(&mut self) -> Result<Self::DoneItem> {
        Ok(())
    }
}
