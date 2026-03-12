use dpp::platform_value::Identifier;
use dpp::prelude::UserFeeIncrease;
use dpp::ProtocolError;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::token_transition::TokenTransitionV0Methods;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::contract::data_contract_update::DataContractUpdateTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::{BumpIdentityDataContractNonceAction, BumpIdentityDataContractNonceActionV0};

impl BumpIdentityDataContractNonceAction {
    /// from borrowed base transition
    pub fn from_batched_transition_ref(
        value: BatchedTransitionRef,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        match value {
            BatchedTransitionRef::Document(document) => {
                Self::from_borrowed_document_base_transition(
                    document.base(),
                    identity_id,
                    user_fee_increase,
                )
            }
            BatchedTransitionRef::Token(token) => Self::from_borrowed_token_base_transition(
                token.base(),
                identity_id,
                user_fee_increase,
            ),
        }
    }

    /// helper method
    pub fn try_from_batched_transition_action(
        value: BatchedTransitionAction,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Result<Self, Error> {
        match value {
            BatchedTransitionAction::DocumentAction(document) => {
                Ok(Self::from_document_base_transition_action(
                    document.base_owned(),
                    identity_id,
                    user_fee_increase,
                ))
            }
            BatchedTransitionAction::TokenAction(token) => Ok(Self::from_token_base_transition_action(
                token.base_owned(),
                identity_id,
                user_fee_increase,
            )),
            BatchedTransitionAction::BumpIdentityDataContractNonce(_) => {
                Err(Error::Protocol(Box::new(
                    ProtocolError::CorruptedCodeExecution(
                        "we should never be trying to convert from a BumpIdentityDataContractNonce to a BumpIdentityDataContractNonceAction".to_string(),
                    ),
                )))
            }
        }
    }

    /// helper method
    pub fn try_from_borrowed_batched_transition_action(
        value: &BatchedTransitionAction,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Result<Self, Error> {
        match value {
            BatchedTransitionAction::DocumentAction(document) => {
                Ok(Self::from_borrowed_document_base_transition_action(
                    document.base(),
                    identity_id,
                    user_fee_increase,
                ))
            }
            BatchedTransitionAction::TokenAction(token) => Ok(Self::from_borrowed_token_base_transition_action(
                token.base(),
                identity_id,
                user_fee_increase,
            )),
            BatchedTransitionAction::BumpIdentityDataContractNonce(_) => {
                Err(Error::Protocol(Box::new(
                    ProtocolError::CorruptedCodeExecution(
                        "we should never be trying to convert from a BumpIdentityDataContractNonce to a BumpIdentityDataContractNonceAction".to_string(),
                    ),
                )))
            }
        }
    }

    /// from base transition
    pub fn from_document_base_transition(
        value: DocumentBaseTransition,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpIdentityDataContractNonceActionV0::from_document_base_transition(
            value,
            identity_id,
            user_fee_increase,
        )
        .into()
    }

    /// from borrowed base transition
    pub fn from_borrowed_document_base_transition(
        value: &DocumentBaseTransition,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpIdentityDataContractNonceActionV0::from_borrowed_document_base_transition(
            value,
            identity_id,
            user_fee_increase,
        )
        .into()
    }

    /// from base transition
    pub fn from_document_base_transition_action(
        value: DocumentBaseTransitionAction,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpIdentityDataContractNonceActionV0::from_document_base_transition_action(
            value,
            identity_id,
            user_fee_increase,
        )
        .into()
    }

    /// from borrowed base transition
    pub fn from_borrowed_document_base_transition_action(
        value: &DocumentBaseTransitionAction,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpIdentityDataContractNonceActionV0::from_borrowed_document_base_transition_action(
            value,
            identity_id,
            user_fee_increase,
        )
        .into()
    }

    /// from base transition
    pub fn from_token_base_transition(
        value: TokenBaseTransition,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpIdentityDataContractNonceActionV0::from_token_base_transition(
            value,
            identity_id,
            user_fee_increase,
        )
        .into()
    }

    /// from borrowed base transition
    pub fn from_borrowed_token_base_transition(
        value: &TokenBaseTransition,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpIdentityDataContractNonceActionV0::from_borrowed_token_base_transition(
            value,
            identity_id,
            user_fee_increase,
        )
        .into()
    }

    /// from base transition
    pub fn from_token_base_transition_action(
        value: TokenBaseTransitionAction,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpIdentityDataContractNonceActionV0::from_token_base_transition_action(
            value,
            identity_id,
            user_fee_increase,
        )
        .into()
    }

    /// from borrowed base transition
    pub fn from_borrowed_token_base_transition_action(
        value: &TokenBaseTransitionAction,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpIdentityDataContractNonceActionV0::from_borrowed_token_base_transition_action(
            value,
            identity_id,
            user_fee_increase,
        )
        .into()
    }

    /// from data contract update
    pub fn from_data_contract_update_transition(value: DataContractUpdateTransition) -> Self {
        match value {
            DataContractUpdateTransition::V0(v0) => {
                BumpIdentityDataContractNonceActionV0::from_data_contract_update(v0).into()
            }
        }
    }

    /// from borrowed data contract update
    pub fn from_borrowed_data_contract_update_transition(
        value: &DataContractUpdateTransition,
    ) -> Self {
        match value {
            DataContractUpdateTransition::V0(v0) => {
                BumpIdentityDataContractNonceActionV0::from_borrowed_data_contract_update(v0).into()
            }
        }
    }

    /// from data contract update action
    pub fn from_data_contract_update_transition_action(
        value: DataContractUpdateTransitionAction,
    ) -> Self {
        match value {
            DataContractUpdateTransitionAction::V0(v0) => {
                BumpIdentityDataContractNonceActionV0::from_data_contract_update_action(v0).into()
            }
        }
    }

    /// from borrowed data contract update action
    pub fn from_borrowed_data_contract_update_transition_action(
        value: &DataContractUpdateTransitionAction,
    ) -> Self {
        match value {
            DataContractUpdateTransitionAction::V0(v0) => {
                BumpIdentityDataContractNonceActionV0::from_borrowed_data_contract_update_action(v0)
                    .into()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::contract::DataContractFetchInfo;
    use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{
        DocumentBaseTransitionActionV0,
    };
    use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionV0;
    use crate::state_transition_action::contract::data_contract_update::v0::DataContractUpdateTransitionActionV0;
    use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceActionAccessorsV0;
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

    fn assert_v0_fields(action: &BumpIdentityDataContractNonceAction, expected_contract_id: Identifier) {
        match action {
            BumpIdentityDataContractNonceAction::V0(v0) => {
                assert_eq!(v0.identity_id, Identifier::from(TEST_IDENTITY_ID));
                assert_eq!(v0.data_contract_id, expected_contract_id);
                assert_eq!(v0.identity_contract_nonce, TEST_NONCE);
                assert_eq!(v0.user_fee_increase, TEST_FEE);
            }
        }
    }

    // ---- DocumentBaseTransition ----

    #[test]
    fn test_from_document_base_transition() {
        let base = DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Identifier::from([0xCC; 32]),
            identity_contract_nonce: TEST_NONCE,
            document_type_name: "test".to_string(),
            data_contract_id: Identifier::from(TEST_CONTRACT_ID),
        });
        let action = BumpIdentityDataContractNonceAction::from_document_base_transition(
            base,
            Identifier::from(TEST_IDENTITY_ID),
            TEST_FEE,
        );
        assert_v0_fields(&action, Identifier::from(TEST_CONTRACT_ID));
    }

    #[test]
    fn test_from_borrowed_document_base_transition() {
        let base = DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Identifier::from([0xCC; 32]),
            identity_contract_nonce: TEST_NONCE,
            document_type_name: "test".to_string(),
            data_contract_id: Identifier::from(TEST_CONTRACT_ID),
        });
        let action = BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition(
            &base,
            Identifier::from(TEST_IDENTITY_ID),
            TEST_FEE,
        );
        assert_v0_fields(&action, Identifier::from(TEST_CONTRACT_ID));
    }

    // ---- DocumentBaseTransitionAction ----

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
        let action = BumpIdentityDataContractNonceAction::from_document_base_transition_action(
            base_action,
            Identifier::from(TEST_IDENTITY_ID),
            TEST_FEE,
        );
        assert_v0_fields(&action, contract_id);
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
            BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition_action(
                &base_action,
                Identifier::from(TEST_IDENTITY_ID),
                TEST_FEE,
            );
        assert_v0_fields(&action, contract_id);
    }

