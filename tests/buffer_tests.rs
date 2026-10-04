use bitiodine::buffer_operations::{read_array, read_slice, read_u16, read_u8, read_var_int};
use bitiodine::error::ParseError;

#[test]
fn test_read_u8_u16_u32_u64() {
    let data = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0];
    let mut slice = &data[..];

    assert_eq!(read_u8(&mut slice).unwrap(), 0x12);
    assert_eq!(read_u16(&mut slice).unwrap(), 0x5634); // little-endian
    assert_eq!(read_slice(&mut slice, 1).unwrap(), &[0x78]);
    assert_eq!(slice, &[0x9a, 0xbc, 0xde, 0xf0]);
}

#[test]
fn test_read_array() {
    let data = [1, 2, 3, 4];
    let mut slice = &data[..];
    let arr: &[u8; 4] = read_array(&mut slice).unwrap();
    assert_eq!(arr, &[1, 2, 3, 4]);
    assert!(slice.is_empty());
}

#[test]
fn test_read_var_int() {
    // 1-byte VarInt (< 0xfd)
    let mut slice = &[0x42u8][..];
    assert_eq!(read_var_int(&mut slice).unwrap(), 0x42);

    // 2-byte VarInt (0xfd prefix)
    let mut slice = &[0xfd, 0x01, 0x02][..];
    assert_eq!(read_var_int(&mut slice).unwrap(), 0x0201);

    // 4-byte VarInt (0xfe prefix)
    let mut slice = &[0xfe, 0x01, 0x02, 0x03, 0x04][..];
    assert_eq!(read_var_int(&mut slice).unwrap(), 0x04030201);

    // 8-byte VarInt (0xff prefix)
    let mut slice = &[0xff, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08][..];
    assert_eq!(read_var_int(&mut slice).unwrap(), 0x0807060504030201);
}

#[test]
fn compact_size_boundaries_and_every_truncation() {
    for value in [
        0,
        0xfc,
        0xfd,
        0xffff,
        0x10000,
        0xffffffff,
        0x100000000,
        u64::MAX,
    ] {
        let encoded = bitcoin::consensus::serialize(&bitcoin::consensus::encode::VarInt(value));
        for end in 0..encoded.len() {
            assert!(
                matches!(read_var_int(&mut &encoded[..end]), Err(ParseError::Eof)),
                "accepted truncation at {end} for {value}"
            );
        }
        let mut with_sentinel = encoded;
        with_sentinel.push(0x42);
        let mut slice = with_sentinel.as_slice();
        assert_eq!(read_var_int(&mut slice).unwrap(), value);
        assert_eq!(slice, &[0x42]);
    }
}

#[test]
fn nonminimal_compact_sizes_are_invalid() {
    for encoded in [
        &[0xfd, 0, 0][..],
        &[0xfd, 0xfc, 0],
        &[0xfe, 0, 0, 0, 0],
        &[0xfe, 0xff, 0xff, 0, 0],
        &[0xff, 0, 0, 0, 0, 0, 0, 0, 0],
        &[0xff, 0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0],
    ] {
        assert!(matches!(
            read_var_int(&mut &encoded[..]),
            Err(ParseError::Invalid)
        ));
    }
}

#[test]
fn oversized_reads_fail_without_panicking() {
    let mut slice = &[1, 2, 3][..];
    assert!(read_slice(&mut slice, usize::MAX).is_err());
    assert!(slice.is_empty());
    assert_eq!(read_slice(&mut slice, 0).unwrap(), &[]);
    assert!(read_array::<4>(&mut &[1, 2, 3][..]).is_err());
}

#[test]
fn test_blockchain_xor_handling() {
    use bitiodine::BlockChain;
    use std::fs::{self, File};
    use std::io::Write;

    let temp_dir = std::env::temp_dir().join("bitiodine_test_xor");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let xor_key: [u8; 8] = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0];
    fs::write(temp_dir.join("xor.dat"), xor_key).unwrap();

    let magic = [0xf9, 0xbe, 0xb4, 0xd9];
    let len = (80u32).to_le_bytes();
    let mut raw_data = Vec::new();
    raw_data.extend_from_slice(&magic);
    raw_data.extend_from_slice(&len);
    raw_data.extend_from_slice(&[0u8; 80]); // 80 bytes header

    let mut obfuscated = raw_data.clone();
    for (i, b) in obfuscated.iter_mut().enumerate() {
        *b ^= xor_key[i % 8];
    }

    let mut file = File::create(temp_dir.join("blk00000.dat")).unwrap();
    file.write_all(&obfuscated).unwrap();
    drop(file);

    let chain = BlockChain::read(&temp_dir, 0).unwrap();
    assert_eq!(chain.len(), 1);

    let _ = fs::remove_dir_all(&temp_dir);
}
