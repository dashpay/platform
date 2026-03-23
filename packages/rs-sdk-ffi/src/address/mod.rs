//! Address operations (queries and state transitions)

pub mod queries;
pub mod transitions;

// Re-export query functions
pub use queries::*;

// Re-export transition functions
pub use transitions::*;

use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};
use dash_sdk::dpp::address_funds::PlatformAddress;
use dash_sdk::dpp::dashcore::blockdata::script::Script;
use dash_sdk::drive::grovedb::operations::proof::GroveDBProof;
use std::ffi::CString;
use std::panic::{self, AssertUnwindSafe};

/// Encode a P2PKH script pubkey as a bech32m platform address string (DIP-18).
///
/// Takes a 25-byte P2PKH scriptPubKey (OP_DUP OP_HASH160 <20-byte-hash> OP_EQUALVERIFY OP_CHECKSIG)
/// and returns the bech32m-encoded platform address (e.g. "tdash1..." on testnet).
///
/// # Safety
/// - `script_pubkey` must point to `script_len` bytes
/// - `network`: 0 = mainnet, 1 = testnet, 2 = regtest, 3 = devnet
/// - Returns a DashSDKResult whose `data` field is a heap-allocated C string.
///   Free it with `dash_sdk_string_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_encode_platform_address(
    script_pubkey: *const u8,
    script_len: u32,
    network: u32,
) -> DashSDKResult {
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        if script_pubkey.is_null() {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "script_pubkey is null".to_string(),
            ));
        }

        let script = std::slice::from_raw_parts(script_pubkey, script_len as usize);

        // Parse P2PKH script: 76 a9 14 <20-byte-hash> 88 ac
        if script.len() != 25
            || script[0] != 0x76
            || script[1] != 0xa9
            || script[2] != 0x14
            || script[23] != 0x88
            || script[24] != 0xac
        {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!(
                    "Invalid P2PKH scriptPubKey: expected 25 bytes (76 a9 14 ... 88 ac), got {} bytes",
                    script.len()
                ),
            ));
        }

        let hash: [u8; 20] = script[3..23]
            .try_into()
            .expect("slice is exactly 20 bytes after length check");
        let platform_addr = PlatformAddress::P2pkh(hash);

        let dash_network = match network {
            0 => dash_sdk::dpp::dashcore::Network::Mainnet,
            1 => dash_sdk::dpp::dashcore::Network::Testnet,
            2 => dash_sdk::dpp::dashcore::Network::Regtest,
            3 => dash_sdk::dpp::dashcore::Network::Devnet,
            _ => {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Unknown network code: {}. Expected 0 (mainnet), 1 (testnet), 2 (regtest), or 3 (devnet)", network),
                ));
            }
        };

        let encoded = platform_addr.to_bech32m_string(dash_network);

        match CString::new(encoded) {
            Ok(c_str) => DashSDKResult::success_string(c_str.into_raw()),
            Err(_) => DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                "Failed to create C string from encoded address".to_string(),
            )),
        }
    }));
    match result {
        Ok(result) => result,
        Err(panic_info) => {
            let panic_message = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic occurred".to_string()
            };
            DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!(
                    "Panic during dash_sdk_encode_platform_address: {}",
                    panic_message
                ),
            ))
        }
    }
}

/// Convert a P2PKH scriptPubKey to the GroveDB storage key format for platform addresses.
///
/// Takes a 25-byte P2PKH scriptPubKey and returns 21-byte storage key
/// (0x00 type byte + 20-byte pubkey hash) used as the key in the platform
/// address funds tree.
///
/// # Safety
/// - `script_pubkey` must point to `script_len` bytes (expected 25 for P2PKH)
/// - `out_key` must point to a buffer of at least 21 bytes
/// - Returns true on success, false on error
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_script_to_platform_address_key(
    script_pubkey: *const u8,
    script_len: u32,
    out_key: *mut u8,
    out_key_len: *mut u32,
) -> bool {
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        if script_pubkey.is_null() || out_key.is_null() || out_key_len.is_null() {
            return false;
        }

        let script_bytes = std::slice::from_raw_parts(script_pubkey, script_len as usize);
        let script = Script::from_bytes(script_bytes);

        // Read the caller-supplied capacity before writing
        let capacity = *out_key_len as usize;

        // Extract the pubkey hash from the P2PKH script
        if let Some(hash_bytes) = script.p2pkh_public_key_hash_bytes() {
            if hash_bytes.len() == 20 {
                let hash: [u8; 20] = hash_bytes
                    .try_into()
                    .expect("hash_bytes is exactly 20 bytes after length check");
                let platform_addr = PlatformAddress::P2pkh(hash);
                let storage_bytes = platform_addr.to_bytes();
                if storage_bytes.len() > capacity {
                    return false;
                }
                std::ptr::copy_nonoverlapping(storage_bytes.as_ptr(), out_key, storage_bytes.len());
                *out_key_len = storage_bytes.len() as u32;
                return true;
            }
        }

        false
    }));
    result.unwrap_or_default()
}

/// Format a raw GroveDB proof as a human-readable string.
///
/// Decodes the proof bytes and uses GroveDBProof's Display implementation
/// to produce a tree-structured visualization of the proof contents.
///
/// # Safety
/// - `proof_bytes` must point to `proof_len` bytes
/// - Returns a DashSDKResult whose `data` field is a heap-allocated C string.
///   Free it with `dash_sdk_string_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_format_grovedb_proof(
    proof_bytes: *const u8,
    proof_len: u32,
) -> DashSDKResult {
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        if proof_bytes.is_null() || proof_len == 0 {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "proof_bytes is null or empty".to_string(),
            ));
        }

        let bytes = std::slice::from_raw_parts(proof_bytes, proof_len as usize);

        let config = bincode::config::standard()
            .with_big_endian()
            .with_limit::<{ 256 * 1024 * 1024 }>();

        let grovedb_proof: GroveDBProof = match bincode::decode_from_slice(bytes, config) {
            Ok((proof, _)) => proof,
            Err(e) => {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Failed to decode GroveDB proof: {}", e),
                ));
            }
        };

        let formatted = format!("{}", grovedb_proof);

        match CString::new(formatted) {
            Ok(c_str) => DashSDKResult::success_string(c_str.into_raw()),
            Err(_) => DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                "Failed to create C string from formatted proof".to_string(),
            )),
        }
    }));
    match result {
        Ok(result) => result,
        Err(panic_info) => {
            let panic_message = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic occurred".to_string()
            };
            DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!(
                    "Panic during dash_sdk_format_grovedb_proof: {}",
                    panic_message
                ),
            ))
        }
    }
}
