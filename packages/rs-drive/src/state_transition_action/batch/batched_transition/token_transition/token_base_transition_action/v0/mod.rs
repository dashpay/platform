use crate::drive::contract::DataContractFetchInfo;
use crate::error::Error;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
use dpp::data_contract::TokenContractPosition;
use dpp::group::group_action::GroupAction;
use dpp::group::GroupStateTransitionResolvedInfo;
use dpp::identifier::Identifier;
use dpp::prelude::IdentityNonce;
use dpp::ProtocolError;
use std::sync::Arc;

/// transformer
pub mod transformer;

/// A type for the original group action if one exists
pub type OriginalGroupAction = Option<GroupAction>;

/// Token base transition action v0
#[derive(Debug, Clone)]
pub struct TokenBaseTransitionActionV0 {
    /// Token Id
    pub token_id: Identifier,
    /// The identity contract nonce, used to prevent replay attacks
    pub identity_contract_nonce: IdentityNonce,
    /// The token position within the data contract
    pub token_contract_position: u16,
    /// A potential data contract
    pub data_contract: Arc<DataContractFetchInfo>,
    /// Using group multi party rules for authentication
    /// If this is set we should store in group
    pub store_in_group: Option<(GroupStateTransitionResolvedInfo, OriginalGroupAction)>,
    /// Should the action be performed.
    /// This is true if we don't store in group.
    /// And also true if we store in group and with this have enough signatures to perform the action
    pub perform_action: bool,
}

/// Token base transition action accessors v0
pub trait TokenBaseTransitionActionAccessorsV0 {
    /// The token position within the data contract
    fn token_position(&self) -> TokenContractPosition;

    /// The token id
    fn token_id(&self) -> Identifier;

    /// Returns the data contract ID
    fn data_contract_id(&self) -> Identifier;

    /// Returns a reference to the data contract fetch info, without cloning
    fn data_contract_fetch_info_ref(&self) -> &Arc<DataContractFetchInfo>;

    /// Returns the data contract fetch info (cloned Arc)
    fn data_contract_fetch_info(&self) -> Arc<DataContractFetchInfo>;

    /// Returns the identity contract nonce
    fn identity_contract_nonce(&self) -> IdentityNonce;

    /// Gets the token configuration associated to the action
    fn token_configuration(&self) -> Result<&TokenConfiguration, Error>;

    /// Gets the store_in_group field (optional)
    fn store_in_group(&self) -> Option<&GroupStateTransitionResolvedInfo>;

    /// Gets the original group action if we are in a group action, and we are not the proposer
    fn original_group_action(&self) -> Option<&GroupAction>;

    /// Gets the perform_action field
    fn perform_action(&self) -> bool;
}

impl TokenBaseTransitionActionAccessorsV0 for TokenBaseTransitionActionV0 {
    fn token_position(&self) -> u16 {
        self.token_contract_position
    }

    fn token_id(&self) -> Identifier {
        self.token_id
    }

    fn data_contract_id(&self) -> Identifier {
        self.data_contract.contract.id()
    }

    fn data_contract_fetch_info_ref(&self) -> &Arc<DataContractFetchInfo> {
        &self.data_contract
    }

    fn data_contract_fetch_info(&self) -> Arc<DataContractFetchInfo> {
        self.data_contract.clone()
    }

    fn identity_contract_nonce(&self) -> IdentityNonce {
        self.identity_contract_nonce
    }

    fn token_configuration(&self) -> Result<&TokenConfiguration, Error> {
        self.data_contract
            .as_ref()
            .contract
            .tokens()
            .get(&self.token_contract_position)
            .ok_or(Error::Protocol(Box::new(
                ProtocolError::CorruptedCodeExecution(format!(
                    "data contract does not have a token at position {}",
                    self.token_contract_position
                )),
            )))
    }

    fn store_in_group(&self) -> Option<&GroupStateTransitionResolvedInfo> {
        self.store_in_group.as_ref().map(|(s, _)| s)
    }

    fn original_group_action(&self) -> Option<&GroupAction> {
        self.store_in_group
            .as_ref()
            .and_then(|(_, group_action)| group_action.as_ref())
    }

