use bitiodine::error::ParseError;
use bitiodine::visitors::DumpTxHashes;
use bitiodine::{Block, BlockChainVisitor, Hash, Transaction, Transactions};

fn transaction(witness: bool) -> Vec<u8> {
    let mut bytes = 2u32.to_le_bytes().to_vec();
    if witness {
        bytes.extend([0, 1]);
    }
    bytes.push(1);
    bytes.extend([0; 32]);
    bytes.extend(u32::MAX.to_le_bytes());
    bytes.extend([2, 1, 1]);
    bytes.extend(u32::MAX.to_le_bytes());
    bytes.push(1);
    bytes.extend(42u64.to_le_bytes());
    bytes.extend([1, 0x51]);
    if witness {
        bytes.extend([1, 2, 0xaa, 0xbb]);
    }
    bytes.extend(0u32.to_le_bytes());
    bytes
}

#[test]
fn every_truncated_transaction_returns_an_error() {
    for witness in [false, true] {
        let bytes = transaction(witness);
        for end in 0..bytes.len() {
            let mut slice = &bytes[..end];
            assert!(
                Transaction::read_and_walk(
                    &mut slice,
                    &mut DumpTxHashes::new(),
                    0,
                    0,
                    &mut Default::default(),
                    &mut Default::default()
                )
                .is_err(),
                "accepted truncation at {end}"
            );
        }
    }
}

#[test]
fn legacy_and_witness_txids_use_stripped_serialization() {
    let stripped = transaction(false);
    for witness in [false, true] {
        let bytes = transaction(witness);
        let mut slice = bytes.as_slice();
        let tx = Transaction::read_and_walk(
            &mut slice,
            &mut DumpTxHashes::new(),
            0,
            0,
            &mut Default::default(),
            &mut Default::default(),
        )
        .unwrap();
        assert_eq!(tx.txid, Hash::from_data(&stripped));
        assert!(slice.is_empty());
    }
}

#[test]
fn undeclared_transaction_bytes_return_an_error() {
    let bytes = transaction(false);
    let txs = Transactions {
        count: 0,
        slice: &bytes,
    };
    assert!(txs
        .walk(
            &mut DumpTxHashes::new(),
            0,
            0,
            &mut Default::default(),
            &mut Default::default()
        )
        .is_err());
}

#[test]
fn multiple_inputs_outputs_and_witnesses_match_rust_bitcoin() {
    for witness in [false, true] {
        let mut expected: bitcoin::Transaction =
            bitcoin::consensus::deserialize(&transaction(witness)).unwrap();
        expected.input.push(expected.input[0].clone());
        expected.input[1].previous_output.vout = 7;
        // A witness transaction may have an empty stack for an individual input.
        expected.input[1].witness = bitcoin::Witness::default();
        expected.output.push(expected.output[0].clone());
        expected.output[1].value = bitcoin::Amount::from_sat(12345);
        expected.lock_time = bitcoin::absolute::LockTime::from_consensus(500_000_001);
        let bytes = bitcoin::consensus::serialize(&expected);
        let next_transaction = transaction(false);
        let mut combined = bytes.clone();
        combined.extend_from_slice(&next_transaction);
        let mut slice = combined.as_slice();
        let actual = Transaction::read_and_walk(
            &mut slice,
            &mut DumpTxHashes::new(),
            0,
            0,
            &mut Default::default(),
            &mut Default::default(),
        )
        .unwrap();
        assert_eq!(actual.txid.to_string(), expected.compute_txid().to_string());
        assert_eq!(
            actual.wtxid().to_string(),
            expected.compute_wtxid().to_string()
        );
        assert_eq!(actual.version, expected.version.0 as u32);
        assert_eq!(actual.txins_count, 2);
        assert_eq!(actual.txouts_count, 2);
        assert_eq!(actual.lock_time, expected.lock_time.to_consensus_u32());
        assert_eq!(actual.slice, bytes);
        assert_eq!(slice, next_transaction);
    }
}

#[test]
fn unsupported_witness_flags_are_rejected() {
    for flag in [0, 2, 3, 0xff] {
        let mut bytes = transaction(true);
        bytes[5] = flag;
        assert!(bitcoin::consensus::deserialize::<bitcoin::Transaction>(&bytes).is_err());
        assert!(Transaction::read_and_walk(
            &mut bytes.as_slice(),
            &mut DumpTxHashes::new(),
            0,
            0,
            &mut Default::default(),
            &mut Default::default(),
        )
        .is_err());
    }
}

#[test]
fn noncanonical_transaction_counts_and_lengths_are_rejected() {
    for (field, witness, offset) in [
        ("input count", false, 4),
        ("input script length", false, 41),
        ("output count", false, 48),
        ("output script length", false, 57),
        ("witness input count", true, 6),
        ("witness item count", true, 61),
        ("witness item length", true, 62),
    ] {
        let mut bytes = transaction(witness);
        let value = bytes[offset];
        bytes.splice(offset..=offset, [0xfd, value, 0]);
        assert!(bitcoin::consensus::deserialize::<bitcoin::Transaction>(&bytes).is_err());
        assert!(
            matches!(
                Transaction::read_and_walk(
                    &mut bytes.as_slice(),
                    &mut DumpTxHashes::new(),
                    0,
                    0,
                    &mut Default::default(),
                    &mut Default::default(),
                ),
                Err(ParseError::Invalid)
            ),
            "accepted noncanonical {field}"
        );
    }
    assert!(matches!(
        Transactions::new(&[0xfd, 1, 0]),
        Err(ParseError::Invalid)
    ));
    let mut block = vec![0; 80];
    block.extend([0xfd, 1, 0]);
    assert!(matches!(
        Block(&block).transactions(),
        Err(ParseError::Invalid)
    ));
}

