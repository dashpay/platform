use crate::state_transition_action::contract::data_contract_create::DataContractCreateTransitionAction;
use crate::state_transition_action::identity::identity_credit_transfer::IdentityCreditTransferTransitionAction;
use crate::state_transition_action::identity::identity_credit_withdrawal::IdentityCreditWithdrawalTransitionAction;
use crate::state_transition_action::identity::identity_update::IdentityUpdateTransitionAction;
use crate::state_transition_action::system::bump_identity_nonce_action::{
    BumpIdentityNonceAction, BumpIdentityNonceActionV0,
};
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;

impl BumpIdentityNonceAction {
    /// from identity update
    pub fn from_identity_update_transition(value: IdentityUpdateTransition) -> Self {
        match value {
            IdentityUpdateTransition::V0(v0) => {
                BumpIdentityNonceActionV0::from_identity_update(v0).into()
            }
        }
    }

    /// from borrowed identity update
    pub fn from_borrowed_identity_update_transition(value: &IdentityUpdateTransition) -> Self {
        match value {
            IdentityUpdateTransition::V0(v0) => {
                BumpIdentityNonceActionV0::from_borrowed_identity_update(v0).into()
            }
        }
    }

    /// from identity update action
    pub fn from_identity_update_transition_action(value: IdentityUpdateTransitionAction) -> Self {
        match value {
            IdentityUpdateTransitionAction::V0(v0) => {
                BumpIdentityNonceActionV0::from_identity_update_action(v0).into()
            }
        }
    }

    /// from borrowed identity update action
    pub fn from_borrowed_identity_update_transition_action(
        value: &IdentityUpdateTransitionAction,
    ) -> Self {
        match value {
            IdentityUpdateTransitionAction::V0(v0) => {
                BumpIdentityNonceActionV0::from_borrowed_identity_update_action(v0).into()
            }
        }
    }

    /// from data contract create transition
    pub fn from_data_contract_create_transition(value: DataContractCreateTransition) -> Self {
        match value {
            DataContractCreateTransition::V0(v0) => {
                BumpIdentityNonceActionV0::from_contract_create(v0).into()
            }
        }
    }

    /// from borrowed data contract create transition
    pub fn from_borrowed_data_contract_create_transition(
        value: &DataContractCreateTransition,
    ) -> Self {
        match value {
            DataContractCreateTransition::V0(v0) => {
                BumpIdentityNonceActionV0::from_borrowed_contract_create(v0).into()
            }
        }
    }

    /// from data contract create transition action
    pub fn from_data_contract_create_action(value: DataContractCreateTransitionAction) -> Self {
        match value {
            DataContractCreateTransitionAction::V0(v0) => {
                BumpIdentityNonceActionV0::from_contract_create_action(v0).into()
            }
        }
    }

    /// from borrowed data contract create transition action
    pub fn from_borrowed_data_contract_create_action(
        value: &DataContractCreateTransitionAction,
    ) -> Self {
        match value {
            DataContractCreateTransitionAction::V0(v0) => {
                BumpIdentityNonceActionV0::from_borrowed_contract_create_action(v0).into()
            }
        }
    }

    /// from identity transfer
    pub fn from_identity_credit_transfer_transition(
        value: IdentityCreditTransferTransition,
    ) -> Self {
        match value {
            IdentityCreditTransferTransition::V0(v0) => {
                BumpIdentityNonceActionV0::from_identity_credit_transfer(v0).into()
            }
        }
    }

    /// from borrowed identity transfer
    pub fn from_borrowed_identity_credit_transfer_transition(
        value: &IdentityCreditTransferTransition,
    ) -> Self {
        match value {
            IdentityCreditTransferTransition::V0(v0) => {
                BumpIdentityNonceActionV0::from_borrowed_identity_credit_transfer(v0).into()
            }
        }
    }

    /// from identity transfer action
    pub fn from_identity_credit_transfer_transition_action(
        value: IdentityCreditTransferTransitionAction,
    ) -> Self {
        match value {
            IdentityCreditTransferTransitionAction::V0(v0) => {
                BumpIdentityNonceActionV0::from_identity_credit_transfer_action(v0).into()
            }
        }
    }

    /// from borrowed identity transfer action
    pub fn from_borrowed_identity_credit_transfer_transition_action(
        value: &IdentityCreditTransferTransitionAction,
    ) -> Self {
        match value {
            IdentityCreditTransferTransitionAction::V0(v0) => {
                BumpIdentityNonceActionV0::from_borrowed_identity_credit_transfer_action(v0).into()
            }
        }
    }

