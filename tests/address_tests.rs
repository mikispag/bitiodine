use bitiodine::address::{Address, CompactAddress};
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

#[test]
fn test_compact_address_size() {
    assert!(std::mem::size_of::<CompactAddress>() <= 42);
    assert_eq!(std::mem::align_of::<CompactAddress>(), 1);
}

fn hex_hash20(s: &str) -> [u8; 20] {
    let mut buf = [0u8; 20];
    hex::decode_to_slice(s, &mut buf).unwrap();
    buf
}

fn address_hash160(address: &Address) -> Hash160 {
    match address {
        Address::Base58 { hash, .. } => Hash160(*hash),
        Address::Witness { .. } => Hash160::default(),
    }
}

#[test]
fn test_compact_address_display_parity() {
    use bitcoin_bech32::constants::Network;
    use bitcoin_bech32::{u5, WitnessProgram};

    let wp20 = WitnessProgram::new(u5::try_from_u8(0).unwrap(), vec![0x75; 20], Network::Bitcoin)
        .unwrap();
    let wp32 = WitnessProgram::new(u5::try_from_u8(1).unwrap(), vec![0x51; 32], Network::Bitcoin)
        .unwrap();
    let wp_short = WitnessProgram::new(
        u5::try_from_u8(2).unwrap(),
        vec![0xAA, 0xBB],
        Network::Bitcoin,
    )
    .unwrap();

    let pairs = vec![
        (
            Address::from_hash160(&Hash160(hex_hash20("751e76e8199196d454941c45d1b3a323f1433bd6")), 0x00),
            CompactAddress::from_hash160(&Hash160(hex_hash20("751e76e8199196d454941c45d1b3a323f1433bd6")), 0x00),
        ),
        (
            Address::from_hash160(&Hash160(hex_hash20("8f55563b9a19f321c211e9b9f38ecd686daec030")), 0x05),
            CompactAddress::from_hash160(&Hash160(hex_hash20("8f55563b9a19f321c211e9b9f38ecd686daec030")), 0x05),
        ),
        (
            Address::from_witness_program(&wp20),
            CompactAddress::from_witness_program(&wp20),
        ),
        (
            Address::from_witness_program(&wp32),
            CompactAddress::from_witness_program(&wp32),
        ),
        (
            Address::from_witness_program(&wp_short),
            CompactAddress::from_witness_program(&wp_short),
        ),
    ];

    for (address, compact) in &pairs {
        assert_eq!(address.to_string(), compact.to_string());
        assert_eq!(address_hash160(address), compact.hash160());
    }
}

#[test]
fn test_compact_address_ordering_parity() {
    use bitcoin_bech32::constants::Network;
    use bitcoin_bech32::{u5, WitnessProgram};

    let wp_prefix = WitnessProgram::new(
        u5::try_from_u8(1).unwrap(),
        vec![0x01, 0x02],
        Network::Bitcoin,
    )
    .unwrap();
    let wp_prefix_long = WitnessProgram::new(
        u5::try_from_u8(1).unwrap(),
        vec![0x01, 0x02, 0x00, 0x05],
        Network::Bitcoin,
    )
    .unwrap();
    let wp_v1 = WitnessProgram::new(
        u5::try_from_u8(1).unwrap(),
        vec![0x01; 32],
        Network::Bitcoin,
    )
    .unwrap();

    let mut addresses = [
        Address::from_hash160(&Hash160([0x11; 20]), 0x00),
        Address::from_hash160(&Hash160([0x22; 20]), 0x00),
        Address::from_hash160(&Hash160([0x33; 20]), 0x05),
        Address::from_hash160(&Hash160([0x44; 20]), 0x05),
        Address::from_witness_program(&wp_prefix),
        Address::from_witness_program(&wp_prefix_long),
        Address::from_witness_program(&wp_v1),
    ];

    let mut compacts: Vec<CompactAddress> = addresses
        .iter()
        .map(|address| match address {
            Address::Base58 { version, hash } => {
                CompactAddress::from_hash160(&Hash160(*hash), *version)
            }
            Address::Witness { version, program } => {
                let wp = bitcoin_bech32::WitnessProgram::new(
                    bitcoin_bech32::u5::try_from_u8(*version).unwrap(),
                    program.to_vec(),
                    bitcoin_bech32::constants::Network::Bitcoin,
                )
                .unwrap();
                CompactAddress::from_witness_program(&wp)
            }
        })
        .collect();

    addresses.sort();
    compacts.sort();

    for (address, compact) in addresses.iter().zip(compacts.iter()) {
        assert_eq!(address.to_string(), compact.to_string());
    }
}
