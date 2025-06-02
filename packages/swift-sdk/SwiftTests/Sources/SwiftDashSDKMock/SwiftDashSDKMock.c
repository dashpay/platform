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
static int g_signer_count = 0;

// Mock implementations

void swift_dash_sdk_init(void) {
    g_initialized = 1;
}

SDKHandle *swift_dash_sdk_create(SwiftDashSDKConfig config) {
    if (!g_initialized) return NULL;
    
    // Simulate failure for invalid configs
    if (config.request_timeout_ms == 0) return NULL;
    
    g_sdk_count++;
    // Return a non-null dummy pointer
    return (SDKHandle *)((uintptr_t)0x1000 + g_sdk_count);
}

void swift_dash_sdk_destroy(SDKHandle *handle) {
    if (handle != NULL) {
        g_sdk_count--;
    }
}

SwiftDashNetwork swift_dash_sdk_get_network(SDKHandle *handle) {
    if (handle == NULL) {
        return SwiftDashNetwork_Testnet; // Default
    }
    // Mock: return based on handle value
    uintptr_t value = (uintptr_t)handle;
    return (SwiftDashNetwork)(value % 4);
}

char *swift_dash_sdk_get_version(void) {
    return strdup("2.0.0-mock");
}

SwiftDashSDKConfig swift_dash_sdk_config_mainnet(void) {
    SwiftDashSDKConfig config = {
        .network = SwiftDashNetwork_Mainnet,
        .skip_asset_lock_proof_verification = false,
        .request_retry_count = 3,
        .request_timeout_ms = 30000
    };
    return config;
}

SwiftDashSDKConfig swift_dash_sdk_config_testnet(void) {
    SwiftDashSDKConfig config = {
        .network = SwiftDashNetwork_Testnet,
        .skip_asset_lock_proof_verification = false,
        .request_retry_count = 3,
        .request_timeout_ms = 30000
    };
    return config;
}

SwiftDashSDKConfig swift_dash_sdk_config_local(void) {
    SwiftDashSDKConfig config = {
        .network = SwiftDashNetwork_Local,
        .skip_asset_lock_proof_verification = true,
        .request_retry_count = 1,
        .request_timeout_ms = 10000
    };
    return config;
}

