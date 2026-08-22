use crate::hash::Hash;
use crate::hash160::Hash160;
use std::fmt;

#[derive(PartialEq, Eq, Debug, Clone, Default, Hash, Ord, PartialOrd)]
pub struct Address(pub String);

impl fmt::Display for Address {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl Address {
    pub fn from_pubkey(pubkey: &[u8], version: u8) -> Address {
        let hash160 = Hash160::from_data(pubkey);
        Address::from_hash160(&hash160, version)
    }

    pub fn from_hash160(hash160: &Hash160, version: u8) -> Address {
        let mut v = Vec::with_capacity(1 + 20 + 4);
        v.push(version);
        v.extend_from_slice(hash160.as_slice());
        let h = Hash::from_data(&v);
        v.extend_from_slice(&h[0..4]);
        Address(bs58::encode(v).into_string())
    }

    pub fn as_slice(&self) -> &str {
        &self.0
    }

    pub fn as_mut_slice(&mut self) -> &mut str {
        &mut self.0
    }
}
