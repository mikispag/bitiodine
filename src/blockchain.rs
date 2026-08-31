use log::{debug, info};
use memmap2::Mmap;
use smallvec::SmallVec;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::block::Block;
use crate::error::ParseResult;
use crate::hash::{Hash, ZERO_HASH};
use crate::visitors::BlockChainVisitor;

pub type OutputMap<T> = HashMap<Hash, SmallVec<[(u32, T); 1]>>;

pub struct BlockChain {
    blocks_dir: PathBuf,
    num_files: usize,
    xor_key: Option<Vec<u8>>,
}

fn apply_xor(buf: &mut [u8], key: &[u8]) {
    if key.is_empty() || key.iter().all(|&b| b == 0) {
        return;
    }
    if key.len() == 8 {
        let key_u64 = u64::from_ne_bytes(key.try_into().unwrap());
        let (prefix, chunks, suffix) = unsafe { buf.align_to_mut::<u64>() };
        for (i, b) in prefix.iter_mut().enumerate() {
            *b ^= key[i % 8];
        }
        for chunk in chunks.iter_mut() {
            *chunk ^= key_u64;
        }
        let offset = (prefix.len() + chunks.len() * 8) % 8;
        for (i, b) in suffix.iter_mut().enumerate() {
            *b ^= key[(offset + i) % 8];
        }
    } else {
        for (i, b) in buf.iter_mut().enumerate() {
            *b ^= key[i % key.len()];
        }
    }
}

impl BlockChain {
    /// Discovers all `blk*.dat` Bitcoin block files in `blocks_dir` and reads `xor.dat` if present.
    /// Files are memory-mapped on-demand one file at a time during `walk()` to bound RAM usage.
    /// At most `max_files` files are processed (0 = no limit).
    ///
    /// # Safety
    /// The caller must ensure that the block files are not concurrently mutated or truncated
    /// by another process (such as `bitcoind`) while mapped, as that could cause undefined behavior.
    pub unsafe fn read<P: AsRef<Path>>(blocks_dir: P, max_files: usize) -> BlockChain {
        let blocks_dir_path = blocks_dir.as_ref().to_path_buf();
        let xor_key = std::fs::read(blocks_dir_path.join("xor.dat"))
            .ok()
            .filter(|k| !k.is_empty() && k.iter().any(|&b| b != 0));
        if xor_key.is_some() {
            info!("Detected XOR obfuscation key in xor.dat");
        }

        let mut num_files = 0;
        loop {
            let blk_path = blocks_dir_path.join(format!("blk{:05}.dat", num_files));
            if !blk_path.exists() {
                break;
            }
            num_files += 1;
        }
        if max_files > 0 && num_files > max_files {
            num_files = max_files;
        }

        BlockChain {
            blocks_dir: blocks_dir_path,
            num_files,
            xor_key,
        }
    }

    pub fn len(&self) -> usize {
        self.num_files
    }

    pub fn is_empty(&self) -> bool {
        self.num_files == 0
    }

