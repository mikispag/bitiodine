use bitiodine::{Block, BlockChain, BlockChainVisitor, Hash, Transaction, ZERO_HASH};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new(records: &[Vec<u8>], key: Option<[u8; 8]>, padding: usize) -> Self {
        let path = std::env::temp_dir().join(format!(
            "bitiodine-chain-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        let mut bytes = records.concat();
        if let Some(key) = key {
            for (i, byte) in bytes.iter_mut().enumerate() {
                *byte ^= key[i % 8];
            }
            std::fs::write(path.join("xor.dat"), key).unwrap();
        }
        bytes.resize(bytes.len() + padding, 0);
        std::fs::write(path.join("blk00000.dat"), bytes).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[derive(Default)]
struct Capture {
    nonces: Vec<u32>,
    txids: Vec<Hash>,
}
impl BlockChainVisitor for Capture {
    type BlockItem = ();
    type TransactionItem = ();
    type OutputItem = ();
    type DoneItem = ();
    fn new() -> Self {
        Self::default()
    }
    fn visit_block_begin(&mut self, block: Block<'_>, _: u64) {
        self.nonces.push(block.header().nonce());
    }
    fn visit_transaction_begin(&mut self, _: &mut ()) {}
    fn visit_transaction_end(&mut self, tx: Transaction<'_>, _: &mut (), _: ()) {
        self.txids.push(tx.txid);
    }
    fn done(&mut self) -> bitiodine::Result<()> {
        Ok(())
    }
}

fn record(previous: Hash, nonce: u32, bits: u32, locktime: u32) -> (Hash, Vec<u8>) {
    let mut tx = 1u32.to_le_bytes().to_vec();
    tx.push(1);
    tx.extend([0; 32]);
    tx.extend(u32::MAX.to_le_bytes());
    tx.extend([2, 1, nonce as u8]);
    tx.extend(u32::MAX.to_le_bytes());
    tx.push(1);
    tx.extend(5_000_000_000u64.to_le_bytes());
    tx.extend([1, 0x51]);
    tx.extend(locktime.to_le_bytes());
    let mut body = 1u32.to_le_bytes().to_vec();
    body.extend(previous.0);
    body.extend(Hash::from_data(&tx).0);
    body.extend(1_700_000_000u32.to_le_bytes());
    body.extend(bits.to_le_bytes());
    body.extend(nonce.to_le_bytes());
    let hash = Hash::from_data(&body);
    body.push(1);
    body.extend(tx);
    let mut frame = 0xd9b4bef9u32.to_le_bytes().to_vec();
    frame.extend((body.len() as u32).to_le_bytes());
    frame.extend(body);
    (hash, frame)
}

fn walk(fixture: &Fixture) -> bitiodine::ParseResult<(u64, Hash, Capture)> {
    let chain = BlockChain::read(&fixture.0, 0)?;
    let mut capture = Capture::new();
    let (height, hash, _) = chain.walk(&mut capture)?;
    Ok((height, hash, capture))
}

#[test]
fn fork_selection_is_independent_of_disk_order() {
    let (g, genesis) = record(ZERO_HASH, 0, 0x1d00ffff, 0);
    let (a, a1) = record(g, 1, 0x1d00ffff, 0);
    let (_, a2) = record(a, 2, 0x1d00ffff, 0);
    let (b, b1) = record(g, 3, 0x1d00ffff, 0);
    let (b, b2) = record(b, 4, 0x1d00ffff, 0);
    let (tip, b3) = record(b, 5, 0x1d00ffff, 0);
    for records in [
        vec![
            genesis.clone(),
            a1.clone(),
            b1.clone(),
            a2.clone(),
            b2.clone(),
            b3.clone(),
        ],
        vec![b3, a2, b2, b1, a1, genesis],
    ] {
        let (count, hash, capture) = walk(&Fixture::new(&records, None, 0)).unwrap();
        assert_eq!((count, hash), (4, tip));
        assert_eq!(capture.nonces, [0, 3, 4, 5]);
    }
}

#[test]
fn greater_work_wins_over_greater_height() {
    let (g, genesis) = record(ZERO_HASH, 0, 0x1d00ffff, 0);
    let (a, a1) = record(g, 1, 0x1d00ffff, 0);
    let (_, a2) = record(a, 2, 0x1d00ffff, 0);
    let (tip, b1) = record(g, 3, 0x1c00ffff, 0);
    let (count, hash, capture) = walk(&Fixture::new(&[genesis, a1, a2, b1], None, 0)).unwrap();
    assert_eq!((count, hash), (2, tip));
    assert_eq!(capture.nonces, [0, 3]);
}

#[test]
fn xor_preserves_zero_ciphertext_and_preallocated_padding() {
    let (_, bytes) = record(ZERO_HASH, 6, 0x1d00ffff, 0x01000000);
    let (_, tip, plain) = walk(&Fixture::new(std::slice::from_ref(&bytes), None, 0)).unwrap();
    for padding in [0, 3, 1024] {
        let (_, decoded_tip, decoded) = walk(&Fixture::new(
            std::slice::from_ref(&bytes),
            Some([1; 8]),
            padding,
        ))
        .unwrap();
        assert_eq!(decoded_tip, tip);
        assert_eq!(decoded.txids, plain.txids);
    }
    // A valid frame header itself can encode to eight zero bytes.
    let key: [u8; 8] = bytes[..8].try_into().unwrap();
    let (_, _, decoded) = walk(&Fixture::new(&[bytes], Some(key), 32)).unwrap();
    assert_eq!(decoded.txids, plain.txids);
}

#[test]
fn vanished_file_is_an_error() {
    let (_, bytes) = record(ZERO_HASH, 0, 0x1d00ffff, 0);
    let fixture = Fixture::new(&[bytes], None, 0);
    let chain = BlockChain::read(&fixture.0, 0).unwrap();
    std::fs::remove_file(fixture.0.join("blk00000.dat")).unwrap();
    assert!(chain.walk(&mut Capture::new()).is_err());
}

#[test]
fn disconnected_input_is_not_a_successful_empty_walk() {
    let (_, bytes) = record(Hash([1; 32]), 1, 0x1d00ffff, 0);
    assert!(walk(&Fixture::new(&[bytes], None, 0)).is_err());
}

#[test]
fn equal_work_tips_are_rejected_before_visiting() {
    let (g, genesis) = record(ZERO_HASH, 0, 0x1d00ffff, 0);
    let (_, a) = record(g, 1, 0x1d00ffff, 0);
    let (_, b) = record(g, 2, 0x1d00ffff, 0);
    let fixture = Fixture::new(&[genesis, a, b], None, 0);
    let chain = BlockChain::read(&fixture.0, 0).unwrap();
    let mut visitor = Capture::new();
    assert!(chain.walk(&mut visitor).is_err());
    assert!(visitor.nonces.is_empty());
}

#[test]
fn cross_file_order_and_duplicate_records_do_not_change_the_chain() {
    let (g, genesis) = record(ZERO_HASH, 0, 0x1d00ffff, 0);
    let (_, child) = record(g, 1, 0x1d00ffff, 0);
    let fixture = Fixture::new(std::slice::from_ref(&child), None, 0);
    std::fs::write(fixture.0.join("blk00001.dat"), [genesis, child].concat()).unwrap();
    let (height, _, visitor) = walk(&fixture).unwrap();
    assert_eq!(height, 2);
    assert_eq!(visitor.nonces, [0, 1]);
}

#[test]
fn invalid_keys_files_and_targets_return_errors() {
    for bits in [0, 0x1d80ffff, 0x2300ffff, 0x22010000] {
        let (_, bytes) = record(ZERO_HASH, 0, bits, 0);
        assert!(walk(&Fixture::new(&[bytes], None, 0)).is_err());
    }
    let (_, bytes) = record(ZERO_HASH, 0, 0x1d00ffff, 0);
    let fixture = Fixture::new(&[bytes], None, 0);
    std::fs::write(fixture.0.join("xor.dat"), [1; 7]).unwrap();
    assert!(BlockChain::read(&fixture.0, 0).is_err());
    std::fs::remove_file(fixture.0.join("xor.dat")).unwrap();
    std::fs::rename(
        fixture.0.join("blk00000.dat"),
        fixture.0.join("blk00001.dat"),
    )
    .unwrap();
    assert!(BlockChain::read(&fixture.0, 0).is_err());
    std::fs::create_dir(fixture.0.join("blk00000.dat")).unwrap();
    assert!(BlockChain::read(&fixture.0, 0).is_err());
}

#[test]
fn merkle_mismatch_is_a_returned_error() {
    let (_, mut bytes) = record(ZERO_HASH, 0, 0x1d00ffff, 0);
    bytes[8 + 36] ^= 1;
    let fixture = Fixture::new(&[bytes], None, 0);
    let chain = BlockChain::read(&fixture.0, 0).unwrap();
    assert!(chain
        .walk(&mut bitiodine::visitors::MerkleVisitor::new())
        .is_err());
}

#[test]
fn trusted_tip_overrides_unvalidated_higher_work_candidates() {
    let (g, genesis) = record(ZERO_HASH, 0, 0x1d00ffff, 0);
    let (accepted, a) = record(g, 1, 0x1d00ffff, 0);
    let (_, rejected_candidate) = record(g, 2, 0x1c00ffff, 0);
    let fixture = Fixture::new(&[rejected_candidate, a, genesis], None, 0);
    let chain = BlockChain::read(&fixture.0, 0).unwrap().with_tip(accepted);
    let mut visitor = Capture::new();
    let (height, tip, _) = chain.walk(&mut visitor).unwrap();
    assert_eq!((height, tip), (2, accepted));
    assert_eq!(visitor.nonces, [0, 1]);
    let chain = BlockChain::read(&fixture.0, 0)
        .unwrap()
        .with_tip(Hash([0xab; 32]));
    let mut visitor = Capture::new();
    assert!(chain.walk(&mut visitor).is_err());
    assert!(visitor.nonces.is_empty());
}

#[test]
fn nonuniform_xor_handles_record_offsets_and_restarts_at_each_file() {
    let (_, mut genesis) = record(ZERO_HASH, 0, 0x1d00ffff, 0);
    // Extend the output script before locktime so the next frame is unaligned.
    let script_length = genesis.len() - 6;
    genesis[script_length] = 2;
    genesis.insert(genesis.len() - 4, 0x51);
    let body_len = (genesis.len() - 8) as u32;
    genesis[4..8].copy_from_slice(&body_len.to_le_bytes());
    let txid = Hash::from_data(&genesis[8 + 80 + 1..]);
    genesis[8 + 36..8 + 68].copy_from_slice(&txid.0);
    let g = Hash::from_data(&genesis[8..8 + 80]);
    let (a, first) = record(g, 1, 0x1d00ffff, 1);
    let (tip, second) = record(a, 2, 0x1d00ffff, 2);
    assert_ne!(genesis.len() % 8, 0);
    let key = [0x00, 0x11, 0x23, 0x45, 0x67, 0x89, 0xab, 0xff];
    let fixture = Fixture::new(&[genesis, second], Some(key), 5);
    let mut encrypted = first;
    for (offset, byte) in encrypted.iter_mut().enumerate() {
        *byte ^= key[offset % key.len()];
    }
    encrypted.extend([0; 13]);
    std::fs::write(fixture.0.join("blk00001.dat"), encrypted).unwrap();
    let (count, selected_tip, visitor) = walk(&fixture).unwrap();
    assert_eq!((count, selected_tip), (3, tip));
    assert_eq!(visitor.nonces, [0, 1, 2]);
}

#[test]
fn malformed_frames_fail_before_visiting_any_block() {
    let (_, valid) = record(ZERO_HASH, 0, 0x1d00ffff, 0);
    let mut cases = Vec::new();
    for size in [1, 7, 8, valid.len() - 1] {
        cases.push(valid[..size].to_vec());
    }
    for size in [0u32, 80, 4_000_001, u32::MAX] {
        let mut invalid = valid.clone();
        invalid[4..8].copy_from_slice(&size.to_le_bytes());
        cases.push(invalid);
    }
    let mut wrong_magic = valid.clone();
    wrong_magic[0] ^= 1;
    cases.push(wrong_magic);
    let mut nonzero_padding = valid;
    nonzero_padding.extend([0; 16]);
    nonzero_padding.push(1);
    cases.push(nonzero_padding);
    for bytes in cases {
        let fixture = Fixture::new(&[bytes], None, 0);
        let chain = BlockChain::read(&fixture.0, 0).unwrap();
        let mut visitor = Capture::new();
        assert!(chain.walk(&mut visitor).is_err());
        assert!(visitor.nonces.is_empty());
    }
}

#[test]
fn file_size_changes_after_discovery_fail_before_visiting() {
    let (_, bytes) = record(ZERO_HASH, 0, 0x1d00ffff, 0);
    for new_size in [bytes.len() - 1, bytes.len() + 1] {
        let fixture = Fixture::new(std::slice::from_ref(&bytes), None, 0);
        let chain = BlockChain::read(&fixture.0, 0).unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .open(fixture.0.join("blk00000.dat"))
            .unwrap()
            .set_len(new_size as u64)
            .unwrap();
        let mut visitor = Capture::new();
        let error = chain.walk(&mut visitor).unwrap_err();
        assert!(error.to_string().contains("changed during analysis"));
        assert!(visitor.nonces.is_empty());
    }
}

#[test]
fn requested_tip_with_missing_ancestors_does_not_fall_back_to_connected_chain() {
    let (_, genesis) = record(ZERO_HASH, 0, 0x1d00ffff, 0);
    let (tip, disconnected) = record(Hash([0x77; 32]), 1, 0x1d00ffff, 0);
    let fixture = Fixture::new(&[genesis, disconnected], None, 0);
    let chain = BlockChain::read(&fixture.0, 0).unwrap().with_tip(tip);
    let mut visitor = Capture::new();
    let error = chain.walk(&mut visitor).unwrap_err();
    assert!(error.to_string().contains("missing ancestors"));
    assert!(visitor.nonces.is_empty());
}

#[test]
fn index_and_traversal_errors_identify_the_file_and_record_offset() {
    let (g, genesis) = record(ZERO_HASH, 0, 0x1d00ffff, 0);
    let (_, mut child) = record(g, 1, 0x1d00ffff, 0);
    child[0] ^= 1;
    let fixture = Fixture::new(&[genesis.clone(), child], None, 0);
    let error = BlockChain::read(&fixture.0, 0)
        .unwrap()
        .walk(&mut Capture::new())
        .unwrap_err()
        .to_string();
    assert!(error.contains("blk00000.dat"), "{error}");
    assert!(
        error.contains(&format!("byte offset {}", genesis.len())),
        "{error}"
    );

    let (_, mut child) = record(g, 1, 0x1d00ffff, 0);
    child[8 + 80] = 2; // Header indexes successfully, but the second transaction is missing.
    let fixture = Fixture::new(&[genesis.clone(), child], None, 0);
    let error = BlockChain::read(&fixture.0, 0)
        .unwrap()
        .walk(&mut Capture::new())
        .unwrap_err()
        .to_string();
    assert!(error.contains("blk00000.dat"), "{error}");
    assert!(
        error.contains(&format!("byte offset {}", genesis.len() + 8)),
        "{error}"
    );
    assert!(error.contains("unexpected end of input"), "{error}");
}
