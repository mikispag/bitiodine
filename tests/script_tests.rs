use bitiodine_rust::script::{
    bytes_to_bool, bytes_to_i32, bytes_to_u32, is_valid_pubkey, HighLevel, Script,
};

#[test]
fn test_script_p2pkh_recognition() {
    // Standard P2PKH script: OP_DUP OP_HASH160 <20 bytes> OP_EQUALVERIFY OP_CHECKSIG
    // 76 a9 14 <20 bytes> 88 ac
    let mut script_bytes = vec![0x76, 0xa9, 0x14];
    script_bytes.extend_from_slice(&[0xaa; 20]);
    script_bytes.extend_from_slice(&[0x88, 0xac]);

    let script = Script::new(&script_bytes, 1600000000, 500000);
    match script.to_highlevel() {
        HighLevel::PayToPubkeyHash(pkh) => {
            assert_eq!(pkh, &[0xaa; 20]);
        }
        other => panic!("Expected PayToPubkeyHash, got {:?}", other),
    }
}

#[test]
fn test_script_op_return_recognition() {
    // OP_RETURN <len> <data>
    let script_bytes = [0x6a, 0x04, 0xde, 0xad, 0xbe, 0xef];
    let script = Script::new(&script_bytes, 1600000000, 500000);
    match script.to_highlevel() {
        HighLevel::DataOutput(data) => {
            assert_eq!(data, &[0xde, 0xad, 0xbe, 0xef]);
        }
        other => panic!("Expected DataOutput, got {:?}", other),
    }
}

#[test]
fn test_pubkey_validation() {
    assert!(is_valid_pubkey(&[0x02; 33]));
    assert!(is_valid_pubkey(&[0x03; 33]));
    assert!(is_valid_pubkey(&[0x04; 65]));
    assert!(!is_valid_pubkey(&[0x05; 33]));
    assert!(!is_valid_pubkey(&[]));
}

#[test]
fn test_bytes_to_bool_and_int() {
    assert!(!bytes_to_bool(&[]));
    assert!(bytes_to_bool(&[0x01]));
    assert!(!bytes_to_bool(&[0x00]));

    assert_eq!(bytes_to_i32(&[]).unwrap(), 0);
    assert_eq!(bytes_to_i32(&[0x01]).unwrap(), 1);
    assert_eq!(bytes_to_u32(&[0x01]).unwrap(), 1);
}
