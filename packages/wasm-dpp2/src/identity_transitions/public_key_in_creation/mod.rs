use crate::contract_bounds::ContractBoundsWasm;
use crate::enums::keys::key_type::KeyTypeWasm;
use crate::enums::keys::purpose::PurposeWasm;
use crate::enums::keys::security_level::SecurityLevelWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identity_public_key::IdentityPublicKeyWasm;
use crate::utils::IntoWasm;
use dpp::identity::contract_bounds::ContractBounds;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::platform_value::string_encoding::Encoding::Hex;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::state_transition::public_key_in_creation::accessors::{
    IdentityPublicKeyInCreationV0Getters, IdentityPublicKeyInCreationV0Setters,
};
use dpp::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

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

impl TryFrom<JsValue> for IdentityPublicKeyInCreationWasm {
    type Error = WasmDppError;
    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        let value =
            value.to_wasm::<IdentityPublicKeyInCreationWasm>("IdentityPublicKeyInCreation")?;

        Ok(value.clone())
    }
}

impl From<IdentityPublicKeyInCreationWasm> for IdentityPublicKey {
    fn from(value: IdentityPublicKeyInCreationWasm) -> Self {
        let contract_bounds = match value.0.contract_bounds() {
            None => None,
            Some(bounds) => Some(bounds.clone()),
        };

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
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "IdentityPublicKeyInCreation".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "IdentityPublicKeyInCreation".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        id: u32,
        js_purpose: JsValue,
        js_security_level: JsValue,
        js_key_type: JsValue,
        read_only: bool,
        binary_data: Vec<u8>,
        signature: Option<Vec<u8>>,
        js_contract_bounds: &JsValue,
    ) -> WasmDppResult<IdentityPublicKeyInCreationWasm> {
        let purpose = PurposeWasm::try_from(js_purpose)?;
        let security_level = SecurityLevelWasm::try_from(js_security_level)?;
        let key_type = KeyTypeWasm::try_from(js_key_type)?;
        let contract_bounds: Option<ContractBounds> =
            match js_contract_bounds.is_undefined() | js_contract_bounds.is_null() {
                true => None,
                false => Some(
                    js_contract_bounds
                        .to_wasm::<ContractBoundsWasm>("ContractBounds")?
                        .clone()
                        .into(),
                ),
            };

        Ok(IdentityPublicKeyInCreationWasm(
            IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
                id,
                key_type: KeyType::from(key_type),
                purpose: Purpose::from(purpose),
                security_level: SecurityLevel::from(security_level),
                contract_bounds,
                read_only,
                data: BinaryData::from(binary_data),
                signature: BinaryData::from(signature.unwrap_or(Vec::new())),
            }),
        ))
    }

    #[wasm_bindgen(js_name = toIdentityPublicKey)]
    pub fn to_identity_public_key(&self) -> WasmDppResult<IdentityPublicKeyWasm> {
        Ok(IdentityPublicKeyWasm::new(
            self.0.id(),
            JsValue::from(PurposeWasm::from(self.0.purpose())),
            JsValue::from(SecurityLevelWasm::from(self.0.security_level())),
            JsValue::from(KeyTypeWasm::from(self.0.key_type())),
            self.0.read_only(),
            self.0.data().to_string(Hex).as_str(),
            None,
            &JsValue::from(self.get_contract_bounds().clone()),
        )?)
    }

    #[wasm_bindgen(js_name = "getHash")]
    pub fn get_hash(&self) -> WasmDppResult<Vec<u8>> {
        let hash = self.0.hash()?;
        Ok(hash.to_vec())
    }

    #[wasm_bindgen(getter = "contractBounds")]
    pub fn get_contract_bounds(&self) -> Option<ContractBoundsWasm> {
        match self.0.contract_bounds() {
            Some(bounds) => Some(ContractBoundsWasm::from(bounds.clone())),
            None => None,
        }
    }

    #[wasm_bindgen(getter = keyId)]
    pub fn get_key_id(&self) -> u32 {
        self.0.id()
    }

    #[wasm_bindgen(getter = purpose)]
    pub fn get_purpose(&self) -> String {
        PurposeWasm::from(self.0.purpose()).into()
    }

    #[wasm_bindgen(getter = securityLevel)]
    pub fn get_security_level(&self) -> String {
        SecurityLevelWasm::from(self.0.security_level()).into()
    }

    #[wasm_bindgen(getter = keyType)]
    pub fn get_key_type(&self) -> String {
        KeyTypeWasm::from(self.0.key_type()).into()
    }

    #[wasm_bindgen(getter = readOnly)]
    pub fn get_read_only(&self) -> bool {
        self.0.read_only()
    }

    #[wasm_bindgen(getter = data)]
    pub fn get_data(&self) -> Vec<u8> {
        self.0.data().to_vec()
    }

    #[wasm_bindgen(getter = signature)]
    pub fn get_signature(&self) -> Vec<u8> {
        self.0.signature().to_vec()
    }

    #[wasm_bindgen(setter = keyId)]
    pub fn set_key_id(&mut self, key_id: u32) {
        self.0.set_id(key_id)
    }

    #[wasm_bindgen(setter = purpose)]
    pub fn set_purpose(&mut self, js_purpose: JsValue) -> WasmDppResult<()> {
        let purpose = PurposeWasm::try_from(js_purpose)?;
        self.0.set_purpose(Purpose::from(purpose));
        Ok(())
    }

    #[wasm_bindgen(setter = securityLevel)]
    pub fn set_security_level(&mut self, js_security_level: JsValue) -> WasmDppResult<()> {
        let security_level = SecurityLevelWasm::try_from(js_security_level)?;
        self.0
            .set_security_level(SecurityLevel::from(security_level));
        Ok(())
    }

    #[wasm_bindgen(setter = keyType)]
    pub fn set_key_type(&mut self, key_type: JsValue) -> WasmDppResult<()> {
        let key_type = KeyTypeWasm::try_from(key_type)?;
        self.0.set_type(key_type.into());
        Ok(())
    }

    #[wasm_bindgen(setter = readOnly)]
    pub fn set_read_only(&mut self, read_only: bool) {
        self.0.set_read_only(read_only)
    }

    #[wasm_bindgen(setter = data)]
    pub fn set_data(&mut self, binary_data: Vec<u8>) {
        let data = BinaryData::from(binary_data);
        self.0.set_data(data)
    }

    #[wasm_bindgen(setter = signature)]
    pub fn set_signature(&mut self, binary_data: Vec<u8>) {
        let signature = BinaryData::from(binary_data);
        self.0.set_signature(signature)
    }

    #[wasm_bindgen(setter = "contractBounds")]
    pub fn set_contract_bounds(&mut self, js_bounds: &JsValue) -> WasmDppResult<()> {
        match js_bounds.is_undefined() {
            true => self.0.set_contract_bounds(None),
            false => {
                let bounds = js_bounds
                    .to_wasm::<ContractBoundsWasm>("ContractBounds")?
                    .clone();

                self.0.set_contract_bounds(Some(bounds.into()))
            }
        };

        Ok(())
    }
}

impl IdentityPublicKeyInCreationWasm {
    pub fn vec_from_js_value(
        js_add_public_keys: &js_sys::Array,
    ) -> WasmDppResult<Vec<IdentityPublicKeyInCreationWasm>> {
        let add_public_keys: Vec<IdentityPublicKeyInCreationWasm> = js_add_public_keys
            .iter()
            .map(IdentityPublicKeyInCreationWasm::try_from)
            .collect::<Result<Vec<IdentityPublicKeyInCreationWasm>, WasmDppError>>()?;

        Ok(add_public_keys)
    }
}
