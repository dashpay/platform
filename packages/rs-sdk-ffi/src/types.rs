//! Common types used across the FFI boundary

use std::os::raw::{c_char, c_void};

/// Opaque handle to an SDK instance
pub struct SDKHandle {
    _private: [u8; 0],
}

/// Opaque handle to an Identity
pub struct IdentityHandle {
    _private: [u8; 0],
}

/// Opaque handle to a Document
pub struct DocumentHandle {
    _private: [u8; 0],
}

/// Opaque handle to a DataContract
pub struct DataContractHandle {
    _private: [u8; 0],
}

/// Opaque handle to a Signer
pub struct SignerHandle {
    _private: [u8; 0],
}

/// Opaque handle to an IdentityPublicKey
pub struct IdentityPublicKeyHandle {
    _private: [u8; 0],
}

/// Alias for compatibility
pub type DashSDKPublicKeyHandle = IdentityPublicKeyHandle;

/// Network type for SDK configuration
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashSDKNetwork {
    /// Mainnet
    SDKMainnet = 0,
    /// Testnet
    SDKTestnet = 1,
    /// Regtest
    SDKRegtest = 2,
    /// Devnet
    SDKDevnet = 3,
    /// Local development network
    SDKLocal = 4,
}

/// SDK configuration
#[repr(C)]
#[derive(Copy, Clone)]
pub struct DashSDKConfig {
    /// Network to connect to
    pub network: DashSDKNetwork,
    /// Comma-separated list of DAPI addresses (e.g., "http://127.0.0.1:3000,http://127.0.0.1:3001")
    /// If null or empty, will use mock SDK
    pub dapi_addresses: *const c_char,
    /// Skip asset lock proof verification (for testing)
    pub skip_asset_lock_proof_verification: bool,
    /// Number of retries for failed requests
    pub request_retry_count: u32,
    /// Timeout for requests in milliseconds
    pub request_timeout_ms: u64,
}

/// Result data type indicator for iOS
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashSDKResultDataType {
    /// No data (void/null)
    NoData = 0,
    /// C string (char*)
    String = 1,
    /// Binary data with length
    BinaryData = 2,
    /// Identity handle
    ResultIdentityHandle = 3,
    /// Document handle
    ResultDocumentHandle = 4,
    /// Data contract handle
    ResultDataContractHandle = 5,
    /// Map of identity IDs to balances
    IdentityBalanceMap = 6,
    /// Public key handle
    ResultPublicKeyHandle = 7,
    /// Address info (single address with balance and nonce)
    AddressInfo = 8,
    /// Map of addresses to their info
    AddressInfoMap = 9,
    /// Trunk state for address synchronization
    TrunkState = 10,
    /// Branch state for address synchronization
    BranchState = 11,
    /// Recent address balance changes
    RecentBalanceChanges = 12,
    /// Recent compacted address balance changes
    CompactedBalanceChanges = 16,
    /// Identity top-up from addresses result
    IdentityTopUpFromAddressesResult = 13,
    /// Identity transfer to addresses result
    IdentityTransferToAddressesResult = 14,
    /// Identity create from addresses result
    IdentityCreateFromAddressesResult = 15,
}

/// Binary data container for results
#[repr(C)]
pub struct DashSDKBinaryData {
    /// Pointer to the data
    pub data: *mut u8,
    /// Length of the data
    pub len: usize,
}

/// Single entry in an identity balance map
#[repr(C)]
pub struct DashSDKIdentityBalanceEntry {
    /// Identity ID (32 bytes)
    pub identity_id: [u8; 32],
    /// Balance in credits (u64::MAX means identity not found)
    pub balance: u64,
}

/// Map of identity IDs to balances
#[repr(C)]
pub struct DashSDKIdentityBalanceMap {
    /// Array of entries
    pub entries: *mut DashSDKIdentityBalanceEntry,
    /// Number of entries
    pub count: usize,
}

/// Information about a Platform address including its nonce and balance
#[repr(C)]
pub struct DashSDKAddressInfo {
    /// Address bytes (variable length, typically 20-32 bytes)
    pub address: *mut u8,
    /// Length of address bytes
    pub address_len: usize,
    /// Nonce associated with the address (u32::MAX means address not found)
    pub nonce: u32,
    /// Balance in credits (u64::MAX means address not found)
    pub balance: u64,
}

/// Single entry in an address info map
#[repr(C)]
pub struct DashSDKAddressInfoEntry {
    /// Address bytes (variable length, typically 20-32 bytes)
    pub address: *mut u8,
    /// Length of address bytes
    pub address_len: usize,
    /// Nonce associated with the address (u32::MAX means address not found)
    pub nonce: u32,
    /// Balance in credits (u64::MAX means address not found)
    pub balance: u64,
}

/// Map of addresses to their info
#[repr(C)]
pub struct DashSDKAddressInfoMap {
    /// Array of entries
    pub entries: *mut DashSDKAddressInfoEntry,
    /// Number of entries
    pub count: usize,
}

/// Single element in trunk state (address with balance/nonce)
#[repr(C)]
pub struct DashSDKTrunkStateElement {
    /// Address key bytes
    pub key: *mut u8,
    /// Length of key bytes
    pub key_len: usize,
    /// Nonce associated with the address
    pub nonce: u32,
    /// Balance in credits
    pub balance: u64,
}

