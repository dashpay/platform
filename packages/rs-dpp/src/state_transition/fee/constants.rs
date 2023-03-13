use crate::{identity::KeyType, prelude::Fee};

pub const BASE_ST_PROCESSING_FEE: Fee = 10000; // 84000
pub const FEE_MULTIPLIER: Fee = 2;
pub const DEFAULT_USER_TIP: Fee = 0;
pub const STORAGE_CREDIT_PER_BYTE: Fee = 5000;
pub const PROCESSING_CREDIT_PER_BYTE: Fee = 12;
pub const DELETE_BASE_PROCESSING_COST: Fee = 2000; // 20000
pub const READ_BASE_PROCESSING_COST: Fee = 8400; // 8400
pub const WRITE_BASE_PROCESSING_COST: Fee = 6000; // 60000

pub const fn signature_verify_cost(key_type: KeyType) -> Fee {
    match key_type {
        KeyType::ECDSA_SECP256K1 => 3000,
        KeyType::BLS12_381 => 6000,
        KeyType::ECDSA_HASH160 => 3000,
        KeyType::BIP13_SCRIPT_HASH => 6000,
    }
}
