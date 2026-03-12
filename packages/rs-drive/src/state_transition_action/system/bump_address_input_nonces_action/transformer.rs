use crate::state_transition_action::address_funds::address_funds_transfer::AddressFundsTransferTransitionAction;
use crate::state_transition_action::identity::identity_create_from_addresses::IdentityCreateFromAddressesTransitionAction;
use crate::state_transition_action::identity::identity_topup_from_addresses::IdentityTopUpFromAddressesTransitionAction;
use crate::state_transition_action::system::bump_address_input_nonces_action::{
    BumpAddressInputNoncesAction, BumpAddressInputNoncesActionV0,
};
use dpp::address_funds::fee_strategy::AddressFundsFeeStrategy;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, UserFeeIncrease};
use dpp::state_transition::state_transitions::address_funds::address_funds_transfer_transition::AddressFundsTransferTransition;
use dpp::state_transition::state_transitions::identity::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use dpp::state_transition::state_transitions::identity::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
use std::collections::BTreeMap;

impl BumpAddressInputNoncesAction {
    // Generic constructor from pre-computed inputs

    /// from inputs with remaining balance (used by shield transitions where
    /// the remaining balances are computed by the processing pipeline)
    pub fn from_inputs_with_remaining_balance(
        inputs_with_remaining_balance: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        fee_strategy: &AddressFundsFeeStrategy,
        penalty_credits: Credits,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpAddressInputNoncesActionV0::from_inputs_with_remaining_balance(
            inputs_with_remaining_balance,
            fee_strategy,
            penalty_credits,
            user_fee_increase,
        )
        .into()
    }

    // IdentityCreateFromAddresses transformers

    /// from IdentityCreateFromAddresses transition
    pub fn from_identity_create_from_addresses_transition(
        value: IdentityCreateFromAddressesTransition,
        penalty_credits: Credits,
    ) -> Self {
        match value {
            IdentityCreateFromAddressesTransition::V0(ref v0) => {
                BumpAddressInputNoncesActionV0::from_borrowed_identity_create_from_addresses_transition(v0, penalty_credits)
                    .into()
            }
        }
    }

    /// from borrowed IdentityCreateFromAddresses transition
    pub fn from_borrowed_identity_create_from_addresses_transition(
        value: &IdentityCreateFromAddressesTransition,
        penalty_credits: Credits,
    ) -> Self {
        match value {
            IdentityCreateFromAddressesTransition::V0(v0) => {
                BumpAddressInputNoncesActionV0::from_borrowed_identity_create_from_addresses_transition(v0, penalty_credits)
                    .into()
            }
        }
    }

    /// from IdentityCreateFromAddresses transition action
    pub fn from_identity_create_from_addresses_transition_action(
        value: IdentityCreateFromAddressesTransitionAction,
        penalty_credits: Credits,
    ) -> Self {
        match value {
            IdentityCreateFromAddressesTransitionAction::V0(ref v0) => {
                BumpAddressInputNoncesActionV0::from_borrowed_identity_create_from_addresses_transition_action(v0, penalty_credits)
                    .into()
            }
        }
    }

    /// from borrowed IdentityCreateFromAddresses transition action
    pub fn from_borrowed_identity_create_from_addresses_transition_action(
        value: &IdentityCreateFromAddressesTransitionAction,
        penalty_credits: Credits,
    ) -> Self {
        match value {
            IdentityCreateFromAddressesTransitionAction::V0(v0) => {
                BumpAddressInputNoncesActionV0::from_borrowed_identity_create_from_addresses_transition_action(v0, penalty_credits)
                    .into()
            }
        }
    }

    // IdentityTopUpFromAddresses transformers

    /// from IdentityTopUpFromAddresses transition
    pub fn from_identity_topup_from_addresses_transition(
        value: IdentityTopUpFromAddressesTransition,
        penalty_credits: Credits,
    ) -> Self {
        match value {
            IdentityTopUpFromAddressesTransition::V0(ref v0) => {
                BumpAddressInputNoncesActionV0::from_borrowed_identity_topup_from_addresses_transition(v0, penalty_credits)
                    .into()
            }
        }
    }

    /// from borrowed IdentityTopUpFromAddresses transition
    pub fn from_borrowed_identity_topup_from_addresses_transition(
        value: &IdentityTopUpFromAddressesTransition,
        penalty_credits: Credits,
    ) -> Self {
        match value {
            IdentityTopUpFromAddressesTransition::V0(v0) => {
                BumpAddressInputNoncesActionV0::from_borrowed_identity_topup_from_addresses_transition(v0, penalty_credits)
                    .into()
            }
        }
    }

