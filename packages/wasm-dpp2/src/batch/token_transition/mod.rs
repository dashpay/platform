use crate::batch::token_transitions::config_update::TokenConfigUpdateTransitionWasm;
use crate::batch::token_transitions::direct_purchase::TokenDirectPurchaseTransitionWasm;
use crate::batch::token_transitions::set_price_for_direct_purchase::TokenSetPriceForDirectPurchaseTransitionWasm;
use crate::batch::token_transitions::token_burn::TokenBurnTransitionWasm;
use crate::batch::token_transitions::token_claim::TokenClaimTransitionWasm;
use crate::batch::token_transitions::token_destroy_frozen_funds::TokenDestroyFrozenFundsTransitionWasm;
use crate::batch::token_transitions::token_emergency_action::TokenEmergencyActionTransitionWasm;
use crate::batch::token_transitions::token_freeze::TokenFreezeTransitionWasm;
use crate::batch::token_transitions::token_mint::TokenMintTransitionWasm;
use crate::batch::token_transitions::token_transfer::TokenTransferTransitionWasm;
use crate::batch::token_transitions::token_unfreeze::TokenUnFreezeTransitionWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::utils::{IntoWasm, get_class_type};
use dpp::prelude::IdentityNonce;
use dpp::state_transition::batch_transition::batched_transition::token_transition::{
    TokenTransition, TokenTransitionV0Methods,
};
use dpp::state_transition::batch_transition::{
    TokenBurnTransition, TokenClaimTransition, TokenConfigUpdateTransition,
    TokenDestroyFrozenFundsTransition, TokenDirectPurchaseTransition,
    TokenEmergencyActionTransition, TokenFreezeTransition, TokenMintTransition,
    TokenSetPriceForDirectPurchaseTransition, TokenTransferTransition, TokenUnfreezeTransition,
};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name=TokenTransition)]
pub struct TokenTransitionWasm(TokenTransition);

impl From<TokenTransition> for TokenTransitionWasm {
    fn from(transition: TokenTransition) -> Self {
        Self(transition)
    }
}

impl From<TokenTransitionWasm> for TokenTransition {
    fn from(transition: TokenTransitionWasm) -> Self {
        transition.0
    }
}

#[wasm_bindgen(js_class = TokenTransition)]
impl TokenTransitionWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(js_transition: &JsValue) -> WasmDppResult<TokenTransitionWasm> {
        if !js_transition.is_object() {
            return Err(WasmDppError::invalid_argument("Bad token transition input"));
        }

        let transition = match get_class_type(js_transition)?.as_str() {
            "TokenMintTransition" => TokenTransition::from(TokenMintTransition::from(
                js_transition
                    .to_wasm::<TokenMintTransitionWasm>("TokenMintTransition")?
                    .clone(),
            )),
            "TokenUnFreezeTransition" => TokenTransition::from(TokenUnfreezeTransition::from(
                js_transition
                    .to_wasm::<TokenUnFreezeTransitionWasm>("TokenUnFreezeTransition")?
                    .clone(),
            )),
            "TokenTransferTransition" => TokenTransition::from(TokenTransferTransition::from(
                js_transition
                    .to_wasm::<TokenTransferTransitionWasm>("TokenTransferTransition")?
                    .clone(),
            )),
            "TokenFreezeTransition" => TokenTransition::from(TokenFreezeTransition::from(
                js_transition
                    .to_wasm::<TokenFreezeTransitionWasm>("TokenFreezeTransition")?
                    .clone(),
            )),
            "TokenDestroyFrozenFundsTransition" => {
                TokenTransition::from(TokenDestroyFrozenFundsTransition::from(
                    js_transition
                        .to_wasm::<TokenDestroyFrozenFundsTransitionWasm>(
                            "TokenDestroyFrozenFundsTransition",
                        )?
                        .clone(),
                ))
            }
            "TokenClaimTransition" => TokenTransition::from(TokenClaimTransition::from(
                js_transition
                    .to_wasm::<TokenClaimTransitionWasm>("TokenClaimTransition")?
                    .clone(),
            )),
            "TokenBurnTransition" => TokenTransition::from(TokenBurnTransition::from(
                js_transition
                    .to_wasm::<TokenBurnTransitionWasm>("TokenBurnTransition")?
                    .clone(),
            )),
            "TokenSetPriceForDirectPurchaseTransition" => {
                TokenTransition::from(TokenSetPriceForDirectPurchaseTransition::from(
                    js_transition
                        .to_wasm::<TokenSetPriceForDirectPurchaseTransitionWasm>(
                            "TokenSetPriceForDirectPurchaseTransition",
                        )?
                        .clone(),
                ))
            }
            "TokenDirectPurchaseTransition" => {
                TokenTransition::from(TokenDirectPurchaseTransition::from(
                    js_transition
                        .to_wasm::<TokenDirectPurchaseTransitionWasm>(
                            "TokenDirectPurchaseTransition",
                        )?
                        .clone(),
                ))
            }
            "TokenConfigUpdateTransition" => {
                TokenTransition::from(TokenConfigUpdateTransition::from(
                    js_transition
                        .to_wasm::<TokenConfigUpdateTransitionWasm>("TokenConfigUpdateTransition")?
                        .clone(),
                ))
            }
            "TokenEmergencyActionTransition" => {
                TokenTransition::from(TokenEmergencyActionTransition::from(
                    js_transition
                        .to_wasm::<TokenEmergencyActionTransitionWasm>(
                            "TokenEmergencyActionTransition",
                        )?
                        .clone(),
                ))
            }
            _ => {
                return Err(WasmDppError::invalid_argument("Bad token transition input"));
            }
        };

