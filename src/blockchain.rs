use bitcoin::pow::{CompactTarget, Target, Work};
use log::info;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::block::{Block, MAINNET_MAGIC};
use crate::error::{ParseError, ParseResult};
use crate::hash::{Hash, ZERO_HASH};
use crate::header::BlockHeader;
use crate::visitors::BlockChainVisitor;

pub type OutputMap<T> = HashMap<Hash, crate::transactions::OutputItemList<T>>;

struct BlockFile {
    path: PathBuf,
    len: u64,
    modified: SystemTime,
}

impl BlockFile {
    fn error(&self, offset: u64, error: impl std::fmt::Display) -> ParseError {
        ParseError::Chain(format!(
            "{} at byte offset {offset}: {error}",
            self.path.display()
        ))
    }

    fn check(&self) -> ParseResult<()> {
        let metadata = std::fs::metadata(&self.path).map_err(|error| self.error(0, error))?;
        if metadata.len() != self.len
            || metadata.modified().map_err(|error| self.error(0, error))? != self.modified
        {
            return Err(ParseError::Chain(format!(
                "Block file changed during analysis: {}",
                self.path.display()
            )));
        }
        Ok(())
    }
}

struct IndexedBlock {
    hash: Hash,
    previous: Hash,
    file: usize,
    offset: u64,
    len: u32,
    work: Work,
}

pub struct BlockChain {
    files: Vec<BlockFile>,
    xor_key: [u8; 8],
    tip: Option<Hash>,
}

fn apply_xor(bytes: &mut [u8], key: &[u8; 8], offset: u64) {
    let mut rotated = [0; 8];
    for (i, byte) in rotated.iter_mut().enumerate() {
        *byte = key[((offset % 8) as usize + i) % 8];
    }
    let key_word = u64::from_ne_bytes(rotated);
    let mut chunks = bytes.chunks_exact_mut(8);
    for chunk in &mut chunks {
        let word = u64::from_ne_bytes(chunk.try_into().unwrap()) ^ key_word;
        chunk.copy_from_slice(&word.to_ne_bytes());
    }
    for (byte, key_byte) in chunks.into_remainder().iter_mut().zip(rotated) {
        *byte ^= key_byte;
    }
}

