use std::collections::HashMap;
use std::io::Write;

use foldhash::fast::RandomState;

use crate::address::CompactAddress;
use crate::block::Block;
use crate::error::Result;
use crate::hash::{Hash, ZERO_HASH};
use crate::hash160::Hash160;
use crate::script::HighLevel;
use crate::transactions::{TransactionInput, TransactionOutput};
use crate::visitors::BlockChainVisitor;

pub struct DumpBalances {
    pub balances: HashMap<CompactAddress, i64, RandomState>,
}

impl Default for DumpBalances {
    fn default() -> Self {
        Self::new()
    }
}

impl DumpBalances {
    pub fn write_csv<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        for (address, balance) in &self.balances {
            if *balance == 0 {
                continue;
            }
            writeln!(
                writer,
                "{:.8},{},{}",
                (*balance as f64) * 1e-8,
                address.hash160(),
                address
            )?;
        }
        Ok(())
    }
}

impl BlockChainVisitor for DumpBalances {
    type BlockItem = bool;
    type TransactionItem = ();
    type OutputItem = (CompactAddress, i64);
    type DoneItem = (usize, String);

    fn new() -> Self {
        Self {
            balances: HashMap::with_capacity_and_hasher(1_000_000, Default::default()),
        }
    }

    fn visit_block_begin<'a>(&mut self, block: Block<'a>, height: u64) -> Self::BlockItem {
        // The genesis coinbase is historical transaction data, but never enters
        // Bitcoin's UTXO set. Other height-zero synthetic blocks are unaffected.
        height == 0
            && block.header().cur_hash()
                == Hash::from_pretty(
                    "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f",
                )
                .expect("constant mainnet genesis hash")
    }

    fn visit_transaction_begin(&mut self, _block_item: &mut Self::BlockItem) {}

    fn visit_transaction_input<'a>(
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

        if let Some(output_item) = output_item {
            self.visit_output_removed(output_item);
        }
    }

    fn visit_output_removed(&mut self, (address, value): Self::OutputItem) {
        let prev_balance = self.balances.get(&address).copied().unwrap_or(0);
        if prev_balance == value {
            self.balances.remove(&address);
        } else {
            *self.balances.entry(address).or_insert(0) -= value;
        }
    }

    fn visit_transaction_output<'a>(
        &mut self,
        txout: TransactionOutput<'a>,
        is_genesis: &mut Self::BlockItem,
        _transaction_item: &mut (),
    ) -> Option<Self::OutputItem> {
        if *is_genesis {
            return None;
        }
        let value = txout.value as i64;
        match txout.script.to_highlevel() {
            HighLevel::PayToPubkeyHash(pkh) => {
                let hash160 = Hash160::from_slice(pkh);
                let address = CompactAddress::from_hash160(hash160, 0x00);
                *self.balances.entry(address).or_insert(0) += value;
                Some((address, value))
            }
            HighLevel::PayToScriptHash(pkh) => {
                let hash160 = Hash160::from_slice(pkh);
                let address = CompactAddress::from_hash160(hash160, 0x05);
                *self.balances.entry(address).or_insert(0) += value;
                Some((address, value))
            }
            HighLevel::PayToPubkey(pk) => {
                let hash160 = Hash160::from_data(pk);
                let address = CompactAddress::from_hash160(&hash160, 0x00);
                *self.balances.entry(address).or_insert(0) += value;
                Some((address, value))
            }
            HighLevel::PayToWitnessPubkeyHash(ref w)
            | HighLevel::PayToWitnessScriptHash(ref w)
            | HighLevel::PayToWitnessTaproot(ref w)
            | HighLevel::PayToWitnessGeneral(ref w) => {
                let address = CompactAddress::from_witness_program(w);
                *self.balances.entry(address).or_insert(0) += value;
                Some((address, value))
            }
            _ => None,
        }
    }

    fn done(&mut self) -> Result<Self::DoneItem> {
        Ok((self.balances.len(), String::new()))
    }
}
