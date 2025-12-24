// Mock implementation of Swift Dash SDK for testing
// This provides mock implementations of all the C functions

#include "SwiftDashSDK.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <stdint.h>

// Global state for testing
static int g_initialized = 0;
static int g_sdk_count = 0;

// Test configuration data
static const char* g_existing_identity_id = "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF";
static const char* g_existing_data_contract_id = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";

// Error helper
static struct SwiftDashSwiftDashError* create_error(enum SwiftDashSwiftDashErrorCode code, const char* message) {
    struct SwiftDashSwiftDashError* error = malloc(sizeof(struct SwiftDashSwiftDashError));
    error->code = code;
    error->message = strdup(message);
    return error;
}

// Result helpers
static struct SwiftDashSwiftDashResult success_result(void* data) {
    struct SwiftDashSwiftDashResult result = {
        .success = true,
        .data = data,
        .error = NULL
    };
    return result;
}

static struct SwiftDashSwiftDashResult error_result(enum SwiftDashSwiftDashErrorCode code, const char* message) {
    struct SwiftDashSwiftDashResult result = {
        .success = false,
        .data = NULL,
        .error = create_error(code, message)
    };
    return result;
}

// Mock implementations

void swift_dash_sdk_init(void) {
    g_initialized = 1;
}

const char *swift_dash_sdk_version(void) {
    return "2.0.0-mock";
}

struct SwiftDashSDKHandle *swift_dash_sdk_create(struct SwiftDashSwiftDashSDKConfig config) {
    if (!g_initialized) return NULL;
    
    g_sdk_count++;
    // Return a non-null dummy pointer
    return (struct SwiftDashSDKHandle *)((uintptr_t)0x1000 + g_sdk_count);
}

void swift_dash_sdk_destroy(struct SwiftDashSDKHandle *handle) {
    if (handle != NULL) {
        g_sdk_count--;
    }
}

enum SwiftDashSwiftDashNetwork swift_dash_sdk_get_network(const struct SwiftDashSDKHandle *handle) {
    if (handle == NULL) {
        return Testnet; // Default
    }
    // Mock: return testnet for simplicity
    return Testnet;
}

const char *swift_dash_sdk_get_version(void) {
    return "2.0.0-mock";
}

struct SwiftDashSwiftDashSDKConfig swift_dash_sdk_config_mainnet(void) {
    struct SwiftDashSwiftDashSDKConfig config = {
        .network = Mainnet,
        .dapi_addresses = "mainnet-seeds.dash.org:443"
    };
    return config;
}

struct SwiftDashSwiftDashSDKConfig swift_dash_sdk_config_testnet(void) {
    struct SwiftDashSwiftDashSDKConfig config = {
        .network = Testnet,
        .dapi_addresses = "testnet-seeds.dash.org:443"
    };
    return config;
}

struct SwiftDashSwiftDashSDKConfig swift_dash_sdk_config_local(void) {
    struct SwiftDashSwiftDashSDKConfig config = {
        .network = Local,
        .dapi_addresses = "127.0.0.1:3000"
    };
    return config;
}

struct SwiftDashSwiftDashPutSettings swift_dash_put_settings_default(void) {
    struct SwiftDashSwiftDashPutSettings settings = {
        .connect_timeout_ms = 0,
        .timeout_ms = 0,
        .retries = 0,
        .ban_failed_address = false,
        .identity_nonce_stale_time_s = 0,
        .user_fee_increase = 0,
        .allow_signing_with_any_security_level = false,
        .allow_signing_with_any_purpose = false,
        .wait_timeout_ms = 0
    };
    return settings;
}

