use std::io::{self, Write};

use crate::block::Block;
use crate::error::{ParseResult, Result};
use crate::script::HighLevel;
use crate::transactions::{Transaction, TransactionOutput};
use crate::visitors::BlockChainVisitor;

/// Writes `txid,vout,payload_hex` CSV rows without retaining OP_RETURN UTXOs.
pub struct DataOutputFinder<'w> {
    writer: Box<dyn Write + 'w>,
    error: Option<io::Error>,
}

impl Default for DataOutputFinder<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'w> DataOutputFinder<'w> {
    pub fn with_writer(writer: impl Write + 'w) -> Self {
        Self {
            writer: Box::new(writer),
            error: None,
        }
    }
}

impl BlockChainVisitor for DataOutputFinder<'_> {
    type BlockItem = ();
    type TransactionItem = (u64, Vec<(u64, String)>);
    type OutputItem = ();
    type DoneItem = ();

    fn new() -> Self {
        Self::with_writer(io::stdout())
    }

    fn visit_block_begin<'a>(&mut self, _block: Block<'a>, _height: u64) {}

    fn visit_transaction_begin(&mut self, _block_item: &mut ()) -> Self::TransactionItem {
        (0, Vec::new())
    }

    fn visit_transaction_output<'a>(
        &mut self,
        txout: TransactionOutput<'a>,
        _block_item: &mut (),
        transaction: &mut Self::TransactionItem,
    ) -> Option<Self::OutputItem> {
        if self.error.is_none() {
            if let HighLevel::DataOutput(data) = txout.script.to_highlevel() {
                transaction.1.push((transaction.0, hex::encode(data)));
            }
        }
        transaction.0 += 1;
        None
    }

    fn visit_transaction_end<'a>(
        &mut self,
        tx: Transaction<'a>,
        _block_item: &mut (),
        transaction: Self::TransactionItem,
    ) {
        for (index, payload) in transaction.1 {
            if let Err(error) = writeln!(self.writer, "{},{},{}", tx.txid, index, payload) {
                self.error = Some(error);
                break;
            }
        }
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
