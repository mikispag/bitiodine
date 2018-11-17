use preamble::*;

pub struct DumpBalances {
    balances: HashMap<(Address, Option<Hash160>), i64>,
}

const OUTPUT_STRING_CAPACITY: usize = 100000000usize;

impl<'a> BlockChainVisitor<'a> for DumpBalances {
    type BlockItem = ();
    type TransactionItem = ();
    type OutputItem = (Address, Option<Hash160>, i64);
    type DoneItem = (usize, String);

    fn new() -> Self {
        Self {
            balances: HashMap::with_capacity(1000000),
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

        match output_item {
            Some((address, hash160, value)) => {
                let prev_balance = self
                    .balances
                    .get(&(address.to_owned(), hash160))
                    .unwrap_or(&0)
                    .to_owned();
                if prev_balance == value {
                    self.balances.remove(&(address.to_owned(), hash160));
                } else {
                    *self
                        .balances
                        .entry((address.to_owned(), hash160))
                        .or_insert(0) -= value;
                }
            }
            None => {}
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
                    .entry((address.to_owned(), Some(*hash160)))
                    .or_insert(0) += value;
                Some((address, Some(*hash160), value))
            }
            HighLevel::PayToScriptHash(pkh) => {
                let hash160 = Hash160::from_slice(pkh);
                let address = Address::from_hash160(hash160, 0x05);
                *self
                    .balances
                    .entry((address.to_owned(), Some(*hash160)))
                    .or_insert(0) += value;
                Some((address, Some(*hash160), value))
            }
            HighLevel::PayToPubkey(pk) => {
                let hash160 = &Hash160::from_data(pk);
                let address = Address::from_hash160(hash160, 0x00);
                *self
                    .balances
                    .entry((address.to_owned(), Some(*hash160)))
                    .or_insert(0) += value;
                Some((address, Some(*hash160), value))
            }
            HighLevel::PayToWitnessPubkeyHash(w) | HighLevel::PayToWitnessScriptHash(w) => {
                let address = Address(w.to_address());
                *self.balances.entry((address.to_owned(), None)).or_insert(0) += value;
                Some((address, None, value))
            }
            _ => None,
        }
    }

    fn done(&mut self) -> Result<Self::DoneItem> {
        let mut output_string = String::with_capacity(OUTPUT_STRING_CAPACITY);

        for (address_tuple, balance) in &self.balances {
            if *balance == 0 {
                continue;
            }
            let address = &address_tuple.0;
            let hash160 = address_tuple.1.unwrap_or_default();
            output_string.push_str(&format!(
                "{:.8},{},{}\n",
                balance.to_owned() as f64 * 10f64.powf(-8f64),
                hash160,
                address
            ));
        }

        Ok((self.balances.len(), output_string))
    }
}
