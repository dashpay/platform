use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::serialization;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use js_sys::{Object, Reflect};
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

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        serialization::to_json(&self.0)
    }

    #[wasm_bindgen(js_name = "fromJSON")]
    pub fn from_json(js_value: JsValue) -> WasmDppResult<ResourceVoteChoiceWasm> {
        let choice: ResourceVoteChoice = serialization::from_json(js_value)?;
        Ok(ResourceVoteChoiceWasm(choice))
    }

    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        // Custom object format: { type: string, identityId?: Identifier }
        let obj = Object::new();
        let type_str = self.get_type();
        Reflect::set(&obj, &"type".into(), &type_str.into())
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
        if let ResourceVoteChoice::TowardsIdentity(id) = &self.0 {
            Reflect::set(&obj, &"identityId".into(), &JsValue::from(IdentifierWasm::from(*id)))
                .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
        }
        Ok(obj.into())
    }

    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(js_value: JsValue) -> WasmDppResult<ResourceVoteChoiceWasm> {
        let type_str = Reflect::get(&js_value, &"type".into())
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?
            .as_string()
            .ok_or_else(|| WasmDppError::invalid_argument("type must be a string"))?;

        match type_str.as_str() {
            "TowardsIdentity" => {
                let id_js = Reflect::get(&js_value, &"identityId".into())
                    .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
                let id = IdentifierWasm::try_from(&id_js)?;
                Ok(ResourceVoteChoiceWasm(ResourceVoteChoice::TowardsIdentity(id.into())))
            }
            "Abstain" => Ok(ResourceVoteChoiceWasm(ResourceVoteChoice::Abstain)),
            "Lock" => Ok(ResourceVoteChoiceWasm(ResourceVoteChoice::Lock)),
            other => Err(WasmDppError::invalid_argument(format!(
                "Unknown ResourceVoteChoice type: {}",
                other
            ))),
        }
    }
}
