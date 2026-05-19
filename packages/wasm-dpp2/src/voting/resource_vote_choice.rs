use crate::error::WasmDppResult;
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::{
    impl_try_from_js_value, impl_try_from_options, impl_wasm_conversions_inner, impl_wasm_type_info,
};
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * ResourceVoteChoice serialized as a plain object.
 */
export type ResourceVoteChoiceObject =
    | { type: "towardsIdentity"; data: Uint8Array }
    | { type: "abstain" }
    | { type: "lock" };

/**
 * ResourceVoteChoice serialized as JSON.
 */
export type ResourceVoteChoiceJSON =
    | { type: "towardsIdentity"; data: string }
    | { type: "abstain" }
    | { type: "lock" };
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ResourceVoteChoiceObject")]
    pub type ResourceVoteChoiceObjectJs;

    #[wasm_bindgen(typescript_type = "ResourceVoteChoiceJSON")]
    pub type ResourceVoteChoiceJSONJs;
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
#[wasm_bindgen(js_name = "ResourceVoteChoice")]
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
    pub fn towards_identity(id: IdentifierLikeJs) -> WasmDppResult<Self> {
        Ok(ResourceVoteChoiceWasm(ResourceVoteChoice::TowardsIdentity(
            id.try_into()?,
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
    pub fn value(&self) -> Option<IdentifierWasm> {
        match self.0 {
            ResourceVoteChoice::TowardsIdentity(id) => Some(IdentifierWasm::from(id)),
            ResourceVoteChoice::Abstain => None,
            ResourceVoteChoice::Lock => None,
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

impl_try_from_js_value!(ResourceVoteChoiceWasm, "ResourceVoteChoice");
impl_try_from_options!(ResourceVoteChoiceWasm);
impl_wasm_conversions_inner!(
    ResourceVoteChoiceWasm,
    ResourceVoteChoice,
    ResourceVoteChoice,
    ResourceVoteChoiceObjectJs,
    ResourceVoteChoiceJSONJs
);
impl_wasm_type_info!(ResourceVoteChoiceWasm, ResourceVoteChoice);
