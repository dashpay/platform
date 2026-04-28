use derive_more::From;
use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
use dpp::data_contract::TokenContractPosition;
use dpp::group::group_action::GroupAction;
use dpp::group::GroupStateTransitionResolvedInfo;
use dpp::platform_value::Identifier;
use dpp::prelude::IdentityNonce;
use std::sync::Arc;

/// transformer module
pub mod transformer;
mod v0;

use crate::drive::contract::DataContractFetchInfo;

use crate::error::Error;
pub use v0::*;

/// document base transition action
#[derive(Debug, Clone, From)]
pub enum TokenBaseTransitionAction {
    /// v0
    V0(TokenBaseTransitionActionV0),
}

impl TokenBaseTransitionActionAccessorsV0 for TokenBaseTransitionAction {
    fn token_position(&self) -> TokenContractPosition {
        match self {
            TokenBaseTransitionAction::V0(v0) => v0.token_contract_position,
        }
    }

    fn token_id(&self) -> Identifier {
        match self {
            TokenBaseTransitionAction::V0(v0) => v0.token_id,
        }
    }

    fn data_contract_id(&self) -> Identifier {
        match self {
            TokenBaseTransitionAction::V0(v0) => v0.data_contract_id(),
        }
    }

    fn data_contract_fetch_info_ref(&self) -> &Arc<DataContractFetchInfo> {
        match self {
            TokenBaseTransitionAction::V0(v0) => v0.data_contract_fetch_info_ref(),
        }
    }

    fn data_contract_fetch_info(&self) -> Arc<DataContractFetchInfo> {
        match self {
            TokenBaseTransitionAction::V0(v0) => v0.data_contract_fetch_info(),
        }
    }

    fn identity_contract_nonce(&self) -> IdentityNonce {
        match self {
            TokenBaseTransitionAction::V0(v0) => v0.identity_contract_nonce(),
        }
    }

    fn token_configuration(&self) -> Result<&TokenConfiguration, Error> {
        match self {
            TokenBaseTransitionAction::V0(v0) => v0.token_configuration(),
        }
    }

    fn store_in_group(&self) -> Option<&GroupStateTransitionResolvedInfo> {
        match self {
            TokenBaseTransitionAction::V0(v0) => v0.store_in_group(),
        }
    }

    fn original_group_action(&self) -> Option<&GroupAction> {
        match self {
            TokenBaseTransitionAction::V0(v0) => v0.original_group_action(),
        }
    }

