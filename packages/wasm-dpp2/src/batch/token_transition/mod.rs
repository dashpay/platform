use crate::batch::token_transitions::config_update::TokenConfigUpdateTransitionWASM;
use crate::batch::token_transitions::direct_purchase::TokenDirectPurchaseTransitionWASM;
use crate::batch::token_transitions::set_price_for_direct_purchase::TokenSetPriceForDirectPurchaseTransitionWASM;
use crate::batch::token_transitions::token_burn::TokenBurnTransitionWASM;
use crate::batch::token_transitions::token_claim::TokenClaimTransitionWASM;
use crate::batch::token_transitions::token_destroy_frozen_funds::TokenDestroyFrozenFundsTransitionWASM;
use crate::batch::token_transitions::token_emergency_action::TokenEmergencyActionTransitionWASM;
use crate::batch::token_transitions::token_freeze::TokenFreezeTransitionWASM;
use crate::batch::token_transitions::token_mint::TokenMintTransitionWASM;
use crate::batch::token_transitions::token_transfer::TokenTransferTransitionWASM;
use crate::batch::token_transitions::token_unfreeze::TokenUnFreezeTransitionWASM;
use crate::identifier::IdentifierWASM;
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
#[wasm_bindgen(js_name=TokenTransitionWASM)]
pub struct TokenTransitionWASM(TokenTransition);

impl From<TokenTransition> for TokenTransitionWASM {
    fn from(transition: TokenTransition) -> Self {
        Self(transition)
    }
}

impl From<TokenTransitionWASM> for TokenTransition {
    fn from(transition: TokenTransitionWASM) -> Self {
        transition.0
    }
}

#[wasm_bindgen]
impl TokenTransitionWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenTransitionWASM".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenTransitionWASM".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(js_transition: &JsValue) -> Result<TokenTransitionWASM, JsValue> {
        let transition = match js_transition.is_object() {
            true => match get_class_type(js_transition)?.as_str() {
                "TokenMintTransitionWASM" => Ok(TokenTransition::from(TokenMintTransition::from(
                    js_transition
                        .to_wasm::<TokenMintTransitionWASM>("TokenMintTransitionWASM")?
                        .clone(),
                ))),
                "TokenUnFreezeTransitionWASM" => {
                    Ok(TokenTransition::from(TokenUnfreezeTransition::from(
                        js_transition
                            .to_wasm::<TokenUnFreezeTransitionWASM>("TokenUnFreezeTransitionWASM")?
                            .clone(),
                    )))
                }
                "TokenTransferTransitionWASM" => {
                    Ok(TokenTransition::from(TokenTransferTransition::from(
                        js_transition
                            .to_wasm::<TokenTransferTransitionWASM>("TokenTransferTransitionWASM")?
                            .clone(),
                    )))
                }
                "TokenFreezeTransitionWASM" => {
                    Ok(TokenTransition::from(TokenFreezeTransition::from(
                        js_transition
                            .to_wasm::<TokenFreezeTransitionWASM>("TokenFreezeTransitionWASM")?
                            .clone(),
                    )))
                }
                "TokenDestroyFrozenFundsTransitionWASM" => Ok(TokenTransition::from(
                    TokenDestroyFrozenFundsTransition::from(
                        js_transition
                            .to_wasm::<TokenDestroyFrozenFundsTransitionWASM>(
                                "TokenDestroyFrozenFundsTransitionWASM",
                            )?
                            .clone(),
                    ),
                )),
                "TokenClaimTransitionWASM" => {
                    Ok(TokenTransition::from(TokenClaimTransition::from(
                        js_transition
                            .to_wasm::<TokenClaimTransitionWASM>("TokenClaimTransitionWASM")?
                            .clone(),
                    )))
                }
                "TokenBurnTransitionWASM" => Ok(TokenTransition::from(TokenBurnTransition::from(
                    js_transition
                        .to_wasm::<TokenBurnTransitionWASM>("TokenBurnTransitionWASM")?
                        .clone(),
                ))),
                "TokenSetPriceForDirectPurchaseTransitionWASM" => Ok(TokenTransition::from(
                    TokenSetPriceForDirectPurchaseTransition::from(
                        js_transition
                            .to_wasm::<TokenSetPriceForDirectPurchaseTransitionWASM>(
                                "TokenSetPriceForDirectPurchaseTransitionWASM",
                            )?
                            .clone(),
                    ),
                )),
                "TokenDirectPurchaseTransitionWASM" => {
                    Ok(TokenTransition::from(TokenDirectPurchaseTransition::from(
                        js_transition
                            .to_wasm::<TokenDirectPurchaseTransitionWASM>(
                                "TokenDirectPurchaseTransitionWASM",
                            )?
                            .clone(),
                    )))
                }
                "TokenConfigUpdateTransitionWASM" => {
                    Ok(TokenTransition::from(TokenConfigUpdateTransition::from(
                        js_transition
                            .to_wasm::<TokenConfigUpdateTransitionWASM>(
                                "TokenConfigUpdateTransitionWASM",
                            )?
                            .clone(),
                    )))
                }
                "TokenEmergencyActionTransitionWASM" => {
                    Ok(TokenTransition::from(TokenEmergencyActionTransition::from(
                        js_transition
                            .to_wasm::<TokenEmergencyActionTransitionWASM>(
                                "TokenEmergencyActionTransitionWASM",
                            )?
                            .clone(),
                    )))
                }
                _ => Err(JsValue::from("Bad token transition input")),
            },
            false => Err(JsValue::from("Bad token transition input")),
        }?;

