use crate::data_contract::contract_bounds::ContractBoundsWasm;
use crate::enums::keys::key_type::{KeyTypeLikeJs, KeyTypeWasm};
use crate::enums::keys::purpose::{PurposeLikeJs, PurposeWasm};
use crate::enums::keys::security_level::{SecurityLevelLikeJs, SecurityLevelWasm};
use crate::error::{WasmDppError, WasmDppResult};
use crate::identity::public_key::IdentityPublicKeyWasm;
use crate::impl_try_from_js_value;
use crate::impl_wasm_conversions;
use crate::impl_wasm_type_info;
use crate::utils::{try_from_options, try_from_options_optional, try_to_u32};
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::state_transition::public_key_in_creation::accessors::{
    IdentityPublicKeyInCreationV0Getters, IdentityPublicKeyInCreationV0Setters,
};
use dpp::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
use serde::Deserialize;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdentityPublicKeyInCreationOptions {
    key_id: u32,
    data: Vec<u8>,
    #[serde(default)]
    is_read_only: bool,
    #[serde(default)]
    signature: Option<Vec<u8>>,
}

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
export interface IdentityPublicKeyInCreationOptions {
    keyId: number;
    purpose: PurposeLike;
    securityLevel: SecurityLevelLike;
    keyType: KeyTypeLike;
    isReadOnly?: boolean;
    data: Uint8Array;
    signature?: Uint8Array;
    contractBounds?: ContractBounds;
}

/**
 * IdentityPublicKeyInCreation serialized as a plain object.
 */
export interface IdentityPublicKeyInCreationObject {
    keyId: number;
    purpose: Purpose;
    securityLevel: SecurityLevel;
    keyType: KeyType;
    isReadOnly: boolean;
    data: Uint8Array;
    signature?: Uint8Array;
    contractBounds?: ContractBoundsObject;
}

/**
 * IdentityPublicKeyInCreation serialized as JSON.
 */
export interface IdentityPublicKeyInCreationJSON {
    keyId: number;
    purpose: string;
    securityLevel: string;
    keyType: string;
    isReadOnly: boolean;
    data: string;
    signature?: string;
    contractBounds?: ContractBoundsJSON;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IdentityPublicKeyInCreationOptions")]
    pub type IdentityPublicKeyInCreationOptionsJs;

    #[wasm_bindgen(typescript_type = "IdentityPublicKeyInCreationObject")]
    pub type IdentityPublicKeyInCreationObjectJs;

    #[wasm_bindgen(typescript_type = "IdentityPublicKeyInCreationJSON")]
    pub type IdentityPublicKeyInCreationJSONJs;
}

#[derive(Clone)]
#[wasm_bindgen(js_name = "IdentityPublicKeyInCreation")]
pub struct IdentityPublicKeyInCreationWasm(IdentityPublicKeyInCreation);

impl From<IdentityPublicKeyInCreation> for IdentityPublicKeyInCreationWasm {
    fn from(value: IdentityPublicKeyInCreation) -> Self {
        IdentityPublicKeyInCreationWasm(value)
    }
}

impl From<IdentityPublicKeyInCreationWasm> for IdentityPublicKeyInCreation {
    fn from(value: IdentityPublicKeyInCreationWasm) -> Self {
        value.0
    }
}

impl From<IdentityPublicKeyInCreationWasm> for IdentityPublicKey {
    fn from(value: IdentityPublicKeyInCreationWasm) -> Self {
        let contract_bounds = value.0.contract_bounds().cloned();

        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: value.0.id(),
            purpose: value.0.purpose(),
            security_level: value.0.security_level(),
            contract_bounds,
            key_type: value.0.key_type(),
            read_only: value.0.read_only(),
            data: value.0.data().clone(),
            disabled_at: None,
        })
    }
}

