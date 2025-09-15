#ifndef DASH_SDK_FFI_H
#define DASH_SDK_FFI_H

#pragma once

/* Generated with cbindgen:0.29.0 */

/* This file is auto-generated. Do not modify manually. */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>

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

// Network type for SDK configuration
typedef enum DashSDKNetwork {
  // Mainnet
  DashSDKNetwork_Mainnet = 0,
  // Testnet
  DashSDKNetwork_Testnet = 1,
  // Devnet
  DashSDKNetwork_Devnet = 2,
  // Local development network
  DashSDKNetwork_Local = 3,
} DashSDKNetwork;

// Result data type indicator for iOS
typedef enum DashSDKResultDataType {
  // No data (void/null)
  DashSDKResultDataType_None = 0,
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
} DashSDKResultDataType;

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

// Token distribution type for claim operations
typedef enum DashSDKTokenDistributionType {
  // Pre-programmed distribution
  DashSDKTokenDistributionType_PreProgrammed = 0,
  // Perpetual distribution
  DashSDKTokenDistributionType_Perpetual = 1,
} DashSDKTokenDistributionType;

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

typedef struct FFIDashSpvClient {
  uint8_t opaque[0];
} FFIDashSpvClient;

// Handle for Core SDK that can be passed to Platform SDK
// This matches the definition from dash_spv_ffi.h
typedef struct CoreSDKHandle {
  struct FFIDashSpvClient *client;
} CoreSDKHandle;

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

// Document creation parameters
typedef struct DashSDKDocumentCreateParams {
  // Data contract handle
  const struct DataContractHandle *data_contract_handle;
  // Document type name
  const char *document_type;
  // Owner identity handle
  const struct IdentityHandle *owner_identity_handle;
  // JSON string of document properties
  const char *properties_json;
} DashSDKDocumentCreateParams;

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

// Extended SDK configuration with context provider support
typedef struct DashSDKConfigExtended {
  // Base SDK configuration
  struct DashSDKConfig base_config;
  // Optional context provider handle
  struct ContextProviderHandle *context_provider;
  // Optional Core SDK handle for automatic context provider creation
  struct CoreSDKHandle *core_sdk_handle;
} DashSDKConfigExtended;

// Function pointer type for iOS signing callback
// Returns pointer to allocated byte array (caller must free with dash_sdk_bytes_free)
// Returns null on error
typedef uint8_t *(*IOSSignCallback)(const uint8_t *identity_public_key_bytes, uintptr_t identity_public_key_len, const uint8_t *data, uintptr_t data_len, uintptr_t *result_len);

// Function pointer type for iOS can_sign_with callback
typedef bool (*IOSCanSignCallback)(const uint8_t *identity_public_key_bytes, uintptr_t identity_public_key_len);

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
  CoreSDKClient *core_client;
  struct dash_sdk_handle_t *platform_sdk;
  bool integration_enabled;
} UnifiedSDKHandle;

