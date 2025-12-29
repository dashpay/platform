use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::impl_wasm_conversions;
use crate::utils::IntoWasm;
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
    pub fn towards_identity(
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")] js_id: &JsValue,
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

impl ResourceVoteChoiceWasm {
    /// Try to extract a ResourceVoteChoice from an options object field.
    ///
    /// This helper reads the specified field from an options object and converts it
    /// to a ResourceVoteChoiceWasm.
    pub fn try_from_options(options: &JsValue, field_name: &str) -> WasmDppResult<Self> {
        let choice_js =
            js_sys::Reflect::get(options, &JsValue::from_str(field_name)).map_err(|_| {
                WasmDppError::invalid_argument(format!("Missing '{}' field", field_name))
            })?;

        if choice_js.is_undefined() || choice_js.is_null() {
            return Err(WasmDppError::invalid_argument(format!(
                "'{}' is required",
                field_name
            )));
        }

        choice_js
            .to_wasm::<ResourceVoteChoiceWasm>("ResourceVoteChoice")
            .map(|boxed| (*boxed).clone())
    }
}

impl_wasm_conversions!(ResourceVoteChoiceWasm, ResourceVoteChoice);
