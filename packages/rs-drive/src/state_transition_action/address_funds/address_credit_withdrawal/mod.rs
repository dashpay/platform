/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::address_funds::address_credit_withdrawal::v0::AddressCreditWithdrawalTransitionActionV0;
use derive_more::From;
use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
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

    /// fee strategy
    pub fn fee_strategy(&self) -> &AddressFundsFeeStrategy {
        match self {
            AddressCreditWithdrawalTransitionAction::V0(transition) => &transition.fee_strategy,
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

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::address_funds::fee_strategy::AddressFundsFeeStrategyStep;
    use dpp::document::DocumentV0;
    use dpp::prelude::Identifier;

    fn make_document() -> Document {
        Document::V0(DocumentV0 {
            contract_version: None,
            id: Identifier::from([0x11; 32]),
            owner_id: Identifier::from([0x22; 32]),
            properties: Default::default(),
            revision: Some(1),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        })
    }

    fn make_action() -> AddressCreditWithdrawalTransitionAction {
        let addr_a = PlatformAddress::P2pkh([0xAA; 20]);
        let addr_b = PlatformAddress::P2sh([0xCC; 20]);

        let mut inputs = BTreeMap::new();
        inputs.insert(addr_a, (5_u32, 10000_u64));

        let fee_strategy = vec![AddressFundsFeeStrategyStep::ReduceOutput(0)];

        let v0 = AddressCreditWithdrawalTransitionActionV0 {
            inputs_with_remaining_balance: inputs,
            output: Some((addr_b, 2000)),
            fee_strategy,
            user_fee_increase: 4,
            prepared_withdrawal_document: make_document(),
            amount: 8000,
        };
        AddressCreditWithdrawalTransitionAction::from(v0)
    }

    #[test]
    fn test_from_v0() {
        let action = make_action();
        assert!(matches!(
            action,
            AddressCreditWithdrawalTransitionAction::V0(_)
        ));
    }

    #[test]
    fn test_inputs_with_remaining_balance() {
        let action = make_action();
        let inputs = action.inputs_with_remaining_balance();
        assert_eq!(inputs.len(), 1);
        let addr_a = PlatformAddress::P2pkh([0xAA; 20]);
        let (nonce, balance) = inputs.get(&addr_a).unwrap();
        assert_eq!(*nonce, 5);
        assert_eq!(*balance, 10000);
    }

    #[test]
    fn test_output_some() {
        let action = make_action();
        let output = action.output();
        assert!(output.is_some());
        let (addr, credits) = output.unwrap();
        assert_eq!(addr, PlatformAddress::P2sh([0xCC; 20]));
        assert_eq!(credits, 2000);
    }

    #[test]
    fn test_output_none() {
        let v0 = AddressCreditWithdrawalTransitionActionV0 {
            inputs_with_remaining_balance: BTreeMap::new(),
            output: None,
            fee_strategy: vec![],
            user_fee_increase: 0,
            prepared_withdrawal_document: make_document(),
            amount: 500,
        };
        let action = AddressCreditWithdrawalTransitionAction::from(v0);
        assert!(action.output().is_none());
    }

    #[test]
    fn test_user_fee_increase() {
        let action = make_action();
        assert_eq!(action.user_fee_increase(), 4);
    }

    #[test]
    fn test_fee_strategy() {
        let action = make_action();
        let strategy = action.fee_strategy();
        assert_eq!(strategy.len(), 1);
        assert!(matches!(
            strategy[0],
            AddressFundsFeeStrategyStep::ReduceOutput(0)
        ));
    }

    #[test]
    fn test_prepared_withdrawal_document_ref() {
        let action = make_action();
        let doc = action.prepared_withdrawal_document();
        match doc {
            Document::V0(v0) => {
                assert_eq!(v0.id, Identifier::from([0x11; 32]));
                assert_eq!(v0.owner_id, Identifier::from([0x22; 32]));
            }
        }
    }

    #[test]
    fn test_prepared_withdrawal_document_owned() {
        let action = make_action();
        let doc = action.prepared_withdrawal_document_owned();
        match doc {
            Document::V0(v0) => {
                assert_eq!(v0.id, Identifier::from([0x11; 32]));
            }
        }
    }

    #[test]
    fn test_amount() {
        let action = make_action();
        assert_eq!(action.amount(), 8000);
    }

    #[test]
    fn test_clone() {
        let action = make_action();
        let cloned = action.clone();
        assert_eq!(cloned.amount(), 8000);
        assert_eq!(cloned.user_fee_increase(), 4);
        assert!(cloned.output().is_some());
    }

    #[test]
    fn test_debug() {
        let action = make_action();
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("V0"));
    }
}
