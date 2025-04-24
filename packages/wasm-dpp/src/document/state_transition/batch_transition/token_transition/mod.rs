pub mod burn;
mod claim;
pub mod config;
pub mod destroy;
pub mod direct_purchase;
pub mod emergency_action;
pub mod freeze;
pub mod mint;
pub mod set_price_for_direct_purchase;
pub mod transfer;
pub mod unfreeze;

use crate::batch_transition::token_transition::burn::TokenBurnTransitionWasm;
use crate::batch_transition::token_transition::claim::TokenClaimTransitionWasm;
use crate::batch_transition::token_transition::config::TokenConfigUpdateTransitionWasm;
use crate::batch_transition::token_transition::destroy::TokenDestroyFrozenFundsTransitionWasm;
use crate::batch_transition::token_transition::direct_purchase::TokenDirectPurchaseTransitionWasm;
use crate::batch_transition::token_transition::emergency_action::TokenEmergencyActionTransitionWasm;
use crate::batch_transition::token_transition::freeze::TokenFreezeTransitionWasm;
use crate::batch_transition::token_transition::mint::TokenMintTransitionWasm;
use crate::batch_transition::token_transition::set_price_for_direct_purchase::TokenSetPriceForDirectPurchaseTransitionWasm;
use crate::batch_transition::token_transition::transfer::TokenTransferTransitionWasm;
use crate::batch_transition::token_transition::unfreeze::TokenUnfreezeTransitionWasm;
use crate::identifier::IdentifierWrapper;
use dpp::prelude::IdentityNonce;
use dpp::state_transition::batch_transition::batched_transition::token_transition::{
    TokenTransition, TokenTransitionV0Methods,
};
use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use js_sys::Number;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen]
pub enum TokenTransitionType {
    Burn,
    Mint,
    Transfer,
    Freeze,
    Unfreeze,
    DestroyFrozenFunds,
    Claim,
    EmergencyAction,
    ConfigUpdate,
    DirectPurchase,
    SetPriceForDirectPurchase,
}

impl From<&TokenTransition> for TokenTransitionType {
    fn from(t: &TokenTransition) -> Self {
        match t {
            TokenTransition::Burn(_) => TokenTransitionType::Burn,
            TokenTransition::Mint(_) => TokenTransitionType::Mint,
            TokenTransition::Transfer(_) => TokenTransitionType::Transfer,
            TokenTransition::Freeze(_) => TokenTransitionType::Freeze,
            TokenTransition::Unfreeze(_) => TokenTransitionType::Unfreeze,
            TokenTransition::DestroyFrozenFunds(_) => TokenTransitionType::DestroyFrozenFunds,
            TokenTransition::EmergencyAction(_) => TokenTransitionType::EmergencyAction,
            TokenTransition::ConfigUpdate(_) => TokenTransitionType::ConfigUpdate,
            TokenTransition::Claim(_) => TokenTransitionType::Claim,
            TokenTransition::DirectPurchase(_) => TokenTransitionType::DirectPurchase,
            TokenTransition::SetPriceForDirectPurchase(_) => {
                TokenTransitionType::SetPriceForDirectPurchase
            }
        }
    }
}

#[wasm_bindgen(js_name=TokenTransition)]
#[derive(Debug, Clone)]
pub struct TokenTransitionWasm(TokenTransition);

impl From<TokenTransition> for TokenTransitionWasm {
    fn from(t: TokenTransition) -> Self {
        TokenTransitionWasm(t)
    }
}

impl From<TokenTransitionWasm> for TokenTransition {
    fn from(t: TokenTransitionWasm) -> Self {
        t.0
    }
}

#[wasm_bindgen(js_class = TokenTransition)]
impl TokenTransitionWasm {
    #[wasm_bindgen(js_name=getTransitionType)]
    pub fn transition_type(&self) -> TokenTransitionType {
        TokenTransitionType::from(&self.0)
    }

    #[wasm_bindgen(js_name=getTokenId)]
    pub fn token_id(&self) -> IdentifierWrapper {
        self.0.base().token_id().into()
    }

    #[wasm_bindgen(js_name=getTokenContractPosition)]
    pub fn token_contract_position(&self) -> Number {
        self.0.base().token_contract_position().into()
    }

    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn data_contract_id(&self) -> IdentifierWrapper {
        self.0.base().data_contract_id().into()
    }

    #[wasm_bindgen(js_name=getHistoricalDocumentTypeName)]
    pub fn historical_document_type_name(&self) -> String {
        self.0.historical_document_type_name().to_string()
    }

    #[wasm_bindgen(js_name=getHistoricalDocumentId)]
    pub fn historical_document_id(&self, owner_id: IdentifierWrapper) -> IdentifierWrapper {
        self.0.historical_document_id(owner_id.into()).into()
    }

    #[wasm_bindgen(js_name=getIdentityContractNonce)]
    pub fn identity_contract_nonce(&self) -> IdentityNonce {
        self.0.base().identity_contract_nonce()
    }

    #[wasm_bindgen(js_name=toTransition)]
    pub fn to_transition(&self) -> JsValue {
        match &self.0 {
            TokenTransition::Burn(burn) => TokenBurnTransitionWasm::from(burn.clone()).into(),
            TokenTransition::Mint(mint) => TokenMintTransitionWasm::from(mint.clone()).into(),
            TokenTransition::Transfer(transfer) => {
                TokenTransferTransitionWasm::from(transfer.clone()).into()
            }
            TokenTransition::Freeze(freeze) => {
                TokenFreezeTransitionWasm::from(freeze.clone()).into()
            }
            TokenTransition::Unfreeze(unfreeze) => {
                TokenUnfreezeTransitionWasm::from(unfreeze.clone()).into()
            }
            TokenTransition::DestroyFrozenFunds(destroy_frozen_funds) => {
                TokenDestroyFrozenFundsTransitionWasm::from(destroy_frozen_funds.clone()).into()
            }
            TokenTransition::EmergencyAction(emergency_action) => {
                TokenEmergencyActionTransitionWasm::from(emergency_action.clone()).into()
            }
            TokenTransition::ConfigUpdate(config_update) => {
                TokenConfigUpdateTransitionWasm::from(config_update.clone()).into()
            }
            TokenTransition::Claim(claim) => TokenClaimTransitionWasm::from(claim.clone()).into(),
            TokenTransition::DirectPurchase(direct_purchase) => {
                TokenDirectPurchaseTransitionWasm::from(direct_purchase.clone()).into()
            }
            TokenTransition::SetPriceForDirectPurchase(set_price) => {
                TokenSetPriceForDirectPurchaseTransitionWasm::from(set_price.clone()).into()
            }
        }
    }
}
