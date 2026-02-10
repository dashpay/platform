use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::identity::public_key::IdentityPublicKeyWasm;
use crate::impl_try_from_js_value;
use crate::impl_try_from_options;
use crate::impl_wasm_type_info;
use crate::serialization;
use crate::utils::try_to_u64;
use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::{Identity, KeyID};
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::Identifier;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable, ValueConvertible};
use dpp::version::{PlatformVersion, TryFromPlatformVersioned};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const IDENTITY_TYPES_TS: &str = r#"
/**
 * Identity serialized as a plain object.
 */
export interface IdentityObject {
    $version: string;
    id: Identifier;
    publicKeys: IdentityPublicKeyObject[];
    balance: bigint;
    revision: bigint;
}

/**
 * Identity serialized as JSON (with string identifiers).
 */
export interface IdentityJSON {
    $version: string;
    id: string;
    publicKeys: IdentityPublicKeyJSON[];
    balance: number;
    revision: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IdentityObject")]
    pub type IdentityObjectJs;

    #[wasm_bindgen(typescript_type = "IdentityJSON")]
    pub type IdentityJSONJs;
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
#[wasm_bindgen(js_name = "Identity")]
pub struct IdentityWasm(Identity);

impl From<Identity> for IdentityWasm {
    fn from(identity: Identity) -> Self {
        Self(identity)
    }
}

impl From<IdentityWasm> for Identity {
    fn from(wasm: IdentityWasm) -> Self {
        wasm.0
    }
}

#[wasm_bindgen(js_class = Identity)]
impl IdentityWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(id: IdentifierLikeJs) -> WasmDppResult<IdentityWasm> {
        let identifier: Identifier = id.try_into()?;
        let identity = Identity::create_basic_identity(identifier, PlatformVersion::first())?;
        Ok(IdentityWasm(identity))
    }

    #[wasm_bindgen(setter = "id")]
    pub fn set_id(&mut self, id: IdentifierLikeJs) -> WasmDppResult<()> {
        self.0.set_id(id.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(setter = "balance")]
    pub fn set_balance(&mut self, balance: JsValue) -> WasmDppResult<()> {
        self.0.set_balance(try_to_u64(&balance, "balance")?);
        Ok(())
    }

    #[wasm_bindgen(setter = "revision")]
    pub fn set_revision(&mut self, revision: JsValue) -> WasmDppResult<()> {
        self.0.set_revision(try_to_u64(&revision, "revision")?);
        Ok(())
    }

    #[wasm_bindgen(js_name = "addPublicKey")]
    pub fn add_public_key(
        &mut self,
        #[wasm_bindgen(js_name = "publicKey")] public_key: &IdentityPublicKeyWasm,
    ) {
        self.0.add_public_key(public_key.clone().into());
    }

    // GETTERS

    #[wasm_bindgen(getter = "id")]
    pub fn id(&self) -> IdentifierWasm {
        self.0.id().into()
    }

    #[wasm_bindgen(getter = "balance")]
    pub fn balance(&self) -> u64 {
        self.0.balance()
    }

    #[wasm_bindgen(getter = "revision")]
    pub fn revision(&self) -> u64 {
        self.0.revision()
    }

    #[wasm_bindgen(js_name = "getPublicKeyById")]
    pub fn get_public_key_by_id(
        &self,
        #[wasm_bindgen(js_name = "keyId")] key_id: KeyID,
    ) -> Option<IdentityPublicKeyWasm> {
        self.0
            .get_public_key_by_id(key_id)
            .map(|key| IdentityPublicKeyWasm::from(key.clone()))
    }

    #[wasm_bindgen(getter = "publicKeys")]
    pub fn public_keys(&self) -> Vec<IdentityPublicKeyWasm> {
        self.0
            .public_keys()
            .values()
            .map(|key| IdentityPublicKeyWasm::from(key.clone()))
            .collect()
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<IdentityWasm> {
        let bytes =
            decode(hex.as_str(), Hex).map_err(|e| WasmDppError::serialization(e.to_string()))?;

        IdentityWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> WasmDppResult<IdentityWasm> {
        let bytes = decode(base64.as_str(), Base64)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;

        IdentityWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> WasmDppResult<Vec<u8>> {
        Ok(self.0.serialize_to_bytes()?)
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self) -> WasmDppResult<String> {
        let bytes = self.0.serialize_to_bytes()?;
        Ok(encode(bytes.as_slice(), Hex))
    }

    #[wasm_bindgen(js_name = "toBase64")]
    pub fn to_base64(&self) -> WasmDppResult<String> {
        let bytes = self.0.serialize_to_bytes()?;
        Ok(encode(bytes.as_slice(), Base64))
    }

    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<IdentityObjectJs> {
        // Use platform_value conversion which handles BigInt for balance/revision
        // and outputs id as Uint8Array, publicKeys as plain objects
        let value = self.0.to_object()?;
        let js_value = serialization::platform_value_to_object(&value)?;
        Ok(js_value.into())
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<IdentityJSONJs> {
        let js_value = serialization::to_json(&self.0)?;
        Ok(js_value.into())
    }

    #[wasm_bindgen(js_name = "fromJSON")]
    pub fn from_json(value: IdentityJSONJs) -> WasmDppResult<IdentityWasm> {
        serialization::from_json(value.into()).map(IdentityWasm)
    }

    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(value: IdentityObjectJs) -> WasmDppResult<IdentityWasm> {
        let platform_value = serialization::js_value_to_platform_value(&value.into())?;
        let identity =
            Identity::try_from_platform_versioned(platform_value, PlatformVersion::latest())?;
        Ok(IdentityWasm(identity))
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<IdentityWasm> {
        let identity = Identity::deserialize_from_bytes(bytes.as_slice())?;
        Ok(IdentityWasm(identity))
    }
}

impl_try_from_js_value!(IdentityWasm, "Identity");
impl_try_from_options!(IdentityWasm);
impl_wasm_type_info!(IdentityWasm, Identity);
