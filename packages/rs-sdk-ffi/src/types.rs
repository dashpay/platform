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

// Single source of truth for the network type across the Rust-side
// SDK and the FFI boundary. `Network` is the typed enum; `FFINetwork`
// is the `#[repr(C)]` mirror cbindgen emits for callers. Every FFI
// entry point in this crate takes / returns `FFINetwork`; internal
// Rust code converts via `Network::from(ffi)` / `ffi.into()`.
pub use dash_network::ffi::FFINetwork;
pub use dash_network::Network;

/// SDK configuration passed from C callers.
///
/// # Pointer lifetime
///
/// `dapi_addresses` is a borrowed `*const c_char` whose memory is owned by the
/// caller. The pointer is **only read during the FFI entry-point call** (e.g.,
/// `dash_sdk_create`, `dash_sdk_create_trusted`, `dash_sdk_create_with_callbacks`)
/// and the string data is copied into Rust-owned memory immediately. Callers may
/// free the original C string as soon as the creation function returns.
///
/// `Copy` is intentionally **not** derived: duplicating raw pointers via implicit
/// copies risks use-after-free if the original string is freed while a copy is
/// still in use.
// TODO(CMT-007, #3711): FFI cannot express initial protocol-version seed for
// older-network interop. Deferred — pending core SDK fix for the broader
// "first-request-on-default-SDK uses latest() wire shape" issue (CMT-005).
// Once SDK auto-detects PV before encoding the first request, FFI inherits
// it without API surface changes.
#[repr(C)]
pub struct DashSDKConfig {
    /// Network to connect to
    pub network: FFINetwork,
    /// Comma-separated list of DAPI addresses (e.g., "http://127.0.0.1:3000,http://127.0.0.1:3001")
    /// If null or empty, will use mock SDK.
    ///
    /// This pointer is only read during the creation call; the string data is
    /// immediately copied into Rust-owned memory.
    pub dapi_addresses: *const c_char,
    /// Skip asset lock proof verification (for testing)
    pub skip_asset_lock_proof_verification: bool,
    /// Number of retries for failed requests
    pub request_retry_count: u32,
    /// Timeout for requests in milliseconds
    pub request_timeout_ms: u64,
    /// Optional override for the trusted-context-provider quorum lookup base URL
    /// (e.g., `"https://quorums.devnet.example.networks.dash.org"` or
    /// `"http://127.0.0.1:22444"`). When null/empty, the provider uses the
    /// default endpoint derived from `network` (mainnet/testnet only — devnet
    /// needs an explicit URL, regtest defaults to the local sidecar).
    ///
    /// **Only honored on the `dash_sdk_create_trusted` path** — that's the
    /// path that builds a `TrustedHttpContextProvider`, which is the
    /// component that actually performs quorum lookups. The callback-based
    /// path (`dash_sdk_create_with_callbacks`) uses `CallbackContextProvider`
    /// and ignores this field entirely; non-null values there are silently
    /// dropped.
    ///
    /// Same lifetime contract as `dapi_addresses`: borrowed, copied
    /// immediately, caller may free after the FFI call returns.
    pub quorum_url: *const c_char,
    /// Pin to a specific Dash Platform protocol version.
    /// `0` keeps the SDK default — auto-detect seeded at the default initial
    /// protocol-version floor, ratcheting up to the network's version; any
    /// non-zero value is forwarded to `SdkBuilder::with_version` and rejected
    /// if unknown.
    pub platform_version: u32,
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
        // Use into_boxed_slice() to shrink capacity to exactly len,
        // so that the free function can safely reconstruct with capacity == len.
        let data = data.into_boxed_slice();
        let len = data.len();
        let data_ptr = Box::into_raw(data) as *mut u8;

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
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            info.address,
            info.address_len,
        )));
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
                drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                    entry.address,
                    entry.address_len,
                )));
            }
        }
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            map.entries,
            map.count,
        )));
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
                drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                    element.key,
                    element.key_len,
                )));
            }
        }
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            state.elements,
            state.elements_count,
        )));
    }

    // Free leaf boundaries
    if !state.leaf_boundaries.is_null() && state.leaf_boundaries_count > 0 {
        let boundaries_slice =
            std::slice::from_raw_parts_mut(state.leaf_boundaries, state.leaf_boundaries_count);
        for boundary in boundaries_slice.iter() {
            if !boundary.key.is_null() && boundary.key_len > 0 {
                drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                    boundary.key,
                    boundary.key_len,
                )));
            }
        }
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            state.leaf_boundaries,
            state.leaf_boundaries_count,
        )));
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
                drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                    element.key,
                    element.key_len,
                )));
            }
        }
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            state.elements,
            state.elements_count,
        )));
    }

    // Free leaf boundaries
    if !state.leaf_boundaries.is_null() && state.leaf_boundaries_count > 0 {
        let boundaries_slice =
            std::slice::from_raw_parts_mut(state.leaf_boundaries, state.leaf_boundaries_count);
        for boundary in boundaries_slice.iter() {
            if !boundary.key.is_null() && boundary.key_len > 0 {
                drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                    boundary.key,
                    boundary.key_len,
                )));
            }
        }
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            state.leaf_boundaries,
            state.leaf_boundaries_count,
        )));
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
                        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                            change.address,
                            change.address_len,
                        )));
                    }
                }
                drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                    block.changes,
                    block.changes_count,
                )));
            }
        }
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            changes.blocks,
            changes.blocks_count,
        )));
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
                        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                            change.address,
                            change.address_len,
                        )));
                    }
                    // Free add entries
                    if !change.add_entries.is_null() && change.add_entries_count > 0 {
                        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                            change.add_entries,
                            change.add_entries_count,
                        )));
                    }
                }
                drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                    range.changes,
                    range.changes_count,
                )));
            }
        }
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            changes.ranges,
            changes.ranges_count,
        )));
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
    /// The label this contender actually requested ("pizza"), decoded from
    /// their `domain` document — the contest itself is keyed by the
    /// homograph-normalized form ("p1zza"). Null when the document could not
    /// be decoded; callers fall back to the normalized name rather than
    /// guessing a spelling.
    pub label: *mut c_char,
}

