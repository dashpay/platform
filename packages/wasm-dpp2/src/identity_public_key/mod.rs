use crate::contract_bounds::ContractBoundsWASM;
use crate::enums::keys::key_type::KeyTypeWASM;
use crate::enums::keys::purpose::PurposeWASM;
use crate::enums::keys::security_level::SecurityLevelWASM;
use crate::enums::network::NetworkWASM;
use crate::utils::{IntoWasm, WithJsError};
use dpp::dashcore::Network;
use dpp::dashcore::secp256k1::hashes::hex::{Case, DisplayHex};
use dpp::identity::contract_bounds::ContractBounds;
use dpp::identity::hash::IdentityPublicKeyHashMethodsV0;
use dpp::identity::identity_public_key::accessors::v0::{
    IdentityPublicKeyGettersV0, IdentityPublicKeySettersV0,
};
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel, TimestampMillis};
use dpp::platform_value::BinaryData;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};

#[derive(Clone)]
#[wasm_bindgen(js_name = IdentityPublicKeyWASM)]
pub struct IdentityPublicKeyWASM(IdentityPublicKey);

impl From<IdentityPublicKey> for IdentityPublicKeyWASM {
    fn from(value: IdentityPublicKey) -> Self {
        IdentityPublicKeyWASM(value)
    }
}

impl From<IdentityPublicKeyWASM> for IdentityPublicKey {
    fn from(value: IdentityPublicKeyWASM) -> Self {
        value.0
    }
}

#[wasm_bindgen]
impl IdentityPublicKeyWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "IdentityPublicKeyWASM".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "IdentityPublicKeyWASM".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        id: u32,
        js_purpose: JsValue,
        js_security_level: JsValue,
        js_key_type: JsValue,
        read_only: bool,
        binary_data: &str,
        disabled_at: Option<TimestampMillis>,
        js_contract_bounds: &JsValue,
    ) -> Result<Self, JsValue> {
        let purpose = PurposeWASM::try_from(js_purpose)?;
        let security_level = SecurityLevelWASM::try_from(js_security_level)?;
        let key_type = KeyTypeWASM::try_from(js_key_type)?;
        let contract_bounds: Option<ContractBounds> =
            match js_contract_bounds.is_undefined() | js_contract_bounds.is_null() {
                true => None,
                false => Some(
                    js_contract_bounds
                        .to_wasm::<ContractBoundsWASM>("ContractBoundsWASM")?
                        .clone()
                        .into(),
                ),
            };

        Ok(IdentityPublicKeyWASM(IdentityPublicKey::from(
            IdentityPublicKeyV0 {
                id,
                purpose: Purpose::from(purpose),
                security_level: SecurityLevel::from(security_level),
                contract_bounds,
                key_type: KeyType::from(key_type),
                read_only,
                data: BinaryData::from_string(binary_data, Hex).unwrap(),
                disabled_at,
            },
        )))
    }
}

#[wasm_bindgen]
impl IdentityPublicKeyWASM {
    #[wasm_bindgen(js_name = "validatePrivateKey")]
    pub fn validate_private_key(
        &self,
        js_private_key_bytes: Vec<u8>,
        js_network: JsValue,
    ) -> Result<bool, JsValue> {
        let mut private_key_bytes = [0u8; 32];
        let len = js_private_key_bytes.len().min(32);
        private_key_bytes[..len].copy_from_slice(&js_private_key_bytes[..len]);

        let network = Network::from(NetworkWASM::try_from(js_network)?);

        self.0
            .validate_private_key_bytes(&private_key_bytes, network)
            .with_js_error()
    }

    #[wasm_bindgen(js_name = "getContractBounds")]
    pub fn contract_bounds(&self) -> JsValue {
        match self.0.contract_bounds() {
            None => JsValue::undefined(),
            Some(bounds) => JsValue::from(ContractBoundsWASM::from(bounds.clone())),
        }
    }

    #[wasm_bindgen(getter = keyId)]
    pub fn get_key_id(&self) -> u32 {
        self.0.id()
    }

    #[wasm_bindgen(getter = purpose)]
    pub fn get_purpose(&self) -> String {
        PurposeWASM::from(self.0.purpose()).into()
    }

    #[wasm_bindgen(getter = purposeNumber)]
    pub fn get_purpose_number(&self) -> PurposeWASM {
        PurposeWASM::from(self.0.purpose())
    }

    #[wasm_bindgen(getter = securityLevel)]
    pub fn get_security_level(&self) -> String {
        SecurityLevelWASM::from(self.0.security_level()).into()
    }

    #[wasm_bindgen(getter = securityLevelNumber)]
    pub fn get_security_level_number(&self) -> SecurityLevelWASM {
        SecurityLevelWASM::from(self.0.security_level())
    }

    #[wasm_bindgen(getter = keyType)]
    pub fn get_key_type(&self) -> String {
        KeyTypeWASM::from(self.0.key_type()).into()
    }

    #[wasm_bindgen(getter = keyTypeNumber)]
    pub fn get_key_type_number(&self) -> KeyTypeWASM {
        KeyTypeWASM::from(self.0.key_type())
    }

