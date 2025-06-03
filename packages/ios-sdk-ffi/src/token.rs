//! Token operations

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::{IOSSDKPutSettings, IdentityHandle, SDKHandle, SignerHandle};
use crate::{FFIError, IOSSDKError, IOSSDKErrorCode, IOSSDKResult};

use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::{Identifier, Identity};
use platform_value::string_encoding::Encoding;

/// Token transfer parameters
#[repr(C)]
pub struct IOSSDKTokenTransferParams {
    /// Token contract ID (Base58 encoded) - mutually exclusive with serialized_contract
    pub token_contract_id: *const c_char,
    /// Serialized data contract (bincode) - mutually exclusive with token_contract_id
    pub serialized_contract: *const u8,
    /// Length of serialized contract data
    pub serialized_contract_len: usize,
    /// Token position in the contract (defaults to 0 if not specified)
    pub token_position: u16,
    /// Recipient identity ID (Base58 encoded)
    pub recipient_id: *const c_char,
    /// Amount to transfer
    pub amount: u64,
    /// Optional public note
    pub public_note: *const c_char,
    /// Optional private encrypted note
    pub private_encrypted_note: *const c_char,
    /// Optional shared encrypted note
    pub shared_encrypted_note: *const c_char,
}

/// Token mint parameters
#[repr(C)]
pub struct IOSSDKTokenMintParams {
    /// Token contract ID (Base58 encoded) - mutually exclusive with serialized_contract
    pub token_contract_id: *const c_char,
    /// Serialized data contract (bincode) - mutually exclusive with token_contract_id
    pub serialized_contract: *const u8,
    /// Length of serialized contract data
    pub serialized_contract_len: usize,
    /// Token position in the contract (defaults to 0 if not specified)
    pub token_position: u16,
    /// Recipient identity ID (Base58 encoded)
    pub recipient_id: *const c_char,
    /// Amount to mint
    pub amount: u64,
    /// Optional public note
    pub public_note: *const c_char,
}

/// Token burn parameters
#[repr(C)]
pub struct IOSSDKTokenBurnParams {
    /// Token contract ID (Base58 encoded) - mutually exclusive with serialized_contract
    pub token_contract_id: *const c_char,
    /// Serialized data contract (bincode) - mutually exclusive with token_contract_id
    pub serialized_contract: *const u8,
    /// Length of serialized contract data
    pub serialized_contract_len: usize,
    /// Token position in the contract (defaults to 0 if not specified)
    pub token_position: u16,
    /// Amount to burn
    pub amount: u64,
    /// Optional public note
    pub public_note: *const c_char,
}

/// Token distribution type for claim operations
#[repr(C)]
pub enum IOSSDKTokenDistributionType {
    /// Pre-programmed distribution
    PreProgrammed = 0,
    /// Perpetual distribution
    Perpetual = 1,
}

/// Token claim parameters
#[repr(C)]
pub struct IOSSDKTokenClaimParams {
    /// Token contract ID (Base58 encoded) - mutually exclusive with serialized_contract
    pub token_contract_id: *const c_char,
    /// Serialized data contract (bincode) - mutually exclusive with token_contract_id
    pub serialized_contract: *const u8,
    /// Length of serialized contract data
    pub serialized_contract_len: usize,
    /// Token position in the contract (defaults to 0 if not specified)
    pub token_position: u16,
    /// Distribution type (PreProgrammed or Perpetual)
    pub distribution_type: IOSSDKTokenDistributionType,
    /// Optional public note
    pub public_note: *const c_char,
}

/// Authorized action takers for token operations
#[repr(C)]
#[derive(Copy, Clone)]
pub enum IOSSDKAuthorizedActionTakers {
    /// No one can perform the action
    NoOne = 0,
    /// Only the contract owner can perform the action
    ContractOwner = 1,
    /// Main group can perform the action
    MainGroup = 2,
    /// A specific identity (requires identity_id to be set)
    Identity = 3,
    /// A specific group (requires group_position to be set)
    Group = 4,
}

/// Token configuration update type
#[repr(C)]
#[derive(Copy, Clone)]
pub enum IOSSDKTokenConfigUpdateType {
    /// No change
    NoChange = 0,
    /// Update max supply (requires amount field)
    MaxSupply = 1,
    /// Update minting allow choosing destination (requires bool_value field)
    MintingAllowChoosingDestination = 2,
    /// Update new tokens destination identity (requires identity_id field)
    NewTokensDestinationIdentity = 3,
    /// Update manual minting permissions (requires action_takers field)
    ManualMinting = 4,
    /// Update manual burning permissions (requires action_takers field)
    ManualBurning = 5,
    /// Update freeze permissions (requires action_takers field)
    Freeze = 6,
    /// Update unfreeze permissions (requires action_takers field)
    Unfreeze = 7,
    /// Update main control group (requires group_position field)
    MainControlGroup = 8,
}

/// Token configuration update parameters
#[repr(C)]
pub struct IOSSDKTokenConfigUpdateParams {
    /// Token contract ID (Base58 encoded) - mutually exclusive with serialized_contract
    pub token_contract_id: *const c_char,
    /// Serialized data contract (bincode) - mutually exclusive with token_contract_id
    pub serialized_contract: *const u8,
    /// Length of serialized contract data
    pub serialized_contract_len: usize,
    /// Token position in the contract (defaults to 0 if not specified)
    pub token_position: u16,
    /// The type of configuration update
    pub update_type: IOSSDKTokenConfigUpdateType,
    /// For MaxSupply updates - the new max supply (0 for no limit)
    pub amount: u64,
    /// For boolean updates like MintingAllowChoosingDestination
    pub bool_value: bool,
    /// For identity-based updates - Base58 encoded identity ID
    pub identity_id: *const c_char,
    /// For group-based updates - the group position
    pub group_position: u16,
    /// For permission updates - the authorized action takers
    pub action_takers: IOSSDKAuthorizedActionTakers,
    /// Optional public note
    pub public_note: *const c_char,
}

/// Token emergency action type
#[repr(C)]
#[derive(Copy, Clone)]
pub enum IOSSDKTokenEmergencyAction {
    /// Pause token operations
    Pause = 0,
    /// Resume token operations
    Resume = 1,
}

/// Token emergency action parameters
#[repr(C)]
pub struct IOSSDKTokenEmergencyActionParams {
    /// Token contract ID (Base58 encoded) - mutually exclusive with serialized_contract
    pub token_contract_id: *const c_char,
    /// Serialized data contract (bincode) - mutually exclusive with token_contract_id
    pub serialized_contract: *const u8,
    /// Length of serialized contract data
    pub serialized_contract_len: usize,
    /// Token position in the contract (defaults to 0 if not specified)
    pub token_position: u16,
    /// The emergency action to perform
    pub action: IOSSDKTokenEmergencyAction,
    /// Optional public note
    pub public_note: *const c_char,
}

/// Token destroy frozen funds parameters
#[repr(C)]
pub struct IOSSDKTokenDestroyFrozenFundsParams {
    /// Token contract ID (Base58 encoded) - mutually exclusive with serialized_contract
    pub token_contract_id: *const c_char,
    /// Serialized data contract (bincode) - mutually exclusive with token_contract_id
    pub serialized_contract: *const u8,
    /// Length of serialized contract data
    pub serialized_contract_len: usize,
    /// Token position in the contract (defaults to 0 if not specified)
    pub token_position: u16,
    /// The frozen identity whose funds to destroy (Base58 encoded)
    pub frozen_identity_id: *const c_char,
    /// Optional public note
    pub public_note: *const c_char,
}

/// Token freeze/unfreeze parameters
#[repr(C)]
pub struct IOSSDKTokenFreezeParams {
    /// Token contract ID (Base58 encoded) - mutually exclusive with serialized_contract
    pub token_contract_id: *const c_char,
    /// Serialized data contract (bincode) - mutually exclusive with token_contract_id
    pub serialized_contract: *const u8,
    /// Length of serialized contract data
    pub serialized_contract_len: usize,
    /// Token position in the contract (defaults to 0 if not specified)
    pub token_position: u16,
    /// The identity to freeze/unfreeze (Base58 encoded)
    pub target_identity_id: *const c_char,
    /// Optional public note
    pub public_note: *const c_char,
}

/// Token purchase parameters
#[repr(C)]
pub struct IOSSDKTokenPurchaseParams {
    /// Token contract ID (Base58 encoded) - mutually exclusive with serialized_contract
    pub token_contract_id: *const c_char,
    /// Serialized data contract (bincode) - mutually exclusive with token_contract_id
    pub serialized_contract: *const u8,
    /// Length of serialized contract data
    pub serialized_contract_len: usize,
    /// Token position in the contract (defaults to 0 if not specified)
    pub token_position: u16,
    /// Amount of tokens to purchase
    pub amount: u64,
    /// Total agreed price in credits
    pub total_agreed_price: u64,
}

/// Token pricing type
#[repr(C)]
#[derive(Copy, Clone)]
pub enum IOSSDKTokenPricingType {
    /// Single flat price for all amounts
    SinglePrice = 0,
    /// Tiered pricing based on amounts
    SetPrices = 1,
}

/// Token price entry for tiered pricing
#[repr(C)]
pub struct IOSSDKTokenPriceEntry {
    /// Token amount threshold
    pub amount: u64,
    /// Price in credits for this amount
    pub price: u64,
}

