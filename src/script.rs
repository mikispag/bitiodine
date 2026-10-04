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
    /// Complete OP_RETURN script containing multiple pushes, preserving their encoding.
    DataOutputScript(&'a [u8]),
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
        // Assign destinations only from complete standard scripts. Prefix removal
        // belongs to the heuristic fallback and can change spending conditions.
        let script_slice = self.slice;

        // Check for SegWit v0-v16 Witness Programs (BIP-141 / BIP-341 / BIP-350)
        // Format: OP_n <push len 2..40> <data>
        if self.height >= 481_824 && script_slice.len() >= 4 && script_slice.len() <= 42 {
            let first = script_slice[0];
            let push_len = script_slice[1] as usize;
            if (first == 0x00 || (0x51..=0x60).contains(&first))
                && script_slice.len() == 2 + push_len
            {
                if let Ok(w) = WitnessProgram::from_scriptpubkey(script_slice, Network::Bitcoin) {
                    match (first, push_len) {
                        (0x00, 20) => return HighLevel::PayToWitnessPubkeyHash(w),
                        (0x00, 32) => return HighLevel::PayToWitnessScriptHash(w),
                        (0x51, 32) => return HighLevel::PayToWitnessTaproot(w),
                        _ => return HighLevel::PayToWitnessGeneral(w),
                    }
                }
            }
        }

        match script_slice.len() {
            0 => {
                return HighLevel::Donation;
            }
            23 => {
                if script_slice[..2] == [0xa6, 0x14] && script_slice[22] == 0x87 {
                    let hash_bytes: &'a [u8; 20] = script_slice[2..22].try_into().unwrap();
                    return HighLevel::Challenge(ChallengeType::Ripemd160(hash_bytes));
                }
                if script_slice[..2] == [0xa7, 0x14] && script_slice[22] == 0x87 {
                    let hash_bytes: &'a [u8; 20] = script_slice[2..22].try_into().unwrap();
                    return HighLevel::Challenge(ChallengeType::Sha1(hash_bytes));
                }
                if script_slice[..2] == [0xa9, 0x14] && script_slice[22] == 0x87 {
                    let hash_bytes: &'a [u8; 20] = script_slice[2..22].try_into().unwrap();
                    if self.timestamp >= 1333238400 {
                        return HighLevel::PayToScriptHash(hash_bytes);
                    }
                    return HighLevel::Challenge(ChallengeType::Hash160(hash_bytes));
                }
            }
            25 => {
                if script_slice[..3] == [0x76, 0xa9, 0x14] && script_slice[23..] == [0x88, 0xac] {
                    let pkh: &'a [u8; 20] = script_slice[3..23].try_into().unwrap();
                    return HighLevel::PayToPubkeyHash(pkh);
                }
            }
            35 => {
                if script_slice[0] == 33 && script_slice[34] == 0xac {
                    let pubkey = &script_slice[1..1 + 33];
                    if is_valid_pubkey(pubkey) {
                        return HighLevel::PayToPubkey(pubkey);
                    } else {
                        return HighLevel::Invalid;
                    }
                }
                if script_slice[..2] == [0xa8, 0x20] && script_slice[34] == 0x87 {
                    let hash_bytes: &'a [u8; 32] = script_slice[2..34].try_into().unwrap();
                    return HighLevel::Challenge(ChallengeType::Sha256(hash_bytes));
                }
                if script_slice[..2] == [0xaa, 0x20] && script_slice[34] == 0x87 {
                    let hash_bytes: &'a [u8; 32] = script_slice[2..34].try_into().unwrap();
                    return HighLevel::Challenge(ChallengeType::Hash256(hash_bytes));
                }
            }
            67 if script_slice[0] == 65 && script_slice[66] == 0xac => {
                let pubkey = &script_slice[1..1 + 65];
                if is_valid_pubkey(pubkey) {
                    return HighLevel::PayToPubkey(pubkey);
                } else {
                    return HighLevel::Invalid;
                }
            }
            _ => {}
        }

        if let Ok(res) = self.iter().read_pay_to_multisig() {
            return res;
        }

        // Preserve the complete script for multiple pushes; concatenating their
        // payloads loses boundaries and protocol markers such as OP_13.
        if script_slice.first() == Some(&0x6a) {
            let mut payload = &script_slice[1..];
            let mut single_payload = &[][..];
            let mut pushes = 0;
            while !payload.is_empty() {
                // Bytecode::read skips NOPs. Require a literal push opcode here
                // so an arbitrary script suffix is not mistaken for data.
                if !(payload[0] <= 0x4f || (0x51..=0x60).contains(&payload[0])) {
                    return HighLevel::Invalid;
                }
                match Bytecode::read(&mut payload, self.height) {
                    Ok(OP_PUSH(data)) => {
                        single_payload = data;
                        pushes += 1;
                    }
                    _ => return HighLevel::Invalid,
                }
            }
            return if pushes <= 1 {
                HighLevel::DataOutput(single_payload)
            } else {
                HighLevel::DataOutputScript(script_slice)
            };
        }

        let mut skipped_iter = self.iter();
        skipped_iter.skip_nops();
        let skipped_slice = skipped_iter.slice;

        if skipped_slice.is_empty() {
            return HighLevel::Donation;
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
                            return if is_valid_pubkey(bytes) {
                                HighLevel::Unknown(*self)
                            } else {
                                HighLevel::Invalid
                            };
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
                                        return if bytes.len() == 20 {
                                            HighLevel::Unknown(*self)
                                        } else {
                                            HighLevel::Invalid
                                        };
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
                    Err(_) => return HighLevel::Invalid,
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

    pub fn read_pay_to_multisig(mut self) -> ParseResult<HighLevel<'a>> {
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

    match (pubkey[0], pubkey.len()) {
        (0x02, 33) | (0x03, 33) | (0x04, 65) => true,
        // Historical hybrid encodings remain valid under legacy consensus.
        // libsecp256k1 checks the curve point and the prefix's Y parity bit.
        (0x06 | 0x07, 65) => bitcoin::secp256k1::PublicKey::from_slice(pubkey).is_ok(),
        _ => false,
    }
}

pub fn bytes_to_bool(bytes: &[u8]) -> bool {
    match bytes.split_last() {
        Some((&last, rest)) => last & 0x7f != 0 || rest.iter().any(|b| *b != 0),
        None => false,
    }
}
