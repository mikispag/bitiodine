use sha2::{Digest, Sha256};
use std::fmt;
use std::ops::{Deref, DerefMut};

#[repr(transparent)]
#[derive(PartialEq, Eq, Debug, Copy, Clone, Default, Ord, PartialOrd, Hash)]
pub struct Hash(pub [u8; 32]);

impl fmt::Display for Hash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut hash = self.0;
        hash.reverse();
        write!(formatter, "{}", hex::encode(hash))
    }
}

impl Hash {
    pub fn from_pretty(s: &str) -> Result<Hash, hex::FromHexError> {
        let mut buf = [0u8; 32];
        hex::decode_to_slice(s, &mut buf)?;
        buf.reverse();
        Ok(Hash(buf))
    }

    pub fn from_data(data: &[u8]) -> Hash {
        let first = Sha256::digest(data);
        let second = Sha256::digest(first);
        Hash(second.into())
    }

    #[inline]
    pub fn from_slice(slice: &[u8; 32]) -> &Hash {
        // Safe: Hash is #[repr(transparent)] over [u8; 32]
        unsafe { &*(slice.as_ptr() as *const Hash) }
    }

    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl Deref for Hash {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl DerefMut for Hash {
    fn deref_mut(&mut self) -> &mut [u8] {
        self.as_mut_slice()
    }
}

pub static ZERO_HASH: Hash = Hash([0; 32]);