/// Token set price parameters
#[repr(C)]
pub struct IOSSDKTokenSetPriceParams {
    /// Token contract ID (Base58 encoded) - mutually exclusive with serialized_contract
    pub token_contract_id: *const c_char,
    /// Serialized data contract (bincode) - mutually exclusive with token_contract_id
    pub serialized_contract: *const u8,
    /// Length of serialized contract data
    pub serialized_contract_len: usize,
    /// Token position in the contract (defaults to 0 if not specified)
    pub token_position: u16,
    /// Pricing type
    pub pricing_type: IOSSDKTokenPricingType,
    /// For SinglePrice - the price in credits (ignored for SetPrices)
    pub single_price: u64,
    /// For SetPrices - array of price entries (ignored for SinglePrice)
    pub price_entries: *const IOSSDKTokenPriceEntry,
    /// Number of price entries
    pub price_entries_count: u32,
    /// Optional public note
    pub public_note: *const c_char,
}

/// Transfer tokens from one identity to another
///
/// # Parameters
/// - `sender_identity_handle`: Identity handle of the sender
/// - `params`: Transfer parameters
/// - `identity_public_key_handle`: Public key for signing
/// - `signer_handle`: Cryptographic signer
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
///
/// # Returns
/// Serialized state transition on success
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_token_transfer(
    sdk_handle: *mut SDKHandle,
    sender_identity_handle: *const IdentityHandle,
    params: *const IOSSDKTokenTransferParams,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    put_settings: *const IOSSDKPutSettings,
) -> IOSSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || sender_identity_handle.is_null()
        || params.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let sender_identity = &*(sender_identity_handle as *const Identity);
    let identity_public_key =
        &*(identity_public_key_handle as *const dpp::identity::IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);
    let params = &*params;

    // Validate that either contract ID or serialized contract is provided (but not both)
    let has_contract_id = !params.token_contract_id.is_null();
    let has_serialized_contract =
        !params.serialized_contract.is_null() && params.serialized_contract_len > 0;

    if !has_contract_id && !has_serialized_contract {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Either token contract ID or serialized contract must be provided".to_string(),
        ));
    }

    if has_contract_id && has_serialized_contract {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Cannot provide both token contract ID and serialized contract".to_string(),
        ));
    }

    // Validate recipient ID
    if params.recipient_id.is_null() {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Recipient ID is required".to_string(),
        ));
    }

    let recipient_id_str = match CStr::from_ptr(params.recipient_id).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
    };

    let recipient_id = match Identifier::from_string(recipient_id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return IOSSDKResult::error(IOSSDKError::new(
                IOSSDKErrorCode::InvalidParameter,
                format!("Invalid recipient ID: {}", e),
            ))
        }
    };

    // Parse optional notes
    let public_note = if params.public_note.is_null() {
        None
    } else {
        match CStr::from_ptr(params.public_note).to_str() {
            Ok(s) => Some(s.to_string()),
            Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
        }
    };

    let _private_encrypted_note = if params.private_encrypted_note.is_null() {
        None
    } else {
        match CStr::from_ptr(params.private_encrypted_note).to_str() {
            Ok(s) => Some(s.as_bytes().to_vec()),
            Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
        }
    };

    let _shared_encrypted_note = if params.shared_encrypted_note.is_null() {
        None
    } else {
        match CStr::from_ptr(params.shared_encrypted_note).to_str() {
            Ok(s) => Some(s.as_bytes().to_vec()),
            Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
        }
    };

    let result: Result<Vec<u8>, FFIError> = wrapper.runtime.block_on(async {
        // Convert settings
        let settings = crate::identity::convert_put_settings(put_settings);

        // Get the data contract either by fetching or deserializing
        use dash_sdk::platform::Fetch;
        use dpp::prelude::DataContract;

        let data_contract = if has_contract_id {
// Parse and fetch the contract ID
let token_contract_id_str = match CStr::from_ptr(params.token_contract_id).to_str() {
    Ok(s) => s,
    Err(e) => return Err(FFIError::from(e)),
};

let token_contract_id = match Identifier::from_string(token_contract_id_str, Encoding::Base58) {
    Ok(id) => id,
    Err(e) => {
        return Err(FFIError::InternalError(format!("Invalid token contract ID: {}", e)))
    }
};

// Fetch the data contract
DataContract::fetch(&wrapper.sdk, token_contract_id)
    .await
    .map_err(FFIError::from)?
    .ok_or_else(|| FFIError::InternalError("Token contract not found".to_string()))?
        } else {
// Deserialize the provided contract
let contract_slice = std::slice::from_raw_parts(
    params.serialized_contract,
    params.serialized_contract_len
);

use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;

DataContract::versioned_deserialize(
    contract_slice,
    false, // skip validation since it's already validated
    wrapper.sdk.version(),
)
.map_err(|e| FFIError::InternalError(format!("Failed to deserialize contract: {}", e)))?
        };

        // Create token transfer transition builder
        use dash_sdk::platform::transition::fungible_tokens::transfer::TokenTransferTransitionBuilder;

        let mut builder = TokenTransferTransitionBuilder::new(
&data_contract,
params.token_position,
sender_identity.id(),
recipient_id,
params.amount,
        );

        // Add optional notes
        if let Some(note) = public_note {
builder = builder.with_public_note(note);
        }

        // TODO: Implement encrypted notes with proper parameters
        // if let Some(note) = private_encrypted_note {
        //     builder = builder.with_private_encrypted_note(note);
        // }

        // if let Some(note) = shared_encrypted_note {
        //     builder = builder.with_shared_encrypted_note(note);
        // }

        // Add settings and user fee increase
        if let Some(settings) = settings {
if let Some(fee_increase) = settings.user_fee_increase {
    builder = builder.with_user_fee_increase(fee_increase);
}
builder = builder.with_settings(settings);
        }

        // Sign the transition
        let state_transition = builder
.sign(
    &wrapper.sdk,
    identity_public_key,
    signer,
    wrapper.sdk.version(),
    settings.and_then(|s| s.state_transition_creation_options),
)
.await
.map_err(|e| FFIError::InternalError(format!("Failed to sign transition: {}", e)))?;

        // Serialize the state transition with bincode
        let config = bincode::config::standard();
        bincode::encode_to_vec(&state_transition, config).map_err(|e| {
FFIError::InternalError(format!("Failed to serialize state transition: {}", e))
        })
    });

    match result {
        Ok(serialized_data) => IOSSDKResult::success_binary(serialized_data),
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Mint tokens to an identity
///
/// # Parameters
/// - `minter_identity_handle`: Identity handle of the minter (must have minting permissions)
/// - `params`: Mint parameters
/// - `identity_public_key_handle`: Public key for signing
/// - `signer_handle`: Cryptographic signer
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
///
/// # Returns
/// Serialized state transition on success
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_token_mint(
    sdk_handle: *mut SDKHandle,
    minter_identity_handle: *const IdentityHandle,
    params: *const IOSSDKTokenMintParams,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    put_settings: *const IOSSDKPutSettings,
) -> IOSSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || minter_identity_handle.is_null()
        || params.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let minter_identity = &*(minter_identity_handle as *const Identity);
    let identity_public_key =
        &*(identity_public_key_handle as *const dpp::identity::IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);
    let params = &*params;

    // Validate that either contract ID or serialized contract is provided (but not both)
    let has_contract_id = !params.token_contract_id.is_null();
    let has_serialized_contract =
        !params.serialized_contract.is_null() && params.serialized_contract_len > 0;

    if !has_contract_id && !has_serialized_contract {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Either token contract ID or serialized contract must be provided".to_string(),
        ));
    }

    if has_contract_id && has_serialized_contract {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Cannot provide both token contract ID and serialized contract".to_string(),
        ));
    }

    // Parse optional recipient ID
    let recipient_id = if params.recipient_id.is_null() {
        None
    } else {
        let recipient_id_str = match CStr::from_ptr(params.recipient_id).to_str() {
            Ok(s) => s,
            Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
        };

        match Identifier::from_string(recipient_id_str, Encoding::Base58) {
            Ok(id) => Some(id),
            Err(e) => {
                return IOSSDKResult::error(IOSSDKError::new(
                    IOSSDKErrorCode::InvalidParameter,
                    format!("Invalid recipient ID: {}", e),
                ))
            }
        }
    };

    // Parse optional public note
    let public_note = if params.public_note.is_null() {
        None
    } else {
        match CStr::from_ptr(params.public_note).to_str() {
            Ok(s) => Some(s.to_string()),
            Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
        }
    };

    let result: Result<Vec<u8>, FFIError> = wrapper.runtime.block_on(async {
        // Convert settings
        let settings = crate::identity::convert_put_settings(put_settings);

        // Get the data contract either by fetching or deserializing
        use dash_sdk::platform::Fetch;
        use dpp::prelude::DataContract;

        let data_contract = if has_contract_id {
// Parse and fetch the contract ID
let token_contract_id_str = match CStr::from_ptr(params.token_contract_id).to_str() {
    Ok(s) => s,
    Err(e) => return Err(FFIError::from(e)),
};

let token_contract_id = match Identifier::from_string(token_contract_id_str, Encoding::Base58) {
    Ok(id) => id,
    Err(e) => {
        return Err(FFIError::InternalError(format!("Invalid token contract ID: {}", e)))
    }
};

// Fetch the data contract
DataContract::fetch(&wrapper.sdk, token_contract_id)
    .await
    .map_err(FFIError::from)?
    .ok_or_else(|| FFIError::InternalError("Token contract not found".to_string()))?
        } else {
// Deserialize the provided contract
let contract_slice = std::slice::from_raw_parts(
    params.serialized_contract,
    params.serialized_contract_len
);

use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;

DataContract::versioned_deserialize(
    contract_slice,
    false, // skip validation since it's already validated
    wrapper.sdk.version(),
)
.map_err(|e| FFIError::InternalError(format!("Failed to deserialize contract: {}", e)))?
        };

        // Create token mint transition builder
        use dash_sdk::platform::transition::fungible_tokens::mint::TokenMintTransitionBuilder;

        let mut builder = TokenMintTransitionBuilder::new(
&data_contract,
params.token_position,
minter_identity.id(),
params.amount,
        );

        // Set optional recipient
        if let Some(recipient_id) = recipient_id {
builder = builder.issued_to_identity_id(recipient_id);
        }

        // Add optional public note
        if let Some(note) = public_note {
builder = builder.with_public_note(note);
        }

        // Add settings and user fee increase
        if let Some(settings) = settings {
if let Some(fee_increase) = settings.user_fee_increase {
    builder = builder.with_user_fee_increase(fee_increase);
}
builder = builder.with_settings(settings);
        }

        // Sign the transition
        let state_transition = builder
.sign(
    &wrapper.sdk,
    identity_public_key,
    signer,
    wrapper.sdk.version(),
    settings.and_then(|s| s.state_transition_creation_options),
)
.await
.map_err(|e| FFIError::InternalError(format!("Failed to sign transition: {}", e)))?;

        // Serialize the state transition with bincode
        let config = bincode::config::standard();
        bincode::encode_to_vec(&state_transition, config).map_err(|e| {
FFIError::InternalError(format!("Failed to serialize state transition: {}", e))
        })
    });

    match result {
        Ok(serialized_data) => IOSSDKResult::success_binary(serialized_data),
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Burn tokens from an identity
///
/// # Parameters
/// - `owner_identity_handle`: Identity handle of the token owner
/// - `params`: Burn parameters
/// - `identity_public_key_handle`: Public key for signing
/// - `signer_handle`: Cryptographic signer
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
///
/// # Returns
/// Serialized state transition on success
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_token_burn(
    sdk_handle: *mut SDKHandle,
    owner_identity_handle: *const IdentityHandle,
    params: *const IOSSDKTokenBurnParams,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    put_settings: *const IOSSDKPutSettings,
) -> IOSSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || owner_identity_handle.is_null()
        || params.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let owner_identity = &*(owner_identity_handle as *const Identity);
    let identity_public_key =
        &*(identity_public_key_handle as *const dpp::identity::IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);
    let params = &*params;

    // Validate that either contract ID or serialized contract is provided (but not both)
    let has_contract_id = !params.token_contract_id.is_null();
    let has_serialized_contract =
        !params.serialized_contract.is_null() && params.serialized_contract_len > 0;

    if !has_contract_id && !has_serialized_contract {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Either token contract ID or serialized contract must be provided".to_string(),
        ));
    }

    if has_contract_id && has_serialized_contract {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Cannot provide both token contract ID and serialized contract".to_string(),
        ));
    }

    // Parse optional public note
    let public_note = if params.public_note.is_null() {
        None
    } else {
        match CStr::from_ptr(params.public_note).to_str() {
            Ok(s) => Some(s.to_string()),
            Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
        }
    };

    let result: Result<Vec<u8>, FFIError> = wrapper.runtime.block_on(async {
        // Convert settings
        let settings = crate::identity::convert_put_settings(put_settings);

        use dash_sdk::platform::Fetch;
        use dpp::prelude::DataContract;

        // Get the data contract either by fetching or deserializing
        let data_contract = if has_contract_id {
// Parse and fetch the contract ID
let token_contract_id_str = match CStr::from_ptr(params.token_contract_id).to_str() {
    Ok(s) => s,
    Err(e) => return Err(FFIError::from(e)),
};

let token_contract_id = match Identifier::from_string(token_contract_id_str, Encoding::Base58) {
    Ok(id) => id,
    Err(e) => {
        return Err(FFIError::InternalError(format!("Invalid token contract ID: {}", e)))
    }
};

// Fetch the data contract
DataContract::fetch(&wrapper.sdk, token_contract_id)
    .await
    .map_err(FFIError::from)?
    .ok_or_else(|| FFIError::InternalError("Token contract not found".to_string()))?
        } else {
// Deserialize the provided contract
let contract_slice = std::slice::from_raw_parts(
    params.serialized_contract,
    params.serialized_contract_len
);

use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;

DataContract::versioned_deserialize(
    contract_slice,
    false, // skip validation since it's already validated
    wrapper.sdk.version(),
)
.map_err(|e| FFIError::InternalError(format!("Failed to deserialize contract: {}", e)))?
        };

        // Create token burn transition builder
        use dash_sdk::platform::transition::fungible_tokens::burn::TokenBurnTransitionBuilder;

        let mut builder = TokenBurnTransitionBuilder::new(
&data_contract,
params.token_position,
owner_identity.id(),
params.amount,
        );

        // Add optional public note
        if let Some(note) = public_note {
builder = builder.with_public_note(note);
        }

        // Add settings and user fee increase
        if let Some(settings) = settings {
if let Some(fee_increase) = settings.user_fee_increase {
    builder = builder.with_user_fee_increase(fee_increase);
}
builder = builder.with_settings(settings);
        }

        // Sign the transition
        let state_transition = builder
.sign(
    &wrapper.sdk,
    identity_public_key,
    signer,
    wrapper.sdk.version(),
    settings.and_then(|s| s.state_transition_creation_options),
)
.await
.map_err(|e| FFIError::InternalError(format!("Failed to sign transition: {}", e)))?;

        // Serialize the state transition with bincode
        let config = bincode::config::standard();
        bincode::encode_to_vec(&state_transition, config).map_err(|e| {
FFIError::InternalError(format!("Failed to serialize state transition: {}", e))
        })
    });

    match result {
        Ok(serialized_data) => IOSSDKResult::success_binary(serialized_data),
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Claim tokens from a distribution
///
/// # Parameters
/// - `owner_identity_handle`: Identity handle of the claimer
/// - `params`: Claim parameters
/// - `identity_public_key_handle`: Public key for signing
/// - `signer_handle`: Cryptographic signer
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
///
/// # Returns
/// Serialized state transition on success
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_token_claim(
    sdk_handle: *mut SDKHandle,
    owner_identity_handle: *const IdentityHandle,
    params: *const IOSSDKTokenClaimParams,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    put_settings: *const IOSSDKPutSettings,
) -> IOSSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || owner_identity_handle.is_null()
        || params.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let owner_identity = &*(owner_identity_handle as *const Identity);
    let identity_public_key =
        &*(identity_public_key_handle as *const dpp::identity::IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);
    let params = &*params;

    // Validate that either contract ID or serialized contract is provided (but not both)
    let has_contract_id = !params.token_contract_id.is_null();
    let has_serialized_contract =
        !params.serialized_contract.is_null() && params.serialized_contract_len > 0;

    if !has_contract_id && !has_serialized_contract {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Either token contract ID or serialized contract must be provided".to_string(),
        ));
    }

    if has_contract_id && has_serialized_contract {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Cannot provide both token contract ID and serialized contract".to_string(),
        ));
    }

    // Parse optional public note
    let public_note = if params.public_note.is_null() {
        None
    } else {
        match CStr::from_ptr(params.public_note).to_str() {
            Ok(s) => Some(s.to_string()),
            Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
        }
    };

    // Convert distribution type
    let distribution_type = match params.distribution_type {
        IOSSDKTokenDistributionType::PreProgrammed => {
dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType::PreProgrammed
        }
        IOSSDKTokenDistributionType::Perpetual => {
dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType::Perpetual
        }
    };

    let result: Result<Vec<u8>, FFIError> = wrapper.runtime.block_on(async {
        // Convert settings
        let settings = crate::identity::convert_put_settings(put_settings);

        use dash_sdk::platform::Fetch;
        use dpp::prelude::DataContract;

        // Get the data contract either by fetching or deserializing
        let data_contract = if has_contract_id {
// Parse and fetch the contract ID
let token_contract_id_str = match CStr::from_ptr(params.token_contract_id).to_str() {
    Ok(s) => s,
    Err(e) => return Err(FFIError::from(e)),
};

let token_contract_id = match Identifier::from_string(token_contract_id_str, Encoding::Base58) {
    Ok(id) => id,
    Err(e) => {
        return Err(FFIError::InternalError(format!("Invalid token contract ID: {}", e)))
    }
};

// Fetch the data contract
DataContract::fetch(&wrapper.sdk, token_contract_id)
    .await
    .map_err(FFIError::from)?
    .ok_or_else(|| FFIError::InternalError("Token contract not found".to_string()))?
        } else {
// Deserialize the provided contract
let contract_slice = std::slice::from_raw_parts(
    params.serialized_contract,
    params.serialized_contract_len
);

use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;

DataContract::versioned_deserialize(
    contract_slice,
    false, // skip validation since it's already validated
    wrapper.sdk.version(),
)
.map_err(|e| FFIError::InternalError(format!("Failed to deserialize contract: {}", e)))?
        };

        // Create token claim transition builder
        use dash_sdk::platform::transition::fungible_tokens::claim::TokenClaimTransitionBuilder;

        let mut builder = TokenClaimTransitionBuilder::new(
&data_contract,
params.token_position,
owner_identity.id(),
distribution_type,
        );

        // Add optional public note
        if let Some(note) = public_note {
builder = builder.with_public_note(note);
        }

        // Add settings and user fee increase
        if let Some(settings) = settings {
if let Some(fee_increase) = settings.user_fee_increase {
    builder = builder.with_user_fee_increase(fee_increase);
}
builder = builder.with_settings(settings);
        }

        // Sign the transition
        let state_transition = builder
.sign(
    &wrapper.sdk,
    identity_public_key,
    signer,
    wrapper.sdk.version(),
    settings.and_then(|s| s.state_transition_creation_options),
)
.await
.map_err(|e| FFIError::InternalError(format!("Failed to sign transition: {}", e)))?;

        // Serialize the state transition with bincode
        let config = bincode::config::standard();
        bincode::encode_to_vec(&state_transition, config).map_err(|e| {
FFIError::InternalError(format!("Failed to serialize state transition: {}", e))
        })
    });

    match result {
        Ok(serialized_data) => IOSSDKResult::success_binary(serialized_data),
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Update token configuration
///
/// # Parameters
/// - `owner_identity_handle`: Identity handle of the token owner/admin
/// - `params`: Configuration update parameters
/// - `identity_public_key_handle`: Public key for signing
/// - `signer_handle`: Cryptographic signer
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
///
/// # Returns
/// Serialized state transition on success
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_token_config_update(
    sdk_handle: *mut SDKHandle,
    owner_identity_handle: *const IdentityHandle,
    params: *const IOSSDKTokenConfigUpdateParams,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    put_settings: *const IOSSDKPutSettings,
) -> IOSSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || owner_identity_handle.is_null()
        || params.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let owner_identity = &*(owner_identity_handle as *const Identity);
    let identity_public_key =
        &*(identity_public_key_handle as *const dpp::identity::IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);
    let params = &*params;

    // Validate that either contract ID or serialized contract is provided (but not both)
    let has_contract_id = !params.token_contract_id.is_null();
    let has_serialized_contract =
        !params.serialized_contract.is_null() && params.serialized_contract_len > 0;

    if !has_contract_id && !has_serialized_contract {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Either token contract ID or serialized contract must be provided".to_string(),
        ));
    }

    if has_contract_id && has_serialized_contract {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Cannot provide both token contract ID and serialized contract".to_string(),
        ));
    }

    // Parse optional public note
    let public_note = if params.public_note.is_null() {
        None
    } else {
        match CStr::from_ptr(params.public_note).to_str() {
            Ok(s) => Some(s.to_string()),
            Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
        }
    };

    let result: Result<Vec<u8>, FFIError> = wrapper.runtime.block_on(async {
        // Convert settings
        let settings = crate::identity::convert_put_settings(put_settings);

        use dash_sdk::platform::Fetch;
        use dpp::prelude::DataContract;
        use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;

        // Get the data contract either by fetching or deserializing
        let data_contract = if has_contract_id {
// Parse and fetch the contract ID
let token_contract_id_str = match CStr::from_ptr(params.token_contract_id).to_str() {
    Ok(s) => s,
    Err(e) => return Err(FFIError::from(e)),
};

let token_contract_id = match Identifier::from_string(token_contract_id_str, Encoding::Base58) {
    Ok(id) => id,
    Err(e) => {
        return Err(FFIError::InternalError(format!("Invalid token contract ID: {}", e)))
    }
};

// Fetch the data contract
DataContract::fetch(&wrapper.sdk, token_contract_id)
    .await
    .map_err(FFIError::from)?
    .ok_or_else(|| FFIError::InternalError("Token contract not found".to_string()))?
        } else {
// Deserialize the provided contract
let contract_slice = std::slice::from_raw_parts(
    params.serialized_contract,
    params.serialized_contract_len
);

use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;

DataContract::versioned_deserialize(
    contract_slice,
    false, // skip validation since it's already validated
    wrapper.sdk.version(),
)
.map_err(|e| FFIError::InternalError(format!("Failed to deserialize contract: {}", e)))?
        };

        // Build the configuration change item based on the update type
        let config_change_item = match params.update_type {
IOSSDKTokenConfigUpdateType::NoChange => {
    TokenConfigurationChangeItem::TokenConfigurationNoChange
}
IOSSDKTokenConfigUpdateType::MaxSupply => {
    let max_supply = if params.amount == 0 {
        None
    } else {
        Some(params.amount)
    };
    TokenConfigurationChangeItem::MaxSupply(max_supply)
}
IOSSDKTokenConfigUpdateType::MintingAllowChoosingDestination => {
    TokenConfigurationChangeItem::MintingAllowChoosingDestination(params.bool_value)
}
IOSSDKTokenConfigUpdateType::NewTokensDestinationIdentity => {
    let dest_identity = if params.identity_id.is_null() {
        None
    } else {
        let identity_id_str = match CStr::from_ptr(params.identity_id).to_str() {
Ok(s) => s,
Err(e) => return Err(FFIError::from(e)),
        };
        let identity_id = match Identifier::from_string(identity_id_str, Encoding::Base58) {
Ok(id) => id,
Err(e) => {
    return Err(FFIError::InternalError(format!("Invalid identity ID: {}", e)))
}
        };
        Some(identity_id)
    };
    TokenConfigurationChangeItem::NewTokensDestinationIdentity(dest_identity)
}
IOSSDKTokenConfigUpdateType::ManualMinting => {
    let action_takers = convert_authorized_action_takers(params.action_takers, params)?;
    TokenConfigurationChangeItem::ManualMinting(action_takers)
}
IOSSDKTokenConfigUpdateType::ManualBurning => {
    let action_takers = convert_authorized_action_takers(params.action_takers, params)?;
    TokenConfigurationChangeItem::ManualBurning(action_takers)
}
IOSSDKTokenConfigUpdateType::Freeze => {
    let action_takers = convert_authorized_action_takers(params.action_takers, params)?;
    TokenConfigurationChangeItem::Freeze(action_takers)
}
IOSSDKTokenConfigUpdateType::Unfreeze => {
    let action_takers = convert_authorized_action_takers(params.action_takers, params)?;
    TokenConfigurationChangeItem::Unfreeze(action_takers)
}
IOSSDKTokenConfigUpdateType::MainControlGroup => {
    let group_position = if params.group_position == 0 {
        None
    } else {
        Some(params.group_position)
    };
    TokenConfigurationChangeItem::MainControlGroup(group_position)
}
        };

        // Create token config update transition builder
        use dash_sdk::platform::transition::fungible_tokens::config_update::TokenConfigUpdateTransitionBuilder;

        let mut builder = TokenConfigUpdateTransitionBuilder::new(
&data_contract,
params.token_position,
owner_identity.id(),
config_change_item,
        );

        // Add optional public note
        if let Some(note) = public_note {
builder = builder.with_public_note(note);
        }

        // Add settings and user fee increase
        if let Some(settings) = settings {
if let Some(fee_increase) = settings.user_fee_increase {
    builder = builder.with_user_fee_increase(fee_increase);
}
builder = builder.with_settings(settings);
        }

        // Sign the transition
        let state_transition = builder
.sign(
    &wrapper.sdk,
    identity_public_key,
    signer,
    wrapper.sdk.version(),
    settings.and_then(|s| s.state_transition_creation_options),
)
.await
.map_err(|e| FFIError::InternalError(format!("Failed to sign transition: {}", e)))?;

        // Serialize the state transition with bincode
        let config = bincode::config::standard();
        bincode::encode_to_vec(&state_transition, config).map_err(|e| {
FFIError::InternalError(format!("Failed to serialize state transition: {}", e))
        })
    });

    match result {
        Ok(serialized_data) => IOSSDKResult::success_binary(serialized_data),
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Helper function to convert iOS authorized action takers to Rust type
unsafe fn convert_authorized_action_takers(
    action_takers: IOSSDKAuthorizedActionTakers,
    params: &IOSSDKTokenConfigUpdateParams,
) -> Result<
    dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers,
    FFIError,
> {
    use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;

    match action_takers {
        IOSSDKAuthorizedActionTakers::NoOne => Ok(AuthorizedActionTakers::NoOne),
        IOSSDKAuthorizedActionTakers::ContractOwner => Ok(AuthorizedActionTakers::ContractOwner),
        IOSSDKAuthorizedActionTakers::MainGroup => Ok(AuthorizedActionTakers::MainGroup),
        IOSSDKAuthorizedActionTakers::Identity => {
            if params.identity_id.is_null() {
                return Err(FFIError::InternalError(
                    "Identity ID required for Identity action taker".to_string(),
                ));
            }
            let identity_id_str = match CStr::from_ptr(params.identity_id).to_str() {
                Ok(s) => s,
                Err(e) => return Err(FFIError::from(e)),
            };
            let identity_id = match Identifier::from_string(identity_id_str, Encoding::Base58) {
                Ok(id) => id,
                Err(e) => {
                    return Err(FFIError::InternalError(format!(
                        "Invalid identity ID: {}",
                        e
                    )))
                }
            };
            Ok(AuthorizedActionTakers::Identity(identity_id))
        }
        IOSSDKAuthorizedActionTakers::Group => {
            Ok(AuthorizedActionTakers::Group(params.group_position))
        }
    }
}

/// Perform emergency action on tokens (pause/resume)
///
/// # Parameters
/// - `actor_identity_handle`: Identity handle of the actor performing the emergency action
/// - `params`: Emergency action parameters
/// - `identity_public_key_handle`: Public key for signing
/// - `signer_handle`: Cryptographic signer
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
///
/// # Returns
/// Serialized state transition on success
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_token_emergency_action(
    sdk_handle: *mut SDKHandle,
    actor_identity_handle: *const IdentityHandle,
    params: *const IOSSDKTokenEmergencyActionParams,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    put_settings: *const IOSSDKPutSettings,
) -> IOSSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || actor_identity_handle.is_null()
        || params.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let actor_identity = &*(actor_identity_handle as *const Identity);
    let identity_public_key =
        &*(identity_public_key_handle as *const dpp::identity::IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);
    let params = &*params;

    // Validate that either contract ID or serialized contract is provided (but not both)
    let has_contract_id = !params.token_contract_id.is_null();
    let has_serialized_contract =
        !params.serialized_contract.is_null() && params.serialized_contract_len > 0;

    if !has_contract_id && !has_serialized_contract {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Either token contract ID or serialized contract must be provided".to_string(),
        ));
    }

    if has_contract_id && has_serialized_contract {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Cannot provide both token contract ID and serialized contract".to_string(),
        ));
    }

    // Parse optional public note
    let public_note = if params.public_note.is_null() {
        None
    } else {
        match CStr::from_ptr(params.public_note).to_str() {
            Ok(s) => Some(s.to_string()),
            Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
        }
    };

    // Convert emergency action type
    let action = match params.action {
        IOSSDKTokenEmergencyAction::Pause => {
            dpp::tokens::emergency_action::TokenEmergencyAction::Pause
        }
        IOSSDKTokenEmergencyAction::Resume => {
            dpp::tokens::emergency_action::TokenEmergencyAction::Resume
        }
    };

    let result: Result<Vec<u8>, FFIError> = wrapper.runtime.block_on(async {
        // Convert settings
        let settings = crate::identity::convert_put_settings(put_settings);

        use dash_sdk::platform::Fetch;
        use dpp::prelude::DataContract;

        // Get the data contract either by fetching or deserializing
        let data_contract = if has_contract_id {
// Parse and fetch the contract ID
let token_contract_id_str = match CStr::from_ptr(params.token_contract_id).to_str() {
    Ok(s) => s,
    Err(e) => return Err(FFIError::from(e)),
};

let token_contract_id = match Identifier::from_string(token_contract_id_str, Encoding::Base58) {
    Ok(id) => id,
    Err(e) => {
        return Err(FFIError::InternalError(format!("Invalid token contract ID: {}", e)))
    }
};

// Fetch the data contract
DataContract::fetch(&wrapper.sdk, token_contract_id)
    .await
    .map_err(FFIError::from)?
    .ok_or_else(|| FFIError::InternalError("Token contract not found".to_string()))?
        } else {
// Deserialize the provided contract
let contract_slice = std::slice::from_raw_parts(
    params.serialized_contract,
    params.serialized_contract_len
);

use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;

DataContract::versioned_deserialize(
    contract_slice,
    false, // skip validation since it's already validated
    wrapper.sdk.version(),
)
.map_err(|e| FFIError::InternalError(format!("Failed to deserialize contract: {}", e)))?
        };

        // Create token emergency action transition builder
        use dash_sdk::platform::transition::fungible_tokens::emergency_action::TokenEmergencyActionTransitionBuilder;

        let mut builder = match action {
dpp::tokens::emergency_action::TokenEmergencyAction::Pause => {
    TokenEmergencyActionTransitionBuilder::pause(
        &data_contract,
        params.token_position,
        actor_identity.id(),
    )
}
dpp::tokens::emergency_action::TokenEmergencyAction::Resume => {
    TokenEmergencyActionTransitionBuilder::resume(
        &data_contract,
        params.token_position,
        actor_identity.id(),
    )
}
        };

        // Add optional public note
        if let Some(note) = public_note {
builder = builder.with_public_note(note);
        }

        // Add settings and user fee increase
        if let Some(settings) = settings {
if let Some(fee_increase) = settings.user_fee_increase {
    builder = builder.with_user_fee_increase(fee_increase);
}
builder = builder.with_settings(settings);
        }

        // Sign the transition
        let state_transition = builder
.sign(
    &wrapper.sdk,
    identity_public_key,
    signer,
    wrapper.sdk.version(),
    settings.and_then(|s| s.state_transition_creation_options),
)
.await
.map_err(|e| FFIError::InternalError(format!("Failed to sign transition: {}", e)))?;

        // Serialize the state transition with bincode
        let config = bincode::config::standard();
        bincode::encode_to_vec(&state_transition, config).map_err(|e| {
FFIError::InternalError(format!("Failed to serialize state transition: {}", e))
        })
    });

    match result {
        Ok(serialized_data) => IOSSDKResult::success_binary(serialized_data),
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Destroy tokens belonging to a frozen identity
///
/// # Parameters
/// - `actor_identity_handle`: Identity handle of the actor performing the destroy action
/// - `params`: Destroy frozen funds parameters
/// - `identity_public_key_handle`: Public key for signing
/// - `signer_handle`: Cryptographic signer
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
///
/// # Returns
/// Serialized state transition on success
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_token_destroy_frozen_funds(
    sdk_handle: *mut SDKHandle,
    actor_identity_handle: *const IdentityHandle,
    params: *const IOSSDKTokenDestroyFrozenFundsParams,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    put_settings: *const IOSSDKPutSettings,
) -> IOSSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || actor_identity_handle.is_null()
        || params.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let actor_identity = &*(actor_identity_handle as *const Identity);
    let identity_public_key =
        &*(identity_public_key_handle as *const dpp::identity::IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);
    let params = &*params;

    // Validate that either contract ID or serialized contract is provided (but not both)
    let has_contract_id = !params.token_contract_id.is_null();
    let has_serialized_contract =
        !params.serialized_contract.is_null() && params.serialized_contract_len > 0;

    if !has_contract_id && !has_serialized_contract {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Either token contract ID or serialized contract must be provided".to_string(),
        ));
    }

    if has_contract_id && has_serialized_contract {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Cannot provide both token contract ID and serialized contract".to_string(),
        ));
    }

    // Validate frozen identity ID
    if params.frozen_identity_id.is_null() {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Frozen identity ID is required".to_string(),
        ));
    }

    let frozen_identity_id_str = match CStr::from_ptr(params.frozen_identity_id).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
    };

    let frozen_identity_id = match Identifier::from_string(frozen_identity_id_str, Encoding::Base58)
    {
        Ok(id) => id,
        Err(e) => {
            return IOSSDKResult::error(IOSSDKError::new(
                IOSSDKErrorCode::InvalidParameter,
                format!("Invalid frozen identity ID: {}", e),
            ))
        }
    };

    // Parse optional public note
    let public_note = if params.public_note.is_null() {
        None
    } else {
        match CStr::from_ptr(params.public_note).to_str() {
            Ok(s) => Some(s.to_string()),
            Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
        }
    };

    let result: Result<Vec<u8>, FFIError> = wrapper.runtime.block_on(async {
        // Convert settings
        let settings = crate::identity::convert_put_settings(put_settings);

        use dash_sdk::platform::Fetch;
        use dpp::prelude::DataContract;

        // Get the data contract either by fetching or deserializing
        let data_contract = if has_contract_id {
// Parse and fetch the contract ID
let token_contract_id_str = match CStr::from_ptr(params.token_contract_id).to_str() {
    Ok(s) => s,
    Err(e) => return Err(FFIError::from(e)),
};

let token_contract_id = match Identifier::from_string(token_contract_id_str, Encoding::Base58) {
    Ok(id) => id,
    Err(e) => {
        return Err(FFIError::InternalError(format!("Invalid token contract ID: {}", e)))
    }
};

// Fetch the data contract
DataContract::fetch(&wrapper.sdk, token_contract_id)
    .await
    .map_err(FFIError::from)?
    .ok_or_else(|| FFIError::InternalError("Token contract not found".to_string()))?
        } else {
// Deserialize the provided contract
let contract_slice = std::slice::from_raw_parts(
    params.serialized_contract,
    params.serialized_contract_len
);

use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;

DataContract::versioned_deserialize(
    contract_slice,
    false, // skip validation since it's already validated
    wrapper.sdk.version(),
)
.map_err(|e| FFIError::InternalError(format!("Failed to deserialize contract: {}", e)))?
        };

        // Create token destroy frozen funds transition builder
        use dash_sdk::platform::transition::fungible_tokens::destroy::TokenDestroyFrozenFundsTransitionBuilder;

        let mut builder = TokenDestroyFrozenFundsTransitionBuilder::new(
&data_contract,
params.token_position,
actor_identity.id(),
frozen_identity_id,
        );

        // Add optional public note
        if let Some(note) = public_note {
builder = builder.with_public_note(note);
        }

        // Add settings and user fee increase
        if let Some(settings) = settings {
if let Some(fee_increase) = settings.user_fee_increase {
    builder = builder.with_user_fee_increase(fee_increase);
}
builder = builder.with_settings(settings);
        }

        // Sign the transition
        let state_transition = builder
.sign(
    &wrapper.sdk,
    identity_public_key,
    signer,
    wrapper.sdk.version(),
    settings.and_then(|s| s.state_transition_creation_options),
)
.await
.map_err(|e| FFIError::InternalError(format!("Failed to sign transition: {}", e)))?;

        // Serialize the state transition with bincode
        let config = bincode::config::standard();
        bincode::encode_to_vec(&state_transition, config).map_err(|e| {
FFIError::InternalError(format!("Failed to serialize state transition: {}", e))
        })
    });

    match result {
        Ok(serialized_data) => IOSSDKResult::success_binary(serialized_data),
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Freeze tokens for a specific identity
///
/// # Parameters
/// - `actor_identity_handle`: Identity handle of the actor performing the freeze action
/// - `params`: Freeze parameters
/// - `identity_public_key_handle`: Public key for signing
/// - `signer_handle`: Cryptographic signer
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
///
/// # Returns
/// Serialized state transition on success
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_token_freeze(
    sdk_handle: *mut SDKHandle,
    actor_identity_handle: *const IdentityHandle,
    params: *const IOSSDKTokenFreezeParams,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    put_settings: *const IOSSDKPutSettings,
) -> IOSSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || actor_identity_handle.is_null()
        || params.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let actor_identity = &*(actor_identity_handle as *const Identity);
    let identity_public_key =
        &*(identity_public_key_handle as *const dpp::identity::IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);
    let params = &*params;

    // Validate that either contract ID or serialized contract is provided (but not both)
    let has_contract_id = !params.token_contract_id.is_null();
    let has_serialized_contract =
        !params.serialized_contract.is_null() && params.serialized_contract_len > 0;

    if !has_contract_id && !has_serialized_contract {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Either token contract ID or serialized contract must be provided".to_string(),
        ));
    }

    if has_contract_id && has_serialized_contract {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Cannot provide both token contract ID and serialized contract".to_string(),
        ));
    }

    // Validate target identity ID
    if params.target_identity_id.is_null() {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Target identity ID is required".to_string(),
        ));
    }

    let target_identity_id_str = match CStr::from_ptr(params.target_identity_id).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
    };

    let target_identity_id = match Identifier::from_string(target_identity_id_str, Encoding::Base58)
    {
        Ok(id) => id,
        Err(e) => {
            return IOSSDKResult::error(IOSSDKError::new(
                IOSSDKErrorCode::InvalidParameter,
                format!("Invalid target identity ID: {}", e),
            ))
        }
    };

    // Parse optional public note
    let public_note = if params.public_note.is_null() {
        None
    } else {
        match CStr::from_ptr(params.public_note).to_str() {
            Ok(s) => Some(s.to_string()),
            Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
        }
    };

    let result: Result<Vec<u8>, FFIError> = wrapper.runtime.block_on(async {
        // Convert settings
        let settings = crate::identity::convert_put_settings(put_settings);

        use dash_sdk::platform::Fetch;
        use dpp::prelude::DataContract;

        // Get the data contract either by fetching or deserializing
        let data_contract = if has_contract_id {
// Parse and fetch the contract ID
let token_contract_id_str = match CStr::from_ptr(params.token_contract_id).to_str() {
    Ok(s) => s,
    Err(e) => return Err(FFIError::from(e)),
};

let token_contract_id = match Identifier::from_string(token_contract_id_str, Encoding::Base58) {
    Ok(id) => id,
    Err(e) => {
        return Err(FFIError::InternalError(format!("Invalid token contract ID: {}", e)))
    }
};

// Fetch the data contract
DataContract::fetch(&wrapper.sdk, token_contract_id)
    .await
    .map_err(FFIError::from)?
    .ok_or_else(|| FFIError::InternalError("Token contract not found".to_string()))?
        } else {
// Deserialize the provided contract
let contract_slice = std::slice::from_raw_parts(
    params.serialized_contract,
    params.serialized_contract_len
);

use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;

DataContract::versioned_deserialize(
    contract_slice,
    false, // skip validation since it's already validated
    wrapper.sdk.version(),
)
.map_err(|e| FFIError::InternalError(format!("Failed to deserialize contract: {}", e)))?
        };

        // Create token freeze transition builder
        use dash_sdk::platform::transition::fungible_tokens::freeze::TokenFreezeTransitionBuilder;

        let mut builder = TokenFreezeTransitionBuilder::new(
&data_contract,
params.token_position,
actor_identity.id(),
target_identity_id,
        );

        // Add optional public note
        if let Some(note) = public_note {
builder = builder.with_public_note(note);
        }

        // Add settings and user fee increase
        if let Some(settings) = settings {
if let Some(fee_increase) = settings.user_fee_increase {
    builder = builder.with_user_fee_increase(fee_increase);
}
builder = builder.with_settings(settings);
        }

        // Sign the transition
        let state_transition = builder
.sign(
    &wrapper.sdk,
    identity_public_key,
    signer,
    wrapper.sdk.version(),
    settings.and_then(|s| s.state_transition_creation_options),
)
.await
.map_err(|e| FFIError::InternalError(format!("Failed to sign transition: {}", e)))?;

        // Serialize the state transition with bincode
        let config = bincode::config::standard();
        bincode::encode_to_vec(&state_transition, config).map_err(|e| {
FFIError::InternalError(format!("Failed to serialize state transition: {}", e))
        })
    });

    match result {
        Ok(serialized_data) => IOSSDKResult::success_binary(serialized_data),
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Unfreeze tokens for a specific identity
///
/// # Parameters
/// - `actor_identity_handle`: Identity handle of the actor performing the unfreeze action
/// - `params`: Unfreeze parameters (uses same struct as freeze)
/// - `identity_public_key_handle`: Public key for signing
/// - `signer_handle`: Cryptographic signer
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
///
/// # Returns
/// Serialized state transition on success
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_token_unfreeze(
    sdk_handle: *mut SDKHandle,
    actor_identity_handle: *const IdentityHandle,
    params: *const IOSSDKTokenFreezeParams,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    put_settings: *const IOSSDKPutSettings,
) -> IOSSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || actor_identity_handle.is_null()
        || params.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let actor_identity = &*(actor_identity_handle as *const Identity);
    let identity_public_key =
        &*(identity_public_key_handle as *const dpp::identity::IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);
    let params = &*params;

    // Validate that either contract ID or serialized contract is provided (but not both)
    let has_contract_id = !params.token_contract_id.is_null();
    let has_serialized_contract =
        !params.serialized_contract.is_null() && params.serialized_contract_len > 0;

    if !has_contract_id && !has_serialized_contract {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Either token contract ID or serialized contract must be provided".to_string(),
        ));
    }

    if has_contract_id && has_serialized_contract {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Cannot provide both token contract ID and serialized contract".to_string(),
        ));
    }

    // Validate target identity ID
    if params.target_identity_id.is_null() {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Target identity ID is required".to_string(),
        ));
    }

    let target_identity_id_str = match CStr::from_ptr(params.target_identity_id).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
    };

    let target_identity_id = match Identifier::from_string(target_identity_id_str, Encoding::Base58)
    {
        Ok(id) => id,
        Err(e) => {
            return IOSSDKResult::error(IOSSDKError::new(
                IOSSDKErrorCode::InvalidParameter,
                format!("Invalid target identity ID: {}", e),
            ))
        }
    };

    // Parse optional public note
    let public_note = if params.public_note.is_null() {
        None
    } else {
        match CStr::from_ptr(params.public_note).to_str() {
            Ok(s) => Some(s.to_string()),
            Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
        }
    };

    let result: Result<Vec<u8>, FFIError> = wrapper.runtime.block_on(async {
        // Convert settings
        let settings = crate::identity::convert_put_settings(put_settings);

        use dash_sdk::platform::Fetch;
        use dpp::prelude::DataContract;

        // Get the data contract either by fetching or deserializing
        let data_contract = if has_contract_id {
// Parse and fetch the contract ID
let token_contract_id_str = match CStr::from_ptr(params.token_contract_id).to_str() {
    Ok(s) => s,
    Err(e) => return Err(FFIError::from(e)),
};

let token_contract_id = match Identifier::from_string(token_contract_id_str, Encoding::Base58) {
    Ok(id) => id,
    Err(e) => {
        return Err(FFIError::InternalError(format!("Invalid token contract ID: {}", e)))
    }
};

// Fetch the data contract
DataContract::fetch(&wrapper.sdk, token_contract_id)
    .await
    .map_err(FFIError::from)?
    .ok_or_else(|| FFIError::InternalError("Token contract not found".to_string()))?
        } else {
// Deserialize the provided contract
let contract_slice = std::slice::from_raw_parts(
    params.serialized_contract,
    params.serialized_contract_len
);

use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;

DataContract::versioned_deserialize(
    contract_slice,
    false, // skip validation since it's already validated
    wrapper.sdk.version(),
)
.map_err(|e| FFIError::InternalError(format!("Failed to deserialize contract: {}", e)))?
        };

        // Create token unfreeze transition builder
        use dash_sdk::platform::transition::fungible_tokens::unfreeze::TokenUnfreezeTransitionBuilder;

        let mut builder = TokenUnfreezeTransitionBuilder::new(
&data_contract,
params.token_position,
actor_identity.id(),
target_identity_id,
        );

        // Add optional public note
        if let Some(note) = public_note {
builder = builder.with_public_note(note);
        }

        // Add settings and user fee increase
        if let Some(settings) = settings {
if let Some(fee_increase) = settings.user_fee_increase {
    builder = builder.with_user_fee_increase(fee_increase);
}
builder = builder.with_settings(settings);
        }

        // Sign the transition
        let state_transition = builder
.sign(
    &wrapper.sdk,
    identity_public_key,
    signer,
    wrapper.sdk.version(),
    settings.and_then(|s| s.state_transition_creation_options),
)
.await
.map_err(|e| FFIError::InternalError(format!("Failed to sign transition: {}", e)))?;

        // Serialize the state transition with bincode
        let config = bincode::config::standard();
        bincode::encode_to_vec(&state_transition, config).map_err(|e| {
FFIError::InternalError(format!("Failed to serialize state transition: {}", e))
        })
    });

    match result {
        Ok(serialized_data) => IOSSDKResult::success_binary(serialized_data),
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Purchase tokens directly from the issuer
///
/// # Parameters
/// - `purchaser_identity_handle`: Identity handle of the purchaser
/// - `params`: Purchase parameters
/// - `identity_public_key_handle`: Public key for signing
/// - `signer_handle`: Cryptographic signer
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
///
/// # Returns
/// Serialized state transition on success
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_token_purchase(
    sdk_handle: *mut SDKHandle,
    purchaser_identity_handle: *const IdentityHandle,
    params: *const IOSSDKTokenPurchaseParams,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    put_settings: *const IOSSDKPutSettings,
) -> IOSSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || purchaser_identity_handle.is_null()
        || params.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let purchaser_identity = &*(purchaser_identity_handle as *const Identity);
    let identity_public_key =
        &*(identity_public_key_handle as *const dpp::identity::IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);
    let params = &*params;

    // Validate that either contract ID or serialized contract is provided (but not both)
    let has_contract_id = !params.token_contract_id.is_null();
    let has_serialized_contract =
        !params.serialized_contract.is_null() && params.serialized_contract_len > 0;

    if !has_contract_id && !has_serialized_contract {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Either token contract ID or serialized contract must be provided".to_string(),
        ));
    }

    if has_contract_id && has_serialized_contract {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Cannot provide both token contract ID and serialized contract".to_string(),
        ));
    }

    let result: Result<Vec<u8>, FFIError> = wrapper.runtime.block_on(async {
        // Convert settings
        let settings = crate::identity::convert_put_settings(put_settings);

        use dash_sdk::platform::Fetch;
        use dpp::prelude::DataContract;

        // Get the data contract either by fetching or deserializing
        let data_contract = if has_contract_id {
// Parse and fetch the contract ID
let token_contract_id_str = match CStr::from_ptr(params.token_contract_id).to_str() {
    Ok(s) => s,
    Err(e) => return Err(FFIError::from(e)),
};

let token_contract_id = match Identifier::from_string(token_contract_id_str, Encoding::Base58) {
    Ok(id) => id,
    Err(e) => {
        return Err(FFIError::InternalError(format!("Invalid token contract ID: {}", e)))
    }
};

// Fetch the data contract
DataContract::fetch(&wrapper.sdk, token_contract_id)
    .await
    .map_err(FFIError::from)?
    .ok_or_else(|| FFIError::InternalError("Token contract not found".to_string()))?
        } else {
// Deserialize the provided contract
let contract_slice = std::slice::from_raw_parts(
    params.serialized_contract,
    params.serialized_contract_len
);

use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;

DataContract::versioned_deserialize(
    contract_slice,
    false, // skip validation since it's already validated
    wrapper.sdk.version(),
)
.map_err(|e| FFIError::InternalError(format!("Failed to deserialize contract: {}", e)))?
        };

        // Create token purchase transition builder
        use dash_sdk::platform::transition::fungible_tokens::purchase::TokenDirectPurchaseTransitionBuilder;

        let mut builder = TokenDirectPurchaseTransitionBuilder::new(
&data_contract,
params.token_position,
purchaser_identity.id(),
params.amount,
params.total_agreed_price,
        );

        // Add settings and user fee increase
        if let Some(settings) = settings {
if let Some(fee_increase) = settings.user_fee_increase {
    builder = builder.with_user_fee_increase(fee_increase);
}
builder = builder.with_settings(settings);
        }

        // Sign the transition
        let state_transition = builder
.sign(
    &wrapper.sdk,
    identity_public_key,
    signer,
    wrapper.sdk.version(),
    settings.and_then(|s| s.state_transition_creation_options),
)
.await
.map_err(|e| FFIError::InternalError(format!("Failed to sign transition: {}", e)))?;

        // Serialize the state transition with bincode
        let config = bincode::config::standard();
        bincode::encode_to_vec(&state_transition, config).map_err(|e| {
FFIError::InternalError(format!("Failed to serialize state transition: {}", e))
        })
    });

    match result {
        Ok(serialized_data) => IOSSDKResult::success_binary(serialized_data),
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Set or update token price
///
/// # Parameters
/// - `issuer_identity_handle`: Identity handle of the token issuer
/// - `params`: Set price parameters
/// - `identity_public_key_handle`: Public key for signing
/// - `signer_handle`: Cryptographic signer
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
///
/// # Returns
/// Serialized state transition on success
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_token_set_price(
    sdk_handle: *mut SDKHandle,
    issuer_identity_handle: *const IdentityHandle,
    params: *const IOSSDKTokenSetPriceParams,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    put_settings: *const IOSSDKPutSettings,
) -> IOSSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || issuer_identity_handle.is_null()
        || params.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let issuer_identity = &*(issuer_identity_handle as *const Identity);
    let identity_public_key =
        &*(identity_public_key_handle as *const dpp::identity::IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);
    let params = &*params;

    // Validate that either contract ID or serialized contract is provided (but not both)
    let has_contract_id = !params.token_contract_id.is_null();
    let has_serialized_contract =
        !params.serialized_contract.is_null() && params.serialized_contract_len > 0;

    if !has_contract_id && !has_serialized_contract {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Either token contract ID or serialized contract must be provided".to_string(),
        ));
    }

    if has_contract_id && has_serialized_contract {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Cannot provide both token contract ID and serialized contract".to_string(),
        ));
    }

    // Parse optional public note
    let public_note = if params.public_note.is_null() {
        None
    } else {
        match CStr::from_ptr(params.public_note).to_str() {
            Ok(s) => Some(s.to_string()),
            Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
        }
    };

    let result: Result<Vec<u8>, FFIError> = wrapper.runtime.block_on(async {
        // Convert settings
        let settings = crate::identity::convert_put_settings(put_settings);

        use dash_sdk::platform::Fetch;
        use dpp::prelude::DataContract;
        use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;

        // Get the data contract either by fetching or deserializing
        let data_contract = if has_contract_id {
// Parse and fetch the contract ID
let token_contract_id_str = match CStr::from_ptr(params.token_contract_id).to_str() {
    Ok(s) => s,
    Err(e) => return Err(FFIError::from(e)),
};

let token_contract_id = match Identifier::from_string(token_contract_id_str, Encoding::Base58) {
    Ok(id) => id,
    Err(e) => {
        return Err(FFIError::InternalError(format!("Invalid token contract ID: {}", e)))
    }
};

// Fetch the data contract
DataContract::fetch(&wrapper.sdk, token_contract_id)
    .await
    .map_err(FFIError::from)?
    .ok_or_else(|| FFIError::InternalError("Token contract not found".to_string()))?
        } else {
// Deserialize the provided contract
let contract_slice = std::slice::from_raw_parts(
    params.serialized_contract,
    params.serialized_contract_len
);

use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;

DataContract::versioned_deserialize(
    contract_slice,
    false, // skip validation since it's already validated
    wrapper.sdk.version(),
)
.map_err(|e| FFIError::InternalError(format!("Failed to deserialize contract: {}", e)))?
        };

        // Build the pricing schedule based on the pricing type
        let pricing_schedule = match params.pricing_type {
IOSSDKTokenPricingType::SinglePrice => {
    Some(TokenPricingSchedule::SinglePrice(params.single_price))
}
IOSSDKTokenPricingType::SetPrices => {
    if params.price_entries.is_null() || params.price_entries_count == 0 {
        return Err(FFIError::InternalError("Price entries required for SetPrices".to_string()));
    }

    let price_entries_slice = std::slice::from_raw_parts(
        params.price_entries,
        params.price_entries_count as usize,
    );

    let mut price_map = std::collections::BTreeMap::new();
    for entry in price_entries_slice {
        price_map.insert(entry.amount, entry.price);
    }

    Some(TokenPricingSchedule::SetPrices(price_map))
}
        };

        // Create token set price transition builder
        use dash_sdk::platform::transition::fungible_tokens::set_price::TokenChangeDirectPurchasePriceTransitionBuilder;

        let mut builder = TokenChangeDirectPurchasePriceTransitionBuilder::new(
&data_contract,
params.token_position,
issuer_identity.id(),
pricing_schedule,
        );

        // Add optional public note
        if let Some(note) = public_note {
builder = builder.with_public_note(note);
        }

        // Add settings and user fee increase
        if let Some(settings) = settings {
if let Some(fee_increase) = settings.user_fee_increase {
    builder = builder.with_user_fee_increase(fee_increase);
}
builder = builder.with_settings(settings);
        }

        // Sign the transition
        let state_transition = builder
.sign(
    &wrapper.sdk,
    identity_public_key,
    signer,
    wrapper.sdk.version(),
    settings.and_then(|s| s.state_transition_creation_options),
)
.await
.map_err(|e| FFIError::InternalError(format!("Failed to sign transition: {}", e)))?;

        // Serialize the state transition with bincode
        let config = bincode::config::standard();
        bincode::encode_to_vec(&state_transition, config).map_err(|e| {
FFIError::InternalError(format!("Failed to serialize state transition: {}", e))
        })
    });

    match result {
        Ok(serialized_data) => IOSSDKResult::success_binary(serialized_data),
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Token IDs array parameter for batch token balance queries
#[repr(C)]
pub struct IOSSDKTokenIdsArray {
    /// Array of Base58-encoded token ID strings
    pub token_ids: *const *const c_char,
    /// Number of token IDs in the array
    pub count: u32,
}

/// Get token balances for an identity for specified token IDs
///
/// # Parameters
/// - `identity_id`: Base58-encoded identity ID
/// - `token_ids`: Array of Base58-encoded token IDs (pass null for all tokens)
///
/// # Returns
/// JSON string containing array of token balances (format: [{"token_id": "...", "balance": 123}, ...])
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_token_get_identity_balances(
    sdk_handle: *const SDKHandle,
    identity_id: *const c_char,
    token_ids: *const IOSSDKTokenIdsArray,
) -> IOSSDKResult {
    if sdk_handle.is_null() || identity_id.is_null() {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "SDK handle and identity ID are required".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let identity_id_str = match CStr::from_ptr(identity_id).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
    };

    let identity_identifier = match Identifier::from_string(identity_id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return IOSSDKResult::error(IOSSDKError::new(
                IOSSDKErrorCode::InvalidParameter,
                format!("Invalid identity ID: {}", e),
            ))
        }
    };

    // Parse token IDs array (if provided)
    let mut token_identifiers = Vec::new();
    if !token_ids.is_null() {
        let token_ids_array = &*token_ids;
        if !token_ids_array.token_ids.is_null() && token_ids_array.count > 0 {
            let token_id_ptrs = std::slice::from_raw_parts(
                token_ids_array.token_ids,
                token_ids_array.count as usize,
            );

            for &token_id_ptr in token_id_ptrs {
                if token_id_ptr.is_null() {
                    return IOSSDKResult::error(IOSSDKError::new(
                        IOSSDKErrorCode::InvalidParameter,
                        "Token ID in array is null".to_string(),
                    ));
                }

                let token_id_str = match CStr::from_ptr(token_id_ptr).to_str() {
                    Ok(s) => s,
                    Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
                };

                let token_identifier = match Identifier::from_string(token_id_str, Encoding::Base58)
                {
                    Ok(id) => id,
                    Err(e) => {
                        return IOSSDKResult::error(IOSSDKError::new(
                            IOSSDKErrorCode::InvalidParameter,
                            format!("Invalid token ID '{}': {}", token_id_str, e),
                        ))
                    }
                };

                token_identifiers.push(token_identifier);
            }
        }
    }

    let result: Result<String, FFIError> = wrapper.runtime.block_on(async {
        // Query token balances for the identity
        use dash_sdk::platform::tokens::identity_token_balances::{
            IdentityTokenBalances, IdentityTokenBalancesQuery,
        };
        use dash_sdk::platform::FetchMany;
        use dpp::balances::credits::TokenAmount;

        let query = IdentityTokenBalancesQuery {
            identity_id: identity_identifier,
            token_ids: token_identifiers, // Empty to get all tokens, specific IDs for targeted query
        };

        // Fetch token balances
        let token_balances: IdentityTokenBalances = TokenAmount::fetch_many(&wrapper.sdk, query)
            .await
            .map_err(|e| FFIError::InternalError(format!("Token balances query failed: {}", e)))?;

        // IdentityTokenBalances derefs to IndexMap<Identifier, Option<TokenAmount>>
        // where TokenAmount is u64

        let mut balance_objects = Vec::new();

        // Iterate over the token balances map
        for (token_id, balance_opt) in token_balances.iter() {
            // Convert token ID to Base58 string
            let token_id_str = token_id.to_string(Encoding::Base58);

            // Extract balance value (handle Option<u64>)
            match balance_opt {
                Some(balance) => {
                    balance_objects.push(format!(
                        r#"{{"token_id": "{}", "balance": {}}}"#,
                        token_id_str, balance
                    ));
                }
                None => {
                    // Token exists but has no balance (or proof of absence)
                    balance_objects.push(format!(
                        r#"{{"token_id": "{}", "balance": null}}"#,
                        token_id_str
                    ));
                }
            }
        }

        // Create JSON array of token balances
        let json_result = format!("[{}]", balance_objects.join(", "));
        Ok(json_result)
    });

    match result {
        Ok(json_str) => {
            let c_str = match CString::new(json_str) {
                Ok(s) => s,
                Err(e) => {
                    return IOSSDKResult::error(
                        FFIError::InternalError(format!("Failed to create CString: {}", e)).into(),
                    )
                }
            };
            IOSSDKResult::success_string(c_str.into_raw())
        }
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Get token information for an identity for specified token IDs
///
/// # Parameters
/// - `identity_id`: Base58-encoded identity ID
/// - `token_ids`: Array of Base58-encoded token IDs (pass null for all tokens)
///
/// # Returns
/// JSON string containing array of token information
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_token_get_identity_infos(
    sdk_handle: *const SDKHandle,
    identity_id: *const c_char,
    token_ids: *const IOSSDKTokenIdsArray,
) -> IOSSDKResult {
    if sdk_handle.is_null() || identity_id.is_null() {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "SDK handle and identity ID are required".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let identity_id_str = match CStr::from_ptr(identity_id).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
    };

    let identity_identifier = match Identifier::from_string(identity_id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return IOSSDKResult::error(IOSSDKError::new(
                IOSSDKErrorCode::InvalidParameter,
                format!("Invalid identity ID: {}", e),
            ))
        }
    };

    // Parse token IDs array (if provided)
    let mut token_identifiers = Vec::new();
    if !token_ids.is_null() {
        let token_ids_array = &*token_ids;
        if !token_ids_array.token_ids.is_null() && token_ids_array.count > 0 {
            let token_id_ptrs = std::slice::from_raw_parts(
                token_ids_array.token_ids,
                token_ids_array.count as usize,
            );

            for &token_id_ptr in token_id_ptrs {
                if token_id_ptr.is_null() {
                    return IOSSDKResult::error(IOSSDKError::new(
                        IOSSDKErrorCode::InvalidParameter,
                        "Token ID in array is null".to_string(),
                    ));
                }

                let token_id_str = match CStr::from_ptr(token_id_ptr).to_str() {
                    Ok(s) => s,
                    Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
                };

                let token_identifier = match Identifier::from_string(token_id_str, Encoding::Base58)
                {
                    Ok(id) => id,
                    Err(e) => {
                        return IOSSDKResult::error(IOSSDKError::new(
                            IOSSDKErrorCode::InvalidParameter,
                            format!("Invalid token ID '{}': {}", token_id_str, e),
                        ))
                    }
                };

                token_identifiers.push(token_identifier);
            }
        }
    }

    let result: Result<String, FFIError> = wrapper.runtime.block_on(async {
        // Query token information for the identity
        use dash_sdk::platform::tokens::token_info::IdentityTokenInfosQuery;
        use dash_sdk::platform::FetchMany;
        use dash_sdk::query_types::token_info::IdentityTokenInfos;
        use dpp::tokens::info::IdentityTokenInfo;

        let query = IdentityTokenInfosQuery {
            identity_id: identity_identifier,
            token_ids: token_identifiers, // Empty to get all tokens, specific IDs for targeted query
        };

        // Fetch token information
        let token_infos: IdentityTokenInfos = IdentityTokenInfo::fetch_many(&wrapper.sdk, query)
            .await
            .map_err(|e| FFIError::InternalError(format!("Token infos query failed: {}", e)))?;

        // Parse the IdentityTokenInfos structure and format as JSON manually
        let mut info_entries = Vec::new();

        // Iterate over the token information map
        for (token_id, token_info_opt) in token_infos.iter() {
            let token_id_str = token_id.to_string(Encoding::Base58);

            let entry = match token_info_opt {
                Some(_token_info) => {
                    // For now, create a simple representation of token info
                    // You may need to expand this based on the actual IdentityTokenInfo structure
                    format!(
                        r#"{{"token_id": "{}", "info": {{"available": true}}}}"#,
                        token_id_str
                    )
                }
                None => {
                    format!(r#"{{"token_id": "{}", "info": null}}"#, token_id_str)
                }
            };

            info_entries.push(entry);
        }

        let json_result = format!("[{}]", info_entries.join(", "));
        Ok(json_result)
    });

    match result {
        Ok(json_str) => {
            let c_str = match CString::new(json_str) {
                Ok(s) => s,
                Err(e) => {
                    return IOSSDKResult::error(
                        FFIError::InternalError(format!("Failed to create CString: {}", e)).into(),
                    )
                }
            };
            IOSSDKResult::success_string(c_str.into_raw())
        }
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Get token statuses for specified token IDs
///
/// # Parameters
/// - `token_ids`: Array of Base58-encoded token IDs
///
/// # Returns
/// JSON string containing array of token statuses
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_token_get_statuses(
    sdk_handle: *const SDKHandle,
    token_ids: *const IOSSDKTokenIdsArray,
) -> IOSSDKResult {
    if sdk_handle.is_null() || token_ids.is_null() {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "SDK handle and token IDs are required".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    // Parse token IDs array
    let mut token_identifiers = Vec::new();
    let token_ids_array = &*token_ids;
    if !token_ids_array.token_ids.is_null() && token_ids_array.count > 0 {
        let token_id_ptrs =
            std::slice::from_raw_parts(token_ids_array.token_ids, token_ids_array.count as usize);

        for &token_id_ptr in token_id_ptrs {
            if token_id_ptr.is_null() {
                return IOSSDKResult::error(IOSSDKError::new(
                    IOSSDKErrorCode::InvalidParameter,
                    "Token ID in array is null".to_string(),
                ));
            }

            let token_id_str = match CStr::from_ptr(token_id_ptr).to_str() {
                Ok(s) => s,
                Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
            };

            let token_identifier = match Identifier::from_string(token_id_str, Encoding::Base58) {
                Ok(id) => id,
                Err(e) => {
                    return IOSSDKResult::error(IOSSDKError::new(
                        IOSSDKErrorCode::InvalidParameter,
                        format!("Invalid token ID '{}': {}", token_id_str, e),
                    ))
                }
            };

            token_identifiers.push(token_identifier);
        }
    } else {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Token IDs array is empty or invalid".to_string(),
        ));
    }

    let result: Result<String, FFIError> = wrapper.runtime.block_on(async {
        // Query token statuses
        use dash_sdk::platform::FetchMany;
        use dash_sdk::query_types::token_status::TokenStatuses;
        use dpp::tokens::status::TokenStatus;

        // Fetch token statuses using Vec<Identifier> as the query
        let token_statuses: TokenStatuses =
            TokenStatus::fetch_many(&wrapper.sdk, token_identifiers)
                .await
                .map_err(|e| {
                    FFIError::InternalError(format!("Token statuses query failed: {}", e))
                })?;

        // Parse the TokenStatuses structure and format as JSON manually
        let mut status_entries = Vec::new();

        // Iterate over the token statuses map
        for (token_id, token_status_opt) in token_statuses.iter() {
            let token_id_str = token_id.to_string(Encoding::Base58);

            let entry = match token_status_opt {
                Some(_token_status) => {
                    // For now, create a simple representation of token status
                    // You may need to expand this based on the actual TokenStatus structure
                    format!(
                        r#"{{"token_id": "{}", "status": {{"available": true}}}}"#,
                        token_id_str
                    )
                }
                None => {
                    format!(r#"{{"token_id": "{}", "status": null}}"#, token_id_str)
                }
            };

            status_entries.push(entry);
        }

        let json_result = format!("[{}]", status_entries.join(", "));
        Ok(json_result)
    });

    match result {
        Ok(json_str) => {
            let c_str = match CString::new(json_str) {
                Ok(s) => s,
                Err(e) => {
                    return IOSSDKResult::error(
                        FFIError::InternalError(format!("Failed to create CString: {}", e)).into(),
                    )
                }
            };
            IOSSDKResult::success_string(c_str.into_raw())
        }
        Err(e) => IOSSDKResult::error(e.into()),
    }
}
