use bitiodine::hash::Hash;
use bitiodine::visitors::{BlockChainVisitor, Clusterizer};
use bitiodine::BlockChain;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

struct Record {
    frame: Vec<u8>,
    hash: [u8; 32],
    txid: [u8; 32],
}

impl Record {
    fn new(previous: [u8; 32], nonce: u32, inputs: &[([u8; 32], u32)], tags: &[u8]) -> Self {
        let mut tx = 1u32.to_le_bytes().to_vec();
        tx.push(inputs.len().max(1) as u8);
        if inputs.is_empty() {
            tx.extend([0; 32]);
            tx.extend(u32::MAX.to_le_bytes());
            tx.extend([2, 1, nonce as u8]);
            tx.extend(u32::MAX.to_le_bytes());
        } else {
            for (txid, vout) in inputs {
                tx.extend(txid);
                tx.extend(vout.to_le_bytes());
                tx.push(0);
                tx.extend(u32::MAX.to_le_bytes());
            }
        }
        tx.push(tags.len() as u8);
        for tag in tags {
            tx.extend(1_000u64.to_le_bytes());
            tx.extend([25, 0x76, 0xa9, 20]);
            tx.extend([*tag; 20]);
            tx.extend([0x88, 0xac]);
        }
        tx.extend(0u32.to_le_bytes());
        let txid = Sha256::digest(Sha256::digest(&tx)).into();
        let mut payload = 1u32.to_le_bytes().to_vec();
        payload.extend(previous);
        payload.extend(txid);
        payload.extend(1_700_000_000u32.to_le_bytes());
        payload.extend(0x1d00ffffu32.to_le_bytes());
        payload.extend(nonce.to_le_bytes());
        let hash = Sha256::digest(Sha256::digest(&payload)).into();
        payload.push(1);
        payload.extend(tx);
        let mut frame = vec![0xf9, 0xbe, 0xb4, 0xd9];
        frame.extend((payload.len() as u32).to_le_bytes());
        frame.extend(payload);
        Self { frame, hash, txid }
    }

    fn tip(&self) -> String {
        let mut hash = self.hash;
        hash.reverse();
        hex::encode(hash)
    }
}

