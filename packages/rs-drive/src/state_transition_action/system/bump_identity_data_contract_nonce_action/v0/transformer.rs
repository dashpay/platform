use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::platform_value::Identifier;
use dpp::prelude::UserFeeIncrease;
use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0;
use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use crate::state_transition_action::contract::data_contract_update::v0::DataContractUpdateTransitionActionV0;
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceActionV0;

impl BumpIdentityDataContractNonceActionV0 {
    /// from base transition
    pub fn from_document_base_transition(
        value: DocumentBaseTransition,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpIdentityDataContractNonceActionV0 {
            identity_id,
            data_contract_id: value.data_contract_id(),
            identity_contract_nonce: value.identity_contract_nonce(),
            user_fee_increase,
        }
    }

    /// from borrowed base transition
    pub fn from_borrowed_document_base_transition(
        value: &DocumentBaseTransition,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpIdentityDataContractNonceActionV0 {
            identity_id,
            data_contract_id: value.data_contract_id(),
            identity_contract_nonce: value.identity_contract_nonce(),
            user_fee_increase,
        }
    }

    /// from base transition
    pub fn from_document_base_transition_action(
        value: DocumentBaseTransitionAction,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpIdentityDataContractNonceActionV0 {
            identity_id,
            data_contract_id: value.data_contract_id(),
            identity_contract_nonce: value.identity_contract_nonce(),
            user_fee_increase,
        }
    }

    /// from borrowed base transition
    pub fn from_borrowed_document_base_transition_action(
        value: &DocumentBaseTransitionAction,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpIdentityDataContractNonceActionV0 {
            identity_id,
            data_contract_id: value.data_contract_id(),
            identity_contract_nonce: value.identity_contract_nonce(),
            user_fee_increase,
        }
    }

    /// from base transition
    pub fn from_token_base_transition(
        value: TokenBaseTransition,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpIdentityDataContractNonceActionV0 {
            identity_id,
            data_contract_id: value.data_contract_id(),
            identity_contract_nonce: value.identity_contract_nonce(),
            user_fee_increase,
        }
    }

    /// from borrowed base transition
    pub fn from_borrowed_token_base_transition(
        value: &TokenBaseTransition,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpIdentityDataContractNonceActionV0 {
            identity_id,
            data_contract_id: value.data_contract_id(),
            identity_contract_nonce: value.identity_contract_nonce(),
            user_fee_increase,
        }
    }

    /// from base transition
    pub fn from_token_base_transition_action(
        value: TokenBaseTransitionAction,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpIdentityDataContractNonceActionV0 {
            identity_id,
            data_contract_id: value.data_contract_id(),
            identity_contract_nonce: value.identity_contract_nonce(),
            user_fee_increase,
        }
    }

    /// from borrowed base transition
    pub fn from_borrowed_token_base_transition_action(
        value: &TokenBaseTransitionAction,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpIdentityDataContractNonceActionV0 {
            identity_id,
            data_contract_id: value.data_contract_id(),
            identity_contract_nonce: value.identity_contract_nonce(),
            user_fee_increase,
        }
    }