/// Leaf boundary in trunk state (subtree that needs further queries)
#[repr(C)]
pub struct DashSDKLeafBoundary {
    /// Leaf key bytes
    pub key: *mut u8,
    /// Length of key bytes
    pub key_len: usize,
    /// Expected hash (32 bytes)
    pub hash: [u8; 32],
    /// Estimated element count in this subtree (0 if unknown)
    pub estimated_count: u64,
}

/// Trunk state for address synchronization
#[repr(C)]
pub struct DashSDKTrunkState {
    /// Array of elements (addresses with balances found at trunk level)
    pub elements: *mut DashSDKTrunkStateElement,
    /// Number of elements
    pub elements_count: usize,
    /// Array of leaf boundaries (subtrees needing branch queries)
    pub leaf_boundaries: *mut DashSDKLeafBoundary,
    /// Number of leaf boundaries
    pub leaf_boundaries_count: usize,
    /// Checkpoint height for consistency
    pub checkpoint_height: u64,
}

/// Branch state for address synchronization (result of branch query)
#[repr(C)]
pub struct DashSDKBranchState {
    /// Array of elements (addresses with balances found in this branch)
    pub elements: *mut DashSDKTrunkStateElement,
    /// Number of elements
    pub elements_count: usize,
    /// Array of leaf boundaries (deeper subtrees needing further queries)
    pub leaf_boundaries: *mut DashSDKLeafBoundary,
    /// Number of leaf boundaries
    pub leaf_boundaries_count: usize,
}

/// Credit operation type
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum DashSDKCreditOperationType {
    /// Setting credits to a value
    SetCredits = 0,
    /// Adding to credits
    AddToCredits = 1,
}

/// A single balance change for an address
#[repr(C)]
pub struct DashSDKAddressBalanceChange {
    /// Address bytes
    pub address: *mut u8,
    /// Length of address bytes
    pub address_len: usize,
    /// Operation type
    pub operation_type: DashSDKCreditOperationType,
    /// Credit amount
    pub credits: u64,
}

/// Balance changes for a single block
#[repr(C)]
pub struct DashSDKBlockBalanceChanges {
    /// Block height
    pub block_height: u64,
    /// Array of balance changes
    pub changes: *mut DashSDKAddressBalanceChange,
    /// Number of changes
    pub changes_count: usize,
}

/// Recent address balance changes across multiple blocks
#[repr(C)]
pub struct DashSDKRecentBalanceChanges {
    /// Array of block balance changes
    pub blocks: *mut DashSDKBlockBalanceChanges,
    /// Number of blocks
    pub blocks_count: usize,
}

/// Block-aware credit operation type for compacted balance changes
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum DashSDKBlockAwareCreditOperationType {
    /// Set credits to a final value
    BlockAwareSetCredits = 0,
    /// Add to credits with block height entries
    BlockAwareAddToCreditsOperations = 1,
}

/// Entry for block height to credits mapping
#[repr(C)]
pub struct DashSDKBlockHeightCreditEntry {
    /// Block height
    pub block_height: u64,
    /// Credit amount
    pub credits: u64,
}

/// A compacted balance change for an address (supports block-aware operations)
#[repr(C)]
pub struct DashSDKCompactedAddressChange {
    /// Address bytes
    pub address: *mut u8,
    /// Length of address bytes
    pub address_len: usize,
    /// Operation type
    pub operation_type: DashSDKBlockAwareCreditOperationType,
    /// For SetCredits: the final value; for AddToCreditsOperations: ignored (use entries)
    pub set_credits_value: u64,
    /// For AddToCreditsOperations: array of block height/credit entries
    pub add_entries: *mut DashSDKBlockHeightCreditEntry,
    /// Number of entries (0 for SetCredits)
    pub add_entries_count: usize,
}

/// Compacted balance changes for a range of blocks
#[repr(C)]
pub struct DashSDKCompactedBlockRange {
    /// Start block height of the range
    pub start_block_height: u64,
    /// End block height of the range
    pub end_block_height: u64,
    /// Array of address changes
    pub changes: *mut DashSDKCompactedAddressChange,
    /// Number of changes
    pub changes_count: usize,
}

/// Recent compacted address balance changes across multiple ranges
#[repr(C)]
pub struct DashSDKCompactedBalanceChanges {
    /// Array of compacted block ranges
    pub ranges: *mut DashSDKCompactedBlockRange,
    /// Number of ranges
    pub ranges_count: usize,
}

// MARK: - Address State Transition Types

/// Input entry for address transfer (address with amount and private key)
#[repr(C)]
pub struct DashSDKAddressTransferInput {
    /// Address bytes (variable length, typically 21 bytes: 1 type + 20 hash)
    pub address: *const u8,
    /// Length of address bytes
    pub address_len: usize,
    /// Amount to spend from this address
    pub amount: u64,
    /// Nonce for this address (0 = auto-fetch)
    pub nonce: u32,
    /// Private key for signing (32 bytes)
    pub private_key: *const u8,
}

