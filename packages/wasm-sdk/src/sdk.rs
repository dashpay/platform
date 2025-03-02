use crate::context_provider::WasmContext;
use crate::dpp::{DataContractWasm, IdentityWasm};
use dash_sdk::dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dash_sdk::dpp::dashcore::{Network, PrivateKey};
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::data_contract::DataContractFactory;
use dash_sdk::dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dash_sdk::dpp::identity::signer::Signer;
use dash_sdk::dpp::identity::IdentityV0;
use dash_sdk::dpp::prelude::AssetLockProof;
use dash_sdk::dpp::serialization::PlatformSerializableWithPlatformVersion;
use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dash_sdk::platform::transition::put_identity::PutIdentity;
use dash_sdk::platform::{DataContract, Document, DocumentQuery, Fetch, Identifier, Identity};
use dash_sdk::sdk::AddressList;
use dash_sdk::{Sdk, SdkBuilder};
use platform_value::platform_value;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsError;
use web_sys::{console, js_sys};

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
pub async fn identity_fetch(sdk: &WasmSdk, base58_id: &str) -> Result<IdentityWasm, JsError> {
    let id = Identifier::from_string(
        base58_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    Identity::fetch_by_identifier(sdk, id)
        .await?
        .ok_or_else(|| JsError::new("Identity not found"))
        .map(Into::into)
}

#[wasm_bindgen]
pub async fn data_contract_fetch(
    sdk: &WasmSdk,
    base58_id: &str,
) -> Result<DataContractWasm, JsError> {
    let id = Identifier::from_string(
        base58_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    DataContract::fetch_by_identifier(sdk, id)
        .await?
        .ok_or_else(|| JsError::new("Data contract not found"))
        .map(Into::into)
}

#[wasm_bindgen]
pub async fn identity_put(sdk: &WasmSdk) {
    // This is just a mock implementation to show how to use the SDK and ensure proper linking
    // of all required dependencies. This function is not supposed to work.
    let id = Identifier::from_bytes(&[0; 32]).expect("create identifier");

    let identity = Identity::V0(IdentityV0 {
        id,
        public_keys: BTreeMap::new(),
        balance: 0,
        revision: 0,
    });

    let asset_lock_proof = AssetLockProof::default();
    let asset_lock_proof_private_key =
        PrivateKey::from_slice(&[0; 32], Network::Testnet).expect("create private key");

    let signer = MockSigner;
    let _pushed: Identity = identity
        .put_to_platform(
            sdk,
            asset_lock_proof,
            &asset_lock_proof_private_key,
            &signer,
            None,
        )
        .await
        .expect("put identity")
        .broadcast_and_wait(sdk, None)
        .await
        .unwrap();
}

#[wasm_bindgen]
pub async fn epoch_testing() {
    let sdk = SdkBuilder::new(AddressList::new())
        .build()
        .expect("build sdk");

    let _ei = ExtendedEpochInfo::fetch(&sdk, 0)
        .await
        .expect("fetch extended epoch info")
        .expect("extended epoch info not found");
}

#[wasm_bindgen]
pub async fn docs_testing(sdk: &WasmSdk) {
    let id = Identifier::random();

    let factory = DataContractFactory::new(1).expect("create data contract factory");
    factory
        .create(id, 1, platform_value!({}), None, None)
        .expect("create data contract");

    let dc = DataContract::fetch(sdk, id)
        .await
        .expect("fetch data contract")
        .expect("data contract not found");

    let dcs = dc
        .serialize_to_bytes_with_platform_version(sdk.version())
        .expect("serialize data contract");

    let query = DocumentQuery::new(dc.clone(), "asd").expect("create query");
    let doc = Document::fetch(sdk, query)
        .await
        .expect("fetch document")
        .expect("document not found");

    let document_type = dc
        .document_type_for_name("aaa")
        .expect("document type for name");
    let doc_serialized = doc
        .serialize(document_type, sdk.version())
        .expect("serialize document");

    let msg = js_sys::JsString::from_str(&format!("{:?} {:?} ", dcs, doc_serialized))
        .expect("create js string");
    console::log_1(&msg);
}

#[derive(Clone, Debug)]
struct MockSigner;
impl Signer for MockSigner {
    fn can_sign_with(&self, _identity_public_key: &dash_sdk::platform::IdentityPublicKey) -> bool {
        true
    }
    fn sign(
        &self,
        _identity_public_key: &dash_sdk::platform::IdentityPublicKey,
        _data: &[u8],
    ) -> Result<dash_sdk::dpp::platform_value::BinaryData, dash_sdk::dpp::ProtocolError> {
        todo!("signature creation is not implemented due to lack of dash platform wallet support in wasm")
    }
}