// Identity functions
char *swift_dash_identity_fetch(const struct SwiftDashSDKHandle *sdk_handle, const char *identity_id) {
    if (sdk_handle == NULL || identity_id == NULL) return NULL;
    
    // Return null for non-existent identities
    if (strcmp(identity_id, "1111111111111111111111111111111111111111111") == 0) {
        return NULL;
    }
    
    // Return mock identity JSON for known identity
    if (strcmp(identity_id, g_existing_identity_id) == 0) {
        const char* json = "{\"id\":\"4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF\",\"publicKeys\":[{\"id\":0,\"type\":0,\"purpose\":0,\"securityLevel\":2,\"data\":\"test_key\"}]}";
        return strdup(json);
    }
    
    return NULL;
}

uint64_t swift_dash_identity_get_balance(const struct SwiftDashSDKHandle *sdk_handle, const char *identity_id) {
    if (sdk_handle == NULL || identity_id == NULL) return 0;
    
    if (strcmp(identity_id, g_existing_identity_id) == 0) {
        return 1000000; // Mock balance
    }
    
    return 0;
}

char *swift_dash_identity_resolve_name(const struct SwiftDashSDKHandle *sdk_handle, const char *name) {
    if (sdk_handle == NULL || name == NULL) return NULL;
    
    if (strcmp(name, "dash") == 0) {
        const char* json = "{\"identity\":\"4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF\",\"alias\":\"dash\"}";
        return strdup(json);
    }
    
    return NULL;
}

struct SwiftDashSwiftDashResult swift_dash_identity_transfer_credits(const struct SwiftDashSDKHandle *sdk_handle,
                                                                     const char *from_identity_id,
                                                                     const char *to_identity_id,
                                                                     uint64_t amount,
                                                                     const uint8_t *private_key,
                                                                     size_t private_key_len) {
    if (sdk_handle == NULL || from_identity_id == NULL || to_identity_id == NULL || private_key == NULL) {
        return error_result(InvalidParameter, "Missing required parameters");
    }
    
    return error_result(NotImplemented, "Credit transfer not yet implemented");
}

struct SwiftDashSwiftDashResult swift_dash_identity_create(const struct SwiftDashSDKHandle *sdk_handle,
                                                           const uint8_t *public_key,
                                                           size_t public_key_len) {
    if (sdk_handle == NULL || public_key == NULL) {
        return error_result(InvalidParameter, "Missing required parameters");
    }
    
    return error_result(NotImplemented, "Identity creation not yet implemented");
}

// Data contract functions
char *swift_dash_data_contract_fetch(const struct SwiftDashSDKHandle *sdk_handle, const char *contract_id) {
    if (sdk_handle == NULL || contract_id == NULL) return NULL;
    
    // Return null for non-existent contracts
    if (strcmp(contract_id, "1111111111111111111111111111111111111111111") == 0) {
        return NULL;
    }
    
    // Return mock contract JSON for known contract
    if (strcmp(contract_id, g_existing_data_contract_id) == 0) {
        const char* json = "{\"id\":\"GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec\",\"version\":1,\"documents\":{\"domain\":{\"type\":\"object\"}}}";
        return strdup(json);
    }
    
    return NULL;
}

char *swift_dash_data_contract_get_history(const struct SwiftDashSDKHandle *sdk_handle,
                                           const char *contract_id,
                                           uint32_t limit,
                                           uint32_t offset) {
    if (sdk_handle == NULL || contract_id == NULL) return NULL;
    
    if (strcmp(contract_id, g_existing_data_contract_id) == 0) {
        const char* json = "{\"contract_id\":\"GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec\",\"history\":[]}";
        return strdup(json);
    }
    
    return NULL;
}

struct SwiftDashSwiftDashResult swift_dash_data_contract_create(const struct SwiftDashSDKHandle *sdk_handle,
                                                                const char *schema_json,
                                                                const char *owner_id) {
    if (sdk_handle == NULL || schema_json == NULL || owner_id == NULL) {
        return error_result(InvalidParameter, "Missing required parameters");
    }
    
    return error_result(NotImplemented, "Data contract creation not yet implemented");
}

