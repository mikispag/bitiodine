use ripemd::Ripemd160;
use sha2::{Digest, Sha256};
use std::fmt;
use std::ops::{Deref, DerefMut};

#[repr(transparent)]
#[derive(PartialEq, Eq, Debug, Copy, Clone, Default, Ord, PartialOrd, Hash)]
pub struct Hash160(pub [u8; 20]);

impl fmt::Display for Hash160 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", hex::encode(self.0))
    }
}

impl Hash160 {
    pub fn from_pretty(s: &str) -> Result<Hash160, hex::FromHexError> {
        let mut buf = [0u8; 20];
        hex::decode_to_slice(s, &mut buf)?;
        buf.reverse();
        Ok(Hash160(buf))
    }

    pub fn from_data(data: &[u8]) -> Hash160 {
        let sha = Sha256::digest(data);
        let ripe = Ripemd160::digest(sha);
        Hash160(ripe.into())
    }

    #[inline]
    pub fn from_slice(slice: &[u8; 20]) -> &Hash160 {
        // Safe: Hash160 is #[repr(transparent)] over [u8; 20]
        unsafe { &*(slice.as_ptr() as *const Hash160) }
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

impl Deref for Hash160 {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl DerefMut for Hash160 {
    fn deref_mut(&mut self) -> &mut [u8] {
        self.as_mut_slice()
    }
}