    #[wasm_bindgen(getter = readOnly)]
    pub fn get_read_only(&self) -> bool {
        self.0.read_only()
    }

    #[wasm_bindgen(getter = data)]
    pub fn get_data(&self) -> String {
        self.0.data().to_string(Hex)
    }

    #[wasm_bindgen(getter = disabledAt)]
    pub fn get_disabled_at(&self) -> Option<u64> {
        self.0.disabled_at()
    }

    #[wasm_bindgen(setter = keyId)]
    pub fn set_key_id(&mut self, key_id: u32) {
        self.0.set_id(key_id)
    }

    #[wasm_bindgen(setter = purpose)]
    pub fn set_purpose(&mut self, purpose: JsValue) -> Result<(), JsValue> {
        Ok(self
            .0
            .set_purpose(Purpose::from(PurposeWASM::try_from(purpose)?)))
    }

    #[wasm_bindgen(setter = purposeNumber)]
    pub fn set_purpose_number(&mut self, purpose: JsValue) -> Result<(), JsValue> {
        self.set_purpose(purpose)
    }

    #[wasm_bindgen(setter = securityLevel)]
    pub fn set_security_level(&mut self, security_level: JsValue) -> Result<(), JsValue> {
        Ok(self
            .0
            .set_security_level(SecurityLevel::from(SecurityLevelWASM::try_from(
                security_level,
            )?)))
    }

    #[wasm_bindgen(setter = securityLevelNumber)]
    pub fn set_security_level_number(&mut self, security_level: JsValue) -> Result<(), JsValue> {
        self.set_security_level(security_level)
    }

    #[wasm_bindgen(setter = keyType)]
    pub fn set_key_type(&mut self, key_type: JsValue) -> Result<(), JsValue> {
        Ok(self
            .0
            .set_key_type(KeyType::from(KeyTypeWASM::try_from(key_type)?)))
    }

    #[wasm_bindgen(setter = keyTypeNumber)]
    pub fn set_key_type_number(&mut self, key_type: JsValue) -> Result<(), JsValue> {
        self.set_key_type(key_type)
    }

    #[wasm_bindgen(setter = readOnly)]
    pub fn set_read_only(&mut self, read_only: bool) {
        self.0.set_read_only(read_only)
    }

    #[wasm_bindgen(setter = data)]
    pub fn set_data(&mut self, binary_data: &str) {
        let data = BinaryData::from_string(binary_data, Hex).unwrap();

        self.0.set_data(data)
    }

    #[wasm_bindgen(setter = disabledAt)]
    pub fn set_disabled_at(&mut self, disabled_at: u64) {
        self.0.set_disabled_at(disabled_at)
    }

    #[wasm_bindgen(js_name = "getPublicKeyHash")]
    pub fn public_key_hash(&self) -> Result<String, JsValue> {
        let hash = self
            .0
            .public_key_hash()
            .with_js_error()
            .map(|slice| slice.to_vec())?
            .to_hex_string(Case::Lower);

        Ok(hash)
    }

    #[wasm_bindgen(js_name = "isMaster")]
    pub fn is_master(&self) -> bool {
        self.0.is_master()
    }

    #[wasm_bindgen(js_name = bytes)]
    pub fn to_byes(&self) -> Result<Vec<u8>, JsValue> {
        self.0.serialize_to_bytes().with_js_error()
    }

    #[wasm_bindgen(js_name = hex)]
    pub fn to_hex(&self) -> Result<String, JsValue> {
        Ok(encode(
            self.0.serialize_to_bytes().with_js_error()?.as_slice(),
            Hex,
        ))
    }

    #[wasm_bindgen(js_name = base64)]
    pub fn to_base64(&self) -> Result<String, JsValue> {
        Ok(encode(
            self.0.serialize_to_bytes().with_js_error()?.as_slice(),
            Base64,
        ))
    }

    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(bytes: Vec<u8>) -> Result<IdentityPublicKeyWASM, JsValue> {
        let public_key =
            IdentityPublicKey::deserialize_from_bytes(bytes.as_slice()).with_js_error()?;

        Ok(IdentityPublicKeyWASM(public_key))
    }

    #[wasm_bindgen(js_name = fromHex)]
    pub fn from_hex(hex: String) -> Result<IdentityPublicKeyWASM, JsValue> {
        let bytes = decode(&hex, Hex).map_err(|err| JsValue::from(JsError::from(err)))?;

        let public_key =
            IdentityPublicKey::deserialize_from_bytes(bytes.as_slice()).with_js_error()?;

        Ok(IdentityPublicKeyWASM(public_key))
    }

    #[wasm_bindgen(js_name = fromBase64)]
    pub fn from_base64(hex: String) -> Result<IdentityPublicKeyWASM, JsValue> {
        let bytes = decode(&hex, Base64).map_err(|err| JsValue::from(JsError::from(err)))?;

        let public_key =
            IdentityPublicKey::deserialize_from_bytes(bytes.as_slice()).with_js_error()?;

        Ok(IdentityPublicKeyWASM(public_key))
    }
}
