use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

fn protected_name(path: &Path) -> bool {
    path.file_name().is_some_and(|name| {
        let name = name.to_string_lossy();
        name == "xor.dat" || (name.starts_with("blk") && name.ends_with(".dat"))
    })
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

/// Resolve a directory that may not exist yet without creating it.
fn resolve_directory(path: &Path) -> io::Result<PathBuf> {
    if path.as_os_str().is_empty() {
        return Path::new(".").canonicalize();
    }
    match path.canonicalize() {
        Ok(path) => Ok(path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if fs::symlink_metadata(path).is_ok() {
                return Err(invalid("State directory must not be a dangling symlink"));
            }
            let name = path
                .file_name()
                .ok_or_else(|| invalid("Invalid state directory"))?;
            Ok(resolve_directory(path.parent().unwrap_or_else(|| Path::new(".")))?.join(name))
        }
        Err(error) => Err(error),
    }
}

/// State writes must be disjoint from both the read-only source and CSV output.
pub fn validate_state_dir(blocks_dir: &Path, output: &Path, state_dir: &Path) -> io::Result<()> {
    let blocks_dir = blocks_dir.canonicalize()?;
    let output = validate_output(&blocks_dir, output)?;
    let state_dir = resolve_directory(state_dir)?;
    if state_dir.starts_with(&blocks_dir) || blocks_dir.starts_with(&state_dir) {
        return Err(invalid(
            "Checkpoint directory and blockchain input directory must be disjoint",
        ));
    }
    if output.starts_with(&state_dir) {
        return Err(invalid("Output must be outside the checkpoint directory"));
    }
    let mut managed = Vec::new();
    for name in [
        "cluster.lock",
        "clusters.sqlite3",
        "clusters.sqlite3-wal",
        "clusters.sqlite3-shm",
        "clusters.sqlite3-journal",
    ] {
        let path = state_dir.join(name);
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if !metadata.is_file() {
                    return Err(invalid(
                        "Checkpoint files must be regular files, not symlinks or devices",
                    ));
                }
                if output.exists() && same_file::is_same_file(&path, &output)? {
                    return Err(invalid("Output aliases a checkpoint file"));
                }
                for other in &managed {
                    if same_file::is_same_file(&path, other)? {
                        return Err(invalid("Checkpoint files must not alias each other"));
                    }
                }
                managed.push(path);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    if !managed.is_empty() {
        for entry in fs::read_dir(blocks_dir)? {
            let input = entry?.path();
            if protected_name(&input) {
                for path in &managed {
                    if same_file::is_same_file(&input, path)? {
                        return Err(invalid("Checkpoint file aliases blockchain input"));
                    }
                }
            }
        }
    }
    Ok(())
}

/// Resolves the destination and rejects source aliases, reserved names and devices.
pub fn validate_output(blocks_dir: &Path, output: &Path) -> io::Result<PathBuf> {
    let blocks_dir = blocks_dir.canonicalize()?;
    let destination = match output.canonicalize() {
        Ok(path) => path,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if fs::symlink_metadata(output).is_ok() {
                return Err(invalid("Output must not be a dangling symlink"));
            }
            let name = output
                .file_name()
                .ok_or_else(|| invalid("Output must name a regular file"))?;
            output
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."))
                .canonicalize()?
                .join(name)
        }
        Err(error) => return Err(error),
    };
    if destination.parent() == Some(blocks_dir.as_path()) && protected_name(&destination) {
        return Err(invalid(format!(
            "Output would overwrite blockchain input: {}",
            destination.display()
        )));
    }
    match fs::metadata(&destination) {
        Ok(metadata) => {
            if !metadata.is_file() {
                return Err(invalid("Output must be a regular file"));
            }
            // Check every source, including files excluded by --max-blocks.
            for entry in fs::read_dir(&blocks_dir)? {
                let input = entry?.path();
                if protected_name(&input) && same_file::is_same_file(&input, &destination)? {
                    return Err(invalid(format!(
                        "Output aliases blockchain input: {}",
                        input.display()
                    )));
                }
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    Ok(destination)
}

struct PendingOutput(PathBuf);

impl Drop for PendingOutput {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// Publishes a complete CSV only after writing, flushing and syncing succeed.
pub fn write_atomic(
    blocks_dir: &Path,
    output: &Path,
    write: impl FnOnce(&mut BufWriter<File>) -> io::Result<()>,
) -> io::Result<()> {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let destination = validate_output(blocks_dir, output)?;
    let parent = destination.parent().unwrap();
    let (temporary, file) = loop {
        let path = parent.join(format!(
            ".bitiodine-{}-{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => break (PendingOutput(path), file),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };
    let mut writer = BufWriter::new(file);
    if let Ok(metadata) = fs::metadata(&destination) {
        writer.get_ref().set_permissions(metadata.permissions())?;
    }
    write(&mut writer)?;
    writer.flush()?;
    writer.get_ref().sync_all()?;
    drop(writer);
    validate_output(blocks_dir, &destination)?;
    fs::rename(&temporary.0, &destination)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_write_preserves_previous_csv_and_removes_temporary_file() {
        let directory =
            std::env::temp_dir().join(format!("bitiodine-atomic-output-{}", std::process::id()));
        fs::create_dir(&directory).unwrap();
        let output = directory.join("results.csv");
        fs::write(&output, b"previous results\n").unwrap();
        let result = write_atomic(&directory, &output, |writer| {
            writer.write_all(b"partial replacement\n")?;
            writer.flush()?;
            Err(io::Error::other("injected write failure"))
        });
        assert!(result.is_err());
        assert_eq!(fs::read(&output).unwrap(), b"previous results\n");
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
        fs::remove_dir_all(directory).unwrap();
    }
}
