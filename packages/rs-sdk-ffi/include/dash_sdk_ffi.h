#ifndef DASH_SDK_FFI_H
#define DASH_SDK_FFI_H

#pragma once

/* Generated with cbindgen:0.29.2 */

/* This file is auto-generated. Do not modify manually. */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>
#include "dash_spv_ffi.h"

// Result data type indicator for iOS
typedef enum DashSDKResultDataType {
  // No data (void/null)
  DashSDKResultDataType_NoData = 0,
  // C string (char*)
  DashSDKResultDataType_String = 1,
  // Binary data with length
  DashSDKResultDataType_BinaryData = 2,
  // Identity handle
  DashSDKResultDataType_ResultIdentityHandle = 3,
  // Document handle
  DashSDKResultDataType_ResultDocumentHandle = 4,
  // Data contract handle
  DashSDKResultDataType_ResultDataContractHandle = 5,
  // Map of identity IDs to balances
  DashSDKResultDataType_IdentityBalanceMap = 6,
  // Public key handle
  DashSDKResultDataType_ResultPublicKeyHandle = 7,
} DashSDKResultDataType;

// Error codes returned by FFI functions
typedef enum DashSDKErrorCode {
  // Operation completed successfully
  DashSDKErrorCode_Success = 0,
  // Invalid parameter passed to function
  DashSDKErrorCode_InvalidParameter = 1,
  // SDK not initialized or in invalid state
  DashSDKErrorCode_InvalidState = 2,
  // Network error occurred
  DashSDKErrorCode_NetworkError = 3,
  // Serialization/deserialization error
  DashSDKErrorCode_SerializationError = 4,
  // Platform protocol error
  DashSDKErrorCode_ProtocolError = 5,
  // Cryptographic operation failed
  DashSDKErrorCode_CryptoError = 6,
  // Resource not found
  DashSDKErrorCode_NotFound = 7,
  // Operation timed out
  DashSDKErrorCode_Timeout = 8,
  // Feature not implemented
  DashSDKErrorCode_NotImplemented = 9,
  // Internal error
  DashSDKErrorCode_InternalError = 99,
} DashSDKErrorCode;

// Gas fees payer option
typedef enum DashSDKGasFeesPaidBy {
  // The document owner pays the gas fees
  DashSDKGasFeesPaidBy_DocumentOwner = 0,
  // The contract owner pays the gas fees
  DashSDKGasFeesPaidBy_GasFeesContractOwner = 1,
  // Prefer contract owner but fallback to document owner if insufficient balance
  DashSDKGasFeesPaidBy_GasFeesPreferContractOwner = 2,
} DashSDKGasFeesPaidBy;

// Document field value types
typedef enum DashSDKDocumentFieldType {
  DashSDKDocumentFieldType_FieldString = 0,
  DashSDKDocumentFieldType_FieldInteger = 1,
  DashSDKDocumentFieldType_FieldFloat = 2,
  DashSDKDocumentFieldType_FieldBoolean = 3,
  DashSDKDocumentFieldType_FieldBytes = 4,
  DashSDKDocumentFieldType_FieldArray = 5,
  DashSDKDocumentFieldType_FieldObject = 6,
  DashSDKDocumentFieldType_FieldNull = 7,
} DashSDKDocumentFieldType;

// State transition type for key selection
typedef enum StateTransitionType {
  StateTransitionType_IdentityUpdate = 0,
  StateTransitionType_IdentityTopUp = 1,
  StateTransitionType_IdentityCreditTransfer = 2,
  StateTransitionType_IdentityCreditWithdrawal = 3,
  StateTransitionType_DocumentsBatch = 4,
  StateTransitionType_DataContractCreate = 5,
  StateTransitionType_DataContractUpdate = 6,
} StateTransitionType;

// Network type for SDK configuration
typedef enum DashSDKNetwork {
  // Mainnet
  DashSDKNetwork_SDKMainnet = 0,
  // Testnet
  DashSDKNetwork_SDKTestnet = 1,
  // Regtest
  DashSDKNetwork_SDKRegtest = 2,
  // Devnet
  DashSDKNetwork_SDKDevnet = 3,
  // Local development network
  DashSDKNetwork_SDKLocal = 4,
} DashSDKNetwork;

// Token distribution type for claim operations
typedef enum DashSDKTokenDistributionType {
  // Pre-programmed distribution
  DashSDKTokenDistributionType_PreProgrammed = 0,
  // Perpetual distribution
  DashSDKTokenDistributionType_Perpetual = 1,
} DashSDKTokenDistributionType;

// Token configuration update type
typedef enum DashSDKTokenConfigUpdateType {
  // No change
  DashSDKTokenConfigUpdateType_NoChange = 0,
  // Update max supply (requires amount field)
  DashSDKTokenConfigUpdateType_MaxSupply = 1,
  // Update minting allow choosing destination (requires bool_value field)
  DashSDKTokenConfigUpdateType_MintingAllowChoosingDestination = 2,
  // Update new tokens destination identity (requires identity_id field)
  DashSDKTokenConfigUpdateType_NewTokensDestinationIdentity = 3,
  // Update manual minting permissions (requires action_takers field)
  DashSDKTokenConfigUpdateType_ManualMinting = 4,
  // Update manual burning permissions (requires action_takers field)
  DashSDKTokenConfigUpdateType_ManualBurning = 5,
  // Update freeze permissions (requires action_takers field)
  DashSDKTokenConfigUpdateType_Freeze = 6,
  // Update unfreeze permissions (requires action_takers field)
  DashSDKTokenConfigUpdateType_Unfreeze = 7,
  // Update main control group (requires group_position field)
  DashSDKTokenConfigUpdateType_MainControlGroup = 8,
} DashSDKTokenConfigUpdateType;

// Authorized action takers for token operations
typedef enum DashSDKAuthorizedActionTakers {
  // No one can perform the action
  DashSDKAuthorizedActionTakers_NoOne = 0,
  // Only the contract owner can perform the action
  DashSDKAuthorizedActionTakers_AuthorizedContractOwner = 1,
  // Main group can perform the action
  DashSDKAuthorizedActionTakers_MainGroup = 2,
  // A specific identity (requires identity_id to be set)
  DashSDKAuthorizedActionTakers_Identity = 3,
  // A specific group (requires group_position to be set)
  DashSDKAuthorizedActionTakers_Group = 4,
} DashSDKAuthorizedActionTakers;

// Token emergency action type
typedef enum DashSDKTokenEmergencyAction {
  // Pause token operations
  DashSDKTokenEmergencyAction_Pause = 0,
  // Resume token operations
  DashSDKTokenEmergencyAction_Resume = 1,
} DashSDKTokenEmergencyAction;

// Token pricing type
typedef enum DashSDKTokenPricingType {
  // Single flat price for all amounts
  DashSDKTokenPricingType_SinglePrice = 0,
  // Tiered pricing based on amounts
  DashSDKTokenPricingType_SetPrices = 1,
} DashSDKTokenPricingType;

// Opaque handle to a DataContract
typedef struct DataContractHandle DataContractHandle;

// Opaque handle to a Document
typedef struct DocumentHandle DocumentHandle;

// Opaque handle to an Identity
typedef struct IdentityHandle IdentityHandle;

// Opaque handle to an IdentityPublicKey
typedef struct IdentityPublicKeyHandle IdentityPublicKeyHandle;

// Opaque handle to an SDK instance
typedef struct dash_sdk_handle_t dash_sdk_handle_t;

// Opaque handle to a Signer
typedef struct SignerHandle SignerHandle;

// A found address with its balance (FFI-compatible)
typedef struct DashSDKFoundAddress {
  // The derivation index for this address
  uint32_t index;
  // Pointer to the address key bytes
  uint8_t *key;
  // Length of the key in bytes
  uintptr_t key_len;
  // Nonce associated with this address
  uint32_t nonce;
  // Balance in credits at this address
  uint64_t balance;
} DashSDKFoundAddress;

// An address proven absent from the tree (FFI-compatible)
typedef struct DashSDKAbsentAddress {
  // The derivation index for this address
  uint32_t index;
  // Pointer to the address key bytes
  uint8_t *key;
  // Length of the key in bytes
  uintptr_t key_len;
} DashSDKAbsentAddress;

// Metrics about the synchronization process (FFI-compatible)
typedef struct DashSDKAddressSyncMetrics {
  // Number of trunk queries (always 1 for a successful sync)
  uint32_t trunk_queries;
  // Number of branch queries
  uint32_t branch_queries;
  // Total elements seen across all proofs.
  //
  // This gives an indication of the "anonymity set" - how many addresses
  // were potentially being queried from the server's perspective.
  uint32_t total_elements_seen;
  // Total proof bytes received
  uint32_t total_proof_bytes;
  // Number of iterations (0 = trunk only, 1+ = trunk plus branch rounds)
  uint32_t iterations;
} DashSDKAddressSyncMetrics;

// Result of address synchronization (FFI-compatible)
typedef struct DashSDKAddressSyncResult {
  // Array of found addresses with balances
  struct DashSDKFoundAddress *found;
  // Number of found addresses
  uintptr_t found_count;
  // Array of addresses proven absent
  struct DashSDKAbsentAddress *absent;
  // Number of absent addresses
  uintptr_t absent_count;
  // Highest found index (for HD wallets)
  // Only valid if has_highest_found_index is true
  uint32_t highest_found_index;
  // Whether highest_found_index is valid
  bool has_highest_found_index;
  // Metrics about the sync process
  struct DashSDKAddressSyncMetrics metrics;
} DashSDKAddressSyncResult;

// Function pointer type for getting the gap limit
typedef uint32_t (*GetGapLimitFn)(void *context);

// A pending address entry for the provider callback
typedef struct DashSDKPendingAddress {
  // The derivation index for this address
  uint32_t index;
  // Pointer to the address key bytes (32 bytes)
  const uint8_t *key;
  // Length of the key in bytes
  uintptr_t key_len;
} DashSDKPendingAddress;

// List of pending addresses returned by the provider callback
typedef struct DashSDKPendingAddressList {
  // Array of pending addresses
  struct DashSDKPendingAddress *addresses;
  // Number of addresses
  uintptr_t count;
} DashSDKPendingAddressList;

// Function pointer type for getting pending addresses
//
// Returns a list of pending addresses that need to be synchronized.
// The returned list must remain valid until the next call to this function
// or until the sync operation completes.
typedef struct DashSDKPendingAddressList (*GetPendingAddressesFn)(void *context);

// Function pointer type for handling a found address
//
// Called when an address is found in the tree with a balance and nonce.
typedef void (*OnAddressFoundFn)(void *context, uint32_t index, const uint8_t *key, uintptr_t key_len, uint32_t nonce, uint64_t balance);

// Function pointer type for handling an absent address
//
// Called when an address is proven absent from the tree.
typedef void (*OnAddressAbsentFn)(void *context, uint32_t index, const uint8_t *key, uintptr_t key_len);

// Optional function pointer type for checking if there are pending addresses.
// Nullable — may be null (cbindgen translates this to a nullable C function pointer).
typedef bool (*OptionalHasPendingFn)(void *context);

// Optional function pointer type for getting the highest found index.
// Nullable — may be null (cbindgen translates this to a nullable C function pointer).
typedef uint32_t (*OptionalGetHighestFoundIndexFn)(void *context);

// Optional destructor for cleanup.
// Nullable — may be null (cbindgen translates this to a nullable C function pointer).
typedef void (*OptionalDestroyProviderFn)(void *context);

// VTable for address provider callbacks
typedef struct AddressProviderVTable {
  // Get the gap limit for this provider
  GetGapLimitFn gap_limit;
  // Get currently pending addresses to synchronize
  GetPendingAddressesFn pending_addresses;
  // Called when an address is found with a balance
  OnAddressFoundFn on_address_found;
  // Called when an address is proven absent
  OnAddressAbsentFn on_address_absent;
  // Check if there are still pending addresses.
  // May be null; if null the default implementation (pending_addresses is non-empty) is used.
  OptionalHasPendingFn has_pending;
  // Get the highest found index.
  // May be null; if null returns None.
  OptionalGetHighestFoundIndexFn highest_found_index;
  // Optional destructor for cleanup. May be null.
  OptionalDestroyProviderFn destroy;
} AddressProviderVTable;

// FFI-compatible address provider using callbacks
typedef struct AddressProviderFFI {
  // Opaque context pointer passed to all callbacks
  void *context;
  // Pointer to the vtable containing callback functions
  const struct AddressProviderVTable *vtable;
} AddressProviderFFI;

// Configuration for address synchronization (FFI-compatible)
typedef struct DashSDKAddressSyncConfig {
  // Minimum privacy count - subtrees smaller than this will be expanded
  // to include ancestor subtrees for better privacy.
  //
  // Higher values provide better privacy but may increase the number of
  // elements returned per query.
  //
  // Default: 32
  uint64_t min_privacy_count;
  // Maximum concurrent branch queries.
  //
  // Higher values can speed up synchronization but increase memory usage
  // and network load.
  //
  // Default: 10
  uint32_t max_concurrent_requests;
  // Maximum number of iterations (safety limit).
  //
  // The sync process iterates until all addresses are resolved. This limit
  // prevents infinite loops in case of unexpected behavior.
  //
  // Default: 50
  uint32_t max_iterations;
} DashSDKAddressSyncConfig;

// Error structure returned by FFI functions
typedef struct DashSDKError {
  // Error code
  enum DashSDKErrorCode code;
  // Human-readable error message (null-terminated C string)
  // Caller must free this with dash_sdk_error_free
  char *message;
} DashSDKError;

// Result type for FFI functions that return data
typedef struct DashSDKResult {
  // Type of data being returned
  enum DashSDKResultDataType data_type;
  // Pointer to the result data (null on error)
  void *data;
  // Error information (null on success)
  struct DashSDKError *error;
} DashSDKResult;

// Opaque handle to a context provider
typedef struct ContextProviderHandle {
  uint8_t private_[0];
} ContextProviderHandle;

// Result type for FFI callbacks
typedef struct CallbackResult {
  bool success;
  int32_t error_code;
  const char *error_message;
} CallbackResult;

// Function pointer type for getting platform activation height
typedef struct CallbackResult (*GetPlatformActivationHeightFn)(void *handle, uint32_t *out_height);

// Function pointer type for getting quorum public key
typedef struct CallbackResult (*GetQuorumPublicKeyFn)(void *handle, uint32_t quorum_type, const uint8_t *quorum_hash, uint32_t core_chain_locked_height, uint8_t *out_pubkey);

// Container for context provider callbacks
typedef struct ContextProviderCallbacks {
  // Handle to the Core SDK instance
  void *core_handle;
  // Function to get platform activation height
  GetPlatformActivationHeightFn get_platform_activation_height;
  // Function to get quorum public key
  GetQuorumPublicKeyFn get_quorum_public_key;
} ContextProviderCallbacks;

// Result structure for data contract fetch with serialization
typedef struct DashSDKDataContractFetchResult {
  // Handle to the data contract (null on error or if not requested)
  struct DataContractHandle *contract_handle;
  // JSON representation of the contract (null on error or if not requested)
  char *json_string;
  // Serialized contract bytes (null on error or if not requested)
  uint8_t *serialized_data;
  // Length of serialized data
  uintptr_t serialized_data_len;
  // Error information (null on success)
  struct DashSDKError *error;
} DashSDKDataContractFetchResult;

// Document creation parameters
typedef struct DashSDKDocumentCreateParams {
  // Data contract ID (base58 encoded)
  const char *data_contract_id;
  // Document type name
  const char *document_type;
  // Owner identity ID (base58 encoded)
  const char *owner_identity_id;
  // JSON string of document properties
  const char *properties_json;
} DashSDKDocumentCreateParams;

// Document creation result containing handle and entropy
typedef struct DashSDKDocumentCreateResult {
  // Handle to the created document
  struct DocumentHandle *document_handle;
  // Entropy used for document ID generation (32 bytes)
  uint8_t entropy[32];
} DashSDKDocumentCreateResult;

// Document handle creation parameters
typedef struct DashSDKDocumentHandleParams {
  // Document ID (base58 encoded)
  const char *id;
  // Data contract ID (base58 encoded)
  const char *data_contract_id;
  // Document type name
  const char *document_type;
  // Owner identity ID (base58 encoded)
  const char *owner_identity_id;
  // JSON string of document properties
  const char *properties_json;
  // Optional revision number (0 means no revision)
  uint64_t revision;
} DashSDKDocumentHandleParams;

// Token payment information for transactions
typedef struct DashSDKTokenPaymentInfo {
  // Payment token contract ID (32 bytes), null for same contract
  const uint8_t (*payment_token_contract_id)[32];
  // Token position within the contract (0-based index)
  uint16_t token_contract_position;
  // Minimum token cost (0 means no minimum)
  uint64_t minimum_token_cost;
  // Maximum token cost (0 means no maximum)
  uint64_t maximum_token_cost;
  // Who pays the gas fees
  enum DashSDKGasFeesPaidBy gas_fees_paid_by;
} DashSDKTokenPaymentInfo;

