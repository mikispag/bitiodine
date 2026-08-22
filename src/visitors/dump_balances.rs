use std::collections::HashMap;
use std::io::Write;

use crate::address::Address;
use crate::block::Block;
use crate::error::Result;
use crate::hash::ZERO_HASH;
use crate::hash160::Hash160;
use crate::script::HighLevel;
use crate::transactions::{TransactionInput, TransactionOutput};
use crate::visitors::BlockChainVisitor;

pub struct DumpBalances {
    pub balances: HashMap<(Address, Option<Hash160>), i64>,
}

impl Default for DumpBalances {
    fn default() -> Self {
        Self::new()
    }
}

impl DumpBalances {
    pub fn write_csv<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        for (address_tuple, balance) in &self.balances {
            if *balance == 0 {
                continue;
            }
            let address = &address_tuple.0;
            let hash160 = address_tuple.1.unwrap_or_default();
            writeln!(
                writer,
                "{:.8},{},{}",
                (*balance as f64) * 1e-8,
                hash160,
                address
            )?;
        }
        Ok(())
    }
}

impl<'a> BlockChainVisitor<'a> for DumpBalances {
    type BlockItem = ();
    type TransactionItem = ();
    type OutputItem = (Address, Option<Hash160>, i64);
    type DoneItem = (usize, String);

    fn new() -> Self {
        Self {
            balances: HashMap::with_capacity(1_000_000),
        }
    }

    fn visit_block_begin(&mut self, _block: Block<'a>, _height: u64) {}

    fn visit_transaction_begin(&mut self, _hasher: &mut ()) {}

    fn visit_transaction_input(
        &mut self,
        txin: TransactionInput<'a>,
        _block_item: &mut Self::BlockItem,
        _tx_item: &mut Self::TransactionItem,
        output_item: Option<Self::OutputItem>,
    ) {
        // Ignore coinbase
        if txin.prev_hash == &ZERO_HASH {
            return;
        }

        if let Some((address, hash160, value)) = output_item {
            let key = (address, hash160);
            let prev_balance = self.balances.get(&key).copied().unwrap_or(0);
            if prev_balance == value {
                self.balances.remove(&key);
            } else {
                *self.balances.entry(key).or_insert(0) -= value;
            }
        }
    }

    fn visit_transaction_output(
        &mut self,
        txout: TransactionOutput<'a>,
        _block_item: &mut (),
        _transaction_item: &mut (),
    ) -> Option<Self::OutputItem> {
        let value = txout.value as i64;
        match txout.script.to_highlevel() {
            HighLevel::PayToPubkeyHash(pkh) => {
                let hash160 = Hash160::from_slice(pkh);
                let address = Address::from_hash160(hash160, 0x00);
                *self
                    .balances
                    .entry((address.clone(), Some(*hash160)))
                    .or_insert(0) += value;
                Some((address, Some(*hash160), value))
            }
            HighLevel::PayToScriptHash(pkh) => {
                let hash160 = Hash160::from_slice(pkh);
                let address = Address::from_hash160(hash160, 0x05);
                *self
                    .balances
                    .entry((address.clone(), Some(*hash160)))
                    .or_insert(0) += value;
                Some((address, Some(*hash160), value))
            }
            HighLevel::PayToPubkey(pk) => {
                let hash160 = Hash160::from_data(pk);
                let address = Address::from_hash160(&hash160, 0x00);
                *self
                    .balances
                    .entry((address.clone(), Some(hash160)))
                    .or_insert(0) += value;
                Some((address, Some(hash160), value))
            }
            HighLevel::PayToWitnessPubkeyHash(w)
            | HighLevel::PayToWitnessScriptHash(w)
            | HighLevel::PayToWitnessTaproot(w)
            | HighLevel::PayToWitnessGeneral(w) => {
                let address = Address(w.to_address());
                *self.balances.entry((address.clone(), None)).or_insert(0) += value;
                Some((address, None, value))
            }
            _ => None,
        }
    }

    fn done(&mut self) -> Result<Self::DoneItem> {
        let mut output_string = String::new();

        for (address_tuple, balance) in &self.balances {
            if *balance == 0 {
                continue;
            }
            let address = &address_tuple.0;
            let hash160 = address_tuple.1.unwrap_or_default();
            output_string.push_str(&format!(
                "{:.8},{},{}\n",
                (*balance as f64) * 1e-8,
                hash160,
                address
            ));
        }

        Ok((self.balances.len(), output_string))
    }
}
