use crypto::digest::Digest;
use crypto::sha2::Sha256;
use rustc_serialize::hex::{FromHex, ToHex};
use std::ops::{Deref, DerefMut};
use std::{fmt, hash, mem};

#[derive(PartialEq, Eq, Debug, Copy, Clone, Default, Ord, PartialOrd)]
pub struct Hash([u8; 32]);

impl fmt::Display for Hash {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let mut hash = self.0;
        hash.reverse();
        let hash = hash.to_hex();
        hash.fmt(formatter)
    }
}

impl hash::Hash for Hash {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: hash::Hasher,
    {
        hasher.write(&self.0[..]);
    }
}

impl Hash {
    pub fn from_pretty(s: &str) -> Hash {
        let buf = s.from_hex().unwrap();
        assert_eq!(buf.len(), 32);
        let mut out = [0u8; 32];
        for n in 0..32 {
            out[n] = buf[31 - n];
        }
        Hash(out)
    }

    pub fn from_data(data: &[u8]) -> Hash {
        let mut out = [0u8; 32];
        let mut hasher1 = Sha256::new();
        let mut hasher2 = hasher1;

        hasher1.input(data);
        hasher1.result(&mut out);

        hasher2.input(&out);
        hasher2.result(&mut out);

        Hash(out)
    }

    pub fn from_slice(slice: &[u8; 32]) -> &Hash {
        unsafe { mem::transmute(slice) }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

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
