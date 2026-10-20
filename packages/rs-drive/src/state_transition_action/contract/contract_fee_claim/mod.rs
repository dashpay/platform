/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::contract::contract_fee_claim::v0::ContractFeeClaimTransitionActionV0;
use derive_more::From;
use dpp::block::epoch::EpochIndex;
use dpp::data_contract::document_type::action_fees::{ContractFeePot, ContractFeePotLastClaim};
use dpp::fee::Credits;
use dpp::platform_value::Identifier;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};
use std::collections::BTreeMap;

/// The action of a contract fee claim transition: the payout of one of a contract's fee pots,
/// with what each recipient is paid as settled when the transition was validated.
#[derive(Debug, Clone, From)]
pub enum ContractFeeClaimTransitionAction {
    /// v0
    V0(ContractFeeClaimTransitionActionV0),
}

impl ContractFeeClaimTransitionAction {
    /// The claimant that signed
    pub fn claimant_id(&self) -> Identifier {
        match self {
            ContractFeeClaimTransitionAction::V0(action) => action.claimant_id,
        }
    }

    /// The contract whose pot is paid out
    pub fn data_contract_id(&self) -> Identifier {
        match self {
            ContractFeeClaimTransitionAction::V0(action) => action.data_contract_id,
        }
    }

    /// The claimant's nonce for the contract
    pub fn identity_contract_nonce(&self) -> IdentityNonce {
        match self {
            ContractFeeClaimTransitionAction::V0(action) => action.identity_contract_nonce,
        }
    }

    /// The pot that is paid out
    pub fn pot(&self) -> ContractFeePot {
        match self {
            ContractFeeClaimTransitionAction::V0(action) => action.pot,
        }
    }

    /// The epoch of the claim
    pub fn epoch_index(&self) -> EpochIndex {
        match self {
            ContractFeeClaimTransitionAction::V0(action) => action.epoch_index,
        }
    }

    /// What the claim records as the pot's last claim: its epoch, the time of its block and
    /// the claimant
    pub fn last_claim(&self) -> ContractFeePotLastClaim {
        match self {
            ContractFeeClaimTransitionAction::V0(action) => ContractFeePotLastClaim {
                epoch_index: action.epoch_index,
                time_ms: action.time_ms,
                claimant_id: action.claimant_id,
            },
        }
    }

    /// What each recipient is paid
    pub fn payouts(&self) -> &BTreeMap<Identifier, Credits> {
        match self {
            ContractFeeClaimTransitionAction::V0(action) => &action.payouts,
        }
    }

    /// The members whose moderation action counts the claim resets
    pub fn settled_action_counts(&self) -> &[Identifier] {
        match self {
            ContractFeeClaimTransitionAction::V0(action) => &action.settled_action_counts,
        }
    }

    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            ContractFeeClaimTransitionAction::V0(action) => action.user_fee_increase,
        }
    }
}
