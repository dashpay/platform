use crate::identity::KeyType;

use super::Credits;

pub const BASE_ST_PROCESSING_FEE: Credits = 10000; // 84000
pub const FEE_MULTIPLIER: Credits = 2;
pub const DEFAULT_USER_TIP: Credits = 0;
pub const STORAGE_CREDIT_PER_BYTE: Credits = 5000;
pub const PROCESSING_CREDIT_PER_BYTE: Credits = 12;
pub const DELETE_BASE_PROCESSING_COST: Credits = 2000; // 20000
pub const READ_BASE_PROCESSING_COST: Credits = 8400; // 8400
pub const WRITE_BASE_PROCESSING_COST: Credits = 6000; // 60000

pub const fn signature_verify_cost(key_type: KeyType) -> Credits {
    match key_type {
        KeyType::ECDSA_SECP256K1 => 3000,
        KeyType::BLS12_381 => 6000,
        KeyType::ECDSA_HASH160 => 3000,
        KeyType::BIP13_SCRIPT_HASH => 6000,
    }
}