impl BlockChain {
    /// Discovers consecutive mainnet block files; a zero file limit means no limit.
    ///
    /// Use a stable copy or a stopped node: changes between discovery, indexing and
    /// traversal are errors. Parsing uses owned buffers and does not memory-map files.
    pub fn read<P: AsRef<Path>>(blocks_dir: P, max_files: usize) -> ParseResult<Self> {
        let directory = blocks_dir.as_ref();
        let xor_key = match std::fs::read(directory.join("xor.dat")) {
            Ok(bytes) => bytes.try_into().map_err(|_| {
                ParseError::Chain("xor.dat must contain exactly eight bytes".into())
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => [0; 8],
            Err(e) => return Err(e.into()),
        };
        let mut paths = Vec::new();
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let Some(number) = name
                .strip_prefix("blk")
                .and_then(|n| n.strip_suffix(".dat"))
            else {
                continue;
            };
            if number.len() < 5 || !number.bytes().all(|b| b.is_ascii_digit()) {
                continue;
            }
            let number: usize = number.parse().map_err(|_| ParseError::Invalid)?;
            paths.push((number, entry.path()));
        }
        paths.sort_unstable();
        if max_files > 0 {
            paths.truncate(max_files);
        }
        let mut files = Vec::with_capacity(paths.len());
        for (expected, (number, path)) in paths.into_iter().enumerate() {
            if number != expected {
                return Err(ParseError::Chain(format!(
                    "Missing block file blk{expected:05}.dat"
                )));
            }
            let metadata = std::fs::metadata(&path)?;
            if !metadata.is_file() {
                return Err(ParseError::Chain(format!(
                    "Not a regular block file: {}",
                    path.display()
                )));
            }
            files.push(BlockFile {
                path,
                len: metadata.len(),
                modified: metadata.modified()?,
            });
        }
        Ok(Self {
            files,
            xor_key,
            tip: None,
        })
    }

    /// Anchors traversal to a tip independently verified by Bitcoin Core.
    /// Raw block files can also contain candidates rejected during validation.
    pub fn with_tip(mut self, tip: Hash) -> Self {
        self.tip = Some(tip);
        self
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    fn index(&self) -> ParseResult<Vec<IndexedBlock>> {
        let mut blocks = Vec::new();
        let mut known = std::collections::HashSet::new();
        for (file_index, file) in self.files.iter().enumerate() {
            file.check()?;
            info!(
                "Indexing block file {}/{}...",
                file_index + 1,
                self.files.len()
            );
            let mut reader =
                BufReader::new(File::open(&file.path).map_err(|error| file.error(0, error))?);
            let mut offset = 0;
            while offset < file.len {
                let count = (file.len - offset).min(8) as usize;
                let mut raw_frame = [0; 8];
                reader
                    .read_exact(&mut raw_frame[..count])
                    .map_err(|error| file.error(offset, error))?;
                // Padding starts only at a record boundary and fills the rest of
                // the file. A zero ciphertext header can also begin a real record.
                if raw_frame[..count].iter().all(|b| *b == 0) {
                    let mut tail = [0; 8192];
                    let all_zero = loop {
                        let count = reader
                            .read(&mut tail)
                            .map_err(|error| file.error(offset, error))?;
                        if count == 0 {
                            break true;
                        }
                        if tail[..count].iter().any(|b| *b != 0) {
                            break false;
                        }
                    };
                    if all_zero {
                        break;
                    }
                    reader
                        .seek(SeekFrom::Start(offset + count as u64))
                        .map_err(|error| file.error(offset, error))?;
                }
                let mut frame = raw_frame;
                apply_xor(&mut frame[..count], &self.xor_key, offset);
                if count < 8 || frame[..4] != MAINNET_MAGIC.to_le_bytes() {
                    return Err(file.error(
                        offset,
                        if count < 8 {
                            ParseError::Eof
                        } else {
                            ParseError::Invalid
                        },
                    ));
                }
                let len = u32::from_le_bytes(frame[4..].try_into().unwrap());
                if !(81..=4_000_000).contains(&len) {
                    return Err(file.error(offset, ParseError::Invalid));
                }
                let end = offset + 8 + u64::from(len);
                if end > file.len {
                    return Err(file.error(offset, ParseError::Eof));
                }
                let mut bytes = [0; 80];
                reader
                    .read_exact(&mut bytes)
                    .map_err(|error| file.error(offset, error))?;
                apply_xor(&mut bytes, &self.xor_key, offset + 8);
                let header = BlockHeader::new(&bytes);
                let bits = header.bits();
                let exponent = bits >> 24;
                let mantissa = bits & 0x007fffff;
                if bits & 0x00800000 != 0
                    || exponent > 34
                    || (exponent > 33 && mantissa > 0xff)
                    || (exponent > 32 && mantissa > 0xffff)
                {
                    return Err(file.error(offset, ParseError::Invalid));
                }
                let target = Target::from_compact(CompactTarget::from_consensus(bits));
                if target == Target::ZERO {
                    return Err(file.error(offset, ParseError::Invalid));
                }
                let hash = header.cur_hash();
                if known.insert(hash) {
                    blocks.push(IndexedBlock {
                        hash,
                        previous: *header.prev_hash(),
                        file: file_index,
                        offset: offset + 8,
                        len,
                        work: target.to_work(),
                    });
                }
                reader
                    .seek(SeekFrom::Start(end))
                    .map_err(|error| file.error(offset, error))?;
                offset = end;
            }
            file.check()?;
        }
        Ok(blocks)
    }

    /// Visits the anchored chain, or the unique most-work connected candidate,
    /// after indexing all selected files. Use with_tip for Core's validated chain;
    /// selecting by header work alone does not perform consensus validation.
    /// Without an anchor, equal-work best tips are rejected as ambiguous.
    /// Visitor state may be partially updated if traversal returns an error.
    pub fn walk<V: BlockChainVisitor>(
        &self,
        visitor: &mut V,
    ) -> ParseResult<(u64, Hash, OutputMap<V::OutputItem>)> {
        let blocks = self.index()?;
        let path = select_chain(&blocks, self.tip)?;
        let tip = blocks[*path.last().unwrap()].hash;
        info!("Selected {} blocks ending at {}", path.len(), tip);
        let mut output_items = OutputMap::default();
        let mut buffer = Vec::new();
        let mut open_file: Option<(usize, File)> = None;
        for (height, &index) in path.iter().enumerate() {
            let block = &blocks[index];
            let source = &self.files[block.file];
            if open_file.as_ref().map(|(n, _)| *n) != Some(block.file) {
                source.check()?;
                open_file = Some((
                    block.file,
                    File::open(&source.path).map_err(|error| source.error(block.offset, error))?,
                ));
            }
            let file = &mut open_file.as_mut().unwrap().1;
            file.seek(SeekFrom::Start(block.offset))
                .map_err(|error| source.error(block.offset, error))?;
            buffer.resize(block.len as usize, 0);
            file.read_exact(&mut buffer)
                .map_err(|error| source.error(block.offset, error))?;
            apply_xor(&mut buffer, &self.xor_key, block.offset);
            let parsed = Block(&buffer);
            if parsed.header().cur_hash() != block.hash {
                return Err(source.error(block.offset, "Block header changed after indexing"));
            }
            parsed
                .walk(visitor, height as u64, &mut output_items)
                .map_err(|error| source.error(block.offset, error))?;
        }
        for file in &self.files {
            file.check()?;
        }
        info!("OutputMap entries at end of walk: {}", output_items.len());
        Ok((path.len() as u64, tip, output_items))
    }
}

fn select_chain(blocks: &[IndexedBlock], requested_tip: Option<Hash>) -> ParseResult<Vec<usize>> {
    let by_hash: HashMap<_, _> = blocks
        .iter()
        .enumerate()
        .map(|(i, b)| (b.hash, i))
        .collect();
    let mut resolved = vec![false; blocks.len()];
    let mut scores = vec![None; blocks.len()];
    let zero = Work::from_be_bytes([0; 32]);
    let maximum = Work::from_be_bytes([0xff; 32]);
    let mut pending = Vec::new();
    for start in 0..blocks.len() {
        if resolved[start] {
            continue;
        }
        let mut current = start;
        let mut score = loop {
            if resolved[current] {
                break scores[current];
            }
            resolved[current] = true;
            pending.push(current);
            if blocks[current].previous == ZERO_HASH {
                break Some(zero);
            }
            match by_hash.get(&blocks[current].previous) {
                Some(&parent) => current = parent,
                None => break None,
            }
        };
        while let Some(index) = pending.pop() {
            if let Some(total) = score {
                if blocks[index].work > maximum - total {
                    return Err(ParseError::Invalid);
                }
                score = Some(total + blocks[index].work);
            }
            scores[index] = score;
        }
    }
    let tip = if let Some(hash) = requested_tip {
        let index = *by_hash.get(&hash).ok_or_else(|| {
            ParseError::Chain(format!(
                "Requested chain tip {hash} is missing from the selected files"
            ))
        })?;
        if scores[index].is_none() {
            return Err(ParseError::Chain(format!(
                "Requested chain tip {hash} has missing ancestors"
            )));
        }
        index
    } else {
        let best_score = scores.iter().flatten().max().copied().ok_or_else(|| {
            ParseError::Chain("No complete chain connects to a genesis block".into())
        })?;
        let mut tips = scores
            .iter()
            .enumerate()
            .filter_map(|(i, score)| (*score == Some(best_score)).then_some(i));
        let tip = tips.next().unwrap();
        if tips.next().is_some() {
            return Err(ParseError::Chain(
                "Multiple equal-work chain tips; specify the independently validated --chain-tip"
                    .into(),
            ));
        }
        tip
    };
    let mut path = vec![tip];
    let mut current = tip;
    while blocks[current].previous != ZERO_HASH {
        current = by_hash[&blocks[current].previous];
        path.push(current);
    }
    path.reverse();
    Ok(path)
}
