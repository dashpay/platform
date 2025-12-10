use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use dpp::identity::contract_bounds::ContractBounds;
use dpp::prelude::Identifier;
use serde::Serialize;
use serde_json::Value as JsonValue;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "ContractBounds")]
#[derive(Clone)]
pub struct ContractBoundsWasm(ContractBounds);

impl From<ContractBounds> for ContractBoundsWasm {
    fn from(bounds: ContractBounds) -> Self {
        ContractBoundsWasm(bounds)
    }
}

impl From<ContractBoundsWasm> for ContractBounds {
    fn from(bounds: ContractBoundsWasm) -> Self {
        bounds.0
    }
}

#[wasm_bindgen(js_class = ContractBounds)]
impl ContractBoundsWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "ContractBounds".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "ContractBounds".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        js_contract_id: &JsValue,
        document_type_name: Option<String>,
    ) -> WasmDppResult<ContractBoundsWasm> {
        let contract_id: Identifier = IdentifierWasm::try_from(js_contract_id)?.into();

        Ok(ContractBoundsWasm(match document_type_name {
            Some(document_type_name) => ContractBounds::SingleContractDocumentType {
                id: contract_id,
                document_type_name,
            },
            None => ContractBounds::SingleContract { id: contract_id },
        }))
    }

    #[wasm_bindgen(js_name = "SingleContract")]
    pub fn single_contract(
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        js_contract_id: &JsValue,
    ) -> WasmDppResult<ContractBoundsWasm> {
        let contract_id: Identifier = IdentifierWasm::try_from(js_contract_id)?.into();

        Ok(ContractBoundsWasm(ContractBounds::SingleContract {
            id: contract_id,
        }))
    }

    #[wasm_bindgen(js_name = "SingleContractDocumentType")]
    pub fn single_contract_document_type_name(
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        js_contract_id: &JsValue,
        document_type_name: String,
    ) -> WasmDppResult<ContractBoundsWasm> {
        let contract_id: Identifier = IdentifierWasm::try_from(js_contract_id)?.into();

        Ok(ContractBoundsWasm(
            ContractBounds::SingleContractDocumentType {
                id: contract_id,
                document_type_name,
            },
        ))
    }

    #[wasm_bindgen(getter = "identifier")]
    pub fn id(&self) -> IdentifierWasm {
        (*self.0.identifier()).into()
    }

    #[wasm_bindgen(getter = "documentTypeName")]
    pub fn document_type_name(&self) -> Option<String> {
        self.0.document_type().cloned()
    }

    #[wasm_bindgen(getter = "contractBoundsType")]
    pub fn contract_bounds_type(&self) -> String {
        self.0.contract_bounds_type_string().into()
    }

    #[wasm_bindgen(getter = "contractBoundsTypeNumber")]
    pub fn contract_bounds_type_number(&self) -> u8 {
        self.0.contract_bounds_type()
    }

    #[wasm_bindgen(setter = "identifier")]
    pub fn set_id(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        js_contract_id: &JsValue,
    ) -> WasmDppResult<()> {
        let contract_id: Identifier = IdentifierWasm::try_from(js_contract_id)?.into();

        self.0 = match self.clone().0 {
            ContractBounds::SingleContract { .. } => {
                ContractBounds::SingleContract { id: contract_id }
            }
            ContractBounds::SingleContractDocumentType {
                document_type_name, ..
            } => ContractBounds::SingleContractDocumentType {
                id: contract_id,
                document_type_name,
            },
        };

        Ok(())
    }

    #[wasm_bindgen(setter = "documentTypeName")]
    pub fn set_document_type_name(&mut self, document_type_name: String) {
        self.0 = match self.clone().0 {
            ContractBounds::SingleContract { .. } => self.clone().0,
            ContractBounds::SingleContractDocumentType { id, .. } => {
                ContractBounds::SingleContractDocumentType {
                    id,
                    document_type_name,
                }
            }
        }
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        let json_value = serde_json::to_value(&self.0)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        json_value
            .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
            .map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    #[wasm_bindgen(js_name = "fromJSON")]
    pub fn from_json(js_value: JsValue) -> WasmDppResult<ContractBoundsWasm> {
        let json_value: JsonValue = serde_wasm_bindgen::from_value(js_value)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        let bounds: ContractBounds = serde_json::from_value(json_value)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        Ok(ContractBoundsWasm(bounds))
    }

    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        serde_wasm_bindgen::to_value(&self.0)
            .map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(js_value: JsValue) -> WasmDppResult<ContractBoundsWasm> {
        let bounds: ContractBounds = serde_wasm_bindgen::from_value(js_value)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        Ok(ContractBoundsWasm(bounds))
    }
}
