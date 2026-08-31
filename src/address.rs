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

pub const COMPACT_ADDRESS_SIZE: usize = 42;
const WITNESS_TAG: u8 = 0x80;

/// Copy, 42-byte, align-1, zero-heap address for hot maps.
/// Byte 0 doubles as kind + version: values < 0x80 are Base58 version bytes
/// (0x00 P2PKH, 0x05 P2SH) followed by the 20-byte HASH160; values >= 0x80 are
/// witness programs with version = byte0 & 0x1f, the program zero-padded at
/// bytes 1..41 and its length at byte 41. Derived `Ord` over the raw bytes is
/// equivalent to `Address`'s derived ordering (variant, version, lexicographic
/// payload with prefix-shorter-first semantics).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[repr(transparent)]
pub struct CompactAddress([u8; COMPACT_ADDRESS_SIZE]);

impl CompactAddress {
    pub fn from_hash160(hash160: &Hash160, version: u8) -> Self {
        debug_assert!(version < WITNESS_TAG);
        let mut raw = [0u8; COMPACT_ADDRESS_SIZE];
        raw[0] = version;
        raw[1..21].copy_from_slice(&hash160.0);
        CompactAddress(raw)
    }

    pub fn from_witness_program(w: &WitnessProgram) -> Self {
        let program = w.program();
        debug_assert!(program.len() <= 40);
        let mut raw = [0u8; COMPACT_ADDRESS_SIZE];
        raw[0] = WITNESS_TAG | w.version().to_u8();
        raw[1..1 + program.len()].copy_from_slice(program);
        raw[41] = program.len() as u8;
        CompactAddress(raw)
    }

    /// HASH160 for the Base58 variant; all-zero (like the previous
    /// `Option<Hash160>::None -> default` semantics) for witness programs.
    pub fn hash160(&self) -> Hash160 {
        if self.0[0] >= WITNESS_TAG {
            Hash160::default()
        } else {
            let mut hash = [0u8; 20];
            hash.copy_from_slice(&self.0[1..21]);
            Hash160(hash)
        }
    }
}

impl fmt::Display for CompactAddress {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tag = self.0[0];
        if tag < WITNESS_TAG {
            let mut v = [0u8; 25];
            v[0] = tag;
            v[1..21].copy_from_slice(&self.0[1..21]);
            let h = Hash::from_data(&v[0..21]);
            v[21..25].copy_from_slice(&h[0..4]);
            formatter.write_str(&bs58::encode(&v).into_string())
        } else {
            let witness_version = tag & 0x1F;
            let len = self.0[41] as usize;
            if let Ok(u5_ver) = u5::try_from_u8(witness_version) {
                if let Ok(wp) =
                    WitnessProgram::new(u5_ver, self.0[1..1 + len].to_vec(), Network::Bitcoin)
                {
                    return formatter.write_str(&wp.to_address());
                }
            }
            write!(formatter, "invalid_witness_v{}", witness_version)
        }
    }
}
