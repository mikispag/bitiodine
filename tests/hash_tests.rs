use bitiodine::hash::{Hash, ZERO_HASH};
use bitiodine::hash160::Hash160;

#[test]
fn test_hash_display_and_from_pretty() {
    let genesis_hex = "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f";
    let hash = Hash::from_pretty(genesis_hex).expect("Failed to parse genesis hex");
    assert_eq!(hash.to_string(), genesis_hex);
    assert_ne!(hash, ZERO_HASH);
}

#[test]
fn test_hash_from_data_double_sha256() {
    // Double SHA256 of empty byte array:
    // SHA256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
    // SHA256(SHA256("")) = 5df6e0e2761359d30a8275058e299fcc0381534545f55cf43e41983f5d4c9456
    // Note: Hash::to_string() reverses bytes for standard Bitcoin display order.
    let hash = Hash::from_data(b"");
    let mut expected_bytes = [0u8; 32];
    hex::decode_to_slice(
        "5df6e0e2761359d30a8275058e299fcc0381534545f55cf43e41983f5d4c9456",
        &mut expected_bytes,
    )
    .unwrap();
    assert_eq!(hash.as_slice(), &expected_bytes);
}

#[test]
fn test_hash160_from_data() {
    // HASH160("") = RIPEMD160(SHA256(""))
    // SHA256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
    // RIPEMD160(SHA256("")) = b472a266d0bd89c13706a4132ccfb16f7c3b9fcb
    let hash160 = Hash160::from_data(b"");
    assert_eq!(
        hash160.to_string(),
        "b472a266d0bd89c13706a4132ccfb16f7c3b9fcb"
    );
}

#[test]
fn test_zero_hash() {
    assert_eq!(ZERO_HASH.as_slice(), &[0u8; 32]);
    assert_eq!(
        ZERO_HASH.to_string(),
        "0000000000000000000000000000000000000000000000000000000000000000"
    );
}