        Ok(TokenTransitionWasm(transition))
    }

    #[wasm_bindgen(js_name = "getTransition")]
    pub fn to_transition(&self) -> JsValue {
        match self.clone().0 {
            TokenTransition::Burn(token_transition) => {
                TokenBurnTransitionWasm::from(token_transition).into()
            }
            TokenTransition::Mint(token_transition) => {
                TokenMintTransitionWasm::from(token_transition).into()
            }
            TokenTransition::Transfer(token_transition) => {
                TokenTransferTransitionWasm::from(token_transition).into()
            }
            TokenTransition::Freeze(token_transition) => {
                TokenFreezeTransitionWasm::from(token_transition).into()
            }
            TokenTransition::Unfreeze(token_transition) => {
                TokenUnFreezeTransitionWasm::from(token_transition).into()
            }
            TokenTransition::DestroyFrozenFunds(token_transition) => {
                TokenDestroyFrozenFundsTransitionWasm::from(token_transition).into()
            }
            TokenTransition::Claim(token_transition) => {
                TokenClaimTransitionWasm::from(token_transition).into()
            }
            TokenTransition::EmergencyAction(token_transition) => {
                TokenEmergencyActionTransitionWasm::from(token_transition).into()
            }
            TokenTransition::ConfigUpdate(token_transition) => {
                TokenConfigUpdateTransitionWasm::from(token_transition).into()
            }
            TokenTransition::DirectPurchase(token_transition) => {
                TokenDirectPurchaseTransitionWasm::from(token_transition).into()
            }
            TokenTransition::SetPriceForDirectPurchase(token_transition) => {
                TokenSetPriceForDirectPurchaseTransitionWasm::from(token_transition).into()
            }
        }
    }

    #[wasm_bindgen(js_name = "getTransitionTypeNumber")]
    pub fn get_transition_type_number(&self) -> u8 {
        match self.clone().0 {
            TokenTransition::Burn(_) => 0,
            TokenTransition::Mint(_) => 1,
            TokenTransition::Transfer(_) => 2,
            TokenTransition::Freeze(_) => 3,
            TokenTransition::Unfreeze(_) => 4,
            TokenTransition::DestroyFrozenFunds(_) => 5,
            TokenTransition::Claim(_) => 6,
            TokenTransition::EmergencyAction(_) => 7,
            TokenTransition::ConfigUpdate(_) => 8,
            TokenTransition::DirectPurchase(_) => 9,
            TokenTransition::SetPriceForDirectPurchase(_) => 10,
        }
    }

    #[wasm_bindgen(js_name = "getTransitionType")]
    pub fn get_transition_type(&self) -> String {
        match self.clone().0 {
            TokenTransition::Burn(_) => "Burn".to_string(),
            TokenTransition::Mint(_) => "Mint".to_string(),
            TokenTransition::Transfer(_) => "Transfer".to_string(),
            TokenTransition::Freeze(_) => "Freeze".to_string(),
            TokenTransition::Unfreeze(_) => "Unfreeze".to_string(),
            TokenTransition::DestroyFrozenFunds(_) => "DestroyFrozenFunds".to_string(),
            TokenTransition::Claim(_) => "Claim".to_string(),
            TokenTransition::EmergencyAction(_) => "EmergencyAction".to_string(),
            TokenTransition::ConfigUpdate(_) => "ConfigUpdate".to_string(),
            TokenTransition::DirectPurchase(_) => "DirectPurchase".to_string(),
            TokenTransition::SetPriceForDirectPurchase(_) => {
                "SetPriceForDirectPurchase".to_string()
            }
        }
    }

    #[wasm_bindgen(js_name = "getHistoricalDocumentTypeName")]
    pub fn get_historical_document_type_name(&self) -> String {
        self.0.historical_document_type_name().to_string()
    }

    #[wasm_bindgen(js_name = "getHistoricalDocumentId")]
    pub fn get_historical_document_id(&self, js_owner: &JsValue) -> WasmDppResult<IdentifierWasm> {
        let owner = IdentifierWasm::try_from(js_owner)?;
        Ok(self.0.historical_document_id(owner.into()).into())
    }

    #[wasm_bindgen(getter = "identityContractNonce")]
    pub fn get_identity_contract_nonce(&self) -> IdentityNonce {
        self.0.identity_contract_nonce()
    }

    #[wasm_bindgen(getter = "tokenId")]
    pub fn get_token_id(&self) -> IdentifierWasm {
        self.0.token_id().into()
    }

    #[wasm_bindgen(getter = "contractId")]
    pub fn get_contract_id(&self) -> IdentifierWasm {
        self.0.data_contract_id().into()
    }

    #[wasm_bindgen(setter = "identityContractNonce")]
    pub fn set_identity_contract_nonce(&mut self, nonce: IdentityNonce) {
        self.0.set_identity_contract_nonce(nonce)
    }

    #[wasm_bindgen(setter = "tokenId")]
    pub fn set_token_id(&mut self, js_id: &JsValue) -> WasmDppResult<()> {
        let id = IdentifierWasm::try_from(js_id)?;

        self.0.set_token_id(id.into());

        Ok(())
    }

    #[wasm_bindgen(setter = "contractId")]
    pub fn set_contract_id(&mut self, js_id: &JsValue) -> WasmDppResult<()> {
        let id = IdentifierWasm::try_from(js_id)?;

        self.0.set_data_contract_id(id.into());

        Ok(())
    }
}
