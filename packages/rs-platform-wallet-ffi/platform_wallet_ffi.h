/*
 * Platform Wallet FFI - C Header File
 *
 * C-compatible FFI bindings for rs-platform-wallet
 * Provides unified wallet management with Platform identity support
 */

#ifndef PLATFORM_WALLET_FFI_H
#define PLATFORM_WALLET_FFI_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ============================================================================
 * Types
 * ========================================================================== */

typedef uint64_t Handle;
#define NULL_HANDLE 0

/* Network types */
typedef enum {
    NETWORK_TYPE_MAINNET = 0,
    NETWORK_TYPE_TESTNET = 1,
    NETWORK_TYPE_DEVNET = 2,
    NETWORK_TYPE_REGTEST = 3,
} NetworkType;

/* FFI Result codes */
typedef enum {
    PLATFORM_WALLET_FFI_SUCCESS = 0,
    PLATFORM_WALLET_FFI_ERROR_INVALID_HANDLE = 1,
    PLATFORM_WALLET_FFI_ERROR_INVALID_PARAMETER = 2,
    PLATFORM_WALLET_FFI_ERROR_NULL_POINTER = 3,
    PLATFORM_WALLET_FFI_ERROR_SERIALIZATION = 4,
    PLATFORM_WALLET_FFI_ERROR_DESERIALIZATION = 5,
    PLATFORM_WALLET_FFI_ERROR_WALLET_OPERATION = 6,
    PLATFORM_WALLET_FFI_ERROR_IDENTITY_NOT_FOUND = 7,
    PLATFORM_WALLET_FFI_ERROR_CONTACT_NOT_FOUND = 8,
    PLATFORM_WALLET_FFI_ERROR_INVALID_NETWORK = 9,
    PLATFORM_WALLET_FFI_ERROR_INVALID_IDENTIFIER = 10,
    PLATFORM_WALLET_FFI_ERROR_MEMORY_ALLOCATION = 11,
    PLATFORM_WALLET_FFI_ERROR_UTF8_CONVERSION = 12,
    PLATFORM_WALLET_FFI_ERROR_UNKNOWN = 99,
} PlatformWalletFFIResult;

/* Error information */
typedef struct {
    PlatformWalletFFIResult code;
    char* message;
} PlatformWalletFFIError;

/* Identifier (32 bytes) */
typedef struct {
    uint8_t bytes[32];
} IdentifierBytes;

/* Block time */
typedef struct {
    uint64_t height;
    uint32_t core_height;
    uint64_t timestamp;
} BlockTime;

/* Contact request */
typedef struct {
    IdentifierBytes identity_id;
    char* label;
    uint64_t timestamp;
} ContactRequest;

/* Established contact */
typedef struct {
    IdentifierBytes identity_id;
    char* label;
    uint64_t established_at;
} EstablishedContact;

/* Array of identifiers */
typedef struct {
    IdentifierBytes* items;
    size_t count;
} IdentifierArray;

/* ============================================================================
 * Library Management
 * ========================================================================== */

/**
 * Initialize the FFI library
 * Must be called before using any other functions
 */
void platform_wallet_ffi_init(void);

/**
 * Get the version of the platform wallet FFI library
 * @return Version string (do not free)
 */
const char* platform_wallet_ffi_version(void);

/* ============================================================================
 * Error Management
 * ========================================================================== */

/**
 * Free error message
 * @param error Error to free
 */
void platform_wallet_ffi_error_free(PlatformWalletFFIError error);

/* ============================================================================
 * PlatformWalletInfo Functions
 * ========================================================================== */

