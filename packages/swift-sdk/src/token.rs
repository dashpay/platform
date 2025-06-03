//! Token operations for Swift SDK
//!
//! This module provides Swift-friendly wrappers for token operations
//! available in the ios-sdk-ffi crate.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::error::{SwiftDashError, SwiftDashResult};

/// Swift-friendly token transfer parameters
#[repr(C)]
pub struct SwiftDashTokenTransferParams {
    /// Token contract ID (Base58 encoded string)
    pub token_contract_id: *const c_char,
    /// Recipient identity ID (Base58 encoded string)
    pub recipient_id: *const c_char,
    /// Amount to transfer
    pub amount: u64,
    /// Optional public note
    pub public_note: *const c_char,
}

/// Swift-friendly token mint parameters
#[repr(C)]
pub struct SwiftDashTokenMintParams {
    /// Token contract ID (Base58 encoded string)
    pub token_contract_id: *const c_char,
    /// Recipient identity ID (Base58 encoded string)
    pub recipient_id: *const c_char,
    /// Amount to mint
    pub amount: u64,
    /// Optional public note
    pub public_note: *const c_char,
}

/// Swift-friendly token burn parameters
#[repr(C)]
pub struct SwiftDashTokenBurnParams {
    /// Token contract ID (Base58 encoded string)
    pub token_contract_id: *const c_char,
    /// Amount to burn
    pub amount: u64,
    /// Optional public note
    pub public_note: *const c_char,
}

/// Token distribution type for claim operations
#[repr(C)]
pub enum SwiftDashTokenDistributionType {
    /// Pre-programmed distribution
    PreProgrammed = 0,
    /// Perpetual distribution
    Perpetual = 1,
}

/// Swift-friendly token claim parameters
#[repr(C)]
pub struct SwiftDashTokenClaimParams {
    /// Token contract ID (Base58 encoded string)
    pub token_contract_id: *const c_char,
    /// Distribution type (PreProgrammed or Perpetual)
    pub distribution_type: SwiftDashTokenDistributionType,
    /// Optional public note
    pub public_note: *const c_char,
}

/// Transfer tokens between identities
#[no_mangle]
pub extern "C" fn swift_dash_token_transfer(
    sdk_handle: ios_sdk_ffi::SDKHandle,
    sender_identity_handle: ios_sdk_ffi::IdentityHandle,
    params: SwiftDashTokenTransferParams,
    public_key_id: u32,
    signer_handle: ios_sdk_ffi::SignerHandle,
    put_settings: ios_sdk_ffi::IOSSDKPutSettings,
) -> SwiftDashResult {
    let ffi_params = ios_sdk_ffi::IOSSDKTokenTransferParams {
        token_contract_id: params.token_contract_id,
        serialized_contract: std::ptr::null(),
        serialized_contract_len: 0,
        token_position: 0, // Default to first token
        recipient_id: params.recipient_id,
        amount: params.amount,
        public_note: params.public_note,
        private_encrypted_note: std::ptr::null(),
        shared_encrypted_note: std::ptr::null(),
    };

    let result = unsafe {
        ios_sdk_ffi::ios_sdk_token_transfer(
            sdk_handle,
            sender_identity_handle,
            ffi_params,
            public_key_id,
            signer_handle,
            put_settings,
        )
    };

    SwiftDashResult::from_ffi_result(result)
}

/// Transfer tokens and wait for confirmation
#[no_mangle]
pub extern "C" fn swift_dash_token_transfer_and_wait(
    sdk_handle: ios_sdk_ffi::SDKHandle,
    sender_identity_handle: ios_sdk_ffi::IdentityHandle,
    params: SwiftDashTokenTransferParams,
    public_key_id: u32,
    signer_handle: ios_sdk_ffi::SignerHandle,
    put_settings: ios_sdk_ffi::IOSSDKPutSettings,
) -> SwiftDashResult {
    let ffi_params = ios_sdk_ffi::IOSSDKTokenTransferParams {
        token_contract_id: params.token_contract_id,
        serialized_contract: std::ptr::null(),
        serialized_contract_len: 0,
        token_position: 0, // Default to first token
        recipient_id: params.recipient_id,
        amount: params.amount,
        public_note: params.public_note,
        private_encrypted_note: std::ptr::null(),
        shared_encrypted_note: std::ptr::null(),
    };

    let result = unsafe {
        ios_sdk_ffi::ios_sdk_token_transfer_and_wait(
            sdk_handle,
            sender_identity_handle,
            ffi_params,
            public_key_id,
            signer_handle,
            put_settings,
        )
    };

    SwiftDashResult::from_ffi_result(result)
}

