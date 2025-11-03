use crate::error::WasmDppResult;
use crate::identifier::IdentifierWasm;
use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use dpp::platform_value::string_encoding::Encoding::Base58;
use dpp::platform_value::string_encoding::encode;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "AuthorizedActionTakers")]
pub struct AuthorizedActionTakersWasm(AuthorizedActionTakers);

impl From<AuthorizedActionTakers> for AuthorizedActionTakersWasm {
    fn from(action: AuthorizedActionTakers) -> Self {
        AuthorizedActionTakersWasm(action)
    }
}

impl From<AuthorizedActionTakersWasm> for AuthorizedActionTakers {
    fn from(action: AuthorizedActionTakersWasm) -> Self {
        action.0
    }
}

#[wasm_bindgen(js_class = AuthorizedActionTakers)]
impl AuthorizedActionTakersWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "AuthorizedActionTakers".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "AuthorizedActionTakers".to_string()
    }

    #[wasm_bindgen(js_name = "NoOne")]
    pub fn no_one() -> Self {
        AuthorizedActionTakersWasm(AuthorizedActionTakers::NoOne)
    }

    #[wasm_bindgen(js_name = "ContractOwner")]
    pub fn contract_owner() -> Self {
        AuthorizedActionTakersWasm(AuthorizedActionTakers::ContractOwner)
    }

    #[wasm_bindgen(js_name = "Identity")]
    pub fn identity(js_identity_id: &JsValue) -> WasmDppResult<Self> {
        let identity_id = IdentifierWasm::try_from(js_identity_id)?;

        Ok(AuthorizedActionTakersWasm(
            AuthorizedActionTakers::Identity(identity_id.into()),
        ))
    }

    #[wasm_bindgen(js_name = "MainGroup")]
    pub fn main_group() -> Self {
        AuthorizedActionTakersWasm(AuthorizedActionTakers::MainGroup)
    }

    #[wasm_bindgen(js_name = "Group")]
    pub fn group(group_contract_position: u16) -> Self {
        AuthorizedActionTakersWasm(AuthorizedActionTakers::Group(group_contract_position))
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
                JsValue::from(IdentifierWasm::from(identifier))
            }
            AuthorizedActionTakers::MainGroup => JsValue::undefined(),
            AuthorizedActionTakers::Group(position) => JsValue::from(position),
        }
    }
}