/**
 * Create a new PlatformWalletInfo from seed bytes
 * @param seed_bytes Seed bytes (typically 64 bytes)
 * @param seed_len Length of seed
 * @param out_handle Output handle
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult platform_wallet_info_create_from_seed(
    const uint8_t* seed_bytes,
    size_t seed_len,
    Handle* out_handle,
    PlatformWalletFFIError* out_error
);

/**
 * Create a new PlatformWalletInfo from mnemonic
 * @param mnemonic BIP39 mnemonic phrase
 * @param passphrase Optional passphrase (can be NULL)
 * @param out_handle Output handle
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult platform_wallet_info_create_from_mnemonic(
    const char* mnemonic,
    const char* passphrase,
    Handle* out_handle,
    PlatformWalletFFIError* out_error
);

/**
 * Get the identity manager for a specific network
 * @param wallet_handle Wallet handle
 * @param network Network type
 * @param out_handle Output handle for identity manager
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult platform_wallet_info_get_identity_manager(
    Handle wallet_handle,
    NetworkType network,
    Handle* out_handle,
    PlatformWalletFFIError* out_error
);

/**
 * Set identity manager for a network
 * @param wallet_handle Wallet handle
 * @param network Network type
 * @param manager_handle Identity manager handle
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult platform_wallet_info_set_identity_manager(
    Handle wallet_handle,
    NetworkType network,
    Handle manager_handle,
    PlatformWalletFFIError* out_error
);

/**
 * Serialize PlatformWalletInfo to JSON
 * @param wallet_handle Wallet handle
 * @param out_json Output JSON string (caller must free with platform_wallet_string_free)
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult platform_wallet_info_to_json(
    Handle wallet_handle,
    char** out_json,
    PlatformWalletFFIError* out_error
);

/**
 * Destroy PlatformWalletInfo and free resources
 * @param wallet_handle Wallet handle
 * @return Result code
 */
PlatformWalletFFIResult platform_wallet_info_destroy(Handle wallet_handle);

/* ============================================================================
 * IdentityManager Functions
 * ========================================================================== */

/**
 * Create a new empty IdentityManager
 * @param out_handle Output handle
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult identity_manager_create(
    Handle* out_handle,
    PlatformWalletFFIError* out_error
);

/**
 * Add a managed identity to the manager
 * @param manager_handle Manager handle
 * @param identity_handle Identity handle
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult identity_manager_add_identity(
    Handle manager_handle,
    Handle identity_handle,
    PlatformWalletFFIError* out_error
);

/**
 * Remove an identity from the manager
 * @param manager_handle Manager handle
 * @param identity_id Identity ID to remove
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult identity_manager_remove_identity(
    Handle manager_handle,
    IdentifierBytes identity_id,
    PlatformWalletFFIError* out_error
);

/**
 * Get an identity by ID
 * @param manager_handle Manager handle
 * @param identity_id Identity ID
 * @param out_handle Output handle for identity
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult identity_manager_get_identity(
    Handle manager_handle,
    IdentifierBytes identity_id,
    Handle* out_handle,
    PlatformWalletFFIError* out_error
);

/**
 * Get all identity IDs
 * @param manager_handle Manager handle
 * @param out_array Output array (caller must free with platform_wallet_identifier_array_free)
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult identity_manager_get_all_identity_ids(
    Handle manager_handle,
    IdentifierArray* out_array,
    PlatformWalletFFIError* out_error
);

/**
 * Get the primary identity ID
 * @param manager_handle Manager handle
 * @param out_id Output identifier
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult identity_manager_get_primary_identity_id(
    Handle manager_handle,
    IdentifierBytes* out_id,
    PlatformWalletFFIError* out_error
);

/**
 * Set the primary identity
 * @param manager_handle Manager handle
 * @param identity_id Identity ID to set as primary
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult identity_manager_set_primary_identity(
    Handle manager_handle,
    IdentifierBytes identity_id,
    PlatformWalletFFIError* out_error
);

/**
 * Get the count of identities
 * @param manager_handle Manager handle
 * @param out_count Output count
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult identity_manager_get_identity_count(
    Handle manager_handle,
    size_t* out_count,
    PlatformWalletFFIError* out_error
);

/**
 * Destroy IdentityManager and free resources
 * @param manager_handle Manager handle
 * @return Result code
 */
PlatformWalletFFIResult identity_manager_destroy(Handle manager_handle);

/* ============================================================================
 * ManagedIdentity Functions
 * ========================================================================== */