/// Output entry for address transfer (address with amount)
#[repr(C)]
pub struct DashSDKAddressTransferOutput {
    /// Address bytes (variable length, typically 21 bytes: 1 type + 20 hash)
    pub address: *const u8,
    /// Length of address bytes
    pub address_len: usize,
    /// Amount to receive at this address
    pub amount: u64,
}

/// Pooling strategy for withdrawals
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashSDKPooling {
    /// Never pool withdrawals
    Never = 0,
    /// Pool if available
    IfAvailable = 1,
    /// Standard pooling
    Standard = 2,
}

/// Asset lock proof type
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashSDKAssetLockProofType {
    /// Instant lock proof
    Instant = 0,
    /// Chain lock proof
    Chain = 1,
}

/// Result type for FFI functions that return data
#[repr(C)]
pub struct DashSDKResult {
    /// Type of data being returned
    pub data_type: DashSDKResultDataType,
    /// Pointer to the result data (null on error)
    pub data: *mut c_void,
    /// Error information (null on success)
    pub error: *mut super::DashSDKError,
}

impl DashSDKResult {
    /// Create a success result (backward compatibility - assumes no data type)
    pub fn success(data: *mut c_void) -> Self {
        DashSDKResult {
            data_type: DashSDKResultDataType::NoData,
            data,
            error: std::ptr::null_mut(),
        }
    }

    /// Create a success result with string data
    pub fn success_string(data: *mut c_char) -> Self {
        DashSDKResult {
            data_type: DashSDKResultDataType::String,
            data: data as *mut c_void,
            error: std::ptr::null_mut(),
        }
    }

    /// Create a success result with binary data
    pub fn success_binary(data: Vec<u8>) -> Self {
        let len = data.len();
        let data_ptr = data.as_ptr() as *mut u8;
        std::mem::forget(data); // Prevent deallocation

        let binary_data = Box::new(DashSDKBinaryData {
            data: data_ptr,
            len,
        });

        DashSDKResult {
            data_type: DashSDKResultDataType::BinaryData,
            data: Box::into_raw(binary_data) as *mut c_void,
            error: std::ptr::null_mut(),
        }
    }

    /// Create a success result with a handle
    pub fn success_handle(handle: *mut c_void, handle_type: DashSDKResultDataType) -> Self {
        DashSDKResult {
            data_type: handle_type,
            data: handle,
            error: std::ptr::null_mut(),
        }
    }

    /// Create a success result with an identity balance map
    pub fn success_identity_balance_map(map: DashSDKIdentityBalanceMap) -> Self {
        DashSDKResult {
            data_type: DashSDKResultDataType::IdentityBalanceMap,
            data: Box::into_raw(Box::new(map)) as *mut c_void,
            error: std::ptr::null_mut(),
        }
    }

    /// Create a success result with address info
    pub fn success_address_info(info: DashSDKAddressInfo) -> Self {
        DashSDKResult {
            data_type: DashSDKResultDataType::AddressInfo,
            data: Box::into_raw(Box::new(info)) as *mut c_void,
            error: std::ptr::null_mut(),
        }
    }

    /// Create a success result with an address info map
    pub fn success_address_info_map(map: DashSDKAddressInfoMap) -> Self {
        DashSDKResult {
            data_type: DashSDKResultDataType::AddressInfoMap,
            data: Box::into_raw(Box::new(map)) as *mut c_void,
            error: std::ptr::null_mut(),
        }
    }

    /// Create a success result with trunk state
    pub fn success_trunk_state(state: DashSDKTrunkState) -> Self {
        DashSDKResult {
            data_type: DashSDKResultDataType::TrunkState,
            data: Box::into_raw(Box::new(state)) as *mut c_void,
            error: std::ptr::null_mut(),
        }
    }

    /// Create a success result with branch state
    pub fn success_branch_state(state: DashSDKBranchState) -> Self {
        DashSDKResult {
            data_type: DashSDKResultDataType::BranchState,
            data: Box::into_raw(Box::new(state)) as *mut c_void,
            error: std::ptr::null_mut(),
        }
    }

    /// Create a success result with recent balance changes
    pub fn success_recent_balance_changes(changes: DashSDKRecentBalanceChanges) -> Self {
        DashSDKResult {
            data_type: DashSDKResultDataType::RecentBalanceChanges,
            data: Box::into_raw(Box::new(changes)) as *mut c_void,
            error: std::ptr::null_mut(),
        }
    }

    /// Create a success result with compacted balance changes
    pub fn success_compacted_balance_changes(changes: DashSDKCompactedBalanceChanges) -> Self {
        DashSDKResult {
            data_type: DashSDKResultDataType::CompactedBalanceChanges,
            data: Box::into_raw(Box::new(changes)) as *mut c_void,
            error: std::ptr::null_mut(),
        }
    }

    /// Create an error result
    pub fn error(error: super::DashSDKError) -> Self {
        DashSDKResult {
            data_type: DashSDKResultDataType::NoData,
            data: std::ptr::null_mut(),
            error: Box::into_raw(Box::new(error)),
        }
    }
}

