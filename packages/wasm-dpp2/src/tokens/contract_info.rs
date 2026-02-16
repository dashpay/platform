use crate::identifier::IdentifierWasm;
use crate::impl_wasm_type_info;
use dpp::data_contract::TokenContractPosition;
use dpp::tokens::contract_info::TokenContractInfo;
use dpp::tokens::contract_info::v0::TokenContractInfoV0Accessors;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = "TokenContractInfo")]
#[derive(Clone, Debug, PartialEq)]
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

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let obj = js_sys::Object::new();
        let contract_id: IdentifierWasm = self.contract_id();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("contractId"),
            &JsValue::from_str(&contract_id.to_base58()),
        )?;
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("tokenContractPosition"),
            &JsValue::from_f64(self.token_contract_position() as f64),
        )?;
        Ok(obj.into())
    }
}

impl_wasm_type_info!(TokenContractInfoWasm, TokenContractInfo);
