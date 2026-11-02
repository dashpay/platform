use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_from_for_extern_type;
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::token_transitions::config_update::TokenConfigUpdateTransitionWasm;
use crate::state_transitions::batch::token_transitions::direct_purchase::TokenDirectPurchaseTransitionWasm;
use crate::state_transitions::batch::token_transitions::set_price_for_direct_purchase::TokenSetPriceForDirectPurchaseTransitionWasm;
use crate::state_transitions::batch::token_transitions::token_burn::TokenBurnTransitionWasm;
use crate::state_transitions::batch::token_transitions::token_burn_from_pool::TokenBurnFromPoolTransitionWasm;
use crate::state_transitions::batch::token_transitions::token_claim::TokenClaimTransitionWasm;
use crate::state_transitions::batch::token_transitions::token_claim_to_pool::TokenClaimToPoolTransitionWasm;
use crate::state_transitions::batch::token_transitions::token_destroy_frozen_funds::TokenDestroyFrozenFundsTransitionWasm;
use crate::state_transitions::batch::token_transitions::token_direct_purchase_to_pool::TokenDirectPurchaseToPoolTransitionWasm;
use crate::state_transitions::batch::token_transitions::token_emergency_action::TokenEmergencyActionTransitionWasm;
use crate::state_transitions::batch::token_transitions::token_freeze::TokenFreezeTransitionWasm;
use crate::state_transitions::batch::token_transitions::token_mint::TokenMintTransitionWasm;
use crate::state_transitions::batch::token_transitions::token_mint_to_pool::TokenMintToPoolTransitionWasm;
use crate::state_transitions::batch::token_transitions::token_shield::TokenShieldTransitionWasm;
use crate::state_transitions::batch::token_transitions::token_shielded_transfer::TokenShieldedTransferTransitionWasm;
use crate::state_transitions::batch::token_transitions::token_transfer::TokenTransferTransitionWasm;
use crate::state_transitions::batch::token_transitions::token_unfreeze::TokenUnFreezeTransitionWasm;
use crate::state_transitions::batch::token_transitions::token_unshield::TokenUnshieldTransitionWasm;
use crate::utils::get_class_type;
use dpp::prelude::{Identifier, IdentityNonce};
use dpp::state_transition::batch_transition::batched_transition::token_transition::{
    TokenTransition, TokenTransitionV0Methods,
};
use dpp::state_transition::batch_transition::{
    TokenBurnFromPoolTransition, TokenBurnTransition, TokenClaimToPoolTransition,
    TokenClaimTransition, TokenConfigUpdateTransition, TokenDestroyFrozenFundsTransition,
    TokenDirectPurchaseToPoolTransition, TokenDirectPurchaseTransition,
    TokenEmergencyActionTransition, TokenFreezeTransition, TokenMintToPoolTransition,
    TokenMintTransition, TokenSetPriceForDirectPurchaseTransition, TokenShieldTransition,
    TokenShieldedTransferTransition, TokenTransferTransition, TokenUnfreezeTransition,
    TokenUnshieldTransition,
};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TOKEN_TRANSITION_TYPES_TS: &str = r#"
export type TokenTransitionLike = TokenMintTransition | TokenBurnTransition | TokenTransferTransition | TokenFreezeTransition | TokenUnFreezeTransition | TokenDestroyFrozenFundsTransition | TokenClaimTransition | TokenEmergencyActionTransition | TokenConfigUpdateTransition | TokenDirectPurchaseTransition | TokenSetPriceForDirectPurchaseTransition | TokenShieldTransition | TokenUnshieldTransition | TokenShieldedTransferTransition | TokenMintToPoolTransition | TokenBurnFromPoolTransition | TokenClaimToPoolTransition | TokenDirectPurchaseToPoolTransition;
"#;

/// Extern type for flexible TokenTransition input
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenTransitionLike")]
    pub type TokenTransitionLikeJs;
}

