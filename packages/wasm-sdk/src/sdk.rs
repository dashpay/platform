use crate::context_provider::WasmContext;
use crate::dpp::{DataContractWasm, IdentityWasm};
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dpp::dashcore::{Network, PrivateKey};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContractFactory;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityV0;
use dpp::prelude::AssetLockProof;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
// use dash_sdk::platform::transition::broadcast::BroadcastStateTransition; // Not available in WASM
// use dash_sdk::platform::transition::put_identity::PutIdentity; // Not available in WASM
use dpp::document::Document;
use dpp::data_contract::DataContract;
use dpp::identity::Identity;
use platform_value::Identifier;
// use dash_sdk::sdk::AddressList; // Not available in WASM
// use dash_sdk::{Sdk, SdkBuilder}; // Not available in WASM
use platform_value::platform_value;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsError;
use web_sys::{console, js_sys};

// Mock SDK types for WASM compatibility
#[derive(Debug, Clone)]
pub struct Sdk {
    version: platform_version::version::PlatformVersion,
}

#[derive(Debug, Clone)]
pub struct SdkBuilder {
    context_provider: Option<WasmContext>,
}

impl SdkBuilder {
    pub fn new_mainnet() -> Self {
        SdkBuilder {
            context_provider: None,
        }
    }
    
    pub fn new_testnet() -> Self {
        SdkBuilder {
            context_provider: None,
        }
    }
    
    pub fn with_context_provider(mut self, context_provider: WasmContext) -> Self {
        self.context_provider = Some(context_provider);
        self
    }
    
    pub fn build(self) -> Result<Sdk, JsError> {
        Ok(Sdk {
            version: platform_version::version::PlatformVersion::latest().clone(),
        })
    }
}

#[wasm_bindgen]
pub struct WasmSdk(Sdk);
// Dereference JsSdk to Sdk so that we can use &JsSdk everywhere where &sdk is needed
impl std::ops::Deref for WasmSdk {
    type Target = Sdk;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<Sdk> for WasmSdk {
    fn as_ref(&self) -> &Sdk {
        &self.0
    }
}

impl From<Sdk> for WasmSdk {
    fn from(sdk: Sdk) -> Self {
        WasmSdk(sdk)
    }
}

impl WasmSdk {
    pub fn version(&self) -> &platform_version::version::PlatformVersion {
        &self.0.version
    }
    
    /// Get the network name (mainnet, testnet, devnet)
    pub fn network(&self) -> String {
        // For now, default to testnet
        // In production, this would be set during SDK initialization
        "testnet".to_string()
    }
    
    /// Process identity nonce response from platform
    pub fn process_identity_nonce_response(&self, response_bytes: &[u8]) -> Result<u64, JsError> {
        // This would be called by JavaScript after it receives the response
        // For now, return a mock value
        Ok(0)
    }
    
    /// Process identity contract nonce response from platform
    pub fn process_identity_contract_nonce_response(&self, response_bytes: &[u8]) -> Result<u64, JsError> {
        // This would be called by JavaScript after it receives the response
        // For now, return a mock value
        Ok(0)
    }
}

#[wasm_bindgen]
pub struct WasmSdkBuilder(SdkBuilder);

impl Deref for WasmSdkBuilder {
    type Target = SdkBuilder;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for WasmSdkBuilder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[wasm_bindgen]
impl WasmSdkBuilder {
    pub fn new_mainnet() -> Self {
        let sdk_builder = SdkBuilder::new_mainnet().with_context_provider(WasmContext {});

        Self(sdk_builder)
    }

    pub fn new_testnet() -> Self {
        WasmSdkBuilder(SdkBuilder::new_testnet()).with_context_provider(WasmContext {})
    }

    pub fn build(self) -> Result<WasmSdk, JsError> {
        Ok(WasmSdk(self.0.build()?))
    }

    pub fn with_context_provider(self, context_provider: WasmContext) -> Self {
        WasmSdkBuilder(self.0.with_context_provider(context_provider))
    }
}

#[wasm_bindgen]
pub fn prepare_identity_fetch_request(sdk: &WasmSdk, base58_id: &str, prove: bool) -> Result<Vec<u8>, JsError> {
    let id = Identifier::from_string(
        base58_id,
        platform_value::string_encoding::Encoding::Base58,
    )?;

    
    // Use serializer module to prepare the request
    use crate::serializer::serialize_get_identity_request;
    serialize_get_identity_request(base58_id, prove)
        .map(|bytes| bytes.to_vec())
}


#[derive(Clone, Debug)]
struct MockSigner;
impl Signer for MockSigner {
    fn can_sign_with(&self, _identity_public_key: &dpp::identity::IdentityPublicKey) -> bool {
        true
    }
    fn sign(
        &self,
        _identity_public_key: &dpp::identity::IdentityPublicKey,
        _data: &[u8],
    ) -> Result<platform_value::BinaryData, dpp::ProtocolError> {
        todo!("signature creation is not implemented due to lack of dash platform wallet support in wasm")
    }
}
