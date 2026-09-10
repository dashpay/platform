use crate::error::WasmDppError;
use crate::error::WasmDppResult;
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_try_from_js_value;
use crate::impl_wasm_conversions_inner;
use crate::impl_wasm_type_info;
use dpp::identity::contract_bounds::{
    AuthenticationScope, ContractBounds, authentication_scope::permissions,
};
use dpp::prelude::Identifier;
use dpp::serialization::JsonConvertible;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

/// Combine explicitly granted actions with bitwise OR.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug)]
pub enum AuthenticationPermission {
    DocumentCreate = 1,
    DocumentReplace = 2,
    DocumentDelete = 4,
    DocumentTransfer = 8,
    DocumentUpdatePrice = 16,
    DocumentPurchase = 32,
    DocumentTokenPayment = 64,
    TokenBurn = 128,
    TokenMint = 256,
    TokenTransfer = 512,
    TokenFreeze = 1024,
    TokenUnfreeze = 2048,
    TokenDestroyFrozenFunds = 4096,
    TokenClaim = 8192,
    TokenEmergencyAction = 16384,
    TokenConfigUpdate = 32768,
    TokenDirectPurchase = 65536,
    TokenSetPrice = 131072,
}
// wasm-bindgen requires literal discriminants; keep them tied to consensus bits.
const _: () = {
    assert!(AuthenticationPermission::DocumentCreate as u32 == permissions::DOCUMENT_CREATE);
    assert!(AuthenticationPermission::DocumentReplace as u32 == permissions::DOCUMENT_REPLACE);
    assert!(AuthenticationPermission::DocumentDelete as u32 == permissions::DOCUMENT_DELETE);
    assert!(AuthenticationPermission::DocumentTransfer as u32 == permissions::DOCUMENT_TRANSFER);
    assert!(
        AuthenticationPermission::DocumentUpdatePrice as u32 == permissions::DOCUMENT_UPDATE_PRICE
    );
    assert!(AuthenticationPermission::DocumentPurchase as u32 == permissions::DOCUMENT_PURCHASE);
    assert!(
        AuthenticationPermission::DocumentTokenPayment as u32
            == permissions::DOCUMENT_TOKEN_PAYMENT
    );
    assert!(AuthenticationPermission::TokenBurn as u32 == permissions::TOKEN_BURN);
    assert!(AuthenticationPermission::TokenMint as u32 == permissions::TOKEN_MINT);
    assert!(AuthenticationPermission::TokenTransfer as u32 == permissions::TOKEN_TRANSFER);
    assert!(AuthenticationPermission::TokenFreeze as u32 == permissions::TOKEN_FREEZE);
    assert!(AuthenticationPermission::TokenUnfreeze as u32 == permissions::TOKEN_UNFREEZE);
    assert!(
        AuthenticationPermission::TokenDestroyFrozenFunds as u32
            == permissions::TOKEN_DESTROY_FROZEN_FUNDS
    );
    assert!(AuthenticationPermission::TokenClaim as u32 == permissions::TOKEN_CLAIM);
    assert!(
        AuthenticationPermission::TokenEmergencyAction as u32
            == permissions::TOKEN_EMERGENCY_ACTION
    );
    assert!(AuthenticationPermission::TokenConfigUpdate as u32 == permissions::TOKEN_CONFIG_UPDATE);
    assert!(
        AuthenticationPermission::TokenDirectPurchase as u32 == permissions::TOKEN_DIRECT_PURCHASE
    );
    assert!(AuthenticationPermission::TokenSetPrice as u32 == permissions::TOKEN_SET_PRICE);
};

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
export interface ContractScopeInput { id: string; documentTypes?: string[] | null; }
export interface AuthenticationScopeJSON {
    $formatVersion: "0";
    contracts: ContractScopeInput[];
    permissions: number;
    expiresAt: number | string | null;
}

/**
 * ContractBounds serialized as a plain object.
 */
export type ContractBoundsObject =
    | { $type: "singleContract"; id: Uint8Array }
    | { $type: "documentType"; id: Uint8Array; documentTypeName: string }
    | { $type: "scoped"; $formatVersion: "0"; contracts: { id: Uint8Array; documentTypes?: string[] | null }[]; permissions: number; expiresAt?: bigint | null };

/** ContractBounds serialized as JSON. */
export type ContractBoundsJSON =
    | { $type: "singleContract"; id: string }
    | { $type: "documentType"; id: string; documentTypeName: string }
    | ({ $type: "scoped" } & AuthenticationScopeJSON);
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

    /// Creates an application delegation. Sort order is canonicalized by the
    /// constructor; duplicates/empty restrictions remain errors.
    #[wasm_bindgen(js_name = "Scoped")]
    pub fn scoped(
        contracts: JsValue,
        permissions: u32,
        expires_at: Option<u64>,
    ) -> WasmDppResult<ContractBoundsWasm> {
        let contracts_json: serde_json::Value = serde_wasm_bindgen::from_value(contracts)
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;
        let mut scope = AuthenticationScope::from_json(serde_json::json!({
            "$formatVersion": "0", "contracts": contracts_json,
            "permissions": permissions, "expiresAt": expires_at,
        }))?;
        let AuthenticationScope::V0(ref mut inner) = scope;
        inner.contracts.sort_by_key(|entry| entry.id);
        for entry in &mut inner.contracts {
            if let Some(names) = &mut entry.document_types {
                names.sort();
            }
        }
        scope.validate()?;
        Ok(ContractBoundsWasm(ContractBounds::Scoped(scope)))
    }

    #[wasm_bindgen(getter)]
    pub fn scope(&self) -> WasmDppResult<JsValue> {
        match &self.0 {
            ContractBounds::Scoped(scope) => {
                crate::serialization::conversions::json_to_js_value(&scope.to_json()?)
            }
            _ => Ok(JsValue::UNDEFINED),
        }
    }

    #[wasm_bindgen(getter = "identifier")]
    pub fn id(&self) -> Option<IdentifierWasm> {
        self.0.identifier().copied().map(Into::into)
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
            ContractBounds::Scoped(_) => {
                return Err(WasmDppError::invalid_argument(
                    "replace the complete scope to change scoped bounds",
                ));
            }
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
    ) -> WasmDppResult<()> {
        self.0 = match self.clone().0 {
            ContractBounds::Scoped(_) => {
                return Err(WasmDppError::invalid_argument(
                    "replace the complete scope to change scoped bounds",
                ));
            }
            ContractBounds::SingleContract { .. } => self.clone().0,
            ContractBounds::SingleContractDocumentType { id, .. } => {
                ContractBounds::SingleContractDocumentType {
                    id,
                    document_type_name,
                }
            }
        };
        Ok(())
    }
}

impl_wasm_conversions_inner!(
    ContractBoundsWasm,
    ContractBounds,
    ContractBounds,
    ContractBoundsObjectJs,
    ContractBoundsJSONJs
);
impl_try_from_js_value!(ContractBoundsWasm, "ContractBounds");
impl_wasm_type_info!(ContractBoundsWasm, ContractBounds);
