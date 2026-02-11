use crate::error::WasmDppResult;
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_try_from_js_value;
use crate::impl_wasm_conversions;
use crate::impl_wasm_type_info;
use dpp::identity::contract_bounds::ContractBounds;
use dpp::prelude::Identifier;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * ContractBounds serialized as a plain object.
 */
export interface ContractBoundsObject {
    identifier: Uint8Array;
    documentTypeName?: string;
    contractBoundsType: "SingleContract" | "SingleContractDocumentType";
}

/**
 * ContractBounds serialized as JSON.
 */
export interface ContractBoundsJSON {
    identifier: string;
    documentTypeName?: string;
    contractBoundsType: "SingleContract" | "SingleContractDocumentType";
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ContractBoundsObject")]
    pub type ContractBoundsObjectJs;

    #[wasm_bindgen(typescript_type = "ContractBoundsJSON")]
    pub type ContractBoundsJSONJs;
}

#[wasm_bindgen(js_name = "ContractBounds")]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
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
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        #[wasm_bindgen(js_name = "contractId")] contract_id: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "documentTypeName")] document_type_name: Option<String>,
    ) -> WasmDppResult<ContractBoundsWasm> {
        let contract_id: Identifier = contract_id.try_into()?;

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
        #[wasm_bindgen(js_name = "contractId")] contract_id: IdentifierLikeJs,
    ) -> WasmDppResult<ContractBoundsWasm> {
        let contract_id: Identifier = contract_id.try_into()?;

        Ok(ContractBoundsWasm(ContractBounds::SingleContract {
            id: contract_id,
        }))
    }

    #[wasm_bindgen(js_name = "SingleContractDocumentType")]
    pub fn single_contract_document_type_name(
        #[wasm_bindgen(js_name = "contractId")] contract_id: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "documentTypeName")] document_type_name: String,
    ) -> WasmDppResult<ContractBoundsWasm> {
        let contract_id: Identifier = contract_id.try_into()?;

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
        #[wasm_bindgen(js_name = "contractId")] contract_id: IdentifierLikeJs,
    ) -> WasmDppResult<()> {
        let contract_id: Identifier = contract_id.try_into()?;

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
    pub fn set_document_type_name(
        &mut self,
        #[wasm_bindgen(js_name = "documentTypeName")] document_type_name: String,
    ) {
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
}

impl_wasm_conversions!(
    ContractBoundsWasm,
    ContractBounds,
    ContractBoundsObjectJs,
    ContractBoundsJSONJs
);
impl_try_from_js_value!(ContractBoundsWasm, "ContractBounds");
impl_wasm_type_info!(ContractBoundsWasm, ContractBounds);
