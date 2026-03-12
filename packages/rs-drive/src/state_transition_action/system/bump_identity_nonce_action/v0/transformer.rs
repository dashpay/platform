use crate::state_transition_action::contract::data_contract_create::v0::DataContractCreateTransitionActionV0;
use crate::state_transition_action::identity::identity_credit_transfer::v0::IdentityCreditTransferTransitionActionV0;
use crate::state_transition_action::identity::identity_credit_withdrawal::v0::IdentityCreditWithdrawalTransitionActionV0;
use crate::state_transition_action::identity::identity_update::v0::IdentityUpdateTransitionActionV0;
use crate::state_transition_action::system::bump_identity_nonce_action::BumpIdentityNonceActionV0;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
use dpp::state_transition::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
use dpp::state_transition::StateTransitionHasUserFeeIncrease;

impl BumpIdentityNonceActionV0 {
    /// from identity update
    pub fn from_identity_update(value: IdentityUpdateTransitionV0) -> Self {
        let IdentityUpdateTransitionV0 {
            identity_id,
            nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id,
            identity_nonce: nonce,
            user_fee_increase,
        }
    }

    /// from borrowed identity update
    pub fn from_borrowed_identity_update(value: &IdentityUpdateTransitionV0) -> Self {
        let IdentityUpdateTransitionV0 {
            identity_id,
            nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id: *identity_id,
            identity_nonce: *nonce,
            user_fee_increase: *user_fee_increase,
        }
    }

    /// from identity update action
    pub fn from_identity_update_action(value: IdentityUpdateTransitionActionV0) -> Self {
        let IdentityUpdateTransitionActionV0 {
            identity_id,
            nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id,
            identity_nonce: nonce,
            user_fee_increase,
        }
    }

    /// from borrowed identity update action
    pub fn from_borrowed_identity_update_action(value: &IdentityUpdateTransitionActionV0) -> Self {
        let IdentityUpdateTransitionActionV0 {
            identity_id,
            nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id: *identity_id,
            identity_nonce: *nonce,
            user_fee_increase: *user_fee_increase,
        }
    }

    /// from contract create
    pub fn from_contract_create(value: DataContractCreateTransitionV0) -> Self {
        let DataContractCreateTransitionV0 {
            data_contract,
            identity_nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id: data_contract.owner_id(),
            identity_nonce,
            user_fee_increase,
        }
    }

    /// from borrowed contract create
    pub fn from_borrowed_contract_create(value: &DataContractCreateTransitionV0) -> Self {
        let DataContractCreateTransitionV0 {
            data_contract,
            identity_nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id: data_contract.owner_id(),
            identity_nonce: *identity_nonce,
            user_fee_increase: *user_fee_increase,
        }
    }

    /// from contract create action
    pub fn from_contract_create_action(value: DataContractCreateTransitionActionV0) -> Self {
        let DataContractCreateTransitionActionV0 {
            data_contract,
            identity_nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id: data_contract.owner_id(),
            identity_nonce,
            user_fee_increase,
        }
    }

    /// from contract create action
    pub fn from_borrowed_contract_create_action(
        value: &DataContractCreateTransitionActionV0,
    ) -> Self {
        let DataContractCreateTransitionActionV0 {
            data_contract,
            identity_nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id: data_contract.owner_id(),
            identity_nonce: *identity_nonce,
            user_fee_increase: *user_fee_increase,
        }
    }

    /// from identity credit transfer
    pub fn from_identity_credit_transfer(value: IdentityCreditTransferTransitionV0) -> Self {
        let IdentityCreditTransferTransitionV0 {
            identity_id,
            nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id,
            identity_nonce: nonce,
            user_fee_increase,
        }
    }

    /// from borrowed identity credit transfer
    pub fn from_borrowed_identity_credit_transfer(
        value: &IdentityCreditTransferTransitionV0,
    ) -> Self {
        let IdentityCreditTransferTransitionV0 {
            identity_id,
            nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id: *identity_id,
            identity_nonce: *nonce,
            user_fee_increase: *user_fee_increase,
        }
    }

    /// from identity credit transfer action
    pub fn from_identity_credit_transfer_action(
        value: IdentityCreditTransferTransitionActionV0,
    ) -> Self {
        let IdentityCreditTransferTransitionActionV0 {
            identity_id,
            nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id,
            identity_nonce: nonce,
            user_fee_increase,
        }
    }

    /// from borrowed identity credit transfer action
    pub fn from_borrowed_identity_credit_transfer_action(
        value: &IdentityCreditTransferTransitionActionV0,
    ) -> Self {
        let IdentityCreditTransferTransitionActionV0 {
            identity_id,
            nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id: *identity_id,
            identity_nonce: *nonce,
            user_fee_increase: *user_fee_increase,
        }
    }

