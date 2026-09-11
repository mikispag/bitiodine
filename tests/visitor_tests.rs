use std::io::{self, Write};

use bitiodine::address::{Address, CompactAddress};
use bitiodine::block::Block;
use bitiodine::blockchain::OutputMap;
use bitiodine::error::ParseError;
use bitiodine::hash::Hash;
use bitiodine::hash160::Hash160;
use bitiodine::script::Script;
use bitiodine::transactions::{Transaction, TransactionInput, TransactionOutput};
use bitiodine::visitors::{
    BlockChainVisitor, Clusterizer, DataOutputFinder, DonationFinder, DumpAddresses, DumpBalances,
    DumpTxHashes,
};

fn pubkey() -> Vec<u8> {
    hex::decode("0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798").unwrap()
}

fn output(script: &[u8], value: u64) -> TransactionOutput<'_> {
    TransactionOutput {
        value,
        script: Script::new(script, 1_700_000_000, 800_000),
        slice: &[],
    }
}

fn coinbase(outputs: &[(u64, &[u8])]) -> Vec<u8> {
    let mut bytes = 1u32.to_le_bytes().to_vec();
    bytes.push(1);
    bytes.extend([0; 32]);
    bytes.extend(u32::MAX.to_le_bytes());
    bytes.extend([2, 1, 1]);
    bytes.extend(u32::MAX.to_le_bytes());
    bytes.push(outputs.len() as u8);
    for (value, script) in outputs {
        bytes.extend(value.to_le_bytes());
        bytes.push(script.len() as u8);
        bytes.extend(*script);
    }
    bytes.extend(0u32.to_le_bytes());
    bytes
}

fn walk_transaction<V: BlockChainVisitor<BlockItem = ()>>(
    visitor: &mut V,
    bytes: &[u8],
) -> OutputMap<V::OutputItem> {
    let mut outputs = OutputMap::default();
    let mut slice = bytes;
    Transaction::read_and_walk(
        &mut slice,
        visitor,
        1_700_000_000,
        800_000,
        &mut (),
        &mut outputs,
    )
    .unwrap();
    assert!(slice.is_empty());
    outputs
}

#[test]
fn p2pk_inputs_participate_in_clustering() {
    let pk = pubkey();
    let mut p2pk = vec![33];
    p2pk.extend(&pk);
    p2pk.push(0xac);
    let mut p2pkh = vec![0x76, 0xa9, 0x14];
    p2pkh.extend([7; 20]);
    p2pkh.extend([0x88, 0xac]);

    let mut visitor = Clusterizer::new();
    let mut transaction = visitor.visit_transaction_begin(&mut ());
    let a = visitor
        .visit_transaction_output(output(&p2pk, 50), &mut (), &mut transaction)
        .expect("P2PK must retain its public-key identifier for co-spend analysis");
    let b = visitor
        .visit_transaction_output(output(&p2pkh, 50), &mut (), &mut transaction)
        .unwrap();
    assert_eq!(a, CompactAddress::from_hash160(&Hash160::from_data(&pk), 0));

    let prev_hash = Hash::from_data(b"previous transaction");
    let input = TransactionInput {
        prev_hash: &prev_hash,
        prev_index: 0,
        script: Script::new(&[], 1_700_000_000, 800_000),
        sequence_no: u32::MAX,
        slice: &[],
    };
    let mut transaction = visitor.visit_transaction_begin(&mut ());
    visitor.visit_transaction_input(input, &mut (), &mut transaction, Some(a));
    visitor.visit_transaction_input(input, &mut (), &mut transaction, Some(b));
    visitor.visit_transaction_end(
        Transaction {
            version: 1,
            txid: Hash::from_data(b"co-spend"),
            txins_count: 2,
            txouts_count: 0,
            lock_time: 0,
            slice: &[],
        },
        &mut (),
        transaction,
    );
    assert!(visitor.clusters.find(&a).is_some());
    assert_eq!(visitor.clusters.find(&a), visitor.clusters.find(&b));
}

