use bitiodine::bytecode::Bytecode;
use bitiodine::script::{
    bytes_to_bool, bytes_to_i32, bytes_to_u32, is_valid_pubkey, ChallengeType, HighLevel, Script,
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

#[test]
fn test_script_p2sh_recognition_at_activation() {
    let mut bytes = vec![0xa9, 0x14];
    bytes.extend_from_slice(&[0xaa; 20]);
    bytes.push(0x87);
    assert_eq!(
        Script::new(&bytes, 1333238400, 173805).to_highlevel(),
        HighLevel::PayToScriptHash(&[0xaa; 20])
    );
    assert_eq!(
        Script::new(&bytes, 1333238399, 173804).to_highlevel(),
        HighLevel::Challenge(ChallengeType::Hash160(&[0xaa; 20]))
    );
}

fn is_address_output(output: HighLevel<'_>) -> bool {
    matches!(
        output,
        HighLevel::PayToPubkey(_)
            | HighLevel::PayToPubkeyHash(_)
            | HighLevel::PayToScriptHash(_)
            | HighLevel::PayToWitnessPubkeyHash(_)
            | HighLevel::PayToWitnessScriptHash(_)
            | HighLevel::PayToWitnessTaproot(_)
            | HighLevel::PayToWitnessGeneral(_)
    )
}

#[test]
fn test_address_recognition_requires_the_complete_script() {
    let mut p2sh = vec![0xa9, 0x14];
    p2sh.extend_from_slice(&[0xaa; 20]);
    p2sh.push(0x87);
    let mut p2pkh = vec![0x76, 0xa9, 0x14];
    p2pkh.extend_from_slice(&[0xaa; 20]);
    p2pkh.extend_from_slice(&[0x88, 0xac]);
    let mut p2pk = vec![0x21];
    p2pk.extend_from_slice(&[0x02; 33]);
    p2pk.push(0xac);
    let mut witness = vec![0, 0x14];
    witness.extend_from_slice(&[0xaa; 20]);
    for standard in [p2sh, p2pkh, p2pk, witness] {
        assert!(is_address_output(
            Script::new(&standard, 1700000000, 800000).to_highlevel()
        ));
        for prefix in [&[0x75][..], &[0x61], &[0x51, 0x75]] {
            let mut prefixed = prefix.to_vec();
            prefixed.extend_from_slice(&standard);
            assert!(
                !is_address_output(Script::new(&prefixed, 1700000000, 800000).to_highlevel()),
                "prefixed script {} must not receive a standard destination",
                hex::encode(prefixed)
            );
        }
        let mut suffixed = standard;
        suffixed.push(0x61);
        assert!(!is_address_output(
            Script::new(&suffixed, 1700000000, 800000).to_highlevel()
        ));
    }
}

#[test]
fn test_failed_multisig_probe_preserves_the_script_cursor() {
    let mut bytes = vec![0x69, 0x21];
    bytes.extend_from_slice(
        &hex::decode("0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798").unwrap(),
    );
    bytes.push(0xac);
    let script = Script::new(&bytes, 1700000000, 800000);
    assert_eq!(script.to_highlevel(), HighLevel::Unknown(script));
    assert_eq!(
        Script::new(&[0x6a, 0x51], 1700000000, 800000).to_highlevel(),
        HighLevel::DataOutput(&[1])
    );
}

#[test]
fn test_nonminimal_signature_scripts_do_not_panic_or_invent_destinations() {
    let mut p2pk = vec![0x4c, 33];
    p2pk.extend_from_slice(&[0x02; 33]);
    p2pk.push(0xac);
    let mut p2pkh = vec![0x76, 0xa9, 0x4c, 20];
    p2pkh.extend_from_slice(&[0xaa; 20]);
    p2pkh.extend_from_slice(&[0x88, 0xac]);
    for bytes in [p2pk, p2pkh] {
        let script = Script::new(&bytes, 1700000000, 800000);
        assert_eq!(script.to_highlevel(), HighLevel::Unknown(script));
    }
    let mut malformed_hash = vec![0x76, 0xa9, 33];
    malformed_hash.extend_from_slice(&[0x02; 33]);
    malformed_hash.extend_from_slice(&[0x88, 0xac]);
    assert_eq!(
        Script::new(&malformed_hash, 1700000000, 800000).to_highlevel(),
        HighLevel::Invalid
    );
}

#[test]
fn test_op_return_pushdata_payloads() {
    for (prefix, size) in [
        (vec![0x6a, 0x4c, 80], 80),
        (vec![0x6a, 0x4d, 0, 1], 256),
        (vec![0x6a, 0x4e, 4, 0, 0, 0], 4),
    ] {
        let payload = vec![0x61; size];
        let mut bytes = prefix;
        bytes.extend_from_slice(&payload);
        assert_eq!(
            Script::new(&bytes, 1700000000, 800000).to_highlevel(),
            HighLevel::DataOutput(&payload)
        );
        bytes.pop();
        assert_eq!(
            Script::new(&bytes, 1700000000, 800000).to_highlevel(),
            HighLevel::Invalid
        );
    }
    for bytes in [&[0x6a][..], &[0x6a, 0], &[0x6a, 0x4c, 0]] {
        assert_eq!(
            Script::new(bytes, 1700000000, 800000).to_highlevel(),
            HighLevel::DataOutput(&[])
        );
    }
}

#[test]
fn test_script_truth_values_use_the_final_sign_bit() {
    for bytes in [&[][..], &[0], &[0x80], &[0, 0x80], &[0, 0, 0x80]] {
        assert!(!bytes_to_bool(bytes), "{} is false", hex::encode(bytes));
    }
    for bytes in [&[1][..], &[0x81], &[0x80, 0], &[0x80, 0x80], &[0, 1]] {
        assert!(bytes_to_bool(bytes), "{} is true", hex::encode(bytes));
    }
    assert_eq!(
        Script::new(&[2, 0x80, 0], 1700000000, 800000).to_highlevel(),
        HighLevel::Donation
    );
    assert_eq!(
        Script::new(&[2, 0, 0x80], 1700000000, 800000).to_highlevel(),
        HighLevel::Invalid
    );
}
