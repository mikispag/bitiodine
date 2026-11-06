use bitiodine::visitors::clusterizer::DisjointSet;

#[test]
fn test_disjoint_set_union_find() {
    let mut ds: DisjointSet<&str> = DisjointSet::new();

    ds.make_set("addr1");
    ds.make_set("addr2");
    ds.make_set("addr3");
    ds.make_set("addr4");

    assert_eq!(ds.size(), 4);

    // Duplicate make_set should not increase size
    let tag = ds.make_set("addr1");
    assert_eq!(ds.size(), 4);
    assert_eq!(ds.find(&"addr1"), Some(tag));

    // Initial roots should be distinct
    let r1 = ds.find(&"addr1").unwrap();
    let r2 = ds.find(&"addr2").unwrap();
    let r3 = ds.find(&"addr3").unwrap();
    let _r4 = ds.find(&"addr4").unwrap();

    assert_ne!(r1, r2);
    assert_ne!(r2, r3);

    // Union addr1 and addr2
    ds.union(&"addr1", &"addr2").unwrap();
    assert_eq!(ds.find(&"addr1"), ds.find(&"addr2"));

    // Union addr3 and addr4
    ds.union(&"addr3", &"addr4").unwrap();
    assert_eq!(ds.find(&"addr3"), ds.find(&"addr4"));
    assert_ne!(ds.find(&"addr1"), ds.find(&"addr3"));

    // Union the two clusters together
    ds.union(&"addr2", &"addr3").unwrap();
    assert_eq!(ds.find(&"addr1"), ds.find(&"addr4"));

    // Finalize
    ds.finalize();
    assert_eq!(ds.find(&"addr1"), ds.find(&"addr4"));
}

#[test]
fn test_clusterizer_deterministic_representatives() {
    use bitiodine::address::CompactAddress;
    use bitiodine::hash160::Hash160;
    use bitiodine::visitors::clusterizer::Clusterizer;

    let mut clusterizer = Clusterizer::new();
    let addr_a = CompactAddress::from_hash160(&Hash160([0x11; 20]), 0x00);
    let addr_b = CompactAddress::from_hash160(&Hash160([0x22; 20]), 0x00);
    let addr_c = CompactAddress::from_hash160(&Hash160([0x33; 20]), 0x05);
    let addr_d = CompactAddress::from_hash160(&Hash160([0x44; 20]), 0x05);

    clusterizer.clusters.make_set(addr_b);
    clusterizer.clusters.make_set(addr_a);
    clusterizer.clusters.union(&addr_b, &addr_a);

    clusterizer.clusters.make_set(addr_d);
    clusterizer.clusters.make_set(addr_c);
    clusterizer.clusters.union(&addr_d, &addr_c);

    clusterizer.clusters.finalize();

    let mut csv_buf = Vec::new();
    clusterizer.write_csv(&mut csv_buf).unwrap();
    let csv = String::from_utf8(csv_buf).unwrap();

    let rep_ab = if addr_a < addr_b { addr_a } else { addr_b };
    let rep_cd = if addr_c < addr_d { addr_c } else { addr_d };

    let expected_entries = vec![
        (addr_a, rep_ab),
        (addr_b, rep_ab),
        (addr_c, rep_cd),
        (addr_d, rep_cd),
    ];

    // Row order is map-iteration order (unsorted); compare content as a set.
    let mut actual_lines: Vec<&str> = csv.lines().collect();
    actual_lines.sort_unstable();
    let mut expected_lines: Vec<String> = expected_entries
        .into_iter()
        .map(|(a, rep)| format!("{},{}", a, rep))
        .collect();
    expected_lines.sort_unstable();

    assert_eq!(actual_lines, expected_lines);
}

#[test]
fn csv_uses_canonical_roots_before_finalization() {
    use bitiodine::address::CompactAddress;
    use bitiodine::hash160::Hash160;
    use bitiodine::visitors::Clusterizer;

    let mut clusterizer = Clusterizer::new();
    let addresses: Vec<_> = (1..=4)
        .map(|n| CompactAddress::from_hash160(&Hash160([n; 20]), 0))
        .collect();
    for address in &addresses {
        clusterizer.clusters.make_set(*address);
    }
    clusterizer.clusters.union(&addresses[0], &addresses[1]);
    clusterizer.clusters.union(&addresses[2], &addresses[3]);
    clusterizer.clusters.union(&addresses[1], &addresses[3]);

    let mut csv = Vec::new();
    clusterizer.write_csv(&mut csv).unwrap();
    let representative = addresses.iter().min().unwrap().to_string();
    let csv = String::from_utf8(csv).unwrap();
    assert_eq!(csv.lines().count(), 4);
    for row in csv.lines() {
        assert_eq!(row.split_once(',').unwrap().1, representative);
    }
}