    #[allow(clippy::too_many_arguments)]
    fn walk_slice<V: BlockChainVisitor>(
        &self,
        mut slice: &[u8],
        goal_prev_hash: &mut Hash,
        last_block_buf: &mut Option<Vec<u8>>,
        height: &mut u64,
        skipped: &mut HashMap<Hash, Vec<u8>>,
        output_items: &mut OutputMap<V::OutputItem>,
        visitor: &mut V,
    ) -> ParseResult<()> {
        while !slice.is_empty() {
            if skipped.contains_key(goal_prev_hash) {
                if let Some(lb_bytes) = last_block_buf.take() {
                    let lb = Block(&lb_bytes);
                    lb.walk(visitor, *height, output_items)?;
                    debug!(
                        "(rewind - pre-step) Block {} - {} -> {}",
                        height,
                        lb.header().prev_hash(),
                        lb.header().cur_hash()
                    );
                    *height += 1;
                }
                while let Some(block_bytes) = skipped.remove(goal_prev_hash) {
                    let block = Block(&block_bytes);
                    block.walk(visitor, *height, output_items)?;
                    debug!(
                        "(rewind) Block {} - {} -> {}",
                        height,
                        block.header().prev_hash(),
                        block.header().cur_hash()
                    );
                    *height += 1;
                    *goal_prev_hash = block.header().cur_hash();
                }
            }

            let block = match Block::read(&mut slice)? {
                Some(block) => block,
                None => {
                    assert_eq!(slice.len(), 0);
                    break;
                }
            };

            debug!(
                "Block candidate for height {} - goal_prev_hash = {}, prev_hash = {}, cur_hash = {}",
                height,
                goal_prev_hash,
                block.header().prev_hash(),
                block.header().cur_hash()
            );

            if block.header().prev_hash() != goal_prev_hash {
                skipped.insert(*block.header().prev_hash(), block.0.to_vec());

                if let Some(ref lb_bytes) = last_block_buf {
                    let lb = Block(lb_bytes);
                    if block.header().prev_hash() == lb.header().prev_hash() {
                        debug!(
                            "Chain split detected: {} <-> {}. Detecting main chain and orphan.",
                            lb.header().cur_hash(),
                            block.header().cur_hash()
                        );

                        let first_orphan_bytes = lb_bytes.clone();
                        let second_orphan_bytes = block.0.to_vec();
                        let first_cur_hash = Block(&first_orphan_bytes).header().cur_hash();
                        let second_cur_hash = Block(&second_orphan_bytes).header().cur_hash();

                        loop {
                            let block = match Block::read(&mut slice)? {
                                Some(block) => block,
                                None => {
                                    assert_eq!(slice.len(), 0);
                                    break;
                                }
                            };
                            skipped.insert(*block.header().prev_hash(), block.0.to_vec());
                            if block.header().prev_hash() == &first_cur_hash {
                                // First wins
                                debug!("Chain split: {} is on the main chain!", first_cur_hash);
                                break;
                            }
                            if block.header().prev_hash() == &second_cur_hash {
                                // Second wins
                                debug!("Chain split: {} is on the main chain!", second_cur_hash);
                                *goal_prev_hash = second_cur_hash;
                                *last_block_buf = Some(second_orphan_bytes);
                                break;
                            }
                        }
                    }
                }
                continue;
            }

            if let Some(lb_bytes) = last_block_buf.take() {
                let lb = Block(&lb_bytes);
                lb.walk(visitor, *height, output_items)?;
                debug!(
                    "(last_block) Block {} - {} -> {}",
                    height,
                    lb.header().prev_hash(),
                    lb.header().cur_hash()
                );
                *height += 1;
            }

            *goal_prev_hash = block.header().cur_hash();
            *last_block_buf = Some(block.0.to_vec());
        }

        Ok(())
    }

    pub fn walk<V: BlockChainVisitor>(
        &self,
        visitor: &mut V,
    ) -> ParseResult<(u64, Hash, OutputMap<V::OutputItem>)> {
        let mut skipped: HashMap<Hash, Vec<u8>> = Default::default();
        let mut output_items: OutputMap<V::OutputItem> = Default::default();
        let mut goal_prev_hash: Hash = ZERO_HASH;
        let mut last_block_buf: Option<Vec<u8>> = None;
        let mut height = 0;
        let mut scratch: Vec<u8> = Vec::new();

        for n in 0..self.num_files {
            info!(
                "Parsing the blockchain: block file {}/{}...",
                n,
                self.num_files.saturating_sub(1)
            );

            let blk_path = self.blocks_dir.join(format!("blk{:05}.dat", n));
            let mut file = match File::open(&blk_path) {
                Ok(f) => f,
                Err(_) => break,
            };

            let file_len = file.metadata().map(|m| m.len()).unwrap_or(0) as usize;
            if file_len == 0 {
                continue;
            }

            if let Some(ref key) = self.xor_key {
                let file_len = file_len as usize;
                if scratch.len() < file_len {
                    scratch.resize(file_len, 0);
                }
                if file.read_exact(&mut scratch[..file_len]).is_err() {
                    break;
                }
                let non_zero_len = scratch[..file_len]
                    .iter()
                    .rposition(|&b| b != 0)
                    .map_or(0, |idx| idx + 1);
                if non_zero_len == 0 {
                    continue;
                }
                apply_xor(&mut scratch[..non_zero_len], key);
                self.walk_slice(
                    &scratch[..file_len],
                    &mut goal_prev_hash,
                    &mut last_block_buf,
                    &mut height,
                    &mut skipped,
                    &mut output_items,
                    visitor,
                )?;
            } else {
                let mmap = match unsafe { Mmap::map(&file) } {
                    Ok(m) => m,
                    Err(_) => break,
                };
                self.walk_slice(
                    &mmap,
                    &mut goal_prev_hash,
                    &mut last_block_buf,
                    &mut height,
                    &mut skipped,
                    &mut output_items,
                    visitor,
                )?;
            }
        }

        if let Some(lb_bytes) = last_block_buf.take() {
            let lb = Block(&lb_bytes);
            lb.walk(visitor, height, &mut output_items)?;
            height += 1;
        }

        while let Some(block_bytes) = skipped.remove(&goal_prev_hash) {
            let block = Block(&block_bytes);
            block.walk(visitor, height, &mut output_items)?;
            height += 1;
            goal_prev_hash = block.header().cur_hash();
        }

        info!("OutputMap entries at end of walk: {}", output_items.len());

        Ok((height, goal_prev_hash, output_items))
    }
}