#[test]
fn genesis_outputs_do_not_enter_the_balance_utxo_set() {
    let header = hex::decode(concat!(
        "01000000",
        "0000000000000000000000000000000000000000000000000000000000000000",
        "3ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4a",
        "29ab5f49ffff001d1dac2b7c"
    ))
    .unwrap();
    assert_eq!(
        Block(&header).header().cur_hash().to_string(),
        "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f"
    );
    let mut visitor = DumpBalances::new();
    let mut block = visitor.visit_block_begin(Block(&header), 0);
    let mut script = vec![0x76, 0xa9, 0x14];
    script.extend([7; 20]);
    script.extend([0x88, 0xac]);
    assert!(visitor
        .visit_transaction_output(output(&script, 5_000_000_000), &mut block, &mut ())
        .is_none());
    assert!(visitor.balances.is_empty());

    let mut header = [0; 80];
    header[4] = 1;
    let mut block = visitor.visit_block_begin(Block(&header), 1);
    assert!(visitor
        .visit_transaction_output(output(&script, 5_000_000_000), &mut block, &mut ())
        .is_some());
    assert_eq!(visitor.balances.values().sum::<i64>(), 5_000_000_000);
}

#[test]
fn finders_do_not_retain_outputs_for_spend_tracking() {
    let mut data = DataOutputFinder::new();
    let mut data_transaction = data.visit_transaction_begin(&mut ());
    assert!(data
        .visit_transaction_output(output(b"\x6a\x03abc", 0), &mut (), &mut data_transaction)
        .is_none());
    let mut donation = DonationFinder::new();
    let mut donation_transaction = donation.visit_transaction_begin(&mut ());
    assert!(donation
        .visit_transaction_output(output(&[], 7), &mut (), &mut donation_transaction)
        .is_none());
}

#[test]
fn finders_emit_outpoints_and_exact_payloads_without_retaining_utxos() {
    let bytes = coinbase(&[(0, b"\x6a\x03\xff,\n"), (7, &[]), (0, &[])]);
    let txid = Hash::from_data(&bytes);
    let mut data_rows = Vec::new();
    let mut donation_rows = Vec::new();
    {
        let mut data = DataOutputFinder::with_writer(&mut data_rows);
        let mut donations = DonationFinder::with_writer(&mut donation_rows);
        assert!(walk_transaction(&mut data, &bytes).is_empty());
        assert!(walk_transaction(&mut donations, &bytes).is_empty());
        data.done().unwrap();
        donations.done().unwrap();
    }
    assert_eq!(
        String::from_utf8(data_rows).unwrap(),
        format!("{txid},0,ff2c0a\n")
    );
    assert_eq!(
        String::from_utf8(donation_rows).unwrap(),
        format!("{txid},1,7,\n")
    );
}

#[test]
fn p2pk_and_multisig_export_public_key_identifiers() {
    let pk = pubkey();
    let mut p2pk = vec![33];
    p2pk.extend(&pk);
    p2pk.push(0xac);
    let mut multisig = vec![0x51, 33];
    multisig.extend(&pk);
    multisig.extend([0x51, 0xae]);
    let bytes = coinbase(&[(11, &p2pk), (13, &multisig)]);
    let mut rows = Vec::new();
    {
        let mut visitor = DumpAddresses::with_writer(&mut rows);
        assert!(walk_transaction(&mut visitor, &bytes).is_empty());
        visitor.done().unwrap();
    }
    let identifier = Address::from_pubkey(&pk, 0x00);
    assert_eq!(
        String::from_utf8(rows).unwrap(),
        format!("{identifier}\n{identifier}\n")
    );
}

#[test]
fn p2sh_outputs_reach_balance_and_cluster_visitors() {
    let mut script = vec![0xa9, 0x14];
    script.extend([3; 20]);
    script.push(0x87);
    let expected = CompactAddress::from_hash160(&Hash160([3; 20]), 0x05);
    let mut balances = DumpBalances::new();
    let mut header = [0; 80];
    header[4] = 1;
    let mut block = balances.visit_block_begin(Block(&header), 800_000);
    let balance_output = balances
        .visit_transaction_output(output(&script, 17), &mut block, &mut ())
        .unwrap();
    assert_eq!(balance_output, (expected, 17));
    assert_eq!(balances.balances.get(&expected), Some(&17));

    let mut clusters = Clusterizer::new();
    let mut transaction = clusters.visit_transaction_begin(&mut ());
    assert_eq!(
        clusters.visit_transaction_output(output(&script, 17), &mut (), &mut transaction),
        Some(expected)
    );
}