// Put settings for platform operations
typedef struct DashSDKPutSettings {
  // Timeout for establishing a connection (milliseconds), 0 means use default
  uint64_t connect_timeout_ms;
  // Timeout for single request (milliseconds), 0 means use default
  uint64_t timeout_ms;
  // Number of retries in case of failed requests, 0 means use default
  uint32_t retries;
  // Ban DAPI address if node not responded or responded with error
  bool ban_failed_address;
  // Identity nonce stale time in seconds, 0 means use default
  uint64_t identity_nonce_stale_time_s;
  // User fee increase (additional percentage of processing fee), 0 means no increase
  uint16_t user_fee_increase;
  // Enable signing with any security level (for debugging)
  bool allow_signing_with_any_security_level;
  // Enable signing with any purpose (for debugging)
  bool allow_signing_with_any_purpose;
  // Wait timeout in milliseconds, 0 means use default
  uint64_t wait_timeout_ms;
} DashSDKPutSettings;

// State transition creation options for advanced use cases
typedef struct DashSDKStateTransitionCreationOptions {
  // Allow signing with any security level (for debugging)
  bool allow_signing_with_any_security_level;
  // Allow signing with any purpose (for debugging)
  bool allow_signing_with_any_purpose;
  // Batch feature version (0 means use default)
  uint16_t batch_feature_version;
  // Method feature version (0 means use default)
  uint16_t method_feature_version;
  // Base feature version (0 means use default)
  uint16_t base_feature_version;
} DashSDKStateTransitionCreationOptions;

// Document field value
typedef struct DashSDKDocumentField {
  // Field name (null-terminated)
  char *name;
  // Field type
  enum DashSDKDocumentFieldType field_type;
  // Field value as string representation (null-terminated)
  // For complex types, this will be JSON-encoded
  char *value;
  // Raw integer value (for Integer type)
  int64_t int_value;
  // Raw float value (for Float type)
  double float_value;
  // Raw boolean value (for Boolean type)
  bool bool_value;
} DashSDKDocumentField;

// Document information
typedef struct DashSDKDocumentInfo {
  // Document ID as hex string (null-terminated)
  char *id;
  // Owner ID as hex string (null-terminated)
  char *owner_id;
  // Data contract ID as hex string (null-terminated)
  char *data_contract_id;
  // Document type (null-terminated)
  char *document_type;
  // Revision number
  uint64_t revision;
  // Created at timestamp (milliseconds since epoch)
  int64_t created_at;
  // Updated at timestamp (milliseconds since epoch)
  int64_t updated_at;
  // Number of data fields
  uintptr_t data_fields_count;
  // Array of data fields
  struct DashSDKDocumentField *data_fields;
} DashSDKDocumentInfo;

// Document search parameters
typedef struct DashSDKDocumentSearchParams {
  // Data contract handle
  const struct DataContractHandle *data_contract_handle;
  // Document type name
  const char *document_type;
  // JSON string of where clauses (optional)
  const char *where_json;
  // JSON string of order by clauses (optional)
  const char *order_by_json;
  // Limit number of results (0 = default)
  uint32_t limit;
  // Start from index (for pagination)
  uint32_t start_at;
} DashSDKDocumentSearchParams;

// Represents a simple name to timestamp mapping
typedef struct DashSDKNameTimestamp {
  // The name
  char *name;
  // End timestamp in milliseconds
  uint64_t end_time;
} DashSDKNameTimestamp;

// Represents a list of name-timestamp pairs
typedef struct DashSDKNameTimestampList {
  // Array of name-timestamp pairs
  struct DashSDKNameTimestamp *entries;
  // Number of entries
  uintptr_t count;
} DashSDKNameTimestampList;

// Represents a contender in a contested DPNS name
typedef struct DashSDKContender {
  // Identity ID of the contender (base58 string)
  char *identity_id;
  // Vote count for this contender
  uint32_t vote_count;
} DashSDKContender;

// Represents contest information for a DPNS name
typedef struct DashSDKContestInfo {
  // Array of contenders
  struct DashSDKContender *contenders;
  // Number of contenders
  uintptr_t contender_count;
  // Abstain vote tally (0 if none)
  uint32_t abstain_votes;
  // Lock vote tally (0 if none)
  uint32_t lock_votes;
  // End time in milliseconds since epoch
  uint64_t end_time;
  // Whether there is a winner
  bool has_winner;
} DashSDKContestInfo;

// Represents a contested DPNS name entry
typedef struct DashSDKContestedName {
  // The contested name
  char *name;
  // Contest information
  struct DashSDKContestInfo contest_info;
} DashSDKContestedName;

// Represents a list of contested names
typedef struct DashSDKContestedNamesList {
  // Array of contested names
  struct DashSDKContestedName *names;
  // Number of names
  uintptr_t count;
} DashSDKContestedNamesList;

// Result structure for DPNS registration
typedef struct DpnsRegistrationResult {
  // JSON representation of the preorder document
  char *preorder_document_json;
  // JSON representation of the domain document
  char *domain_document_json;
  // The full domain name (e.g., "alice.dash")
  char *full_domain_name;
} DpnsRegistrationResult;

// Public key data for creating identity
typedef struct DashSDKPublicKeyData {
  // Key ID (0-255)
  uint8_t id;
  // Key purpose (0-6)
  uint8_t purpose;
  // Security level (0-3)
  uint8_t security_level;
  // Key type (0-4)
  uint8_t key_type;
  // Whether key is read-only
  bool read_only;
  // Public key data pointer
  const uint8_t *data;
  // Public key data length
  uintptr_t data_len;
  // Disabled timestamp (0 if not disabled)
  uint64_t disabled_at;
} DashSDKPublicKeyData;

// Identity information
typedef struct DashSDKIdentityInfo {
  // Identity ID as hex string (null-terminated)
  char *id;
  // Balance in credits
  uint64_t balance;
  // Revision number
  uint64_t revision;
  // Public keys count
  uint32_t public_keys_count;
} DashSDKIdentityInfo;

// Result structure for credit transfer operations
typedef struct DashSDKTransferCreditsResult {
  // Sender's final balance after transfer
  uint64_t sender_balance;
  // Receiver's final balance after transfer
  uint64_t receiver_balance;
} DashSDKTransferCreditsResult;

// SDK configuration
typedef struct DashSDKConfig {
  // Network to connect to
  enum DashSDKNetwork network;
  // Comma-separated list of DAPI addresses (e.g., "http://127.0.0.1:3000,http://127.0.0.1:3001")
  // If null or empty, will use mock SDK
  const char *dapi_addresses;
  // Skip asset lock proof verification (for testing)
  bool skip_asset_lock_proof_verification;
  // Number of retries for failed requests
  uint32_t request_retry_count;
  // Timeout for requests in milliseconds
  uint64_t request_timeout_ms;
} DashSDKConfig;

// Handle for Core SDK that can be passed to Platform SDK
// This matches the definition from dash_spv_ffi.h
typedef struct CoreSDKHandle {
  void *client;
} CoreSDKHandle;

// Extended SDK configuration with context provider support
typedef struct DashSDKConfigExtended {
  // Base SDK configuration
  struct DashSDKConfig base_config;
  // Optional context provider handle
  struct ContextProviderHandle *context_provider;
  // Optional Core SDK handle for automatic context provider creation
  struct CoreSDKHandle *core_sdk_handle;
} DashSDKConfigExtended;

// Function pointer type for signing callback from iOS/external code
// Returns pointer to allocated byte array (caller must free with dash_sdk_bytes_free)
// Returns null on error
typedef uint8_t *(*SignCallback)(const void *signer, const uint8_t *identity_public_key_bytes, uintptr_t identity_public_key_len, const uint8_t *data, uintptr_t data_len, uintptr_t *result_len);

// Function pointer type for can_sign_with callback from iOS/external code
typedef bool (*CanSignCallback)(const void *signer, const uint8_t *identity_public_key_bytes, uintptr_t identity_public_key_len);

// Function pointer type for destructor callback
// This is an Option to allow for NULL pointers from C
typedef void (*DestroyCallback)(void *signer);

// Signature result structure
typedef struct DashSDKSignature {
  uint8_t *signature;
  uintptr_t signature_len;
} DashSDKSignature;

// Token burn parameters
typedef struct DashSDKTokenBurnParams {
  // Token contract ID (Base58 encoded) - mutually exclusive with serialized_contract
  const char *token_contract_id;
  // Serialized data contract (bincode) - mutually exclusive with token_contract_id
  const uint8_t *serialized_contract;
  // Length of serialized contract data
  uintptr_t serialized_contract_len;
  // Token position in the contract (defaults to 0 if not specified)
  uint16_t token_position;
  // Amount to burn
  uint64_t amount;
  // Optional public note
  const char *public_note;
} DashSDKTokenBurnParams;

// Token claim parameters
typedef struct DashSDKTokenClaimParams {
  // Token contract ID (Base58 encoded) - mutually exclusive with serialized_contract
  const char *token_contract_id;
  // Serialized data contract (bincode) - mutually exclusive with token_contract_id
  const uint8_t *serialized_contract;
  // Length of serialized contract data
  uintptr_t serialized_contract_len;
  // Token position in the contract (defaults to 0 if not specified)
  uint16_t token_position;
  // Distribution type (PreProgrammed or Perpetual)
  enum DashSDKTokenDistributionType distribution_type;
  // Optional public note
  const char *public_note;
} DashSDKTokenClaimParams;

// Token mint parameters
typedef struct DashSDKTokenMintParams {
  // Token contract ID (Base58 encoded) - mutually exclusive with serialized_contract
  const char *token_contract_id;
  // Serialized data contract (bincode) - mutually exclusive with token_contract_id
  const uint8_t *serialized_contract;
  // Length of serialized contract data
  uintptr_t serialized_contract_len;
  // Token position in the contract (defaults to 0 if not specified)
  uint16_t token_position;
  // Recipient identity ID (32 bytes) - optional
  const uint8_t *recipient_id;
  // Amount to mint
  uint64_t amount;
  // Optional public note
  const char *public_note;
} DashSDKTokenMintParams;

// Token transfer parameters
typedef struct DashSDKTokenTransferParams {
  // Token contract ID (Base58 encoded) - mutually exclusive with serialized_contract
  const char *token_contract_id;
  // Serialized data contract (bincode) - mutually exclusive with token_contract_id
  const uint8_t *serialized_contract;
  // Length of serialized contract data
  uintptr_t serialized_contract_len;
  // Token position in the contract (defaults to 0 if not specified)
  uint16_t token_position;
  // Recipient identity ID (32 bytes)
  const uint8_t *recipient_id;
  // Amount to transfer
  uint64_t amount;
  // Optional public note
  const char *public_note;
  // Optional private encrypted note
  const char *private_encrypted_note;
  // Optional shared encrypted note
  const char *shared_encrypted_note;
} DashSDKTokenTransferParams;

// Token configuration update parameters
typedef struct DashSDKTokenConfigUpdateParams {
  // Token contract ID (Base58 encoded) - mutually exclusive with serialized_contract
  const char *token_contract_id;
  // Serialized data contract (bincode) - mutually exclusive with token_contract_id
  const uint8_t *serialized_contract;
  // Length of serialized contract data
  uintptr_t serialized_contract_len;
  // Token position in the contract (defaults to 0 if not specified)
  uint16_t token_position;
  // The type of configuration update
  enum DashSDKTokenConfigUpdateType update_type;
  // For MaxSupply updates - the new max supply (0 for no limit)
  uint64_t amount;
  // For boolean updates like MintingAllowChoosingDestination
  bool bool_value;
  // For identity-based updates - identity ID (32 bytes)
  const uint8_t *identity_id;
  // For group-based updates - the group position
  uint16_t group_position;
  // For permission updates - the authorized action takers
  enum DashSDKAuthorizedActionTakers action_takers;
  // Optional public note
  const char *public_note;
} DashSDKTokenConfigUpdateParams;

// Token destroy frozen funds parameters
typedef struct DashSDKTokenDestroyFrozenFundsParams {
  // Token contract ID (Base58 encoded) - mutually exclusive with serialized_contract
  const char *token_contract_id;
  // Serialized data contract (bincode) - mutually exclusive with token_contract_id
  const uint8_t *serialized_contract;
  // Length of serialized contract data
  uintptr_t serialized_contract_len;
  // Token position in the contract (defaults to 0 if not specified)
  uint16_t token_position;
  // The frozen identity whose funds to destroy (32 bytes)
  const uint8_t *frozen_identity_id;
  // Optional public note
  const char *public_note;
} DashSDKTokenDestroyFrozenFundsParams;

// Token emergency action parameters
typedef struct DashSDKTokenEmergencyActionParams {
  // Token contract ID (Base58 encoded) - mutually exclusive with serialized_contract
  const char *token_contract_id;
  // Serialized data contract (bincode) - mutually exclusive with token_contract_id
  const uint8_t *serialized_contract;
  // Length of serialized contract data
  uintptr_t serialized_contract_len;
  // Token position in the contract (defaults to 0 if not specified)
  uint16_t token_position;
  // The emergency action to perform
  enum DashSDKTokenEmergencyAction action;
  // Optional public note
  const char *public_note;
} DashSDKTokenEmergencyActionParams;

// Token freeze/unfreeze parameters
typedef struct DashSDKTokenFreezeParams {
  // Token contract ID (Base58 encoded) - mutually exclusive with serialized_contract
  const char *token_contract_id;
  // Serialized data contract (bincode) - mutually exclusive with token_contract_id
  const uint8_t *serialized_contract;
  // Length of serialized contract data
  uintptr_t serialized_contract_len;
  // Token position in the contract (defaults to 0 if not specified)
  uint16_t token_position;
  // The identity to freeze/unfreeze (32 bytes)
  const uint8_t *target_identity_id;
  // Optional public note
  const char *public_note;
} DashSDKTokenFreezeParams;

// Token purchase parameters
typedef struct DashSDKTokenPurchaseParams {
  // Token contract ID (Base58 encoded) - mutually exclusive with serialized_contract
  const char *token_contract_id;
  // Serialized data contract (bincode) - mutually exclusive with token_contract_id
  const uint8_t *serialized_contract;
  // Length of serialized contract data
  uintptr_t serialized_contract_len;
  // Token position in the contract (defaults to 0 if not specified)
  uint16_t token_position;
  // Amount of tokens to purchase
  uint64_t amount;
  // Total agreed price in credits
  uint64_t total_agreed_price;
} DashSDKTokenPurchaseParams;

// Token price entry for tiered pricing
typedef struct DashSDKTokenPriceEntry {
  // Token amount threshold
  uint64_t amount;
  // Price in credits for this amount
  uint64_t price;
} DashSDKTokenPriceEntry;

// Token set price parameters
typedef struct DashSDKTokenSetPriceParams {
  // Token contract ID (Base58 encoded) - mutually exclusive with serialized_contract
  const char *token_contract_id;
  // Serialized data contract (bincode) - mutually exclusive with token_contract_id
  const uint8_t *serialized_contract;
  // Length of serialized contract data
  uintptr_t serialized_contract_len;
  // Token position in the contract (defaults to 0 if not specified)
  uint16_t token_position;
  // Pricing type
  enum DashSDKTokenPricingType pricing_type;
  // For SinglePrice - the price in credits (ignored for SetPrices)
  uint64_t single_price;
  // For SetPrices - array of price entries (ignored for SinglePrice)
  const struct DashSDKTokenPriceEntry *price_entries;
  // Number of price entries
  uint32_t price_entries_count;
  // Optional public note
  const char *public_note;
} DashSDKTokenSetPriceParams;

// Binary data container for results
typedef struct DashSDKBinaryData {
  // Pointer to the data
  uint8_t *data;
  // Length of the data
  uintptr_t len;
} DashSDKBinaryData;

// Single entry in an identity balance map
typedef struct DashSDKIdentityBalanceEntry {
  // Identity ID (32 bytes)
  uint8_t identity_id[32];
  // Balance in credits (u64::MAX means identity not found)
  uint64_t balance;
} DashSDKIdentityBalanceEntry;

// Map of identity IDs to balances
typedef struct DashSDKIdentityBalanceMap {
  // Array of entries
  struct DashSDKIdentityBalanceEntry *entries;
  // Number of entries
  uintptr_t count;
} DashSDKIdentityBalanceMap;

// Unified SDK handle containing both Core and Platform SDKs
typedef struct UnifiedSDKHandle {
  FFIDashSpvClient *core_client;
  struct dash_sdk_handle_t *platform_sdk;
  bool integration_enabled;
} UnifiedSDKHandle;

// Unified SDK configuration combining both Core and Platform settings
typedef struct UnifiedSDKConfig {
  // Core SDK configuration (ignored if core feature disabled)
  const FFIClientConfig *core_config;
  // Platform SDK configuration
  struct DashSDKConfig platform_config;
  // Whether to enable cross-layer integration
  bool enable_integration;
} UnifiedSDKConfig;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

