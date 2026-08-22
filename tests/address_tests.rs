use bitiodine::address::Address;
use bitiodine::hash160::Hash160;

#[test]
fn test_address_generation_p2pkh() {
    // Genesis block coinbase payout pubkey hash160 (Satoshi's Genesis address: 1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa)
    // Pubkey: 04678afdb0fe5548271967f1a67130b7105cd6a828e03909a67962e0ea1f61deb649f6bc3f4cef38c4f35504e51ec112de5c384df7ba0b8d578a4c702b6bf11d5f
    let mut pubkey = [0u8; 65];
    hex::decode_to_slice("04678afdb0fe5548271967f1a67130b7105cd6a828e03909a67962e0ea1f61deb649f6bc3f4cef38c4f35504e51ec112de5c384df7ba0b8d578a4c702b6bf11d5f", &mut pubkey).unwrap();

    let address = Address::from_pubkey(&pubkey, 0x00);
    assert_eq!(address.to_string(), "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa");
}

#[test]
fn test_address_from_hash160_p2sh() {
    // P2SH address starting with '3'
    let mut hash_bytes = [0u8; 20];
    hex::decode_to_slice("8f55563b9a19f321c211e9b9f38ecd686daec030", &mut hash_bytes).unwrap();
    let hash160 = Hash160::from_slice(&hash_bytes);
    let address = Address::from_hash160(hash160, 0x05);
    assert!(address.to_string().starts_with('3'));
}
