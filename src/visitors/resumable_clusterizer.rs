use smallvec::SmallVec;

use crate::address::CompactAddress;
use crate::block::Block;
use crate::cluster_store::ClusterStore;
use crate::error::{ParseError, ParseResult, Result};
use crate::hash::ZERO_HASH;
use crate::transactions::{Transaction, TransactionInput, TransactionOutput};
use crate::visitors::clusterizer::{is_coinjoin, output_address};
use crate::visitors::BlockChainVisitor;

/// Clusters through a persistent store, retaining only the current transaction in RAM.
/// Begins each block transaction; the caller commits it after checking source data.
pub struct ResumableClusterizer<'store> {
    store: &'store mut ClusterStore,
    error: Option<ParseError>,
}

pub struct ResumableTransaction {
    input_addresses: SmallVec<[CompactAddress; 8]>,
    output_values: SmallVec<[u64; 4]>,
    outputs: SmallVec<[(u32, CompactAddress); 4]>,
    output_index: u32,
}

impl<'store> ResumableClusterizer<'store> {
    pub fn with_store(store: &'store mut ClusterStore) -> Self {
        Self { store, error: None }
    }

    pub fn store(&self) -> &ClusterStore {
        self.store
    }

    pub fn store_mut(&mut self) -> &mut ClusterStore {
        self.store
    }
}

impl BlockChainVisitor for ResumableClusterizer<'_> {
    type BlockItem = ();
    type TransactionItem = ResumableTransaction;
    type OutputItem = ();
    type DoneItem = ();

    fn visit_block_begin<'a>(&mut self, block: Block<'a>, height: u64) {
        self.error = self
            .store
            .begin_block(height, block.header().cur_hash())
            .err();
    }

    fn visit_transaction_begin(&mut self, _block: &mut ()) -> Self::TransactionItem {
        ResumableTransaction {
            input_addresses: SmallVec::new(),
            output_values: SmallVec::new(),
            outputs: SmallVec::new(),
            output_index: 0,
        }
    }

    fn visit_transaction_input<'a>(
        &mut self,
        txin: TransactionInput<'a>,
        _block: &mut (),
        transaction: &mut Self::TransactionItem,
        _output: Option<()>,
    ) {
        if self.error.is_some() || txin.prev_hash == &ZERO_HASH {
            return;
        }
        match self.store.spend(txin.prev_hash, txin.prev_index) {
            Ok(Some(address)) if !transaction.input_addresses.contains(&address) => {
                transaction.input_addresses.push(address);
            }
            Ok(_) => {}
            Err(error) => self.error = Some(error),
        }
    }

    fn visit_transaction_output<'a>(
        &mut self,
        txout: TransactionOutput<'a>,
        _block: &mut (),
        transaction: &mut Self::TransactionItem,
    ) -> Option<()> {
        if self.error.is_none() {
            transaction.output_values.push(txout.value);
            if let Some(address) = output_address(txout) {
                transaction
                    .outputs
                    .push((transaction.output_index, address));
            }
        }
        transaction.output_index += 1;
        // Previous outputs live in SQLite; the walk's in-memory OutputMap stays empty.
        None
    }

    fn visit_transaction_end<'a>(
        &mut self,
        tx: Transaction<'a>,
        _block: &mut (),
        mut transaction: Self::TransactionItem,
    ) {
        if self.error.is_some() {
            return;
        }
        if let Err(error) = self.store.replace_outputs(&tx.txid, &transaction.outputs) {
            self.error = Some(error);
            return;
        }
        if transaction.input_addresses.len() > 1 && !is_coinjoin(&mut transaction.output_values) {
            if let Err(error) = self.store.merge(&transaction.input_addresses) {
                self.error = Some(error);
            }
        }
    }

    fn visit_block_end<'a>(
        &mut self,
        _block: Block<'a>,
        _height: u64,
        _block_item: (),
    ) -> ParseResult<()> {
        match self.error.take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    fn done(&mut self) -> Result<()> {
        Ok(())
    }
}
