use crypto::digest::Digest;
use crypto::sha2::Sha256;
use preamble::*;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct Transactions<'a> {
    pub count: u64,
    pub slice: &'a [u8],
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct Transaction<'a> {
    pub version: u32,
    pub txid: Hash,
    pub txins_count: u64,
    pub txouts_count: u64,
    pub lock_time: u32,
    pub slice: &'a [u8],
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct TransactionInput<'a> {
    pub prev_hash: &'a Hash,
    pub prev_index: u32,
    pub script: Script<'a>,
    pub sequence_no: u32,
    pub slice: &'a [u8],
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct TransactionOutput<'a> {
    pub value: u64,
    pub script: Script<'a>,
    pub slice: &'a [u8],
}

impl<'a> Transactions<'a> {
    pub fn new(mut slice: &[u8]) -> Result<Transactions> {
        let count = read_var_int(&mut slice)?;
        Ok(Transactions { count, slice })
    }

    pub fn walk<V: BlockChainVisitor<'a>>(
        self,
        visitor: &mut V,
        timestamp: u32,
        height: u64,
        block_item: &mut V::BlockItem,
        output_items: &mut HashMap<Hash, VecMap<V::OutputItem>>,
    ) -> ParseResult<()> {
        let mut slice = self.slice;
        for _ in 0..self.count {
            Transaction::read_and_walk(
                &mut slice,
                visitor,
                timestamp,
                height,
                block_item,
                output_items,
            )?;
        }

        assert_eq!(slice.len(), 0);

        Ok(())
    }
}

impl<'a> Transaction<'a> {
    pub fn read_and_walk<V: BlockChainVisitor<'a>>(
        slice: &mut &'a [u8],
        visitor: &mut V,
        timestamp: u32,
        height: u64,
        block_item: &mut V::BlockItem,
        output_items: &mut HashMap<Hash, VecMap<V::OutputItem>>,
    ) -> ParseResult<Transaction<'a>> {
        // Visit the raw transaction before parsing
        let mut transaction_item = visitor.visit_transaction_begin(block_item);

        let mut tx_hash = [0u8; 32];
        let mut sha256_hasher1 = Sha256::new();
        let mut sha256_hasher2 = sha256_hasher1;

        // Save the initial position in two slices
        let mut init_slice = *slice;

        sha256_hasher1.input(&slice[..4]);
        let version = read_u32(slice)?;

        let marker = slice[0];
        let txins_count: u64;
        let mut slice_inputs_and_outputs = *slice;
        if marker == 0x00 {
            // Consume marker
            *slice = &slice[1..];
            let flag = read_u8(slice)?;
            slice_inputs_and_outputs = *slice;
            if flag == 0x01 {
                txins_count = read_var_int(slice)?;
            } else {
                return Err(ParseError::Invalid);
            }
        } else {
            txins_count = read_var_int(slice)?;
        }

        // Read the inputs
        for _ in 0..txins_count {
            let i = TransactionInput::read(slice, timestamp, height)?;
            let mut output_item = None;
            if let HashEntry::Occupied(mut occupied) = output_items.entry(*i.prev_hash) {
                output_item = occupied.get_mut().remove(i.prev_index as usize);
                if occupied.get().len() == 0 {
                    occupied.remove();
                }
            }
            visitor.visit_transaction_input(i, block_item, &mut transaction_item, output_item);
        }

        // Read the outputs
        let txouts_count = read_var_int(slice)?;

        let mut cur_output_items = VecMap::with_capacity(txouts_count as usize);
        for n in 0..txouts_count {
            let o = TransactionOutput::read(slice, timestamp, height)?;
            let output_item =
                visitor.visit_transaction_output(o, block_item, &mut transaction_item);

            if let Some(output_item) = output_item {
                cur_output_items.insert(n as usize, output_item);
            }
        }

        // Hash the transaction data before the witnesses
        let slice_inputs_and_outputs_len = slice_inputs_and_outputs.len();
        sha256_hasher1.input(read_slice(
            &mut slice_inputs_and_outputs,
            slice_inputs_and_outputs_len - slice.len(),
        )?);

        // Read the witnesses
        if marker == 0x00 {
            for _ in 0..txins_count {
                let item_count = read_var_int(slice)?;
                for _ in 0..item_count {
                    let witness_len = read_var_int(slice)? as usize;
                    let _witness = read_slice(slice, witness_len);
                }
            }
        }

        sha256_hasher1.input(&slice[..4]);
        let lock_time = read_u32(slice)?;
        sha256_hasher1.result(&mut tx_hash);
        sha256_hasher2.input(&tx_hash);
        sha256_hasher2.result(&mut tx_hash);

        let init_slice_len = init_slice.len();
        let tx = Transaction {
            version,
            txid: *Hash::from_slice(&tx_hash),
            txins_count,
            txouts_count,
            lock_time,
            slice: read_slice(&mut init_slice, init_slice_len - slice.len())?,
        };

        if cur_output_items.len() > 0 {
            let len = cur_output_items.len();
            cur_output_items.reserve_len_exact(len);
            output_items.insert(*Hash::from_slice(&tx_hash), cur_output_items);
        }

        visitor.visit_transaction_end(tx, block_item, transaction_item);
        Ok(tx)
    }
}

impl<'a> TransactionInput<'a> {
    pub fn read(slice: &mut &'a [u8], timestamp: u32, height: u64) -> Result<TransactionInput<'a>> {
        // Save the initial position
        let mut init_slice = *slice;

        // Read the prev_hash
        let prev_hash = Hash::from_slice(read_array!(slice, 32)?);

        // Read the prev_index
        let prev_index = read_u32(slice)?;

        // Read the script
        let nbytes = read_var_int(slice)? as usize;
        let script = read_slice(slice, nbytes)?;

        // Read the sequence_no
        let sequence_no = read_u32(slice)?;

        let init_slice_len = init_slice.len();
        Ok(TransactionInput {
            prev_hash,
            prev_index,
            script: Script::new(script, timestamp, height),
            sequence_no,
            slice: read_slice(&mut init_slice, init_slice_len - slice.len())?,
        })
    }
}

impl<'a> TransactionOutput<'a> {
    pub fn read(
        slice: &mut &'a [u8],
        timestamp: u32,
        height: u64,
    ) -> Result<TransactionOutput<'a>> {
        // Save the initial position
        let mut init_slice = *slice;

        // Read the value
        let value = read_u64(slice)?;

        // Read the script
        let nbytes = read_var_int(slice)? as usize;
        let script = read_slice(slice, nbytes)?;

        // Return the transaction output
        let init_slice_len = init_slice.len();
        Ok(TransactionOutput {
            value,
            script: Script::new(script, timestamp, height),
            slice: read_slice(&mut init_slice, init_slice_len - slice.len())?,
        })
    }
}
