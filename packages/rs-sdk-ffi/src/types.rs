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
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_string_free(s: *mut c_char) {
    if !s.is_null() {
        let _ = std::ffi::CString::from_raw(s);
    }
}

/// Free binary data allocated by the FFI
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
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_info_free(info: *mut DashSDKIdentityInfo) {
    if info.is_null() {
        return;
    }

    let info = Box::from_raw(info);
    dash_sdk_string_free(info.id);
}

/// Free a document info structure
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
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_contender_free(contender: *mut DashSDKContender) {
    if contender.is_null() {
        return;
    }

    let contender = Box::from_raw(contender);
    dash_sdk_string_free(contender.identity_id);
}

/// Free contest info structure
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
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_name_timestamp_free(entry: *mut DashSDKNameTimestamp) {
    if entry.is_null() {
        return;
    }

    let entry = Box::from_raw(entry);
    dash_sdk_string_free(entry.name);
}

/// Free a name-timestamp list
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
