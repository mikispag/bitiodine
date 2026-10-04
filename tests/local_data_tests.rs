//! Opt-in, read-only checks against existing Bitcoin Core block files.
//! Run with BITIODINE_BLOCKS_DIR=/path/to/blocks cargo test --locked --release
//! --test local_data_tests -- --ignored --nocapture. No RPC or node process is used.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;

use bitcoin::consensus::{deserialize, serialize};
use bitcoin::hashes::Hash as _;
use bitiodine::{
    Block, BlockChainVisitor, Hash, MerkleHasher, Transaction, TransactionInput, TransactionOutput,
};

#[derive(Default)]
struct Differential {
    transactions: usize,
    witness_transactions: usize,
    outputs: usize,
    taproot_outputs: usize,
}

impl BlockChainVisitor for Differential {
    type BlockItem = (bitcoin::Block, usize, MerkleHasher);
    type TransactionItem = (usize, usize, usize);
    type OutputItem = ();
    type DoneItem = ();

    fn new() -> Self {
        Self::default()
    }

    fn visit_block_begin(&mut self, block: Block<'_>, _: u64) -> Self::BlockItem {
        let reference: bitcoin::Block = deserialize(block.0).unwrap();
        assert_eq!(
            block.header().cur_hash().0,
            reference.block_hash().to_byte_array()
        );
        assert_eq!(
            block.header().prev_hash().0,
            reference.header.prev_blockhash.to_byte_array()
        );
        assert_eq!(
            block.header().version(),
            reference.header.version.to_consensus()
        );
        assert_eq!(block.header().timestamp(), reference.header.time);
        assert_eq!(block.header().bits(), reference.header.bits.to_consensus());
        assert_eq!(block.header().nonce(), reference.header.nonce);
        assert!(reference.check_merkle_root());
        assert!(reference.check_witness_commitment());
        (reference, 0, MerkleHasher::default())
    }

    fn visit_transaction_begin(&mut self, _: &mut Self::BlockItem) -> Self::TransactionItem {
        (0, 0, 0)
    }

    fn visit_transaction_input(
        &mut self,
        input: TransactionInput<'_>,
        block: &mut Self::BlockItem,
        counts: &mut Self::TransactionItem,
        _: Option<()>,
    ) {
        let reference = &block.0.txdata[block.1].input[counts.0];
        assert_eq!(
            input.prev_hash.0,
            reference.previous_output.txid.to_byte_array()
        );
        assert_eq!(input.prev_index, reference.previous_output.vout);
        assert_eq!(input.sequence_no, reference.sequence.0);
        assert_eq!(input.script.as_slice(), reference.script_sig.as_bytes());
        assert_eq!(input.slice, serialize(reference));
        counts.0 += 1;
    }

    fn visit_transaction_output(
        &mut self,
        output: TransactionOutput<'_>,
        block: &mut Self::BlockItem,
        counts: &mut Self::TransactionItem,
    ) -> Option<()> {
        let reference = &block.0.txdata[block.1].output[counts.1];
        assert_eq!(output.value, reference.value.to_sat());
        assert_eq!(output.script.as_slice(), reference.script_pubkey.as_bytes());
        assert_eq!(output.slice, serialize(reference));
        self.outputs += 1;
        self.taproot_outputs += usize::from(reference.script_pubkey.is_p2tr());
        counts.1 += 1;
        None
    }

    fn visit_transaction_end(
        &mut self,
        tx: Transaction<'_>,
        block: &mut Self::BlockItem,
        counts: Self::TransactionItem,
    ) {
        let reference = &block.0.txdata[block.1];
        assert_eq!(tx.txid.0, reference.compute_txid().to_byte_array());
        assert_eq!(tx.wtxid().0, reference.compute_wtxid().to_byte_array());
        assert_eq!(tx.version, reference.version.0 as u32);
        assert_eq!(tx.lock_time, reference.lock_time.to_consensus_u32());
        assert_eq!(tx.txins_count as usize, reference.input.len());
        assert_eq!(tx.txouts_count as usize, reference.output.len());
        assert_eq!(
            (counts.0, counts.1),
            (reference.input.len(), reference.output.len())
        );
        assert_eq!(
            counts.2,
            reference
                .input
                .iter()
                .map(|input| input.witness.len())
                .sum::<usize>()
        );
        assert_eq!(tx.slice, serialize(reference));
        self.transactions += 1;
        self.witness_transactions +=
            usize::from(reference.input.iter().any(|i| !i.witness.is_empty()));
        block.2.add(tx.txid);
        block.1 += 1;
    }

