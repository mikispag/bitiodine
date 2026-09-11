use std::io::{self, Write};

use crate::address::Address;
use crate::block::Block;
use crate::error::{ParseResult, Result};
use crate::hash160::Hash160;
use crate::script::HighLevel;
use crate::transactions::TransactionOutput;
use crate::visitors::BlockChainVisitor;

/// Emits output addresses and P2PKH-form identifiers for public keys in P2PK
/// and bare multisig scripts. Public-key identifiers are not script destinations.
pub struct DumpAddresses<'w> {
    writer: Box<dyn Write + 'w>,
    error: Option<io::Error>,
}

impl Default for DumpAddresses<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'w> DumpAddresses<'w> {
    pub fn with_writer(writer: impl Write + 'w) -> Self {
        Self {
            writer: Box::new(writer),
            error: None,
        }
    }
}

impl BlockChainVisitor for DumpAddresses<'_> {
    type BlockItem = ();
    type TransactionItem = ();
    type OutputItem = ();
    type DoneItem = ();

    fn new() -> Self {
        Self::with_writer(io::stdout())
    }

    fn visit_block_begin<'a>(&mut self, _block: Block<'a>, _height: u64) {}

    fn visit_transaction_begin(&mut self, _block_item: &mut Self::BlockItem) {}

    fn visit_transaction_output<'a>(
        &mut self,
        txout: TransactionOutput<'a>,
        _block_item: &mut (),
        _transaction_item: &mut (),
    ) -> Option<Self::OutputItem> {
        if self.error.is_some() {
            return None;
        }
        let addresses = match txout.script.to_highlevel() {
            HighLevel::PayToPubkey(pk) => Some(vec![Address::from_pubkey(pk, 0x00)]),
            HighLevel::PayToPubkeyHash(pkh) => {
                Some(vec![Address::from_hash160(Hash160::from_slice(pkh), 0x00)])
            }
            HighLevel::PayToScriptHash(pkh) => {
                Some(vec![Address::from_hash160(Hash160::from_slice(pkh), 0x05)])
            }
            HighLevel::PayToMultisig(_, pks) => Some(
                pks.iter()
                    .map(|pk| Address::from_pubkey(pk, 0x00))
                    .collect(),
            ),
            HighLevel::PayToWitnessPubkeyHash(ref w)
            | HighLevel::PayToWitnessScriptHash(ref w)
            | HighLevel::PayToWitnessTaproot(ref w)
            | HighLevel::PayToWitnessGeneral(ref w) => Some(vec![Address::from_witness_program(w)]),
            _ => None,
        };

        if let Some(addresses) = addresses {
            for address in addresses {
                if let Err(error) = writeln!(self.writer, "{}", address) {
                    self.error = Some(error);
                    break;
                }
            }
        }
        None
    }

    fn visit_block_end<'a>(
        &mut self,
        _block: Block<'a>,
        _height: u64,
        _block_item: (),
    ) -> ParseResult<()> {
        if let Some(error) = self.error.take() {
            return Err(error.into());
        }
        Ok(())
    }

    fn done(&mut self) -> Result<Self::DoneItem> {
        if let Some(error) = self.error.take() {
            return Err(error.into());
        }
        self.writer.flush()?;
        Ok(())
    }
}
