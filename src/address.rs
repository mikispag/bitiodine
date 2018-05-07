use base58::ToBase58;
use hash::Hash;
use hash160::Hash160;
use std::fmt;

#[derive(PartialEq, Eq, Debug, Clone, Default, Hash, Ord, PartialOrd)]
pub struct Address(pub String);

impl fmt::Display for Address {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.0.fmt(formatter)
    }
}

impl Address {
    pub fn from_pubkey(pubkey: &[u8], version: u8) -> Address {
        let hash160 = Hash160::from_data(pubkey);
        return Address::from_hash160(&hash160, version);
    }

    pub fn from_hash160(hash160: &Hash160, version: u8) -> Address {
        let v: Vec<u8> = [&[version], hash160.as_slice()].concat();
        let h = Hash::from_data(&v);
        Address([&v, &h[0..4]].concat().to_base58())
    }

    pub fn as_slice(&self) -> &str {
        &self.0
    }

    pub fn as_mut_slice(&mut self) -> &mut str {
        &mut self.0
    }
}