struct SwiftDashSwiftDashResult swift_dash_data_contract_update(const struct SwiftDashSDKHandle *sdk_handle,
                                                                const char *contract_id,
                                                                const char *schema_json,
                                                                uint32_t version) {
    if (sdk_handle == NULL || contract_id == NULL || schema_json == NULL) {
        return error_result(InvalidParameter, "Missing required parameters");
    }
    
    return error_result(NotImplemented, "Data contract update not yet implemented");
}

// Document functions
char *swift_dash_document_fetch(const struct SwiftDashSDKHandle *sdk_handle,
                                const char *data_contract_id,
                                const char *document_type,
                                const char *document_id) {
    if (sdk_handle == NULL || data_contract_id == NULL || document_type == NULL || document_id == NULL) {
        return NULL;
    }
    
    return NULL; // Document fetching not implemented in mock
}

char *swift_dash_document_search(const struct SwiftDashSDKHandle *sdk_handle,
                                 const char *data_contract_id,
                                 const char *document_type,
                                 const char *query_json,
                                 uint32_t limit) {
    if (sdk_handle == NULL || data_contract_id == NULL || document_type == NULL) {
        return NULL;
    }
    
    return NULL; // Document search not implemented in mock
}

struct SwiftDashSwiftDashResult swift_dash_document_create(const struct SwiftDashSDKHandle *sdk_handle,
                                                           const char *data_contract_id,
                                                           const char *document_type,
                                                           const char *properties_json,
                                                           const char *identity_id) {
    if (sdk_handle == NULL || data_contract_id == NULL || document_type == NULL) {
        return error_result(InvalidParameter, "Missing required parameters");
    }
    
    return error_result(NotImplemented, "Document creation not yet implemented");
}

struct SwiftDashSwiftDashResult swift_dash_document_update(const struct SwiftDashSDKHandle *sdk_handle,
                                                           const char *document_id,
                                                           const char *properties_json,
                                                           uint64_t revision) {
    if (sdk_handle == NULL || document_id == NULL) {
        return error_result(InvalidParameter, "Missing required parameters");
    }
    
    return error_result(NotImplemented, "Document update not yet implemented");
}

struct SwiftDashSwiftDashResult swift_dash_document_delete(const struct SwiftDashSDKHandle *sdk_handle,
                                                           const char *document_id) {
    if (sdk_handle == NULL || document_id == NULL) {
        return error_result(InvalidParameter, "Missing required parameters");
    }
    
    return error_result(NotImplemented, "Document deletion not yet implemented");
}

// Signer functions
struct SwiftDashSwiftDashSigner *swift_dash_signer_create(SwiftDashSwiftSignCallback sign_callback,
                                                          SwiftDashSwiftCanSignCallback can_sign_callback) {
    if (sign_callback == NULL || can_sign_callback == NULL) return NULL;
    
    struct SwiftDashSwiftDashSigner *signer = malloc(sizeof(struct SwiftDashSwiftDashSigner));
    signer->sign_callback = sign_callback;
    signer->can_sign_callback = can_sign_callback;
    return signer;
}

void swift_dash_signer_free(struct SwiftDashSwiftDashSigner *signer) {
    if (signer != NULL) {
        free(signer);
    }
}

bool swift_dash_signer_can_sign(const struct SwiftDashSwiftDashSigner *signer,
                                const unsigned char *identity_public_key_bytes,
                                size_t identity_public_key_len) {
    if (signer == NULL || identity_public_key_bytes == NULL) return false;
    
    return signer->can_sign_callback(identity_public_key_bytes, identity_public_key_len);
}

unsigned char *swift_dash_signer_sign(const struct SwiftDashSwiftDashSigner *signer,
                                      const unsigned char *identity_public_key_bytes,
                                      size_t identity_public_key_len,
                                      const unsigned char *data,
                                      size_t data_len,
                                      size_t *result_len) {
    if (signer == NULL || identity_public_key_bytes == NULL || data == NULL || result_len == NULL) {
        return NULL;
    }
    
    return signer->sign_callback(identity_public_key_bytes, identity_public_key_len, data, data_len, result_len);
}

