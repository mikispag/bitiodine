use preamble::*;

#[derive(Clone, Copy)]
pub struct BlockHeader<'a>(&'a [u8; 80]);

impl<'a> BlockHeader<'a> {
    pub fn new(slice: &[u8; 80]) -> BlockHeader {
        BlockHeader(slice)
    }

    pub fn as_slice(&self) -> &'a [u8; 80] {
        &self.0
    }

    pub fn version(&self) -> i32 {
        let mut slice = &self.0[0..];
        slice.read_i32::<LittleEndian>().unwrap()
    }

    pub fn cur_hash(&self) -> Hash {
        Hash::from_data(self.0)
    }

    pub fn prev_hash(&self) -> &'a Hash {
        Hash::from_slice(array_ref!(self.0, 4, 32))
    }

    pub fn merkle_root(&self) -> &'a Hash {
        Hash::from_slice(array_ref!(self.0, 36, 32))
    }

    pub fn timestamp(&self) -> u32 {
        let mut slice = &self.0[68..];
        slice.read_u32::<LittleEndian>().unwrap()
    }

    pub fn bits(&self) -> u32 {
        let mut slice = &self.0[72..];
        slice.read_u32::<LittleEndian>().unwrap()
    }

    pub fn nonce(&self) -> u32 {
        let mut slice = &self.0[76..];
        slice.read_u32::<LittleEndian>().unwrap()
    }
}