/// Identity information
#[repr(C)]
pub struct DashSDKIdentityInfo {
    /// Identity ID as hex string (null-terminated)
    pub id: *mut c_char,
    /// Balance in credits
    pub balance: u64,
    /// Revision number
    pub revision: u64,
    /// Public keys count
    pub public_keys_count: u32,
}

/// Document field value types
#[repr(C)]
pub enum DashSDKDocumentFieldType {
    FieldString = 0,
    FieldInteger = 1,
    FieldFloat = 2,
    FieldBoolean = 3,
    FieldBytes = 4,
    FieldArray = 5,
    FieldObject = 6,
    FieldNull = 7,
}

/// Document field value
#[repr(C)]
pub struct DashSDKDocumentField {
    /// Field name (null-terminated)
    pub name: *mut c_char,
    /// Field type
    pub field_type: DashSDKDocumentFieldType,
    /// Field value as string representation (null-terminated)
    /// For complex types, this will be JSON-encoded
    pub value: *mut c_char,
    /// Raw integer value (for Integer type)
    pub int_value: i64,
    /// Raw float value (for Float type)
    pub float_value: f64,
    /// Raw boolean value (for Boolean type)
    pub bool_value: bool,
}

/// Document information
#[repr(C)]
pub struct DashSDKDocumentInfo {
    /// Document ID as hex string (null-terminated)
    pub id: *mut c_char,
    /// Owner ID as hex string (null-terminated)
    pub owner_id: *mut c_char,
    /// Data contract ID as hex string (null-terminated)
    pub data_contract_id: *mut c_char,
    /// Document type (null-terminated)
    pub document_type: *mut c_char,
    /// Revision number
    pub revision: u64,
    /// Created at timestamp (milliseconds since epoch)
    pub created_at: i64,
    /// Updated at timestamp (milliseconds since epoch)
    pub updated_at: i64,
    /// Number of data fields
    pub data_fields_count: usize,
    /// Array of data fields
    pub data_fields: *mut DashSDKDocumentField,
}

/// Put settings for platform operations
#[repr(C)]
pub struct DashSDKPutSettings {
    /// Timeout for establishing a connection (milliseconds), 0 means use default
    pub connect_timeout_ms: u64,
    /// Timeout for single request (milliseconds), 0 means use default
    pub timeout_ms: u64,
    /// Number of retries in case of failed requests, 0 means use default
    pub retries: u32,
    /// Ban DAPI address if node not responded or responded with error
    pub ban_failed_address: bool,
    /// Identity nonce stale time in seconds, 0 means use default
    pub identity_nonce_stale_time_s: u64,
    /// User fee increase (additional percentage of processing fee), 0 means no increase
    pub user_fee_increase: u16,
    /// Enable signing with any security level (for debugging)
    pub allow_signing_with_any_security_level: bool,
    /// Enable signing with any purpose (for debugging)
    pub allow_signing_with_any_purpose: bool,
    /// Wait timeout in milliseconds, 0 means use default
    pub wait_timeout_ms: u64,
}

/// Gas fees payer option
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DashSDKGasFeesPaidBy {
    /// The document owner pays the gas fees
    DocumentOwner = 0,
    /// The contract owner pays the gas fees
    GasFeesContractOwner = 1,
    /// Prefer contract owner but fallback to document owner if insufficient balance
    GasFeesPreferContractOwner = 2,
}

/// Token payment information for transactions
#[repr(C)]
pub struct DashSDKTokenPaymentInfo {
    /// Payment token contract ID (32 bytes), null for same contract
    pub payment_token_contract_id: *const [u8; 32],
    /// Token position within the contract (0-based index)
    pub token_contract_position: u16,
    /// Minimum token cost (0 means no minimum)
    pub minimum_token_cost: u64,
    /// Maximum token cost (0 means no maximum)
    pub maximum_token_cost: u64,
    /// Who pays the gas fees
    pub gas_fees_paid_by: DashSDKGasFeesPaidBy,
}

/// State transition creation options for advanced use cases
#[repr(C)]
pub struct DashSDKStateTransitionCreationOptions {
    /// Allow signing with any security level (for debugging)
    pub allow_signing_with_any_security_level: bool,
    /// Allow signing with any purpose (for debugging)
    pub allow_signing_with_any_purpose: bool,
    /// Batch feature version (0 means use default)
    pub batch_feature_version: u16,
    /// Method feature version (0 means use default)
    pub method_feature_version: u16,
    /// Base feature version (0 means use default)
    pub base_feature_version: u16,
}

/// Free a string allocated by the FFI
///
/// # Safety
/// - `s` must be a pointer returned by this SDK to a heap-allocated NUL-terminated C string.
/// - Passing a pointer not allocated by this SDK, or a pointer already freed, results in undefined behavior.
/// - `s` may be null, in which case this is a no-op.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_string_free(s: *mut c_char) {
    if !s.is_null() {
        let _ = std::ffi::CString::from_raw(s);
    }
}

/// Free binary data allocated by the FFI
///
/// # Safety
/// - `binary_data` must be a valid pointer returned by this SDK and not previously freed.
/// - When non-null, the function takes ownership and frees both the struct and its internal buffer.
/// - Do not use `binary_data` after this call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_binary_data_free(binary_data: *mut DashSDKBinaryData) {
    if binary_data.is_null() {
        return;
    }

    let data = Box::from_raw(binary_data);
    if !data.data.is_null() && data.len > 0 {
        // Reconstruct the Vec to properly deallocate
        let _ = Vec::from_raw_parts(data.data, data.len, data.len);
    }
}