SwiftDashPutSettings swift_dash_put_settings_default(void) {
    SwiftDashPutSettings settings = {
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
IdentityHandle *swift_dash_identity_fetch(SDKHandle *sdk_handle, const char *identity_id) {
    if (sdk_handle == NULL || identity_id == NULL) return NULL;
    
    // Mock: return handle based on ID
    if (strcmp(identity_id, "test_identity_123") == 0) {
        return (IdentityHandle *)0x2000;
    }
    return NULL;
}

SwiftDashIdentityInfo *swift_dash_identity_get_info(IdentityHandle *identity_handle) {
    if (identity_handle == NULL) return NULL;
    
    SwiftDashIdentityInfo *info = malloc(sizeof(SwiftDashIdentityInfo));
    info->id = strdup("test_identity_123");
    info->balance = 1000000;
    info->revision = 1;
    info->public_keys_count = 2;
    
    return info;
}

SwiftDashBinaryData *swift_dash_identity_put_to_platform_with_instant_lock(
    SDKHandle *sdk_handle,
    IdentityHandle *identity_handle,
    uint32_t public_key_id,
    SignerHandle *signer_handle,
    const SwiftDashPutSettings *settings
) {
    if (sdk_handle == NULL || identity_handle == NULL || signer_handle == NULL) {
        return NULL;
    }
    
    // Mock state transition data
    SwiftDashBinaryData *data = malloc(sizeof(SwiftDashBinaryData));
    data->len = 64;
    data->data = malloc(data->len);
    memset(data->data, 0xAB, data->len); // Fill with dummy data
    
    return data;
}

IdentityHandle *swift_dash_identity_put_to_platform_with_instant_lock_and_wait(
    SDKHandle *sdk_handle,
    IdentityHandle *identity_handle,
    uint32_t public_key_id,
    SignerHandle *signer_handle,
    const SwiftDashPutSettings *settings
) {
    if (sdk_handle == NULL || identity_handle == NULL || signer_handle == NULL) {
        return NULL;
    }
    
    // Return the same handle (simulating confirmed identity)
    return identity_handle;
}

SwiftDashTransferCreditsResult *swift_dash_identity_transfer_credits(
    SDKHandle *sdk_handle,
    IdentityHandle *identity_handle,
    const char *recipient_id,
    uint64_t amount,
    uint32_t public_key_id,
    SignerHandle *signer_handle,
    const SwiftDashPutSettings *settings
) {
    if (sdk_handle == NULL || identity_handle == NULL || recipient_id == NULL || signer_handle == NULL) {
        return NULL;
    }
    
    SwiftDashTransferCreditsResult *result = malloc(sizeof(SwiftDashTransferCreditsResult));
    result->amount = amount;
    result->recipient_id = strdup(recipient_id);
    result->transaction_data_len = 32;
    result->transaction_data = malloc(result->transaction_data_len);
    memset(result->transaction_data, 0xFF, result->transaction_data_len);
    
    return result;
}

// Data contract functions
DataContractHandle *swift_dash_data_contract_fetch(SDKHandle *sdk_handle, const char *contract_id) {
    if (sdk_handle == NULL || contract_id == NULL) return NULL;
    
    if (strcmp(contract_id, "test_contract_456") == 0) {
        return (DataContractHandle *)0x3000;
    }
    return NULL;
}

DataContractHandle *swift_dash_data_contract_create(
    SDKHandle *sdk_handle,
    const char *owner_identity_id,
    const char *schema_json
) {
    if (sdk_handle == NULL || owner_identity_id == NULL || schema_json == NULL) {
        return NULL;
    }
    
    return (DataContractHandle *)0x3001;
}

char *swift_dash_data_contract_get_info(DataContractHandle *contract_handle) {
    if (contract_handle == NULL) return NULL;
    
    return strdup("{\"id\":\"test_contract_456\",\"version\":1}");
}

SwiftDashBinaryData *swift_dash_data_contract_put_to_platform(
    SDKHandle *sdk_handle,
    DataContractHandle *contract_handle,
    uint32_t public_key_id,
    SignerHandle *signer_handle,
    const SwiftDashPutSettings *settings
) {
    if (sdk_handle == NULL || contract_handle == NULL || signer_handle == NULL) {
        return NULL;
    }
    
    SwiftDashBinaryData *data = malloc(sizeof(SwiftDashBinaryData));
    data->len = 128;
    data->data = malloc(data->len);
    memset(data->data, 0xCC, data->len);
    
    return data;
}

// Document functions
DocumentHandle *swift_dash_document_create(
    SDKHandle *sdk_handle,
    DataContractHandle *contract_handle,
    const char *owner_identity_id,
    const char *document_type,
    const char *data_json
) {
    if (sdk_handle == NULL || contract_handle == NULL || 
        owner_identity_id == NULL || document_type == NULL || data_json == NULL) {
        return NULL;
    }
    
    return (DocumentHandle *)0x4000;
}

DocumentHandle *swift_dash_document_fetch(
    SDKHandle *sdk_handle,
    DataContractHandle *contract_handle,
    const char *document_type,
    const char *document_id
) {
    if (sdk_handle == NULL || contract_handle == NULL || 
        document_type == NULL || document_id == NULL) {
        return NULL;
    }
    
    if (strcmp(document_id, "test_doc_789") == 0) {
        return (DocumentHandle *)0x4001;
    }
    return NULL;
}

SwiftDashDocumentInfo *swift_dash_document_get_info(DocumentHandle *document_handle) {
    if (document_handle == NULL) return NULL;
    
    SwiftDashDocumentInfo *info = malloc(sizeof(SwiftDashDocumentInfo));
    info->id = strdup("test_doc_789");
    info->owner_id = strdup("test_identity_123");
    info->data_contract_id = strdup("test_contract_456");
    info->document_type = strdup("message");
    info->revision = 1;
    info->created_at = 1640000000000;
    info->updated_at = 1640000001000;
    
    return info;
}

SwiftDashBinaryData *swift_dash_document_put_to_platform(
    SDKHandle *sdk_handle,
    DocumentHandle *document_handle,
    uint32_t public_key_id,
    SignerHandle *signer_handle,
    const SwiftDashPutSettings *settings
) {
    if (sdk_handle == NULL || document_handle == NULL || signer_handle == NULL) {
        return NULL;
    }
    
    SwiftDashBinaryData *data = malloc(sizeof(SwiftDashBinaryData));
    data->len = 256;
    data->data = malloc(data->len);
    memset(data->data, 0xDD, data->len);
    
    return data;
}

// Signer functions
SignerHandle *swift_dash_signer_create_test(void) {
    g_signer_count++;
    return (SignerHandle *)((uintptr_t)0x5000 + g_signer_count);
}

void swift_dash_signer_destroy(SignerHandle *handle) {
    if (handle != NULL) {
        g_signer_count--;
    }
}

// Memory management
void swift_dash_error_free(SwiftDashError *error) {
    if (error != NULL) {
        if (error->message != NULL) {
            free(error->message);
        }
        free(error);
    }
}

void swift_dash_identity_info_free(SwiftDashIdentityInfo *info) {
    if (info != NULL) {
        if (info->id != NULL) free(info->id);
        free(info);
    }
}

void swift_dash_document_info_free(SwiftDashDocumentInfo *info) {
    if (info != NULL) {
        if (info->id != NULL) free(info->id);
        if (info->owner_id != NULL) free(info->owner_id);
        if (info->data_contract_id != NULL) free(info->data_contract_id);
        if (info->document_type != NULL) free(info->document_type);
        free(info);
    }
}

void swift_dash_binary_data_free(SwiftDashBinaryData *data) {
    if (data != NULL) {
        if (data->data != NULL) free(data->data);
        free(data);
    }
}

void swift_dash_transfer_credits_result_free(SwiftDashTransferCreditsResult *result) {
    if (result != NULL) {
        if (result->recipient_id != NULL) free(result->recipient_id);
        if (result->transaction_data != NULL) free(result->transaction_data);
        free(result);
    }
}