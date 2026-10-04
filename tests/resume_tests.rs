use bitiodine::cluster_store::ClusterStore;
use bitiodine::visitors::clusterizer::DisjointSet;
use bitiodine::visitors::{Clusterizer, ResumableClusterizer};
use bitiodine::{BlockChain, BlockChainVisitor, Hash, MerkleHasher, ZERO_HASH};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Fixture(PathBuf);

impl Fixture {
    fn new(records: &[Vec<u8>]) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let directory = std::env::temp_dir().join(format!(
            "bitiodine-resume-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&directory).unwrap();
        std::fs::create_dir(directory.join("blocks")).unwrap();
        let fixture = Self(directory);
        fixture.write(records);
        fixture
    }

    fn write(&self, records: &[Vec<u8>]) {
        std::fs::write(self.0.join("blocks/blk00000.dat"), records.concat()).unwrap();
    }

    fn chain(&self, tip: Hash) -> BlockChain {
        BlockChain::read(self.0.join("blocks"), 0)
            .unwrap()
            .with_tip(tip)
    }

    fn store(&self) -> ClusterStore {
        ClusterStore::open(&self.0.join("state")).unwrap()
    }

    fn resume(&self, tip: Hash) -> u64 {
        let mut store = self.store();
        let (count, selected) = self
            .chain(tip)
            .walk_clusterizer_resumable(&mut ResumableClusterizer::with_store(&mut store))
            .unwrap();
        assert_eq!(selected, tip);
        assert_eq!(store.tip().unwrap(), Some((count - 1, tip)));
        count
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn transaction(inputs: &[(Hash, u32)], outputs: &[u8], tag: u32) -> (Hash, Vec<u8>) {
    let mut bytes = 1u32.to_le_bytes().to_vec();
    if inputs.is_empty() {
        bytes.push(1);
        bytes.extend(ZERO_HASH.0);
        bytes.extend(u32::MAX.to_le_bytes());
        bytes.extend([2, 1, tag as u8]);
        bytes.extend(u32::MAX.to_le_bytes());
    } else {
        bytes.push(inputs.len() as u8);
        for (hash, index) in inputs {
            bytes.extend(hash.0);
            bytes.extend(index.to_le_bytes());
            bytes.push(0);
            bytes.extend(u32::MAX.to_le_bytes());
        }
    }
    bytes.push(outputs.len() as u8);
    for (index, &address) in outputs.iter().enumerate() {
        bytes.extend(((index as u64 + 1) * 1000).to_le_bytes());
        bytes.extend([25, 0x76, 0xa9, 20]);
        bytes.extend([address; 20]);
        bytes.extend([0x88, 0xac]);
    }
    bytes.extend(tag.to_le_bytes());
    (Hash::from_data(&bytes), bytes)
}

fn record(previous: Hash, nonce: u32, transactions: &[(Hash, Vec<u8>)]) -> (Hash, Vec<u8>) {
    let mut merkle = MerkleHasher::default();
    for (txid, _) in transactions {
        merkle.add(*txid);
    }
    let mut body = 1u32.to_le_bytes().to_vec();
    body.extend(previous.0);
    body.extend(merkle.finish().unwrap().0);
    body.extend(1_700_000_000u32.to_le_bytes());
    body.extend(0x1d00ffffu32.to_le_bytes());
    body.extend(nonce.to_le_bytes());
    let hash = Hash::from_data(&body);
    body.push(transactions.len() as u8);
    for (_, transaction) in transactions {
        body.extend(transaction);
    }
    let mut frame = 0xd9b4bef9u32.to_le_bytes().to_vec();
    frame.extend((body.len() as u32).to_le_bytes());
    frame.extend(body);
    (hash, frame)
}

struct Fork {
    records: Vec<Vec<u8>>,
    common: Hash,
    old_first: Hash,
    old_tip: Hash,
    new_tip: Hash,
}

impl Fork {
    fn new() -> Self {
        let genesis_tx = transaction(&[], &[1, 2, 3, 4], 0);
        let (g, genesis) = record(ZERO_HASH, 0, std::slice::from_ref(&genesis_tx));
        let common_tx = transaction(&[(genesis_tx.0, 0), (genesis_tx.0, 1)], &[5, 6], 1);
        let (common, common_block) = record(g, 1, std::slice::from_ref(&common_tx));
        let old_tx = transaction(&[(genesis_tx.0, 2), (genesis_tx.0, 3)], &[7, 8], 2);
        let (old_first, old_first_block) = record(common, 2, std::slice::from_ref(&old_tx));
        let old_last_tx = transaction(&[(old_tx.0, 0), (old_tx.0, 1)], &[9], 3);
        let (old_tip, old_last_block) = record(old_first, 3, &[old_last_tx]);
        let new_tx = transaction(&[(common_tx.0, 0), (common_tx.0, 1)], &[10, 11], 4);
        let (new_first, new_first_block) = record(common, 4, std::slice::from_ref(&new_tx));
        let new_last_tx = transaction(&[(new_tx.0, 0), (new_tx.0, 1)], &[12], 5);
        let (new_tip, new_last_block) = record(new_first, 5, &[new_last_tx]);
        Self {
            records: vec![
                genesis,
                common_block,
                old_first_block,
                old_last_block,
                new_first_block,
                new_last_block,
            ],
            common,
            old_first,
            old_tip,
            new_tip,
        }
    }
}

fn sorted_csv(bytes: Vec<u8>) -> Vec<String> {
    let text = String::from_utf8(bytes).unwrap();
    let mut rows: Vec<_> = text.lines().map(str::to_owned).collect();
    rows.sort_unstable();
    rows
}

fn assert_matches_uninterrupted(fixture: &Fixture, tip: Hash) {
    let mut reference = Clusterizer {
        clusters: DisjointSet::with_capacity(0),
    };
    let (count, selected, outputs) = fixture.chain(tip).walk(&mut reference).unwrap();
    reference.done().unwrap();
    let mut expected = Vec::new();
    reference.write_csv(&mut expected).unwrap();

    let mut store = fixture.store();
    assert_eq!(store.tip().unwrap(), Some((count - 1, selected)));
    let mut actual = Vec::new();
    store.write_csv(&mut actual).unwrap();
    assert_eq!(sorted_csv(actual), sorted_csv(expected));

    let expected_utxos = outputs
        .values()
        .map(|items| items.len() as u64)
        .sum::<u64>();
    assert_eq!(store.utxo_count().unwrap(), expected_utxos);
    // Inspect every persisted UTXO without adding a production iterator or map.
    store.begin_block(count, Hash([0xfa; 32])).unwrap();
    for (txid, items) in outputs {
        for (index, address) in items {
            assert_eq!(store.spend(&txid, index).unwrap(), Some(address));
        }
    }
    assert_eq!(store.utxo_count().unwrap(), 0);
    store.abort_block().unwrap();
    assert_eq!(store.utxo_count().unwrap(), expected_utxos);
    assert_eq!(store.tip().unwrap(), Some((count - 1, selected)));
}

#[test]
fn reopening_and_extending_chain_matches_uninterrupted_state() {
    let fork = Fork::new();
    let fixture = Fixture::new(&fork.records[..2]);
    assert_eq!(fixture.resume(fork.common), 2);
    assert_matches_uninterrupted(&fixture, fork.common);
    fixture.write(&fork.records);
    assert_eq!(fixture.resume(fork.old_tip), 4);
    assert_matches_uninterrupted(&fixture, fork.old_tip);
}

#[test]
fn reopening_at_completed_tip_preserves_clusters_and_utxos() {
    let fork = Fork::new();
    let fixture = Fixture::new(&fork.records);
    assert_eq!(fixture.resume(fork.old_tip), 4);
    assert_eq!(fixture.resume(fork.old_tip), 4);
    assert_matches_uninterrupted(&fixture, fork.old_tip);
}

#[test]
fn shallow_reorg_restores_spends_and_removes_stale_merges() {
    let fork = Fork::new();
    let fixture = Fixture::new(&fork.records);
    for tip in [fork.old_tip, fork.new_tip, fork.common, fork.old_first] {
        fixture.resume(tip);
        assert_matches_uninterrupted(&fixture, tip);
    }
}

#[test]
fn deep_reorg_replays_from_genesis_without_stale_state() {
    let mut fork = Fork::new();
    let mut old_tip = fork.old_tip;
    for tag in 1000..1145 {
        let coinbase = transaction(&[], &[20], tag);
        let (tip, frame) = record(old_tip, tag, &[coinbase]);
        old_tip = tip;
        fork.records.push(frame);
    }
    let fixture = Fixture::new(&fork.records);
    assert_eq!(fixture.resume(old_tip), 149);
    assert_eq!(fixture.resume(fork.new_tip), 4);
    assert_matches_uninterrupted(&fixture, fork.new_tip);
    assert_eq!(fixture.store().hash_at(148).unwrap(), None);
}

#[test]
fn failed_block_rolls_back_partial_spends_outputs_and_merges() {
    let fork = Fork::new();
    let fixture = Fixture::new(&fork.records);
    fixture.resume(fork.common);
    let mut broken = fork.records.clone();
    broken[2][8 + 80] = 2; // First transaction changes clusters; missing second fails parsing.
    fixture.write(&broken);
    {
        let mut store = fixture.store();
        let mut visitor = ResumableClusterizer::with_store(&mut store);
        assert!(fixture
            .chain(fork.old_first)
            .walk_clusterizer_resumable(&mut visitor)
            .is_err());
        assert_eq!(visitor.store().tip().unwrap(), Some((1, fork.common)));
    }
    assert_matches_uninterrupted(&fixture, fork.common);
    fixture.write(&fork.records);
    fixture.resume(fork.old_first);
    assert_matches_uninterrupted(&fixture, fork.old_first);
}

#[test]
fn source_change_before_resume_preserves_committed_state() {
    let fork = Fork::new();
    let fixture = Fixture::new(&fork.records);
    fixture.resume(fork.common);
    let chain = fixture.chain(fork.old_tip);
    let mut changed = fork.records.clone();
    changed.last_mut().unwrap().push(0);
    fixture.write(&changed);
    {
        let mut store = fixture.store();
        let error = chain
            .walk_clusterizer_resumable(&mut ResumableClusterizer::with_store(&mut store))
            .unwrap_err();
        assert!(error.to_string().contains("changed during analysis"));
        assert_eq!(store.tip().unwrap(), Some((1, fork.common)));
    }
    fixture.write(&fork.records);
    assert_matches_uninterrupted(&fixture, fork.common);
}

#[test]
fn traversal_budget_failure_does_not_rewind_or_update_persistent_state() {
    let fork = Fork::new();
    let fixture = Fixture::new(&fork.records);
    fixture.resume(fork.old_tip);
    {
        let mut store = fixture.store();
        let error = fixture
            .chain(fork.new_tip)
            .with_memory_limit(1)
            .walk_clusterizer_resumable(&mut ResumableClusterizer::with_store(&mut store))
            .unwrap_err();
        assert!(error.to_string().contains("memory budget exceeded"));
        assert_eq!(store.tip().unwrap(), Some((3, fork.old_tip)));
    }
    assert_matches_uninterrupted(&fixture, fork.old_tip);
    let chain = BlockChain::read_with_memory_limit(fixture.0.join("blocks"), 0, 16 * 1024 * 1024)
        .unwrap()
        .with_tip(fork.new_tip);
    {
        let mut store = fixture.store();
        assert_eq!(
            chain
                .walk_clusterizer_resumable(&mut ResumableClusterizer::with_store(&mut store))
                .unwrap(),
            (4, fork.new_tip)
        );
    }
    assert_matches_uninterrupted(&fixture, fork.new_tip);
}

#[test]
fn discovery_and_header_growth_respect_traversal_budget() {
    let fork = Fork::new();
    let fixture = Fixture::new(&fork.records);
    assert!(BlockChain::read_with_memory_limit(fixture.0.join("blocks"), 0, 1).is_err());
    // Allows discovery and one indexed header, but cannot grow the full index.
    let limit = 4_008_192 + 4096 + 1024;
    let chain = BlockChain::read_with_memory_limit(fixture.0.join("blocks"), 0, limit)
        .unwrap()
        .with_tip(fork.common);
    let mut visitor = Clusterizer {
        clusters: DisjointSet::with_capacity(0),
    };
    let error = chain.walk(&mut visitor).unwrap_err();
    assert!(error.to_string().contains("memory budget exceeded"));
    assert!(visitor.clusters.is_empty());
}
