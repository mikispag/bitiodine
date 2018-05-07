use preamble::*;

pub struct DumpAddresses;

impl<'a> BlockChainVisitor<'a> for DumpAddresses {
    type BlockItem = ();
    type TransactionItem = ();
    type OutputItem = ();
    type DoneItem = ();

    fn new() -> Self {
        Self {}
    }

    fn visit_block_begin(&mut self, _block: Block<'a>, _height: u64) {}

    fn visit_transaction_begin(&mut self, _block_item: &mut Self::BlockItem) {}

    fn visit_transaction_output(&mut self, txout: TransactionOutput<'a>, _block_item: &mut (), _transaction_item: &mut ()) -> Option<Self::OutputItem> {
        let addresses = match txout.script.to_highlevel() {
            HighLevel::PayToPubkeyHash(pkh) => Some(vec![Address::from_hash160(Hash160::from_slice(pkh), 0x00)]),
            HighLevel::PayToScriptHash(pkh) => Some(vec![Address::from_hash160(Hash160::from_slice(pkh), 0x05)]),
            HighLevel::PayToMultisig(_, pks) => Some(pks.iter().map(|pk| Address::from_pubkey(pk, 0x05)).collect()),
            HighLevel::PayToWitnessPubkeyHash(w) | HighLevel::PayToWitnessScriptHash(w) => Some(vec![Address(w.to_address())]),
            _ => None,
        };

        if addresses.is_some() {
            for address in addresses.unwrap() {
                println!("{}", address);
            }
            Some(());
        }
        None
    }

    fn done(&mut self) -> Result<Self::DoneItem> {
        Ok(())
    }
}
