use crate::error::WasmSdkError;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::document::{Document, DocumentV0Getters};
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::signer::Signer;
use dash_sdk::dpp::identity::{Purpose, SecurityLevel};
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::dpns_usernames::{
    convert_to_homograph_safe_chars, is_contested_username, is_valid_username,
    RegisterDpnsNameInput,
};
use dash_sdk::platform::{Fetch, Identity};
use serde::{Deserialize, Serialize};
use simple_signer::SingleKeySigner;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterDpnsNameResult {
    pub preorder_document_id: String,
    pub domain_document_id: String,
    pub full_domain_name: String,
}

#[wasm_bindgen]
impl WasmSdk {
    /// Convert a string to homograph-safe characters
    #[wasm_bindgen(js_name = "dpnsConvertToHomographSafe")]
    pub fn dpns_convert_to_homograph_safe(input: &str) -> String {
        convert_to_homograph_safe_chars(input)
    }

    /// Check if a username is valid according to DPNS rules
    #[wasm_bindgen(js_name = "dpnsIsValidUsername")]
    pub fn dpns_is_valid_username(label: &str) -> bool {
        is_valid_username(label)
    }

    /// Check if a username is contested (requires masternode voting)
    #[wasm_bindgen(js_name = "dpnsIsContestedUsername")]
    pub fn dpns_is_contested_username(label: &str) -> bool {
        is_contested_username(label)
    }

    /// Register a DPNS username
    #[wasm_bindgen(js_name = "dpnsRegisterName")]
    pub async fn dpns_register_name(
        &self,
        label: &str,
        identity_id: &str,
        public_key_id: u32,
        private_key_wif: &str,
        preorder_callback: Option<js_sys::Function>,
    ) -> Result<JsValue, WasmSdkError> {
        // Parse identity ID
        let identity_id_parsed = Identifier::from_string(
            identity_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        // Fetch the identity
        let identity = Identity::fetch(self.as_ref(), identity_id_parsed)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Identity not found"))?;

        // Create signer
        let signer = SingleKeySigner::new(private_key_wif).map_err(|e| {
            WasmSdkError::invalid_argument(format!("Invalid private key WIF: {}", e))
        })?;

        // Get the specific identity public key
        let identity_public_key = identity
            .get_public_key_by_id(public_key_id)
            .ok_or_else(|| {
                WasmSdkError::not_found(format!("Public key with ID {} not found", public_key_id))
            })?
            .clone();

        // Validate the key meets DPNS requirements
        let key_purpose = identity_public_key.purpose();
        let key_security_level = identity_public_key.security_level();

        // Check purpose
        if key_purpose != Purpose::AUTHENTICATION {
            return Err(WasmSdkError::invalid_argument(format!(
                "Cannot register DPNS name with key ID {}: key has purpose {:?} but AUTHENTICATION is required.\n\
                Use a key with purpose AUTHENTICATION (usually keys 0-2).",
                public_key_id, key_purpose
            )));
        }

        // Check security level
        if key_security_level != SecurityLevel::CRITICAL
            && key_security_level != SecurityLevel::HIGH
        {
            let available_keys: Vec<String> = identity
                .public_keys()
                .iter()
                .filter_map(|(key_id, k)| {
                    if k.purpose() == Purpose::AUTHENTICATION
                        && (k.security_level() == SecurityLevel::CRITICAL
                            || k.security_level() == SecurityLevel::HIGH)
                    {
                        let level_name = if k.security_level() == SecurityLevel::CRITICAL {
                            "CRITICAL"
                        } else {
                            "HIGH"
                        };
                        Some(
                            String::from("  Key ")
                                + &key_id.to_string()
                                + ": "
                                + level_name
                                + " security level",
                        )
                    } else {
                        None
                    }
                })
                .collect();

            let suggestion = if available_keys.is_empty() {
                "No suitable keys found in this identity.".to_string()
            } else {
                format!("Try one of these keys:\n{}", available_keys.join("\n"))
            };

            return Err(WasmSdkError::invalid_argument(format!(
                "Cannot register DPNS name with key ID {}: key has {:?} security level but CRITICAL or HIGH is required.\n\
                \n\
                DPNS registration requires a key with:\n\
                - Purpose: AUTHENTICATION\n\
                - Security Level: CRITICAL or HIGH (not MASTER)\n\
                \n\
                {}",
                public_key_id, key_security_level, suggestion
            )));
        }

        // Validate private key matches public key
        if !signer.can_sign_with(&identity_public_key) {
            return Err(WasmSdkError::invalid_argument(format!(
                "The provided private key does not match public key ID {}.\n\
                \n\
                Public key {} details:\n\
                - Security Level: {:?}\n\
                - Purpose: {:?}\n\
                \n\
                Please verify you're using the correct private key (WIF) for this key.",
                public_key_id, public_key_id, key_security_level, key_purpose
            )));
        }

        // Store the JS callback in a thread-local variable that we can access from the closure
        thread_local! {
            static PREORDER_CALLBACK: std::cell::RefCell<Option<js_sys::Function>> = const { std::cell::RefCell::new(None) };
        }

        // Set the callback if provided
        if let Some(ref js_callback) = preorder_callback {
            PREORDER_CALLBACK.with(|cb| {
                *cb.borrow_mut() = Some(js_callback.clone());
            });
        }

        // Create a Rust callback that will call the JavaScript callback
        let callback_box = if preorder_callback.is_some() {
            Some(Box::new(move |doc: &Document| {
                PREORDER_CALLBACK.with(|cb| {
                    if let Some(js_callback) = cb.borrow().as_ref() {
                        let preorder_info = serde_json::json!({
                            "documentId": doc.id().to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58),
                            "ownerId": doc.owner_id().to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58),
                            "revision": doc.revision().unwrap_or(0),
                            "createdAt": doc.created_at(),
                            "createdAtBlockHeight": doc.created_at_block_height(),
                            "createdAtCoreBlockHeight": doc.created_at_core_block_height(),
                            "message": "Preorder document submitted successfully",
                        });

                        if let Ok(js_value) = serde_wasm_bindgen::to_value(&preorder_info) {
                            let _ = js_callback.call1(&wasm_bindgen::JsValue::NULL, &js_value);
                        }
                    }
                });
            }) as Box<dyn FnOnce(&Document) + Send>)
        } else {
            None
        };