#[test]
fn superfluous_witness_serialization_is_rejected() {
    let mut bytes = transaction(true);
    let witness_start = bytes.len() - 8;
    bytes.splice(witness_start..witness_start + 4, [0]);
    assert!(bitcoin::consensus::deserialize::<bitcoin::Transaction>(&bytes).is_err());
    assert!(matches!(
        Transaction::read_and_walk(
            &mut bytes.as_slice(),
            &mut DumpTxHashes::new(),
            0,
            0,
            &mut Default::default(),
            &mut Default::default(),
        ),
        Err(ParseError::Invalid)
    ));
}

#[test]
fn a_single_empty_witness_item_is_accepted() {
    let mut expected: bitcoin::Transaction =
        bitcoin::consensus::deserialize(&transaction(true)).unwrap();
    expected.input[0].witness = bitcoin::Witness::from_slice(&[Vec::<u8>::new()]);
    let bytes = bitcoin::consensus::serialize(&expected);
    let actual = Transaction::read_and_walk(
        &mut bytes.as_slice(),
        &mut DumpTxHashes::new(),
        0,
        0,
        &mut Default::default(),
        &mut Default::default(),
    )
    .unwrap();
    assert_eq!(actual.txid.to_string(), expected.compute_txid().to_string());
    assert_eq!(
        actual.wtxid().to_string(),
        expected.compute_wtxid().to_string()
    );
}

#[test]
fn multibyte_counts_lengths_and_witness_callbacks_match_rust_bitcoin() {
    #[derive(Default)]
    struct WitnessCollector(Vec<(u64, u64, Vec<u8>)>);

    impl BlockChainVisitor for WitnessCollector {
        type BlockItem = ();
        type TransactionItem = ();
        type OutputItem = ();
        type DoneItem = ();

        fn new() -> Self {
            Self::default()
        }
        fn visit_block_begin(&mut self, _: Block<'_>, _: u64) {}
        fn visit_transaction_begin(&mut self, _: &mut ()) {}
        fn visit_transaction_witness(
            &mut self,
            input_index: u64,
            item_index: u64,
            item: &[u8],
            _: &mut (),
            _: &mut (),
        ) {
            self.0.push((input_index, item_index, item.to_vec()));
        }
        fn done(&mut self) -> bitiodine::error::Result<()> {
            Ok(())
        }
    }

    let mut expected: bitcoin::Transaction =
        bitcoin::consensus::deserialize(&transaction(false)).unwrap();
    expected.input.resize(253, expected.input[0].clone());
    expected.output.resize(253, expected.output[0].clone());
    expected.input[0].script_sig = bitcoin::ScriptBuf::from_bytes(vec![0x51; 253]);
    expected.output[0].script_pubkey = bitcoin::ScriptBuf::from_bytes(vec![0x51; 253]);
    let mut items = vec![Vec::<u8>::new(); 253];
    items[0] = vec![0x42; 253];
    expected.input[1].witness = bitcoin::Witness::from_slice(&items);
    expected.input[252].witness = bitcoin::Witness::from_slice(&[vec![0x11; 32]]);
    let bytes = bitcoin::consensus::serialize(&expected);
    let mut visitor = WitnessCollector::new();
    let actual = Transaction::read_and_walk(
        &mut bytes.as_slice(),
        &mut visitor,
        0,
        0,
        &mut (),
        &mut Default::default(),
    )
    .unwrap();
    assert_eq!(actual.txins_count, 253);
    assert_eq!(actual.txouts_count, 253);
    assert_eq!(actual.txid.to_string(), expected.compute_txid().to_string());
    assert_eq!(
        actual.wtxid().to_string(),
        expected.compute_wtxid().to_string()
    );
    let mut expected_items: Vec<_> = items
        .into_iter()
        .enumerate()
        .map(|(index, item)| (1, index as u64, item))
        .collect();
    expected_items.push((252, 0, vec![0x11; 32]));
    assert_eq!(visitor.0, expected_items);
}

#[test]
fn oversized_script_and_witness_lengths_do_not_wrap() {
    for offset in [43, 59, 62] {
        let mut bytes = transaction(true);
        let oversized = bitcoin::consensus::serialize(&bitcoin::consensus::encode::VarInt(1 << 32));
        bytes.splice(offset..=offset, oversized);
        let result = Transaction::read_and_walk(
            &mut bytes.as_slice(),
            &mut DumpTxHashes::new(),
            0,
            0,
            &mut Default::default(),
            &mut Default::default(),
        );
        if usize::BITS < 64 {
            assert!(matches!(result, Err(ParseError::Invalid)));
        } else {
            assert!(matches!(result, Err(ParseError::Eof)));
        }
    }
}
