use crate::identifier::IdentifierWASM;
use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use dpp::platform_value::string_encoding::Encoding::Base58;
use dpp::platform_value::string_encoding::encode;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "AuthorizedActionTakersWASM")]
pub struct AuthorizedActionTakersWASM(AuthorizedActionTakers);

impl From<AuthorizedActionTakers> for AuthorizedActionTakersWASM {
    fn from(action: AuthorizedActionTakers) -> Self {
        AuthorizedActionTakersWASM(action)
    }
}

impl From<AuthorizedActionTakersWASM> for AuthorizedActionTakers {
    fn from(action: AuthorizedActionTakersWASM) -> Self {
        action.0
    }
}

#[wasm_bindgen]
impl AuthorizedActionTakersWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "AuthorizedActionTakersWASM".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "AuthorizedActionTakersWASM".to_string()
    }

    #[wasm_bindgen(js_name = "NoOne")]
    pub fn no_one() -> Self {
        AuthorizedActionTakersWASM(AuthorizedActionTakers::NoOne)
    }

    #[wasm_bindgen(js_name = "ContractOwner")]
    pub fn contract_owner() -> Self {
        AuthorizedActionTakersWASM(AuthorizedActionTakers::ContractOwner)
    }

    #[wasm_bindgen(js_name = "Identity")]
    pub fn identity(js_identity_id: &JsValue) -> Result<Self, JsValue> {
        let identity_id = IdentifierWASM::try_from(js_identity_id)?;

        Ok(AuthorizedActionTakersWASM(
            AuthorizedActionTakers::Identity(identity_id.into()),
        ))
    }

    #[wasm_bindgen(js_name = "MainGroup")]
    pub fn main_group() -> Self {
        AuthorizedActionTakersWASM(AuthorizedActionTakers::MainGroup)
    }

    #[wasm_bindgen(js_name = "Group")]
    pub fn group(group_contract_position: u16) -> Self {
        AuthorizedActionTakersWASM(AuthorizedActionTakers::Group(group_contract_position))
    }

    #[wasm_bindgen(js_name = "getTakerType")]
    pub fn taker_type(&self) -> String {
        match self.0 {
            AuthorizedActionTakers::NoOne => "NoOne".to_string(),
            AuthorizedActionTakers::ContractOwner => "ContractOwner".to_string(),
            AuthorizedActionTakers::Identity(identifier) => {
                format!("Identity({})", encode(identifier.as_slice(), Base58))
            }
            AuthorizedActionTakers::MainGroup => "MainGroup".to_string(),
            AuthorizedActionTakers::Group(group) => format!("Group({})", group),
        }
    }

    #[wasm_bindgen(js_name = "getValue")]
    pub fn get_value(&self) -> JsValue {
        match self.0 {
            AuthorizedActionTakers::NoOne => JsValue::undefined(),
            AuthorizedActionTakers::ContractOwner => JsValue::undefined(),
            AuthorizedActionTakers::Identity(identifier) => {
                JsValue::from(IdentifierWASM::from(identifier))
            }
            AuthorizedActionTakers::MainGroup => JsValue::undefined(),
            AuthorizedActionTakers::Group(position) => JsValue::from(position),
        }
    }
}