/// Represents contest information for a DPNS name
#[repr(C)]
pub struct DashSDKContestInfo {
    /// Array of contenders
    pub contenders: *mut DashSDKContender,
    /// Number of contenders
    pub contender_count: usize,
    /// The distinct labels the contenders requested, in contender order and
    /// already de-duplicated — `["pizza", "p1zza"]` for a contest normalized to
    /// `"p1zza"`. Derived here rather than by the caller so display policy has
    /// one home. Null/zero when no contender document could be decoded; callers
    /// fall back to the normalized contest name.
    pub requested_labels: *mut *mut c_char,
    /// Number of entries in `requested_labels`.
    pub requested_label_count: usize,
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

    let mut contender = Box::from_raw(contender);
    free_contender_fields(&mut *contender);
}

/// Release the heap strings owned by one contender.
///
/// Every destructor that owns contenders funnels through this: the fields were
/// duplicated across four free paths, and adding `label` to the struct without
/// updating all of them leaked one allocation per decoded contender on three of
/// them. A single helper makes the next added field a one-line change.
///
/// # Safety
/// - `contender` must point to a live `DashSDKContender` whose strings were
///   allocated by this SDK and not yet freed.
unsafe fn free_contender_fields(contender: *mut DashSDKContender) {
    dash_sdk_string_free((*contender).identity_id);
    // Null whenever the contender document could not be decoded;
    // `dash_sdk_string_free` is a no-op on null.
    dash_sdk_string_free((*contender).label);
}

/// Free the contender array owned by a `DashSDKContestInfo`, including each
/// contender's strings.
///
/// # Safety
/// - `contenders` must be either null or a pointer obtained from
///   `Box::into_raw` over a boxed slice of exactly `count` contenders.
unsafe fn free_contender_array(contenders: *mut DashSDKContender, count: usize) {
    if contenders.is_null() || count == 0 {
        return;
    }
    for i in 0..count {
        free_contender_fields(contenders.add(i));
    }
    let _ = Vec::from_raw_parts(contenders, count, count);
}

/// Release a heap array of C strings allocated by this SDK.
///
/// # Safety
/// - `strings` must be null, or a pointer from `Box::into_raw` over a boxed
///   slice of exactly `count` `CString::into_raw` pointers.
unsafe fn free_string_array(strings: *mut *mut c_char, count: usize) {
    if strings.is_null() || count == 0 {
        return;
    }
    for i in 0..count {
        dash_sdk_string_free(*strings.add(i));
    }
    let _ = Vec::from_raw_parts(strings, count, count);
}