    /// from data contract update
    pub fn from_data_contract_update(value: DataContractUpdateTransitionV0) -> Self {
        let DataContractUpdateTransitionV0 {
            data_contract,
            identity_contract_nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityDataContractNonceActionV0 {
            identity_id: data_contract.owner_id(),
            data_contract_id: data_contract.id(),
            identity_contract_nonce,
            user_fee_increase,
        }
    }

    /// from borrowed data contract update
    pub fn from_borrowed_data_contract_update(value: &DataContractUpdateTransitionV0) -> Self {
        let DataContractUpdateTransitionV0 {
            data_contract,
            identity_contract_nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityDataContractNonceActionV0 {
            identity_id: data_contract.owner_id(),
            data_contract_id: data_contract.id(),
            identity_contract_nonce: *identity_contract_nonce,
            user_fee_increase: *user_fee_increase,
        }
    }

    /// from data contract update action
    pub fn from_data_contract_update_action(value: DataContractUpdateTransitionActionV0) -> Self {
        let DataContractUpdateTransitionActionV0 {
            data_contract,
            identity_contract_nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityDataContractNonceActionV0 {
            identity_id: data_contract.owner_id(),
            data_contract_id: data_contract.id(),
            identity_contract_nonce,
            user_fee_increase,
        }
    }

    /// from borrowed data contract update action
    pub fn from_borrowed_data_contract_update_action(
        value: &DataContractUpdateTransitionActionV0,
    ) -> Self {
        let DataContractUpdateTransitionActionV0 {
            data_contract,
            identity_contract_nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityDataContractNonceActionV0 {
            identity_id: data_contract.owner_id(),
            data_contract_id: data_contract.id(),
            identity_contract_nonce: *identity_contract_nonce,
            user_fee_increase: *user_fee_increase,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::contract::DataContractFetchInfo;
    use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionV0;
    use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionV0;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::identifier::Identifier;
    use dpp::platform_value::BinaryData;
    use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use dpp::version::PlatformVersion;
    use platform_version::TryIntoPlatformVersioned;
    use std::sync::Arc;

    const TEST_IDENTITY_ID: [u8; 32] = [0xAA; 32];
    const TEST_CONTRACT_ID: [u8; 32] = [0xBB; 32];
    const TEST_NONCE: u64 = 77;
    const TEST_FEE: u16 = 3;

    fn make_test_data_contract() -> dpp::data_contract::DataContract {
        let platform_version = PlatformVersion::latest();
        get_data_contract_fixture(
            Some(Identifier::from(TEST_IDENTITY_ID)),
            0,
            platform_version.protocol_version,
        )
        .data_contract_owned()
    }

    fn make_data_contract_fetch_info(
        data_contract: dpp::data_contract::DataContract,
    ) -> Arc<DataContractFetchInfo> {
        Arc::new(DataContractFetchInfo {
            contract: data_contract,
            storage_flags: None,
            cost: Default::default(),
            fee: None,
        })
    }

    // ---- DocumentBaseTransition tests ----

    #[test]
    fn test_from_document_base_transition() {
        let base = DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Identifier::from([0xCC; 32]),
            identity_contract_nonce: TEST_NONCE,
            document_type_name: "test".to_string(),
            data_contract_id: Identifier::from(TEST_CONTRACT_ID),
        });
        let action = BumpIdentityDataContractNonceActionV0::from_document_base_transition(
            base,
            Identifier::from(TEST_IDENTITY_ID),
            TEST_FEE,
        );
        assert_eq!(action.identity_id, Identifier::from(TEST_IDENTITY_ID));
        assert_eq!(action.data_contract_id, Identifier::from(TEST_CONTRACT_ID));
        assert_eq!(action.identity_contract_nonce, TEST_NONCE);
        assert_eq!(action.user_fee_increase, TEST_FEE);
    }

    #[test]
    fn test_from_borrowed_document_base_transition() {
        let base = DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Identifier::from([0xCC; 32]),
            identity_contract_nonce: TEST_NONCE,
            document_type_name: "test".to_string(),
            data_contract_id: Identifier::from(TEST_CONTRACT_ID),
        });
        let action =
            BumpIdentityDataContractNonceActionV0::from_borrowed_document_base_transition(
                &base,
                Identifier::from(TEST_IDENTITY_ID),
                TEST_FEE,
            );
        assert_eq!(action.identity_id, Identifier::from(TEST_IDENTITY_ID));
        assert_eq!(action.data_contract_id, Identifier::from(TEST_CONTRACT_ID));
        assert_eq!(action.identity_contract_nonce, TEST_NONCE);
        assert_eq!(action.user_fee_increase, TEST_FEE);
    }

    // ---- DocumentBaseTransitionAction tests ----

    #[test]
    fn test_from_document_base_transition_action() {
        let data_contract = make_test_data_contract();
        let contract_id = data_contract.id();
        let fetch_info = make_data_contract_fetch_info(data_contract);
        let base_action = DocumentBaseTransitionAction::V0(DocumentBaseTransitionActionV0 {
            id: Identifier::from([0xCC; 32]),
            identity_contract_nonce: TEST_NONCE,
            document_type_name: "niceDocument".to_string(),
            data_contract: fetch_info,
            token_cost: None,
            gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
        });
        let action =
            BumpIdentityDataContractNonceActionV0::from_document_base_transition_action(
                base_action,
                Identifier::from(TEST_IDENTITY_ID),
                TEST_FEE,
            );
        assert_eq!(action.identity_id, Identifier::from(TEST_IDENTITY_ID));
        assert_eq!(action.data_contract_id, contract_id);
        assert_eq!(action.identity_contract_nonce, TEST_NONCE);
        assert_eq!(action.user_fee_increase, TEST_FEE);
    }

    #[test]
    fn test_from_borrowed_document_base_transition_action() {
        let data_contract = make_test_data_contract();
        let contract_id = data_contract.id();
        let fetch_info = make_data_contract_fetch_info(data_contract);
        let base_action = DocumentBaseTransitionAction::V0(DocumentBaseTransitionActionV0 {
            id: Identifier::from([0xCC; 32]),
            identity_contract_nonce: TEST_NONCE,
            document_type_name: "niceDocument".to_string(),
            data_contract: fetch_info,
            token_cost: None,
            gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
        });
        let action =
            BumpIdentityDataContractNonceActionV0::from_borrowed_document_base_transition_action(
                &base_action,
                Identifier::from(TEST_IDENTITY_ID),
                TEST_FEE,
            );
        assert_eq!(action.identity_id, Identifier::from(TEST_IDENTITY_ID));
        assert_eq!(action.data_contract_id, contract_id);
        assert_eq!(action.identity_contract_nonce, TEST_NONCE);
        assert_eq!(action.user_fee_increase, TEST_FEE);
    }

    // ---- TokenBaseTransition tests ----

    #[test]
    fn test_from_token_base_transition() {
        let base = TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: TEST_NONCE,
            token_contract_position: 0,
            data_contract_id: Identifier::from(TEST_CONTRACT_ID),
            token_id: Identifier::from([0xDD; 32]),
            using_group_info: None,
        });
        let action = BumpIdentityDataContractNonceActionV0::from_token_base_transition(
            base,
            Identifier::from(TEST_IDENTITY_ID),
            TEST_FEE,
        );
        assert_eq!(action.identity_id, Identifier::from(TEST_IDENTITY_ID));
        assert_eq!(action.data_contract_id, Identifier::from(TEST_CONTRACT_ID));
        assert_eq!(action.identity_contract_nonce, TEST_NONCE);
        assert_eq!(action.user_fee_increase, TEST_FEE);
    }

