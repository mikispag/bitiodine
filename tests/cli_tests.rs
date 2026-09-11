use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "bitiodine-cli-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn valid_chain(&self) -> String {
        let mut previous = [0; 32];
        let mut previous_txid = [0; 32];
        let mut records = Vec::new();
        for tag in 0..2 {
            let mut tx = 1u32.to_le_bytes().to_vec();
            if tag == 0 {
                tx.push(1);
                tx.extend([0; 32]);
                tx.extend(u32::MAX.to_le_bytes());
                tx.extend([2, 1, tag]);
                tx.extend(u32::MAX.to_le_bytes());
            } else {
                tx.push(2);
                for index in 0u32..2 {
                    tx.extend(previous_txid);
                    tx.extend(index.to_le_bytes());
                    tx.push(0);
                    tx.extend(u32::MAX.to_le_bytes());
                }
            }
            tx.push(2);
            for address in 1..=2 {
                tx.extend(2_500_000_000u64.to_le_bytes());
                tx.extend([25, 0x76, 0xa9, 20]);
                tx.extend([address; 20]);
                tx.extend([0x88, 0xac]);
            }
            tx.extend(0u32.to_le_bytes());
            previous_txid = Sha256::digest(Sha256::digest(&tx)).into();

            let mut header = 1u32.to_le_bytes().to_vec();
            header.extend(previous);
            header.extend(previous_txid);
            header.extend(1_700_000_000u32.to_le_bytes());
            header.extend(0x1d00ffffu32.to_le_bytes());
            header.extend(u32::from(tag).to_le_bytes());
            previous = Sha256::digest(Sha256::digest(&header)).into();
            let mut payload = header;
            payload.push(1);
            payload.extend(tx);
            records.extend([0xf9, 0xbe, 0xb4, 0xd9]);
            records.extend((payload.len() as u32).to_le_bytes());
            records.extend(payload);
        }
        fs::write(self.0.join("blk00000.dat"), records).unwrap();
        previous.reverse();
        hex::encode(previous)
    }

    fn run(&self, action: &str, output: &Path) -> Output {
        self.run_with_args(action, output, &[])
    }

    fn run_with_args(&self, action: &str, output: &Path, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_bitiodine"))
            .arg("--blocks-dir")
            .arg(&self.0)
            .args(["--action", action, "--output"])
            .arg(output)
            .args(args)
            .output()
            .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn missing_block_files_fail() {
    let fixture = Fixture::new();
    let output = fixture.run("merkle", &fixture.0.join("output.csv"));
    assert!(!output.status.success(), "{:?}", output);
    assert!(!String::from_utf8_lossy(&output.stderr).contains("verified successfully"));
}

#[test]
fn malformed_block_files_fail_for_every_action() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join("blk00000.dat"), [0xf9, 0xbe, 0xb4]).unwrap();
    for action in [
        "clusterizer",
        "dump-balances",
        "dump-addresses",
        "dump-tx-hashes",
        "dataoutput-finder",
        "donation-finder",
        "merkle",
    ] {
        let output = fixture.run(action, &fixture.0.join("output.csv"));
        assert!(!output.status.success(), "{action}: {output:?}");
        assert!(!String::from_utf8_lossy(&output.stderr).contains("panicked"));
    }
}

#[test]
fn valid_chain_writes_nonempty_csv_and_verifies_merkle_roots() {
    let fixture = Fixture::new();
    fixture.valid_chain();
    for action in ["clusterizer", "dump-balances"] {
        let path = fixture.0.join(format!("{action}.csv"));
        let output = fixture.run(action, &path);
        assert!(output.status.success(), "{action}: {output:?}");
        assert!(!fs::read(path).unwrap().is_empty());
    }
    let output = fixture.run("merkle", &fixture.0.join("unused.csv"));
    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stderr).contains("verified successfully"));
}

#[cfg(target_os = "linux")]
#[test]
fn csv_flush_errors_fail_without_reporting_success() {
    let fixture = Fixture::new();
    fixture.valid_chain();
    for action in ["clusterizer", "dump-balances"] {
        let output = fixture.run(action, Path::new("/dev/full"));
        assert!(!output.status.success(), "{action}: {output:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("Done!"), "{stderr}");
        assert!(!stderr.contains("panicked"), "{stderr}");
    }
}

#[test]
fn explicit_chain_tip_selects_the_requested_chain() {
    let fixture = Fixture::new();
    let tip = fixture.valid_chain();
    let output = fixture.run_with_args(
        "merkle",
        &fixture.0.join("unused.csv"),
        &["--chain-tip", &tip],
    );
    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stderr).contains("verified successfully"));
}

#[test]
fn unknown_or_invalid_chain_tips_fail() {
    let fixture = Fixture::new();
    fixture.valid_chain();
    for tip in ["00".repeat(32), "not-a-block-hash".to_owned()] {
        let output = fixture.run_with_args(
            "merkle",
            &fixture.0.join("unused.csv"),
            &["--chain-tip", &tip],
        );
        assert!(!output.status.success(), "{output:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("verified successfully"), "{stderr}");
        assert!(!stderr.contains("panicked"), "{stderr}");
    }
}