    /// from IdentityTopUpFromAddresses transition action
    pub fn from_identity_topup_from_addresses_transition_action(
        value: IdentityTopUpFromAddressesTransitionAction,
        penalty_credits: Credits,
    ) -> Self {
        match value {
            IdentityTopUpFromAddressesTransitionAction::V0(ref v0) => {
                BumpAddressInputNoncesActionV0::from_borrowed_identity_topup_from_addresses_transition_action(v0, penalty_credits)
                    .into()
            }
        }
    }

    /// from borrowed IdentityTopUpFromAddresses transition action
    pub fn from_borrowed_identity_topup_from_addresses_transition_action(
        value: &IdentityTopUpFromAddressesTransitionAction,
        penalty_credits: Credits,
    ) -> Self {
        match value {
            IdentityTopUpFromAddressesTransitionAction::V0(v0) => {
                BumpAddressInputNoncesActionV0::from_borrowed_identity_topup_from_addresses_transition_action(v0, penalty_credits)
                    .into()
            }
        }
    }

    // AddressFundsTransfer transformers

    /// from AddressFundsTransfer transition
    pub fn from_address_funds_transfer_transition(
        value: AddressFundsTransferTransition,
        penalty_credits: Credits,
    ) -> Self {
        match value {
            AddressFundsTransferTransition::V0(ref v0) => {
                BumpAddressInputNoncesActionV0::from_borrowed_address_funds_transfer_transition(
                    v0,
                    penalty_credits,
                )
                .into()
            }
        }
    }

    /// from borrowed AddressFundsTransfer transition
    pub fn from_borrowed_address_funds_transfer_transition(
        value: &AddressFundsTransferTransition,
        penalty_credits: Credits,
    ) -> Self {
        match value {
            AddressFundsTransferTransition::V0(v0) => {
                BumpAddressInputNoncesActionV0::from_borrowed_address_funds_transfer_transition(
                    v0,
                    penalty_credits,
                )
                .into()
            }
        }
    }

    /// from AddressFundsTransfer transition action
    pub fn from_address_funds_transfer_transition_action(
        value: AddressFundsTransferTransitionAction,
        penalty_credits: Credits,
    ) -> Self {
        match value {
            AddressFundsTransferTransitionAction::V0(ref v0) => {
                BumpAddressInputNoncesActionV0::from_borrowed_address_funds_transfer_transition_action(v0, penalty_credits)
                    .into()
            }
        }
    }