    fn perform_action(&self) -> bool {
        match self {
            TokenBaseTransitionAction::V0(v0) => v0.perform_action(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::contract::DataContractFetchInfo;
    use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionV0;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::config::v0::DataContractConfigV0;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::group::v0::GroupV0;
    use dpp::data_contract::group::Group;
    use dpp::data_contract::v1::DataContractV1;
    use dpp::group::action_event::GroupActionEvent;
    use dpp::group::group_action::v0::GroupActionV0;
    use dpp::group::group_action::GroupAction;
    use dpp::group::GroupStateTransitionResolvedInfo;
    use dpp::prelude::DataContract;
    use dpp::tokens::token_event::TokenEvent;
    use grovedb_costs::OperationCost;
    use std::collections::BTreeMap;

    fn build_contract_with_token_positions(positions: &[u16]) -> DataContract {
        let mut tokens = BTreeMap::new();
        for p in positions {
            tokens.insert(
                *p,
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
            );
        }
        DataContract::V1(DataContractV1 {
            id: Identifier::new([9u8; 32]),
            version: 1,
            owner_id: Identifier::new([7u8; 32]),
            document_types: Default::default(),
            config: DataContractConfig::V0(DataContractConfigV0 {
                can_be_deleted: false,
                readonly: false,
                keeps_history: false,
                documents_keep_history_contract_default: false,
                documents_mutable_contract_default: false,
                documents_can_be_deleted_contract_default: false,
                requires_identity_encryption_bounded_key: None,
                requires_identity_decryption_bounded_key: None,
            }),
            schema_defs: None,
            created_at: None,
            updated_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            created_at_epoch: None,
            updated_at_epoch: None,
            groups: Default::default(),
            tokens,
            keywords: Vec::new(),
            description: None,
        })
    }

    fn enum_action_with(
        position: u16,
        store_in_group: Option<(GroupStateTransitionResolvedInfo, Option<GroupAction>)>,
        perform_action: bool,
    ) -> TokenBaseTransitionAction {
        let contract = build_contract_with_token_positions(&[0, position]);
        let v0 = TokenBaseTransitionActionV0 {
            token_id: Identifier::new([21u8; 32]),
            identity_contract_nonce: 77,
            token_contract_position: position,
            data_contract: Arc::new(DataContractFetchInfo {
                contract,
                storage_flags: None,
                cost: OperationCost::default(),
                fee: None,
            }),
            store_in_group,
            perform_action,
        };
        TokenBaseTransitionAction::V0(v0)
    }

    #[test]
    fn enum_wrapper_forwards_simple_accessors_to_v0() {
        let action = enum_action_with(5, None, true);

        assert_eq!(action.token_position(), 5);
        assert_eq!(action.token_id(), Identifier::new([21u8; 32]));
        assert_eq!(action.identity_contract_nonce(), 77);
        assert!(action.perform_action());
        assert!(action.store_in_group().is_none());
        assert!(action.original_group_action().is_none());

        // data_contract_id() dispatches through the V0 arm to contract.id()
        let expected_id = action.data_contract_fetch_info_ref().contract.id();
        assert_eq!(action.data_contract_id(), expected_id);
    }

    #[test]
    fn enum_wrapper_data_contract_fetch_info_variants_share_pointer() {
        let action = enum_action_with(0, None, true);
        let reference = action.data_contract_fetch_info_ref();
        let cloned = action.data_contract_fetch_info();
        assert!(Arc::ptr_eq(reference, &cloned));
    }

    #[test]
    fn enum_wrapper_token_configuration_success_and_error() {
        // Success: position 3 is present in the contract.
        let ok_action = enum_action_with(3, None, true);
        assert!(ok_action.token_configuration().is_ok());

        // Construct by hand with a position NOT present in the built contract.
        let contract = build_contract_with_token_positions(&[0]);
        let v0 = TokenBaseTransitionActionV0 {
            token_id: Identifier::new([22u8; 32]),
            identity_contract_nonce: 1,
            token_contract_position: 44,
            data_contract: Arc::new(DataContractFetchInfo {
                contract,
                storage_flags: None,
                cost: OperationCost::default(),
                fee: None,
            }),
            store_in_group: None,
            perform_action: true,
        };
        let err_action = TokenBaseTransitionAction::V0(v0);
        let err = err_action.token_configuration().expect_err("must error");
        assert!(format!("{err:?}").contains("44"));
    }

    #[test]
    fn enum_wrapper_store_in_group_and_original_group_action() {
        let resolved = GroupStateTransitionResolvedInfo {
            group_contract_position: 2,
            group: {
                let mut members = BTreeMap::new();
                members.insert(Identifier::new([8u8; 32]), 5u32);
                Group::V0(GroupV0 {
                    members,
                    required_power: 5,
                })
            },
            action_id: Identifier::new([99u8; 32]),
            action_is_proposer: false,
            signer_power: 5,
        };
        let original = GroupAction::V0(GroupActionV0 {
            contract_id: Identifier::new([1u8; 32]),
            proposer_id: Identifier::new([2u8; 32]),
            token_contract_position: 0,
            event: GroupActionEvent::TokenEvent(TokenEvent::Mint(
                7,
                Identifier::new([3u8; 32]),
                None,
            )),
        });

        let action = enum_action_with(0, Some((resolved.clone(), Some(original.clone()))), false);
        let store = action
            .store_in_group()
            .expect("wrapper must forward Some(resolved)");
        assert_eq!(store.group_contract_position, 2);
        assert_eq!(store.action_id, Identifier::new([99u8; 32]));

        let orig = action
            .original_group_action()
            .expect("wrapper must forward Some(GroupAction)");
        assert_eq!(orig, &original);
        assert!(!action.perform_action());
    }

    #[test]
    fn enum_from_v0_conversion_preserves_fields() {
        // The derive_more::From impl should yield the same enum as manual wrapping.
        let contract = build_contract_with_token_positions(&[0]);
        let v0 = TokenBaseTransitionActionV0 {
            token_id: Identifier::new([30u8; 32]),
            identity_contract_nonce: 9,
            token_contract_position: 0,
            data_contract: Arc::new(DataContractFetchInfo {
                contract,
                storage_flags: None,
                cost: OperationCost::default(),
                fee: None,
            }),
            store_in_group: None,
            perform_action: true,
        };

        let wrapped: TokenBaseTransitionAction = v0.clone().into();
        match &wrapped {
            TokenBaseTransitionAction::V0(inner) => {
                assert_eq!(inner.token_id, v0.token_id);
                assert_eq!(inner.identity_contract_nonce, v0.identity_contract_nonce);
                assert_eq!(inner.token_contract_position, v0.token_contract_position);
                assert!(Arc::ptr_eq(&inner.data_contract, &v0.data_contract));
            }
        }
    }
}
