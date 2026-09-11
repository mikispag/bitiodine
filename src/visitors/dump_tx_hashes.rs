use std::io::{self, Write};

use crate::block::Block;
use crate::error::{ParseResult, Result};
use crate::transactions::Transaction;
use crate::visitors::BlockChainVisitor;

pub struct DumpTxHashes<'w> {
    writer: Box<dyn Write + 'w>,
    error: Option<io::Error>,
}

impl Default for DumpTxHashes<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'w> DumpTxHashes<'w> {
    pub fn with_writer(writer: impl Write + 'w) -> Self {
        Self {
            writer: Box::new(writer),
            error: None,
        }
    }
}

impl BlockChainVisitor for DumpTxHashes<'_> {
    type BlockItem = u64;
    type TransactionItem = ();
    type OutputItem = ();
    type DoneItem = ();

    fn new() -> Self {
        Self::with_writer(io::stdout())
    }

    fn visit_block_begin<'a>(&mut self, block: Block<'a>, height: u64) -> Self::BlockItem {
        if height > 480_000 && height % 1000 == 0 {
            // Block traversal reports a malformed transaction count itself.
            if let Ok(transactions) = block.transactions() {
                if let Err(error) = writeln!(
                    self.writer,
                    "Block {} - {} transactions",
                    height, transactions.count
                ) {
                    self.error = Some(error);
                }
            }
        }
        height
    }

    fn visit_transaction_begin(&mut self, _block_item: &mut Self::BlockItem) {}

    fn visit_transaction_end<'a>(
        &mut self,
        tx: Transaction<'a>,
        block_item: &mut Self::BlockItem,
        _tx_item: Self::TransactionItem,
    ) {
        if self.error.is_none() && *block_item > 480_000 && *block_item % 1000 == 0 {
            if let Err(error) = writeln!(self.writer, "Transaction {}", tx.txid) {
                self.error = Some(error);
            }
        }
    }

    fn visit_block_end<'a>(
        &mut self,
        _block: Block<'a>,
        _height: u64,
        _block_item: u64,
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
