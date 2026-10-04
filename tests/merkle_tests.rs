use bitiodine::hash::Hash;
use bitiodine::merkle::MerkleHasher;

#[test]
fn test_merkle_hasher_single_tx() {
    let mut hasher = MerkleHasher::default();
    let txid =
        Hash::from_pretty("4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b")
            .unwrap();
    hasher.add(txid);
    let root = hasher.finish().unwrap();
    assert_eq!(root, txid);
}

#[test]
fn test_merkle_hasher_two_txs() {
    let mut hasher = MerkleHasher::default();
    let tx1 = Hash::from_pretty("4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b")
        .unwrap();
    let tx2 = Hash::from_pretty("0e3e2357e806b6cdb1f70b54c3a3a17b6714ee1f0e68bebb44a74b1efd512098")
        .unwrap();
    hasher.add(tx1);
    hasher.add(tx2);
    let root = hasher.finish().unwrap();

    let mut buf = [0u8; 64];
    buf[..32].copy_from_slice(tx1.as_slice());
    buf[32..].copy_from_slice(tx2.as_slice());
    let expected = Hash::from_data(&buf);
    assert_eq!(root, expected);
}

#[test]
fn streaming_merkle_matches_layer_by_layer_for_odd_and_even_trees() {
    // Exercise repeated right-edge duplication and carries across several levels.
    for count in 0u32..=129 {
        let mut layer: Vec<_> = (0..count)
            .map(|n| Hash::from_data(&n.to_le_bytes()))
            .collect();
        let mut hasher = MerkleHasher::default();
        for &hash in &layer {
            hasher.add(hash);
        }
        while layer.len() > 1 {
            if layer.len() % 2 != 0 {
                layer.push(*layer.last().unwrap());
            }
            layer = layer
                .chunks_exact(2)
                .map(|pair| {
                    let mut bytes = [0; 64];
                    bytes[..32].copy_from_slice(pair[0].as_slice());
                    bytes[32..].copy_from_slice(pair[1].as_slice());
                    Hash::from_data(&bytes)
                })
                .collect();
        }
        assert_eq!(hasher.finish(), layer.first().copied(), "{count} leaves");
    }
}
