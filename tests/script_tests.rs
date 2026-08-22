use bitiodine_rust::bytecode::Bytecode;
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
fn test_script_taproot_recognition() {
    // Taproot (SegWit v1 / BIP-341) script: OP_1 <32 bytes> (51 20 <32 bytes>)
    let mut script_bytes = vec![0x51, 0x20];
    script_bytes.extend_from_slice(&[0xbb; 32]);

    let script = Script::new(&script_bytes, 1650000000, 750000);
    match script.to_highlevel() {
        HighLevel::PayToWitnessTaproot(w) => {
            let address = w.to_address();
            assert!(
                address.starts_with("bc1p"),
                "Expected bech32m Taproot address starting with bc1p, got {}",
                address
            );
        }
        other => panic!("Expected PayToWitnessTaproot, got {:?}", other),
    }
}

#[test]
fn test_script_general_witness_recognition() {
    // General SegWit (e.g. version 2 with 4 bytes payload: OP_2 04 <4 bytes>)
    let script_bytes = [0x52, 0x04, 0x01, 0x02, 0x03, 0x04];
    let script = Script::new(&script_bytes, 1650000000, 750000);
    match script.to_highlevel() {
        HighLevel::PayToWitnessGeneral(w) => {
            assert_eq!(w.version().to_u8(), 2);
        }
        other => panic!("Expected PayToWitnessGeneral, got {:?}", other),
    }
}

#[test]
fn test_cltv_and_csv_opcodes() {
    let mut slice_cltv = &[0xb1u8][..];
    assert_eq!(
        Bytecode::read(&mut slice_cltv, 400000).unwrap(),
        Bytecode::OP_CHECKLOCKTIMEVERIFY
    );

    let mut slice_csv = &[0xb2u8][..];
    assert_eq!(
        Bytecode::read(&mut slice_csv, 450000).unwrap(),
        Bytecode::OP_CHECKSEQUENCEVERIFY
    );
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
fn test_cscriptnum_little_endian_decoding() {
    assert!(!bytes_to_bool(&[]));
    assert!(bytes_to_bool(&[0x01]));
    assert!(!bytes_to_bool(&[0x00]));

    assert_eq!(bytes_to_i32(&[]).unwrap(), 0);
    assert_eq!(bytes_to_i32(&[0x01]).unwrap(), 1);
    assert_eq!(bytes_to_i32(&[0x81]).unwrap(), -1);
    assert_eq!(bytes_to_i32(&[0x00, 0x01]).unwrap(), 256);
    assert_eq!(bytes_to_i32(&[0x00, 0x81]).unwrap(), -256);
    assert_eq!(bytes_to_i32(&[0x10, 0x27]).unwrap(), 10000); // 0x2710 in little-endian = 10000

    assert_eq!(bytes_to_u32(&[0x01]).unwrap(), 1);
    assert_eq!(bytes_to_u32(&[0x10, 0x27]).unwrap(), 10000);
    assert!(bytes_to_u32(&[0x81]).is_err()); // Negative cannot convert to u32
}
