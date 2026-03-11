use crate::identifier::IdentifierWasm;
use crate::{impl_wasm_conversions_serde, impl_wasm_type_info};
use dpp::data_contract::TokenContractPosition;
use dpp::tokens::contract_info::TokenContractInfo;
use dpp::tokens::contract_info::v0::TokenContractInfoV0Accessors;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TOKEN_CONTRACT_INFO_TYPES_TS: &str = r#"
/**
 * TokenContractInfo serialized as a plain object.
 */
export interface TokenContractInfoObject {
    contractId: Uint8Array;
    tokenContractPosition: number;
}

/**
 * TokenContractInfo serialized as JSON (with string identifiers).
 */
export interface TokenContractInfoJSON {
    contractId: string;
    tokenContractPosition: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenContractInfoObject")]
    pub type TokenContractInfoObjectJs;

    #[wasm_bindgen(typescript_type = "TokenContractInfoJSON")]
    pub type TokenContractInfoJSONJs;
}

#[wasm_bindgen(js_name = "TokenContractInfo")]
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct TokenContractInfoWasm(TokenContractInfo);

impl From<TokenContractInfo> for TokenContractInfoWasm {
    fn from(info: TokenContractInfo) -> Self {
        Self(info)
    }
}

impl From<TokenContractInfoWasm> for TokenContractInfo {
    fn from(info: TokenContractInfoWasm) -> Self {
        info.0
    }
}

#[wasm_bindgen(js_class = TokenContractInfo)]
impl TokenContractInfoWasm {
    #[wasm_bindgen(getter = "contractId")]
    pub fn contract_id(&self) -> IdentifierWasm {
        match &self.0 {
            TokenContractInfo::V0(v0) => v0.contract_id().into(),
        }
    }

    #[wasm_bindgen(getter = "tokenContractPosition")]
    pub fn token_contract_position(&self) -> TokenContractPosition {
        match &self.0 {
            TokenContractInfo::V0(v0) => v0.token_contract_position(),
        }
    }
}

impl_wasm_conversions_serde!(
    TokenContractInfoWasm,
    TokenContractInfo,
    TokenContractInfoObjectJs,
    TokenContractInfoJSONJs
);
impl_wasm_type_info!(TokenContractInfoWasm, TokenContractInfo);