/// Mint new tokens
#[no_mangle]
pub extern "C" fn swift_dash_token_mint(
    sdk_handle: ios_sdk_ffi::SDKHandle,
    identity_handle: ios_sdk_ffi::IdentityHandle,
    params: SwiftDashTokenMintParams,
    public_key_id: u32,
    signer_handle: ios_sdk_ffi::SignerHandle,
    put_settings: ios_sdk_ffi::IOSSDKPutSettings,
) -> SwiftDashResult {
    let ffi_params = ios_sdk_ffi::IOSSDKTokenMintParams {
        token_contract_id: params.token_contract_id,
        serialized_contract: std::ptr::null(),
        serialized_contract_len: 0,
        token_position: 0, // Default to first token
        recipient_id: params.recipient_id,
        amount: params.amount,
        public_note: params.public_note,
    };

    let result = unsafe {
        ios_sdk_ffi::ios_sdk_token_mint(
            sdk_handle,
            identity_handle,
            ffi_params,
            public_key_id,
            signer_handle,
            put_settings,
        )
    };

    SwiftDashResult::from_ffi_result(result)
}

/// Mint new tokens and wait for confirmation
#[no_mangle]
pub extern "C" fn swift_dash_token_mint_and_wait(
    sdk_handle: ios_sdk_ffi::SDKHandle,
    identity_handle: ios_sdk_ffi::IdentityHandle,
    params: SwiftDashTokenMintParams,
    public_key_id: u32,
    signer_handle: ios_sdk_ffi::SignerHandle,
    put_settings: ios_sdk_ffi::IOSSDKPutSettings,
) -> SwiftDashResult {
    let ffi_params = ios_sdk_ffi::IOSSDKTokenMintParams {
        token_contract_id: params.token_contract_id,
        serialized_contract: std::ptr::null(),
        serialized_contract_len: 0,
        token_position: 0, // Default to first token
        recipient_id: params.recipient_id,
        amount: params.amount,
        public_note: params.public_note,
    };

    let result = unsafe {
        ios_sdk_ffi::ios_sdk_token_mint_and_wait(
            sdk_handle,
            identity_handle,
            ffi_params,
            public_key_id,
            signer_handle,
            put_settings,
        )
    };

    SwiftDashResult::from_ffi_result(result)
}

/// Burn tokens
#[no_mangle]
pub extern "C" fn swift_dash_token_burn(
    sdk_handle: ios_sdk_ffi::SDKHandle,
    identity_handle: ios_sdk_ffi::IdentityHandle,
    params: SwiftDashTokenBurnParams,
    public_key_id: u32,
    signer_handle: ios_sdk_ffi::SignerHandle,
    put_settings: ios_sdk_ffi::IOSSDKPutSettings,
) -> SwiftDashResult {
    let ffi_params = ios_sdk_ffi::IOSSDKTokenBurnParams {
        token_contract_id: params.token_contract_id,
        serialized_contract: std::ptr::null(),
        serialized_contract_len: 0,
        token_position: 0, // Default to first token
        amount: params.amount,
        public_note: params.public_note,
    };

    let result = unsafe {
        ios_sdk_ffi::ios_sdk_token_burn(
            sdk_handle,
            identity_handle,
            ffi_params,
            public_key_id,
            signer_handle,
            put_settings,
        )
    };

    SwiftDashResult::from_ffi_result(result)
}

/// Burn tokens and wait for confirmation
#[no_mangle]
pub extern "C" fn swift_dash_token_burn_and_wait(
    sdk_handle: ios_sdk_ffi::SDKHandle,
    identity_handle: ios_sdk_ffi::IdentityHandle,
    params: SwiftDashTokenBurnParams,
    public_key_id: u32,
    signer_handle: ios_sdk_ffi::SignerHandle,
    put_settings: ios_sdk_ffi::IOSSDKPutSettings,
) -> SwiftDashResult {
    let ffi_params = ios_sdk_ffi::IOSSDKTokenBurnParams {
        token_contract_id: params.token_contract_id,
        serialized_contract: std::ptr::null(),
        serialized_contract_len: 0,
        token_position: 0, // Default to first token
        amount: params.amount,
        public_note: params.public_note,
    };

    let result = unsafe {
        ios_sdk_ffi::ios_sdk_token_burn_and_wait(
            sdk_handle,
            identity_handle,
            ffi_params,
            public_key_id,
            signer_handle,
            put_settings,
        )
    };

    SwiftDashResult::from_ffi_result(result)
}