        Ok(TokenTransitionWASM(TokenTransition::from(transition)))
    }

    #[wasm_bindgen(js_name = "getTransition")]
    pub fn to_transition(&self) -> JsValue {
        match self.clone().0 {
            TokenTransition::Burn(token_transition) => {
                TokenBurnTransitionWASM::from(token_transition).into()
            }
            TokenTransition::Mint(token_transition) => {
                TokenMintTransitionWASM::from(token_transition).into()
            }
            TokenTransition::Transfer(token_transition) => {
                TokenTransferTransitionWASM::from(token_transition).into()
            }
            TokenTransition::Freeze(token_transition) => {
                TokenFreezeTransitionWASM::from(token_transition).into()
            }
            TokenTransition::Unfreeze(token_transition) => {
                TokenUnFreezeTransitionWASM::from(token_transition).into()
            }
            TokenTransition::DestroyFrozenFunds(token_transition) => {
                TokenDestroyFrozenFundsTransitionWASM::from(token_transition).into()
            }
            TokenTransition::Claim(token_transition) => {
                TokenClaimTransitionWASM::from(token_transition).into()
            }
            TokenTransition::EmergencyAction(token_transition) => {
                TokenEmergencyActionTransitionWASM::from(token_transition).into()
            }
            TokenTransition::ConfigUpdate(token_transition) => {
                TokenConfigUpdateTransitionWASM::from(token_transition).into()
            }
            TokenTransition::DirectPurchase(token_transition) => {
                TokenDirectPurchaseTransitionWASM::from(token_transition).into()
            }
            TokenTransition::SetPriceForDirectPurchase(token_transition) => {
                TokenSetPriceForDirectPurchaseTransitionWASM::from(token_transition).into()
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
    pub fn get_historical_document_id(
        &self,
        js_owner: &JsValue,
    ) -> Result<IdentifierWASM, JsValue> {
        let owner = IdentifierWASM::try_from(js_owner)?;
        Ok(self.0.historical_document_id(owner.into()).into())
    }

    #[wasm_bindgen(getter = "identityContractNonce")]
    pub fn get_identity_contract_nonce(&self) -> IdentityNonce {
        self.0.identity_contract_nonce()
    }

    #[wasm_bindgen(getter = "tokenId")]
    pub fn get_token_id(&self) -> IdentifierWASM {
        self.0.token_id().into()
    }

    #[wasm_bindgen(getter = "contractId")]
    pub fn get_contract_id(&self) -> IdentifierWASM {
        self.0.data_contract_id().into()
    }

    #[wasm_bindgen(setter = "identityContractNonce")]
    pub fn set_identity_contract_nonce(&mut self, nonce: IdentityNonce) {
        self.0.set_identity_contract_nonce(nonce)
    }

    #[wasm_bindgen(setter = "tokenId")]
    pub fn set_token_id(&mut self, js_id: &JsValue) -> Result<(), JsValue> {
        let id = IdentifierWASM::try_from(js_id)?;

        self.0.set_token_id(id.into());

        Ok(())
    }

    #[wasm_bindgen(setter = "contractId")]
    pub fn set_contract_id(&mut self, js_id: &JsValue) -> Result<(), JsValue> {
        let id = IdentifierWASM::try_from(js_id)?;

        self.0.set_data_contract_id(id.into());

        Ok(())
    }
}
