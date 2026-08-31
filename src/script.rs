use bitcoin_bech32::constants::Network;
use bitcoin_bech32::WitnessProgram;

use crate::bytecode::Bytecode::{self, *};
use crate::error::{ParseError, ParseResult};

#[derive(PartialEq, Clone, Debug)]
pub enum HighLevel<'a> {
    PayToPubkey(&'a [u8]),
    PayToPubkeyHash(&'a [u8; 20]),
    PayToWitnessPubkeyHash(WitnessProgram),
    PayToWitnessScriptHash(WitnessProgram),
    PayToWitnessTaproot(WitnessProgram),
    PayToWitnessGeneral(WitnessProgram),
    PayToMultisig(u32, Vec<&'a [u8]>),
    PayToScriptHash(&'a [u8; 20]),
    DataOutput(&'a [u8]),
    Challenge(ChallengeType<'a>),
    Unknown(Script<'a>),
    Donation,
    Invalid,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum ChallengeType<'a> {
    Ripemd160(&'a [u8; 20]),
    Sha1(&'a [u8; 20]),
    Sha256(&'a [u8; 32]),
    Hash160(&'a [u8; 20]),
    Hash256(&'a [u8; 32]),
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Script<'a> {
    slice: &'a [u8],
    height: u64,
    timestamp: u32,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
struct ScriptIter<'a> {
    slice: &'a [u8],
    height: u64,
    timestamp: u32,
}

impl<'a> Script<'a> {
    pub fn new(slice: &'a [u8], timestamp: u32, height: u64) -> Script<'a> {
        Script {
            slice,
            height,
            timestamp,
        }
    }

    fn iter(&self) -> ScriptIter<'a> {
        ScriptIter {
            slice: self.slice,
            height: self.height,
            timestamp: self.timestamp,
        }
    }

    pub fn as_slice(&self) -> &'a [u8] {
        self.slice
    }

    pub fn to_highlevel(&self) -> HighLevel<'a> {
        let mut skipped_iter = self.iter();
        skipped_iter.skip_nops();
        let skipped_slice = skipped_iter.slice;

        // Check for SegWit v0-v16 Witness Programs (BIP-141 / BIP-341 / BIP-350)
        // Format: OP_n <push len 2..40> <data>
        if self.timestamp >= 1503539857 && skipped_slice.len() >= 4 && skipped_slice.len() <= 42 {
            let first = skipped_slice[0];
            let push_len = skipped_slice[1] as usize;
            if (first == 0x00 || (0x51..=0x60).contains(&first))
                && skipped_slice.len() == 2 + push_len
            {
                if let Ok(w) = WitnessProgram::from_scriptpubkey(skipped_slice, Network::Bitcoin) {
                    match (first, push_len) {
                        (0x00, 20) => return HighLevel::PayToWitnessPubkeyHash(w),
                        (0x00, 32) => return HighLevel::PayToWitnessScriptHash(w),
                        (0x51, 32) => return HighLevel::PayToWitnessTaproot(w),
                        _ => return HighLevel::PayToWitnessGeneral(w),
                    }
                }
            }
        }

        match skipped_slice.len() {
            0 => {
                return HighLevel::Donation;
            }
            23 => {
                if skipped_slice[..2] == [0xa6, 0x14] && skipped_slice[22] == 0x87 {
                    let hash_bytes: &'a [u8; 20] = skipped_slice[2..22].try_into().unwrap();
                    return HighLevel::Challenge(ChallengeType::Ripemd160(hash_bytes));
                }
                if skipped_slice[..2] == [0xa7, 0x14] && skipped_slice[22] == 0x87 {
                    let hash_bytes: &'a [u8; 20] = skipped_slice[2..22].try_into().unwrap();
                    return HighLevel::Challenge(ChallengeType::Sha1(hash_bytes));
                }
                if skipped_slice[..2] == [0xa9, 0x14] && skipped_slice[22] == 0x87 {
                    let hash_bytes: &'a [u8; 20] = skipped_slice[2..22].try_into().unwrap();
                    return HighLevel::Challenge(ChallengeType::Hash160(hash_bytes));
                }
            }
            25 => {
                // The 3-byte terminator variant (0x61 suffix) is handled by the 26-byte arm below.
                if skipped_slice[..3] == [0x76, 0xa9, 0x14] && skipped_slice[23..] == [0x88, 0xac] {
                    let pkh: &'a [u8; 20] = skipped_slice[3..23].try_into().unwrap();
                    return HighLevel::PayToPubkeyHash(pkh);
                }
                if self.timestamp >= 1333238400
                    && self.slice[..2] == [0xa9, 0x14]
                    && self.slice[22] == 0x87
                {
                    let pkh: &'a [u8; 20] = self.slice[2..22].try_into().unwrap();
                    return HighLevel::PayToScriptHash(pkh);
                }
            }
            26 => {
                if skipped_slice[..3] == [0x76, 0xa9, 0x14]
                    && skipped_slice[23..] == [0x88, 0xac, 0x61]
                {
                    let pkh: &'a [u8; 20] = skipped_slice[3..23].try_into().unwrap();
                    return HighLevel::PayToPubkeyHash(pkh);
                }
            }
            35 => {
                if skipped_slice[0] == 33 && skipped_slice[34] == 0xac {
                    let pubkey = &skipped_slice[1..1 + 33];
                    if is_valid_pubkey(pubkey) {
                        return HighLevel::PayToPubkey(pubkey);
                    } else {
                        return HighLevel::Invalid;
                    }
                }
                if skipped_slice[..2] == [0xa8, 0x20] && skipped_slice[34] == 0x87 {
                    let hash_bytes: &'a [u8; 32] = skipped_slice[2..34].try_into().unwrap();
                    return HighLevel::Challenge(ChallengeType::Sha256(hash_bytes));
                }
                if skipped_slice[..2] == [0xaa, 0x20] && skipped_slice[34] == 0x87 {
                    let hash_bytes: &'a [u8; 32] = skipped_slice[2..34].try_into().unwrap();
                    return HighLevel::Challenge(ChallengeType::Hash256(hash_bytes));
                }
            }
            67 if skipped_slice[0] == 65 && skipped_slice[66] == 0xac => {
                let pubkey = &skipped_slice[1..1 + 65];
                if is_valid_pubkey(pubkey) {
                    return HighLevel::PayToPubkey(pubkey);
                } else {
                    return HighLevel::Invalid;
                }
            }
            _ => {}
        }

        if let Ok(res) = skipped_iter.read_pay_to_multisig() {
            return res;
        }

        // OP_RETURN <len(data)> <data>
        if skipped_slice.len() > 2 && skipped_slice[0] == 0x6a {
            let data_len = skipped_slice[1] as usize;
            if skipped_slice.len() != 2 + data_len {
                return HighLevel::Invalid;
            }
            return HighLevel::DataOutput(&skipped_slice[2..2 + data_len]);
        }

        if skipped_slice == b"script" || skipped_slice == [0x76, 0xa9, 0x00, 0x88, 0xac] {
            return HighLevel::Invalid;
        }

        if skipped_slice == b"vvv"
            || skipped_slice == b"v"
            || skipped_slice == [0x53, 0x87]
            || skipped_slice == [0x82]
        {
            return HighLevel::Donation;
        }

        {
            let mut skipped_iter = skipped_iter;
            match skipped_iter.read() {
                Ok(OP_PUSH(bytes)) => match skipped_iter.read() {
                    Err(ParseError::Eof) => {
                        if bytes_to_bool(bytes) {
                            return HighLevel::Donation;
                        } else {
                            return HighLevel::Invalid;
                        }
                    }
                    Err(ParseError::Invalid) => return HighLevel::Invalid,
                    Ok(OP_CHECKSIG) => {
                        if skipped_iter.slice.is_empty() {
                            assert!(!is_valid_pubkey(bytes));
                            return HighLevel::Invalid;
                        }
                    }
                    Ok(OP_PUSH(bytes)) => match skipped_iter.read() {
                        Err(ParseError::Eof) => {
                            if bytes_to_bool(bytes) {
                                return HighLevel::Donation;
                            } else {
                                return HighLevel::Invalid;
                            }
                        }
                        Err(ParseError::Invalid) => return HighLevel::Invalid,
                        _ => (),
                    },
                    _ => (),
                },
                Ok(OP_DUP) => match skipped_iter.read() {
                    Ok(OP_HASH160) => match skipped_iter.read() {
                        Ok(OP_PUSH(bytes)) => match skipped_iter.read() {
                            Err(ParseError::Eof) => {
                                if bytes_to_bool(bytes) {
                                    return HighLevel::Donation;
                                } else {
                                    return HighLevel::Invalid;
                                }
                            }
                            Ok(OP_EQUALVERIFY) => match skipped_iter.read() {
                                Ok(OP_CHECKSIG) => {
                                    if skipped_iter.slice.is_empty() {
                                        assert!(!is_valid_pubkey(bytes));
                                        return HighLevel::Invalid;
                                    } else if skipped_iter.slice.iter().all(|b| *b == 0xac)
                                        || skipped_slice == [0]
                                    {
                                        return HighLevel::Invalid;
                                    }
                                }
                                Err(ParseError::Eof) => {
                                    if bytes.len() == 20 {
                                        let hash_bytes: &'a [u8; 20] =
                                            bytes[..20].try_into().unwrap();
                                        return HighLevel::Challenge(ChallengeType::Hash160(
                                            hash_bytes,
                                        ));
                                    } else {
                                        return HighLevel::Invalid;
                                    }
                                }
                                Err(ParseError::Invalid) => {
                                    return HighLevel::Invalid;
                                }
                                _ => (),
                            },
                            Err(ParseError::Invalid) => return HighLevel::Invalid,
                            _ => (),
                        },
                        Err(ParseError::Invalid) => return HighLevel::Invalid,
                        _ => (),
                    },
                    Err(ParseError::Invalid) => return HighLevel::Invalid,
                    _ => (),
                },
                Err(ParseError::Invalid) => return HighLevel::Invalid,
                _ => (),
            }
        }

        {
            let mut skipped_iter = skipped_iter;
            let mut nest_level = 0;
            loop {
                match skipped_iter.read() {
                    Err(ParseError::Eof) => {
                        if nest_level == 0 {
                            break;
                        } else {
                            return HighLevel::Invalid;
                        }
                    }
                    Err(ParseError::Invalid) => return HighLevel::Invalid,
                    Ok(OP_ELSE | OP_ENDIF | OP_RETURN | OP_INVALID | OP_VER) if nest_level == 0 => {
                        return HighLevel::Invalid
                    }
                    Ok(OP_IF | OP_NOTIF) => {
                        nest_level += 1;
                    }
                    Ok(OP_ENDIF) => {
                        nest_level -= 1;
                    }
                    Ok(_) => {}
                }
            }
        }

        HighLevel::Unknown(*self)
    }
}

impl<'a> ScriptIter<'a> {
    pub fn read(&mut self) -> ParseResult<Bytecode<'a>> {
        Bytecode::read(&mut self.slice, self.height)
    }

    pub fn skip_nops(&mut self) {
        loop {
            let saved = self.slice;
            match self.read() {
                Ok(OP_PUSH(_) | OP_DUP) => {
                    if let Ok(OP_DROP) = self.read() {
                        continue;
                    }
                }
                Ok(OP_DROP | OP_MIN | OP_CHECKSIG | OP_CHECKMULTISIG) => continue,
                _ => {}
            }
            self.slice = saved;
            return;
        }
    }

    pub fn read_pay_to_multisig(&mut self) -> ParseResult<HighLevel<'a>> {
        let signeed = match self.read() {
            Ok(OP_PUSH(data)) => bytes_to_u32(data)?,
            _ => return Err(ParseError::Invalid),
        };

        let mut out: Vec<&[u8]> = Vec::new();

        loop {
            match self.read() {
                Ok(OP_PUSH(bytes)) => out.push(bytes),
                Ok(OP_CHECKMULTISIG) => break,
                _ => return Err(ParseError::Invalid),
            }
        }

        if !self.slice.is_empty() {
            return Err(ParseError::Invalid);
        }

        let sigtotal = match out.pop() {
            Some(slice) => bytes_to_u32(slice)?,
            None => return Err(ParseError::Invalid),
        };

        if sigtotal as usize == out.len() {
            if signeed as usize <= out.iter().filter(|pubkey| is_valid_pubkey(pubkey)).count() {
                out.shrink_to_fit();
                Ok(HighLevel::PayToMultisig(signeed, out))
            } else {
                Ok(HighLevel::Invalid)
            }
        } else {
            Err(ParseError::Invalid)
        }
    }
}

/// Parses a Bitcoin Script stack number (CScriptNum) as a little-endian signed-magnitude integer.
pub fn bytes_to_i32(slice: &[u8]) -> ParseResult<i32> {
    if slice.is_empty() {
        return Ok(0);
    }
    if slice.len() > 4 {
        return Err(ParseError::Invalid);
    }

    let mut res: u32 = 0;
    for (i, &b) in slice.iter().enumerate() {
        res |= (b as u32) << (8 * i);
    }

    let last_byte = *slice.last().unwrap();
    let is_negative = (last_byte & 0x80) != 0;

    if is_negative {
        let mask = 0x80u32 << (8 * (slice.len() - 1));
        let abs_val = res & !mask;
        Ok(-(abs_val as i32))
    } else {
        Ok(res as i32)
    }
}

pub fn bytes_to_u32(slice: &[u8]) -> ParseResult<u32> {
    let res = bytes_to_i32(slice)?;
    if res >= 0 {
        Ok(res as u32)
    } else {
        Err(ParseError::Invalid)
    }
}

pub fn is_valid_pubkey(pubkey: &[u8]) -> bool {
    if pubkey.is_empty() {
        return false;
    }

    matches!(
        (pubkey[0], pubkey.len()),
        (0x02, 33) | (0x03, 33) | (0x04, 65)
    )
}

pub fn bytes_to_bool(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        false
    } else if bytes[0] & 0x7f != 0 {
        true
    } else {
        bytes[1..].iter().any(|b| *b != 0)
    }
}