/// Free an identity info structure
///
/// # Safety
/// - `info` must be a valid pointer to `DashSDKIdentityInfo` allocated by this SDK.
/// - It may be null (no-op). When non-null, this frees any owned strings and the struct.
/// - Do not access `info` after this call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_info_free(info: *mut DashSDKIdentityInfo) {
    if info.is_null() {
        return;
    }

    let info = Box::from_raw(info);
    dash_sdk_string_free(info.id);
}

/// Free a document info structure
///
/// # Safety
/// - `info` must be a valid pointer to `DashSDKDocumentInfo` allocated by this SDK and not already freed.
/// - It may be null (no-op). When non-null, this frees all owned strings and arrays.
/// - Pointer must not be dereferenced after this call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_info_free(info: *mut DashSDKDocumentInfo) {
    if info.is_null() {
        return;
    }

    let info = Box::from_raw(info);

    // Free string fields
    dash_sdk_string_free(info.id);
    dash_sdk_string_free(info.owner_id);
    dash_sdk_string_free(info.data_contract_id);
    dash_sdk_string_free(info.document_type);

    // Free data fields
    if !info.data_fields.is_null() && info.data_fields_count > 0 {
        for i in 0..info.data_fields_count {
            let field = info.data_fields.add(i);
            dash_sdk_string_free((*field).name);
            dash_sdk_string_free((*field).value);
        }
        let _ = Vec::from_raw_parts(
            info.data_fields,
            info.data_fields_count,
            info.data_fields_count,
        );
    }
}

/// Free an identity balance map
///
/// # Safety
/// - `map` must be a valid, non-dangling pointer returned by this SDK.
/// - It may be null (no-op). When non-null, this frees the entries array and the struct.
/// - Using `map` after this function returns is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_balance_map_free(map: *mut DashSDKIdentityBalanceMap) {
    if map.is_null() {
        return;
    }

    let map = Box::from_raw(map);
    if !map.entries.is_null() && map.count > 0 {
        // Free the entries array
        let _ = Vec::from_raw_parts(map.entries, map.count, map.count);
    }
}

/// Free an address info structure
///
/// # Safety
/// - `info` must be a valid pointer to `DashSDKAddressInfo` allocated by this SDK.
/// - It may be null (no-op). When non-null, this frees the address bytes and the struct.
/// - Do not access `info` after this call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_address_info_free(info: *mut DashSDKAddressInfo) {
    if info.is_null() {
        return;
    }

    let info = Box::from_raw(info);
    if !info.address.is_null() && info.address_len > 0 {
        let _ = Vec::from_raw_parts(info.address, info.address_len, info.address_len);
    }
}

/// Free an address info map
///
/// # Safety
/// - `map` must be a valid, non-dangling pointer returned by this SDK.
/// - It may be null (no-op). When non-null, this frees all entries, their address bytes, and the struct.
/// - Using `map` after this function returns is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_address_info_map_free(map: *mut DashSDKAddressInfoMap) {
    if map.is_null() {
        return;
    }

    let map = Box::from_raw(map);
    if !map.entries.is_null() && map.count > 0 {
        let entries_slice = std::slice::from_raw_parts_mut(map.entries, map.count);
        for entry in entries_slice.iter() {
            if !entry.address.is_null() && entry.address_len > 0 {
                let _ = Vec::from_raw_parts(entry.address, entry.address_len, entry.address_len);
            }
        }
        let _ = Vec::from_raw_parts(map.entries, map.count, map.count);
    }
}

/// Free a trunk state structure
///
/// # Safety
/// - `state` must be a valid pointer to `DashSDKTrunkState` allocated by this SDK.
/// - It may be null (no-op). When non-null, this frees all elements, leaf boundaries, and the struct.
/// - Do not access `state` after this call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_trunk_state_free(state: *mut DashSDKTrunkState) {
    if state.is_null() {
        return;
    }

    let state = Box::from_raw(state);

    // Free elements
    if !state.elements.is_null() && state.elements_count > 0 {
        let elements_slice = std::slice::from_raw_parts_mut(state.elements, state.elements_count);
        for element in elements_slice.iter() {
            if !element.key.is_null() && element.key_len > 0 {
                let _ = Vec::from_raw_parts(element.key, element.key_len, element.key_len);
            }
        }
        let _ = Vec::from_raw_parts(state.elements, state.elements_count, state.elements_count);
    }

    // Free leaf boundaries
    if !state.leaf_boundaries.is_null() && state.leaf_boundaries_count > 0 {
        let boundaries_slice =
            std::slice::from_raw_parts_mut(state.leaf_boundaries, state.leaf_boundaries_count);
        for boundary in boundaries_slice.iter() {
            if !boundary.key.is_null() && boundary.key_len > 0 {
                let _ = Vec::from_raw_parts(boundary.key, boundary.key_len, boundary.key_len);
            }
        }
        let _ = Vec::from_raw_parts(
            state.leaf_boundaries,
            state.leaf_boundaries_count,
            state.leaf_boundaries_count,
        );
    }
}

