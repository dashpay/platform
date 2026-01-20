use crate::error::WasmDppResult;
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_wasm_conversions;
use crate::impl_wasm_type_info;
use dpp::identity::contract_bounds::ContractBounds;
use dpp::prelude::Identifier;
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
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        contract_id: IdentifierLikeJs,
        document_type_name: Option<String>,
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
    pub fn single_contract(contract_id: IdentifierLikeJs) -> WasmDppResult<ContractBoundsWasm> {
        let contract_id: Identifier = contract_id.try_into()?;

        Ok(ContractBoundsWasm(ContractBounds::SingleContract {
            id: contract_id,
        }))
    }

    #[wasm_bindgen(js_name = "SingleContractDocumentType")]
    pub fn single_contract_document_type_name(
        contract_id: IdentifierLikeJs,
        document_type_name: String,
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
    pub fn set_id(&mut self, contract_id: IdentifierLikeJs) -> WasmDppResult<()> {
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
}

impl_wasm_conversions!(ContractBoundsWasm, ContractBounds);
impl_wasm_type_info!(ContractBoundsWasm, ContractBounds);