struct Fixture {
    root: PathBuf,
    genesis: Record,
    common: Record,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "bitiodine-resume-cli-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("blocks")).unwrap();
        let genesis = Record::new([0; 32], 0, &[], &[1, 2, 3, 4]);
        let common = Record::new(
            genesis.hash,
            1,
            &[(genesis.txid, 0), (genesis.txid, 1)],
            &[5, 6],
        );
        let fixture = Self {
            root,
            genesis,
            common,
        };
        fixture.append(&fixture.genesis);
        fixture.append(&fixture.common);
        fixture
    }

    fn blocks(&self) -> PathBuf {
        self.root.join("blocks")
    }

    fn state(&self) -> PathBuf {
        self.root.join("state")
    }

    fn csv(&self) -> PathBuf {
        self.root.join("results.csv")
    }

    fn append(&self, block: &Record) {
        OpenOptions::new()
            .append(true)
            .create(true)
            .open(self.blocks().join("blk00000.dat"))
            .unwrap()
            .write_all(&block.frame)
            .unwrap();
    }

    fn run(&self, tip: &str, state: &Path, output: &Path, extra: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_bitiodine"))
            .arg("--blocks-dir")
            .arg(self.blocks())
            .args(["--chain-tip", tip])
            .arg("--checkpoint-dir")
            .arg(state)
            .arg("--output")
            .arg(output)
            .args(extra)
            .output()
            .unwrap()
    }

    fn assert_reference(&self, tip: &str) {
        let mut reference = Clusterizer::new();
        BlockChain::read(self.blocks(), 0)
            .unwrap()
            .with_tip(Hash::from_pretty(tip).unwrap())
            .walk(&mut reference)
            .unwrap();
        reference.done().unwrap();
        let mut expected = Vec::new();
        reference.write_csv(&mut expected).unwrap();
        assert_eq!(
            sorted_rows(&fs::read(self.csv()).unwrap()),
            sorted_rows(&expected)
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

fn sorted_rows(bytes: &[u8]) -> Vec<String> {
    let mut rows: Vec<_> = String::from_utf8(bytes.to_vec())
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    rows.sort_unstable();
    rows
}

fn success(output: &Output) {
    assert!(output.status.success(), "{output:?}");
}

#[test]
fn reopening_extends_the_tip_and_reorgs_remove_abandoned_merges() {
    let fixture = Fixture::new();
    success(&fixture.run(&fixture.common.tip(), &fixture.state(), &fixture.csv(), &[]));
    fixture.assert_reference(&fixture.common.tip());

    let original = Record::new(
        fixture.common.hash,
        2,
        &[(fixture.genesis.txid, 2), (fixture.genesis.txid, 3)],
        &[7, 8],
    );
    fixture.append(&original);
    let resumed = fixture.run(&original.tip(), &fixture.state(), &fixture.csv(), &[]);
    success(&resumed);
    assert!(String::from_utf8_lossy(&resumed.stderr).contains("Resuming clustering after block 1"));
    fixture.assert_reference(&original.tip());
    let original_csv = fs::read(fixture.csv()).unwrap();

    // The sibling spends a different pair of UTXOs. Replaying its ancestry must
    // remove the old branch's 3/4 merge and restore those outputs as unspent.
    let sibling = Record::new(
        fixture.common.hash,
        3,
        &[(fixture.common.txid, 0), (fixture.common.txid, 1)],
        &[9, 10],
    );
    fixture.append(&sibling);
    success(&fixture.run(&sibling.tip(), &fixture.state(), &fixture.csv(), &[]));
    fixture.assert_reference(&sibling.tip());
    assert_ne!(
        sorted_rows(&original_csv),
        sorted_rows(&fs::read(fixture.csv()).unwrap())
    );

    // A shorter selected chain also rolls back merges. Returning to the old
    // branch must work after its prior rows have been undone.
    for tip in [fixture.common.tip(), original.tip(), original.tip()] {
        success(&fixture.run(&tip, &fixture.state(), &fixture.csv(), &[]));
        fixture.assert_reference(&tip);
    }

    // The persistent visitor must apply the same equal-output filter as the
    // in-memory reference, even though it consumes inputs through its own store.
    let coinjoin = Record::new(
        original.hash,
        4,
        &[(original.txid, 0), (original.txid, 1)],
        &[11, 12, 13],
    );
    fixture.append(&coinjoin);
    success(&fixture.run(&coinjoin.tip(), &fixture.state(), &fixture.csv(), &[]));
    fixture.assert_reference(&coinjoin.tip());
    assert_eq!(
        sorted_rows(&original_csv),
        sorted_rows(&fs::read(fixture.csv()).unwrap())
    );
}

#[test]
fn default_state_directory_reopens_next_to_the_output() {
    let fixture = Fixture::new();
    let state = fixture.root.join("results.csv.state");
    for resumed in [false, true] {
        let output = Command::new(env!("CARGO_BIN_EXE_bitiodine"))
            .arg("--blocks-dir")
            .arg(fixture.blocks())
            .args(["--chain-tip", &fixture.common.tip()])
            .arg("--output")
            .arg(fixture.csv())
            .output()
            .unwrap();
        success(&output);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Clustering memory budget:"), "{stderr}");
        if resumed {
            assert!(
                stderr.contains("Resuming clustering after block 1"),
                "{stderr}"
            );
        }
        assert!(state.join("clusters.sqlite3").is_file());
        fixture.assert_reference(&fixture.common.tip());
    }
    assert!(!fixture.blocks().join("results.csv.state").exists());
}

#[test]
fn missing_nested_state_parents_are_created_outside_source_storage() {
    let fixture = Fixture::new();
    let state = fixture.root.join("missing/nested/state");
    success(&fixture.run(&fixture.common.tip(), &state, &fixture.csv(), &[]));
    assert!(state.join("clusters.sqlite3").is_file());
    fixture.assert_reference(&fixture.common.tip());
}

#[test]
fn failed_parsing_preserves_committed_state_and_previous_csv() {
    let fixture = Fixture::new();
    success(&fixture.run(&fixture.common.tip(), &fixture.state(), &fixture.csv(), &[]));
    let before = fs::read(fixture.csv()).unwrap();
    let mut bad = Record::new(
        fixture.common.hash,
        2,
        &[(fixture.genesis.txid, 2), (fixture.genesis.txid, 3)],
        &[7, 8],
    );
    // Keep the block header/record framing intact, but truncate transaction locktime.
    bad.frame.pop();
    let payload_length = (bad.frame.len() - 8) as u32;
    bad.frame[4..8].copy_from_slice(&payload_length.to_le_bytes());
    fixture.append(&bad);
    let failed = fixture.run(&bad.tip(), &fixture.state(), &fixture.csv(), &[]);
    assert!(!failed.status.success(), "{failed:?}");
    assert_eq!(fs::read(fixture.csv()).unwrap(), before);
    success(&fixture.run(&fixture.common.tip(), &fixture.state(), &fixture.csv(), &[]));
    fixture.assert_reference(&fixture.common.tip());
}

#[test]
fn checkpoint_options_are_rejected_for_other_actions() {
    let fixture = Fixture::new();
    for action in ["merkle", "dump-balances", "dump-addresses"] {
        let failed = fixture.run(
            &fixture.common.tip(),
            &fixture.state(),
            &fixture.csv(),
            &["--action", action],
        );
        assert!(!failed.status.success(), "{action}: {failed:?}");
        assert!(!fixture.state().exists());
    }
}

#[test]
fn help_explains_automatic_resume_reorg_recovery_and_memory_limit() {
    let output = Command::new(env!("CARGO_BIN_EXE_bitiodine"))
        .arg("--help")
        .output()
        .unwrap();
    success(&output);
    let help = String::from_utf8(output.stdout)
        .unwrap()
        .to_ascii_lowercase();
    assert!(help.contains("--checkpoint-dir"), "{help}");
    assert!(help.contains("resum"), "{help}");
    assert!(help.contains("reorg"), "{help}");
    assert!(help.contains("32") && help.contains("memory"), "{help}");
}

#[test]
fn state_directory_must_be_disjoint_from_block_storage() {
    let fixture = Fixture::new();
    let source = fixture.blocks().join("blk00000.dat");
    let before = fs::read(&source).unwrap();
    for state in [
        fixture.blocks(),
        fixture.blocks().join("state"),
        fixture.root.clone(),
    ] {
        let failed = fixture.run(&fixture.common.tip(), &state, &fixture.csv(), &[]);
        assert!(!failed.status.success(), "{state:?}: {failed:?}");
        assert_eq!(fs::read(&source).unwrap(), before);
    }
    assert!(!fixture.blocks().join("state").exists());
    assert!(!fixture.blocks().join("clusters.sqlite3").exists());
}

#[test]
fn unusable_state_preserves_previous_csv() {
    let fixture = Fixture::new();
    let previous = b"previous complete results\n";
    fs::write(fixture.csv(), previous).unwrap();
    fs::create_dir(fixture.state()).unwrap();
    let database = fixture.state().join("clusters.sqlite3");
    let invalid_database = b"not a SQLite database\n";
    fs::write(&database, invalid_database).unwrap();
    let failed = fixture.run(&fixture.common.tip(), &fixture.state(), &fixture.csv(), &[]);
    assert!(!failed.status.success(), "{failed:?}");
    assert_eq!(fs::read(fixture.csv()).unwrap(), previous);
    assert_eq!(fs::read(database).unwrap(), invalid_database);
}

#[test]
fn concurrent_writer_lock_preserves_previous_csv() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.state()).unwrap();
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(fixture.state().join("cluster.lock"))
        .unwrap();
    fs2::FileExt::lock_exclusive(&lock).unwrap();
    let previous = b"previous complete results\n";
    fs::write(fixture.csv(), previous).unwrap();
    let failed = fixture.run(&fixture.common.tip(), &fixture.state(), &fixture.csv(), &[]);
    assert!(!failed.status.success(), "{failed:?}");
    assert_eq!(fs::read(fixture.csv()).unwrap(), previous);
}