    // ---- TokenBaseTransition ----

    #[test]
    fn test_from_token_base_transition() {
        let base = TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: TEST_NONCE,
            token_contract_position: 0,
            data_contract_id: Identifier::from(TEST_CONTRACT_ID),
            token_id: Identifier::from([0xDD; 32]),
            using_group_info: None,
        });
        let action = BumpIdentityDataContractNonceAction::from_token_base_transition(
            base,
            Identifier::from(TEST_IDENTITY_ID),
            TEST_FEE,
        );
        assert_v0_fields(&action, Identifier::from(TEST_CONTRACT_ID));
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
        let action = BumpIdentityDataContractNonceAction::from_borrowed_token_base_transition(
            &base,
            Identifier::from(TEST_IDENTITY_ID),
            TEST_FEE,
        );
        assert_v0_fields(&action, Identifier::from(TEST_CONTRACT_ID));
    }

    // ---- TokenBaseTransitionAction ----

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
        let action = BumpIdentityDataContractNonceAction::from_token_base_transition_action(
            base_action,
            Identifier::from(TEST_IDENTITY_ID),
            TEST_FEE,
        );
        assert_v0_fields(&action, contract_id);
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
            BumpIdentityDataContractNonceAction::from_borrowed_token_base_transition_action(
                &base_action,
                Identifier::from(TEST_IDENTITY_ID),
                TEST_FEE,
            );
        assert_v0_fields(&action, contract_id);
    }

    // ---- DataContractUpdateTransition ----

    #[test]
    fn test_from_data_contract_update_transition() {
        let platform_version = PlatformVersion::latest();
        let data_contract = make_test_data_contract();
        let owner_id = data_contract.owner_id();
        let contract_id = data_contract.id();
        let serialized: dpp::data_contract::serialized_version::DataContractInSerializationFormat =
            data_contract
                .try_into_platform_versioned(platform_version)
                .expect("serialize");
        let v0 = dpp::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0 {
            identity_contract_nonce: TEST_NONCE,
            data_contract: serialized,
            user_fee_increase: TEST_FEE,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        };
        let transition = dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition::V0(v0);
        let action =
            BumpIdentityDataContractNonceAction::from_data_contract_update_transition(transition);
        assert_eq!(action.identity_id(), owner_id);
        assert_eq!(action.data_contract_id(), contract_id);
        assert_eq!(action.identity_contract_nonce(), TEST_NONCE);
        assert_eq!(action.user_fee_increase(), TEST_FEE);
    }

    #[test]
    fn test_from_borrowed_data_contract_update_transition() {
        let platform_version = PlatformVersion::latest();
        let data_contract = make_test_data_contract();
        let owner_id = data_contract.owner_id();
        let contract_id = data_contract.id();
        let serialized: dpp::data_contract::serialized_version::DataContractInSerializationFormat =
            data_contract
                .try_into_platform_versioned(platform_version)
                .expect("serialize");
        let v0 = dpp::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0 {
            identity_contract_nonce: TEST_NONCE,
            data_contract: serialized,
            user_fee_increase: TEST_FEE,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        };
        let transition = dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition::V0(v0);
        let action =
            BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                &transition,
            );
        assert_eq!(action.identity_id(), owner_id);
        assert_eq!(action.data_contract_id(), contract_id);
        assert_eq!(action.identity_contract_nonce(), TEST_NONCE);
        assert_eq!(action.user_fee_increase(), TEST_FEE);
    }

    // ---- DataContractUpdateTransitionAction ----

    #[test]
    fn test_from_data_contract_update_transition_action() {
        let data_contract = make_test_data_contract();
        let owner_id = data_contract.owner_id();
        let contract_id = data_contract.id();
        let v0 = DataContractUpdateTransitionActionV0 {
            data_contract,
            identity_contract_nonce: TEST_NONCE,
            user_fee_increase: TEST_FEE,
        };
        let action_enum = DataContractUpdateTransitionAction::V0(v0);
        let action =
            BumpIdentityDataContractNonceAction::from_data_contract_update_transition_action(
                action_enum,
            );
        assert_eq!(action.identity_id(), owner_id);
        assert_eq!(action.data_contract_id(), contract_id);
        assert_eq!(action.identity_contract_nonce(), TEST_NONCE);
        assert_eq!(action.user_fee_increase(), TEST_FEE);
    }

    #[test]
    fn test_from_borrowed_data_contract_update_transition_action() {
        let data_contract = make_test_data_contract();
        let owner_id = data_contract.owner_id();
        let contract_id = data_contract.id();
        let v0 = DataContractUpdateTransitionActionV0 {
            data_contract,
            identity_contract_nonce: TEST_NONCE,
            user_fee_increase: TEST_FEE,
        };
        let action_enum = DataContractUpdateTransitionAction::V0(v0);
        let action =
            BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition_action(
                &action_enum,
            );
        assert_eq!(action.identity_id(), owner_id);
        assert_eq!(action.data_contract_id(), contract_id);
        assert_eq!(action.identity_contract_nonce(), TEST_NONCE);
        assert_eq!(action.user_fee_increase(), TEST_FEE);
    }
}