#[test]
fn coinjoin_filter_prevents_bridging_preexisting_clusters() {
    use bitiodine::address::CompactAddress;
    use bitiodine::hash::Hash;
    use bitiodine::hash160::Hash160;
    use bitiodine::transactions::Transaction;
    use bitiodine::visitors::clusterizer::{Clusterizer, SpendRules, TransactionClusterState};
    use bitiodine::visitors::BlockChainVisitor;

    let [a, b, c, d] =
        [1, 2, 3, 4].map(|byte| CompactAddress::from_hash160(&Hash160([byte; 20]), 0));
    let mut visitor = Clusterizer::new();
    let transaction = Transaction {
        version: 1,
        txid: Hash::from_data(b"co-spend"),
        txins_count: 2,
        txouts_count: 3,
        lock_time: 0,
        slice: &[],
    };
    for (inputs, amounts) in [
        ([a, b], [1, 2, 3]),
        ([c, d], [1, 2, 3]),
        ([b, c], [7, 7, 7]),
    ] {
        visitor.visit_transaction_end(
            transaction,
            &mut SpendRules::default(),
            TransactionClusterState {
                input_addresses: inputs.into_iter().collect(),
                output_values: amounts.into_iter().collect(),
            },
        );
    }
    assert_eq!(visitor.clusters.find(&a), visitor.clusters.find(&b));
    assert_eq!(visitor.clusters.find(&c), visitor.clusters.find(&d));
    assert_ne!(visitor.clusters.find(&a), visitor.clusters.find(&d));

    // Three zero-valued outputs must not suppress an otherwise eligible merge.
    visitor.visit_transaction_end(
        transaction,
        &mut SpendRules::default(),
        TransactionClusterState {
            input_addresses: [b, c].into_iter().collect(),
            output_values: [0, 0, 0].into_iter().collect(),
        },
    );
    assert_eq!(visitor.clusters.find(&a), visitor.clusters.find(&d));
}

mod spend_rules {
    use bitcoin_bech32::constants::Network;
    use bitcoin_bech32::WitnessProgram;
    use bitiodine::address::CompactAddress;
    use bitiodine::block::Block;
    use bitiodine::hash::Hash;
    use bitiodine::hash160::Hash160;
    use bitiodine::script::Script;
    use bitiodine::transactions::{Transaction, TransactionInput};
    use bitiodine::visitors::clusterizer::{
        SpendRules, BIP16_ACTIVATION_TIME, SEGWIT_ACTIVATION_HEIGHT, TAPROOT_ACTIVATION_HEIGHT,
    };
    use bitiodine::visitors::{BlockChainVisitor, Clusterizer};

    const SEGWIT_ERA_TIME: u32 = 1_503_539_857;

