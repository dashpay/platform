use crate::context_provider::{WasmContext, ContextProvider};
use crate::trusted_context_provider::TrustedHttpContextProvider;
use crate::trusted_context_provider_universal::UniversalTrustedHttpContextProvider;
use platform_value::Identifier;
// use dash_sdk::platform::transition::broadcast::BroadcastStateTransition; // Not available in WASM
// use dash_sdk::platform::transition::put_identity::PutIdentity; // Not available in WASM
// use dash_sdk::sdk::AddressList; // Not available in WASM
// use dash_sdk::{Sdk, SdkBuilder}; // Not available in WASM
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsError;

#[derive(Debug, Clone)]
pub enum WasmContextProviderEnum {
    Default(WasmContext),
    TrustedHttp(Box<TrustedHttpContextProvider>),
    UniversalTrustedHttp(Box<UniversalTrustedHttpContextProvider>),
}

impl ContextProvider for WasmContextProviderEnum {
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], crate::context_provider::ContextProviderError> {
        match self {
            WasmContextProviderEnum::Default(provider) => provider.get_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height),
            WasmContextProviderEnum::TrustedHttp(provider) => provider.get_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height),
            WasmContextProviderEnum::UniversalTrustedHttp(provider) => provider.get_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height),
        }
    }

    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &dpp::version::PlatformVersion,
    ) -> Result<Option<std::sync::Arc<dpp::data_contract::DataContract>>, crate::context_provider::ContextProviderError> {
        match self {
            WasmContextProviderEnum::Default(provider) => provider.get_data_contract(id, platform_version),
            WasmContextProviderEnum::TrustedHttp(provider) => provider.get_data_contract(id, platform_version),
            WasmContextProviderEnum::UniversalTrustedHttp(provider) => provider.get_data_contract(id, platform_version),
        }
    }

    fn get_platform_activation_height(&self) -> Result<dpp::prelude::CoreBlockHeight, crate::context_provider::ContextProviderError> {
        match self {
            WasmContextProviderEnum::Default(provider) => provider.get_platform_activation_height(),
            WasmContextProviderEnum::TrustedHttp(provider) => provider.get_platform_activation_height(),
            WasmContextProviderEnum::UniversalTrustedHttp(provider) => provider.get_platform_activation_height(),
        }
    }

    fn get_token_configuration(
        &self,
        token_id: &Identifier,
    ) -> Result<Option<dpp::data_contract::associated_token::token_configuration::TokenConfiguration>, crate::context_provider::ContextProviderError> {
        match self {
            WasmContextProviderEnum::Default(provider) => provider.get_token_configuration(token_id),
            WasmContextProviderEnum::TrustedHttp(provider) => provider.get_token_configuration(token_id),
            WasmContextProviderEnum::UniversalTrustedHttp(provider) => provider.get_token_configuration(token_id),
        }
    }
}

// Mock SDK types for WASM compatibility
#[derive(Debug, Clone)]
pub struct Sdk {
    version: platform_version::version::PlatformVersion,
    context_provider: Option<WasmContextProviderEnum>,
}

#[derive(Debug, Clone)]
pub struct SdkBuilder {
    context_provider: Option<WasmContextProviderEnum>,
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

    pub fn with_context_provider(mut self, context_provider: WasmContextProviderEnum) -> Self {
        self.context_provider = Some(context_provider);
        self
    }

    pub fn build(self) -> Result<Sdk, JsError> {
        Ok(Sdk {
            version: platform_version::version::PlatformVersion::latest().clone(),
            context_provider: self.context_provider,
        })
    }
}

#[wasm_bindgen]
#[derive(Clone)]
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
    pub fn process_identity_nonce_response(&self, _response_bytes: &[u8]) -> Result<u64, JsError> {
        // This would be called by JavaScript after it receives the response
        // For now, return a mock value
        Ok(0)
    }

    /// Process identity contract nonce response from platform
    pub fn process_identity_contract_nonce_response(
        &self,
        _response_bytes: &[u8],
    ) -> Result<u64, JsError> {
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
        let sdk_builder = SdkBuilder::new_mainnet().with_context_provider(WasmContextProviderEnum::Default(WasmContext {}));

        Self(sdk_builder)
    }

    pub fn new_testnet() -> Self {
        let sdk_builder = SdkBuilder::new_testnet().with_context_provider(WasmContextProviderEnum::Default(WasmContext {}));
        Self(sdk_builder)
    }

    pub fn build(self) -> Result<WasmSdk, JsError> {
        Ok(WasmSdk(self.0.build()?))
    }

    pub fn with_context_provider(self, context_provider: WasmContext) -> Self {
        WasmSdkBuilder(self.0.with_context_provider(WasmContextProviderEnum::Default(context_provider)))
    }
    
    pub fn with_trusted_http_context_provider(self, context_provider: TrustedHttpContextProvider) -> Self {
        WasmSdkBuilder(self.0.with_context_provider(WasmContextProviderEnum::TrustedHttp(Box::new(context_provider))))
    }
    
    pub fn with_universal_trusted_context_provider(self, context_provider: UniversalTrustedHttpContextProvider) -> Self {
        WasmSdkBuilder(self.0.with_context_provider(WasmContextProviderEnum::UniversalTrustedHttp(Box::new(context_provider))))
    }
}

#[wasm_bindgen]
pub fn prepare_identity_fetch_request(
    _sdk: &WasmSdk,
    base58_id: &str,
    prove: bool,
) -> Result<Vec<u8>, JsError> {
    let _id =
        Identifier::from_string(base58_id, platform_value::string_encoding::Encoding::Base58)?;

    // Use serializer module to prepare the request
    use crate::serializer::serialize_get_identity_request;
    serialize_get_identity_request(base58_id, prove).map(|bytes| bytes.to_vec())
}
