use crate::hash::Hash;
use crate::hash160::Hash160;
use bitcoin_bech32::constants::Network;
use bitcoin_bech32::u5;
use bitcoin_bech32::WitnessProgram;
use smallvec::SmallVec;
use std::fmt;

#[derive(PartialEq, Eq, Debug, Clone, Hash, Ord, PartialOrd)]
pub enum Address {
    Base58 {
        version: u8,
        hash: [u8; 20],
    },
    Witness {
        version: u8,
        program: SmallVec<[u8; 32]>,
    },
}

impl Default for Address {
    fn default() -> Self {
        Address::Base58 {
            version: 0,
            hash: [0u8; 20],
        }
    }
}

impl fmt::Display for Address {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Address::Base58 { version, hash } => {
                let mut v = [0u8; 25];
                v[0] = *version;
                v[1..21].copy_from_slice(hash);
                let h = Hash::from_data(&v[0..21]);
                v[21..25].copy_from_slice(&h[0..4]);
                let s = bs58::encode(&v).into_string();
                formatter.write_str(&s)
            }
            Address::Witness { version, program } => {
                if let Ok(u5_ver) = u5::try_from_u8(*version) {
                    if let Ok(wp) = WitnessProgram::new(u5_ver, program.to_vec(), Network::Bitcoin)
                    {
                        return formatter.write_str(&wp.to_address());
                    }
                }
                write!(formatter, "invalid_witness_v{}", version)
            }
        }
    }
}

impl Address {
    pub fn from_pubkey(pubkey: &[u8], version: u8) -> Address {
        let hash160 = Hash160::from_data(pubkey);
        Address::from_hash160(&hash160, version)
    }

    pub fn from_hash160(hash160: &Hash160, version: u8) -> Address {
        Address::Base58 {
            version,
            hash: hash160.0,
        }
    }

    pub fn from_witness_program(w: &WitnessProgram) -> Address {
        let version = w.version().to_u8();
        let program = SmallVec::from_slice(w.program());
        Address::Witness { version, program }
    }
}
