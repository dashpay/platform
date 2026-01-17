use crate::error::WasmDppResult;
use crate::identifier::IdentifierWasm;
use crate::{impl_try_from_options, impl_wasm_conversions, impl_wasm_type_info};
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
    #[wasm_bindgen(js_name = "TowardsIdentity")]
    pub fn towards_identity(
        #[wasm_bindgen(unchecked_param_type = "IdentifierLike")] js_id: &JsValue,
    ) -> WasmDppResult<Self> {
        let id = IdentifierWasm::try_from(js_id)?.into();

        Ok(ResourceVoteChoiceWasm(ResourceVoteChoice::TowardsIdentity(
            id,
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

    #[wasm_bindgen(getter = "value")]
    pub fn value(&self) -> JsValue {
        match self.0 {
            ResourceVoteChoice::TowardsIdentity(id) => JsValue::from(IdentifierWasm::from(id)),
            ResourceVoteChoice::Abstain => JsValue::undefined(),
            ResourceVoteChoice::Lock => JsValue::undefined(),
        }
    }

    #[wasm_bindgen(getter = "voteType")]
    pub fn vote_type(&self) -> String {
        match self.0 {
            ResourceVoteChoice::TowardsIdentity(_) => "TowardsIdentity".to_string(),
            ResourceVoteChoice::Abstain => "Abstain".to_string(),
            ResourceVoteChoice::Lock => "Lock".to_string(),
        }
    }
}

impl_try_from_options!(ResourceVoteChoiceWasm, "ResourceVoteChoice");
impl_wasm_conversions!(ResourceVoteChoiceWasm, ResourceVoteChoice);
impl_wasm_type_info!(ResourceVoteChoiceWasm, ResourceVoteChoice);
