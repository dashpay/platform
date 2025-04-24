mod token_transition_action_type;

/// token_base_transition_action
pub mod token_base_transition_action;
/// token_burn_transition_action
pub mod token_burn_transition_action;
/// token_freeze_transition_action
pub mod token_freeze_transition_action;
/// token_issuance_transition_action
pub mod token_mint_transition_action;
/// token_transfer_transition_action
pub mod token_transfer_transition_action;
/// token_unfreeze_transition_action
pub mod token_unfreeze_transition_action;

/// token_config_update_transition_action
pub mod token_config_update_transition_action;
/// token_destroy_frozen_funds_transition_action
pub mod token_destroy_frozen_funds_transition_action;
/// token_emergency_action_transition_action
pub mod token_emergency_action_transition_action;

/// token_claim_transition_action
pub mod token_claim_transition_action;

/// token_direct_purchase_transition_action
pub mod token_direct_purchase_transition_action;

/// token_set_price_for_direct_purchase_transition_action
pub mod token_set_price_for_direct_purchase_transition_action;

use derive_more::From;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_keeps_history_rules::accessors::v0::TokenKeepsHistoryRulesV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::document::Document;
use dpp::identifier::Identifier;
use dpp::prelude::{DataContract, IdentityNonce};
use dpp::ProtocolError;
use platform_version::version::PlatformVersion;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_burn_transition_action::{TokenBurnTransitionAction, TokenBurnTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_config_update_transition_action::{TokenConfigUpdateTransitionAction, TokenConfigUpdateTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_freeze_transition_action::{TokenFreezeTransitionAction, TokenFreezeTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_unfreeze_transition_action::{TokenUnfreezeTransitionAction, TokenUnfreezeTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_mint_transition_action::{TokenMintTransitionAction, TokenMintTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_transfer_transition_action::TokenTransferTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_transfer_transition_action::TokenTransferTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_emergency_action_transition_action::TokenEmergencyActionTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_emergency_action_transition_action::TokenEmergencyActionTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_destroy_frozen_funds_transition_action::TokenDestroyFrozenFundsTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_destroy_frozen_funds_transition_action::TokenDestroyFrozenFundsTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_claim_transition_action::{TokenClaimTransitionAction, TokenClaimTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_direct_purchase_transition_action::{TokenDirectPurchaseTransitionAction, TokenDirectPurchaseTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_set_price_for_direct_purchase_transition_action::{TokenSetPriceForDirectPurchaseTransitionAction, TokenSetPriceForDirectPurchaseTransitionActionAccessorsV0};

/// token action
#[derive(Debug, Clone, From)]
pub enum TokenTransitionAction {
    /// burn
    BurnAction(TokenBurnTransitionAction),
    /// issuance
    MintAction(TokenMintTransitionAction),
    /// transfer
    TransferAction(TokenTransferTransitionAction),
    /// freeze
    FreezeAction(TokenFreezeTransitionAction),
    /// unfreeze
    UnfreezeAction(TokenUnfreezeTransitionAction),
    /// release
    ClaimAction(TokenClaimTransitionAction),
    /// emergency action
    EmergencyActionAction(TokenEmergencyActionTransitionAction),
    /// destroy frozen funds action
    DestroyFrozenFundsAction(TokenDestroyFrozenFundsTransitionAction),
    /// update the token configuration
    ConfigUpdateAction(TokenConfigUpdateTransitionAction),
    /// purchases the token from the contract owner
    DirectPurchaseAction(TokenDirectPurchaseTransitionAction),
    /// sets the price for direct purchase of the token
    SetPriceForDirectPurchaseAction(TokenSetPriceForDirectPurchaseTransitionAction),
}

impl TokenTransitionAction {
    /// Returns a reference to the base token transition action if available
    pub fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenTransitionAction::BurnAction(action) => action.base(),
            TokenTransitionAction::MintAction(action) => action.base(),
            TokenTransitionAction::TransferAction(action) => action.base(),
            TokenTransitionAction::FreezeAction(action) => action.base(),
            TokenTransitionAction::UnfreezeAction(action) => action.base(),
            TokenTransitionAction::ClaimAction(action) => action.base(),
            TokenTransitionAction::EmergencyActionAction(action) => action.base(),
            TokenTransitionAction::DestroyFrozenFundsAction(action) => action.base(),
            TokenTransitionAction::ConfigUpdateAction(action) => action.base(),
            TokenTransitionAction::DirectPurchaseAction(action) => action.base(),
            TokenTransitionAction::SetPriceForDirectPurchaseAction(action) => action.base(),
        }
    }

    /// Consumes self and returns the base token transition action if available
    pub fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenTransitionAction::BurnAction(action) => action.base_owned(),
            TokenTransitionAction::MintAction(action) => action.base_owned(),
            TokenTransitionAction::TransferAction(action) => action.base_owned(),
            TokenTransitionAction::FreezeAction(action) => action.base_owned(),
            TokenTransitionAction::UnfreezeAction(action) => action.base_owned(),
            TokenTransitionAction::ClaimAction(action) => action.base_owned(),
            TokenTransitionAction::EmergencyActionAction(action) => action.base_owned(),
            TokenTransitionAction::DestroyFrozenFundsAction(action) => action.base_owned(),
            TokenTransitionAction::ConfigUpdateAction(action) => action.base_owned(),
            TokenTransitionAction::DirectPurchaseAction(action) => action.base_owned(),
            TokenTransitionAction::SetPriceForDirectPurchaseAction(action) => action.base_owned(),
        }
    }

    /// Historical document type name for the token history contract
    pub fn historical_document_type_name(&self) -> &str {
        match self {
            TokenTransitionAction::BurnAction(_) => "burn",
            TokenTransitionAction::MintAction(_) => "mint",
            TokenTransitionAction::TransferAction(_) => "transfer",
            TokenTransitionAction::FreezeAction(_) => "freeze",
            TokenTransitionAction::UnfreezeAction(_) => "unfreeze",
            TokenTransitionAction::ClaimAction(_) => "claim",
            TokenTransitionAction::EmergencyActionAction(_) => "emergencyAction",
            TokenTransitionAction::DestroyFrozenFundsAction(_) => "destroyFrozenFunds",
            TokenTransitionAction::ConfigUpdateAction(_) => "configUpdate",
            TokenTransitionAction::DirectPurchaseAction(_) => "directPurchase",
            TokenTransitionAction::SetPriceForDirectPurchaseAction(_) => "directPricing",
        }
    }

    /// Historical document type for the token history contract
    pub fn historical_document_type<'a>(
        &self,
        token_history_contract: &'a DataContract,
    ) -> Result<DocumentTypeRef<'a>, ProtocolError> {
        Ok(token_history_contract.document_type_for_name(self.historical_document_type_name())?)
    }

    /// Historical document id
    pub fn historical_document_id(&self, owner_id: Identifier) -> Identifier {
        let token_id = self.base().token_id();
        let name = self.historical_document_type_name();
        let owner_nonce = self.base().identity_contract_nonce();
        Document::generate_document_id_v0(
            &token_id,
            &owner_id,
            format!("history_{}", name).as_str(),
            owner_nonce.to_be_bytes().as_slice(),
        )
    }

    /// Historical document
    pub fn build_historical_document(
        &self,
        token_id: Identifier,
        owner_id: Identifier,
        owner_nonce: IdentityNonce,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<Document, Error> {
        self.associated_token_event()
            .build_historical_document_owned(
                token_id,
                owner_id,
                owner_nonce,
                block_info,
                platform_version,
            )
            .map_err(Error::Protocol)
    }

    /// Do we keep history for this action
    pub fn keeps_history(&self) -> Result<bool, Error> {
        let keeps_history = self.base().token_configuration()?.keeps_history();
        match self {
            TokenTransitionAction::BurnAction(_) => Ok(keeps_history.keeps_burning_history()),
            TokenTransitionAction::MintAction(_) => Ok(keeps_history.keeps_minting_history()),
            TokenTransitionAction::TransferAction(_) => Ok(keeps_history.keeps_transfer_history()),
            TokenTransitionAction::FreezeAction(_) => Ok(keeps_history.keeps_freezing_history()),
            TokenTransitionAction::UnfreezeAction(_) => Ok(keeps_history.keeps_freezing_history()),
            TokenTransitionAction::ClaimAction(_) => Ok(true),
            TokenTransitionAction::EmergencyActionAction(_) => Ok(true),
            TokenTransitionAction::DestroyFrozenFundsAction(_) => Ok(true),
            TokenTransitionAction::ConfigUpdateAction(_) => Ok(true),
            TokenTransitionAction::DirectPurchaseAction(_) => {
                Ok(keeps_history.keeps_direct_purchase_history())
            }
            TokenTransitionAction::SetPriceForDirectPurchaseAction(_) => {
                Ok(keeps_history.keeps_direct_pricing_history())
            }
        }
    }
}