// Initialize the FFI library.
// This should be called once at app startup before using any other functions.
 void dash_sdk_init(void) ;

// Enable logging with the specified level
// Level values: 0 = Error, 1 = Warn, 2 = Info, 3 = Debug, 4 = Trace
 void dash_sdk_enable_logging(uint8_t level) ;

// Get the version of the Dash SDK FFI library
 const char *dash_sdk_version(void) ;

// Synchronize address balances using trunk/branch chunk queries.
//
// This function discovers address balances for addresses supplied by the provider,
// using privacy-preserving chunk queries. It supports HD wallet gap limit behavior
// where finding a used address extends the search range.
//
// # Safety
// - `sdk_handle` must be a valid SDK handle created by this SDK
// - `provider` must be a valid pointer to an AddressProviderFFI structure
// - `config` may be null (uses defaults) or a valid pointer to DashSDKAddressSyncConfig
// - The returned result must be freed with `dash_sdk_address_sync_result_free`
 struct DashSDKAddressSyncResult *dash_sdk_sync_address_balances(const struct dash_sdk_handle_t *sdk_handle, struct AddressProviderFFI *provider, const struct DashSDKAddressSyncConfig *config) ;

// Synchronize address balances and return result with error information.
//
// This is an alternative version that returns a DashSDKResult for better error handling.
//
// # Safety
// - `sdk_handle` must be a valid SDK handle created by this SDK
// - `provider` must be a valid pointer to an AddressProviderFFI structure
// - `config` may be null (uses defaults) or a valid pointer to DashSDKAddressSyncConfig
 struct DashSDKResult dash_sdk_sync_address_balances_with_result(const struct dash_sdk_handle_t *sdk_handle, struct AddressProviderFFI *provider, const struct DashSDKAddressSyncConfig *config) ;

// Free an address sync result
//
// # Safety
// - `result` must be a valid pointer returned by `dash_sdk_sync_address_balances`
//   or null (no-op)
// - After this call, the result must not be used again
 void dash_sdk_address_sync_result_free(struct DashSDKAddressSyncResult *result) ;

// Create an address provider with callbacks
//
// This creates an FFI-compatible address provider that uses callbacks
// for all operations.
//
// # Safety
// - `vtable` must be a valid pointer to an AddressProviderVTable structure
// - `context` is an opaque pointer that will be passed to all callbacks
// - The returned provider must be freed with `dash_sdk_address_provider_free`
 struct AddressProviderFFI *dash_sdk_address_provider_create(const struct AddressProviderVTable *vtable, void *context) ;

// Free an address provider
//
// # Safety
// - `provider` must be a valid pointer returned by `dash_sdk_address_provider_create`
//   or null (no-op)
// - After this call, the provider must not be used again
 void dash_sdk_address_provider_free(struct AddressProviderFFI *provider) ;

// Get the total balance from a sync result
//
// # Safety
// - `result` must be a valid pointer to a DashSDKAddressSyncResult
 uint64_t dash_sdk_address_sync_result_total_balance(const struct DashSDKAddressSyncResult *result) ;

// Get the count of addresses with non-zero balance
//
// # Safety
// - `result` must be a valid pointer to a DashSDKAddressSyncResult
 uintptr_t dash_sdk_address_sync_result_non_zero_count(const struct DashSDKAddressSyncResult *result) ;

// Free a pending address list
//
// This should be called to clean up memory after the sync operation is complete
// if the caller allocated the list dynamically.
//
// Note: This only frees the list structure, not the key data which should be
// managed by the caller.
//
// # Safety
// - `list` must be a valid pointer to a DashSDKPendingAddressList
//   or null (no-op)
 void dash_sdk_pending_address_list_free(struct DashSDKPendingAddressList *list) ;

// Register Core SDK handle and setup callback bridge with Platform SDK
//
// This function implements the core pattern from dash-unified-ffi-old:
// 1. Takes a Core SDK handle
// 2. Creates callback wrappers for the functions Platform SDK needs
// 3. Registers these callbacks with Platform SDK's context provider system
//
// # Safety
// - `core_handle` must be a valid Core SDK handle that remains valid for the SDK lifetime
// - This function should be called once after creating both Core and Platform SDK instances
 int32_t dash_unified_register_core_sdk_handle(void *core_handle) ;

// Initialize the unified SDK system with callback bridge support
//
// This function initializes both Core SDK and Platform SDK and sets up
// the callback bridge pattern for inter-SDK communication.
 int32_t dash_unified_init(void) ;

// Get unified SDK version information including both Core and Platform components
 const char *dash_unified_version(void) ;

// Check if unified SDK has both Core and Platform support
 bool dash_unified_has_full_support(void) ;

// Fetches contested resource identity votes
//
// # Parameters
// * `sdk_handle` - Handle to the SDK instance
// * `identity_id` - Base58-encoded identity identifier
// * `limit` - Maximum number of votes to return (optional, 0 for no limit)
// * `offset` - Number of votes to skip (optional, 0 for no offset)
// * `order_ascending` - Whether to order results in ascending order
//
// # Returns
// * JSON array of votes or null if not found
// * Error message if operation fails
//
// # Safety
// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
// - `identity_id` must be a valid, non-null pointer to a NUL-terminated C string that remains valid during the call.
// - `limit`, `offset`, and `order_ascending` are passed by value; no references are retained.
// - On success, the returned `DashSDKResult` may contain a heap-allocated C string; the caller must free
//   it using the SDK's free routine. The result can also contain no data (null pointer).
// - All pointers provided to this function must be readable and valid.
 struct DashSDKResult dash_sdk_contested_resource_get_identity_votes(const struct dash_sdk_handle_t *sdk_handle, const char *identity_id, uint32_t limit, uint32_t offset, bool order_ascending) ;

// Fetches contested resources
//
// # Parameters
// * `sdk_handle` - Handle to the SDK instance
// * `contract_id` - Base58-encoded contract identifier
// * `document_type_name` - Name of the document type
// * `index_name` - Name of the index
// * `start_index_values_json` - JSON array of hex-encoded start index values
// * `end_index_values_json` - JSON array of hex-encoded end index values
// * `count` - Maximum number of resources to return
// * `order_ascending` - Whether to order results in ascending order
//
// # Returns
// * JSON array of contested resources or null if not found
// * Error message if operation fails
//
// # Safety
// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
// - All C string pointers (`contract_id`, `document_type_name`, `index_name`,
//   `start_index_values_json`, `end_index_values_json`) must be either null (when documented as optional)
//   or valid pointers to NUL-terminated strings that remain valid for the duration of the call.
// - The function reads the `count` and `order_ascending` by value and does not retain references.
// - On success, the returned `DashSDKResult` may contain a heap-allocated C string; the caller must
//   free it using the SDK-provided free routine. The result can also contain no data (null pointer).
// - All pointers passed in must point to readable memory; behavior is undefined if they are dangling.
 struct DashSDKResult dash_sdk_contested_resource_get_resources(const struct dash_sdk_handle_t *sdk_handle, const char *contract_id, const char *document_type_name, const char *index_name, const char *start_index_values_json, const char *end_index_values_json, uint32_t count, bool order_ascending) ;

// Fetches contested resource vote state
//
// # Parameters
// * `sdk_handle` - Handle to the SDK instance
// * `contract_id` - Base58-encoded contract identifier
// * `document_type_name` - Name of the document type
// * `index_name` - Name of the index
// * `index_values_json` - JSON array of hex-encoded index values
// * `result_type` - Result type (0=DOCUMENTS, 1=VOTE_TALLY, 2=DOCUMENTS_AND_VOTE_TALLY)
// * `allow_include_locked_and_abstaining_vote_tally` - Whether to include locked and abstaining votes
// * `count` - Maximum number of results to return
//
// # Returns
// * JSON array of contenders or null if not found
// * Error message if operation fails
//
// # Safety
// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
// - All C string pointers (`contract_id`, `document_type_name`, `index_name`, `index_values_json`)
//   must be valid, non-null pointers to NUL-terminated strings that remain valid for the duration of the call.
// - `result_type` and `allow_include_locked_and_abstaining_vote_tally` are passed by value.
// - The returned result may contain a heap-allocated C string which must be freed by the caller using
//   the SDK's free routine. It may also contain no data (null pointer) on success.
// - All pointers must point to readable memory; passing invalid or dangling pointers results in undefined behavior.
 struct DashSDKResult dash_sdk_contested_resource_get_vote_state(const struct dash_sdk_handle_t *sdk_handle, const char *contract_id, const char *document_type_name, const char *index_name, const char *index_values_json, uint8_t result_type, bool allow_include_locked_and_abstaining_vote_tally, uint32_t count) ;

// Fetches voters for a contested resource identity
//
// # Parameters
// * `sdk_handle` - Handle to the SDK instance
// * `contract_id` - Base58-encoded contract identifier
// * `document_type_name` - Name of the document type
// * `index_name` - Name of the index
// * `index_values_json` - JSON array of hex-encoded index values
// * `contestant_id` - Base58-encoded contestant identifier
// * `count` - Maximum number of voters to return
// * `order_ascending` - Whether to order results in ascending order
//
// # Returns
// * JSON array of voters or null if not found
// * Error message if operation fails
//
// # Safety
// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
// - All C string pointers (`contract_id`, `document_type_name`, `index_name`, `index_values_json`, `contestant_id`)
//   must be valid, non-null pointers to NUL-terminated strings that remain valid for the duration of the call.
// - The function reads `count` and `order_ascending` by value and does not retain references.
// - On success, the returned `DashSDKResult` may contain a heap-allocated C string; the caller must
//   free it using the SDK-provided free routine. The result can also contain no data (null pointer).
// - All pointers must reference readable memory; passing invalid pointers leads to undefined behavior.
 struct DashSDKResult dash_sdk_contested_resource_get_voters_for_identity(const struct dash_sdk_handle_t *sdk_handle, const char *contract_id, const char *document_type_name, const char *index_name, const char *index_values_json, const char *contestant_id, uint32_t count, bool order_ascending) ;

// Create a context provider from callbacks
//
// # Safety
// - `callbacks` must contain valid function pointers
 struct ContextProviderHandle *dash_sdk_context_provider_from_callbacks(const struct ContextProviderCallbacks *callbacks) ;

// Destroy a context provider handle
//
// # Safety
// - `handle` must be a valid context provider handle or null
 void dash_sdk_context_provider_destroy(struct ContextProviderHandle *handle) ;

// Validate that a private key corresponds to a public key using DPP's public_key_data_from_private_key_data
//
// # Safety
// - `private_key_hex` and `public_key_hex` must be valid, non-null pointers to NUL-terminated C strings that
//   remain valid for the duration of the call.
// - `key_type` and `is_testnet` are passed by value; no references are retained.
// - On success, the returned `DashSDKResult` contains a heap-allocated C string pointer which must be freed using
//   the SDK's free routine. It may also return no data (null pointer) to indicate success without payload.
// - Passing invalid or dangling pointers results in undefined behavior.
 struct DashSDKResult dash_sdk_validate_private_key_for_public_key(const char *private_key_hex, const char *public_key_hex, uint8_t key_type, bool is_testnet) ;

// Convert private key to WIF format
//
// # Safety
// - `private_key_hex` must be a valid, non-null pointer to a NUL-terminated C string representing a 32-byte hex key
//   and remain valid for the duration of the call.
// - `is_testnet` is passed by value.
// - On success, the returned `DashSDKResult` contains a heap-allocated C string pointer which must be freed using
//   the SDK's free routine.
 struct DashSDKResult dash_sdk_private_key_to_wif(const char *private_key_hex, bool is_testnet) ;

// Get public key data from private key data
//
// # Safety
// - `private_key_hex` must be a valid, non-null pointer to a NUL-terminated C string representing a 32-byte hex key
//   and remain valid for the duration of the call.
// - `key_type` and `is_testnet` are passed by value; no references are retained.
// - On success, the returned `DashSDKResult` contains a heap-allocated C string pointer which must be freed using
//   the SDK's free routine.
 struct DashSDKResult dash_sdk_public_key_data_from_private_key_data(const char *private_key_hex, uint8_t key_type, bool is_testnet) ;

// Create a new data contract
//
// # Safety
// - `sdk_handle`, `owner_identity_handle`, and `documents_schema_json` must be valid, non-null pointers.
// - `documents_schema_json` must point to a NUL-terminated C string valid for the duration of the call.
// - On success, returns a heap-allocated handle which must be destroyed with the SDK's destroy function.
 struct DashSDKResult dash_sdk_data_contract_create(struct dash_sdk_handle_t *sdk_handle, const struct IdentityHandle *owner_identity_handle, const char *documents_schema_json) ;

// Destroy a data contract handle
//
// # Safety
// - `handle` must be a pointer previously returned by this SDK or null (no-op).
// - After this call, `handle` becomes invalid and must not be used again.
 void dash_sdk_data_contract_destroy(struct DataContractHandle *handle) ;

// Put data contract to platform (broadcast state transition)
//
// # Safety
// - `sdk_handle`, `data_contract_handle`, `identity_public_key_handle`, and `signer_handle` must be valid, non-null pointers.
// - On success, returns serialized data; any heap memory inside the result must be freed using SDK routines.
 struct DashSDKResult dash_sdk_data_contract_put_to_platform(struct dash_sdk_handle_t *sdk_handle, const struct DataContractHandle *data_contract_handle, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle) ;

// Put data contract to platform and wait for confirmation (broadcast state transition and wait for response)
//
// # Safety
// - Same requirements as `dash_sdk_data_contract_put_to_platform`.
// - The function may block while waiting for confirmation; input pointers must remain valid throughout.
// - On success, returns a heap-allocated handle which must be destroyed with the SDK's destroy function.
 struct DashSDKResult dash_sdk_data_contract_put_to_platform_and_wait(struct dash_sdk_handle_t *sdk_handle, const struct DataContractHandle *data_contract_handle, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle) ;

// Fetch a data contract by ID
//
// # Safety
// - `sdk_handle` and `contract_id` must be valid, non-null pointers.
// - `contract_id` must point to a NUL-terminated C string valid for the duration of the call.
// - On success, returns a heap-allocated handle which must be destroyed with the SDK's destroy function.
 struct DashSDKResult dash_sdk_data_contract_fetch(const struct dash_sdk_handle_t *sdk_handle, const char *contract_id) ;

// Fetch a data contract by ID and return as JSON
//
// # Safety
// - `sdk_handle` and `contract_id` must be valid, non-null pointers.
// - `contract_id` must point to a NUL-terminated C string that remains valid for the duration of the call.
// - On success, returns a heap-allocated C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_data_contract_fetch_json(const struct dash_sdk_handle_t *sdk_handle, const char *contract_id) ;

// Fetch multiple data contracts by their IDs
//
// # Safety
// - `sdk_handle` and `contract_ids` must be valid, non-null pointers.
// - `contract_ids` must point to a NUL-terminated C string containing either a JSON array of Base58 IDs or a comma-separated list; it must remain valid for the duration of the call.
// - On success, returns a heap-allocated C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `contract_ids`: Comma-separated list of Base58-encoded contract IDs
//
// # Returns
// JSON string containing contract IDs mapped to their data contracts
 struct DashSDKResult dash_sdk_data_contracts_fetch_many(const struct dash_sdk_handle_t *sdk_handle, const char *contract_ids) ;

// Fetch a data contract by ID with serialization
//
// # Safety
// - `sdk_handle` and `contract_id` must be valid, non-null pointers.
// - `contract_id` must point to a NUL-terminated C string valid for the duration of the call.
// - The returned result contains heap-allocated buffers/handles depending on flags; caller must free them using
//   `dash_sdk_data_contract_fetch_result_free`.
 struct DashSDKDataContractFetchResult dash_sdk_data_contract_fetch_with_serialization(const struct dash_sdk_handle_t *sdk_handle, const char *contract_id, bool return_json, bool return_serialized) ;

// Free the memory allocated for a data contract fetch result
//
// # Safety
// - `result` must be a pointer previously returned by this SDK or null (no-op).
// - After this call, `result` and all contained pointers become invalid and must not be used again.
 void dash_sdk_data_contract_fetch_result_free(struct DashSDKDataContractFetchResult *result) ;

// Fetch data contract history
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `contract_id`: Base58-encoded contract ID
// - `limit`: Maximum number of history entries to return (0 for default)
// - `offset`: Number of entries to skip (for pagination)
// - `start_at_ms`: Start timestamp in milliseconds (0 for beginning)
//
// # Returns
// JSON string containing the data contract history
//
// # Safety
// - `sdk_handle` and `contract_id` must be valid, non-null pointers.
// - `contract_id` must point to a NUL-terminated C string valid for the duration of the call.
// - On success, returns a heap-allocated C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_data_contract_fetch_history(const struct dash_sdk_handle_t *sdk_handle, const char *contract_id, unsigned int limit, unsigned int offset, uint64_t start_at_ms) ;