/// Release everything a `DashSDKContestInfo` owns, without freeing the struct
/// itself — it is embedded in `DashSDKContestedName` in some paths and boxed in
/// others. Single home for "what does a contest info own", so a new field is
/// one edit rather than one per destructor.
///
/// # Safety
/// - `info` must point to a live `DashSDKContestInfo` whose buffers were
///   allocated by this SDK and not yet freed.
unsafe fn free_contest_info_fields(info: *mut DashSDKContestInfo) {
    free_contender_array((*info).contenders, (*info).contender_count);
    free_string_array((*info).requested_labels, (*info).requested_label_count);
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

    let mut info = Box::from_raw(info);
    free_contest_info_fields(&mut *info);
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

    let mut name = Box::from_raw(name);
    dash_sdk_string_free(name.name);

    // Free contest info contents (but not the struct itself as it's embedded)
    free_contest_info_fields(&mut name.contest_info);
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
            free_contest_info_fields(&mut (*name).contest_info);
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

/// Helper to free the entries inside a `DashSDKAddressInfoMap` without freeing
/// the map struct itself (used when the map is embedded in another struct).
///
/// # Safety
/// Caller must ensure the map's entries pointer and count are valid.
pub(crate) unsafe fn free_address_info_map_entries(map: &DashSDKAddressInfoMap) {
    if !map.entries.is_null() && map.count > 0 {
        let entries_slice = std::slice::from_raw_parts_mut(map.entries, map.count);
        for entry in entries_slice.iter() {
            if !entry.address.is_null() && entry.address_len > 0 {
                drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                    entry.address,
                    entry.address_len,
                )));
            }
        }
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            map.entries,
            map.count,
        )));
    }
}

/// Free a `DashSDKResult` and all resources it owns.
///
/// This is a unified free function that inspects the `data_type` field and
/// dispatches to the correct cleanup logic for the `data` pointer. It also
/// frees the `error` field when present. After this call the pointed-to
/// `DashSDKResult` has its `data` and `error` fields set to null so that
/// accidental double-frees are harmless no-ops.
///
/// # Safety
/// - `result` must be a valid, non-null pointer to a `DashSDKResult` whose
///   `data` and `error` fields were produced by this SDK and have not yet
///   been freed.
/// - If `result` is null this is a no-op.
/// - After this call, the `data` and `error` pointers inside the struct are
///   set to null. The `DashSDKResult` struct itself is **not** freed (it is
///   typically stack-allocated by the caller).
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_result_free(result: *mut DashSDKResult) {
    if result.is_null() {
        return;
    }

    let res = &mut *result;

    // ── Free the error field ──────────────────────────────────────────
    if !res.error.is_null() {
        let error = Box::from_raw(res.error);
        if !error.message.is_null() {
            let _ = std::ffi::CString::from_raw(error.message);
        }
        // Box is dropped here, freeing the DashSDKError struct
        res.error = std::ptr::null_mut();
    }

    // ── Free the data field based on data_type ────────────────────────
    if !res.data.is_null() {
        match res.data_type {
            DashSDKResultDataType::NoData => {
                // No data to free; the pointer may be non-null for legacy
                // reasons but no allocation was made via Box.
            }

            DashSDKResultDataType::String => {
                // data is a *mut c_char from CString::into_raw
                let _ = std::ffi::CString::from_raw(res.data as *mut c_char);
            }

            DashSDKResultDataType::BinaryData => {
                // data is Box<DashSDKBinaryData>
                dash_sdk_binary_data_free(res.data as *mut DashSDKBinaryData);
            }

            DashSDKResultDataType::ResultIdentityHandle => {
                // data is Box<Identity> cast to *mut IdentityHandle
                dash_sdk_identity_destroy_ptr(res.data as *mut IdentityHandle);
            }

            DashSDKResultDataType::ResultDocumentHandle => {
                // data is Box<Document> cast to *mut DocumentHandle
                dash_sdk_document_destroy_ptr(res.data as *mut DocumentHandle);
            }

            DashSDKResultDataType::ResultDataContractHandle => {
                // data is Box<DataContract> cast to *mut DataContractHandle
                dash_sdk_data_contract_destroy_ptr(res.data as *mut DataContractHandle);
            }

            DashSDKResultDataType::IdentityBalanceMap => {
                dash_sdk_identity_balance_map_free(res.data as *mut DashSDKIdentityBalanceMap);
            }

            DashSDKResultDataType::ResultPublicKeyHandle => {
                // data is Box<IdentityPublicKey> cast to *mut IdentityPublicKeyHandle
                dash_sdk_public_key_destroy_ptr(res.data as *mut IdentityPublicKeyHandle);
            }

            DashSDKResultDataType::AddressInfo => {
                dash_sdk_address_info_free(res.data as *mut DashSDKAddressInfo);
            }

            DashSDKResultDataType::AddressInfoMap => {
                dash_sdk_address_info_map_free(res.data as *mut DashSDKAddressInfoMap);
            }

            DashSDKResultDataType::TrunkState => {
                dash_sdk_trunk_state_free(res.data as *mut DashSDKTrunkState);
            }

            DashSDKResultDataType::BranchState => {
                dash_sdk_branch_state_free(res.data as *mut DashSDKBranchState);
            }

            DashSDKResultDataType::RecentBalanceChanges => {
                dash_sdk_recent_balance_changes_free(res.data as *mut DashSDKRecentBalanceChanges);
            }

            DashSDKResultDataType::CompactedBalanceChanges => {
                dash_sdk_compacted_balance_changes_free(
                    res.data as *mut DashSDKCompactedBalanceChanges,
                );
            }

            DashSDKResultDataType::IdentityTopUpFromAddressesResult => {
                let inner = Box::from_raw(
                    res.data as *mut super::identity::DashSDKIdentityTopUpFromAddressesResult,
                );
                free_address_info_map_entries(&inner.address_info_map);
            }

            DashSDKResultDataType::IdentityTransferToAddressesResult => {
                let inner = Box::from_raw(
                    res.data as *mut super::identity::DashSDKIdentityTransferToAddressesResult,
                );
                free_address_info_map_entries(&inner.address_info_map);
            }

            DashSDKResultDataType::IdentityCreateFromAddressesResult => {
                let inner = Box::from_raw(
                    res.data as *mut super::identity::DashSDKIdentityCreateFromAddressesResult,
                );
                free_address_info_map_entries(&inner.address_info_map);
                // Note: identity_handle inside is intentionally NOT freed here.
                // The caller must free it separately via dash_sdk_identity_destroy,
                // consistent with dash_sdk_identity_create_from_addresses_result_free.
            }
        }

        res.data = std::ptr::null_mut();
    }
}

