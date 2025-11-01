use crate::error::WasmDppResult;
use crate::identifier::IdentifierWasm;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone)]
#[wasm_bindgen(js_name = ResourceVoteChoice)]
pub struct ResourceVoteChoiceWasm(ResourceVoteChoice);

impl From<ResourceVoteChoice> for ResourceVoteChoiceWasm {
    fn from(choice: ResourceVoteChoice) -> Self {
        Self(choice)
    }
}

impl From<ResourceVoteChoiceWasm> for ResourceVoteChoice {
    fn from(choice: ResourceVoteChoiceWasm) -> Self {
        choice.0
    }
}

#[wasm_bindgen(js_class = ResourceVoteChoice)]
impl ResourceVoteChoiceWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "ResourceVoteChoice".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "ResourceVoteChoice".to_string()
    }

    #[wasm_bindgen(js_name = "TowardsIdentity")]
    pub fn towards_identity(js_id: &JsValue) -> WasmDppResult<Self> {
        let id = IdentifierWasm::try_from(js_id)?;

        Ok(ResourceVoteChoiceWasm(ResourceVoteChoice::TowardsIdentity(
            id.into(),
        )))
    }

    #[wasm_bindgen(js_name = "Abstain")]
    pub fn abstain() -> Self {
        ResourceVoteChoiceWasm(ResourceVoteChoice::Abstain)
    }

    #[wasm_bindgen(js_name = "Lock")]
    pub fn lock() -> Self {
        ResourceVoteChoiceWasm(ResourceVoteChoice::Lock)
    }

    #[wasm_bindgen(js_name = "getValue")]
    pub fn get_value(&self) -> JsValue {
        match self.0 {
            ResourceVoteChoice::TowardsIdentity(id) => JsValue::from(IdentifierWasm::from(id)),
            ResourceVoteChoice::Abstain => JsValue::undefined(),
            ResourceVoteChoice::Lock => JsValue::undefined(),
        }
    }

    #[wasm_bindgen(js_name = "getType")]
    pub fn get_type(&self) -> String {
        match self.0 {
            ResourceVoteChoice::TowardsIdentity(_) => "TowardsIdentity".to_string(),
            ResourceVoteChoice::Abstain => "Abstain".to_string(),
            ResourceVoteChoice::Lock => "Lock".to_string(),
        }
    }
}