/// Free a branch state structure
///
/// # Safety
/// - `state` must be a valid pointer to `DashSDKBranchState` allocated by this SDK.
/// - It may be null (no-op). When non-null, this frees all elements, leaf boundaries, and the struct.
/// - Do not access `state` after this call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_branch_state_free(state: *mut DashSDKBranchState) {
    if state.is_null() {
        return;
    }

    let state = Box::from_raw(state);

    // Free elements
    if !state.elements.is_null() && state.elements_count > 0 {
        let elements_slice = std::slice::from_raw_parts_mut(state.elements, state.elements_count);
        for element in elements_slice.iter() {
            if !element.key.is_null() && element.key_len > 0 {
                let _ = Vec::from_raw_parts(element.key, element.key_len, element.key_len);
            }
        }
        let _ = Vec::from_raw_parts(state.elements, state.elements_count, state.elements_count);
    }

    // Free leaf boundaries
    if !state.leaf_boundaries.is_null() && state.leaf_boundaries_count > 0 {
        let boundaries_slice =
            std::slice::from_raw_parts_mut(state.leaf_boundaries, state.leaf_boundaries_count);
        for boundary in boundaries_slice.iter() {
            if !boundary.key.is_null() && boundary.key_len > 0 {
                let _ = Vec::from_raw_parts(boundary.key, boundary.key_len, boundary.key_len);
            }
        }
        let _ = Vec::from_raw_parts(
            state.leaf_boundaries,
            state.leaf_boundaries_count,
            state.leaf_boundaries_count,
        );
    }
}

/// Free a recent balance changes structure
///
/// # Safety
/// - `changes` must be a valid pointer to `DashSDKRecentBalanceChanges` allocated by this SDK.
/// - It may be null (no-op). When non-null, this frees all blocks, changes, addresses, and the struct.
/// - Do not access `changes` after this call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_recent_balance_changes_free(
    changes: *mut DashSDKRecentBalanceChanges,
) {
    if changes.is_null() {
        return;
    }

    let changes = Box::from_raw(changes);

    // Free blocks
    if !changes.blocks.is_null() && changes.blocks_count > 0 {
        let blocks_slice = std::slice::from_raw_parts_mut(changes.blocks, changes.blocks_count);
        for block in blocks_slice.iter() {
            // Free changes within each block
            if !block.changes.is_null() && block.changes_count > 0 {
                let changes_slice =
                    std::slice::from_raw_parts_mut(block.changes, block.changes_count);
                for change in changes_slice.iter() {
                    if !change.address.is_null() && change.address_len > 0 {
                        let _ = Vec::from_raw_parts(
                            change.address,
                            change.address_len,
                            change.address_len,
                        );
                    }
                }
                let _ =
                    Vec::from_raw_parts(block.changes, block.changes_count, block.changes_count);
            }
        }
        let _ = Vec::from_raw_parts(changes.blocks, changes.blocks_count, changes.blocks_count);
    }
}

/// Free a compacted balance changes structure
///
/// # Safety
/// - `changes` must be a valid pointer to `DashSDKCompactedBalanceChanges` allocated by this SDK.
/// - It may be null (no-op). When non-null, this frees all ranges, changes, addresses, entries, and the struct.
/// - Do not access `changes` after this call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_compacted_balance_changes_free(
    changes: *mut DashSDKCompactedBalanceChanges,
) {
    if changes.is_null() {
        return;
    }

    let changes = Box::from_raw(changes);

    // Free ranges
    if !changes.ranges.is_null() && changes.ranges_count > 0 {
        let ranges_slice = std::slice::from_raw_parts_mut(changes.ranges, changes.ranges_count);
        for range in ranges_slice.iter() {
            // Free changes within each range
            if !range.changes.is_null() && range.changes_count > 0 {
                let changes_slice =
                    std::slice::from_raw_parts_mut(range.changes, range.changes_count);
                for change in changes_slice.iter() {
                    // Free address
                    if !change.address.is_null() && change.address_len > 0 {
                        let _ = Vec::from_raw_parts(
                            change.address,
                            change.address_len,
                            change.address_len,
                        );
                    }
                    // Free add entries
                    if !change.add_entries.is_null() && change.add_entries_count > 0 {
                        let _ = Vec::from_raw_parts(
                            change.add_entries,
                            change.add_entries_count,
                            change.add_entries_count,
                        );
                    }
                }
                let _ =
                    Vec::from_raw_parts(range.changes, range.changes_count, range.changes_count);
            }
        }
        let _ = Vec::from_raw_parts(changes.ranges, changes.ranges_count, changes.ranges_count);
    }
}

// DPNS Contested structures

/// Represents a contender in a contested DPNS name
#[repr(C)]
pub struct DashSDKContender {
    /// Identity ID of the contender (base58 string)
    pub identity_id: *mut c_char,
    /// Vote count for this contender
    pub vote_count: u32,
}