#[test]
fn duplicate_coinbase_balances_match_surviving_outputs_and_can_be_spent() {
    let p2pkh = |byte| {
        let mut script = vec![0x76, 0xa9, 0x14];
        script.extend([byte; 20]);
        script.extend([0x88, 0xac]);
        script
    };
    let first_script = p2pkh(1);
    let second_script = p2pkh(2);
    let bytes = coinbase(&[
        (3_000_000_000, &first_script),
        (2_000_000_000, &second_script),
    ]);
    let mut visitor = DumpBalances::new();
    let mut outputs = OutputMap::default();

    for height in [91_812, 91_842] {
        Transaction::read_and_walk(
            &mut bytes.as_slice(),
            &mut visitor,
            1_288_000_000,
            height,
            &mut false,
            &mut outputs,
        )
        .unwrap();
        let utxo_sum: i64 = outputs
            .values()
            .flat_map(|outputs| outputs.iter().map(|(_, (_, value))| value))
            .sum();
        assert_eq!(outputs.len(), 1);
        assert_eq!(utxo_sum, 5_000_000_000);
        assert_eq!(visitor.balances.values().sum::<i64>(), utxo_sum);
    }

    let mut spend = 1u32.to_le_bytes().to_vec();
    spend.push(2);
    for index in 0u32..2 {
        spend.extend(Hash::from_data(&bytes).as_slice());
        spend.extend(index.to_le_bytes());
        spend.push(0);
        spend.extend(u32::MAX.to_le_bytes());
    }
    spend.push(1);
    spend.extend(0u64.to_le_bytes());
    spend.extend([2, 0x6a, 0]);
    spend.extend(0u32.to_le_bytes());
    Transaction::read_and_walk(
        &mut spend.as_slice(),
        &mut visitor,
        1_288_000_000,
        92_000,
        &mut false,
        &mut outputs,
    )
    .unwrap();
    assert!(outputs.is_empty());
    assert!(visitor.balances.is_empty());
}

struct BrokenWriter;

impl Write for BrokenWriter {
    fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "test sink closed",
        ))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn assert_block_write_failure<V: BlockChainVisitor>(mut visitor: V, bytes: &[u8]) {
    let mut block = vec![0; 80];
    block[4] = 1;
    block[68..72].copy_from_slice(&1_700_000_000u32.to_le_bytes());
    block.push(1);
    block.extend(bytes);
    let error = Block(&block)
        .walk(&mut visitor, 800_000, &mut OutputMap::default())
        .unwrap_err();
    assert!(
        matches!(error, ParseError::Io(ref error) if error.kind() == io::ErrorKind::BrokenPipe)
    );
}

#[test]
fn output_write_failures_stop_traversal() {
    let mut p2pk = vec![33];
    p2pk.extend(pubkey());
    p2pk.push(0xac);
    let bytes = coinbase(&[(0, b"\x6a\x03abc"), (7, &[]), (11, &p2pk)]);
    assert_block_write_failure(DataOutputFinder::with_writer(BrokenWriter), &bytes);
    assert_block_write_failure(DonationFinder::with_writer(BrokenWriter), &bytes);
    assert_block_write_failure(DumpAddresses::with_writer(BrokenWriter), &bytes);
    assert_block_write_failure(DumpTxHashes::with_writer(BrokenWriter), &bytes);
}

struct FlushFailure;

impl Write for FlushFailure {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "test flush failed",
        ))
    }
}

#[test]
fn output_finalization_reports_flush_failures() {
    assert!(DataOutputFinder::with_writer(FlushFailure).done().is_err());
    assert!(DonationFinder::with_writer(FlushFailure).done().is_err());
    assert!(DumpAddresses::with_writer(FlushFailure).done().is_err());
    assert!(DumpTxHashes::with_writer(FlushFailure).done().is_err());
}

#[test]
fn transaction_hash_dumper_returns_error_for_missing_transaction_count() {
    let mut visitor = DumpTxHashes::with_writer(io::sink());
    assert!(Block(&[0; 80])
        .walk(&mut visitor, 800_000, &mut OutputMap::default())
        .is_err());
}