#[wasm_bindgen(js_class = IdentityPublicKeyInCreation)]
impl IdentityPublicKeyInCreationWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: IdentityPublicKeyInCreationOptionsJs,
    ) -> WasmDppResult<IdentityPublicKeyInCreationWasm> {
        // Extract purpose (required, complex type)
        let purpose: PurposeWasm = try_from_options(&options, "purpose")?;

        // Extract securityLevel (required, complex type)
        let security_level: SecurityLevelWasm = try_from_options(&options, "securityLevel")?;

        // Extract keyType (required, complex type)
        let key_type: KeyTypeWasm = try_from_options(&options, "keyType")?;

        // Extract contractBounds (optional)
        let contract_bounds: Option<ContractBoundsWasm> =
            try_from_options_optional(&options, "contractBounds")?;

        // Extract simple fields via serde
        let opts: IdentityPublicKeyInCreationOptions = serde_wasm_bindgen::from_value(options.into())
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        Ok(IdentityPublicKeyInCreationWasm(
            IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
                id: opts.key_id,
                key_type: KeyType::from(key_type),
                purpose: Purpose::from(purpose),
                security_level: SecurityLevel::from(security_level),
                contract_bounds: contract_bounds.map(Into::into),
                read_only: opts.is_read_only,
                data: BinaryData::new(opts.data),
                signature: BinaryData::from(opts.signature.unwrap_or_default()),
            }),
        ))
    }

    #[wasm_bindgen(js_name = "toIdentityPublicKey")]
    pub fn to_identity_public_key(&self) -> IdentityPublicKeyWasm {
        let key: IdentityPublicKey = IdentityPublicKey::from(&self.0);
        IdentityPublicKeyWasm::from(key)
    }

    #[wasm_bindgen(js_name = "getHash")]
    pub fn get_hash(&self) -> WasmDppResult<Vec<u8>> {
        let hash = self.0.hash()?;
        Ok(hash.to_vec())
    }

    #[wasm_bindgen(getter = "contractBounds")]
    pub fn contract_bounds(&self) -> Option<ContractBoundsWasm> {
        self.0
            .contract_bounds()
            .map(|bounds| ContractBoundsWasm::from(bounds.clone()))
    }

    #[wasm_bindgen(getter = keyId)]
    pub fn key_id(&self) -> u32 {
        self.0.id()
    }

    #[wasm_bindgen(getter = purpose)]
    pub fn purpose(&self) -> String {
        PurposeWasm::from(self.0.purpose()).into()
    }

    #[wasm_bindgen(getter = securityLevel)]
    pub fn security_level(&self) -> String {
        SecurityLevelWasm::from(self.0.security_level()).into()
    }

    #[wasm_bindgen(getter = keyType)]
    pub fn key_type(&self) -> String {
        KeyTypeWasm::from(self.0.key_type()).into()
    }

    #[wasm_bindgen(getter = "isReadOnly")]
    pub fn is_read_only(&self) -> bool {
        self.0.read_only()
    }

    #[wasm_bindgen(getter = data)]
    pub fn data(&self) -> Vec<u8> {
        self.0.data().to_vec()
    }

    #[wasm_bindgen(getter = signature)]
    pub fn signature(&self) -> Vec<u8> {
        self.0.signature().to_vec()
    }

    #[wasm_bindgen(setter = keyId)]
    pub fn set_key_id(
        &mut self,
        #[wasm_bindgen(js_name = "keyId")] key_id: JsValue,
    ) -> WasmDppResult<()> {
        self.0.set_id(try_to_u32(&key_id, "keyId")?);
        Ok(())
    }

    #[wasm_bindgen(setter = purpose)]
    pub fn set_purpose(&mut self, purpose: PurposeLikeJs) -> WasmDppResult<()> {
        let purpose: Purpose = purpose.try_into()?;
        self.0.set_purpose(purpose);
        Ok(())
    }

    #[wasm_bindgen(setter = securityLevel)]
    pub fn set_security_level(
        &mut self,
        #[wasm_bindgen(js_name = "securityLevel")] security_level: SecurityLevelLikeJs,
    ) -> WasmDppResult<()> {
        let security_level: SecurityLevel = security_level.try_into()?;
        self.0.set_security_level(security_level);
        Ok(())
    }

    #[wasm_bindgen(setter = keyType)]
    pub fn set_key_type(
        &mut self,
        #[wasm_bindgen(js_name = "keyType")] key_type: KeyTypeLikeJs,
    ) -> WasmDppResult<()> {
        let key_type: KeyType = key_type.try_into()?;
        self.0.set_type(key_type);
        Ok(())
    }

    #[wasm_bindgen(setter = "isReadOnly")]
    pub fn set_is_read_only(&mut self, #[wasm_bindgen(js_name = "isReadOnly")] is_read_only: bool) {
        self.0.set_read_only(is_read_only)
    }

    #[wasm_bindgen(setter = data)]
    pub fn set_data(&mut self, #[wasm_bindgen(js_name = "binaryData")] binary_data: Vec<u8>) {
        let data = BinaryData::from(binary_data);
        self.0.set_data(data)
    }

    #[wasm_bindgen(setter = signature)]
    pub fn set_signature(
        &mut self,
        #[wasm_bindgen(js_name = "signatureBytes")] binary_data: Vec<u8>,
    ) {
        let signature = BinaryData::from(binary_data);
        self.0.set_signature(signature)
    }

    #[wasm_bindgen(setter = "contractBounds")]
    pub fn set_contract_bounds(&mut self, bounds: Option<ContractBoundsWasm>) {
        self.0.set_contract_bounds(bounds.map(|b| b.into()));
    }
}

impl IdentityPublicKeyInCreationWasm {
    pub fn vec_from_array(
        add_public_keys: &js_sys::Array,
    ) -> WasmDppResult<Vec<IdentityPublicKeyInCreationWasm>> {
        let add_public_keys: Vec<IdentityPublicKeyInCreationWasm> = add_public_keys
            .iter()
            .map(|v| Self::try_from(&v))
            .collect::<Result<Vec<IdentityPublicKeyInCreationWasm>, WasmDppError>>()?;

        Ok(add_public_keys)
    }
}

impl_try_from_js_value!(IdentityPublicKeyInCreationWasm, "IdentityPublicKeyInCreation");
impl_wasm_conversions!(
    IdentityPublicKeyInCreationWasm,
    IdentityPublicKeyInCreation,
    IdentityPublicKeyInCreationObjectJs,
    IdentityPublicKeyInCreationJSONJs
);
impl_wasm_type_info!(IdentityPublicKeyInCreationWasm, IdentityPublicKeyInCreation);
