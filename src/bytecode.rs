use preamble::*;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[allow(non_camel_case_types)]
pub enum Bytecode<'a> {
    // 0x00-0x4f, 0x51-0x60
    OP_PUSH(&'a [u8]),

    // 0x50, 0x89, 0x8a, 0xba-0xff
    OP_INVALID,

    // 0x61, 0xb0, 0xb2-0xb9
    // Skipped when reading; not actually emitted
    OP_NOP,

    // 0x62
    OP_VER,

    // 0x63
    OP_IF,

    // 0x64
    OP_NOTIF,

    // replaced with ParseError::Invalid
    // 0x65
    // OP_VERIF,
    //
    // 0x66
    // OP_VERNOTIF,
    //
    // 0x67
    OP_ELSE,

    // 0x68
    OP_ENDIF,

    // 0x69
    OP_VERIFY,

    // 0x6a
    OP_RETURN,

    // 0x6b
    OP_TOALTSTACK,

    // 0x6c
    OP_FROMALTSTACK,

    // 0x6d
    OP_2DROP,

    // 0x6e
    OP_2DUP,

    // 0x6f
    OP_3DUP,

    // 0x70
    OP_2OVER,

    // 0x71
    OP_2ROT,

    // 0x72
    OP_2SWAP,

    // 0x73
    OP_IFDUP,

    // 0x74
    OP_DEPTH,

    // 0x75
    OP_DROP,

    // 0x76
    OP_DUP,

    // 0x77
    OP_NIP,

    // 0x78
    OP_OVER,

    // 0x79
    OP_PICK,

    // 0x7a
    OP_ROLL,

    // 0x7b
    OP_ROT,

    // 0x7c
    OP_SWAP,

    // 0x7d
    OP_TUCK,

    // replaced with ParseError::Invalid
    // 0x7e
    // OP_CAT,
    //
    // 0x7f
    // OP_SUBSTR,
    //
    // 0x80
    // OP_LEFT,
    //
    // 0x81
    // OP_RIGHT,
    //
    // 0x82
    OP_SIZE,

    // replaced with ParseError::Invalid
    // 0x83
    // OP_INVERT,
    //
    // 0x84
    // OP_AND,
    //
    // 0x85
    // OP_OR,
    //
    // 0x86
    // OP_XOR,
    //
    // 0x87
    OP_EQUAL,

    // 0x88
    OP_EQUALVERIFY,

    // 0x8b
    OP_1ADD,

    // 0x8c
    OP_1SUB,

    // replaced with ParseError::Invalid
    // 0x8d
    // OP_2MUL,
    //
    // 0x8e
    // OP_2DIV,
    //
    // 0x8f
    OP_NEGATE,

    // 0x90
    OP_ABS,

    // 0x91
    OP_NOT,

    // 0x92
    OP_0NOTEQUAL,

    // 0x93
    OP_ADD,

    // 0x94
    OP_SUB,

    // replaced with ParseError::Invalid
    // 0x95
    // OP_MUL,
    //
    // 0x96
    // OP_DIV,
    //
    // 0x97
    // OP_MOD,
    //
    // 0x98
    // OP_LSHIFT,
    //
    // 0x99
    // OP_RSHIFT,
    //
    // 0x9a
    OP_BOOLAND,

    // 0x9b
    OP_BOOLOR,

    // 0x9c
    OP_NUMEQUAL,

    // 0x9d
    OP_NUMEQUALVERIFY,

    // 0x9e
    OP_NUMNOTEQUAL,

    // 0x9f
    OP_LESSTHAN,

    // 0xa0
    OP_GREATERTHAN,

    // 0xa1
    OP_LESSTHANOREQUAL,

    // 0xa2
    OP_GREATERTHANOREQUAL,

    // 0xa3
    OP_MIN,

    // 0xa4
    OP_MAX,

    // 0xa5
    OP_WITHIN,

    // 0xa6
    OP_RIPEMD160,

    // 0xa7
    OP_SHA1,

    // 0xa8
    OP_SHA256,

    // 0xa9
    OP_HASH160,

    // 0xaa
    OP_HASH256,

    // 0xab
    OP_CODESEPARATOR,

    // 0xac
    OP_CHECKSIG,

    // 0xad
    OP_CHECKSIGVERIFY,

    // 0xae
    OP_CHECKMULTISIG,

    // 0xaf
    OP_CHECKMULTISIGVERIFY,

    // 0xb1
    OP_CHECKLOCKTIMEVERIFY,
}