    fn script(bytes: &[u8]) -> Script<'_> {
        Script::new(bytes, SEGWIT_ERA_TIME, SEGWIT_ACTIVATION_HEIGHT)
    }

    fn witness(version: u8, program: &[u8]) -> CompactAddress {
        let opcode = if version == 0 { 0 } else { 0x50 + version };
        let mut bytes = vec![opcode, program.len() as u8];
        bytes.extend(program);
        let program = WitnessProgram::from_scriptpubkey(&bytes, Network::Bitcoin).unwrap();
        CompactAddress::from_witness_program(&program)
    }

    fn p2sh() -> CompactAddress {
        CompactAddress::from_hash160(&Hash160([2; 20]), 0x05)
    }

    #[test]
    fn spends_count_only_once_their_output_type_is_enforced() {
        let empty = script(&[]);
        let p2pkh = CompactAddress::from_hash160(&Hash160([1; 20]), 0x00);
        let p2wpkh = witness(0, &[3; 20]);
        let p2tr = witness(1, &[4; 32]);
        let v1_short = witness(1, &[5; 20]);
        let v2 = witness(2, &[6; 32]);

        let before_bip16 = SpendRules::at(BIP16_ACTIVATION_TIME - 1, 170_000);
        let after_bip16 = SpendRules::at(BIP16_ACTIVATION_TIME, 180_000);
        assert!(before_bip16.spend_is_authenticated(&p2pkh, &empty));
        assert!(!before_bip16.spend_is_authenticated(&p2sh(), &empty));
        assert!(after_bip16.spend_is_authenticated(&p2sh(), &empty));

        let before_segwit = SpendRules::at(SEGWIT_ERA_TIME, SEGWIT_ACTIVATION_HEIGHT - 1);
        let after_segwit = SpendRules::at(SEGWIT_ERA_TIME, SEGWIT_ACTIVATION_HEIGHT);
        assert!(!before_segwit.spend_is_authenticated(&p2wpkh, &empty));
        assert!(after_segwit.spend_is_authenticated(&p2wpkh, &empty));

        let before_taproot = SpendRules::at(SEGWIT_ERA_TIME, TAPROOT_ACTIVATION_HEIGHT - 1);
        let after_taproot = SpendRules::at(SEGWIT_ERA_TIME, TAPROOT_ACTIVATION_HEIGHT);
        assert!(!before_taproot.spend_is_authenticated(&p2tr, &empty));
        assert!(after_taproot.spend_is_authenticated(&p2tr, &empty));
        assert!(!after_taproot.spend_is_authenticated(&v1_short, &empty));
        assert!(!after_taproot.spend_is_authenticated(&v2, &empty));
    }

    #[test]
    fn wrapped_witness_programs_count_only_under_segwit() {
        let mut wrapped_v0 = vec![22, 0x00, 20];
        wrapped_v0.extend([7; 20]);
        let mut wrapped_v1 = vec![34, 0x51, 32];
        wrapped_v1.extend([8; 32]);
        // OP_0 <signature> <1-of-1 multisig redeem script>
        let mut redeem = vec![0x51, 33, 0x02];
        redeem.extend([9; 32]);
        redeem.extend([0x51, 0xae]);
        let mut multisig = vec![0x00, 71];
        multisig.extend([0x30; 71]);
        multisig.push(redeem.len() as u8);
        multisig.extend(&redeem);

        let between = SpendRules::at(BIP16_ACTIVATION_TIME, SEGWIT_ACTIVATION_HEIGHT - 1);
        let segwit = SpendRules::at(SEGWIT_ERA_TIME, SEGWIT_ACTIVATION_HEIGHT);
        let taproot = SpendRules::at(SEGWIT_ERA_TIME, TAPROOT_ACTIVATION_HEIGHT);
        assert!(between.spend_is_authenticated(&p2sh(), &script(&multisig)));
        assert!(!between.spend_is_authenticated(&p2sh(), &script(&wrapped_v0)));
        assert!(segwit.spend_is_authenticated(&p2sh(), &script(&wrapped_v0)));
        assert!(!taproot.spend_is_authenticated(&p2sh(), &script(&wrapped_v1)));
    }

    fn co_spend(height: u64, addresses: &[CompactAddress]) -> Clusterizer {
        let mut header = [0; 80];
        header[68..72].copy_from_slice(&SEGWIT_ERA_TIME.to_le_bytes());
        let mut clusterizer = Clusterizer::new();
        let mut rules = clusterizer.visit_block_begin(Block(&header), height);
        let mut transaction = clusterizer.visit_transaction_begin(&mut rules);
        let prev_hash = Hash::from_data(b"funding transaction");
        for (index, address) in addresses.iter().enumerate() {
            let input = TransactionInput {
                prev_hash: &prev_hash,
                prev_index: index as u32,
                script: script(&[]),
                sequence_no: u32::MAX,
                slice: &[],
            };
            clusterizer.visit_transaction_input(
                input,
                &mut rules,
                &mut transaction,
                Some(*address),
            );
        }
        let tx = Transaction {
            version: 2,
            txid: Hash::from_data(b"co-spend"),
            txins_count: addresses.len() as u64,
            txouts_count: 0,
            lock_time: 0,
            slice: &[],
        };
        clusterizer.visit_transaction_end(tx, &mut rules, transaction);
        clusterizer
    }

    #[test]
    fn clusterizer_merges_segwit_inputs_only_after_activation() {
        let p2pkh = CompactAddress::from_hash160(&Hash160([1; 20]), 0x00);
        let p2wpkh = witness(0, &[3; 20]);

        let mut before = co_spend(SEGWIT_ACTIVATION_HEIGHT - 1, &[p2pkh, p2wpkh]);
        assert!(before.clusters.is_empty());
        assert_eq!(before.clusters.find(&p2wpkh), None);

        let mut after = co_spend(SEGWIT_ACTIVATION_HEIGHT, &[p2pkh, p2wpkh]);
        assert!(after.clusters.find(&p2wpkh).is_some());
        assert_eq!(after.clusters.find(&p2pkh), after.clusters.find(&p2wpkh));
    }

    /// Drives the resumable (SQLite-backed) clusterizer through a funding block
    /// and a spending block whose header time is `spend_time`; returns CSV rows.
    fn resumable_co_spend_rows(spend_time: u32) -> usize {
        use bitiodine::cluster_store::ClusterStore;
        use bitiodine::transactions::TransactionOutput;
        use bitiodine::visitors::ResumableClusterizer;
        use std::sync::atomic::{AtomicUsize, Ordering};

        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let directory = std::env::temp_dir().join(format!(
            "bitiodine-spend-rules-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut p2sh_script = vec![0xa9, 0x14];
        p2sh_script.extend([2; 20]);
        p2sh_script.push(0x87);
        let mut p2pkh_script = vec![0x76, 0xa9, 0x14];
        p2pkh_script.extend([1; 20]);
        p2pkh_script.extend([0x88, 0xac]);
        let funding = Hash::from_data(b"funding transaction");
        let tx = |txid: &[u8]| Transaction {
            version: 1,
            txid: Hash::from_data(txid),
            txins_count: 0,
            txouts_count: 0,
            lock_time: 0,
            slice: &[],
        };

        let mut store = ClusterStore::open(&directory).unwrap();
        {
            let mut visitor = ResumableClusterizer::with_store(&mut store);
            let mut header = [0; 80];
            header[68..72].copy_from_slice(&BIP16_ACTIVATION_TIME.to_le_bytes());
            let mut rules = visitor.visit_block_begin(Block(&header), 0);
            let mut transaction = visitor.visit_transaction_begin(&mut rules);
            for bytes in [&p2sh_script, &p2pkh_script] {
                let output = TransactionOutput {
                    value: 1_000,
                    script: script(bytes),
                    slice: &[],
                };
                visitor.visit_transaction_output(output, &mut rules, &mut transaction);
            }
            let mut funding_tx = tx(b"unused");
            funding_tx.txid = funding;
            visitor.visit_transaction_end(funding_tx, &mut rules, transaction);
            visitor.visit_block_end(Block(&header), 0, rules).unwrap();
            visitor.store_mut().commit_block().unwrap();

            header[68..72].copy_from_slice(&spend_time.to_le_bytes());
            header[0] = 1; // distinct block hash
            let mut rules = visitor.visit_block_begin(Block(&header), 1);
            let mut transaction = visitor.visit_transaction_begin(&mut rules);
            for index in 0..2 {
                let input = TransactionInput {
                    prev_hash: &funding,
                    prev_index: index,
                    script: script(&[]),
                    sequence_no: u32::MAX,
                    slice: &[],
                };
                visitor.visit_transaction_input(input, &mut rules, &mut transaction, None);
            }
            visitor.visit_transaction_end(tx(b"co-spend"), &mut rules, transaction);
            visitor.visit_block_end(Block(&header), 1, rules).unwrap();
            visitor.store_mut().commit_block().unwrap();
        }
        let mut csv = Vec::new();
        store.write_csv(&mut csv).unwrap();
        drop(store);
        std::fs::remove_dir_all(&directory).unwrap();
        String::from_utf8(csv).unwrap().lines().count()
    }

    #[test]
    fn resumable_clusterizer_applies_spend_rules() {
        assert_eq!(resumable_co_spend_rows(BIP16_ACTIVATION_TIME - 1), 0);
        assert_eq!(resumable_co_spend_rows(BIP16_ACTIVATION_TIME), 2);
    }
}