    /// from identity withdrawal
    pub fn from_identity_credit_withdrawal_transition(
        value: IdentityCreditWithdrawalTransition,
    ) -> Self {
        BumpIdentityNonceActionV0::from_identity_credit_withdrawal(value).into()
    }

    /// from borrowed identity withdrawal
    pub fn from_borrowed_identity_credit_withdrawal_transition(
        value: &IdentityCreditWithdrawalTransition,
    ) -> Self {
        BumpIdentityNonceActionV0::from_borrowed_identity_credit_withdrawal(value).into()
    }

    /// from identity withdrawal action
    pub fn from_identity_credit_withdrawal_transition_action(
        value: IdentityCreditWithdrawalTransitionAction,
    ) -> Self {
        match value {
            IdentityCreditWithdrawalTransitionAction::V0(v0) => {
                BumpIdentityNonceActionV0::from_identity_credit_withdrawal_action(v0).into()
            }
        }
    }

    /// from borrowed identity withdrawal action
    pub fn from_borrowed_identity_credit_withdrawal_transition_action(
        value: &IdentityCreditWithdrawalTransitionAction,
    ) -> Self {
        match value {
            IdentityCreditWithdrawalTransitionAction::V0(v0) => {
                BumpIdentityNonceActionV0::from_borrowed_identity_credit_withdrawal_action(v0)
                    .into()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::document::Document;
    use dpp::identifier::Identifier;
    use dpp::platform_value::BinaryData;
    use dpp::prelude::IdentityNonce;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::PlatformVersion;
    use platform_version::TryIntoPlatformVersioned;

    use crate::state_transition_action::contract::data_contract_create::v0::DataContractCreateTransitionActionV0;
    use crate::state_transition_action::identity::identity_credit_transfer::v0::IdentityCreditTransferTransitionActionV0;
    use crate::state_transition_action::identity::identity_credit_withdrawal::v0::IdentityCreditWithdrawalTransitionActionV0;
    use crate::state_transition_action::identity::identity_update::v0::IdentityUpdateTransitionActionV0;

    const TEST_ID: [u8; 32] = [0xAA; 32];
    const TEST_NONCE: IdentityNonce = 42;
    const TEST_FEE: u16 = 5;

    fn assert_v0(action: &BumpIdentityNonceAction) {
        match action {
            BumpIdentityNonceAction::V0(v0) => {
                assert_eq!(v0.identity_id, Identifier::from(TEST_ID));
                assert_eq!(v0.identity_nonce, TEST_NONCE);
                assert_eq!(v0.user_fee_increase, TEST_FEE);
            }
        }
    }

    // ---- IdentityUpdateTransition ----

    #[test]
    fn test_from_identity_update_transition() {
        let v0 =
            dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0 {
                identity_id: Identifier::from(TEST_ID),
                revision: 1,
                nonce: TEST_NONCE,
                add_public_keys: vec![],
                disable_public_keys: vec![],
                user_fee_increase: TEST_FEE,
                signature_public_key_id: 0,
                signature: BinaryData::default(),
            };
        let transition = IdentityUpdateTransition::V0(v0);
        let action = BumpIdentityNonceAction::from_identity_update_transition(transition);
        assert_v0(&action);
    }

    #[test]
    fn test_from_borrowed_identity_update_transition() {
        let v0 =
            dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0 {
                identity_id: Identifier::from(TEST_ID),
                revision: 1,
                nonce: TEST_NONCE,
                add_public_keys: vec![],
                disable_public_keys: vec![],
                user_fee_increase: TEST_FEE,
                signature_public_key_id: 0,
                signature: BinaryData::default(),
            };
        let transition = IdentityUpdateTransition::V0(v0);
        let action = BumpIdentityNonceAction::from_borrowed_identity_update_transition(&transition);
        assert_v0(&action);
    }

    // ---- IdentityUpdateTransitionAction ----

    #[test]
    fn test_from_identity_update_transition_action() {
        let v0 = IdentityUpdateTransitionActionV0 {
            add_public_keys: vec![],
            disable_public_keys: vec![],
            identity_id: Identifier::from(TEST_ID),
            revision: 1,
            nonce: TEST_NONCE,
            user_fee_increase: TEST_FEE,
        };
        let action_enum = IdentityUpdateTransitionAction::V0(v0);
        let action = BumpIdentityNonceAction::from_identity_update_transition_action(action_enum);
        assert_v0(&action);
    }

    #[test]
    fn test_from_borrowed_identity_update_transition_action() {
        let v0 = IdentityUpdateTransitionActionV0 {
            add_public_keys: vec![],
            disable_public_keys: vec![],
            identity_id: Identifier::from(TEST_ID),
            revision: 1,
            nonce: TEST_NONCE,
            user_fee_increase: TEST_FEE,
        };
        let action_enum = IdentityUpdateTransitionAction::V0(v0);
        let action =
            BumpIdentityNonceAction::from_borrowed_identity_update_transition_action(&action_enum);
        assert_v0(&action);
    }

    // ---- DataContractCreateTransition ----

    #[test]
    fn test_from_data_contract_create_transition() {
        let platform_version = PlatformVersion::latest();
        let data_contract = get_data_contract_fixture(
            Some(Identifier::from(TEST_ID)),
            0,
            platform_version.protocol_version,
        )
        .data_contract_owned();
        let serialized: dpp::data_contract::serialized_version::DataContractInSerializationFormat =
            data_contract
                .try_into_platform_versioned(platform_version)
                .expect("serialize");
        let v0 = dpp::state_transition::data_contract_create_transition::DataContractCreateTransitionV0 {
            data_contract: serialized,
            identity_nonce: TEST_NONCE,
            user_fee_increase: TEST_FEE,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        };
        let transition = DataContractCreateTransition::V0(v0);
        let action = BumpIdentityNonceAction::from_data_contract_create_transition(transition);
        assert_v0(&action);
    }

    #[test]
    fn test_from_borrowed_data_contract_create_transition() {
        let platform_version = PlatformVersion::latest();
        let data_contract = get_data_contract_fixture(
            Some(Identifier::from(TEST_ID)),
            0,
            platform_version.protocol_version,
        )
        .data_contract_owned();
        let serialized: dpp::data_contract::serialized_version::DataContractInSerializationFormat =
            data_contract
                .try_into_platform_versioned(platform_version)
                .expect("serialize");
        let v0 = dpp::state_transition::data_contract_create_transition::DataContractCreateTransitionV0 {
            data_contract: serialized,
            identity_nonce: TEST_NONCE,
            user_fee_increase: TEST_FEE,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        };
        let transition = DataContractCreateTransition::V0(v0);
        let action =
            BumpIdentityNonceAction::from_borrowed_data_contract_create_transition(&transition);
        assert_v0(&action);
    }

    // ---- DataContractCreateTransitionAction ----

    #[test]
    fn test_from_data_contract_create_action() {
        let platform_version = PlatformVersion::latest();
        let data_contract = get_data_contract_fixture(
            Some(Identifier::from(TEST_ID)),
            0,
            platform_version.protocol_version,
        )
        .data_contract_owned();
        let v0 = DataContractCreateTransitionActionV0 {
            data_contract,
            identity_nonce: TEST_NONCE,
            user_fee_increase: TEST_FEE,
        };
        let action_enum = DataContractCreateTransitionAction::V0(v0);
        let action = BumpIdentityNonceAction::from_data_contract_create_action(action_enum);
        assert_v0(&action);
    }

    #[test]
    fn test_from_borrowed_data_contract_create_action() {
        let platform_version = PlatformVersion::latest();
        let data_contract = get_data_contract_fixture(
            Some(Identifier::from(TEST_ID)),
            0,
            platform_version.protocol_version,
        )
        .data_contract_owned();
        let v0 = DataContractCreateTransitionActionV0 {
            data_contract,
            identity_nonce: TEST_NONCE,
            user_fee_increase: TEST_FEE,
        };
        let action_enum = DataContractCreateTransitionAction::V0(v0);
        let action =
            BumpIdentityNonceAction::from_borrowed_data_contract_create_action(&action_enum);
        assert_v0(&action);
    }

    // ---- IdentityCreditTransferTransition ----

    #[test]
    fn test_from_identity_credit_transfer_transition() {
        let v0 = dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0 {
            identity_id: Identifier::from(TEST_ID),
            recipient_id: Identifier::from([0xBB; 32]),
            amount: 1000,
            nonce: TEST_NONCE,
            user_fee_increase: TEST_FEE,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        };
        let transition = IdentityCreditTransferTransition::V0(v0);
        let action = BumpIdentityNonceAction::from_identity_credit_transfer_transition(transition);
        assert_v0(&action);
    }

    #[test]
    fn test_from_borrowed_identity_credit_transfer_transition() {
        let v0 = dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0 {
            identity_id: Identifier::from(TEST_ID),
            recipient_id: Identifier::from([0xBB; 32]),
            amount: 1000,
            nonce: TEST_NONCE,
            user_fee_increase: TEST_FEE,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        };
        let transition = IdentityCreditTransferTransition::V0(v0);
        let action =
            BumpIdentityNonceAction::from_borrowed_identity_credit_transfer_transition(&transition);
        assert_v0(&action);
    }

    // ---- IdentityCreditTransferTransitionAction ----

    #[test]
    fn test_from_identity_credit_transfer_transition_action() {
        let v0 = IdentityCreditTransferTransitionActionV0 {
            transfer_amount: 1000,
            recipient_id: Identifier::from([0xBB; 32]),
            identity_id: Identifier::from(TEST_ID),
            nonce: TEST_NONCE,
            user_fee_increase: TEST_FEE,
        };
        let action_enum = IdentityCreditTransferTransitionAction::V0(v0);
        let action =
            BumpIdentityNonceAction::from_identity_credit_transfer_transition_action(action_enum);
        assert_v0(&action);
    }

    #[test]
    fn test_from_borrowed_identity_credit_transfer_transition_action() {
        let v0 = IdentityCreditTransferTransitionActionV0 {
            transfer_amount: 1000,
            recipient_id: Identifier::from([0xBB; 32]),
            identity_id: Identifier::from(TEST_ID),
            nonce: TEST_NONCE,
            user_fee_increase: TEST_FEE,
        };
        let action_enum = IdentityCreditTransferTransitionAction::V0(v0);
        let action =
            BumpIdentityNonceAction::from_borrowed_identity_credit_transfer_transition_action(
                &action_enum,
            );
        assert_v0(&action);
    }

    // ---- IdentityCreditWithdrawalTransition ----

    #[test]
    fn test_from_identity_credit_withdrawal_transition() {
        let v0 = dpp::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0 {
            identity_id: Identifier::from(TEST_ID),
            amount: 500,
            core_fee_per_byte: 1,
            pooling: Default::default(),
            output_script: Default::default(),
            nonce: TEST_NONCE,
            user_fee_increase: TEST_FEE,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        };
        let transition = IdentityCreditWithdrawalTransition::V0(v0);
        let action =
            BumpIdentityNonceAction::from_identity_credit_withdrawal_transition(transition);
        assert_v0(&action);
    }

    #[test]
    fn test_from_borrowed_identity_credit_withdrawal_transition() {
        let v0 = dpp::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0 {
            identity_id: Identifier::from(TEST_ID),
            amount: 500,
            core_fee_per_byte: 1,
            pooling: Default::default(),
            output_script: Default::default(),
            nonce: TEST_NONCE,
            user_fee_increase: TEST_FEE,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        };
        let transition = IdentityCreditWithdrawalTransition::V0(v0);
        let action = BumpIdentityNonceAction::from_borrowed_identity_credit_withdrawal_transition(
            &transition,
        );
        assert_v0(&action);
    }

    // ---- IdentityCreditWithdrawalTransitionAction ----

    #[test]
    fn test_from_identity_credit_withdrawal_transition_action() {
        let v0 = IdentityCreditWithdrawalTransitionActionV0 {
            identity_id: Identifier::from(TEST_ID),
            nonce: TEST_NONCE,
            prepared_withdrawal_document: Document::V0(Default::default()),
            amount: 500,
            user_fee_increase: TEST_FEE,
        };
        let action_enum = IdentityCreditWithdrawalTransitionAction::V0(v0);
        let action =
            BumpIdentityNonceAction::from_identity_credit_withdrawal_transition_action(action_enum);
        assert_v0(&action);
    }

    #[test]
    fn test_from_borrowed_identity_credit_withdrawal_transition_action() {
        let v0 = IdentityCreditWithdrawalTransitionActionV0 {
            identity_id: Identifier::from(TEST_ID),
            nonce: TEST_NONCE,
            prepared_withdrawal_document: Document::V0(Default::default()),
            amount: 500,
            user_fee_increase: TEST_FEE,
        };
        let action_enum = IdentityCreditWithdrawalTransitionAction::V0(v0);
        let action =
            BumpIdentityNonceAction::from_borrowed_identity_credit_withdrawal_transition_action(
                &action_enum,
            );
        assert_v0(&action);
    }
}
