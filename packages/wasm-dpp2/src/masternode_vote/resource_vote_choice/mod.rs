use crate::identifier::IdentifierWASM;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone)]
#[wasm_bindgen(js_name = ResourceVoteChoice)]
pub struct ResourceVoteChoiceWASM(ResourceVoteChoice);

impl From<ResourceVoteChoice> for ResourceVoteChoiceWASM {
    fn from(choice: ResourceVoteChoice) -> Self {
        Self(choice)
    }
}

impl From<ResourceVoteChoiceWASM> for ResourceVoteChoice {
    fn from(choice: ResourceVoteChoiceWASM) -> Self {
        choice.0
    }
}

#[wasm_bindgen(js_class = ResourceVoteChoice)]
impl ResourceVoteChoiceWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "ResourceVoteChoice".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "ResourceVoteChoice".to_string()
    }

    #[wasm_bindgen(js_name = "TowardsIdentity")]
    pub fn towards_identity(js_id: &JsValue) -> Result<Self, JsValue> {
        let id = IdentifierWASM::try_from(js_id)?;

        Ok(ResourceVoteChoiceWASM(ResourceVoteChoice::TowardsIdentity(
            id.into(),
        )))
    }

    #[wasm_bindgen(js_name = "Abstain")]
    pub fn abstain() -> Self {
        ResourceVoteChoiceWASM(ResourceVoteChoice::Abstain)
    }

    #[wasm_bindgen(js_name = "Lock")]
    pub fn lock() -> Self {
        ResourceVoteChoiceWASM(ResourceVoteChoice::Lock)
    }

    #[wasm_bindgen(js_name = "getValue")]
    pub fn get_value(&self) -> JsValue {
        match self.0.clone() {
            ResourceVoteChoice::TowardsIdentity(id) => JsValue::from(IdentifierWASM::from(id)),
            ResourceVoteChoice::Abstain => JsValue::undefined(),
            ResourceVoteChoice::Lock => JsValue::undefined(),
        }
    }

    #[wasm_bindgen(js_name = "getType")]
    pub fn get_type(&self) -> String {
        match self.0.clone() {
            ResourceVoteChoice::TowardsIdentity(_) => "TowardsIdentity".to_string(),
            ResourceVoteChoice::Abstain => "Abstain".to_string(),
            ResourceVoteChoice::Lock => "Lock".to_string(),
        }
    }
}