/// Represents contest information for a DPNS name
#[repr(C)]
pub struct DashSDKContestInfo {
    /// Array of contenders
    pub contenders: *mut DashSDKContender,
    /// Number of contenders
    pub contender_count: usize,
    /// Abstain vote tally (0 if none)
    pub abstain_votes: u32,
    /// Lock vote tally (0 if none)
    pub lock_votes: u32,
    /// End time in milliseconds since epoch
    pub end_time: u64,
    /// Whether there is a winner
    pub has_winner: bool,
}

/// Represents a contested DPNS name entry
#[repr(C)]
pub struct DashSDKContestedName {
    /// The contested name
    pub name: *mut c_char,
    /// Contest information
    pub contest_info: DashSDKContestInfo,
}

/// Represents a list of contested names
#[repr(C)]
pub struct DashSDKContestedNamesList {
    /// Array of contested names
    pub names: *mut DashSDKContestedName,
    /// Number of names
    pub count: usize,
}

/// Represents a simple name to timestamp mapping
#[repr(C)]
pub struct DashSDKNameTimestamp {
    /// The name
    pub name: *mut c_char,
    /// End timestamp in milliseconds
    pub end_time: u64,
}

/// Represents a list of name-timestamp pairs
#[repr(C)]
pub struct DashSDKNameTimestampList {
    /// Array of name-timestamp pairs
    pub entries: *mut DashSDKNameTimestamp,
    /// Number of entries
    pub count: usize,
}

/// Free a contender structure
///
/// # Safety
/// - `contender` must be a valid, non-dangling pointer obtained from this FFI (e.g., via an SDK function).
/// - It must either be null (a no-op) or point to a heap-allocated `DashSDKContender` that has not been freed yet.
/// - After this call, the pointer must not be used again (use-after-free is undefined behavior).
/// - This function will also free any heap-allocated strings owned by the structure.
/// # Safety
/// - `contender` must be a valid, non-null pointer to a `DashSDKContender` allocated by this SDK, or null for no-op.
/// - The pointer must not be used after this call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_contender_free(contender: *mut DashSDKContender) {
    if contender.is_null() {
        return;
    }

    let contender = Box::from_raw(contender);
    dash_sdk_string_free(contender.identity_id);
}

/// Free contest info structure
///
/// # Safety
/// - `info` must be a valid, non-dangling pointer obtained from this FFI and not previously freed.
/// - It may be null (no-op). When non-null, this frees the owned contender array and contained strings.
/// - Do not use `info` after this call; doing so is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_contest_info_free(info: *mut DashSDKContestInfo) {
    if info.is_null() {
        return;
    }

    let info = Box::from_raw(info);
    if !info.contenders.is_null() && info.contender_count > 0 {
        for i in 0..info.contender_count {
            let contender = info.contenders.add(i);
            dash_sdk_string_free((*contender).identity_id);
        }
        let _ = Vec::from_raw_parts(info.contenders, info.contender_count, info.contender_count);
    }
}

/// Free a contested name structure
///
/// # Safety
/// - `name` must be a valid, non-dangling pointer to a `DashSDKContestedName` allocated by this SDK.
/// - It may be null (no-op). When non-null, this frees the embedded strings and contender buffers.
/// - Do not access `name` after freeing.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_contested_name_free(name: *mut DashSDKContestedName) {
    if name.is_null() {
        return;
    }

    let name = Box::from_raw(name);
    dash_sdk_string_free(name.name);

    // Free contest info contents (but not the struct itself as it's embedded)
    if !name.contest_info.contenders.is_null() && name.contest_info.contender_count > 0 {
        for i in 0..name.contest_info.contender_count {
            let contender = name.contest_info.contenders.add(i);
            dash_sdk_string_free((*contender).identity_id);
        }
        let _ = Vec::from_raw_parts(
            name.contest_info.contenders,
            name.contest_info.contender_count,
            name.contest_info.contender_count,
        );
    }
}

/// Free a contested names list
///
/// # Safety
/// - `list` must be a valid pointer returned by this SDK and not previously freed.
/// - It may be null (no-op). When non-null, this frees the array of names and any nested strings/buffers.
/// - Do not use `list` after this call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_contested_names_list_free(list: *mut DashSDKContestedNamesList) {
    if list.is_null() {
        return;
    }

    let list = Box::from_raw(list);
    if !list.names.is_null() && list.count > 0 {
        for i in 0..list.count {
            let name = list.names.add(i);
            dash_sdk_string_free((*name).name);

            // Free contest info contents
            if !(*name).contest_info.contenders.is_null()
                && (*name).contest_info.contender_count > 0
            {
                for j in 0..(*name).contest_info.contender_count {
                    let contender = (*name).contest_info.contenders.add(j);
                    dash_sdk_string_free((*contender).identity_id);
                }
                let _ = Vec::from_raw_parts(
                    (*name).contest_info.contenders,
                    (*name).contest_info.contender_count,
                    (*name).contest_info.contender_count,
                );
            }
        }
        let _ = Vec::from_raw_parts(list.names, list.count, list.count);
    }
}