/**
 * Create a new ManagedIdentity from DPP Identity bytes
 * @param identity_bytes Serialized identity bytes
 * @param identity_len Length of identity bytes
 * @param out_handle Output handle
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult managed_identity_create_from_identity_bytes(
    const uint8_t* identity_bytes,
    size_t identity_len,
    Handle* out_handle,
    PlatformWalletFFIError* out_error
);

/**
 * Get the identity ID
 * @param identity_handle Identity handle
 * @param out_id Output identifier
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult managed_identity_get_id(
    Handle identity_handle,
    IdentifierBytes* out_id,
    PlatformWalletFFIError* out_error
);

/**
 * Get the identity balance
 * @param identity_handle Identity handle
 * @param out_balance Output balance
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult managed_identity_get_balance(
    Handle identity_handle,
    uint64_t* out_balance,
    PlatformWalletFFIError* out_error
);

/**
 * Get the label
 * @param identity_handle Identity handle
 * @param out_label Output label (caller must free with platform_wallet_string_free, NULL if no label)
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult managed_identity_get_label(
    Handle identity_handle,
    char** out_label,
    PlatformWalletFFIError* out_error
);

/**
 * Set the label
 * @param identity_handle Identity handle
 * @param label Label string (NULL to clear)
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult managed_identity_set_label(
    Handle identity_handle,
    const char* label,
    PlatformWalletFFIError* out_error
);

/**
 * Get last updated balance block time
 * @param identity_handle Identity handle
 * @param out_block_time Output block time (zeroed if not set)
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult managed_identity_get_last_updated_balance_block_time(
    Handle identity_handle,
    BlockTime* out_block_time,
    PlatformWalletFFIError* out_error
);

/**
 * Set last updated balance block time
 * @param identity_handle Identity handle
 * @param block_time Block time to set
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult managed_identity_set_last_updated_balance_block_time(
    Handle identity_handle,
    BlockTime block_time,
    PlatformWalletFFIError* out_error
);

/**
 * Get last synced keys block time
 * @param identity_handle Identity handle
 * @param out_block_time Output block time (zeroed if not set)
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult managed_identity_get_last_synced_keys_block_time(
    Handle identity_handle,
    BlockTime* out_block_time,
    PlatformWalletFFIError* out_error
);

/**
 * Serialize ManagedIdentity to JSON
 * @param identity_handle Identity handle
 * @param out_json Output JSON string (caller must free with platform_wallet_string_free)
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult managed_identity_to_json(
    Handle identity_handle,
    char** out_json,
    PlatformWalletFFIError* out_error
);

/**
 * Destroy ManagedIdentity and free resources
 * @param identity_handle Identity handle
 * @return Result code
 */
PlatformWalletFFIResult managed_identity_destroy(Handle identity_handle);

/* ============================================================================
 * Contact Management Functions
 * ========================================================================== */