// ── Internal helpers for freeing handle types by pointer ──────────────
// These thin wrappers exist so that `dash_sdk_result_free` can free
// opaque handles without importing concrete platform types directly.
// They mirror the logic of the existing `dash_sdk_*_destroy` functions.

/// Free an identity handle pointer (internal helper).
///
/// # Safety
/// - `handle` must be a valid `*mut IdentityHandle` produced by `Box::into_raw`.
unsafe fn dash_sdk_identity_destroy_ptr(handle: *mut IdentityHandle) {
    if !handle.is_null() {
        // IdentityHandle is an opaque ZST; the actual allocation is Box<Identity>.
        let _ = Box::from_raw(handle as *mut dash_sdk::dpp::prelude::Identity);
    }
}

/// Free a document handle pointer (internal helper).
///
/// # Safety
/// - `handle` must be a valid `*mut DocumentHandle` produced by `Box::into_raw`.
unsafe fn dash_sdk_document_destroy_ptr(handle: *mut DocumentHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut dash_sdk::dpp::document::Document);
    }
}

/// Free a data contract handle pointer (internal helper).
///
/// # Safety
/// - `handle` must be a valid `*mut DataContractHandle` produced by `Box::into_raw`.
unsafe fn dash_sdk_data_contract_destroy_ptr(handle: *mut DataContractHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut dash_sdk::dpp::prelude::DataContract);
    }
}