pub use self::Bytecode::*;

impl<'a> Bytecode<'a> {
    fn read_raw(slice: &mut &'a [u8], height: u64) -> ParseResult<Bytecode<'a>> {
        macro_rules! make_static {
            ($val:expr) => {{
                static VAL: [u8; 1] = [$val];
                &VAL[..1]
            }};
        };

        match read_u8(slice)? {
            0 => Ok(OP_PUSH(&[])),
            len @ 0x01...0x4b => {
                let len = len as usize;
                let slice = read_slice(slice, len).map_err(|_| ParseError::Invalid)?;
                Ok(OP_PUSH(slice))
            }
            0x4c => {
                let len = read_u8(slice).map_err(|_| ParseError::Invalid)? as usize;
                let slice = read_slice(slice, len).map_err(|_| ParseError::Invalid)?;
                Ok(OP_PUSH(slice))
            }
            0x4d => {
                let len = read_u16(slice).map_err(|_| ParseError::Invalid)? as usize;
                let slice = read_slice(slice, len).map_err(|_| ParseError::Invalid)?;
                Ok(OP_PUSH(slice))
            }
            0x4e => {
                let len = read_u32(slice).map_err(|_| ParseError::Invalid)? as usize;
                let slice = read_slice(slice, len).map_err(|_| ParseError::Invalid)?;
                Ok(OP_PUSH(slice))
            }
            0x4f => Ok(OP_PUSH(make_static!(0x81))),
            0x50 => Ok(OP_INVALID),
            0x51 => Ok(OP_PUSH(make_static!(0x01))),
            0x52 => Ok(OP_PUSH(make_static!(0x02))),
            0x53 => Ok(OP_PUSH(make_static!(0x03))),
            0x54 => Ok(OP_PUSH(make_static!(0x04))),
            0x55 => Ok(OP_PUSH(make_static!(0x05))),
            0x56 => Ok(OP_PUSH(make_static!(0x06))),
            0x57 => Ok(OP_PUSH(make_static!(0x07))),
            0x58 => Ok(OP_PUSH(make_static!(0x08))),
            0x59 => Ok(OP_PUSH(make_static!(0x09))),
            0x5a => Ok(OP_PUSH(make_static!(0x0a))),
            0x5b => Ok(OP_PUSH(make_static!(0x0b))),
            0x5c => Ok(OP_PUSH(make_static!(0x0c))),
            0x5d => Ok(OP_PUSH(make_static!(0x0d))),
            0x5e => Ok(OP_PUSH(make_static!(0x0e))),
            0x5f => Ok(OP_PUSH(make_static!(0x0f))),
            0x60 => Ok(OP_PUSH(make_static!(0x10))),
            0x61 => Ok(OP_NOP),
            0x62 => Ok(OP_VER),
            0x63 => Ok(OP_IF),
            0x64 => Ok(OP_NOTIF),
            0x65 => Err(ParseError::Invalid), // Simplified from OP_VERIF
            0x66 => Err(ParseError::Invalid), // Simplified from OP_VERNOTIF
            0x67 => Ok(OP_ELSE),
            0x68 => Ok(OP_ENDIF),
            0x69 => Ok(OP_VERIFY),
            0x6a => Ok(OP_RETURN),
            0x6b => Ok(OP_TOALTSTACK),
            0x6c => Ok(OP_FROMALTSTACK),
            0x6d => Ok(OP_2DROP),
            0x6e => Ok(OP_2DUP),
            0x6f => Ok(OP_3DUP),
            0x70 => Ok(OP_2OVER),
            0x71 => Ok(OP_2ROT),
            0x72 => Ok(OP_2SWAP),
            0x73 => Ok(OP_IFDUP),
            0x74 => Ok(OP_DEPTH),
            0x75 => Ok(OP_DROP),
            0x76 => Ok(OP_DUP),
            0x77 => Ok(OP_NIP),
            0x78 => Ok(OP_OVER),
            0x79 => Ok(OP_PICK),
            0x7a => Ok(OP_ROLL),
            0x7b => Ok(OP_ROT),
            0x7c => Ok(OP_SWAP),
            0x7d => Ok(OP_TUCK),
            0x7e => Err(ParseError::Invalid), // Simplified from OP_CAT
            0x7f => Err(ParseError::Invalid), // Simplified from OP_SUBSTR
            0x80 => Err(ParseError::Invalid), // Simplified from OP_LEFT
            0x81 => Err(ParseError::Invalid), // Simplified from OP_RIGHT
            0x82 => Ok(OP_SIZE),
            0x83 => Err(ParseError::Invalid), // Simplified from OP_INVERT
            0x84 => Err(ParseError::Invalid), // Simplified from OP_AND
            0x85 => Err(ParseError::Invalid), // Simplified from OP_OR
            0x86 => Err(ParseError::Invalid), // Simplified from OP_XOR
            0x87 => Ok(OP_EQUAL),
            0x88 => Ok(OP_EQUALVERIFY),
            0x89 => Ok(OP_INVALID),
            0x8a => Ok(OP_INVALID),
            0x8b => Ok(OP_1ADD),
            0x8c => Ok(OP_1SUB),
            0x8d => Err(ParseError::Invalid), // Simplified from OP_2MUL
            0x8e => Err(ParseError::Invalid), // Simplified from OP_2DIV
            0x8f => Ok(OP_NEGATE),
            0x90 => Ok(OP_ABS),
            0x91 => Ok(OP_NOT),
            0x92 => Ok(OP_0NOTEQUAL),
            0x93 => Ok(OP_ADD),
            0x94 => Ok(OP_SUB),
            0x95 => Err(ParseError::Invalid), // Simplified from OP_MUL
            0x96 => Err(ParseError::Invalid), // Simplified from OP_DIV
            0x97 => Err(ParseError::Invalid), // Simplified from OP_MOD
            0x98 => Err(ParseError::Invalid), // Simplified from OP_LSHIFT
            0x99 => Err(ParseError::Invalid), // Simplified from OP_RSHIFT
            0x9a => Ok(OP_BOOLAND),
            0x9b => Ok(OP_BOOLOR),
            0x9c => Ok(OP_NUMEQUAL),
            0x9d => Ok(OP_NUMEQUALVERIFY),
            0x9e => Ok(OP_NUMNOTEQUAL),
            0x9f => Ok(OP_LESSTHAN),
            0xa0 => Ok(OP_GREATERTHAN),
            0xa1 => Ok(OP_LESSTHANOREQUAL),
            0xa2 => Ok(OP_GREATERTHANOREQUAL),
            0xa3 => Ok(OP_MIN),
            0xa4 => Ok(OP_MAX),
            0xa5 => Ok(OP_WITHIN),
            0xa6 => Ok(OP_RIPEMD160),
            0xa7 => Ok(OP_SHA1),
            0xa8 => Ok(OP_SHA256),
            0xa9 => Ok(OP_HASH160),
            0xaa => Ok(OP_HASH256),
            0xab => Ok(OP_CODESEPARATOR),
            0xac => Ok(OP_CHECKSIG),
            0xad => Ok(OP_CHECKSIGVERIFY),
            0xae => Ok(OP_CHECKMULTISIG),
            0xaf => Ok(OP_CHECKMULTISIGVERIFY),
            0xb0 => Ok(OP_NOP),
            0xb1 => {
                if height >= 388381 {
                    Ok(OP_CHECKLOCKTIMEVERIFY)
                } else {
                    Ok(OP_NOP)
                }
            }
            0xb2...0xb9 => Ok(OP_NOP),
            0xba...0xff => Ok(OP_INVALID),
            _ => panic!("Unreachable code branch."),
        }
    }

    pub fn read(slice: &mut &'a [u8], height: u64) -> ParseResult<Bytecode<'a>> {
        loop {
            match Bytecode::read_raw(slice, height) {
                Ok(OP_NOP) => continue,
                res => return res,
            }
        }
    }
}