/**
 * Add a sent contact request
 * @param identity_handle Identity handle
 * @param contact_id Contact identity ID
 * @param label Optional label (can be NULL)
 * @param timestamp Request timestamp
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult managed_identity_add_sent_contact_request(
    Handle identity_handle,
    IdentifierBytes contact_id,
    const char* label,
    uint64_t timestamp,
    PlatformWalletFFIError* out_error
);

/**
 * Add an incoming contact request
 * @param identity_handle Identity handle
 * @param contact_id Contact identity ID
 * @param label Optional label (can be NULL)
 * @param timestamp Request timestamp
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult managed_identity_add_incoming_contact_request(
    Handle identity_handle,
    IdentifierBytes contact_id,
    const char* label,
    uint64_t timestamp,
    PlatformWalletFFIError* out_error
);

/**
 * Remove a sent contact request
 * @param identity_handle Identity handle
 * @param contact_id Contact identity ID
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult managed_identity_remove_sent_contact_request(
    Handle identity_handle,
    IdentifierBytes contact_id,
    PlatformWalletFFIError* out_error
);

/**
 * Remove an incoming contact request
 * @param identity_handle Identity handle
 * @param contact_id Contact identity ID
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult managed_identity_remove_incoming_contact_request(
    Handle identity_handle,
    IdentifierBytes contact_id,
    PlatformWalletFFIError* out_error
);

/**
 * Get all sent contact request IDs
 * @param identity_handle Identity handle
 * @param out_array Output array (caller must free with platform_wallet_identifier_array_free)
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult managed_identity_get_sent_contact_request_ids(
    Handle identity_handle,
    IdentifierArray* out_array,
    PlatformWalletFFIError* out_error
);

/**
 * Get all incoming contact request IDs
 * @param identity_handle Identity handle
 * @param out_array Output array (caller must free with platform_wallet_identifier_array_free)
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult managed_identity_get_incoming_contact_request_ids(
    Handle identity_handle,
    IdentifierArray* out_array,
    PlatformWalletFFIError* out_error
);

/**
 * Get all established contact IDs
 * @param identity_handle Identity handle
 * @param out_array Output array (caller must free with platform_wallet_identifier_array_free)
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult managed_identity_get_established_contact_ids(
    Handle identity_handle,
    IdentifierArray* out_array,
    PlatformWalletFFIError* out_error
);

/**
 * Check if a contact is established
 * @param identity_handle Identity handle
 * @param contact_id Contact identity ID
 * @param out_is_established Output boolean
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult managed_identity_is_contact_established(
    Handle identity_handle,
    IdentifierBytes contact_id,
    bool* out_is_established,
    PlatformWalletFFIError* out_error
);

/**
 * Remove an established contact
 * @param identity_handle Identity handle
 * @param contact_id Contact identity ID
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult managed_identity_remove_established_contact(
    Handle identity_handle,
    IdentifierBytes contact_id,
    PlatformWalletFFIError* out_error
);

/* ============================================================================
 * Utility Functions
 * ========================================================================== */

/**
 * Serialize JSON string to bytes
 * @param json_string JSON string
 * @param out_bytes Output bytes (caller must free with platform_wallet_bytes_free)
 * @param out_len Output length
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult platform_wallet_serialize_to_json_bytes(
    const char* json_string,
    uint8_t** out_bytes,
    size_t* out_len,
    PlatformWalletFFIError* out_error
);

/**
 * Deserialize JSON bytes to string
 * @param bytes Input bytes
 * @param len Input length
 * @param out_json_string Output JSON string (caller must free with platform_wallet_string_free)
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult platform_wallet_deserialize_from_json_bytes(
    const uint8_t* bytes,
    size_t len,
    char** out_json_string,
    PlatformWalletFFIError* out_error
);

/**
 * Generate random identifier
 * @param out_id Output identifier
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult platform_wallet_generate_random_identifier(
    IdentifierBytes* out_id,
    PlatformWalletFFIError* out_error
);

/**
 * Convert identifier to hex string
 * @param id Identifier
 * @param out_hex Output hex string (caller must free with platform_wallet_string_free)
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult platform_wallet_identifier_to_hex(
    IdentifierBytes id,
    char** out_hex,
    PlatformWalletFFIError* out_error
);

/**
 * Convert hex string to identifier
 * @param hex Hex string
 * @param out_id Output identifier
 * @param out_error Optional error output
 * @return Result code
 */
PlatformWalletFFIResult platform_wallet_identifier_from_hex(
    const char* hex,
    IdentifierBytes* out_id,
    PlatformWalletFFIError* out_error
);

/**
 * Free identifier array
 * @param array Array to free
 */
void platform_wallet_identifier_array_free(IdentifierArray array);

/**
 * Free a C string
 * @param s String to free
 */
void platform_wallet_string_free(char* s);

/**
 * Free bytes allocated by FFI functions
 * @param bytes Bytes to free
 * @param len Length of bytes
 */
void platform_wallet_bytes_free(uint8_t* bytes, size_t len);

#ifdef __cplusplus
}
#endif

#endif /* PLATFORM_WALLET_FFI_H */