    fn visit_transaction_witness(
        &mut self,
        input_index: u64,
        item_index: u64,
        item: &[u8],
        block: &mut Self::BlockItem,
        counts: &mut Self::TransactionItem,
    ) {
        let witness = &block.0.txdata[block.1].input[input_index as usize].witness;
        assert_eq!(Some(item), witness.iter().nth(item_index as usize));
        counts.2 += 1;
    }

    fn visit_block_end(
        &mut self,
        block: Block<'_>,
        _: u64,
        (reference, count, hasher): Self::BlockItem,
    ) -> bitiodine::ParseResult<()> {
        assert_eq!(count, reference.txdata.len());
        assert_eq!(hasher.finish(), Some(*block.header().merkle_root()));
        Ok(())
    }

    fn done(&mut self) -> bitiodine::Result<()> {
        Ok(())
    }
}

fn decode(bytes: &mut [u8], key: &[u8; 8], offset: usize) {
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte ^= key[(offset + i) % key.len()];
    }
}

#[test]
#[ignore = "requires BITIODINE_BLOCKS_DIR pointing to stable, existing mainnet block files"]
fn sampled_local_blocks_match_rust_bitcoin() {
    let directory =
        PathBuf::from(std::env::var_os("BITIODINE_BLOCKS_DIR").expect("set BITIODINE_BLOCKS_DIR"));
    let key: [u8; 8] = match std::fs::read(directory.join("xor.dat")) {
        Ok(bytes) => bytes.try_into().expect("eight-byte XOR key"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => [0; 8],
        Err(error) => panic!("read XOR key: {error}"),
    };
    let mut files: Vec<_> = std::fs::read_dir(&directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|name| {
                    name.strip_prefix("blk")
                        .and_then(|n| n.strip_suffix(".dat"))
                        .is_some_and(|n| n.len() >= 5 && n.bytes().all(|b| b.is_ascii_digit()))
                })
        })
        .collect();
    files.sort();
    assert!(
        files.len() >= 2,
        "requires at least two complete block files"
    );
    // Include early history, both sides of SegWit, and recent blocks. Avoid the
    // final file, which may still be growing or have unused preallocated space.
    let mut indexes = vec![0, 500, 1000, 1500, files.len() / 2, files.len() - 2];
    indexes.retain(|index| *index < files.len() - 1);
    indexes.sort_unstable();
    indexes.dedup();
    let mut visitor = Differential::new();
    let mut block_count = 0;
    for index in indexes {
        let file = File::open(&files[index]).unwrap();
        let before = file.metadata().unwrap();
        let mut reader = BufReader::new(file);
        let mut offset = 0;
        for sample in 0..8 {
            let mut frame = [0; 8];
            reader.read_exact(&mut frame).unwrap();
            decode(&mut frame, &key, offset);
            assert_eq!(&frame[..4], &bitiodine::block::MAINNET_MAGIC.to_le_bytes());
            let len = u32::from_le_bytes(frame[4..].try_into().unwrap()) as usize;
            assert!((81..=4_000_000).contains(&len));
            let mut bytes = vec![0; len];
            reader.read_exact(&mut bytes).unwrap();
            decode(&mut bytes, &key, offset + 8);
            let block = Block(&bytes);
            if index == 0 && sample == 0 {
                assert_eq!(
                    block.header().cur_hash(),
                    Hash::from_pretty(
                        "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f"
                    )
                    .unwrap()
                );
            }
            block
                .walk(&mut visitor, 0, &mut Default::default())
                .unwrap();
            offset += 8 + len;
            block_count += 1;
        }
        let after = reader.get_ref().metadata().unwrap();
        assert_eq!(before.len(), after.len(), "source file changed");
        assert_eq!(
            before.modified().unwrap(),
            after.modified().unwrap(),
            "source file changed"
        );
        eprintln!("Compared 8 blocks from {}", files[index].display());
    }
    visitor.done().unwrap();
    assert!(visitor.transactions > 0);
    if files.len() > 1501 {
        assert!(
            visitor.witness_transactions > 0,
            "sample must exercise witness parsing"
        );
    }
    eprintln!(
        "Compared {block_count} blocks, {} transactions ({} witness), {} outputs ({} Taproot)",
        visitor.transactions,
        visitor.witness_transactions,
        visitor.outputs,
        visitor.taproot_outputs
    );
}
