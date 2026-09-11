use bitiodine::visitors::DumpTxHashes;
use bitiodine::{BlockChainVisitor, Hash, Transaction, Transactions};

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
