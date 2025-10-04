use crate::identifier::IdentifierWasm;
use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "TokenDistributionRecipient")]
pub struct TokenDistributionRecipientWasm(TokenDistributionRecipient);

impl From<TokenDistributionRecipient> for TokenDistributionRecipientWasm {
    fn from(distribution_recipient: TokenDistributionRecipient) -> Self {
        TokenDistributionRecipientWasm(distribution_recipient)
    }
}

impl From<TokenDistributionRecipientWasm> for TokenDistributionRecipient {
    fn from(distribution_recipient: TokenDistributionRecipientWasm) -> Self {
        distribution_recipient.0
    }
}

#[wasm_bindgen(js_class = TokenDistributionRecipient)]
impl TokenDistributionRecipientWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenDistributionRecipient".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenDistributionRecipient".to_string()
    }

    #[wasm_bindgen(js_name = "ContractOwner")]
    pub fn contract_owner() -> TokenDistributionRecipientWasm {
        TokenDistributionRecipientWasm(TokenDistributionRecipient::ContractOwner)
    }

    #[wasm_bindgen(js_name = "Identity")]
    pub fn identity(js_identity_id: &JsValue) -> Result<TokenDistributionRecipientWasm, JsValue> {
        let identity_id = IdentifierWasm::try_from(js_identity_id)?;

        Ok(TokenDistributionRecipientWasm(
            TokenDistributionRecipient::Identity(identity_id.into()),
        ))
    }

    #[wasm_bindgen(js_name = "EvonodesByParticipation")]
    pub fn evonodes_by_participation() -> TokenDistributionRecipientWasm {
        TokenDistributionRecipientWasm(TokenDistributionRecipient::EvonodesByParticipation)
    }

    #[wasm_bindgen(js_name = "getType")]
    pub fn get_type(&self) -> String {
        match self.0 {
            TokenDistributionRecipient::EvonodesByParticipation => {
                String::from("EvonodesByParticipation")
            }
            TokenDistributionRecipient::ContractOwner => String::from("ContractOwner"),
            TokenDistributionRecipient::Identity(identity) => String::from(format!(
                "Identity({})",
                IdentifierWasm::from(identity).get_base58()
            )),
        }
    }

    #[wasm_bindgen(js_name = "getValue")]
    pub fn get_value(&self) -> JsValue {
        match self.0 {
            TokenDistributionRecipient::EvonodesByParticipation => JsValue::undefined(),
            TokenDistributionRecipient::ContractOwner => JsValue::undefined(),
            TokenDistributionRecipient::Identity(identifier) => {
                IdentifierWasm::from(identifier).into()
            }
        }
    }
}