    /// from identity credit withdrawal
    pub fn from_identity_credit_withdrawal(value: IdentityCreditWithdrawalTransition) -> Self {
        BumpIdentityNonceActionV0 {
            identity_id: value.identity_id(),
            identity_nonce: value.nonce(),
            user_fee_increase: value.user_fee_increase(),
        }
    }

    /// from borrowed identity credit withdrawal
    pub fn from_borrowed_identity_credit_withdrawal(
        value: &IdentityCreditWithdrawalTransition,
    ) -> Self {
        BumpIdentityNonceActionV0 {
            identity_id: value.identity_id(),
            identity_nonce: value.nonce(),
            user_fee_increase: value.user_fee_increase(),
        }
    }

    /// from identity credit withdrawal action
    pub fn from_identity_credit_withdrawal_action(
        value: IdentityCreditWithdrawalTransitionActionV0,
    ) -> Self {
        let IdentityCreditWithdrawalTransitionActionV0 {
            identity_id,
            nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id,
            identity_nonce: nonce,
            user_fee_increase,
        }
    }

    /// from borrowed identity credit withdrawal action
    pub fn from_borrowed_identity_credit_withdrawal_action(
        value: &IdentityCreditWithdrawalTransitionActionV0,
    ) -> Self {
        let IdentityCreditWithdrawalTransitionActionV0 {
            identity_id,
            nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id: *identity_id,
            identity_nonce: *nonce,
            user_fee_increase: *user_fee_increase,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::document::Document;
    use dpp::identifier::Identifier;
    use dpp::platform_value::BinaryData;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::PlatformVersion;
    use platform_version::TryIntoPlatformVersioned;

    const TEST_IDENTITY_ID: [u8; 32] = [0xAA; 32];
    const TEST_NONCE: u64 = 42;
    const TEST_FEE_INCREASE: u16 = 5;

    // ---- helpers ----

    fn make_identity_update_transition() -> IdentityUpdateTransitionV0 {
        IdentityUpdateTransitionV0 {
            identity_id: Identifier::from(TEST_IDENTITY_ID),
            revision: 1,
            nonce: TEST_NONCE,
            add_public_keys: vec![],
            disable_public_keys: vec![],
            user_fee_increase: TEST_FEE_INCREASE,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        }
    }

    fn make_identity_update_action() -> IdentityUpdateTransitionActionV0 {
        IdentityUpdateTransitionActionV0 {
            add_public_keys: vec![],
            disable_public_keys: vec![],
            identity_id: Identifier::from(TEST_IDENTITY_ID),
            revision: 1,
            nonce: TEST_NONCE,
            user_fee_increase: TEST_FEE_INCREASE,
        }
    }

    fn make_data_contract_create_transition() -> DataContractCreateTransitionV0 {
        let platform_version = PlatformVersion::latest();
        let data_contract = get_data_contract_fixture(
            Some(Identifier::from(TEST_IDENTITY_ID)),
            0,
            platform_version.protocol_version,
        )
        .data_contract_owned();
        let serialized: dpp::data_contract::serialized_version::DataContractInSerializationFormat =
            data_contract
                .try_into_platform_versioned(platform_version)
                .expect("expected to serialize data contract");
        DataContractCreateTransitionV0 {
            data_contract: serialized,
            identity_nonce: TEST_NONCE,
            user_fee_increase: TEST_FEE_INCREASE,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        }
    }

    fn make_data_contract_create_action() -> DataContractCreateTransitionActionV0 {
        let platform_version = PlatformVersion::latest();
        let data_contract = get_data_contract_fixture(
            Some(Identifier::from(TEST_IDENTITY_ID)),
            0,
            platform_version.protocol_version,
        )
        .data_contract_owned();
        DataContractCreateTransitionActionV0 {
            data_contract,
            identity_nonce: TEST_NONCE,
            user_fee_increase: TEST_FEE_INCREASE,
        }
    }

    fn make_credit_transfer_transition() -> IdentityCreditTransferTransitionV0 {
        IdentityCreditTransferTransitionV0 {
            identity_id: Identifier::from(TEST_IDENTITY_ID),
            recipient_id: Identifier::from([0xBB; 32]),
            amount: 1000,
            nonce: TEST_NONCE,
            user_fee_increase: TEST_FEE_INCREASE,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        }
    }

    fn make_credit_transfer_action() -> IdentityCreditTransferTransitionActionV0 {
        IdentityCreditTransferTransitionActionV0 {
            transfer_amount: 1000,
            recipient_id: Identifier::from([0xBB; 32]),
            identity_id: Identifier::from(TEST_IDENTITY_ID),
            nonce: TEST_NONCE,
            user_fee_increase: TEST_FEE_INCREASE,
        }
    }

    fn make_credit_withdrawal_transition_v0()
        -> dpp::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0
    {
        dpp::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0 {
            identity_id: Identifier::from(TEST_IDENTITY_ID),
            amount: 500,
            core_fee_per_byte: 1,
            pooling: Default::default(),
            output_script: Default::default(),
            nonce: TEST_NONCE,
            user_fee_increase: TEST_FEE_INCREASE,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        }
    }

    fn make_credit_withdrawal_action() -> IdentityCreditWithdrawalTransitionActionV0 {
        IdentityCreditWithdrawalTransitionActionV0 {
            identity_id: Identifier::from(TEST_IDENTITY_ID),
            nonce: TEST_NONCE,
            prepared_withdrawal_document: Document::V0(Default::default()),
            amount: 500,
            user_fee_increase: TEST_FEE_INCREASE,
        }
    }

    fn assert_action_fields(action: &BumpIdentityNonceActionV0) {
        assert_eq!(action.identity_id, Identifier::from(TEST_IDENTITY_ID));
        assert_eq!(action.identity_nonce, TEST_NONCE);
        assert_eq!(action.user_fee_increase, TEST_FEE_INCREASE);
    }

    // ---- from_identity_update / from_borrowed_identity_update ----

    #[test]
    fn test_from_identity_update() {
        let transition = make_identity_update_transition();
        let action = BumpIdentityNonceActionV0::from_identity_update(transition);
        assert_action_fields(&action);
    }

    #[test]
    fn test_from_borrowed_identity_update() {
        let transition = make_identity_update_transition();
        let action = BumpIdentityNonceActionV0::from_borrowed_identity_update(&transition);
        assert_action_fields(&action);
    }

    // ---- from_identity_update_action / from_borrowed_identity_update_action ----

    #[test]
    fn test_from_identity_update_action() {
        let action_v0 = make_identity_update_action();
        let action = BumpIdentityNonceActionV0::from_identity_update_action(action_v0);
        assert_action_fields(&action);
    }

    #[test]
    fn test_from_borrowed_identity_update_action() {
        let action_v0 = make_identity_update_action();
        let action = BumpIdentityNonceActionV0::from_borrowed_identity_update_action(&action_v0);
        assert_action_fields(&action);
    }

    // ---- from_contract_create / from_borrowed_contract_create ----

    #[test]
    fn test_from_contract_create() {
        let transition = make_data_contract_create_transition();
        let action = BumpIdentityNonceActionV0::from_contract_create(transition);
        assert_eq!(action.identity_id, Identifier::from(TEST_IDENTITY_ID));
        assert_eq!(action.identity_nonce, TEST_NONCE);
        assert_eq!(action.user_fee_increase, TEST_FEE_INCREASE);
    }

    #[test]
    fn test_from_borrowed_contract_create() {
        let transition = make_data_contract_create_transition();
        let action = BumpIdentityNonceActionV0::from_borrowed_contract_create(&transition);
        assert_eq!(action.identity_id, Identifier::from(TEST_IDENTITY_ID));
        assert_eq!(action.identity_nonce, TEST_NONCE);
        assert_eq!(action.user_fee_increase, TEST_FEE_INCREASE);
    }

    // ---- from_contract_create_action / from_borrowed_contract_create_action ----

    #[test]
    fn test_from_contract_create_action() {
        let action_v0 = make_data_contract_create_action();
        let action = BumpIdentityNonceActionV0::from_contract_create_action(action_v0);
        assert_eq!(action.identity_id, Identifier::from(TEST_IDENTITY_ID));
        assert_eq!(action.identity_nonce, TEST_NONCE);
        assert_eq!(action.user_fee_increase, TEST_FEE_INCREASE);
    }

    #[test]
    fn test_from_borrowed_contract_create_action() {
        let action_v0 = make_data_contract_create_action();
        let action = BumpIdentityNonceActionV0::from_borrowed_contract_create_action(&action_v0);
        assert_eq!(action.identity_id, Identifier::from(TEST_IDENTITY_ID));
        assert_eq!(action.identity_nonce, TEST_NONCE);
        assert_eq!(action.user_fee_increase, TEST_FEE_INCREASE);
    }

    // ---- from_identity_credit_transfer / from_borrowed_identity_credit_transfer ----

    #[test]
    fn test_from_identity_credit_transfer() {
        let transition = make_credit_transfer_transition();
        let action = BumpIdentityNonceActionV0::from_identity_credit_transfer(transition);
        assert_action_fields(&action);
    }

    #[test]
    fn test_from_borrowed_identity_credit_transfer() {
        let transition = make_credit_transfer_transition();
        let action =
            BumpIdentityNonceActionV0::from_borrowed_identity_credit_transfer(&transition);
        assert_action_fields(&action);
    }

    // ---- from_identity_credit_transfer_action / from_borrowed_identity_credit_transfer_action ----

    #[test]
    fn test_from_identity_credit_transfer_action() {
        let action_v0 = make_credit_transfer_action();
        let action = BumpIdentityNonceActionV0::from_identity_credit_transfer_action(action_v0);
        assert_action_fields(&action);
    }

    #[test]
    fn test_from_borrowed_identity_credit_transfer_action() {
        let action_v0 = make_credit_transfer_action();
        let action =
            BumpIdentityNonceActionV0::from_borrowed_identity_credit_transfer_action(&action_v0);
        assert_action_fields(&action);
    }

    // ---- from_identity_credit_withdrawal / from_borrowed_identity_credit_withdrawal ----

    #[test]
    fn test_from_identity_credit_withdrawal_v0() {
        let v0 = make_credit_withdrawal_transition_v0();
        let withdrawal = IdentityCreditWithdrawalTransition::V0(v0);
        let action = BumpIdentityNonceActionV0::from_identity_credit_withdrawal(withdrawal);
        assert_action_fields(&action);
    }

    #[test]
    fn test_from_borrowed_identity_credit_withdrawal_v0() {
        let v0 = make_credit_withdrawal_transition_v0();
        let withdrawal = IdentityCreditWithdrawalTransition::V0(v0);
        let action =
            BumpIdentityNonceActionV0::from_borrowed_identity_credit_withdrawal(&withdrawal);
        assert_action_fields(&action);
    }

    // ---- from_identity_credit_withdrawal_action / from_borrowed_identity_credit_withdrawal_action ----

    #[test]
    fn test_from_identity_credit_withdrawal_action() {
        let action_v0 = make_credit_withdrawal_action();
        let action =
            BumpIdentityNonceActionV0::from_identity_credit_withdrawal_action(action_v0);
        assert_action_fields(&action);
    }

    #[test]
    fn test_from_borrowed_identity_credit_withdrawal_action() {
        let action_v0 = make_credit_withdrawal_action();
        let action =
            BumpIdentityNonceActionV0::from_borrowed_identity_credit_withdrawal_action(&action_v0);
        assert_action_fields(&action);
    }

    // ---- owned vs borrowed produce identical results ----

    #[test]
    fn test_owned_and_borrowed_identity_update_produce_same_result() {
        let transition = make_identity_update_transition();
        let borrowed_result =
            BumpIdentityNonceActionV0::from_borrowed_identity_update(&transition);
        let owned_result = BumpIdentityNonceActionV0::from_identity_update(transition);
        assert_eq!(owned_result.identity_id, borrowed_result.identity_id);
        assert_eq!(owned_result.identity_nonce, borrowed_result.identity_nonce);
        assert_eq!(
            owned_result.user_fee_increase,
            borrowed_result.user_fee_increase
        );
    }

    #[test]
    fn test_owned_and_borrowed_credit_transfer_produce_same_result() {
        let transition = make_credit_transfer_transition();
        let borrowed_result =
            BumpIdentityNonceActionV0::from_borrowed_identity_credit_transfer(&transition);
        let owned_result =
            BumpIdentityNonceActionV0::from_identity_credit_transfer(transition);
        assert_eq!(owned_result.identity_id, borrowed_result.identity_id);
        assert_eq!(owned_result.identity_nonce, borrowed_result.identity_nonce);
        assert_eq!(
            owned_result.user_fee_increase,
            borrowed_result.user_fee_increase
        );
    }

    #[test]
    fn test_owned_and_borrowed_withdrawal_action_produce_same_result() {
        let action_v0 = make_credit_withdrawal_action();
        let borrowed_result =
            BumpIdentityNonceActionV0::from_borrowed_identity_credit_withdrawal_action(&action_v0);
        let owned_result =
            BumpIdentityNonceActionV0::from_identity_credit_withdrawal_action(action_v0);
        assert_eq!(owned_result.identity_id, borrowed_result.identity_id);
        assert_eq!(owned_result.identity_nonce, borrowed_result.identity_nonce);
        assert_eq!(
            owned_result.user_fee_increase,
            borrowed_result.user_fee_increase
        );
    }
}
