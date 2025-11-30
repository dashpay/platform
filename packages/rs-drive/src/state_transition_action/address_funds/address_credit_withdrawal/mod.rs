/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::address_funds::address_credit_withdrawal::v0::AddressCreditWithdrawalTransitionActionV0;
use derive_more::From;
use dpp::address_funds::PlatformAddress;
use dpp::document::Document;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, UserFeeIncrease};
use std::collections::BTreeMap;

/// action
#[derive(Debug, Clone, From)]
pub enum AddressCreditWithdrawalTransitionAction {
    /// v0
    V0(AddressCreditWithdrawalTransitionActionV0),
}

impl AddressCreditWithdrawalTransitionAction {
    /// Get inputs with remaining balance
    pub fn inputs_with_remaining_balance(
        &self,
    ) -> &BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
        match self {
            AddressCreditWithdrawalTransitionAction::V0(transition) => {
                &transition.inputs_with_remaining_balance
            }
        }
    }

    /// Get optional output (change)
    pub fn output(&self) -> Option<(PlatformAddress, Credits)> {
        match self {
            AddressCreditWithdrawalTransitionAction::V0(transition) => transition.output,
        }
    }

    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            AddressCreditWithdrawalTransitionAction::V0(transition) => transition.user_fee_increase,
        }
    }

    /// Get prepared withdrawal document
    pub fn prepared_withdrawal_document(&self) -> &Document {
        match self {
            AddressCreditWithdrawalTransitionAction::V0(transition) => {
                &transition.prepared_withdrawal_document
            }
        }
    }

    /// Get prepared withdrawal document owned
    pub fn prepared_withdrawal_document_owned(self) -> Document {
        match self {
            AddressCreditWithdrawalTransitionAction::V0(transition) => {
                transition.prepared_withdrawal_document
            }
        }
    }

    /// Get withdrawal amount
    pub fn amount(&self) -> Credits {
        match self {
            AddressCreditWithdrawalTransitionAction::V0(transition) => transition.amount,
        }
    }
}