    #[test]
    fn test_from_borrowed_token_base_transition() {
        let base = TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: TEST_NONCE,
            token_contract_position: 0,
            data_contract_id: Identifier::from(TEST_CONTRACT_ID),
            token_id: Identifier::from([0xDD; 32]),
            using_group_info: None,
        });
        let action = BumpIdentityDataContractNonceActionV0::from_borrowed_token_base_transition(
            &base,
            Identifier::from(TEST_IDENTITY_ID),
            TEST_FEE,
        );
        assert_eq!(action.identity_id, Identifier::from(TEST_IDENTITY_ID));
        assert_eq!(action.data_contract_id, Identifier::from(TEST_CONTRACT_ID));
        assert_eq!(action.identity_contract_nonce, TEST_NONCE);
        assert_eq!(action.user_fee_increase, TEST_FEE);
    }

    // ---- TokenBaseTransitionAction tests ----

    #[test]
    fn test_from_token_base_transition_action() {
        let data_contract = make_test_data_contract();
        let contract_id = data_contract.id();
        let fetch_info = make_data_contract_fetch_info(data_contract);
        let base_action = TokenBaseTransitionAction::V0(TokenBaseTransitionActionV0 {
            token_id: Identifier::from([0xDD; 32]),
            identity_contract_nonce: TEST_NONCE,
            token_contract_position: 0,
            data_contract: fetch_info,
            store_in_group: None,
            perform_action: true,
        });
        let action = BumpIdentityDataContractNonceActionV0::from_token_base_transition_action(
            base_action,
            Identifier::from(TEST_IDENTITY_ID),
            TEST_FEE,
        );
        assert_eq!(action.identity_id, Identifier::from(TEST_IDENTITY_ID));
        assert_eq!(action.data_contract_id, contract_id);
        assert_eq!(action.identity_contract_nonce, TEST_NONCE);
        assert_eq!(action.user_fee_increase, TEST_FEE);
    }

    #[test]
    fn test_from_borrowed_token_base_transition_action() {
        let data_contract = make_test_data_contract();
        let contract_id = data_contract.id();
        let fetch_info = make_data_contract_fetch_info(data_contract);
        let base_action = TokenBaseTransitionAction::V0(TokenBaseTransitionActionV0 {
            token_id: Identifier::from([0xDD; 32]),
            identity_contract_nonce: TEST_NONCE,
            token_contract_position: 0,
            data_contract: fetch_info,
            store_in_group: None,
            perform_action: true,
        });
        let action =
            BumpIdentityDataContractNonceActionV0::from_borrowed_token_base_transition_action(
                &base_action,
                Identifier::from(TEST_IDENTITY_ID),
                TEST_FEE,
            );
        assert_eq!(action.identity_id, Identifier::from(TEST_IDENTITY_ID));
        assert_eq!(action.data_contract_id, contract_id);
        assert_eq!(action.identity_contract_nonce, TEST_NONCE);
        assert_eq!(action.user_fee_increase, TEST_FEE);
    }

    // ---- DataContractUpdate tests ----

    #[test]
    fn test_from_data_contract_update() {
        let platform_version = PlatformVersion::latest();
        let data_contract = make_test_data_contract();
        let owner_id = data_contract.owner_id();
        let contract_id = data_contract.id();
        let serialized: dpp::data_contract::serialized_version::DataContractInSerializationFormat =
            data_contract
                .try_into_platform_versioned(platform_version)
                .expect("serialize");
        let transition = DataContractUpdateTransitionV0 {
            identity_contract_nonce: TEST_NONCE,
            data_contract: serialized,
            user_fee_increase: TEST_FEE,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        };
        let action =
            BumpIdentityDataContractNonceActionV0::from_data_contract_update(transition);
        assert_eq!(action.identity_id, owner_id);
        assert_eq!(action.data_contract_id, contract_id);
        assert_eq!(action.identity_contract_nonce, TEST_NONCE);
        assert_eq!(action.user_fee_increase, TEST_FEE);
    }

    #[test]
    fn test_from_borrowed_data_contract_update() {
        let platform_version = PlatformVersion::latest();
        let data_contract = make_test_data_contract();
        let owner_id = data_contract.owner_id();
        let contract_id = data_contract.id();
        let serialized: dpp::data_contract::serialized_version::DataContractInSerializationFormat =
            data_contract
                .try_into_platform_versioned(platform_version)
                .expect("serialize");
        let transition = DataContractUpdateTransitionV0 {
            identity_contract_nonce: TEST_NONCE,
            data_contract: serialized,
            user_fee_increase: TEST_FEE,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        };
        let action =
            BumpIdentityDataContractNonceActionV0::from_borrowed_data_contract_update(&transition);
        assert_eq!(action.identity_id, owner_id);
        assert_eq!(action.data_contract_id, contract_id);
        assert_eq!(action.identity_contract_nonce, TEST_NONCE);
        assert_eq!(action.user_fee_increase, TEST_FEE);
    }

    // ---- DataContractUpdateAction tests ----

    #[test]
    fn test_from_data_contract_update_action() {
        let data_contract = make_test_data_contract();
        let owner_id = data_contract.owner_id();
        let contract_id = data_contract.id();
        let action_v0 = DataContractUpdateTransitionActionV0 {
            data_contract,
            identity_contract_nonce: TEST_NONCE,
            user_fee_increase: TEST_FEE,
        };
        let action =
            BumpIdentityDataContractNonceActionV0::from_data_contract_update_action(action_v0);
        assert_eq!(action.identity_id, owner_id);
        assert_eq!(action.data_contract_id, contract_id);
        assert_eq!(action.identity_contract_nonce, TEST_NONCE);
        assert_eq!(action.user_fee_increase, TEST_FEE);
    }

    #[test]
    fn test_from_borrowed_data_contract_update_action() {
        let data_contract = make_test_data_contract();
        let owner_id = data_contract.owner_id();
        let contract_id = data_contract.id();
        let action_v0 = DataContractUpdateTransitionActionV0 {
            data_contract,
            identity_contract_nonce: TEST_NONCE,
            user_fee_increase: TEST_FEE,
        };
        let action =
            BumpIdentityDataContractNonceActionV0::from_borrowed_data_contract_update_action(
                &action_v0,
            );
        assert_eq!(action.identity_id, owner_id);
        assert_eq!(action.data_contract_id, contract_id);
        assert_eq!(action.identity_contract_nonce, TEST_NONCE);
        assert_eq!(action.user_fee_increase, TEST_FEE);
    }

    // ---- owned vs borrowed produce same results ----

    #[test]
    fn test_owned_and_borrowed_document_base_transition_produce_same_result() {
        let base = DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Identifier::from([0xCC; 32]),
            identity_contract_nonce: TEST_NONCE,
            document_type_name: "test".to_string(),
            data_contract_id: Identifier::from(TEST_CONTRACT_ID),
        });
        let borrowed_result =
            BumpIdentityDataContractNonceActionV0::from_borrowed_document_base_transition(
                &base,
                Identifier::from(TEST_IDENTITY_ID),
                TEST_FEE,
            );
        let owned_result =
            BumpIdentityDataContractNonceActionV0::from_document_base_transition(
                base,
                Identifier::from(TEST_IDENTITY_ID),
                TEST_FEE,
            );
        assert_eq!(owned_result.identity_id, borrowed_result.identity_id);
        assert_eq!(owned_result.data_contract_id, borrowed_result.data_contract_id);
        assert_eq!(
            owned_result.identity_contract_nonce,
            borrowed_result.identity_contract_nonce
        );
        assert_eq!(
            owned_result.user_fee_increase,
            borrowed_result.user_fee_increase
        );
    }

    #[test]
    fn test_owned_and_borrowed_token_base_transition_produce_same_result() {
        let base = TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: TEST_NONCE,
            token_contract_position: 0,
            data_contract_id: Identifier::from(TEST_CONTRACT_ID),
            token_id: Identifier::from([0xDD; 32]),
            using_group_info: None,
        });
        let borrowed_result =
            BumpIdentityDataContractNonceActionV0::from_borrowed_token_base_transition(
                &base,
                Identifier::from(TEST_IDENTITY_ID),
                TEST_FEE,
            );
        let owned_result =
            BumpIdentityDataContractNonceActionV0::from_token_base_transition(
                base,
                Identifier::from(TEST_IDENTITY_ID),
                TEST_FEE,
            );
        assert_eq!(owned_result.identity_id, borrowed_result.identity_id);
        assert_eq!(owned_result.data_contract_id, borrowed_result.data_contract_id);
        assert_eq!(
            owned_result.identity_contract_nonce,
            borrowed_result.identity_contract_nonce
        );
        assert_eq!(
            owned_result.user_fee_increase,
            borrowed_result.user_fee_increase
        );
    }
}
