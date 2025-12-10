use crate::error::{WasmDppError, WasmDppResult};
use crate::group::action_event::GroupActionEventWasm;
use crate::identifier::IdentifierWasm;
use dpp::data_contract::TokenContractPosition;
use dpp::group::group_action::{GroupAction, GroupActionAccessors};
use js_sys::{Object, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "GroupAction")]
pub struct GroupActionWasm(GroupAction);

impl From<GroupAction> for GroupActionWasm {
    fn from(action: GroupAction) -> Self {
        GroupActionWasm(action)
    }
}

impl From<GroupActionWasm> for GroupAction {
    fn from(action: GroupActionWasm) -> Self {
        action.0
    }
}

#[wasm_bindgen(js_class = GroupAction)]
impl GroupActionWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "GroupAction".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "GroupAction".to_string()
    }

    #[wasm_bindgen(getter = "contractId")]
    pub fn contract_id(&self) -> IdentifierWasm {
        self.0.contract_id().into()
    }

    #[wasm_bindgen(getter = "proposerId")]
    pub fn proposer_id(&self) -> IdentifierWasm {
        self.0.proposer_id().into()
    }

    #[wasm_bindgen(getter = "tokenContractPosition")]
    pub fn token_contract_position(&self) -> TokenContractPosition {
        self.0.token_contract_position()
    }

    #[wasm_bindgen(getter = "event")]
    pub fn event(&self) -> GroupActionEventWasm {
        GroupActionEventWasm::from(self.0.event().clone())
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        let obj = Object::new();
        Reflect::set(
            &obj,
            &"contractId".into(),
            &self.contract_id().to_base58().into(),
        )
        .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
        Reflect::set(
            &obj,
            &"proposerId".into(),
            &self.proposer_id().to_base58().into(),
        )
        .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
        Reflect::set(
            &obj,
            &"tokenContractPosition".into(),
            &JsValue::from(self.token_contract_position()),
        )
        .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
        Reflect::set(&obj, &"event".into(), &self.event().to_json()?)
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
        Ok(obj.into())
    }

    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        let obj = Object::new();
        Reflect::set(
            &obj,
            &"contractId".into(),
            &JsValue::from(self.contract_id()),
        )
        .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
        Reflect::set(
            &obj,
            &"proposerId".into(),
            &JsValue::from(self.proposer_id()),
        )
        .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
        Reflect::set(
            &obj,
            &"tokenContractPosition".into(),
            &JsValue::from(self.token_contract_position()),
        )
        .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
        Reflect::set(&obj, &"event".into(), &self.event().to_object()?)
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
        Ok(obj.into())
    }
}
