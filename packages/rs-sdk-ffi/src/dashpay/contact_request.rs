//! DashPay contact request operations

use crate::{
    signer::VTableSigner, utils, DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError,
    SDKHandle, SDKWrapper,
};
use dash_sdk::dpp::dashcore::secp256k1::{PublicKey, SecretKey};
use dash_sdk::dpp::identity::{Identity, IdentityPublicKey};
use dash_sdk::platform::dashpay::{
    ContactRequestInput, ContactRequestResult, EcdhProvider, RecipientIdentity,
    SendContactRequestInput, SendContactRequestResult,
};
use dash_sdk::{Error, Sdk};
use std::ffi::CStr;
use std::sync::Arc;

// Helper functions to work around Rust type inference limitations with complex generic enums

async fn create_contact_request_with_shared_secret(
    sdk: &Sdk,
    input: ContactRequestInput,
    shared_secret: [u8; 32],
    extended_public_key: Vec<u8>,
) -> Result<ContactRequestResult, Error> {
    // Use turbofish to help with type inference - specify dummy types for unused F/Fut
    type DummyF = fn(
        &IdentityPublicKey,
        u32,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<SecretKey, Error>> + Send>,
    >;
    type DummyFut =
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<SecretKey, Error>> + Send>>;

    sdk.create_contact_request::<DummyF, DummyFut, _, _, _, _>(
        input,
        EcdhProvider::ClientSide {
            get_shared_secret: move |_public_key: &PublicKey| async move { Ok(shared_secret) },
        },
        move |_account_ref| async move { Ok(extended_public_key.clone()) },
    )
    .await
}

async fn create_contact_request_with_private_key(
    sdk: &Sdk,
    input: ContactRequestInput,
    private_key: SecretKey,
    extended_public_key: Vec<u8>,
) -> Result<ContactRequestResult, Error> {
    // Use turbofish to help with type inference - specify dummy types for unused G/Gut
    type DummyG = fn(
        &PublicKey,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<[u8; 32], Error>> + Send>,
    >;
    type DummyGut =
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<[u8; 32], Error>> + Send>>;

    sdk.create_contact_request::<_, _, DummyG, DummyGut, _, _>(
        input,
        EcdhProvider::SdkSide {
            get_private_key: move |_key: &IdentityPublicKey, _index: u32| async move {
                Ok(private_key)
            },
        },
        move |_account_ref| async move { Ok(extended_public_key.clone()) },
    )
    .await
}

async fn send_contact_request_with_shared_secret<S: dash_sdk::dpp::identity::signer::Signer>(
    sdk: &Sdk,
    send_input: SendContactRequestInput<S>,
    shared_secret: [u8; 32],
    extended_public_key: Vec<u8>,
) -> Result<SendContactRequestResult, Error> {
    // Use turbofish to help with type inference - specify dummy types for unused F/Fut
    type DummyF = fn(
        &IdentityPublicKey,
        u32,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<SecretKey, Error>> + Send>,
    >;
    type DummyFut =
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<SecretKey, Error>> + Send>>;

    sdk.send_contact_request::<S, DummyF, DummyFut, _, _, _, _>(
        send_input,
        EcdhProvider::ClientSide {
            get_shared_secret: move |_public_key: &PublicKey| async move { Ok(shared_secret) },
        },
        move |_account_ref| async move { Ok(extended_public_key.clone()) },
    )
    .await
}

async fn send_contact_request_with_private_key<S: dash_sdk::dpp::identity::signer::Signer>(
    sdk: &Sdk,
    send_input: SendContactRequestInput<S>,
    private_key: SecretKey,
    extended_public_key: Vec<u8>,
) -> Result<SendContactRequestResult, Error> {
    // Use turbofish to help with type inference - specify dummy types for unused G/Gut
    type DummyG = fn(
        &PublicKey,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<[u8; 32], Error>> + Send>,
    >;
    type DummyGut =
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<[u8; 32], Error>> + Send>>;

    sdk.send_contact_request::<S, _, _, DummyG, DummyGut, _, _>(
        send_input,
        EcdhProvider::SdkSide {
            get_private_key: move |_key: &IdentityPublicKey, _index: u32| async move {
                Ok(private_key)
            },
        },
        move |_account_ref| async move { Ok(extended_public_key.clone()) },
    )
    .await
}

/// ECDH mode for contact request encryption
#[repr(C)]
pub enum DashSDKEcdhMode {
    /// Client performs ECDH and provides the shared secret (for hardware wallets)
    ClientSide = 0,
    /// SDK performs ECDH using the provided private key (for software wallets)
    SdkSide = 1,
}

/// Input parameters for creating a contact request
#[repr(C)]
pub struct DashSDKContactRequestParams {
    /// The sender identity handle
    pub sender_identity: *const std::os::raw::c_void,
    /// The recipient identity ID (32 bytes)
    pub recipient_id: *const u8,
    /// Whether to fetch the recipient identity (true) or use provided recipient_identity
    pub fetch_recipient: bool,
    /// The recipient identity handle (if fetch_recipient is false)
    pub recipient_identity: *const std::os::raw::c_void,
    /// The sender's encryption key index
    pub sender_key_index: u32,
    /// The recipient's encryption key index
    pub recipient_key_index: u32,
    /// Reference to the DashPay receiving account
    pub account_reference: u32,
    /// Optional account label (NUL-terminated C string, unencrypted)
    pub account_label: *const std::os::raw::c_char,
    /// Optional auto-accept proof bytes
    pub auto_accept_proof: *const u8,
    /// Length of auto_accept_proof (0 if not provided, must be 38-102 if provided)
    pub auto_accept_proof_len: usize,
    /// ECDH mode (ClientSide or SdkSide)
    pub ecdh_mode: DashSDKEcdhMode,
    /// For SdkSide: the sender's private key (32 bytes)
    /// For ClientSide: ignored (can be null)
    pub sender_private_key: *const u8,
    /// For ClientSide: the shared secret (32 bytes)
    /// For SdkSide: ignored (can be null)
    pub shared_secret: *const u8,
    /// The extended public key to share (unencrypted, typically 78 bytes)
    pub extended_public_key: *const u8,
    /// Length of extended_public_key
    pub extended_public_key_len: usize,
}

/// Result of creating a contact request
#[repr(C)]
pub struct DashSDKContactRequestResult {
    /// Document ID as hex string
    pub document_id: *mut std::os::raw::c_char,
    /// Owner ID (sender ID) as hex string
    pub owner_id: *mut std::os::raw::c_char,
    /// Document properties as JSON string
    pub properties_json: *mut std::os::raw::c_char,
}

/// Result of sending a contact request
#[repr(C)]
pub struct DashSDKSendContactRequestResult {
    /// The created document as JSON string
    pub document_json: *mut std::os::raw::c_char,
    /// Recipient identity ID as hex string
    pub recipient_id: *mut std::os::raw::c_char,
    /// Account reference
    pub account_reference: u32,
}

/// Create a contact request document
///
/// This creates a local contact request document according to DIP-15 specification.
/// The document is not yet submitted to the platform.
///
/// # Safety
/// - `handle` must be a valid SDK handle
/// - All pointer parameters must be valid for their specified types
/// - String parameters must be NUL-terminated
/// - Byte array parameters must have valid lengths
///
/// # Returns
/// Returns a DashSDKContactRequestResult on success
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_dashpay_create_contact_request(
    handle: *const SDKHandle,
    params: *const DashSDKContactRequestParams,
) -> DashSDKResult {
    if handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if params.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Parameters are null".to_string(),
        ));
    }

    let params = &*params;
    let wrapper = &*(handle as *const SDKWrapper);
    let sdk = &wrapper.sdk;

    // Validate required parameters
    if params.sender_identity.is_null() || params.recipient_id.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Sender identity or recipient ID is null".to_string(),
        ));
    }

    if params.extended_public_key.is_null() || params.extended_public_key_len == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Extended public key is null or empty".to_string(),
        ));
    }

    // Get sender identity from handle
    let sender_identity_arc = Arc::from_raw(params.sender_identity as *const Identity);
    let sender_identity = (*sender_identity_arc).clone();
    std::mem::forget(sender_identity_arc);

    // Parse recipient ID
    let recipient_id_bytes = std::slice::from_raw_parts(params.recipient_id, 32);
    let recipient_id = match dash_sdk::dpp::prelude::Identifier::from_bytes(recipient_id_bytes) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid recipient ID: {}", e),
            ));
        }
    };

    // Determine recipient (fetch or use provided)
    let recipient = if params.fetch_recipient {
        RecipientIdentity::Identifier(recipient_id)
    } else {
        if params.recipient_identity.is_null() {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "Recipient identity is null but fetch_recipient is false".to_string(),
            ));
        }
        let recipient_identity_arc = Arc::from_raw(params.recipient_identity as *const Identity);
        let recipient_identity = (*recipient_identity_arc).clone();
        std::mem::forget(recipient_identity_arc);
        RecipientIdentity::Identity(recipient_identity)
    };

    // Parse account label if provided
    let account_label = if !params.account_label.is_null() {
        match CStr::from_ptr(params.account_label).to_str() {
            Ok(s) => Some(s.to_string()),
            Err(e) => {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Invalid UTF-8 in account label: {}", e),
                ));
            }
        }
    } else {
        None
    };

    // Parse auto-accept proof if provided
    let auto_accept_proof =
        if !params.auto_accept_proof.is_null() && params.auto_accept_proof_len > 0 {
            Some(
                std::slice::from_raw_parts(params.auto_accept_proof, params.auto_accept_proof_len)
                    .to_vec(),
            )
        } else {
            None
        };

    // Get extended public key
    let extended_public_key =
        std::slice::from_raw_parts(params.extended_public_key, params.extended_public_key_len)
            .to_vec();

    // Create input
    let input = ContactRequestInput {
        sender_identity,
        recipient,
        sender_key_index: params.sender_key_index,
        recipient_key_index: params.recipient_key_index,
        account_reference: params.account_reference,
        account_label,
        auto_accept_proof,
    };

    // Create ECDH provider and call SDK based on mode
    let result = match params.ecdh_mode {
        DashSDKEcdhMode::ClientSide => {
            // Client provides shared secret
            if params.shared_secret.is_null() {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    "Shared secret is null for ClientSide ECDH mode".to_string(),
                ));
            }

            let shared_secret_bytes = std::slice::from_raw_parts(params.shared_secret, 32);
            let mut shared_secret = [0u8; 32];
            shared_secret.copy_from_slice(shared_secret_bytes);

            wrapper.runtime.block_on(async {
                create_contact_request_with_shared_secret(
                    sdk,
                    input,
                    shared_secret,
                    extended_public_key,
                )
                .await
                .map_err(FFIError::from)
            })
        }
        DashSDKEcdhMode::SdkSide => {
            // SDK performs ECDH with private key
            if params.sender_private_key.is_null() {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    "Sender private key is null for SdkSide ECDH mode".to_string(),
                ));
            }

            let private_key_bytes = std::slice::from_raw_parts(params.sender_private_key, 32);
            let private_key = match SecretKey::from_slice(private_key_bytes) {
                Ok(key) => key,
                Err(e) => {
                    return DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::InvalidParameter,
                        format!("Invalid private key: {}", e),
                    ));
                }
            };

            wrapper.runtime.block_on(async {
                create_contact_request_with_private_key(
                    sdk,
                    input,
                    private_key,
                    extended_public_key,
                )
                .await
                .map_err(FFIError::from)
            })
        }
    };

    match result {
        Ok(contact_request_result) => {
            // Convert document ID to hex string
            let document_id_hex = contact_request_result
                .id
                .to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58);
            let document_id_cstring = match utils::c_string_from(document_id_hex) {
                Ok(s) => s,
                Err(e) => return DashSDKResult::error(e),
            };

            // Convert owner ID to hex string
            let owner_id_hex = contact_request_result
                .owner_id
                .to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58);
            let owner_id_cstring = match utils::c_string_from(owner_id_hex) {
                Ok(s) => s,
                Err(e) => {
                    // Clean up document ID string
                    let _ = std::ffi::CString::from_raw(document_id_cstring);
                    return DashSDKResult::error(e);
                }
            };

            // Convert properties to JSON
            let properties_json = match serde_json::to_string(&contact_request_result.properties) {
                Ok(json) => json,
                Err(e) => {
                    // Clean up previous strings
                    let _ = std::ffi::CString::from_raw(document_id_cstring);
                    let _ = std::ffi::CString::from_raw(owner_id_cstring);
                    return DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::SerializationError,
                        format!("Failed to serialize properties: {}", e),
                    ));
                }
            };

            let properties_cstring = match utils::c_string_from(properties_json) {
                Ok(s) => s,
                Err(e) => {
                    // Clean up previous strings
                    let _ = std::ffi::CString::from_raw(document_id_cstring);
                    let _ = std::ffi::CString::from_raw(owner_id_cstring);
                    return DashSDKResult::error(e);
                }
            };

            // Create result structure
            let result = Box::new(DashSDKContactRequestResult {
                document_id: document_id_cstring,
                owner_id: owner_id_cstring,
                properties_json: properties_cstring,
            });

            DashSDKResult::success(Box::into_raw(result) as *mut std::os::raw::c_void)
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Send a contact request to the platform
///
/// This creates a contact request document and submits it to the platform.
///
/// # Safety
/// - All parameters must be valid
/// - Signer must be valid and not previously freed
///
/// # Returns
/// Returns a DashSDKSendContactRequestResult on success
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_dashpay_send_contact_request(
    handle: *const SDKHandle,
    params: *const DashSDKContactRequestParams,
    identity_public_key: *const std::os::raw::c_void,
    signer: *const std::os::raw::c_void,
) -> DashSDKResult {
    if handle.is_null() || params.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle or parameters are null".to_string(),
        ));
    }

    if identity_public_key.is_null() || signer.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Identity public key or signer is null".to_string(),
        ));
    }

    let params = &*params;
    let wrapper = &*(handle as *const SDKWrapper);
    let sdk = &wrapper.sdk;

    // Validate required parameters (same as create_contact_request)
    if params.sender_identity.is_null() || params.recipient_id.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Sender identity or recipient ID is null".to_string(),
        ));
    }

    if params.extended_public_key.is_null() || params.extended_public_key_len == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Extended public key is null or empty".to_string(),
        ));
    }

    // Get sender identity from handle
    let sender_identity_arc = Arc::from_raw(params.sender_identity as *const Identity);
    let sender_identity = (*sender_identity_arc).clone();
    std::mem::forget(sender_identity_arc);

    // Parse recipient ID
    let recipient_id_bytes = std::slice::from_raw_parts(params.recipient_id, 32);
    let recipient_id = match dash_sdk::dpp::prelude::Identifier::from_bytes(recipient_id_bytes) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid recipient ID: {}", e),
            ));
        }
    };

    // Determine recipient (fetch or use provided)
    let recipient = if params.fetch_recipient {
        RecipientIdentity::Identifier(recipient_id)
    } else {
        if params.recipient_identity.is_null() {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "Recipient identity is null but fetch_recipient is false".to_string(),
            ));
        }
        let recipient_identity_arc = Arc::from_raw(params.recipient_identity as *const Identity);
        let recipient_identity = (*recipient_identity_arc).clone();
        std::mem::forget(recipient_identity_arc);
        RecipientIdentity::Identity(recipient_identity)
    };

    // Parse account label if provided
    let account_label = if !params.account_label.is_null() {
        match CStr::from_ptr(params.account_label).to_str() {
            Ok(s) => Some(s.to_string()),
            Err(e) => {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Invalid UTF-8 in account label: {}", e),
                ));
            }
        }
    } else {
        None
    };

    // Parse auto-accept proof if provided
    let auto_accept_proof =
        if !params.auto_accept_proof.is_null() && params.auto_accept_proof_len > 0 {
            Some(
                std::slice::from_raw_parts(params.auto_accept_proof, params.auto_accept_proof_len)
                    .to_vec(),
            )
        } else {
            None
        };

    // Get extended public key
    let extended_public_key =
        std::slice::from_raw_parts(params.extended_public_key, params.extended_public_key_len)
            .to_vec();

    // Get identity public key from handle
    let key_arc = Arc::from_raw(identity_public_key as *const IdentityPublicKey);
    let key_clone = (*key_arc).clone();
    std::mem::forget(key_arc);

    // Get signer from handle
    let signer_arc = Arc::from_raw(signer as *const VTableSigner);
    let signer_clone = *signer_arc;
    std::mem::forget(signer_arc);

    // Create contact request input
    let contact_request_input = ContactRequestInput {
        sender_identity,
        recipient,
        sender_key_index: params.sender_key_index,
        recipient_key_index: params.recipient_key_index,
        account_reference: params.account_reference,
        account_label,
        auto_accept_proof,
    };

    // Create send input
    let send_input = SendContactRequestInput {
        contact_request: contact_request_input,
        identity_public_key: key_clone,
        signer: signer_clone,
    };

    // Send contact request based on ECDH mode
    let result = match params.ecdh_mode {
        DashSDKEcdhMode::ClientSide => {
            if params.shared_secret.is_null() {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    "Shared secret is null for ClientSide ECDH mode".to_string(),
                ));
            }

            let shared_secret_bytes = std::slice::from_raw_parts(params.shared_secret, 32);
            let mut shared_secret = [0u8; 32];
            shared_secret.copy_from_slice(shared_secret_bytes);

            wrapper.runtime.block_on(async {
                send_contact_request_with_shared_secret(
                    sdk,
                    send_input,
                    shared_secret,
                    extended_public_key,
                )
                .await
                .map_err(FFIError::from)
            })
        }
        DashSDKEcdhMode::SdkSide => {
            if params.sender_private_key.is_null() {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    "Sender private key is null for SdkSide ECDH mode".to_string(),
                ));
            }

            let private_key_bytes = std::slice::from_raw_parts(params.sender_private_key, 32);
            let private_key = match SecretKey::from_slice(private_key_bytes) {
                Ok(key) => key,
                Err(e) => {
                    return DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::InvalidParameter,
                        format!("Invalid private key: {}", e),
                    ));
                }
            };

            wrapper.runtime.block_on(async {
                send_contact_request_with_private_key(
                    sdk,
                    send_input,
                    private_key,
                    extended_public_key,
                )
                .await
                .map_err(FFIError::from)
            })
        }
    };

    match result {
        Ok(send_result) => {
            // Serialize document to JSON
            let document_json = match serde_json::to_string(&send_result.document) {
                Ok(json) => json,
                Err(e) => {
                    return DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::SerializationError,
                        format!("Failed to serialize document: {}", e),
                    ));
                }
            };

            let document_cstring = match utils::c_string_from(document_json) {
                Ok(s) => s,
                Err(e) => return DashSDKResult::error(e),
            };

            // Convert recipient ID to hex string
            let recipient_id_hex = send_result
                .recipient_id
                .to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58);
            let recipient_id_cstring = match utils::c_string_from(recipient_id_hex) {
                Ok(s) => s,
                Err(e) => {
                    // Clean up document string
                    let _ = std::ffi::CString::from_raw(document_cstring);
                    return DashSDKResult::error(e);
                }
            };

            // Create result structure
            let result = Box::new(DashSDKSendContactRequestResult {
                document_json: document_cstring,
                recipient_id: recipient_id_cstring,
                account_reference: send_result.account_reference,
            });

            DashSDKResult::success(Box::into_raw(result) as *mut std::os::raw::c_void)
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Free a contact request result
///
/// # Safety
/// - `result` must be a valid DashSDKContactRequestResult pointer
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_dashpay_contact_request_result_free(
    result: *mut DashSDKContactRequestResult,
) {
    if !result.is_null() {
        let result = Box::from_raw(result);

        if !result.document_id.is_null() {
            let _ = std::ffi::CString::from_raw(result.document_id);
        }
        if !result.owner_id.is_null() {
            let _ = std::ffi::CString::from_raw(result.owner_id);
        }
        if !result.properties_json.is_null() {
            let _ = std::ffi::CString::from_raw(result.properties_json);
        }
    }
}

/// Free a send contact request result
///
/// # Safety
/// - `result` must be a valid DashSDKSendContactRequestResult pointer
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_dashpay_send_contact_request_result_free(
    result: *mut DashSDKSendContactRequestResult,
) {
    if !result.is_null() {
        let result = Box::from_raw(result);

        if !result.document_json.is_null() {
            let _ = std::ffi::CString::from_raw(result.document_json);
        }
        if !result.recipient_id.is_null() {
            let _ = std::ffi::CString::from_raw(result.recipient_id);
        }
    }
}
