use log::{debug, info};
use memmap2::Mmap;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use vec_map::VecMap;

use crate::block::Block;
use crate::error::ParseResult;
use crate::hash::{Hash, ZERO_HASH};
use crate::visitors::BlockChainVisitor;

pub type OutputMap<T> = HashMap<Hash, VecMap<T>>;

pub struct BlockChain {
    maps: Vec<Mmap>,
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
    /// Reads and memory-maps all `blk*.dat` Bitcoin block files from `blocks_dir`.
    /// Automatically handles XOR-obfuscated blocks (`xor.dat`) used by modern Bitcoin Core.
    ///
    /// # Safety
    /// The caller must ensure that the block files are not concurrently mutated or truncated
    /// by another process (such as `bitcoind`) while mapped, as that could cause undefined behavior.
    pub unsafe fn read<P: AsRef<Path>>(blocks_dir: P) -> BlockChain {
        let mut maps: Vec<Mmap> = Vec::new();
        let mut n: usize = 0;
        let blocks_dir_path = blocks_dir.as_ref();

        let xor_key = std::fs::read(blocks_dir_path.join("xor.dat")).ok();
        let has_xor = matches!(&xor_key, Some(k) if !k.is_empty() && k.iter().any(|&b| b != 0));
        if has_xor {
            info!("Detected XOR obfuscation key in xor.dat");
        }

        loop {
            let blk_path = blocks_dir_path.join(format!("blk{:05}.dat", n));
            match File::open(&blk_path) {
                Ok(f) => {
                    n += 1;
                    // Skip empty/0-byte preallocated files
                    if f.metadata().map(|m| m.len()).unwrap_or(0) == 0 {
                        continue;
                    }
                    if let Some(key) = xor_key
                        .as_ref()
                        .filter(|k| !k.is_empty() && k.iter().any(|&b| b != 0))
                    {
                        match memmap2::MmapOptions::new().map_copy(&f) {
                            Ok(mut m) => {
                                let non_zero_len =
                                    m.iter().rposition(|&b| b != 0).map_or(0, |idx| idx + 1);
                                if non_zero_len == 0 {
                                    continue;
                                }
                                apply_xor(&mut m[..non_zero_len], key);
                                match m.make_read_only() {
                                    Ok(m_ro) => maps.push(m_ro),
                                    Err(_) => break,
                                }
                            }
                            Err(_) => break,
                        }
                    } else {
                        match Mmap::map(&f) {
                            Ok(m) => {
                                maps.push(m);
                            }
                            Err(_) => {
                                break;
                            }
                        }
                    }
                }
                Err(_) => {
                    break;
                }
            };
        }

        BlockChain { maps }
    }

    pub fn len(&self) -> usize {
        self.maps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.maps.is_empty()
    }

    #[allow(clippy::too_many_arguments)]
    fn walk_slice<'a, V: BlockChainVisitor<'a>>(
        &'a self,
        mut slice: &'a [u8],
        goal_prev_hash: &mut Hash,
        last_block: &mut Option<Block<'a>>,
        height: &mut u64,
        skipped: &mut HashMap<Hash, Block<'a>>,
        output_items: &mut OutputMap<V::OutputItem>,
        visitor: &mut V,
    ) -> ParseResult<()> {
        while !slice.is_empty() {
            if skipped.contains_key(goal_prev_hash) {
                if let Some(lb) = last_block.take() {
                    lb.walk(visitor, *height, output_items)?;
                    debug!(
                        "(rewind - pre-step) Block {} - {} -> {}",
                        height,
                        lb.header().prev_hash(),
                        lb.header().cur_hash()
                    );
                    *height += 1;
                }
                while let Some(block) = skipped.remove(goal_prev_hash) {
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
                skipped.insert(*block.header().prev_hash(), block);

                if last_block.is_some()
                    && block.header().prev_hash() == last_block.unwrap().header().prev_hash()
                {
                    debug!(
                        "Chain split detected: {} <-> {}. Detecting main chain and orphan.",
                        last_block.unwrap().header().cur_hash(),
                        block.header().cur_hash()
                    );

                    let first_orphan = last_block.unwrap();
                    let second_orphan = block;

                    loop {
                        let block = match Block::read(&mut slice)? {
                            Some(block) => block,
                            None => {
                                assert_eq!(slice.len(), 0);
                                break;
                            }
                        };
                        skipped.insert(*block.header().prev_hash(), block);
                        if block.header().prev_hash() == &first_orphan.header().cur_hash() {
                            // First wins
                            debug!(
                                "Chain split: {} is on the main chain!",
                                first_orphan.header().cur_hash()
                            );
                            break;
                        }
                        if block.header().prev_hash() == &second_orphan.header().cur_hash() {
                            // Second wins
                            debug!(
                                "Chain split: {} is on the main chain!",
                                second_orphan.header().cur_hash()
                            );
                            *goal_prev_hash = second_orphan.header().cur_hash();
                            *last_block = Some(second_orphan);
                            break;
                        }
                    }
                }
                continue;
            }

            if let Some(lb) = last_block.take() {
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
            *last_block = Some(block);
        }

        Ok(())
    }

    pub fn walk<'a, V: BlockChainVisitor<'a>>(
        &'a self,
        visitor: &mut V,
    ) -> ParseResult<(u64, Hash, OutputMap<V::OutputItem>)> {
        let mut skipped: HashMap<Hash, Block> = Default::default();
        let mut output_items: OutputMap<V::OutputItem> = Default::default();
        let mut goal_prev_hash: Hash = ZERO_HASH;
        let mut last_block: Option<Block> = None;
        let mut height = 0;

        for (n, map) in self.maps.iter().enumerate() {
            info!(
                "Parsing the blockchain: block file {}/{}...",
                n,
                self.maps.len().saturating_sub(1)
            );
            self.walk_slice(
                map,
                &mut goal_prev_hash,
                &mut last_block,
                &mut height,
                &mut skipped,
                &mut output_items,
                visitor,
            )?;
        }

        if let Some(lb) = last_block.take() {
            lb.walk(visitor, height, &mut output_items)?;
            height += 1;
        }

        while let Some(block) = skipped.remove(&goal_prev_hash) {
            block.walk(visitor, height, &mut output_items)?;
            height += 1;
            goal_prev_hash = block.header().cur_hash();
        }

        Ok((height, goal_prev_hash, output_items))
    }
}