/// Free a name-timestamp structure
///
/// # Safety
/// - `entry` must be a valid, non-dangling pointer to a `DashSDKNameTimestamp` allocated by this SDK.
/// - It may be null (no-op). When non-null, this frees the owned string and the struct.
/// - Do not use `entry` after this call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_name_timestamp_free(entry: *mut DashSDKNameTimestamp) {
    if entry.is_null() {
        return;
    }

    let entry = Box::from_raw(entry);
    dash_sdk_string_free(entry.name);
}

/// Free a name-timestamp list
///
/// # Safety
/// - `list` must be a valid pointer to a `DashSDKNameTimestampList` allocated by this SDK.
/// - It may be null (no-op). When non-null, this frees the entries array and contained strings.
/// - Pointer must not be used after this call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_name_timestamp_list_free(list: *mut DashSDKNameTimestampList) {
    if list.is_null() {
        return;
    }

    let list = Box::from_raw(list);
    if !list.entries.is_null() && list.count > 0 {
        for i in 0..list.count {
            let entry = list.entries.add(i);
            dash_sdk_string_free((*entry).name);
        }
        let _ = Vec::from_raw_parts(list.entries, list.count, list.count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Proves that `DashSDKResult::success_binary` discards Vec capacity,
    /// which causes `dash_sdk_binary_data_free` to reconstruct the Vec with
    /// `len` used as `capacity`. When the original Vec had capacity > len,
    /// the free function passes the wrong size to the allocator -- this is
    /// undefined behavior per the `GlobalAlloc::dealloc` contract.
    ///
    /// This test does NOT invoke the buggy free path (doing so would be
    /// actual UB). Instead it demonstrates that the capacity information is
    /// irrecoverably lost during the `success_binary` roundtrip.
    #[test]
    fn test_success_binary_loses_vec_capacity() {
        // 1. Create a Vec with more capacity than length.
        let mut vec: Vec<u8> = Vec::with_capacity(100);
        vec.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

        let original_len = vec.len(); // 10
        let original_capacity = vec.capacity(); // >= 100

        assert_eq!(original_len, 10);
        assert!(
            original_capacity >= 100,
            "Vec::with_capacity(100) should have capacity >= 100, got {}",
            original_capacity
        );
        assert!(
            original_capacity > original_len,
            "capacity ({}) must be greater than len ({}) for this test to be meaningful",
            original_capacity,
            original_len
        );

        // 2. Pass through success_binary -- this calls `data.as_ptr()`,
        //    captures `data.len()`, then `mem::forget(data)`.
        //    The capacity is never stored anywhere.
        let result = DashSDKResult::success_binary(vec);

        // 3. The result should contain binary data.
        assert_eq!(result.data_type, DashSDKResultDataType::BinaryData);
        assert!(!result.data.is_null());

        // 4. Extract the DashSDKBinaryData to inspect what was stored.
        let binary_data = unsafe { &*(result.data as *const DashSDKBinaryData) };

        // 5. Prove the mismatch: DashSDKBinaryData only has `len`, which
        //    equals the original Vec's length -- NOT its capacity.
        assert_eq!(
            binary_data.len, original_len,
            "DashSDKBinaryData.len should equal the original Vec length"
        );

        // 6. The struct has no capacity field. If dash_sdk_binary_data_free
        //    were called, it would do:
        //
        //      Vec::from_raw_parts(data.data, data.len, data.len)
        //                                              ^^^^^^^^
        //                                       capacity = len = 10
        //
        //    But the actual allocation was for capacity >= 100. The allocator
        //    would be asked to deallocate 10 bytes from an allocation that
        //    was originally 100+ bytes -- undefined behavior.
        assert!(
            binary_data.len < original_capacity,
            "BUG CONFIRMED: DashSDKBinaryData.len ({}) < original capacity ({}). \
             The free function will reconstruct Vec with capacity={} but the \
             allocator originally allocated {} bytes. This is UB on dealloc.",
            binary_data.len,
            original_capacity,
            binary_data.len,
            original_capacity
        );

        // 7. Clean up safely: we reconstruct the Vec with the CORRECT
        //    capacity to avoid triggering the very UB we are proving exists.
        //    This is the key insight -- only we (the test) know the true
        //    capacity. The FFI consumer would not.
        unsafe {
            let binary_data = Box::from_raw(result.data as *mut DashSDKBinaryData);
            let _ = Vec::from_raw_parts(binary_data.data, binary_data.len, original_capacity);
        }
    }

    /// Proves that DashSDKBinaryData has no capacity field, making it
    /// structurally impossible to perform a correct deallocation when
    /// the original Vec's capacity differs from its length.
    #[test]
    fn test_binary_data_struct_has_no_capacity_field() {
        // DashSDKBinaryData is repr(C) with two fields: *mut u8 and usize.
        // On a 64-bit platform, that is exactly 16 bytes (8 + 8).
        // A correct representation would need a third field for capacity,
        // making it 24 bytes.
        let struct_size = std::mem::size_of::<DashSDKBinaryData>();

        // Two pointer-sized fields: data pointer + len
        let expected_size = std::mem::size_of::<*mut u8>() + std::mem::size_of::<usize>();

        assert_eq!(
            struct_size, expected_size,
            "DashSDKBinaryData is {} bytes (only ptr + len). \
             It would need {} more bytes for a capacity field.",
            struct_size,
            std::mem::size_of::<usize>()
        );
    }
}
