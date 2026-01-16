use crate::error::WasmDppResult;
use crate::impl_wasm_type_info;
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
    #[wasm_bindgen(js_name = "ContractOwner")]
    pub fn contract_owner() -> TokenDistributionRecipientWasm {
        TokenDistributionRecipientWasm(TokenDistributionRecipient::ContractOwner)
    }

    #[wasm_bindgen(js_name = "Identity")]
    pub fn identity(
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        js_identity_id: &JsValue,
    ) -> WasmDppResult<TokenDistributionRecipientWasm> {
        let identity_id = IdentifierWasm::try_from(js_identity_id)?.into();

        Ok(TokenDistributionRecipientWasm(
            TokenDistributionRecipient::Identity(identity_id),
        ))
    }

    #[wasm_bindgen(js_name = "EvonodesByParticipation")]
    pub fn evonodes_by_participation() -> TokenDistributionRecipientWasm {
        TokenDistributionRecipientWasm(TokenDistributionRecipient::EvonodesByParticipation)
    }

    #[wasm_bindgen(getter = "recipientType")]
    pub fn recipient_type(&self) -> String {
        match self.0 {
            TokenDistributionRecipient::EvonodesByParticipation => {
                String::from("EvonodesByParticipation")
            }
            TokenDistributionRecipient::ContractOwner => String::from("ContractOwner"),
            TokenDistributionRecipient::Identity(identity) => {
                format!("Identity({})", IdentifierWasm::from(identity).to_base58())
            }
        }
    }

    #[wasm_bindgen(getter = "value")]
    pub fn value(&self) -> JsValue {
        match self.0 {
            TokenDistributionRecipient::EvonodesByParticipation => JsValue::undefined(),
            TokenDistributionRecipient::ContractOwner => JsValue::undefined(),
            TokenDistributionRecipient::Identity(identifier) => {
                IdentifierWasm::from(identifier).into()
            }
        }
    }
}

impl_wasm_type_info!(TokenDistributionRecipientWasm, TokenDistributionRecipient);