// Get schema for a specific document type
//
// # Safety
// - `contract_handle` and `document_type` must be valid, non-null pointers.
// - `document_type` must point to a NUL-terminated C string valid for the duration of the call.
// - Returns a heap-allocated C string pointer on success; caller must free it using SDK routines.
 char *dash_sdk_data_contract_get_schema(const struct DataContractHandle *contract_handle, const char *document_type) ;

// Create a new document
//
// # Safety
// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
// - `params` must be a valid, non-null pointer to a `DashSDKDocumentCreateParams` structure.
// - All C string fields inside `params` (`data_contract_id`, `document_type`, `owner_identity_id`, `properties_json`)
//   must be valid pointers to NUL-terminated strings and remain valid for the duration of the call.
// - On success, the returned `DashSDKResult` contains a heap-allocated handle which must be freed by the caller
//   using the appropriate SDK destroy function.
// - Passing dangling or invalid pointers results in undefined behavior.
 struct DashSDKResult dash_sdk_document_create(struct dash_sdk_handle_t *sdk_handle, const struct DashSDKDocumentCreateParams *params) ;

// Free a document creation result
//
// # Safety
// - `result` must be either null (no-op) or a pointer previously returned by this SDK.
// - After this call, `result` becomes invalid and must not be used again.
 void dash_sdk_document_create_result_free(struct DashSDKDocumentCreateResult *result) ;

// Create a document handle from parameters
// This creates a Document object directly without broadcasting to the network
//
// # Safety
// - `params` must be a valid, non-null pointer to a `DashSDKDocumentHandleParams` structure.
// - All C string fields inside `params` must be valid pointers to NUL-terminated strings and remain valid
//   for the duration of the call.
// - On success, the returned `DashSDKResult` contains a heap-allocated `DocumentHandle` which must be freed by the caller
//   using the appropriate SDK destroy function.
// - Passing dangling or invalid pointers results in undefined behavior.
 struct DashSDKResult dash_sdk_document_make_handle(const struct DashSDKDocumentHandleParams *params) ;