        // Create registration input with the callback
        let input = RegisterDpnsNameInput {
            label: label.to_string(),
            identity,
            identity_public_key,
            signer,
            preorder_callback: callback_box,
        };

        // Register the name
        let result = self.as_ref().register_dpns_name(input).await?;

        // Clear the thread-local callback
        PREORDER_CALLBACK.with(|cb| {
            *cb.borrow_mut() = None;
        });

        // Convert result to JS-friendly format
        let js_result = RegisterDpnsNameResult {
            preorder_document_id: result
                .preorder_document
                .id()
                .to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58),
            domain_document_id: result
                .domain_document
                .id()
                .to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58),
            full_domain_name: result.full_domain_name,
        };

        // Serialize to JsValue
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        js_result
            .serialize(&serializer)
            .map_err(|e| WasmSdkError::serialization(format!("Failed to serialize result: {}", e)))
    }

    /// Check if a DPNS name is available
    #[wasm_bindgen(js_name = "dpnsIsNameAvailable")]
    pub async fn dpns_is_name_available(&self, label: &str) -> Result<bool, WasmSdkError> {
        self.as_ref()
            .is_dpns_name_available(label)
            .await
            .map_err(WasmSdkError::from)
    }

    /// Resolve a DPNS name to an identity ID
    #[wasm_bindgen(js_name = "dpnsResolveName")]
    pub async fn dpns_resolve_name(&self, name: &str) -> Result<JsValue, WasmSdkError> {
        let result = self.as_ref().resolve_dpns_name(name).await?;

        match result {
            Some(identity_id) => {
                let id_string = identity_id
                    .to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58);
                Ok(wasm_bindgen::JsValue::from_str(&id_string))
            }
            None => Ok(wasm_bindgen::JsValue::NULL),
        }
    }
}
