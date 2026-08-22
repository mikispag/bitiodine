use bitiodine::buffer_operations::{read_array, read_slice, read_u16, read_u8, read_var_int};

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