// Unified SDK configuration combining both Core and Platform settings
typedef struct UnifiedSDKConfig {
  // Core SDK configuration (ignored if core feature disabled)
  CoreSDKConfig core_config;
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

// Get the version of the Dash SDK FFI library
 const char *dash_sdk_version(void) ;

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
// This function is unsafe because it handles raw pointers from C
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
// This function is unsafe because it handles raw pointers from C
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
// This function is unsafe because it handles raw pointers from C
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
// This function is unsafe because it handles raw pointers from C
 struct DashSDKResult dash_sdk_contested_resource_get_voters_for_identity(const struct dash_sdk_handle_t *sdk_handle, const char *contract_id, const char *document_type_name, const char *index_name, const char *index_values_json, const char *contestant_id, uint32_t count, bool order_ascending) ;

// Create a context provider from a Core SDK handle (DEPRECATED)
//
// This function is deprecated. Use dash_sdk_context_provider_from_callbacks instead.
//
// # Safety
// - `core_handle` must be a valid Core SDK handle
// - String parameters must be valid UTF-8 C strings or null
 struct ContextProviderHandle *dash_sdk_context_provider_from_core(struct CoreSDKHandle *core_handle, const char *core_rpc_url, const char *core_rpc_user, const char *core_rpc_password) ;

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

// Initialize the Core SDK
// Returns 0 on success, error code on failure
 int32_t dash_core_sdk_init(void) ;

// Create a Core SDK client with testnet config
//
// # Safety
// - Returns null on failure
 CoreSDKClient *dash_core_sdk_create_client_testnet(void) ;

// Create a Core SDK client with mainnet config
//
// # Safety
// - Returns null on failure
 CoreSDKClient *dash_core_sdk_create_client_mainnet(void) ;

// Create a Core SDK client with custom config
//
// # Safety
// - `config` must be a valid CoreSDKConfig pointer
// - Returns null on failure
 CoreSDKClient *dash_core_sdk_create_client(const CoreSDKConfig *config) ;

// Destroy a Core SDK client
//
// # Safety
// - `client` must be a valid Core SDK client handle or null
 void dash_core_sdk_destroy_client(CoreSDKClient *client) ;

// Start the Core SDK client (begin sync)
//
// # Safety
// - `client` must be a valid Core SDK client handle
 int32_t dash_core_sdk_start(CoreSDKClient *client) ;

// Stop the Core SDK client
//
// # Safety
// - `client` must be a valid Core SDK client handle
 int32_t dash_core_sdk_stop(CoreSDKClient *client) ;

// Sync Core SDK client to tip
//
// # Safety
// - `client` must be a valid Core SDK client handle
 int32_t dash_core_sdk_sync_to_tip(CoreSDKClient *client) ;

// Get the current sync progress
//
// # Safety
// - `client` must be a valid Core SDK client handle
// - Returns pointer to FFISyncProgress structure (caller must free it)
 FFISyncProgress *dash_core_sdk_get_sync_progress(CoreSDKClient *client) ;

// Get Core SDK statistics
//
// # Safety
// - `client` must be a valid Core SDK client handle
// - Returns pointer to FFISpvStats structure (caller must free it)
 FFISpvStats *dash_core_sdk_get_stats(CoreSDKClient *client) ;

// Get the current block height
//
// # Safety
// - `client` must be a valid Core SDK client handle
// - `height` must point to a valid u32
 int32_t dash_core_sdk_get_block_height(CoreSDKClient *client, uint32_t *height) ;

// Add an address to watch
//
// # Safety
// - `client` must be a valid Core SDK client handle
// - `address` must be a valid null-terminated C string
 int32_t dash_core_sdk_watch_address(CoreSDKClient *client, const char *address) ;

// Remove an address from watching
//
// # Safety
// - `client` must be a valid Core SDK client handle
// - `address` must be a valid null-terminated C string
 int32_t dash_core_sdk_unwatch_address(CoreSDKClient *client, const char *address) ;

// Get balance for all watched addresses
//
// # Safety
// - `client` must be a valid Core SDK client handle
// - Returns pointer to FFIBalance structure (caller must free it)
 FFIBalance *dash_core_sdk_get_total_balance(CoreSDKClient *client) ;

// Get platform activation height
//
// # Safety
// - `client` must be a valid Core SDK client handle
// - `height` must point to a valid u32
 int32_t dash_core_sdk_get_platform_activation_height(CoreSDKClient *client, uint32_t *height) ;

// Get quorum public key
//
// # Safety
// - `client` must be a valid Core SDK client handle
// - `quorum_hash` must point to a valid 32-byte buffer
// - `public_key` must point to a valid 48-byte buffer
 int32_t dash_core_sdk_get_quorum_public_key(CoreSDKClient *client, uint32_t quorum_type, const uint8_t *quorum_hash, uint32_t core_chain_locked_height, uint8_t *public_key, uintptr_t public_key_size) ;

// Get Core SDK handle for platform integration
//
// # Safety
// - `client` must be a valid Core SDK client handle
 struct CoreSDKHandle *dash_core_sdk_get_core_handle(CoreSDKClient *client) ;

// Broadcast a transaction
//
// # Safety
// - `client` must be a valid Core SDK client handle
// - `transaction_hex` must be a valid null-terminated C string
 int32_t dash_core_sdk_broadcast_transaction(CoreSDKClient *client, const char *transaction_hex) ;

// Check if Core SDK feature is enabled at runtime
 bool dash_core_sdk_is_enabled(void) ;

// Get Core SDK version
 const char *dash_core_sdk_version(void) ;

// Create a new data contract
 struct DashSDKResult dash_sdk_data_contract_create(struct dash_sdk_handle_t *sdk_handle, const struct IdentityHandle *owner_identity_handle, const char *documents_schema_json) ;

// Destroy a data contract handle
 void dash_sdk_data_contract_destroy(struct DataContractHandle *handle) ;

// Put data contract to platform (broadcast state transition)
 struct DashSDKResult dash_sdk_data_contract_put_to_platform(struct dash_sdk_handle_t *sdk_handle, const struct DataContractHandle *data_contract_handle, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle) ;

// Put data contract to platform and wait for confirmation (broadcast state transition and wait for response)
 struct DashSDKResult dash_sdk_data_contract_put_to_platform_and_wait(struct dash_sdk_handle_t *sdk_handle, const struct DataContractHandle *data_contract_handle, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle) ;

// Fetch a data contract by ID
 struct DashSDKResult dash_sdk_data_contract_fetch(const struct dash_sdk_handle_t *sdk_handle, const char *contract_id) ;

// Fetch multiple data contracts by their IDs
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `contract_ids`: Comma-separated list of Base58-encoded contract IDs
//
// # Returns
// JSON string containing contract IDs mapped to their data contracts
 struct DashSDKResult dash_sdk_data_contracts_fetch_many(const struct dash_sdk_handle_t *sdk_handle, const char *contract_ids) ;

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
 struct DashSDKResult dash_sdk_data_contract_fetch_history(const struct dash_sdk_handle_t *sdk_handle, const char *contract_id, unsigned int limit, unsigned int offset, uint64_t start_at_ms) ;

// Get schema for a specific document type
 char *dash_sdk_data_contract_get_schema(const struct DataContractHandle *contract_handle, const char *document_type) ;

// Create a new document
 struct DashSDKResult dash_sdk_document_create(struct dash_sdk_handle_t *sdk_handle, const struct DashSDKDocumentCreateParams *params) ;

// Delete a document from the platform
 struct DashSDKResult dash_sdk_document_delete(struct dash_sdk_handle_t *sdk_handle, const struct DocumentHandle *document_handle, const struct DataContractHandle *data_contract_handle, const char *document_type_name, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKTokenPaymentInfo *token_payment_info, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Delete a document from the platform and wait for confirmation
 struct DashSDKResult dash_sdk_document_delete_and_wait(struct dash_sdk_handle_t *sdk_handle, const struct DocumentHandle *document_handle, const struct DataContractHandle *data_contract_handle, const char *document_type_name, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKTokenPaymentInfo *token_payment_info, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Update document price (broadcast state transition)
 struct DashSDKResult dash_sdk_document_update_price_of_document(struct dash_sdk_handle_t *sdk_handle, const struct DocumentHandle *document_handle, const struct DataContractHandle *data_contract_handle, const char *document_type_name, uint64_t price, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKTokenPaymentInfo *token_payment_info, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Update document price and wait for confirmation (broadcast state transition and wait for response)
 struct DashSDKResult dash_sdk_document_update_price_of_document_and_wait(struct dash_sdk_handle_t *sdk_handle, const struct DocumentHandle *document_handle, const struct DataContractHandle *data_contract_handle, const char *document_type_name, uint64_t price, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKTokenPaymentInfo *token_payment_info, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Purchase document (broadcast state transition)
 struct DashSDKResult dash_sdk_document_purchase(struct dash_sdk_handle_t *sdk_handle, const struct DocumentHandle *document_handle, const struct DataContractHandle *data_contract_handle, const char *document_type_name, uint64_t price, const char *purchaser_id, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKTokenPaymentInfo *token_payment_info, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Purchase document and wait for confirmation (broadcast state transition and wait for response)
 struct DashSDKResult dash_sdk_document_purchase_and_wait(struct dash_sdk_handle_t *sdk_handle, const struct DocumentHandle *document_handle, const struct DataContractHandle *data_contract_handle, const char *document_type_name, uint64_t price, const char *purchaser_id, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKTokenPaymentInfo *token_payment_info, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Put document to platform (broadcast state transition)
 struct DashSDKResult dash_sdk_document_put_to_platform(struct dash_sdk_handle_t *sdk_handle, const struct DocumentHandle *document_handle, const struct DataContractHandle *data_contract_handle, const char *document_type_name, const uint8_t (*entropy)[32], const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKTokenPaymentInfo *token_payment_info, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Put document to platform and wait for confirmation (broadcast state transition and wait for response)
 struct DashSDKResult dash_sdk_document_put_to_platform_and_wait(struct dash_sdk_handle_t *sdk_handle, const struct DocumentHandle *document_handle, const struct DataContractHandle *data_contract_handle, const char *document_type_name, const uint8_t (*entropy)[32], const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKTokenPaymentInfo *token_payment_info, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Fetch a document by ID
 struct DashSDKResult dash_sdk_document_fetch(const struct dash_sdk_handle_t *sdk_handle, const struct DataContractHandle *data_contract_handle, const char *document_type, const char *document_id) ;

// Get document information
 struct DashSDKDocumentInfo *dash_sdk_document_get_info(const struct DocumentHandle *document_handle) ;

// Search for documents
 struct DashSDKResult dash_sdk_document_search(const struct dash_sdk_handle_t *sdk_handle, const struct DashSDKDocumentSearchParams *params) ;

// Replace document on platform (broadcast state transition)
 struct DashSDKResult dash_sdk_document_replace_on_platform(struct dash_sdk_handle_t *sdk_handle, const struct DocumentHandle *document_handle, const struct DataContractHandle *data_contract_handle, const char *document_type_name, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKTokenPaymentInfo *token_payment_info, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Replace document on platform and wait for confirmation (broadcast state transition and wait for response)
 struct DashSDKResult dash_sdk_document_replace_on_platform_and_wait(struct dash_sdk_handle_t *sdk_handle, const struct DocumentHandle *document_handle, const struct DataContractHandle *data_contract_handle, const char *document_type_name, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKTokenPaymentInfo *token_payment_info, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

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
 struct DashSDKResult dash_sdk_document_transfer_to_identity(struct dash_sdk_handle_t *sdk_handle, const struct DocumentHandle *document_handle, const char *recipient_id, const struct DataContractHandle *data_contract_handle, const char *document_type_name, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKTokenPaymentInfo *token_payment_info, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

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
 struct DashSDKResult dash_sdk_document_transfer_to_identity_and_wait(struct dash_sdk_handle_t *sdk_handle, const struct DocumentHandle *document_handle, const char *recipient_id, const struct DataContractHandle *data_contract_handle, const char *document_type_name, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKTokenPaymentInfo *token_payment_info, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Destroy a document
 struct DashSDKError *dash_sdk_document_destroy(struct dash_sdk_handle_t *sdk_handle, struct DocumentHandle *document_handle) ;

// Destroy a document handle
 void dash_sdk_document_handle_destroy(struct DocumentHandle *handle) ;

// Free an error message
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
// This function is unsafe because it handles raw pointers from C
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
// This function is unsafe because it handles raw pointers from C
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
// This function is unsafe because it handles raw pointers from C
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
// This function is unsafe because it handles raw pointers from C
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
// This function is unsafe because it handles raw pointers from C
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
// This function is unsafe because it handles raw pointers from C
 struct DashSDKResult dash_sdk_group_get_infos(const struct dash_sdk_handle_t *sdk_handle, const char *start_at_position, uint32_t limit) ;

// Create a new identity
 struct DashSDKResult dash_sdk_identity_create(struct dash_sdk_handle_t *sdk_handle) ;

// Get identity information
 struct DashSDKIdentityInfo *dash_sdk_identity_get_info(const struct IdentityHandle *identity_handle) ;

// Destroy an identity handle
 void dash_sdk_identity_destroy(struct IdentityHandle *handle) ;

// Register a name for an identity
 struct DashSDKError *dash_sdk_identity_register_name(struct dash_sdk_handle_t *sdk_handle, const struct IdentityHandle *identity_handle, const char *name) ;

// Put identity to platform with instant lock proof
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
// # Parameters
// - `core_chain_locked_height`: Core height at which the transaction was chain locked
// - `out_point`: 36-byte OutPoint (32-byte txid + 4-byte vout)
// - `private_key`: 32-byte private key associated with the asset lock
// - `put_settings`: Optional settings for the operation (can be null for defaults)
 struct DashSDKResult dash_sdk_identity_put_to_platform_with_chain_lock(struct dash_sdk_handle_t *sdk_handle, const struct IdentityHandle *identity_handle, uint32_t core_chain_locked_height, const uint8_t (*out_point)[36], const uint8_t (*private_key)[32], const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings) ;

// Put identity to platform with chain lock proof and wait for confirmation
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
 struct DashSDKResult dash_sdk_identity_fetch_balance(const struct dash_sdk_handle_t *sdk_handle, const char *identity_id) ;

// Fetch identity balance and revision
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `identity_id`: Base58-encoded identity ID
//
// # Returns
// JSON string containing the balance and revision information
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
 struct DashSDKResult dash_sdk_identity_fetch_by_non_unique_public_key_hash(const struct dash_sdk_handle_t *sdk_handle, const char *public_key_hash, const char *start_after) ;

// Fetch identity by public key hash
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
 struct DashSDKResult dash_sdk_identity_fetch_contract_nonce(const struct dash_sdk_handle_t *sdk_handle, const char *identity_id, const char *contract_id) ;

// Fetch an identity by ID
 struct DashSDKResult dash_sdk_identity_fetch(const struct dash_sdk_handle_t *sdk_handle, const char *identity_id) ;

// Fetch balances for multiple identities
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
 struct DashSDKResult dash_sdk_identities_fetch_contract_keys(const struct dash_sdk_handle_t *sdk_handle, const char *identity_ids, const char *contract_id, const char *document_type_name, const char *purposes) ;

// Fetch identity nonce
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `identity_id`: Base58-encoded identity ID
//
// # Returns
// The nonce of the identity as a string
 struct DashSDKResult dash_sdk_identity_fetch_nonce(const struct dash_sdk_handle_t *sdk_handle, const char *identity_id) ;

// Fetch identity public keys
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
 struct DashSDKResult dash_sdk_identity_resolve_name(const struct dash_sdk_handle_t *sdk_handle, const char *name) ;

// Top up an identity with credits using instant lock proof
 struct DashSDKResult dash_sdk_identity_topup_with_instant_lock(struct dash_sdk_handle_t *sdk_handle, const struct IdentityHandle *identity_handle, const uint8_t *instant_lock_bytes, uintptr_t instant_lock_len, const uint8_t *transaction_bytes, uintptr_t transaction_len, uint32_t output_index, const uint8_t (*private_key)[32], const struct DashSDKPutSettings *put_settings) ;

// Top up an identity with credits using instant lock proof and wait for confirmation
 struct DashSDKResult dash_sdk_identity_topup_with_instant_lock_and_wait(struct dash_sdk_handle_t *sdk_handle, const struct IdentityHandle *identity_handle, const uint8_t *instant_lock_bytes, uintptr_t instant_lock_len, const uint8_t *transaction_bytes, uintptr_t transaction_len, uint32_t output_index, const uint8_t (*private_key)[32], const struct DashSDKPutSettings *put_settings) ;

// Transfer credits from one identity to another
//
// # Parameters
// - `from_identity_handle`: Identity to transfer credits from
// - `to_identity_id`: Base58-encoded ID of the identity to transfer credits to
// - `amount`: Amount of credits to transfer
// - `identity_public_key_handle`: Public key for signing (optional, pass null to auto-select)
// - `signer_handle`: Cryptographic signer
// - `put_settings`: Optional settings for the operation (can be null for defaults)
//
// # Returns
// DashSDKTransferCreditsResult with sender and receiver final balances on success
 struct DashSDKResult dash_sdk_identity_transfer_credits(struct dash_sdk_handle_t *sdk_handle, const struct IdentityHandle *from_identity_handle, const char *to_identity_id, uint64_t amount, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings) ;

// Free a transfer credits result structure
 void dash_sdk_transfer_credits_result_free(struct DashSDKTransferCreditsResult *result) ;

// Withdraw credits from identity to a Dash address
//
// # Parameters
// - `identity_handle`: Identity to withdraw credits from
// - `address`: Base58-encoded Dash address to withdraw to
// - `amount`: Amount of credits to withdraw
// - `core_fee_per_byte`: Core fee per byte (optional, pass 0 for default)
// - `identity_public_key_handle`: Public key for signing (optional, pass null to auto-select)
// - `signer_handle`: Cryptographic signer
// - `put_settings`: Optional settings for the operation (can be null for defaults)
//
// # Returns
// The new balance of the identity after withdrawal
 struct DashSDKResult dash_sdk_identity_withdraw(struct dash_sdk_handle_t *sdk_handle, const struct IdentityHandle *identity_handle, const char *address, uint64_t amount, uint32_t core_fee_per_byte, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings) ;

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
// This function is unsafe because it handles raw pointers from C
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
// This function is unsafe because it handles raw pointers from C
 struct DashSDKResult dash_sdk_protocol_version_get_upgrade_vote_status(const struct dash_sdk_handle_t *sdk_handle, const char *start_pro_tx_hash, uint32_t count) ;

// Create a new SDK instance
 struct DashSDKResult dash_sdk_create(const struct DashSDKConfig *config) ;

// Create a new SDK instance with extended configuration including context provider
 struct DashSDKResult dash_sdk_create_extended(const struct DashSDKConfigExtended *config) ;

// Destroy an SDK instance
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
 enum DashSDKNetwork dash_sdk_get_network(const struct dash_sdk_handle_t *handle) ;

// Create a mock SDK instance with a dump directory (for offline testing)
 struct dash_sdk_handle_t *dash_sdk_create_handle_with_mock(const char *dump_dir) ;

// Create a new iOS signer
 struct SignerHandle *dash_sdk_signer_create(IOSSignCallback sign_callback, IOSCanSignCallback can_sign_callback) ;

// Destroy an iOS signer
 void dash_sdk_signer_destroy(struct SignerHandle *handle) ;

// Free bytes allocated by iOS callbacks
 void dash_sdk_bytes_free(uint8_t *bytes) ;

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
// This function is unsafe because it handles raw pointers from C
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
// This function is unsafe because it handles raw pointers from C
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
// This function is unsafe because it handles raw pointers from C
 struct DashSDKResult dash_sdk_system_get_path_elements(const struct dash_sdk_handle_t *sdk_handle, const char *path_json, const char *keys_json) ;

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
// This function is unsafe because it handles raw pointers from C
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
// This function is unsafe because it handles raw pointers from C
 struct DashSDKResult dash_sdk_system_get_total_credits_in_platform(const struct dash_sdk_handle_t *sdk_handle) ;

// Burn tokens from an identity and wait for confirmation
 struct DashSDKResult dash_sdk_token_burn(struct dash_sdk_handle_t *sdk_handle, const uint8_t *transition_owner_id, const struct DashSDKTokenBurnParams *params, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Claim tokens from a distribution and wait for confirmation
 struct DashSDKResult dash_sdk_token_claim(struct dash_sdk_handle_t *sdk_handle, const uint8_t *transition_owner_id, const struct DashSDKTokenClaimParams *params, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Mint tokens to an identity and wait for confirmation
 struct DashSDKResult dash_sdk_token_mint(struct dash_sdk_handle_t *sdk_handle, const uint8_t *transition_owner_id, const struct DashSDKTokenMintParams *params, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Token transfer to another identity and wait for confirmation
 struct DashSDKResult dash_sdk_token_transfer(struct dash_sdk_handle_t *sdk_handle, const uint8_t *transition_owner_id, const struct DashSDKTokenTransferParams *params, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Update token configuration and wait for confirmation
 struct DashSDKResult dash_sdk_token_update_contract_token_configuration(struct dash_sdk_handle_t *sdk_handle, const uint8_t *transition_owner_id, const struct DashSDKTokenConfigUpdateParams *params, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Destroy frozen token funds and wait for confirmation
 struct DashSDKResult dash_sdk_token_destroy_frozen_funds(struct dash_sdk_handle_t *sdk_handle, const uint8_t *transition_owner_id, const struct DashSDKTokenDestroyFrozenFundsParams *params, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Perform emergency action on token and wait for confirmation
 struct DashSDKResult dash_sdk_token_emergency_action(struct dash_sdk_handle_t *sdk_handle, const uint8_t *transition_owner_id, const struct DashSDKTokenEmergencyActionParams *params, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Freeze a token for an identity and wait for confirmation
 struct DashSDKResult dash_sdk_token_freeze(struct dash_sdk_handle_t *sdk_handle, const uint8_t *transition_owner_id, const struct DashSDKTokenFreezeParams *params, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Unfreeze a token for an identity and wait for confirmation
 struct DashSDKResult dash_sdk_token_unfreeze(struct dash_sdk_handle_t *sdk_handle, const uint8_t *transition_owner_id, const struct DashSDKTokenFreezeParams *params, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Purchase tokens directly and wait for confirmation
 struct DashSDKResult dash_sdk_token_purchase(struct dash_sdk_handle_t *sdk_handle, const uint8_t *transition_owner_id, const struct DashSDKTokenPurchaseParams *params, const struct IdentityPublicKeyHandle *identity_public_key_handle, const struct SignerHandle *signer_handle, const struct DashSDKPutSettings *put_settings, const struct DashSDKStateTransitionCreationOptions *state_transition_creation_options) ;

// Set token price for direct purchase and wait for confirmation
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
 struct DashSDKResult dash_sdk_token_get_identity_balances(const struct dash_sdk_handle_t *sdk_handle, const char *identity_id, const char *token_ids) ;

// Get token contract info
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `token_id`: Base58-encoded token ID
//
// # Returns
// JSON string containing the contract ID and token position, or null if not found
 struct DashSDKResult dash_sdk_token_get_contract_info(const struct dash_sdk_handle_t *sdk_handle, const char *token_id) ;

// Get token direct purchase prices
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `token_ids`: Comma-separated list of Base58-encoded token IDs
//
// # Returns
// JSON string containing token IDs mapped to their pricing information
 struct DashSDKResult dash_sdk_token_get_direct_purchase_prices(const struct dash_sdk_handle_t *sdk_handle, const char *token_ids) ;

// Fetch token balances for multiple identities for a specific token
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `identity_ids`: Comma-separated list of Base58-encoded identity IDs
// - `token_id`: Base58-encoded token ID
//
// # Returns
// JSON string containing identity IDs mapped to their token balances
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
 struct DashSDKResult dash_sdk_token_get_perpetual_distribution_last_claim(const struct dash_sdk_handle_t *sdk_handle, const char *token_id, const char *identity_id) ;

// Get token statuses
//
// # Parameters
// - `sdk_handle`: SDK handle
// - `token_ids`: Comma-separated list of Base58-encoded token IDs
//
// # Returns
// JSON string containing token IDs mapped to their status information
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
 void dash_sdk_string_free(char *s) ;

// Free binary data allocated by the FFI
 void dash_sdk_binary_data_free(struct DashSDKBinaryData *binary_data) ;

// Free an identity info structure
 void dash_sdk_identity_info_free(struct DashSDKIdentityInfo *info) ;

// Free a document info structure
 void dash_sdk_document_info_free(struct DashSDKDocumentInfo *info) ;

// Free an identity balance map
 void dash_sdk_identity_balance_map_free(struct DashSDKIdentityBalanceMap *map) ;

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
 CoreSDKClient *dash_unified_sdk_get_core_client(struct UnifiedSDKHandle *handle) ;

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