/// Free a public key handle pointer (internal helper).
///
/// # Safety
/// - `handle` must be a valid `*mut IdentityPublicKeyHandle` produced by `Box::into_raw`.
unsafe fn dash_sdk_public_key_destroy_ptr(handle: *mut IdentityPublicKeyHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut dash_sdk::dpp::identity::IdentityPublicKey);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};

    /// Verifies that `DashSDKResult::success_binary` now correctly shrinks
    /// Vec capacity to match len via `into_boxed_slice()`, so that the free
    /// function can safely reconstruct with `Vec::from_raw_parts(ptr, len, len)`.
    #[test]
    fn test_success_binary_preserves_capacity_via_shrink() {
        // 1. Create a Vec with more capacity than length.
        let mut vec: Vec<u8> = Vec::with_capacity(100);
        vec.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

        let original_len = vec.len(); // 10

        assert_eq!(original_len, 10);
        assert!(
            vec.capacity() >= 100,
            "Vec::with_capacity(100) should have capacity >= 100, got {}",
            vec.capacity()
        );
        assert!(
            vec.capacity() > original_len,
            "capacity ({}) must be greater than len ({}) for this test to be meaningful",
            vec.capacity(),
            original_len
        );

        // 2. Pass through success_binary -- this now uses into_boxed_slice()
        //    which shrinks the allocation so capacity == len.
        let result = DashSDKResult::success_binary(vec);

        // 3. The result should contain binary data.
        assert_eq!(result.data_type, DashSDKResultDataType::BinaryData);
        assert!(!result.data.is_null());

        // 4. Extract the DashSDKBinaryData to inspect what was stored.
        let binary_data = unsafe { &*(result.data as *const DashSDKBinaryData) };

        // 5. Verify len is correct.
        assert_eq!(
            binary_data.len, original_len,
            "DashSDKBinaryData.len should equal the original Vec length"
        );

        // 6. Now it is safe to free via the FFI free function because
        //    the allocation was shrunk so capacity == len. Verify this
        //    by calling the actual free function.
        unsafe {
            dash_sdk_binary_data_free(result.data as *mut DashSDKBinaryData);
        }
    }

    /// Verifies the full roundtrip: create a Vec with extra capacity, pass it
    /// through success_binary, read back the data, and free it. This exercises
    /// the exact code path that was previously UB.
    #[test]
    fn test_success_binary_roundtrip_with_extra_capacity() {
        let payload: Vec<u8> = (0u8..=255).collect(); // len == 256
        let mut vec = Vec::with_capacity(1024);
        vec.extend_from_slice(&payload);
        assert!(vec.capacity() >= 1024);
        assert_eq!(vec.len(), 256);

        let result = DashSDKResult::success_binary(vec);

        // Verify the data content is intact.
        let binary_data = unsafe { &*(result.data as *const DashSDKBinaryData) };
        assert_eq!(binary_data.len, 256);
        let slice = unsafe { std::slice::from_raw_parts(binary_data.data, binary_data.len) };
        assert_eq!(slice, &payload[..]);

        // Free safely -- this was previously UB when capacity != len.
        unsafe {
            dash_sdk_binary_data_free(result.data as *mut DashSDKBinaryData);
        }
    }

    /// Verifies that DashSDKBinaryData has only two fields (ptr + len).
    /// The fix ensures capacity == len via into_boxed_slice(), so no
    /// capacity field is needed in the struct.
    #[test]
    fn test_binary_data_struct_has_no_capacity_field() {
        // DashSDKBinaryData is repr(C) with two fields: *mut u8 and usize.
        // On a 64-bit platform, that is exactly 16 bytes (8 + 8).
        let struct_size = std::mem::size_of::<DashSDKBinaryData>();

        // Two pointer-sized fields: data pointer + len
        let expected_size = std::mem::size_of::<*mut u8>() + std::mem::size_of::<usize>();

        assert_eq!(
            struct_size, expected_size,
            "DashSDKBinaryData is {} bytes (only ptr + len). \
             The fix guarantees capacity == len via into_boxed_slice().",
            struct_size,
        );
    }

    /// Verify that calling `dash_sdk_result_free` with a null pointer does
    /// not panic or crash.
    #[test]
    fn test_dash_sdk_result_free_null_pointer() {
        unsafe {
            dash_sdk_result_free(std::ptr::null_mut());
        }
        // If we reach here without a crash, the test passes.
    }

    /// Verify that `dash_sdk_result_free` properly frees an error-only result.
    #[test]
    fn test_dash_sdk_result_free_error_result() {
        unsafe {
            let error = super::super::DashSDKError::new(
                super::super::DashSDKErrorCode::InternalError,
                "test error message".to_string(),
            );
            let mut result = DashSDKResult::error(error);

            // Sanity: error is non-null, data is null
            assert!(!result.error.is_null());
            assert!(result.data.is_null());

            dash_sdk_result_free(&mut result as *mut DashSDKResult);

            // After free, both pointers should be null
            assert!(result.error.is_null());
            assert!(result.data.is_null());
        }
    }

    /// Verify that `dash_sdk_result_free` properly frees a success result
    /// containing a String data type.
    #[test]
    fn test_dash_sdk_result_free_success_string() {
        unsafe {
            let c_string = CString::new("hello from FFI").unwrap();
            let mut result = DashSDKResult::success_string(c_string.into_raw());

            assert!(!result.data.is_null());
            assert!(result.error.is_null());
            assert_eq!(result.data_type, DashSDKResultDataType::String);

            dash_sdk_result_free(&mut result as *mut DashSDKResult);

            assert!(result.data.is_null());
        }
    }

    /// Verify that `dash_sdk_result_free` properly frees a success result
    /// containing binary data.
    #[test]
    fn test_dash_sdk_result_free_success_binary() {
        unsafe {
            let data = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
            let mut result = DashSDKResult::success_binary(data);

            assert!(!result.data.is_null());
            assert!(result.error.is_null());
            assert_eq!(result.data_type, DashSDKResultDataType::BinaryData);

            dash_sdk_result_free(&mut result as *mut DashSDKResult);

            assert!(result.data.is_null());
        }
    }

    /// Verify that `dash_sdk_result_free` properly frees a success result
    /// containing an identity balance map.
    #[test]
    fn test_dash_sdk_result_free_success_identity_balance_map() {
        unsafe {
            let entries = vec![
                DashSDKIdentityBalanceEntry {
                    identity_id: [1u8; 32],
                    balance: 1000,
                },
                DashSDKIdentityBalanceEntry {
                    identity_id: [2u8; 32],
                    balance: 2000,
                },
            ];
            let entries_len = entries.len();
            let entries_ptr =
                Box::into_raw(entries.into_boxed_slice()) as *mut DashSDKIdentityBalanceEntry;

            let map = DashSDKIdentityBalanceMap {
                entries: entries_ptr,
                count: entries_len,
            };
            let mut result = DashSDKResult::success_identity_balance_map(map);

            assert!(!result.data.is_null());
            assert_eq!(result.data_type, DashSDKResultDataType::IdentityBalanceMap);

            dash_sdk_result_free(&mut result as *mut DashSDKResult);

            assert!(result.data.is_null());
        }
    }

    /// Verify that `dash_sdk_result_free` properly frees a NoData success
    /// result (null data pointer).
    #[test]
    fn test_dash_sdk_result_free_success_no_data() {
        unsafe {
            let mut result = DashSDKResult::success(std::ptr::null_mut());

            assert!(result.data.is_null());
            assert!(result.error.is_null());
            assert_eq!(result.data_type, DashSDKResultDataType::NoData);

            // Should be a no-op, no crash
            dash_sdk_result_free(&mut result as *mut DashSDKResult);

            assert!(result.data.is_null());
            assert!(result.error.is_null());
        }
    }

    /// Verify double-free safety: after `dash_sdk_result_free` nulls the
    /// pointers, calling it again on the same struct is a safe no-op.
    #[test]
    fn test_dash_sdk_result_free_double_free_safety() {
        unsafe {
            let error = super::super::DashSDKError::new(
                super::super::DashSDKErrorCode::NetworkError,
                "double free test".to_string(),
            );
            let mut result = DashSDKResult::error(error);

            dash_sdk_result_free(&mut result as *mut DashSDKResult);
            // Second call should be a no-op
            dash_sdk_result_free(&mut result as *mut DashSDKResult);

            assert!(result.error.is_null());
            assert!(result.data.is_null());
        }
    }

    // MARK: contender destructors
    //
    // These run under Miri/ASan in CI if enabled, but even without a leak
    // checker they pin the shape that regressed: `label` was added to
    // `DashSDKContender` and only one of the four free paths was updated, so
    // three public destructors leaked one allocation per decoded contender.
    // Every destructor now funnels through `free_contender_fields`.

    /// Build a heap contender the way the DPNS producers do.
    unsafe fn make_contender(id: &str, label: Option<&str>) -> DashSDKContender {
        DashSDKContender {
            identity_id: CString::new(id).unwrap().into_raw(),
            vote_count: 7,
            label: match label {
                Some(l) => CString::new(l).unwrap().into_raw(),
                None => std::ptr::null_mut(),
            },
        }
    }

    unsafe fn make_contest_info(contenders: Vec<DashSDKContender>) -> DashSDKContestInfo {
        // Mirror the producers: the ordered-unique labels are a separate
        // allocation the destructors must also release.
        let mut labels: Vec<*mut c_char> = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        for c in &contenders {
            if c.label.is_null() {
                continue;
            }
            let l = CStr::from_ptr(c.label).to_string_lossy().into_owned();
            if l.is_empty() || seen.contains(&l) {
                continue;
            }
            seen.push(l.clone());
            labels.push(CString::new(l).unwrap().into_raw());
        }
        let label_count = labels.len();
        let labels_ptr = if labels.is_empty() {
            std::ptr::null_mut()
        } else {
            Box::into_raw(labels.into_boxed_slice()) as *mut *mut c_char
        };

        let count = contenders.len();
        let ptr = if count == 0 {
            std::ptr::null_mut()
        } else {
            Box::into_raw(contenders.into_boxed_slice()) as *mut DashSDKContender
        };
        DashSDKContestInfo {
            contenders: ptr,
            contender_count: count,
            requested_labels: labels_ptr,
            requested_label_count: label_count,
            abstain_votes: 1,
            lock_votes: 2,
            end_time: 1_700_000_000_000,
            has_winner: false,
        }
    }

    #[test]
    fn contender_free_releases_label() {
        unsafe {
            let contender = Box::into_raw(Box::new(make_contender("Ab1", Some("pizza"))));
            dash_sdk_contender_free(contender);
        }
    }

    #[test]
    fn contender_free_tolerates_absent_label() {
        unsafe {
            let contender = Box::into_raw(Box::new(make_contender("Ab1", None)));
            dash_sdk_contender_free(contender);
        }
    }

    #[test]
    fn contender_free_is_null_safe() {
        unsafe { dash_sdk_contender_free(std::ptr::null_mut()) };
    }

    #[test]
    fn contest_info_free_releases_every_contender_label() {
        unsafe {
            let info = make_contest_info(vec![
                make_contender("Ab1", Some("pizza")),
                make_contender("Cd2", Some("p1zza")),
                make_contender("Ef3", None),
            ]);
            dash_sdk_contest_info_free(Box::into_raw(Box::new(info)));
        }
    }

    #[test]
    fn contest_info_free_handles_no_contenders() {
        unsafe {
            let info = make_contest_info(vec![]);
            dash_sdk_contest_info_free(Box::into_raw(Box::new(info)));
        }
    }

    #[test]
    fn contested_name_free_releases_labels() {
        unsafe {
            let name = DashSDKContestedName {
                name: CString::new("p1zza").unwrap().into_raw(),
                contest_info: make_contest_info(vec![
                    make_contender("Ab1", Some("pizza")),
                    make_contender("Cd2", None),
                ]),
            };
            dash_sdk_contested_name_free(Box::into_raw(Box::new(name)));
        }
    }

    #[test]
    fn contest_info_free_releases_requested_labels() {
        unsafe {
            // Two distinct spellings plus a repeat and an undecodable row:
            // the ordered-unique array holds two entries, all of which the
            // destructor must release.
            let info = make_contest_info(vec![
                make_contender("Ab1", Some("pizza")),
                make_contender("Cd2", Some("p1zza")),
                make_contender("Ef3", Some("pizza")),
                make_contender("Gh4", None),
            ]);
            assert_eq!(info.requested_label_count, 2);
            dash_sdk_contest_info_free(Box::into_raw(Box::new(info)));
        }
    }

    #[test]
    fn contest_info_has_no_labels_when_nothing_decoded() {
        unsafe {
            let info = make_contest_info(vec![make_contender("Ab1", None)]);
            assert_eq!(info.requested_label_count, 0);
            assert!(info.requested_labels.is_null());
            dash_sdk_contest_info_free(Box::into_raw(Box::new(info)));
        }
    }

    #[test]
    fn contested_names_list_free_releases_labels() {
        unsafe {
            let names = vec![DashSDKContestedName {
                name: CString::new("p1zza").unwrap().into_raw(),
                contest_info: make_contest_info(vec![make_contender("Ab1", Some("pizza"))]),
            }];
            let count = names.len();
            let list = DashSDKContestedNamesList {
                names: Box::into_raw(names.into_boxed_slice()) as *mut DashSDKContestedName,
                count,
            };
            dash_sdk_contested_names_list_free(Box::into_raw(Box::new(list)));
        }
    }
}
