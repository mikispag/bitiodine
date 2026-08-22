use crate::hash::Hash;

#[derive(Clone, Copy)]
pub struct BlockHeader<'a>(&'a [u8; 80]);

impl<'a> BlockHeader<'a> {
    pub fn new(slice: &'a [u8; 80]) -> BlockHeader<'a> {
        BlockHeader(slice)
    }

    pub fn as_slice(&self) -> &'a [u8; 80] {
        self.0
    }

    pub fn version(&self) -> i32 {
        let slice: [u8; 4] = self.0[0..4].try_into().unwrap();
        i32::from_le_bytes(slice)
    }

    pub fn cur_hash(&self) -> Hash {
        Hash::from_data(self.0)
    }

    pub fn prev_hash(&self) -> &'a Hash {
        let slice: &'a [u8; 32] = self.0[4..36].try_into().unwrap();
        Hash::from_slice(slice)
    }

    pub fn merkle_root(&self) -> &'a Hash {
        let slice: &'a [u8; 32] = self.0[36..68].try_into().unwrap();
        Hash::from_slice(slice)
    }

    pub fn timestamp(&self) -> u32 {
        let slice: [u8; 4] = self.0[68..72].try_into().unwrap();
        u32::from_le_bytes(slice)
    }

    pub fn bits(&self) -> u32 {
        let slice: [u8; 4] = self.0[72..76].try_into().unwrap();
        u32::from_le_bytes(slice)
    }

    pub fn nonce(&self) -> u32 {
        let slice: [u8; 4] = self.0[76..80].try_into().unwrap();
        u32::from_le_bytes(slice)
    }
}
