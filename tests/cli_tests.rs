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
        let mut command = Command::new(env!("CARGO_BIN_EXE_bitiodine"));
        if action == "clusterizer" {
            command
                .arg("--checkpoint-dir")
                .arg(self.0.with_extension("state"));
        }
        command
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
        let state = self.0.with_extension("state");
        if state.exists() {
            fs::remove_dir_all(state).unwrap();
        }
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
fn nonregular_csv_destinations_fail_without_reporting_success() {
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

#[test]
fn file_limit_excludes_later_corrupt_files() {
    let fixture = Fixture::new();
    fixture.valid_chain();
    fs::write(fixture.0.join("blk00001.dat"), [1; 8]).unwrap();
    let output = fixture.run_with_args(
        "merkle",
        &fixture.0.join("unused.csv"),
        &["--max-blocks", "1"],
    );
    assert!(output.status.success(), "{output:?}");
    let output = fixture.run("merkle", &fixture.0.join("unused.csv"));
    assert!(!output.status.success(), "{output:?}");
}

#[test]
fn failed_traversal_preserves_existing_csv() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join("blk00000.dat"), [1; 8]).unwrap();
    let path = fixture.0.join("results.csv");
    let previous = b"previous successful results\n";
    fs::write(&path, previous).unwrap();
    for action in ["clusterizer", "dump-balances"] {
        let output = fixture.run(action, &path);
        assert!(!output.status.success(), "{action}: {output:?}");
        assert_eq!(fs::read(&path).unwrap(), previous);
    }
}

#[cfg(unix)]
#[test]
fn csv_outputs_must_not_overwrite_input_data() {
    let mut overwritten = Vec::new();
    for action in ["clusterizer", "dump-balances"] {
        for alias in ["direct", "symlink", "hardlink", "xor.dat"] {
            let fixture = Fixture::new();
            fixture.valid_chain();
            let input = fixture.0.join(if alias == "xor.dat" {
                "xor.dat"
            } else {
                "blk00000.dat"
            });
            if alias == "xor.dat" {
                fs::write(&input, [0; 8]).unwrap();
            }
            let before = fs::read(&input).unwrap();
            let output_path = match alias {
                "symlink" => {
                    let path = fixture.0.join("results.csv");
                    std::os::unix::fs::symlink(&input, &path).unwrap();
                    path
                }
                "hardlink" => {
                    let path = fixture.0.join("results.csv");
                    fs::hard_link(&input, &path).unwrap();
                    path
                }
                _ => input.clone(),
            };
            let output = fixture.run(action, &output_path);
            if output.status.success() || fs::read(&input).unwrap() != before {
                overwritten.push(format!("{action}/{alias}: exit {}", output.status));
            }
        }
    }
    assert!(
        overwritten.is_empty(),
        "input protection failed: {}",
        overwritten.join(", ")
    );
}

#[cfg(unix)]
#[test]
fn csv_output_protects_unselected_files_and_reserved_names() {
    for action in ["clusterizer", "dump-balances"] {
        for alias in [
            "direct",
            "symlink",
            "hardlink",
            "missing-block",
            "missing-xor",
            "dangling",
        ] {
            let fixture = Fixture::new();
            fixture.valid_chain();
            let input = fixture.0.join("blk00001.dat");
            let before = b"unselected block data";
            fs::write(&input, before).unwrap();
            let output_path = match alias {
                "symlink" | "hardlink" => {
                    let path = fixture.0.join("results.csv");
                    if alias == "symlink" {
                        std::os::unix::fs::symlink(&input, &path).unwrap();
                    } else {
                        fs::hard_link(&input, &path).unwrap();
                    }
                    path
                }
                "missing-block" => fixture.0.join("blk00002.dat"),
                "missing-xor" => fixture.0.join("xor.dat"),
                "dangling" => {
                    let path = fixture.0.join("results.csv");
                    std::os::unix::fs::symlink(fixture.0.join("xor.dat"), &path).unwrap();
                    path
                }
                _ => input.clone(),
            };
            let output = fixture.run_with_args(action, &output_path, &["--max-blocks", "1"]);
            assert!(!output.status.success(), "{action}/{alias}: {output:?}");
            assert_eq!(fs::read(&input).unwrap(), before);
            assert!(!fixture.0.join("blk00002.dat").exists());
            assert!(!fixture.0.join("xor.dat").exists());
            assert!(!String::from_utf8_lossy(&output.stderr).contains("Indexing block file"));
        }
    }
}

#[test]
fn successful_csv_replaces_previous_results_without_leaving_temporary_files() {
    let fixture = Fixture::new();
    fixture.valid_chain();
    let path = fixture.0.join("results.csv");
    for action in ["clusterizer", "dump-balances"] {
        fs::write(&path, b"old results\n").unwrap();
        let output = fixture.run(action, &path);
        assert!(output.status.success(), "{action}: {output:?}");
        let csv = fs::read(&path).unwrap();
        assert!(!csv.is_empty());
        assert_ne!(csv, b"old results\n");
        assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 2);
    }
}
