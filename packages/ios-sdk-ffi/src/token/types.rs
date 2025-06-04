//! Common types for token operations

use std::os::raw::c_char;

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
    /// Recipient identity ID (32 bytes)
    pub recipient_id: *const u8,
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
    /// Recipient identity ID (32 bytes) - optional
    pub recipient_id: *const u8,
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
#[derive(Copy, Clone)]
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
    /// For identity-based updates - identity ID (32 bytes)
    pub identity_id: *const u8,
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
    /// The frozen identity whose funds to destroy (32 bytes)
    pub frozen_identity_id: *const u8,
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
    /// The identity to freeze/unfreeze (32 bytes)
    pub target_identity_id: *const u8,
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

/// Token IDs array parameter for batch token balance queries
#[repr(C)]
pub struct IOSSDKTokenIdsArray {
    /// Array of Base58-encoded token ID strings
    pub token_ids: *const *const c_char,
    /// Number of token IDs in the array
    pub count: u32,
}
