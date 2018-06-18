use preamble::*;

#[derive(Clone, Copy, Default)]
struct MerkleEntry {
    hash: Hash,
    height: u32,
}

#[derive(Default)]
pub struct MerkleHasher {
    count: u32,
    hashes: [MerkleEntry; 32],
}

impl MerkleHasher {
    fn add_internal(&mut self, mut hash: Hash, mut height: u32) {
        loop {
            if self.count == 0 {
                break;
            }

            let last = &self.hashes[self.count as usize - 1];
            if last.height != height {
                break;
            }

            let mut buf = [0u8; 64];
            buf[..32].copy_from_slice(last.hash.as_slice());
            buf[32..].copy_from_slice(hash.as_slice());
            hash = Hash::from_data(&buf);
            height += 1;
            self.count -= 1;
        }
        self.hashes[self.count as usize] = MerkleEntry {
            hash: hash,
            height: height,
        };
        self.count += 1;
    }

    pub fn add(&mut self, hash: Hash) {
        self.add_internal(hash, 0)
    }

    pub fn finish(mut self) -> Option<Hash> {
        if self.count == 0 {
            None
        } else {
            while self.count > 1 {
                let last = self.hashes[self.count as usize - 1];
                self.add_internal(last.hash, last.height);
            }

            Some(self.hashes[0].hash)
        }
    }
}