/// Claim tokens from distribution
#[no_mangle]
pub extern "C" fn swift_dash_token_claim(
    sdk_handle: ios_sdk_ffi::SDKHandle,
    identity_handle: ios_sdk_ffi::IdentityHandle,
    params: SwiftDashTokenClaimParams,
    public_key_id: u32,
    signer_handle: ios_sdk_ffi::SignerHandle,
    put_settings: ios_sdk_ffi::IOSSDKPutSettings,
) -> SwiftDashResult {
    let ffi_distribution_type = match params.distribution_type {
        SwiftDashTokenDistributionType::PreProgrammed => ios_sdk_ffi::IOSSDKTokenDistributionType::PreProgrammed,
        SwiftDashTokenDistributionType::Perpetual => ios_sdk_ffi::IOSSDKTokenDistributionType::Perpetual,
    };

    let ffi_params = ios_sdk_ffi::IOSSDKTokenClaimParams {
        token_contract_id: params.token_contract_id,
        serialized_contract: std::ptr::null(),
        serialized_contract_len: 0,
        token_position: 0, // Default to first token
        distribution_type: ffi_distribution_type,
        public_note: params.public_note,
    };

    let result = unsafe {
        ios_sdk_ffi::ios_sdk_token_claim(
            sdk_handle,
            identity_handle,
            ffi_params,
            public_key_id,
            signer_handle,
            put_settings,
        )
    };

    SwiftDashResult::from_ffi_result(result)
}

/// Claim tokens from distribution and wait for confirmation
#[no_mangle]
pub extern "C" fn swift_dash_token_claim_and_wait(
    sdk_handle: ios_sdk_ffi::SDKHandle,
    identity_handle: ios_sdk_ffi::IdentityHandle,
    params: SwiftDashTokenClaimParams,
    public_key_id: u32,
    signer_handle: ios_sdk_ffi::SignerHandle,
    put_settings: ios_sdk_ffi::IOSSDKPutSettings,
) -> SwiftDashResult {
    let ffi_distribution_type = match params.distribution_type {
        SwiftDashTokenDistributionType::PreProgrammed => ios_sdk_ffi::IOSSDKTokenDistributionType::PreProgrammed,
        SwiftDashTokenDistributionType::Perpetual => ios_sdk_ffi::IOSSDKTokenDistributionType::Perpetual,
    };

    let ffi_params = ios_sdk_ffi::IOSSDKTokenClaimParams {
        token_contract_id: params.token_contract_id,
        serialized_contract: std::ptr::null(),
        serialized_contract_len: 0,
        token_position: 0, // Default to first token
        distribution_type: ffi_distribution_type,
        public_note: params.public_note,
    };

    let result = unsafe {
        ios_sdk_ffi::ios_sdk_token_claim_and_wait(
            sdk_handle,
            identity_handle,
            ffi_params,
            public_key_id,
            signer_handle,
            put_settings,
        )
    };

    SwiftDashResult::from_ffi_result(result)
}

/// Get token balance for an identity
#[no_mangle]
pub extern "C" fn swift_dash_token_get_identity_balance(
    sdk_handle: ios_sdk_ffi::SDKHandle,
    identity_id: *const c_char,
    token_contract_id: *const c_char,
    token_position: u16,
) -> SwiftDashResult {
    let result = unsafe {
        ios_sdk_ffi::ios_sdk_token_get_identity_balances(
            sdk_handle,
            identity_id,
            token_contract_id,
            token_position,
        )
    };

    SwiftDashResult::from_ffi_result(result)
}

/// Get token information for an identity
#[no_mangle]
pub extern "C" fn swift_dash_token_get_identity_info(
    sdk_handle: ios_sdk_ffi::SDKHandle,
    identity_id: *const c_char,
    token_contract_id: *const c_char,
    token_position: u16,
) -> SwiftDashResult {
    let result = unsafe {
        ios_sdk_ffi::ios_sdk_token_get_identity_infos(
            sdk_handle,
            identity_id,
            token_contract_id,
            token_position,
        )
    };

    SwiftDashResult::from_ffi_result(result)
}

/// Get token statuses for a contract
#[no_mangle]
pub extern "C" fn swift_dash_token_get_statuses(
    sdk_handle: ios_sdk_ffi::SDKHandle,
    token_contract_id: *const c_char,
    token_position: u16,
) -> SwiftDashResult {
    let result = unsafe {
        ios_sdk_ffi::ios_sdk_token_get_statuses(
            sdk_handle,
            token_contract_id,
            token_position,
        )
    };

    SwiftDashResult::from_ffi_result(result)
}