// Delete a document from the platform
//
// # Safety
// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
// - `document_id`, `owner_id`, `data_contract_id`, and `document_type_name` must be valid, non-null pointers to
//   NUL-terminated C strings that remain valid for the duration of the call.
// - `identity_public_key_handle` and `signer_handle` must be valid, non-null pointers to initialized structures.
// - Optional pointers (`token_payment_info`, `put_settings`, `state_transition_creation_options`) may be null; when non-null they must be valid.
// - On success, the result may contain heap-allocated data that must be freed using SDK-provided routines.
 struct DashSDKResult dash_sdk_document_delete(struct dash_sdk_handle_t *sdk_handle, const char *document_id, const char *owner_id, const char *data_contract_id, const char *document_type_name, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKTokenPaymentInfo *token_payment_info, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Delete a document from the platform and wait for confirmation
//
// # Safety
// - Same requirements as `dash_sdk_document_delete` regarding pointer validity and lifetimes.
// - The function may block while waiting for confirmation; input pointers must remain valid throughout.
// - On success, the result may contain heap-allocated data that must be freed using SDK-provided routines.
 struct DashSDKResult dash_sdk_document_delete_and_wait(struct dash_sdk_handle_t *sdk_handle, const char *document_id, const char *owner_id, const char *data_contract_id, const char *document_type_name, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKTokenPaymentInfo *token_payment_info, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Update document price (broadcast state transition)
//
// # Safety
// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
// - `document_handle`, `data_contract_id`, `document_type_name`, `identity_public_key_handle`, and `signer_handle`
//   must be valid, non-null pointers. `data_contract_id` and `document_type_name` must point to NUL-terminated C strings.
// - Optional pointers (`token_payment_info`, `put_settings`, `state_transition_creation_options`) may be null; when non-null they must be valid.
// - On success, the result may contain heap-allocated data that must be freed using SDK-provided routines.
 struct DashSDKResult dash_sdk_document_update_price_of_document(struct dash_sdk_handle_t *sdk_handle, const struct DocumentHandle *document_handle, const char *data_contract_id, const char *document_type_name, uint64_t price, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKTokenPaymentInfo *token_payment_info, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Update document price and wait for confirmation (broadcast state transition and wait for response)
//
// # Safety
// - Same requirements as `dash_sdk_document_update_price_of_document` regarding pointer validity and lifetimes.
// - The function may block while waiting for confirmation; input pointers must remain valid throughout.
// - On success, the result may contain heap-allocated data that must be freed using SDK-provided routines.
 struct DashSDKResult dash_sdk_document_update_price_of_document_and_wait(struct dash_sdk_handle_t *sdk_handle, const struct DocumentHandle *document_handle, const char *data_contract_id, const char *document_type_name, uint64_t price, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKTokenPaymentInfo *token_payment_info, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Purchase document (broadcast state transition)
//
// # Safety
// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
// - `document_handle`, `data_contract_id`, `document_type_name`, `purchaser_id`, `identity_public_key_handle`, and `signer_handle`
//   must be valid, non-null pointers. All C string pointers must point to NUL-terminated strings.
// - Optional pointers (`token_payment_info`, `put_settings`, `state_transition_creation_options`) may be null; when non-null they must be valid.
// - On success, the result may contain heap-allocated data that must be freed using SDK-provided routines.
 struct DashSDKResult dash_sdk_document_purchase(struct dash_sdk_handle_t *sdk_handle, const struct DocumentHandle *document_handle, const char *data_contract_id, const char *document_type_name, uint64_t price, const char *purchaser_id, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKTokenPaymentInfo *token_payment_info, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Purchase document and wait for confirmation (broadcast state transition and wait for response)
//
// # Safety
// - Same requirements as `dash_sdk_document_purchase` regarding pointer validity and lifetimes.
// - The function may block while waiting for confirmation; input pointers must remain valid throughout.
// - On success, the result may contain heap-allocated data that must be freed using SDK-provided routines.
 struct DashSDKResult dash_sdk_document_purchase_and_wait(struct dash_sdk_handle_t *sdk_handle, const struct DocumentHandle *document_handle, const char *data_contract_id, const char *document_type_name, uint64_t price, const char *purchaser_id, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKTokenPaymentInfo *token_payment_info, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Put document to platform (broadcast state transition)
//
// # Safety
// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
// - `document_handle`, `data_contract_id`, `document_type_name`, `entropy`, `identity_public_key_handle`, and `signer_handle`
//   must be valid, non-null pointers. `data_contract_id` and `document_type_name` must point to NUL-terminated C strings.
// - Optional pointers (`token_payment_info`, `put_settings`, `state_transition_creation_options`) may be null; when non-null they must be valid.
// - On success, the result may contain heap-allocated data that must be freed using SDK-provided routines.
// - All pointers must reference readable memory for the duration of the call.
 struct DashSDKResult dash_sdk_document_put_to_platform(struct dash_sdk_handle_t *sdk_handle, const struct DocumentHandle *document_handle, const char *data_contract_id, const char *document_type_name, const uint8_t (*entropy)[32], const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKTokenPaymentInfo *token_payment_info, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Put document to platform and wait for confirmation (broadcast state transition and wait for response)
//
// # Safety
// - Same requirements as `dash_sdk_document_put_to_platform` regarding pointer validity and lifetimes.
// - The function may block while waiting for confirmation; input pointers must remain valid throughout.
// - On success, the result may contain heap-allocated data that must be freed using SDK-provided routines.
 struct DashSDKResult dash_sdk_document_put_to_platform_and_wait(struct dash_sdk_handle_t *sdk_handle, const struct DocumentHandle *document_handle, const char *data_contract_id, const char *document_type_name, const uint8_t (*entropy)[32], const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKTokenPaymentInfo *token_payment_info, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Fetch a document by ID using contract ID (gets contract from trusted provider)
 struct DashSDKResult dash_sdk_document_fetch_by_contract_id(struct dash_sdk_handle_t *sdk_handle, const char *contract_id, const char *document_type, const char *document_id) ;

// Fetch a document by ID (legacy - requires data contract handle)
//
// # Safety
// - `sdk_handle`, `data_contract_handle`, `document_type`, and `document_id` must be valid, non-null pointers.
// - `document_type` and `document_id` must point to NUL-terminated C strings valid for the duration of the call.
// - On success, returns a handle or no data; any heap memory must be freed using SDK routines.
 struct DashSDKResult dash_sdk_document_fetch(const struct dash_sdk_handle_t *sdk_handle, const struct DataContractHandle *data_contract_handle, const char *document_type, const char *document_id) ;

// Get document information
//
// # Safety
// - `document_handle` must be a valid, non-null pointer to a `DocumentHandle` that remains valid for the duration of the call.
// - Returns a heap-allocated `DashSDKDocumentInfo` pointer on success; caller must free it using the SDK-provided free function.
 struct DashSDKDocumentInfo *dash_sdk_document_get_info(const struct DocumentHandle *document_handle) ;

// Search for documents
//
// # Safety
// - `sdk_handle` and `params` must be valid, non-null pointers.
// - All C string pointers inside `params` must point to NUL-terminated strings and remain valid for the duration of the call; optional strings may be null.
// - On success, returns a heap-allocated C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_document_search(const struct dash_sdk_handle_t *sdk_handle, const struct DashSDKDocumentSearchParams *params) ;

// Replace document on platform (broadcast state transition)
//
// # Safety
// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
// - `document_handle`, `data_contract_id`, `document_type_name`, `identity_public_key_handle`, and `signer_handle`
//   must be valid, non-null pointers. `data_contract_id` and `document_type_name` must point to NUL-terminated C strings.
// - Optional pointers (`token_payment_info`, `put_settings`, `state_transition_creation_options`) may be null; when non-null they must be valid.
// - On success, the result may contain heap-allocated data that must be freed using SDK-provided routines.
 struct DashSDKResult dash_sdk_document_replace_on_platform(struct dash_sdk_handle_t *sdk_handle, const struct DocumentHandle *document_handle, const char *data_contract_id, const char *document_type_name, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKTokenPaymentInfo *token_payment_info, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Replace document on platform and wait for confirmation (broadcast state transition and wait for response)
//
// # Safety
// - Same requirements as `dash_sdk_document_replace_on_platform` regarding pointer validity and lifetimes.
// - The function may block while waiting for confirmation; input pointers must remain valid throughout.
// - On success, the result may contain heap-allocated data that must be freed using SDK-provided routines.
 struct DashSDKResult dash_sdk_document_replace_on_platform_and_wait(struct dash_sdk_handle_t *sdk_handle, const struct DocumentHandle *document_handle, const char *data_contract_id, const char *document_type_name, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKTokenPaymentInfo *token_payment_info, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Transfer document to another identity
//
// # Parameters
// - `document_handle`: Handle to the document to transfer
// - `recipient_id`: Base58-encoded ID of the recipient identity
// - `data_contract_handle`: Handle to the data contract
// - `document_type_name`: Name of the document type
// - `identity_public_key_handle`: Public key for signing
// - `signer_handle`: Cryptographic signer
// - `token_payment_info`: Optional token payment information (can be null for defaults)
// - `put_settings`: Optional settings for the operation (can be null for defaults)
//
// # Returns
// Serialized state transition on success
//
// # Safety
// - `sdk_handle`, `document_handle`, `recipient_id`, `data_contract_id`, `document_type_name`, `identity_public_key_handle`, and `signer_handle` must be valid, non-null pointers.
// - All C string pointers must point to NUL-terminated strings valid for the duration of the call.
// - Optional pointers (`token_payment_info`, `put_settings`, `state_transition_creation_options`) may be null; when non-null they must be valid.
// - On success, any heap memory in the result must be freed using SDK routines.
 struct DashSDKResult dash_sdk_document_transfer_to_identity(struct dash_sdk_handle_t *sdk_handle, const struct DocumentHandle *document_handle, const char *recipient_id, const char *data_contract_id, const char *document_type_name, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKTokenPaymentInfo *token_payment_info, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Transfer document to another identity and wait for confirmation
//
// # Parameters
// - `document_handle`: Handle to the document to transfer
// - `recipient_id`: Base58-encoded ID of the recipient identity
// - `data_contract_handle`: Handle to the data contract
// - `document_type_name`: Name of the document type
// - `identity_public_key_handle`: Public key for signing
// - `signer_handle`: Cryptographic signer
// - `token_payment_info`: Optional token payment information (can be null for defaults)
// - `put_settings`: Optional settings for the operation (can be null for defaults)
//
// # Returns
// Handle to the transferred document on success
//
// # Safety
// - Same requirements as `dash_sdk_document_transfer_to_identity` regarding pointer validity and lifetimes.
// - The function may block while waiting for confirmation; input pointers must remain valid throughout.
// - On success, the result may contain heap-allocated data that must be freed using SDK-provided routines.
 struct DashSDKResult dash_sdk_document_transfer_to_identity_and_wait(struct dash_sdk_handle_t *sdk_handle, const struct DocumentHandle *document_handle, const char *recipient_id, const char *data_contract_id, const char *document_type_name, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKTokenPaymentInfo *token_payment_info, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Destroy a document
//
// # Safety
// - `sdk_handle` and `document_handle` must be valid, non-null pointers.
// - Returns a pointer to an error structure on failure; caller must free with `dash_sdk_error_free`.
 struct DashSDKError *dash_sdk_document_destroy(struct dash_sdk_handle_t *sdk_handle, struct DocumentHandle *document_handle) ;

// Destroy a document handle
//
// # Safety
// - `handle` must be a pointer previously returned by this SDK or null (no-op).
// - After this call, `handle` becomes invalid and must not be used again.
 void dash_sdk_document_handle_destroy(struct DocumentHandle *handle) ;

// Free a document handle (alias for destroy)
//
// # Safety
// - Same as `dash_sdk_document_handle_destroy`.
 void dash_sdk_document_free(struct DocumentHandle *handle) ;

// Set document properties from JSON
//
// # Safety
// - `document_handle` and `properties_json` must be valid, non-null pointers.
// - `properties_json` must point to a NUL-terminated C string valid for the duration of the call.
// - Returns an error pointer on failure; caller must free with `dash_sdk_error_free`.
 struct DashSDKError *dash_sdk_document_set_properties(struct DocumentHandle *document_handle, const char *properties_json) ;

// Convert a string to homograph-safe characters by replacing 'o', 'i', and 'l'
// with '0', '1', and '1' respectively to prevent homograph attacks
//
// # Safety
// - `name` must be a valid null-terminated C string
 struct DashSDKResult dash_sdk_dpns_normalize_username(const char *name) ;

// Check if a username is valid according to DPNS rules
//
// A username is valid if:
// - It's between 3 and 63 characters long
// - It starts and ends with alphanumeric characters (a-zA-Z0-9)
// - It contains only alphanumeric characters and hyphens
// - It doesn't have consecutive hyphens
//
// # Safety
// - `name` must be a valid null-terminated C string
//
// # Returns
// - 1 if the username is valid
// - 0 if the username is invalid
// - -1 if there's an error
 int32_t dash_sdk_dpns_is_valid_username(const char *name) ;

// Check if a username is contested (requires masternode voting)
//
// A username is contested if its normalized label:
// - Is between 3 and 19 characters long (inclusive)
// - Contains only lowercase letters a-z, digits 0-1, and hyphens
//
// # Safety
// - `name` must be a valid null-terminated C string
//
// # Returns
// - 1 if the username is contested
// - 0 if the username is not contested
// - -1 if there's an error
 int32_t dash_sdk_dpns_is_contested_username(const char *name) ;

// Get a validation message for a username
//
// Returns a descriptive message about why a username is invalid, or "valid" if it's valid.
//
// # Safety
// - `name` must be a valid null-terminated C string
 struct DashSDKResult dash_sdk_dpns_get_validation_message(const char *name) ;

// Check if a DPNS username is available
//
// This function checks if a given username is available for registration.
// It also validates the username format and checks if it's contested.
//
// # Arguments
// * `sdk_handle` - Handle to the SDK instance
// * `label` - The username label to check (e.g., "alice")
//
// # Returns
// * On success: A JSON object with availability information
// * On error: An error result
//
// # Safety
// - `sdk_handle` and `label` must be valid, non-null pointers.
// - `label` must point to a NUL-terminated C string valid for the duration of the call.
// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_dpns_check_availability(const struct dash_sdk_handle_t *sdk_handle, const char *label) ;

// Get all contested DPNS usernames where an identity is a contender
//
// # Safety
// - `sdk_handle` and `identity_id` must be valid, non-null pointers.
// - `identity_id` must point to a NUL-terminated C string valid for the duration of the call.
// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_dpns_get_contested_usernames_by_identity(const struct dash_sdk_handle_t *sdk_handle, const char *identity_id, uint32_t limit) ;

// Get the vote state for a contested DPNS username
//
// # Safety
// This function is unsafe because it operates on raw pointers
 struct DashSDKResult dash_sdk_dpns_get_contested_vote_state(const struct dash_sdk_handle_t *sdk_handle, const char *label, uint32_t limit) ;

// Get all contested DPNS usernames
//
// # Safety
// This function is unsafe because it operates on raw pointers
 struct DashSDKResult dash_sdk_dpns_get_all_contested_usernames(const struct dash_sdk_handle_t *sdk_handle, uint32_t limit, const char *start_after) ;

// Get current DPNS contests (active vote polls)
//
// Returns a list of contested DPNS names with their end times.
// The caller is responsible for freeing the result with `dash_sdk_name_timestamp_list_free`.
//
// # Safety
// This function is unsafe because it operates on raw pointers
 struct DashSDKNameTimestampList *dash_sdk_dpns_get_current_contests(const struct dash_sdk_handle_t *sdk_handle, uint64_t start_time, uint64_t end_time, uint16_t limit) ;

// Get all contested DPNS usernames that an identity has voted on
//
// # Safety
// This function is unsafe because it operates on raw pointers
 struct DashSDKResult dash_sdk_dpns_get_identity_votes(const struct dash_sdk_handle_t *sdk_handle, const char *identity_id, uint32_t limit, uint16_t offset) ;

// Get non-resolved DPNS contests for a specific identity
//
// Returns a list of contested but unresolved DPNS usernames where the identity is a contender.
// The caller is responsible for freeing the result with `dash_sdk_contested_names_list_free`.
//
// # Safety
// This function is unsafe because it operates on raw pointers
 struct DashSDKContestedNamesList *dash_sdk_dpns_get_non_resolved_contests_for_identity(const struct dash_sdk_handle_t *sdk_handle, const char *identity_id, uint32_t limit) ;

// Get contested DPNS usernames that are not yet resolved
//
// Returns a list of contested but unresolved DPNS usernames with their contest information.
// The caller is responsible for freeing the result with `dash_sdk_contested_names_list_free`.
//
// # Safety
// This function is unsafe because it operates on raw pointers
 struct DashSDKContestedNamesList *dash_sdk_dpns_get_contested_non_resolved_usernames(const struct dash_sdk_handle_t *sdk_handle, uint32_t limit) ;

// Resolve a DPNS name to an identity ID
//
// This function resolves a DPNS username to its associated identity ID.
// The name can be either:
// - A full domain name (e.g., "alice.dash")
// - Just the label (e.g., "alice")
//
// # Arguments
// * `sdk_handle` - Handle to the SDK instance
// * `name` - The DPNS name to resolve
//
// # Returns
// * On success: A JSON object with the identity ID, or null if not found
// * On error: An error result
//
// # Safety
// - `sdk_handle` and `name` must be valid, non-null pointers.
// - `name` must point to a NUL-terminated C string valid for the duration of the call.
// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_dpns_resolve(const struct dash_sdk_handle_t *sdk_handle, const char *name) ;

// Search for DPNS names that start with a given prefix
//
// This function searches for DPNS usernames that start with the given prefix.
//
// # Arguments
// * `sdk_handle` - Handle to the SDK instance
// * `prefix` - The prefix to search for (e.g., "ali" to find "alice", "alicia", etc.)
// * `limit` - Maximum number of results to return (0 for default of 10)
//
// # Returns
// * On success: A JSON array of username objects
// * On error: An error result
//
// # Safety
// - `sdk_handle` and `prefix` must be valid, non-null pointers.
// - `prefix` must point to a NUL-terminated C string valid for the duration of the call.
// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_dpns_search(const struct dash_sdk_handle_t *sdk_handle, const char *prefix, uint32_t limit) ;

// Get DPNS usernames owned by an identity
//
// This function returns all DPNS usernames associated with a given identity ID.
// It checks for domains where the identity is:
// - The owner of the domain document
// - Listed in records.dashUniqueIdentityId
// - Listed in records.dashAliasIdentityId
//
// # Arguments
// * `sdk_handle` - Handle to the SDK instance
// * `identity_id` - The identity ID to search for (base58 string)
// * `limit` - Maximum number of results to return (0 for default of 10)
//
// # Returns
// * On success: A JSON array of username objects
// * On error: An error result
//
// # Safety
// - `sdk_handle` and `identity_id` must be valid, non-null pointers.
// - `identity_id` must point to a NUL-terminated C string valid for the duration of the call.
// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_dpns_get_usernames(const struct dash_sdk_handle_t *sdk_handle, const char *identity_id, uint32_t limit) ;

// Register a DPNS username in a single operation
//
// This method handles both the preorder and domain registration steps automatically.
// It generates the necessary entropy, creates both documents, and submits them in order.
//
// # Safety
// - `handle` must be a valid, non-null SDK handle pointer.
// - `label` must be a valid pointer to a NUL-terminated C string that remains valid for the duration of the call.
// - `identity`, `identity_public_key`, and `signer` must be valid handles (as raw pointers) obtained from this SDK and not previously freed; they are not consumed by this call.
//
// # Returns
// Returns a DpnsRegistrationResult containing both created documents and the full domain name
 struct DashSDKResult dash_sdk_dpns_register_name(const struct dash_sdk_handle_t *handle, const char *label, const void *identity, const void *identity_public_key, const void *signer) ;

// Free a DPNS registration result
//
// # Safety
// - `result` must be a valid DpnsRegistrationResult pointer created by dash_sdk_dpns_register_name
 void dash_sdk_dpns_registration_result_free(struct DpnsRegistrationResult *result) ;

// Free an error message
//
// # Safety
// - `error` must be a pointer previously returned by this SDK or null (no-op).
// - After this call, `error` becomes invalid and must not be used again.
 void dash_sdk_error_free(struct DashSDKError *error) ;

// Fetches proposed epoch blocks by evonode IDs
//
// # Parameters
// * `sdk_handle` - Handle to the SDK instance
// * `epoch` - Epoch number (optional, 0 for current epoch)
// * `ids_json` - JSON array of hex-encoded evonode pro_tx_hash IDs
//
// # Returns
// * JSON array of evonode proposed block counts or null if not found
// * Error message if operation fails
//
// # Safety
// - `sdk_handle` and `ids_json` must be valid pointers; `ids_json` must point to a NUL-terminated C string with a JSON array of hex IDs.
// - Pointers must remain valid for the duration of the call.
// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_evonode_get_proposed_epoch_blocks_by_ids(const struct dash_sdk_handle_t *sdk_handle, uint32_t epoch, const char *ids_json) ;

// Fetches proposed epoch blocks by range
//
// # Parameters
// * `sdk_handle` - Handle to the SDK instance
// * `epoch` - Epoch number (optional, 0 for current epoch)
// * `limit` - Maximum number of results to return (optional, 0 for no limit)
// * `start_after` - Start after this pro_tx_hash (hex-encoded, optional)
// * `start_at` - Start at this pro_tx_hash (hex-encoded, optional)
//
// # Returns
// * JSON array of evonode proposed block counts or null if not found
// * Error message if operation fails
//
// # Safety
// - `sdk_handle` must be a valid, non-null pointer.
// - `start_after` and `start_at` may be null; when non-null they must point to NUL-terminated C strings with hex-encoded 32-byte hashes.
// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_evonode_get_proposed_epoch_blocks_by_range(const struct dash_sdk_handle_t *sdk_handle, uint32_t epoch, uint32_t limit, const char *start_after, const char *start_at) ;

// Fetches group action signers
//
// # Parameters
// * `sdk_handle` - Handle to the SDK instance
// * `contract_id` - Base58-encoded contract identifier
// * `group_contract_position` - Position of the group in the contract
// * `status` - Action status (0=Pending, 1=Completed, 2=Expired)
// * `action_id` - Base58-encoded action identifier
//
// # Returns
// * JSON array of signers or null if not found
// * Error message if operation fails
//
// # Safety
// - `sdk_handle`, `contract_id`, and `action_id` must be valid, non-null pointers.
// - `contract_id` and `action_id` must point to NUL-terminated C strings valid for the duration of the call.
// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_group_get_action_signers(const struct dash_sdk_handle_t *sdk_handle, const char *contract_id, uint16_t group_contract_position, uint8_t status, const char *action_id) ;

// Fetches group actions
//
// # Parameters
// * `sdk_handle` - Handle to the SDK instance
// * `contract_id` - Base58-encoded contract identifier
// * `group_contract_position` - Position of the group in the contract
// * `status` - Action status (0=Pending, 1=Completed, 2=Expired)
// * `start_at_action_id` - Optional starting action ID (Base58-encoded)
// * `limit` - Maximum number of actions to return
//
// # Returns
// * JSON array of group actions or null if not found
// * Error message if operation fails
//
// # Safety
// - `sdk_handle` and `contract_id` must be valid, non-null pointers.
// - `contract_id` must point to a NUL-terminated C string; `start_at_action_id` may be null, otherwise must be a valid NUL-terminated C string.
// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_group_get_actions(const struct dash_sdk_handle_t *sdk_handle, const char *contract_id, uint16_t group_contract_position, uint8_t status, const char *start_at_action_id, uint16_t limit) ;

// Fetches information about a group
//
// # Parameters
// * `sdk_handle` - Handle to the SDK instance
// * `contract_id` - Base58-encoded contract identifier
// * `group_contract_position` - Position of the group in the contract
//
// # Returns
// * JSON string with group information or null if not found
// * Error message if operation fails
//
// # Safety
// - `sdk_handle` and `contract_id` must be valid, non-null pointers.
// - `contract_id` must point to a NUL-terminated C string valid for the duration of the call.
// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_group_get_info(const struct dash_sdk_handle_t *sdk_handle, const char *contract_id, uint16_t group_contract_position) ;

// Fetches information about multiple groups
//
// # Parameters
// * `sdk_handle` - Handle to the SDK instance
// * `start_at_position` - Starting position (optional, null for beginning)
// * `limit` - Maximum number of groups to return
//
// # Returns
// * JSON array of group information or null if not found
// * Error message if operation fails
//
// # Safety
// - `sdk_handle` must be a valid, non-null pointer.
// - `start_at_position` may be null; when non-null it must point to a NUL-terminated C string representing a number.
// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_group_get_infos(const struct dash_sdk_handle_t *sdk_handle, const char *start_at_position, uint32_t limit) ;

// Create a new identity
//
// # Safety
// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
// - On success, the returned `DashSDKResult` contains a heap-allocated handle that must be freed using the
//   appropriate SDK destroy function.
// - Passing a dangling or invalid pointer results in undefined behavior.
 struct DashSDKResult dash_sdk_identity_create(struct dash_sdk_handle_t *sdk_handle) ;

// Create an identity handle from components
//
// This function creates an identity handle from basic components without
// requiring JSON serialization/deserialization.
//
// # Parameters
// - `identity_id`: 32-byte identity ID
// - `public_keys`: Array of public key data
// - `public_keys_count`: Number of public keys in the array
// - `balance`: Identity balance in credits
// - `revision`: Identity revision number
//
// # Returns
// - Handle to the created identity on success
// - Error if creation fails
//
// # Safety
// - `identity_id` must be a valid pointer to 32 readable bytes.
// - If `public_keys_count > 0`, `public_keys` must be a valid pointer to an array of `DashSDKPublicKeyData`
//   structures of length `public_keys_count`; each `data` field in the array must point to at least `data_len` readable bytes.
// - All pointers must remain valid for the duration of the call.
// - On success, returns a heap-allocated handle; caller must destroy it using the SDK's destroy function.
 struct DashSDKResult dash_sdk_identity_create_from_components(const uint8_t *identity_id, const struct DashSDKPublicKeyData *public_keys, uintptr_t public_keys_count, uint64_t balance, uint64_t revision) ;

// Get a public key from an identity by its ID
//
// # Parameters
// - `identity`: Handle to the identity
// - `key_id`: The ID of the public key to retrieve
//
// # Returns
// - Handle to the public key on success
// - Error if key not found or invalid parameters
//
// # Safety
// - `identity` must be a valid, non-null pointer to an `IdentityHandle` that remains valid for the duration of the call.
// - On success, the returned `DashSDKResult` contains a heap-allocated handle which must be destroyed with the SDK's
//   corresponding destroy function.
// - Passing invalid or dangling pointers results in undefined behavior.
 struct DashSDKResult dash_sdk_identity_get_public_key_by_id(const struct IdentityHandle *identity, uint8_t key_id) ;

// Get identity information
//
// # Safety
// - `identity_handle` must be a valid, non-null pointer to an `IdentityHandle` that remains valid for the duration of the call.
// - Returns a heap-allocated `DashSDKIdentityInfo` pointer; caller must free it using the SDK-provided destroy function.
// - Passing invalid or dangling pointers results in undefined behavior.
 struct DashSDKIdentityInfo *dash_sdk_identity_get_info(const struct IdentityHandle *identity_handle) ;

// Destroy an identity handle
//
// # Safety
// - `handle` must be a pointer previously returned by this SDK or null (no-op).
// - After this call, `handle` becomes invalid and must not be used again.
 void dash_sdk_identity_destroy(struct IdentityHandle *handle) ;

// Get the appropriate signing key for a state transition
//
// This function finds a key that meets the purpose and security level requirements
// for the specified state transition type.
//
// # Parameters
// - `identity_handle`: Handle to the identity
// - `transition_type`: Type of state transition to be signed
//
// # Returns
// - Handle to the identity public key on success
// - Error if no suitable key is found
//
// # Safety
// - `identity_handle` must be a valid, non-null pointer to an `IdentityHandle` that remains valid for the duration of the call.
// - On success, the returned `DashSDKResult` contains a heap-allocated handle which must be destroyed with the SDK's
//   corresponding destroy function.
// - Passing invalid or dangling pointers results in undefined behavior.
 struct DashSDKResult dash_sdk_identity_get_signing_key_for_transition(const struct IdentityHandle *identity_handle, enum StateTransitionType transition_type) ;

// Get the private key data for a transfer key
//
// This function retrieves the private key data that corresponds to the
// lowest security level transfer key. In a real implementation, this would
// interface with a secure key storage system.
//
// # Parameters
// - `identity_handle`: Handle to the identity
// - `key_index`: The key index from the identity public key
//
// # Returns
// - 32-byte private key data on success
// - Error if key not found or not accessible
//
// # Safety
// - `identity_handle` must be a valid, non-null pointer to an `IdentityHandle`.
// - This function returns its result inside `DashSDKResult`; any heap pointers within must be freed using SDK routines.
// - Passing invalid or dangling pointers results in undefined behavior.
 struct DashSDKResult dash_sdk_identity_get_transfer_private_key(const struct IdentityHandle *identity_handle, uint32_t key_index) ;

// Get the key ID from an identity public key
//
// # Safety
// - `key_handle` must be a valid, non-null pointer to an `IdentityPublicKeyHandle`.
// - Returns 0 if the pointer is null.
 uint32_t dash_sdk_identity_public_key_get_id(const struct IdentityPublicKeyHandle *key_handle) ;

// Create an identity public key handle from key data
//
// This function creates an identity public key handle from the raw key data
// without needing to fetch the identity from the network.
//
// # Parameters
// - `key_id`: The key ID
// - `key_type`: The key type (0 = ECDSA_SECP256K1, 1 = BLS12_381, 2 = ECDSA_HASH160, 3 = BIP13_SCRIPT_HASH, 4 = ED25519_HASH160)
// - `purpose`: The key purpose (0 = Authentication, 1 = Encryption, 2 = Decryption, 3 = Transfer, 4 = SystemTransfer, 5 = Voting)
// - `security_level`: The security level (0 = Master, 1 = Critical, 2 = High, 3 = Medium)
// - `public_key_data`: The public key data
// - `public_key_data_len`: Length of the public key data
// - `read_only`: Whether the key is read-only
// - `disabled_at`: Optional timestamp when the key was disabled (0 if not disabled)
//
// # Returns
// - Handle to the identity public key on success
// - Error if parameters are invalid
//
// # Safety
// - `public_key_data` must be a valid, non-null pointer to a buffer of `public_key_data_len` readable bytes.
// - All scalar parameters are passed by value.
// - On success, returns a heap-allocated handle which must be destroyed with the SDK's destroy function.
// - Passing invalid or dangling pointers results in undefined behavior.
 struct DashSDKResult dash_sdk_identity_public_key_create_from_data(uint32_t key_id, uint8_t key_type, uint8_t purpose, uint8_t security_level, const uint8_t *public_key_data, uintptr_t public_key_data_len, bool read_only, uint64_t disabled_at) ;

// Serialize an identity public key to bytes
// Returns the serialized bytes and their length
//
// # Safety
// - `key_handle` must be a valid, non-null pointer to an `IdentityPublicKeyHandle`.
// - `out_bytes` and `out_len` must be valid, non-null pointers to writable memory.
// - Caller must free the returned buffer with the appropriate SDK-provided free function.
 struct DashSDKResult dash_sdk_identity_public_key_to_bytes(const struct IdentityPublicKeyHandle *key_handle, uint8_t **out_bytes, uintptr_t *out_len) ;

// Free an identity public key handle
//
// # Safety
// - `handle` must be a pointer previously returned by this SDK or null (no-op).
// - After this call, `handle` becomes invalid and must not be used again.
 void dash_sdk_identity_public_key_destroy(struct IdentityPublicKeyHandle *handle) ;

// Register a name for an identity
//
// # Safety
// - `_sdk_handle` and `_identity_handle` must be valid pointers when used; currently this stub ignores them.
// - `_name` must be a valid pointer to a NUL-terminated C string if used in the future.
// - Returns a heap-allocated error pointer; caller must free it using `dash_sdk_error_free`.
 struct DashSDKError *dash_sdk_identity_register_name(struct dash_sdk_handle_t *sdk_handle, const struct IdentityHandle *identity_handle, const char *name) ;

// Parse an identity from JSON string to handle
//
// This function takes a JSON string representation of an identity
// (as returned by dash_sdk_identity_fetch) and converts it to an
// identity handle that can be used with other FFI functions.
//
// # Parameters
// - `json_str`: JSON string containing the identity data
//
// # Returns
// - Handle to the parsed identity on success
// - Error if JSON parsing fails
//
// # Safety
// - `json_str` must be a valid, non-null pointer to a NUL-terminated C string and remain valid for the duration of the call.
// - On success, the returned `DashSDKResult` contains a heap-allocated handle which must be freed using the
//   appropriate SDK destroy function to avoid leaks.
 struct DashSDKResult dash_sdk_identity_parse_json(const char *json_str) ;

// Put identity to platform with instant lock proof
//
// # Safety
// - `sdk_handle`, `identity_handle`, `instant_lock_bytes`, `transaction_bytes`, `private_key`, and `signer_handle`
//   must be valid, non-null pointers. Buffer pointers must reference at least the specified lengths.
// - `put_settings` may be null; if non-null it must be valid for the duration of the call.
// - On success, returns serialized data; any heap memory inside the result must be freed using SDK routines.
//
// # Parameters
// - `instant_lock_bytes`: Serialized InstantLock data
// - `transaction_bytes`: Serialized Transaction data
// - `output_index`: Index of the output in the transaction payload
// - `private_key`: 32-byte private key associated with the asset lock
// - `put_settings`: Optional settings for the operation (can be null for defaults)
 struct DashSDKResult dash_sdk_identity_put_to_platform_with_instant_lock(struct dash_sdk_handle_t *sdk_handle, const struct IdentityHandle *identity_handle, const uint8_t *instant_lock_bytes, uintptr_t instant_lock_len, const uint8_t *transaction_bytes, uintptr_t transaction_len, uint32_t output_index, const uint8_t (*private_key)[32], const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings) ;

// Put identity to platform with instant lock proof and wait for confirmation
//
// # Safety
// - Same requirements as `dash_sdk_identity_put_to_platform_with_instant_lock`.
// - The function may block while waiting for confirmation; input pointers must remain valid throughout.
// - On success, returns a heap-allocated handle which must be destroyed with the SDK's destroy function.
//
// # Parameters
// - `instant_lock_bytes`: Serialized InstantLock data
// - `transaction_bytes`: Serialized Transaction data
// - `output_index`: Index of the output in the transaction payload
// - `private_key`: 32-byte private key associated with the asset lock
// - `put_settings`: Optional settings for the operation (can be null for defaults)
//
// # Returns
// Handle to the confirmed identity on success
 struct DashSDKResult dash_sdk_identity_put_to_platform_with_instant_lock_and_wait(struct dash_sdk_handle_t *sdk_handle, const struct IdentityHandle *identity_handle, const uint8_t *instant_lock_bytes, uintptr_t instant_lock_len, const uint8_t *transaction_bytes, uintptr_t transaction_len, uint32_t output_index, const uint8_t (*private_key)[32], const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings) ;

// Put identity to platform with chain lock proof
//
// # Safety
// - `sdk_handle`, `identity_handle`, `out_point`, `private_key`, and `signer_handle` must be valid, non-null pointers.
// - `out_point` must reference 36 readable bytes; `private_key` must reference 32 readable bytes.
// - `put_settings` may be null; if non-null it must be valid for the duration of the call.
// - On success, returns serialized data; any heap memory inside the result must be freed using SDK routines.
//
// # Parameters
// - `core_chain_locked_height`: Core height at which the transaction was chain locked
// - `out_point`: 36-byte OutPoint (32-byte txid + 4-byte vout)
// - `private_key`: 32-byte private key associated with the asset lock
// - `put_settings`: Optional settings for the operation (can be null for defaults)
 struct DashSDKResult dash_sdk_identity_put_to_platform_with_chain_lock(struct dash_sdk_handle_t *sdk_handle, const struct IdentityHandle *identity_handle, uint32_t core_chain_locked_height, const uint8_t (*out_point)[36], const uint8_t (*private_key)[32], const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings) ;

// Put identity to platform with chain lock proof and wait for confirmation
//
// # Safety
// - Same requirements as `dash_sdk_identity_put_to_platform_with_chain_lock`.
// - The function may block while waiting for confirmation; input pointers must remain valid throughout.
// - On success, returns a heap-allocated handle which must be destroyed with the SDK's destroy function.
//
// # Parameters
// - `core_chain_locked_height`: Core height at which the transaction was chain locked
// - `out_point`: 36-byte OutPoint (32-byte txid + 4-byte vout)
// - `private_key`: 32-byte private key associated with the asset lock
// - `put_settings`: Optional settings for the operation (can be null for defaults)
//
// # Returns
// Handle to the confirmed identity on success
 struct DashSDKResult dash_sdk_identity_put_to_platform_with_chain_lock_and_wait(struct dash_sdk_handle_t *sdk_handle, const struct IdentityHandle *identity_handle, uint32_t core_chain_locked_height, const uint8_t (*out_point)[36], const uint8_t (*private_key)[32], const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings) ;

// Fetch identity balance
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `identity_id`: Base58-encoded identity ID
//
// # Returns
// The balance of the identity as a string
//
// # Safety
// - `sdk_handle` and `identity_id` must be valid, non-null pointers.
// - `identity_id` must point to a NUL-terminated C string valid for the duration of the call.
// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_identity_fetch_balance(const struct dash_sdk_handle_t *sdk_handle, const char *identity_id) ;

// Fetch identity balance and revision
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `identity_id`: Base58-encoded identity ID
//
// # Returns
// JSON string containing the balance and revision information
//
// # Safety
// - `sdk_handle` and `identity_id` must be valid, non-null pointers.
// - `identity_id` must point to a NUL-terminated C string valid for the duration of the call.
// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_identity_fetch_balance_and_revision(const struct dash_sdk_handle_t *sdk_handle, const char *identity_id) ;

// Fetch identity by non-unique public key hash with optional pagination
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `public_key_hash`: Hex-encoded 20-byte public key hash
// - `start_after`: Optional Base58-encoded identity ID to start after (for pagination)
//
// # Returns
// JSON string containing the identity information, or null if not found
//
// # Safety
// - `sdk_handle` and `public_key_hash` must be valid, non-null pointers.
// - `public_key_hash` must point to a NUL-terminated C string. `start_after` may be null; if non-null it must be a valid
//   pointer to a NUL-terminated C string.
// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_identity_fetch_by_non_unique_public_key_hash(const struct dash_sdk_handle_t *sdk_handle, const char *public_key_hash, const char *start_after) ;

// Fetch identity by public key hash
//
// # Safety
// - `sdk_handle` and `public_key_hash` must be valid, non-null pointers.
// - `public_key_hash` must point to a NUL-terminated C string valid for the duration of the call.
// - On success, returns a handle or no data; any heap memory must be freed using SDK routines.
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `public_key_hash`: Hex-encoded 20-byte public key hash
//
// # Returns
// JSON string containing the identity information, or null if not found
 struct DashSDKResult dash_sdk_identity_fetch_by_public_key_hash(const struct dash_sdk_handle_t *sdk_handle, const char *public_key_hash) ;

// Fetch identity contract nonce
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `identity_id`: Base58-encoded identity ID
// - `contract_id`: Base58-encoded contract ID
//
// # Returns
// The contract nonce of the identity as a string
//
// # Safety
// - `sdk_handle`, `identity_id`, and `contract_id` must be valid, non-null pointers.
// - `identity_id` and `contract_id` must point to NUL-terminated C strings valid for the duration of the call.
// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_identity_fetch_contract_nonce(const struct dash_sdk_handle_t *sdk_handle, const char *identity_id, const char *contract_id) ;

// Fetch an identity by ID
//
// # Safety
// - `sdk_handle` and `identity_id` must be valid, non-null pointers.
// - `identity_id` must point to a NUL-terminated C string.
// - On success, returns a handle or no data; any heap memory must be freed using SDK routines.
 struct DashSDKResult dash_sdk_identity_fetch(const struct dash_sdk_handle_t *sdk_handle, const char *identity_id) ;

// Fetch an identity by ID and return a handle
//
// This function fetches an identity from the network and returns
// a handle that can be used with other FFI functions like transfers.
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `identity_id`: Base58-encoded identity ID
//
// # Returns
// - Handle to the fetched identity on success
// - Error if fetch fails or identity not found
//
// # Safety
// - `sdk_handle` and `identity_id` must be valid, non-null pointers.
// - `identity_id` must point to a NUL-terminated C string valid for the duration of the call.
// - On success, returns a heap-allocated handle which must be destroyed with the SDK's destroy function.
 struct DashSDKResult dash_sdk_identity_fetch_handle(const struct dash_sdk_handle_t *sdk_handle, const char *identity_id) ;

// Fetch balances for multiple identities
//
// # Safety
// - `sdk_handle` and `identity_ids` must be valid, non-null pointers.
// - `identity_ids` must point to an array of `[u8; 32]` of length `identity_ids_len` and remain valid for the duration of the call.
// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `identity_ids`: Array of identity IDs (32-byte arrays)
// - `identity_ids_len`: Number of identity IDs in the array
//
// # Returns
// DashSDKResult with data_type = IdentityBalanceMap containing identity IDs mapped to their balances
 struct DashSDKResult dash_sdk_identities_fetch_balances(const struct dash_sdk_handle_t *sdk_handle, const uint8_t (*identity_ids)[32], uintptr_t identity_ids_len) ;

// Fetch contract keys for multiple identities
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `identity_ids`: Comma-separated list of Base58-encoded identity IDs
// - `contract_id`: Base58-encoded contract ID
// - `document_type_name`: Optional document type name (pass NULL if not needed)
// - `purposes`: Comma-separated list of key purposes (0=Authentication, 1=Encryption, 2=Decryption, 3=Withdraw)
//
// # Returns
// JSON string containing identity IDs mapped to their contract keys by purpose
//
// # Safety
// - `sdk_handle`, `identity_ids`, `contract_id`, and `purposes` must be valid, non-null pointers.
// - `identity_ids`, `contract_id`, `document_type_name` (when non-null), and `purposes` must point to NUL-terminated C strings valid for the duration of the call.
// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_identities_fetch_contract_keys(const struct dash_sdk_handle_t *sdk_handle, const char *identity_ids, const char *contract_id, const char *document_type_name, const char *purposes) ;

// Fetch identity nonce
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `identity_id`: Base58-encoded identity ID
//
// # Returns
// The nonce of the identity as a string
//
// # Safety
// - `sdk_handle` and `identity_id` must be valid, non-null pointers.
// - `identity_id` must point to a NUL-terminated C string valid for the duration of the call.
// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_identity_fetch_nonce(const struct dash_sdk_handle_t *sdk_handle, const char *identity_id) ;

// Fetch identity public keys
//
// # Safety
// - `sdk_handle` and `identity_id` must be valid, non-null pointers.
// - `identity_id` must point to a NUL-terminated C string valid for the duration of the call.
// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `identity_id`: Base58-encoded identity ID
//
// # Returns
// A JSON string containing the identity's public keys
 struct DashSDKResult dash_sdk_identity_fetch_public_keys(const struct dash_sdk_handle_t *sdk_handle, const char *identity_id) ;

// Resolve a name to an identity
//
// This function takes a name in the format "label.parentdomain" (e.g., "alice.dash")
// or just "label" for top-level domains, and returns the associated identity ID.
//
// # Arguments
// * `sdk_handle` - Handle to the SDK instance
// * `name` - C string containing the name to resolve
//
// # Returns
// * On success: A result containing the resolved identity ID
// * On error: An error result
//
// # Safety
// - `sdk_handle` and `name` must be valid, non-null pointers.
// - `name` must point to a NUL-terminated C string valid for the duration of the call.
// - On success, any heap memory in the result must be freed using SDK routines.
 struct DashSDKResult dash_sdk_identity_resolve_name(const struct dash_sdk_handle_t *sdk_handle, const char *name) ;

// Test function to diagnose the transfer crash
//
// # Safety
// - `sdk_handle` and `identity_id` must be valid, non-null pointers.
// - `identity_id` must point to a NUL-terminated C string valid for the duration of the call.
// - On success, any heap memory in the result must be freed using SDK routines.
 struct DashSDKResult dash_sdk_test_identity_transfer_crash(const struct dash_sdk_handle_t *sdk_handle, const char *identity_id) ;

// Top up an identity with credits using instant lock proof
//
// # Safety
// - `sdk_handle`, `identity_handle`, `instant_lock_bytes`, `transaction_bytes`, and `private_key` must be valid, non-null pointers.
// - Buffer pointers must reference at least the specified lengths.
// - `put_settings` may be null; if non-null it must be valid for the duration of the call.
// - On success, returns serialized data; any heap memory inside the result must be freed using SDK routines.
 struct DashSDKResult dash_sdk_identity_topup_with_instant_lock(struct dash_sdk_handle_t *sdk_handle, const struct IdentityHandle *identity_handle, const uint8_t *instant_lock_bytes, uintptr_t instant_lock_len, const uint8_t *transaction_bytes, uintptr_t transaction_len, uint32_t output_index, const uint8_t (*private_key)[32], const struct DashSDKPutSettings *put_settings) ;

// Top up an identity with credits using instant lock proof and wait for confirmation
//
// # Safety
// - Same requirements as `dash_sdk_identity_topup_with_instant_lock`.
// - The function may block while waiting for confirmation; input pointers must remain valid throughout.
// - On success, returns a heap-allocated handle which must be destroyed with the SDK's destroy function.
 struct DashSDKResult dash_sdk_identity_topup_with_instant_lock_and_wait(struct dash_sdk_handle_t *sdk_handle, const struct IdentityHandle *identity_handle, const uint8_t *instant_lock_bytes, uintptr_t instant_lock_len, const uint8_t *transaction_bytes, uintptr_t transaction_len, uint32_t output_index, const uint8_t (*private_key)[32], const struct DashSDKPutSettings *put_settings) ;

// Transfer credits from one identity to another
//
// # Parameters
// - `from_identity_handle`: Identity to transfer credits from
// - `to_identity_id`: Base58-encoded ID of the identity to transfer credits to
// - `amount`: Amount of credits to transfer
// - `public_key_id`: ID of the public key to use for signing (pass 0 to auto-select TRANSFER key)
// - `signer_handle`: Cryptographic signer
// - `put_settings`: Optional settings for the operation (can be null for defaults)
//
// # Returns
// DashSDKTransferCreditsResult with sender and receiver final balances on success
//
// # Safety
// - `sdk_handle`, `from_identity_handle`, `to_identity_id`, and `signer_handle` must be valid, non-null pointers.
// - `to_identity_id` must point to a NUL-terminated C string valid for the duration of the call.
// - `put_settings` may be null; if non-null it must be valid for the duration of the call.
// - On success, any heap memory included in the result must be freed using SDK routines.
 struct DashSDKResult dash_sdk_identity_transfer_credits(struct dash_sdk_handle_t *sdk_handle, const struct IdentityHandle *from_identity_handle, const char *to_identity_id, uint64_t amount, uint32_t public_key_id, const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings) ;

// Free a transfer credits result structure
//
// # Safety
// - `result` must be a pointer previously returned by this SDK or null (no-op).
// - After this call, `result` becomes invalid and must not be used again.
 void dash_sdk_transfer_credits_result_free(struct DashSDKTransferCreditsResult *result) ;

// Withdraw credits from identity to a Dash address
//
// # Parameters
// - `identity_handle`: Identity to withdraw credits from
// - `address`: Base58-encoded Dash address to withdraw to
// - `amount`: Amount of credits to withdraw
// - `core_fee_per_byte`: Core fee per byte (optional, pass 0 for default)
// - `public_key_id`: ID of the public key to use for signing (pass 0 to auto-select TRANSFER key)
// - `signer_handle`: Cryptographic signer
// - `put_settings`: Optional settings for the operation (can be null for defaults)
//
// # Returns
// The new balance of the identity after withdrawal
//
// # Safety
// - `sdk_handle`, `identity_handle`, `address`, and `signer_handle` must be valid, non-null pointers.
// - `address` must point to a NUL-terminated C string valid for the duration of the call.
// - `put_settings` may be null; if non-null it must be valid for the duration of the call.
// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_identity_withdraw(struct dash_sdk_handle_t *sdk_handle, const struct IdentityHandle *identity_handle, const char *address, uint64_t amount, uint32_t core_fee_per_byte, uint32_t public_key_id, const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings) ;

// Fetches protocol version upgrade state
//
// # Parameters
// * `sdk_handle` - Handle to the SDK instance
//
// # Returns
// * JSON array of protocol version upgrade information
// * Error message if operation fails
//
// # Safety
// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
// - The function does not retain references to the input pointer beyond the duration of the call.
// - On success, the returned `DashSDKResult` may contain a heap-allocated C string; the caller must
//   free it using the SDK's free routine to avoid leaks. It may also return no data (null pointer).
// - Passing a dangling or invalid pointer for `sdk_handle` results in undefined behavior.
 struct DashSDKResult dash_sdk_protocol_version_get_upgrade_state(const struct dash_sdk_handle_t *sdk_handle) ;

// Fetches protocol version upgrade vote status
//
// # Parameters
// * `sdk_handle` - Handle to the SDK instance
// * `start_pro_tx_hash` - Starting masternode pro_tx_hash (hex-encoded, optional)
// * `count` - Number of vote entries to retrieve
//
// # Returns
// * JSON array of masternode protocol version votes or null if not found
// * Error message if operation fails
//
// # Safety
// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
// - `start_pro_tx_hash` may be null (meaning no start); when non-null it must be a valid pointer to a
//   NUL-terminated C string containing a hex-encoded 32-byte hash and remain valid for the duration of the call.
// - `count` is passed by value; no references are retained.
// - On success, the returned `DashSDKResult` may contain a heap-allocated C string which the caller must free
//   using the SDK's free routine. It may also contain no data (null pointer).
// - All pointers must reference readable memory; invalid pointers result in undefined behavior.
 struct DashSDKResult dash_sdk_protocol_version_get_upgrade_vote_status(const struct dash_sdk_handle_t *sdk_handle, const char *start_pro_tx_hash, uint32_t count) ;

// Create a new SDK instance
//
// # Safety
// - `config` must be a valid pointer to a DashSDKConfig structure for the duration of the call.
// - The returned handle inside `DashSDKResult` must be destroyed using the SDK destroy function to avoid leaks.
 struct DashSDKResult dash_sdk_create(const struct DashSDKConfig *config) ;

// Create a new SDK instance with extended configuration including context provider
//
// # Safety
// - `config` must be a valid pointer to a DashSDKConfigExtended structure for the duration of the call.
// - Any embedded pointers (context_provider/core_sdk_handle) must be valid when non-null.
// - The returned handle inside `DashSDKResult` must be destroyed using the SDK destroy function to avoid leaks.
 struct DashSDKResult dash_sdk_create_extended(const struct DashSDKConfigExtended *config) ;

// Create a new SDK instance with trusted setup
//
// This creates an SDK with a trusted context provider that fetches quorum keys and
// data contracts from trusted endpoints instead of requiring proof verification.
//
// # Safety
// - `config` must be a valid pointer to a DashSDKConfig structure
// # Safety
// - `config` must be a valid pointer to a DashSDKConfig structure for the duration of the call.
// - The returned handle inside `DashSDKResult` must be destroyed using the SDK destroy function to avoid leaks.
 struct DashSDKResult dash_sdk_create_trusted(const struct DashSDKConfig *config) ;

// Destroy an SDK instance
// # Safety
// - `handle` must be a valid pointer previously returned by this SDK and not yet destroyed.
// - It may be null (no-op). After this call the handle must not be used again.
 void dash_sdk_destroy(struct dash_sdk_handle_t *handle) ;

// Register global context provider callbacks
//
// This must be called before creating an SDK instance that needs Core SDK functionality.
// The callbacks will be used by all SDK instances created after registration.
//
// # Safety
// - `callbacks` must contain valid function pointers that remain valid for the lifetime of the SDK
 int32_t dash_sdk_register_context_callbacks(const struct ContextProviderCallbacks *callbacks) ;

// Create a new SDK instance with explicit context callbacks
//
// This is an alternative to registering global callbacks. The callbacks are used only for this SDK instance.
//
// # Safety
// - `config` must be a valid pointer to a DashSDKConfig structure
// - `callbacks` must contain valid function pointers that remain valid for the lifetime of the SDK
 struct DashSDKResult dash_sdk_create_with_callbacks(const struct DashSDKConfig *config, const struct ContextProviderCallbacks *callbacks) ;

// Get the current network the SDK is connected to
//
// # Safety
// - `handle` must be a valid pointer to an SDKHandle (or null, in which case a default is returned).
 enum DashSDKNetwork dash_sdk_get_network(const struct dash_sdk_handle_t *handle) ;

// Add known contracts to the SDK's trusted context provider
//
// This allows pre-loading data contracts into the trusted provider's cache,
// avoiding network calls for these contracts.
//
// # Safety
// - `handle` must be a valid SDK handle created with dash_sdk_create_trusted
// - `contract_ids` must be a valid comma-separated list of contract IDs
// - `serialized_contracts` must be a valid pointer to an array of serialized contract data
// - `contract_lengths` must be a valid pointer to an array of contract data lengths
// - `contract_count` must match the actual number of contracts provided
 struct DashSDKResult dash_sdk_add_known_contracts(const struct dash_sdk_handle_t *handle, const char *contract_ids, const uint8_t *const *serialized_contracts, const uintptr_t *contract_lengths, uintptr_t contract_count) ;

// Create a mock SDK instance with a dump directory (for offline testing)
//
// # Safety
// - `dump_dir` must be either null (no dumps) or a valid pointer to a NUL-terminated C string readable for the duration of the call.
// - The returned handle must be destroyed using the SDK destroy function to avoid leaks.
 struct dash_sdk_handle_t *dash_sdk_create_handle_with_mock(const char *dump_dir) ;

// Create a new signer with callbacks from iOS/external code
//
// This creates a VTableSigner that can be used for all state transitions.
// The callbacks should handle the actual signing logic.
//
// # Parameters
// - `sign_callback`: Function to sign data
// - `can_sign_callback`: Function to check if can sign with a key
// - `destroy_callback`: Optional destructor (can be NULL)
// # Safety
// - Callback function pointers must be valid and follow the required ABI and lifetime for the duration of use.
// - The returned `SignerHandle` must be destroyed with `dash_sdk_signer_destroy` to avoid leaks.
 struct SignerHandle *dash_sdk_signer_create(SignCallback sign_callback, CanSignCallback can_sign_callback, DestroyCallback destroy_callback) ;

// Destroy a signer
// # Safety
// - `handle` must be a valid pointer previously returned by this SDK and not yet destroyed.
// - It may be null (no-op). After this call the handle must not be used again.
 void dash_sdk_signer_destroy(struct SignerHandle *handle) ;

// Free bytes allocated by callbacks
// # Safety
// - `bytes` must be a pointer to a buffer allocated by the corresponding FFI and compatible with `libc::free`.
// - It may be null (no-op). After this call the pointer must not be used again.
 void dash_sdk_bytes_free(uint8_t *bytes) ;

// Create a signer from a private key
//
// # Safety
// - `private_key` must be a valid pointer to at least 32 readable bytes.
// - The function reads exactly `private_key_len` bytes; it must be 32.
// - The returned handle inside DashSDKResult must be freed using the appropriate SDK destroy function.
 struct DashSDKResult dash_sdk_signer_create_from_private_key(const uint8_t *private_key, uintptr_t private_key_len) ;

// Sign data with a signer
//
// # Safety
// - `signer_handle` must be a valid pointer obtained from this SDK and not previously destroyed.
// - `data` must be a valid pointer to `data_len` readable bytes.
// - The returned signature pointer inside DashSDKResult must be freed with `dash_sdk_signature_free`.
 struct DashSDKResult dash_sdk_signer_sign(struct SignerHandle *signer_handle, const uint8_t *data, uintptr_t data_len) ;

// Free a signature
//
// # Safety
// - `signature` must be a valid pointer returned by this SDK, or null for no-op.
// - After this call the pointer must not be used again.
 void dash_sdk_signature_free(struct DashSDKSignature *signature) ;

// Fetches information about current quorums
//
// # Parameters
// * `sdk_handle` - Handle to the SDK instance
//
// # Returns
// * JSON string with current quorums information
// * Error message if operation fails
//
// # Safety
// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
// - The function does not retain references to inputs beyond the call.
// - On success, returns a heap-allocated C string pointer; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_system_get_current_quorums_info(const struct dash_sdk_handle_t *sdk_handle) ;

// Fetches information about multiple epochs
//
// # Parameters
// * `sdk_handle` - Handle to the SDK instance
// * `start_epoch` - Starting epoch index (optional, null for default)
// * `count` - Number of epochs to retrieve
// * `ascending` - Whether to return epochs in ascending order
//
// # Returns
// * JSON array of epoch information or null if not found
// * Error message if operation fails
//
// # Safety
// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
// - `start_epoch` may be null (no explicit start); when non-null it must be a valid pointer to a NUL-terminated C string.
// - On success, returns a heap-allocated C string pointer; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_system_get_epochs_info(const struct dash_sdk_handle_t *sdk_handle, const char *start_epoch, uint32_t count, bool ascending) ;

// Fetches path elements
//
// # Parameters
// * `sdk_handle` - Handle to the SDK instance
// * `path_json` - JSON array of path elements (hex-encoded byte arrays)
// * `keys_json` - JSON array of keys (hex-encoded byte arrays)
//
// # Returns
// * JSON array of elements or null if not found
// * Error message if operation fails
//
// # Safety
// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
// - `path_json` and `keys_json` must be valid, non-null pointers to NUL-terminated C strings that remain valid for the duration of the call.
// - On success, returns a heap-allocated C string pointer; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_system_get_path_elements(const struct dash_sdk_handle_t *sdk_handle, const char *path_json, const char *keys_json) ;

// Get platform status including block heights
// # Safety
// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
// - The returned C string pointer (on success) must be freed by the caller using the SDK's free routine.
 struct DashSDKResult dash_sdk_get_platform_status(const struct dash_sdk_handle_t *sdk_handle) ;

// Fetches a prefunded specialized balance
//
// # Parameters
// * `sdk_handle` - Handle to the SDK instance
// * `id` - Base58-encoded identifier
//
// # Returns
// * JSON string with balance or null if not found
// * Error message if operation fails
//
// # Safety
// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
// - `id` must be a valid, non-null pointer to a NUL-terminated C string that remains valid for the duration of the call.
// - On success, returns a heap-allocated C string pointer; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_system_get_prefunded_specialized_balance(const struct dash_sdk_handle_t *sdk_handle, const char *id) ;

// Fetches the total credits in the platform
//
// # Parameters
// * `sdk_handle` - Handle to the SDK instance
//
// # Returns
// * JSON string with total credits
// * Error message if operation fails
//
// # Safety
// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
// - The function does not retain references to inputs beyond the call.
// - On success, returns a heap-allocated C string pointer; caller must free it using SDK routines.
 struct DashSDKResult dash_sdk_system_get_total_credits_in_platform(const struct dash_sdk_handle_t *sdk_handle) ;

// Get SDK status including mode and quorum count
//
// # Safety
// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
// - The returned C string pointer inside DashSDKResult (on success) must be freed by the caller
//   using the SDK's free routine to avoid memory leaks.
 struct DashSDKResult dash_sdk_get_status(const struct dash_sdk_handle_t *sdk_handle) ;

// Burn tokens from an identity and wait for confirmation
//
// # Safety
// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
// - `transition_owner_id` must point to at least 32 readable bytes.
// - `params`, `identity_public_key_handle`, `signer_handle` must be valid pointers to initialized structures.
// - Optional pointers (`put_settings`, `state_transition_creation_options`) may be null; when non-null they must be valid.
// - Caller must free any returned heap memory from the result using SDK free routines.
 struct DashSDKResult dash_sdk_token_burn(struct dash_sdk_handle_t *sdk_handle, const uint8_t *transition_owner_id, const struct DashSDKTokenBurnParams *params, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Claim tokens from a distribution and wait for confirmation
//
// # Safety
// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
// - `transition_owner_id` must point to at least 32 readable bytes.
// - `params`, `identity_public_key_handle`, and `signer_handle` must be valid pointers to initialized structures.
// - Optional pointers (`put_settings`, `state_transition_creation_options`) may be null; when non-null they must be valid.
// - Caller must free any returned heap memory in the result using SDK free routines.
 struct DashSDKResult dash_sdk_token_claim(struct dash_sdk_handle_t *sdk_handle, const uint8_t *transition_owner_id, const struct DashSDKTokenClaimParams *params, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Mint tokens to an identity and wait for confirmation
//
// # Safety
// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
// - `transition_owner_id` must point to at least 32 readable bytes.
// - `params`, `identity_public_key_handle`, and `signer_handle` must be valid pointers to initialized structures.
// - Optional pointers (`put_settings`, `state_transition_creation_options`) may be null; when non-null they must be valid.
// - Caller must free any returned heap memory in the result using SDK free routines.
 struct DashSDKResult dash_sdk_token_mint(struct dash_sdk_handle_t *sdk_handle, const uint8_t *transition_owner_id, const struct DashSDKTokenMintParams *params, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Token transfer to another identity and wait for confirmation
//
// # Safety
// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
// - `transition_owner_id` must point to at least 32 readable bytes.
// - `params`, `identity_public_key_handle`, `signer_handle` must be valid pointers to initialized structures.
// - Optional pointers (`put_settings`, `state_transition_creation_options`) may be null; when non-null they must be valid.
// - Caller must free any returned heap memory in the result using SDK free routines.
 struct DashSDKResult dash_sdk_token_transfer(struct dash_sdk_handle_t *sdk_handle, const uint8_t *transition_owner_id, const struct DashSDKTokenTransferParams *params, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Update token configuration and wait for confirmation
//
// # Safety
// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
// - `transition_owner_id` must point to at least 32 readable bytes.
// - `params`, `identity_public_key_handle`, `signer_handle` must be valid pointers to initialized structures.
// - Optional pointers (`put_settings`, `state_transition_creation_options`) may be null; when non-null they must be valid.
// - Caller must free any returned heap memory in the result using SDK free routines.
 struct DashSDKResult dash_sdk_token_update_contract_token_configuration(struct dash_sdk_handle_t *sdk_handle, const uint8_t *transition_owner_id, const struct DashSDKTokenConfigUpdateParams *params, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Destroy frozen token funds and wait for confirmation
//
// # Safety
// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
// - `transition_owner_id` must point to at least 32 readable bytes.
// - `params`, `identity_public_key_handle`, `signer_handle` must be valid pointers to initialized structures.
// - Optional pointers (`put_settings`, `state_transition_creation_options`) may be null; when non-null they must be valid.
// - The function may allocate and return heap memory via the DashSDKResult; caller must free it using SDK free routines.
 struct DashSDKResult dash_sdk_token_destroy_frozen_funds(struct dash_sdk_handle_t *sdk_handle, const uint8_t *transition_owner_id, const struct DashSDKTokenDestroyFrozenFundsParams *params, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Perform emergency action on token and wait for confirmation
//
// # Safety
// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
// - `transition_owner_id` must point to at least 32 readable bytes.
// - `params`, `identity_public_key_handle`, `signer_handle` must be valid, non-null pointers to initialized structures.
// - Optional pointers (`put_settings`, `state_transition_creation_options`) may be null; when non-null they must be valid.
// - Returned pointers embedded in DashSDKResult must be freed by the caller using SDK free routines.
 struct DashSDKResult dash_sdk_token_emergency_action(struct dash_sdk_handle_t *sdk_handle, const uint8_t *transition_owner_id, const struct DashSDKTokenEmergencyActionParams *params, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Freeze a token for an identity and wait for confirmation
//
// # Safety
// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
// - `transition_owner_id` must point to at least 32 readable bytes.
// - `params`, `identity_public_key_handle`, `signer_handle` must be valid pointers to initialized structures.
// - Optional pointers (`put_settings`, `state_transition_creation_options`) may be null; when non-null they must be valid.
// - Any returned heap memory in the result must be freed by the caller using SDK free routines.
 struct DashSDKResult dash_sdk_token_freeze(struct dash_sdk_handle_t *sdk_handle, const uint8_t *transition_owner_id, const struct DashSDKTokenFreezeParams *params, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Unfreeze a token for an identity and wait for confirmation
//
// # Safety
// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
// - `transition_owner_id` must point to at least 32 readable bytes.
// - `params`, `identity_public_key_handle`, and `signer_handle` must be valid pointers to initialized structures.
// - Optional pointers (`put_settings`, `state_transition_creation_options`) may be null; when non-null they must be valid.
// - Caller must free any returned heap memory from the result using SDK free routines.
 struct DashSDKResult dash_sdk_token_unfreeze(struct dash_sdk_handle_t *sdk_handle, const uint8_t *transition_owner_id, const struct DashSDKTokenFreezeParams *params, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Purchase tokens directly and wait for confirmation
//
// # Safety
// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
// - `transition_owner_id` must point to at least 32 readable bytes.
// - `params`, `identity_public_key_handle`, and `signer_handle` must be valid pointers to initialized structures.
// - Optional pointers (`put_settings`, `state_transition_creation_options`) may be null; when non-null they must be valid.
// - Caller must free any returned heap memory in the result using SDK-provided free routines.
 struct DashSDKResult dash_sdk_token_purchase(struct dash_sdk_handle_t *sdk_handle, const uint8_t *transition_owner_id, const struct DashSDKTokenPurchaseParams *params, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Set token price for direct purchase and wait for confirmation
//
// # Safety
// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
// - `transition_owner_id` must point to at least 32 readable bytes.
// - `params`, `identity_public_key_handle`, `signer_handle` must be valid pointers to initialized structures.
// - Optional pointers (`put_settings`, `state_transition_creation_options`) may be null; when non-null they must be valid.
// - Caller must free any returned heap memory in the result using SDK free routines.
 struct DashSDKResult dash_sdk_token_set_price(struct dash_sdk_handle_t *sdk_handle, const uint8_t *transition_owner_id, const struct DashSDKTokenSetPriceParams *params, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Get identity token balances
//
// This is an alias for dash_sdk_identity_fetch_token_balances for backward compatibility
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `identity_id`: Base58-encoded identity ID
// - `token_ids`: Comma-separated list of Base58-encoded token IDs
//
// # Returns
// JSON string containing token IDs mapped to their balances
// # Safety
// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
// - `identity_id` and `token_ids` must be valid pointers to NUL-terminated C strings and readable during the call.
// - The returned pointer (on success) must be freed using the SDK's free routine to avoid memory leaks.
 struct DashSDKResult dash_sdk_token_get_identity_balances(const struct dash_sdk_handle_t *sdk_handle, const char *identity_id, const char *token_ids) ;

// Get token contract info
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `token_id`: Base58-encoded token ID
//
// # Returns
// JSON string containing the contract ID and token position, or null if not found
// # Safety
// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
// - `token_id` must be a valid pointer to a NUL-terminated C string and readable during the call.
// - The returned C string pointer (on success) must be freed with the SDK's string-free function by the caller.
 struct DashSDKResult dash_sdk_token_get_contract_info(const struct dash_sdk_handle_t *sdk_handle, const char *token_id) ;

// Get token direct purchase prices
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `token_ids`: Comma-separated list of Base58-encoded token IDs
//
// # Returns
// JSON string containing token IDs mapped to their pricing information
// # Safety
// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
// - `token_ids` must be a valid pointer to a NUL-terminated C string and readable during the call.
// - The returned C string pointer (on success) must be freed by the caller using the SDK's free function.
 struct DashSDKResult dash_sdk_token_get_direct_purchase_prices(const struct dash_sdk_handle_t *sdk_handle, const char *token_ids) ;

// Fetch token balances for multiple identities for a specific token
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `identity_ids`: Either a comma-separated list OR a JSON array of Base58-encoded identity IDs
// - `token_id`: Base58-encoded token ID
//
// # Returns
// JSON string containing identity IDs mapped to their token balances
// # Safety
// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
// - `identity_ids` and `token_id` must be valid pointers to NUL-terminated C strings and readable during the call.
// - The returned C string pointer (on success) must be freed using the SDK's free function.
 struct DashSDKResult dash_sdk_identities_fetch_token_balances(const struct dash_sdk_handle_t *sdk_handle, const char *identity_ids, const char *token_id) ;

// Fetch token information for multiple identities for a specific token
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `identity_ids`: Comma-separated list of Base58-encoded identity IDs
// - `token_id`: Base58-encoded token ID
//
// # Returns
// JSON string containing identity IDs mapped to their token information
// # Safety
// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
// - `identity_ids` and `token_id` must be valid pointers to NUL-terminated C strings and readable during the call.
// - The returned C string pointer (on success) must be freed with the SDK's free function.
 struct DashSDKResult dash_sdk_identities_fetch_token_infos(const struct dash_sdk_handle_t *sdk_handle, const char *identity_ids, const char *token_id) ;

// Fetch token balances for a specific identity
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `identity_id`: Base58-encoded identity ID
// - `token_ids`: Comma-separated list of Base58-encoded token IDs
//
// # Returns
// JSON string containing token IDs mapped to their balances
// # Safety
// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
// - `identity_id` and `token_ids` must be valid pointers to NUL-terminated C strings and readable during the call.
// - The returned C string pointer (on success) must be freed using the SDK's free function to avoid leaks.
 struct DashSDKResult dash_sdk_identity_fetch_token_balances(const struct dash_sdk_handle_t *sdk_handle, const char *identity_id, const char *token_ids) ;

// Fetch token information for a specific identity
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `identity_id`: Base58-encoded identity ID
// - `token_ids`: Comma-separated list of Base58-encoded token IDs
//
// # Returns
// JSON string containing token IDs mapped to their information
// # Safety
// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
// - `identity_id` and `token_ids` must be valid pointers to NUL-terminated C strings and readable during the call.
// - The returned C string pointer (on success) must be freed using the SDK's free function to avoid leaks.
 struct DashSDKResult dash_sdk_identity_fetch_token_infos(const struct dash_sdk_handle_t *sdk_handle, const char *identity_id, const char *token_ids) ;

// Get identity token information
//
// This is an alias for dash_sdk_identity_fetch_token_infos for backward compatibility
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `identity_id`: Base58-encoded identity ID
// - `token_ids`: Comma-separated list of Base58-encoded token IDs
//
// # Returns
// JSON string containing token IDs mapped to their information
// # Safety
// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
// - `identity_id` and `token_ids` must be valid pointers to NUL-terminated C strings and readable for the call duration.
// - The returned string pointer (on success) must be freed with the SDK's string free routine to avoid leaks.
 struct DashSDKResult dash_sdk_token_get_identity_infos(const struct dash_sdk_handle_t *sdk_handle, const char *identity_id, const char *token_ids) ;

// Get token perpetual distribution last claim
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `token_id`: Base58-encoded token ID
// - `identity_id`: Base58-encoded identity ID
//
// # Returns
// JSON string containing the last claim information
// # Safety
// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
// - `token_id` and `identity_id` must be valid pointers to NUL-terminated C strings and readable during the call.
// - The returned C string pointer (on success) must be freed by the caller using the SDK's free function to avoid memory leaks.
 struct DashSDKResult dash_sdk_token_get_perpetual_distribution_last_claim(const struct dash_sdk_handle_t *sdk_handle, const char *token_id, const char *identity_id) ;

// Get token statuses
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `token_ids`: Comma-separated list of Base58-encoded token IDs
//
// # Returns
// JSON string containing token IDs mapped to their status information
// # Safety
// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
// - `token_ids` must be a valid pointer to a NUL-terminated C string containing comma-separated IDs.
// - The returned C string pointer (on success) must be freed by the caller using the SDK's free function.
 struct DashSDKResult dash_sdk_token_get_statuses(const struct dash_sdk_handle_t *sdk_handle, const char *token_ids) ;

// Fetches the total supply of a token
//
// # Parameters
// * `sdk_handle` - Handle to the SDK instance
// * `token_id` - Base58-encoded token identifier
//
// # Returns
// * JSON string with token supply info or null if not found
// * Error message if operation fails
//
// # Safety
// This function is unsafe because it handles raw pointers from C
 struct DashSDKResult dash_sdk_token_get_total_supply(const struct dash_sdk_handle_t *sdk_handle, const char *token_id) ;

// Free a string allocated by the FFI
//
// # Safety
// - `s` must be a pointer returned by this SDK to a heap-allocated NUL-terminated C string.
// - Passing a pointer not allocated by this SDK, or a pointer already freed, results in undefined behavior.
// - `s` may be null, in which case this is a no-op.
 void dash_sdk_string_free(char *s) ;

// Free binary data allocated by the FFI
//
// # Safety
// - `binary_data` must be a valid pointer returned by this SDK and not previously freed.
// - When non-null, the function takes ownership and frees both the struct and its internal buffer.
// - Do not use `binary_data` after this call.
 void dash_sdk_binary_data_free(struct DashSDKBinaryData *binary_data) ;

// Free an identity info structure
//
// # Safety
// - `info` must be a valid pointer to `DashSDKIdentityInfo` allocated by this SDK.
// - It may be null (no-op). When non-null, this frees any owned strings and the struct.
// - Do not access `info` after this call.
 void dash_sdk_identity_info_free(struct DashSDKIdentityInfo *info) ;

// Free a document info structure
//
// # Safety
// - `info` must be a valid pointer to `DashSDKDocumentInfo` allocated by this SDK and not already freed.
// - It may be null (no-op). When non-null, this frees all owned strings and arrays.
// - Pointer must not be dereferenced after this call.
 void dash_sdk_document_info_free(struct DashSDKDocumentInfo *info) ;

// Free an identity balance map
//
// # Safety
// - `map` must be a valid, non-dangling pointer returned by this SDK.
// - It may be null (no-op). When non-null, this frees the entries array and the struct.
// - Using `map` after this function returns is undefined behavior.
 void dash_sdk_identity_balance_map_free(struct DashSDKIdentityBalanceMap *map) ;

// Free a contender structure
//
// # Safety
// - `contender` must be a valid, non-dangling pointer obtained from this FFI (e.g., via an SDK function).
// - It must either be null (a no-op) or point to a heap-allocated `DashSDKContender` that has not been freed yet.
// - After this call, the pointer must not be used again (use-after-free is undefined behavior).
// - This function will also free any heap-allocated strings owned by the structure.
// # Safety
// - `contender` must be a valid, non-null pointer to a `DashSDKContender` allocated by this SDK, or null for no-op.
// - The pointer must not be used after this call.
 void dash_sdk_contender_free(struct DashSDKContender *contender) ;

// Free contest info structure
//
// # Safety
// - `info` must be a valid, non-dangling pointer obtained from this FFI and not previously freed.
// - It may be null (no-op). When non-null, this frees the owned contender array and contained strings.
// - Do not use `info` after this call; doing so is undefined behavior.
 void dash_sdk_contest_info_free(struct DashSDKContestInfo *info) ;

// Free a contested name structure
//
// # Safety
// - `name` must be a valid, non-dangling pointer to a `DashSDKContestedName` allocated by this SDK.
// - It may be null (no-op). When non-null, this frees the embedded strings and contender buffers.
// - Do not access `name` after freeing.
 void dash_sdk_contested_name_free(struct DashSDKContestedName *name) ;

// Free a contested names list
//
// # Safety
// - `list` must be a valid pointer returned by this SDK and not previously freed.
// - It may be null (no-op). When non-null, this frees the array of names and any nested strings/buffers.
// - Do not use `list` after this call.
 void dash_sdk_contested_names_list_free(struct DashSDKContestedNamesList *list) ;

// Free a name-timestamp structure
//
// # Safety
// - `entry` must be a valid, non-dangling pointer to a `DashSDKNameTimestamp` allocated by this SDK.
// - It may be null (no-op). When non-null, this frees the owned string and the struct.
// - Do not use `entry` after this call.
 void dash_sdk_name_timestamp_free(struct DashSDKNameTimestamp *entry) ;

// Free a name-timestamp list
//
// # Safety
// - `list` must be a valid pointer to a `DashSDKNameTimestampList` allocated by this SDK.
// - It may be null (no-op). When non-null, this frees the entries array and contained strings.
// - Pointer must not be used after this call.
 void dash_sdk_name_timestamp_list_free(struct DashSDKNameTimestampList *list) ;

// Initialize the unified SDK system
// This initializes both Core SDK (if enabled) and Platform SDK
 int32_t dash_unified_sdk_init(void) ;

// Create a unified SDK handle with both Core and Platform SDKs
//
// # Safety
// - `config` must point to a valid UnifiedSDKConfig structure
 struct UnifiedSDKHandle *dash_unified_sdk_create(const struct UnifiedSDKConfig *config) ;

// Destroy a unified SDK handle
//
// # Safety
// - `handle` must be a valid unified SDK handle or null
 void dash_unified_sdk_destroy(struct UnifiedSDKHandle *handle) ;

// Start both Core and Platform SDKs
//
// # Safety
// - `handle` must be a valid unified SDK handle
 int32_t dash_unified_sdk_start(struct UnifiedSDKHandle *handle) ;

// Stop both Core and Platform SDKs
//
// # Safety
// - `handle` must be a valid unified SDK handle
 int32_t dash_unified_sdk_stop(struct UnifiedSDKHandle *handle) ;

// Get the Core SDK client from a unified handle
//
// # Safety
// - `handle` must be a valid unified SDK handle
 FFIDashSpvClient *dash_unified_sdk_get_core_client(struct UnifiedSDKHandle *handle) ;

// Get the Platform SDK from a unified handle
//
// # Safety
// - `handle` must be a valid unified SDK handle
 struct dash_sdk_handle_t *dash_unified_sdk_get_platform_sdk(struct UnifiedSDKHandle *handle) ;

// Check if integration is enabled for this unified SDK
//
// # Safety
// - `handle` must be a valid unified SDK handle
 bool dash_unified_sdk_is_integration_enabled(struct UnifiedSDKHandle *handle) ;

// Check if Core SDK is available in this unified SDK
//
// # Safety
// - `handle` must be a valid unified SDK handle
 bool dash_unified_sdk_has_core_sdk(struct UnifiedSDKHandle *handle) ;

// Register Core SDK with Platform SDK for context provider callbacks
// This enables Platform SDK to query Core SDK for blockchain state
//
// # Safety
// - `handle` must be a valid unified SDK handle
 int32_t dash_unified_sdk_register_core_context(struct UnifiedSDKHandle *handle) ;

// Get combined status of both SDKs
//
// # Safety
// - `handle` must be a valid unified SDK handle
// - `core_height` must point to a valid u32 (set to 0 if core disabled)
// - `platform_ready` must point to a valid bool
 int32_t dash_unified_sdk_get_status(struct UnifiedSDKHandle *handle, uint32_t *core_height, bool *platform_ready) ;

// Get unified SDK version information
 const char *dash_unified_sdk_version(void) ;

// Check if unified SDK was compiled with core support
 bool dash_unified_sdk_has_core_support(void) ;

// Convert a hex string to base58
//
// # Parameters
// - `hex_string`: Hex encoded string (must be 64 characters for identity IDs)
//
// # Returns
// - Base58 encoded string on success
// - Error if the hex string is invalid
// # Safety
// - `hex_string` must be a valid, non-null pointer to a NUL-terminated C string.
// - The memory pointed to by `hex_string` must be readable for the duration of the call.
// - The returned pointer (on success) must be freed by calling the appropriate free function
//   (e.g., `dash_sdk_string_free`) from this SDK to avoid memory leaks.
 struct DashSDKResult dash_sdk_utils_hex_to_base58(const char *hex_string) ;

// Convert a base58 string to hex
//
// # Parameters
// - `base58_string`: Base58 encoded string
//
// # Returns
// - Hex encoded string on success
// - Error if the base58 string is invalid
// # Safety
// - `base58_string` must be a valid, non-null pointer to a NUL-terminated C string.
// - The memory pointed to by `base58_string` must be readable for the duration of the call.
// - The returned C string pointer (on success) must be freed with the SDK's string free routine to avoid leaks.
 struct DashSDKResult dash_sdk_utils_base58_to_hex(const char *base58_string) ;

// Validate if a string is valid base58
//
// # Parameters
// - `string`: String to validate
//
// # Returns
// - 1 if valid base58, 0 if invalid
// # Safety
// - `string` must be a valid, non-null pointer to a NUL-terminated C string.
// - The memory pointed to by `string` must be readable for the duration of the call.
 uint8_t dash_sdk_utils_is_valid_base58(const char *string) ;

// Fetches vote polls by end date
//
// # Parameters
// * `sdk_handle` - Handle to the SDK instance
// * `start_time_ms` - Start time in milliseconds (optional, 0 for no start time)
// * `start_time_included` - Whether to include the start time
// * `end_time_ms` - End time in milliseconds (optional, 0 for no end time)
// * `end_time_included` - Whether to include the end time
// * `limit` - Maximum number of results to return (optional, 0 for no limit)
// * `offset` - Number of results to skip (optional, 0 for no offset)
// * `ascending` - Whether to order results in ascending order
//
// # Returns
// * JSON array of vote polls grouped by timestamp or null if not found
// * Error message if operation fails
//
// # Safety
// This function is unsafe because it handles raw pointers from C
 struct DashSDKResult dash_sdk_voting_get_vote_polls_by_end_date(const struct dash_sdk_handle_t *sdk_handle, uint64_t start_time_ms, bool start_time_included, uint64_t end_time_ms, bool end_time_included, uint32_t limit, uint32_t offset, bool ascending) ;

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* DASH_SDK_FFI_H */
