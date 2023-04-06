use crate::credits::Credits;
use crate::identity::KeyType;

// TODO: Do we actually use it?
pub const BASE_ST_FEE: Credits = 10000; // 84000
pub const FEE_MULTIPLIER: Credits = 2;
pub const DEFAULT_USER_TIP: Credits = 0;
pub const PROCESSING_CREDIT_PER_BYTE: Credits = 12;
pub const READ_BASE_PROCESSING_COST: Credits = 8400; // 8400

// TODO: Should be moved to usage
pub const fn signature_verify_cost(key_type: KeyType) -> Credits {
    match key_type {
        KeyType::ECDSA_SECP256K1 => 3000,
        KeyType::BLS12_381 => 6000,
        KeyType::ECDSA_HASH160 => 3000,
        KeyType::BIP13_SCRIPT_HASH => 6000,
    }
}