    /// from borrowed AddressFundsTransfer transition action
    pub fn from_borrowed_address_funds_transfer_transition_action(
        value: &AddressFundsTransferTransitionAction,
        penalty_credits: Credits,
    ) -> Self {
        match value {
            AddressFundsTransferTransitionAction::V0(v0) => {
                BumpAddressInputNoncesActionV0::from_borrowed_address_funds_transfer_transition_action(v0, penalty_credits)
                    .into()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::address_funds::address_funds_transfer::v0::AddressFundsTransferTransitionActionV0;
    use crate::state_transition_action::identity::identity_create_from_addresses::v0::IdentityCreateFromAddressesTransitionActionV0;
    use crate::state_transition_action::identity::identity_topup_from_addresses::v0::IdentityTopUpFromAddressesTransitionActionV0;
    use crate::state_transition_action::system::bump_address_input_nonces_action::BumpAddressInputNonceActionAccessorsV0;
    use dpp::address_funds::fee_strategy::AddressFundsFeeStrategyStep;
    use dpp::identifier::Identifier;

    const TEST_FEE: u16 = 7;

    fn make_inputs() -> BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0xAA; 20]), (10_u32, 5000_u64));
        inputs
    }

    fn make_strategy() -> AddressFundsFeeStrategy {
        vec![AddressFundsFeeStrategyStep::DeductFromInput(0)]
    }

    fn assert_is_v0(action: &BumpAddressInputNoncesAction) {
        assert!(matches!(action, BumpAddressInputNoncesAction::V0(_)));
    }

    // ---- from_inputs_with_remaining_balance ----

    #[test]
    fn test_from_inputs_with_remaining_balance() {
        let inputs = make_inputs();
        let strategy = make_strategy();
        let action = BumpAddressInputNoncesAction::from_inputs_with_remaining_balance(
            &inputs, &strategy, 100, TEST_FEE,
        );
        assert_is_v0(&action);
        assert_eq!(action.user_fee_increase(), TEST_FEE);
    }

    // ---- IdentityCreateFromAddresses transition ----

    #[test]
    fn test_from_identity_create_from_addresses_transition() {
        let inputs = make_inputs();
        let strategy = make_strategy();
        let v0 = dpp::state_transition::state_transitions::identity::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0 {
            public_keys: vec![],
            inputs: inputs.clone(),
            output: None,
            fee_strategy: strategy.clone(),
            user_fee_increase: TEST_FEE,
            input_witnesses: vec![],
        };
        let transition = IdentityCreateFromAddressesTransition::V0(v0);
        let action =
            BumpAddressInputNoncesAction::from_identity_create_from_addresses_transition(
                transition, 100,
            );
        assert_is_v0(&action);
        assert_eq!(action.user_fee_increase(), TEST_FEE);
    }

    #[test]
    fn test_from_borrowed_identity_create_from_addresses_transition() {
        let inputs = make_inputs();
        let strategy = make_strategy();
        let v0 = dpp::state_transition::state_transitions::identity::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0 {
            public_keys: vec![],
            inputs: inputs.clone(),
            output: None,
            fee_strategy: strategy.clone(),
            user_fee_increase: TEST_FEE,
            input_witnesses: vec![],
        };
        let transition = IdentityCreateFromAddressesTransition::V0(v0);
        let action =
            BumpAddressInputNoncesAction::from_borrowed_identity_create_from_addresses_transition(
                &transition, 100,
            );
        assert_is_v0(&action);
        assert_eq!(action.user_fee_increase(), TEST_FEE);
    }

    // ---- IdentityCreateFromAddresses action ----

    #[test]
    fn test_from_identity_create_from_addresses_transition_action() {
        let inputs = make_inputs();
        let strategy = make_strategy();
        let v0 = IdentityCreateFromAddressesTransitionActionV0 {
            inputs_with_remaining_balance: inputs.clone(),
            output: None,
            fee_strategy: strategy.clone(),
            public_keys: vec![],
            identity_id: Identifier::from([0xCC; 32]),
            fund_identity_amount: 1000,
            user_fee_increase: TEST_FEE,
        };
        let action_enum = IdentityCreateFromAddressesTransitionAction::V0(v0);
        let action =
            BumpAddressInputNoncesAction::from_identity_create_from_addresses_transition_action(
                action_enum, 50,
            );
        assert_is_v0(&action);
        assert_eq!(action.user_fee_increase(), TEST_FEE);
    }

    #[test]
    fn test_from_borrowed_identity_create_from_addresses_transition_action() {
        let inputs = make_inputs();
        let strategy = make_strategy();
        let v0 = IdentityCreateFromAddressesTransitionActionV0 {
            inputs_with_remaining_balance: inputs.clone(),
            output: None,
            fee_strategy: strategy.clone(),
            public_keys: vec![],
            identity_id: Identifier::from([0xCC; 32]),
            fund_identity_amount: 1000,
            user_fee_increase: TEST_FEE,
        };
        let action_enum = IdentityCreateFromAddressesTransitionAction::V0(v0);
        let action =
            BumpAddressInputNoncesAction::from_borrowed_identity_create_from_addresses_transition_action(
                &action_enum, 50,
            );
        assert_is_v0(&action);
        assert_eq!(action.user_fee_increase(), TEST_FEE);
    }

    // ---- IdentityTopUpFromAddresses transition ----

    #[test]
    fn test_from_identity_topup_from_addresses_transition() {
        let inputs = make_inputs();
        let strategy = make_strategy();
        let v0 = dpp::state_transition::state_transitions::identity::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0 {
            inputs: inputs.clone(),
            output: None,
            identity_id: Identifier::from([0xCC; 32]),
            fee_strategy: strategy.clone(),
            user_fee_increase: TEST_FEE,
            input_witnesses: vec![],
        };
        let transition = IdentityTopUpFromAddressesTransition::V0(v0);
        let action =
            BumpAddressInputNoncesAction::from_identity_topup_from_addresses_transition(
                transition, 100,
            );
        assert_is_v0(&action);
        assert_eq!(action.user_fee_increase(), TEST_FEE);
    }

    #[test]
    fn test_from_borrowed_identity_topup_from_addresses_transition() {
        let inputs = make_inputs();
        let strategy = make_strategy();
        let v0 = dpp::state_transition::state_transitions::identity::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0 {
            inputs: inputs.clone(),
            output: None,
            identity_id: Identifier::from([0xCC; 32]),
            fee_strategy: strategy.clone(),
            user_fee_increase: TEST_FEE,
            input_witnesses: vec![],
        };
        let transition = IdentityTopUpFromAddressesTransition::V0(v0);
        let action =
            BumpAddressInputNoncesAction::from_borrowed_identity_topup_from_addresses_transition(
                &transition, 100,
            );
        assert_is_v0(&action);
        assert_eq!(action.user_fee_increase(), TEST_FEE);
    }

    // ---- IdentityTopUpFromAddresses action ----

    #[test]
    fn test_from_identity_topup_from_addresses_transition_action() {
        let inputs = make_inputs();
        let strategy = make_strategy();
        let v0 = IdentityTopUpFromAddressesTransitionActionV0 {
            inputs_with_remaining_balance: inputs.clone(),
            output: None,
            fee_strategy: strategy.clone(),
            identity_id: Identifier::from([0xCC; 32]),
            topup_amount: 500,
            user_fee_increase: TEST_FEE,
        };
        let action_enum = IdentityTopUpFromAddressesTransitionAction::V0(v0);
        let action =
            BumpAddressInputNoncesAction::from_identity_topup_from_addresses_transition_action(
                action_enum, 75,
            );
        assert_is_v0(&action);
        assert_eq!(action.user_fee_increase(), TEST_FEE);
    }

    #[test]
    fn test_from_borrowed_identity_topup_from_addresses_transition_action() {
        let inputs = make_inputs();
        let strategy = make_strategy();
        let v0 = IdentityTopUpFromAddressesTransitionActionV0 {
            inputs_with_remaining_balance: inputs.clone(),
            output: None,
            fee_strategy: strategy.clone(),
            identity_id: Identifier::from([0xCC; 32]),
            topup_amount: 500,
            user_fee_increase: TEST_FEE,
        };
        let action_enum = IdentityTopUpFromAddressesTransitionAction::V0(v0);
        let action =
            BumpAddressInputNoncesAction::from_borrowed_identity_topup_from_addresses_transition_action(
                &action_enum, 75,
            );
        assert_is_v0(&action);
        assert_eq!(action.user_fee_increase(), TEST_FEE);
    }

    // ---- AddressFundsTransfer transition ----

    #[test]
    fn test_from_address_funds_transfer_transition() {
        let inputs = make_inputs();
        let strategy = make_strategy();
        let v0 = dpp::state_transition::state_transitions::address_funds::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0 {
            inputs: inputs.clone(),
            outputs: BTreeMap::new(),
            fee_strategy: strategy.clone(),
            user_fee_increase: TEST_FEE,
            input_witnesses: vec![],
        };
        let transition = AddressFundsTransferTransition::V0(v0);
        let action = BumpAddressInputNoncesAction::from_address_funds_transfer_transition(
            transition, 200,
        );
        assert_is_v0(&action);
        assert_eq!(action.user_fee_increase(), TEST_FEE);
    }

    #[test]
    fn test_from_borrowed_address_funds_transfer_transition() {
        let inputs = make_inputs();
        let strategy = make_strategy();
        let v0 = dpp::state_transition::state_transitions::address_funds::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0 {
            inputs: inputs.clone(),
            outputs: BTreeMap::new(),
            fee_strategy: strategy.clone(),
            user_fee_increase: TEST_FEE,
            input_witnesses: vec![],
        };
        let transition = AddressFundsTransferTransition::V0(v0);
        let action =
            BumpAddressInputNoncesAction::from_borrowed_address_funds_transfer_transition(
                &transition, 200,
            );
        assert_is_v0(&action);
        assert_eq!(action.user_fee_increase(), TEST_FEE);
    }

    // ---- AddressFundsTransfer action ----

    #[test]
    fn test_from_address_funds_transfer_transition_action() {
        let inputs = make_inputs();
        let strategy = make_strategy();
        let v0 = AddressFundsTransferTransitionActionV0 {
            inputs_with_remaining_balance: inputs.clone(),
            outputs: BTreeMap::new(),
            fee_strategy: strategy.clone(),
            user_fee_increase: TEST_FEE,
        };
        let action_enum = AddressFundsTransferTransitionAction::V0(v0);
        let action =
            BumpAddressInputNoncesAction::from_address_funds_transfer_transition_action(
                action_enum, 300,
            );
        assert_is_v0(&action);
        assert_eq!(action.user_fee_increase(), TEST_FEE);
    }

    #[test]
    fn test_from_borrowed_address_funds_transfer_transition_action() {
        let inputs = make_inputs();
        let strategy = make_strategy();
        let v0 = AddressFundsTransferTransitionActionV0 {
            inputs_with_remaining_balance: inputs.clone(),
            outputs: BTreeMap::new(),
            fee_strategy: strategy.clone(),
            user_fee_increase: TEST_FEE,
        };
        let action_enum = AddressFundsTransferTransitionAction::V0(v0);
        let action =
            BumpAddressInputNoncesAction::from_borrowed_address_funds_transfer_transition_action(
                &action_enum, 300,
            );
        assert_is_v0(&action);
        assert_eq!(action.user_fee_increase(), TEST_FEE);
    }

    // ---- penalty deduction is correctly applied through enum wrapper ----

    #[test]
    fn test_penalty_deduction_through_enum() {
        let inputs = make_inputs();
        let strategy = make_strategy();
        let action = BumpAddressInputNoncesAction::from_inputs_with_remaining_balance(
            &inputs, &strategy, 1000, TEST_FEE,
        );
        let remaining: Credits = action
            .inputs_with_remaining_balance()
            .values()
            .map(|(_, c)| c)
            .sum();
        let original: Credits = inputs.values().map(|(_, c)| c).sum();
        assert_eq!(remaining, original - 1000);
    }
}