#[cfg(unix)]
#[test]
fn sigkill_discards_uncommitted_wal_and_resumes_the_last_complete_block() {
    use bitiodine::address::CompactAddress;
    use bitiodine::cluster_store::ClusterStore;
    use bitiodine::hash160::Hash160;
    use std::io::Read;
    use std::process::Stdio;
    use std::time::{Duration, Instant};

    const CHILD_DIRECTORY: &str = "BITIODINE_TEST_CRASH_DIRECTORY";
    if let Some(directory) = std::env::var_os(CHILD_DIRECTORY) {
        let directory = PathBuf::from(directory);
        let state = directory.join("state");
        let genesis = Record::new([0; 32], 0, &[], &[1, 2, 3, 4]);
        let mut store = ClusterStore::open_with_memory_limit(&state, 2 << 20).unwrap();
        store
            .begin_block(2, Hash::from_data(b"interrupted block"))
            .unwrap();
        for index in [2, 3] {
            assert!(store.spend(&Hash(genesis.txid), index).unwrap().is_some());
        }
        let addresses = [3, 4].map(|tag| CompactAddress::from_hash160(&Hash160([tag; 20]), 0));
        store.merge(&addresses).unwrap();
        // Exceed this child's small page cache so SQLite spills uncommitted
        // frames to WAL. This checks recovery of a dirty WAL, not just lost RAM.
        let outputs: Vec<_> = (0..20_000).map(|index| (index, addresses[0])).collect();
        store
            .replace_outputs(&Hash::from_data(b"interrupted transaction"), &outputs)
            .unwrap();
        assert!(
            fs::metadata(state.join("clusters.sqlite3-wal"))
                .unwrap()
                .len()
                > 32
        );
        fs::write(directory.join("writer-ready"), b"uncommitted WAL flushed").unwrap();
        loop {
            std::thread::park();
        }
    }

    let fixture = Fixture::new();
    success(&fixture.run(&fixture.common.tip(), &fixture.state(), &fixture.csv(), &[]));
    let before = fs::read(fixture.csv()).unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "sigkill_discards_uncommitted_wal_and_resumes_the_last_complete_block",
            "--nocapture",
        ])
        .env(CHILD_DIRECTORY, &fixture.root)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    while !fixture.root.join("writer-ready").exists() {
        if let Some(status) = child.try_wait().unwrap() {
            let mut stderr = String::new();
            child
                .stderr
                .take()
                .unwrap()
                .read_to_string(&mut stderr)
                .unwrap();
            panic!("crash-test child exited before WAL spill: {status}: {stderr}");
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("crash-test child did not reach its uncommitted WAL checkpoint");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    child.kill().unwrap(); // SIGKILL on Unix: no Rust destructors or rollback callback.
    assert!(!child.wait().unwrap().success());
    assert_eq!(fs::read(fixture.csv()).unwrap(), before);
    success(&fixture.run(&fixture.common.tip(), &fixture.state(), &fixture.csv(), &[]));
    fixture.assert_reference(&fixture.common.tip());

    // Inputs removed by the killed transaction must be available for a new block.
    let next = Record::new(
        fixture.common.hash,
        2,
        &[(fixture.genesis.txid, 2), (fixture.genesis.txid, 3)],
        &[7, 8],
    );
    fixture.append(&next);
    success(&fixture.run(&next.tip(), &fixture.state(), &fixture.csv(), &[]));
    fixture.assert_reference(&next.tip());
}

#[cfg(unix)]
#[test]
fn checkpoint_and_output_aliases_cannot_overwrite_protected_files() {
    let fixture = Fixture::new();
    success(&fixture.run(&fixture.common.tip(), &fixture.state(), &fixture.csv(), &[]));
    let database = fixture.state().join("clusters.sqlite3");
    let before = fs::read(&database).unwrap();
    let symlink = fixture.root.join("symlink.csv");
    std::os::unix::fs::symlink(&database, &symlink).unwrap();
    let hardlink = fixture.root.join("hardlink.csv");
    fs::hard_link(&database, &hardlink).unwrap();
    for output in [&database, &symlink, &hardlink] {
        let failed = fixture.run(&fixture.common.tip(), &fixture.state(), output, &[]);
        assert!(!failed.status.success(), "{output:?}: {failed:?}");
        assert_eq!(fs::read(&database).unwrap(), before);
    }
    let alias = fixture.root.join("blocks-alias");
    std::os::unix::fs::symlink(fixture.blocks(), &alias).unwrap();
    let failed = fixture.run(&fixture.common.tip(), &alias, &fixture.csv(), &[]);
    assert!(!failed.status.success(), "{failed:?}");
    assert!(!fixture.blocks().join("cluster.lock").exists());
}

#[cfg(unix)]
#[test]
fn managed_state_file_aliases_cannot_modify_block_sources() {
    for name in [
        "cluster.lock",
        "clusters.sqlite3",
        "clusters.sqlite3-wal",
        "clusters.sqlite3-shm",
        "clusters.sqlite3-journal",
    ] {
        for symlink in [false, true] {
            let fixture = Fixture::new();
            fs::create_dir(fixture.state()).unwrap();
            let source = fixture.blocks().join("blk00000.dat");
            let before = fs::read(&source).unwrap();
            let alias = fixture.state().join(name);
            if symlink {
                std::os::unix::fs::symlink(&source, alias).unwrap();
            } else {
                fs::hard_link(&source, alias).unwrap();
            }
            let failed = fixture.run(&fixture.common.tip(), &fixture.state(), &fixture.csv(), &[]);
            assert!(!failed.status.success(), "{name}/{symlink}: {failed:?}");
            assert_eq!(fs::read(source).unwrap(), before);
            assert!(!fixture.csv().exists());
        }
    }
}
