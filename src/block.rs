use std::collections::HashMap;
use vec_map::VecMap;

use crate::buffer_operations::{read_array, read_slice, read_u32};
use crate::error::{ParseError, ParseResult, Result};
use crate::hash::Hash;
use crate::header::BlockHeader;
use crate::transactions::Transactions;
use crate::visitors::BlockChainVisitor;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct Block<'a>(pub &'a [u8]);

impl<'a> Block<'a> {
    pub fn read(slice: &mut &'a [u8]) -> ParseResult<Option<Block<'a>>> {
        while !slice.is_empty() && slice[0] == 0 {
            *slice = &slice[1..];
        }
        if slice.is_empty() {
            Ok(None)
        } else {
            let block_magic = read_u32(slice)?;
            match block_magic {
                // Incomplete blk file
                0x00 => Ok(None),
                // Bitcoin magic value
                0xd9b4bef9 => {
                    let block_len = read_u32(slice)? as usize;
                    if block_len < 80 {
                        Err(ParseError::Eof)
                    } else {
                        Ok(Some(Block(read_slice(slice, block_len)?)))
                    }
                }
                _ => Err(ParseError::Invalid),
            }
        }
    }

    pub fn header(&self) -> BlockHeader<'a> {
        let mut slice = self.0;
        BlockHeader::new(read_array::<80>(&mut slice).unwrap())
    }

    pub fn transactions(&self) -> Result<Transactions<'a>> {
        Transactions::new(&self.0[80..])
    }

    pub fn walk<V: BlockChainVisitor<'a>>(
        &self,
        visitor: &mut V,
        height: u64,
        output_items: &mut HashMap<Hash, VecMap<V::OutputItem>>,
    ) -> ParseResult<()> {
        let header = self.header();
        let mut block_item = visitor.visit_block_begin(*self, height);
        self.transactions()?.walk(
            visitor,
            header.timestamp(),
            height,
            &mut block_item,
            output_items,
        )?;
        visitor.visit_block_end(*self, height, block_item);
        Ok(())
    }
}