impl_from_for_extern_type!(
    TokenTransitionLikeJs,
    TokenMintTransitionWasm,
    TokenBurnTransitionWasm,
    TokenTransferTransitionWasm,
    TokenFreezeTransitionWasm,
    TokenUnFreezeTransitionWasm,
    TokenDestroyFrozenFundsTransitionWasm,
    TokenClaimTransitionWasm,
    TokenEmergencyActionTransitionWasm,
    TokenConfigUpdateTransitionWasm,
    TokenDirectPurchaseTransitionWasm,
    TokenSetPriceForDirectPurchaseTransitionWasm,
    TokenShieldTransitionWasm,
    TokenUnshieldTransitionWasm,
    TokenShieldedTransferTransitionWasm,
    TokenMintToPoolTransitionWasm,
    TokenBurnFromPoolTransitionWasm,
    TokenClaimToPoolTransitionWasm,
    TokenDirectPurchaseToPoolTransitionWasm,
);

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "TokenTransition")]
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
    #[wasm_bindgen(constructor)]
    pub fn constructor(transition: TokenTransitionLikeJs) -> WasmDppResult<TokenTransitionWasm> {
        let transition: JsValue = transition.into();
        if !transition.is_object() {
            return Err(WasmDppError::invalid_argument("Bad token transition input"));
        }

        let transition = match get_class_type(&transition)?.as_str() {
            "TokenMintTransition" => TokenTransition::from(TokenMintTransition::from(
                TokenMintTransitionWasm::try_from(&transition)?,
            )),
            "TokenUnFreezeTransition" => TokenTransition::from(TokenUnfreezeTransition::from(
                TokenUnFreezeTransitionWasm::try_from(&transition)?,
            )),
            "TokenTransferTransition" => TokenTransition::from(TokenTransferTransition::from(
                TokenTransferTransitionWasm::try_from(&transition)?,
            )),
            "TokenFreezeTransition" => TokenTransition::from(TokenFreezeTransition::from(
                TokenFreezeTransitionWasm::try_from(&transition)?,
            )),
            "TokenDestroyFrozenFundsTransition" => {
                TokenTransition::from(TokenDestroyFrozenFundsTransition::from(
                    TokenDestroyFrozenFundsTransitionWasm::try_from(&transition)?,
                ))
            }
            "TokenClaimTransition" => TokenTransition::from(TokenClaimTransition::from(
                TokenClaimTransitionWasm::try_from(&transition)?,
            )),
            "TokenBurnTransition" => TokenTransition::from(TokenBurnTransition::from(
                TokenBurnTransitionWasm::try_from(&transition)?,
            )),
            "TokenSetPriceForDirectPurchaseTransition" => {
                TokenTransition::from(TokenSetPriceForDirectPurchaseTransition::from(
                    TokenSetPriceForDirectPurchaseTransitionWasm::try_from(&transition)?,
                ))
            }
            "TokenDirectPurchaseTransition" => {
                TokenTransition::from(TokenDirectPurchaseTransition::from(
                    TokenDirectPurchaseTransitionWasm::try_from(&transition)?,
                ))
            }
            "TokenConfigUpdateTransition" => {
                TokenTransition::from(TokenConfigUpdateTransition::from(
                    TokenConfigUpdateTransitionWasm::try_from(&transition)?,
                ))
            }
            "TokenEmergencyActionTransition" => {
                TokenTransition::from(TokenEmergencyActionTransition::from(
                    TokenEmergencyActionTransitionWasm::try_from(&transition)?,
                ))
            }
            "TokenShieldTransition" => TokenTransition::from(TokenShieldTransition::from(
                TokenShieldTransitionWasm::try_from(&transition)?,
            )),
            "TokenUnshieldTransition" => TokenTransition::from(TokenUnshieldTransition::from(
                TokenUnshieldTransitionWasm::try_from(&transition)?,
            )),
            "TokenShieldedTransferTransition" => {
                TokenTransition::from(TokenShieldedTransferTransition::from(
                    TokenShieldedTransferTransitionWasm::try_from(&transition)?,
                ))
            }
            "TokenMintToPoolTransition" => TokenTransition::from(TokenMintToPoolTransition::from(
                TokenMintToPoolTransitionWasm::try_from(&transition)?,
            )),
            "TokenBurnFromPoolTransition" => {
                TokenTransition::from(TokenBurnFromPoolTransition::from(
                    TokenBurnFromPoolTransitionWasm::try_from(&transition)?,
                ))
            }
            "TokenClaimToPoolTransition" => {
                TokenTransition::from(TokenClaimToPoolTransition::from(
                    TokenClaimToPoolTransitionWasm::try_from(&transition)?,
                ))
            }
            "TokenDirectPurchaseToPoolTransition" => {
                TokenTransition::from(TokenDirectPurchaseToPoolTransition::from(
                    TokenDirectPurchaseToPoolTransitionWasm::try_from(&transition)?,
                ))
            }
            _ => {
                return Err(WasmDppError::invalid_argument("Bad token transition input"));
            }
        };

        Ok(TokenTransitionWasm(transition))
    }

    #[wasm_bindgen(getter = "transition")]
    pub fn transition(&self) -> TokenTransitionLikeJs {
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
            TokenTransition::Shield(token_transition) => {
                TokenShieldTransitionWasm::from(token_transition).into()
            }
            TokenTransition::Unshield(token_transition) => {
                TokenUnshieldTransitionWasm::from(token_transition).into()
            }
            TokenTransition::ShieldedTransfer(token_transition) => {
                TokenShieldedTransferTransitionWasm::from(token_transition).into()
            }
            TokenTransition::MintToPool(token_transition) => {
                TokenMintToPoolTransitionWasm::from(token_transition).into()
            }
            TokenTransition::BurnFromPool(token_transition) => {
                TokenBurnFromPoolTransitionWasm::from(token_transition).into()
            }
            TokenTransition::ClaimToPool(token_transition) => {
                TokenClaimToPoolTransitionWasm::from(token_transition).into()
            }
            TokenTransition::DirectPurchaseToPool(token_transition) => {
                TokenDirectPurchaseToPoolTransitionWasm::from(token_transition).into()
            }
        }
    }

    #[wasm_bindgen(getter = "transitionTypeNumber")]
    pub fn transition_type_number(&self) -> u8 {
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
            TokenTransition::Shield(_) => 11,
            TokenTransition::Unshield(_) => 12,
            TokenTransition::ShieldedTransfer(_) => 13,
            TokenTransition::MintToPool(_) => 14,
            TokenTransition::BurnFromPool(_) => 15,
            TokenTransition::ClaimToPool(_) => 16,
            TokenTransition::DirectPurchaseToPool(_) => 17,
        }
    }

    #[wasm_bindgen(getter = "transitionType")]
    pub fn transition_type(&self) -> String {
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
            TokenTransition::Shield(_) => "Shield".to_string(),
            TokenTransition::Unshield(_) => "Unshield".to_string(),
            TokenTransition::ShieldedTransfer(_) => "ShieldedTransfer".to_string(),
            TokenTransition::MintToPool(_) => "MintToPool".to_string(),
            TokenTransition::BurnFromPool(_) => "BurnFromPool".to_string(),
            TokenTransition::ClaimToPool(_) => "ClaimToPool".to_string(),
            TokenTransition::DirectPurchaseToPool(_) => "DirectPurchaseToPool".to_string(),
        }
    }

    #[wasm_bindgen(getter = "historicalDocumentTypeName")]
    pub fn historical_document_type_name(&self) -> String {
        self.0.historical_document_type_name().to_string()
    }

    #[wasm_bindgen(js_name = "getHistoricalDocumentId")]
    pub fn get_historical_document_id(
        &self,
        owner: IdentifierLikeJs,
    ) -> WasmDppResult<IdentifierWasm> {
        let owner: Identifier = owner.try_into()?;
        Ok(self.0.historical_document_id(owner).into())
    }

    #[wasm_bindgen(getter = "identityContractNonce")]
    pub fn identity_contract_nonce(&self) -> IdentityNonce {
        self.0.identity_contract_nonce()
    }

    #[wasm_bindgen(getter = "tokenId")]
    pub fn token_id(&self) -> IdentifierWasm {
        self.0.token_id().into()
    }

    #[wasm_bindgen(getter = "contractId")]
    pub fn contract_id(&self) -> IdentifierWasm {
        self.0.data_contract_id().into()
    }

    #[wasm_bindgen(setter = "identityContractNonce")]
    pub fn set_identity_contract_nonce(&mut self, nonce: &js_sys::BigInt) -> WasmDppResult<()> {
        use crate::utils::try_to_u64;
        self.0
            .set_identity_contract_nonce(try_to_u64(nonce, "identityContractNonce")?);
        Ok(())
    }

    #[wasm_bindgen(setter = "tokenId")]
    pub fn set_token_id(
        &mut self,
        #[wasm_bindgen(js_name = "tokenId")] token_id: IdentifierLikeJs,
    ) -> WasmDppResult<()> {
        let token_id: Identifier = token_id.try_into()?;

        self.0.set_token_id(token_id);

        Ok(())
    }

    #[wasm_bindgen(setter = "contractId")]
    pub fn set_contract_id(
        &mut self,
        #[wasm_bindgen(js_name = "contractId")] contract_id: IdentifierLikeJs,
    ) -> WasmDppResult<()> {
        let contract_id: Identifier = contract_id.try_into()?;

        self.0.set_data_contract_id(contract_id);

        Ok(())
    }
}

impl_try_from_js_value!(TokenTransitionWasm, "TokenTransition");
impl_wasm_type_info!(TokenTransitionWasm, TokenTransition);