// Token functions
char *swift_dash_token_get_total_supply(const struct SwiftDashSDKHandle *sdk_handle, const char *token_contract_id) {
    if (sdk_handle == NULL || token_contract_id == NULL) return NULL;
    
    // Mock token supply
    return strdup("1000000000");
}

struct SwiftDashSwiftDashResult swift_dash_token_transfer(const struct SwiftDashSDKHandle *sdk_handle,
                                                          const char *token_contract_id,
                                                          const char *from_identity_id,
                                                          const char *to_identity_id,
                                                          uint64_t amount) {
    if (sdk_handle == NULL || token_contract_id == NULL || from_identity_id == NULL || to_identity_id == NULL) {
        return error_result(InvalidParameter, "Missing required parameters");
    }
    
    return error_result(NotImplemented, "Token transfer not yet implemented");
}

struct SwiftDashSwiftDashResult swift_dash_token_mint(const struct SwiftDashSDKHandle *sdk_handle,
                                                      const char *token_contract_id,
                                                      const char *to_identity_id,
                                                      uint64_t amount) {
    if (sdk_handle == NULL || token_contract_id == NULL || to_identity_id == NULL) {
        return error_result(InvalidParameter, "Missing required parameters");
    }
    
    return error_result(NotImplemented, "Token minting not yet implemented");
}

struct SwiftDashSwiftDashResult swift_dash_token_burn(const struct SwiftDashSDKHandle *sdk_handle,
                                                      const char *token_contract_id,
                                                      const char *from_identity_id,
                                                      uint64_t amount) {
    if (sdk_handle == NULL || token_contract_id == NULL || from_identity_id == NULL) {
        return error_result(InvalidParameter, "Missing required parameters");
    }
    
    return error_result(NotImplemented, "Token burning not yet implemented");
}

// Memory management
void swift_dash_error_free(struct SwiftDashSwiftDashError *error) {
    if (error != NULL) {
        if (error->message != NULL) {
            free(error->message);
        }
        free(error);
    }
}

void swift_dash_string_free(char *s) {
    if (s != NULL) {
        free(s);
    }
}

void swift_dash_bytes_free(uint8_t *bytes, size_t len) {
    if (bytes != NULL) {
        free(bytes);
    }
}

void swift_dash_identity_info_free(struct SwiftDashSwiftDashIdentityInfo *info) {
    if (info != NULL) {
        if (info->id != NULL) free(info->id);
        free(info);
    }
}

void swift_dash_document_info_free(struct SwiftDashSwiftDashDocumentInfo *info) {
    if (info != NULL) {
        if (info->id != NULL) free(info->id);
        if (info->owner_id != NULL) free(info->owner_id);
        if (info->data_contract_id != NULL) free(info->data_contract_id);
        if (info->document_type != NULL) free(info->document_type);
        free(info);
    }
}

void swift_dash_data_contract_info_free(struct SwiftDashSwiftDashDataContractInfo *info) {
    if (info != NULL) {
        if (info->id != NULL) free(info->id);
        if (info->owner_id != NULL) free(info->owner_id);
        if (info->schema_json != NULL) free(info->schema_json);
        free(info);
    }
}

void swift_dash_binary_data_free(struct SwiftDashSwiftDashBinaryData *data) {
    if (data != NULL) {
        if (data->data != NULL) free(data->data);
        free(data);
    }
}

void swift_dash_transfer_credits_result_free(struct SwiftDashSwiftDashTransferCreditsResult *result) {
    if (result != NULL) {
        if (result->recipient_id != NULL) free(result->recipient_id);
        if (result->transaction_data != NULL) free(result->transaction_data);
        free(result);
    }
}

void swift_dash_token_info_free(struct SwiftDashSwiftDashTokenInfo *info) {
    if (info != NULL) {
        if (info->contract_id != NULL) free(info->contract_id);
        if (info->name != NULL) free(info->name);
        if (info->symbol != NULL) free(info->symbol);
        free(info);
    }
}