    fn perform_action(&self) -> bool {
        self.perform_action
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::contract::DataContractFetchInfo;
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

    /// Build a DataContract with one token at position 0 (and optionally more positions).
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

    fn fetch_info(contract: DataContract) -> Arc<DataContractFetchInfo> {
        Arc::new(DataContractFetchInfo {
            contract,
            storage_flags: None,
            cost: OperationCost::default(),
            fee: None,
        })
    }

    fn sample_group(signer_id: Identifier) -> Group {
        let mut members = BTreeMap::new();
        members.insert(signer_id, 3u32);
        Group::V0(GroupV0 {
            members,
            required_power: 4,
        })
    }

    fn sample_group_action() -> GroupAction {
        GroupAction::V0(GroupActionV0 {
            contract_id: Identifier::new([2u8; 32]),
            proposer_id: Identifier::new([3u8; 32]),
            token_contract_position: 0,
            event: GroupActionEvent::TokenEvent(TokenEvent::Mint(
                100,
                Identifier::new([4u8; 32]),
                Some("original note".to_string()),
            )),
        })
    }

    fn action_with(
        contract: DataContract,
        position: u16,
        store_in_group: Option<(GroupStateTransitionResolvedInfo, OriginalGroupAction)>,
        perform_action: bool,
    ) -> TokenBaseTransitionActionV0 {
        TokenBaseTransitionActionV0 {
            token_id: Identifier::new([11u8; 32]),
            identity_contract_nonce: 42,
            token_contract_position: position,
            data_contract: fetch_info(contract),
            store_in_group,
            perform_action,
        }
    }

    #[test]
    fn accessors_return_struct_fields() {
        let contract = build_contract_with_token_positions(&[0]);
        let expected_contract_id = contract.id();
        let action = action_with(contract, 0, None, true);

        assert_eq!(action.token_position(), 0);
        assert_eq!(action.token_id(), Identifier::new([11u8; 32]));
        // data_contract_id() delegates to the contract, NOT the token_id or any struct field
        assert_eq!(action.data_contract_id(), expected_contract_id);
        assert_eq!(action.identity_contract_nonce(), 42);
        assert!(action.perform_action());
        assert!(action.store_in_group().is_none());
        assert!(action.original_group_action().is_none());
    }

    #[test]
    fn data_contract_fetch_info_ref_and_clone_share_pointer() {
        let contract = build_contract_with_token_positions(&[0]);
        let action = action_with(contract, 0, None, true);

        let reference = action.data_contract_fetch_info_ref();
        let clone = action.data_contract_fetch_info();

        // Both Arc values point to the same allocation as the struct field
        assert!(Arc::ptr_eq(reference, &clone));
        assert!(Arc::ptr_eq(reference, &action.data_contract));
        // Ensure clone bumped the strong count (original + reference + clone)
        assert!(Arc::strong_count(&clone) >= 2);
    }

    #[test]
    fn token_configuration_returns_config_when_position_present() {
        let contract = build_contract_with_token_positions(&[0, 5]);
        let action = action_with(contract, 5, None, true);

        let config = action
            .token_configuration()
            .expect("token_configuration should succeed for a present position");
        // Sanity: returned reference must be one of our constructed entries.
        assert!(matches!(config, TokenConfiguration::V0(_)));
    }

    #[test]
    fn token_configuration_returns_corrupted_code_execution_for_missing_position() {
        let contract = build_contract_with_token_positions(&[0]);
        // Build an action pointing at a position the contract does NOT expose.
        let action = action_with(contract, 99, None, true);

        let err = action
            .token_configuration()
            .expect_err("missing token position must produce an error");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("CorruptedCodeExecution"),
            "expected CorruptedCodeExecution variant, got: {msg}"
        );
        assert!(
            msg.contains("99"),
            "error message should mention the missing position, got: {msg}"
        );
    }

    #[test]
    fn store_in_group_returns_resolved_info_without_original_action() {
        let owner_id = Identifier::new([8u8; 32]);
        let resolved = GroupStateTransitionResolvedInfo {
            group_contract_position: 1,
            group: sample_group(owner_id),
            action_id: Identifier::new([5u8; 32]),
            action_is_proposer: true,
            signer_power: 3,
        };
        let action = action_with(
            build_contract_with_token_positions(&[0]),
            0,
            Some((resolved.clone(), None)),
            true,
        );

        let got = action
            .store_in_group()
            .expect("store_in_group should return Some when set");
        assert_eq!(got.group_contract_position, 1);
        assert_eq!(got.action_id, Identifier::new([5u8; 32]));
        assert!(got.action_is_proposer);
        assert_eq!(got.signer_power, 3);
        // No original group action in this variant
        assert!(action.original_group_action().is_none());
    }

    #[test]
    fn original_group_action_returns_inner_group_action_when_present() {
        let owner_id = Identifier::new([8u8; 32]);
        let resolved = GroupStateTransitionResolvedInfo {
            group_contract_position: 0,
            group: sample_group(owner_id),
            action_id: Identifier::new([6u8; 32]),
            action_is_proposer: false,
            signer_power: 3,
        };
        let group_action = sample_group_action();
        let action = action_with(
            build_contract_with_token_positions(&[0]),
            0,
            Some((resolved, Some(group_action.clone()))),
            false,
        );

        let returned = action
            .original_group_action()
            .expect("original_group_action must propagate inner Some");
        // Same variant / same data
        assert_eq!(returned, &group_action);
        // perform_action was set to false in this fixture — preserve it
        assert!(!action.perform_action());
    }
}
