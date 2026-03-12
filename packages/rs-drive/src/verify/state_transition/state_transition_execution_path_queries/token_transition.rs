use crate::drive::Drive;
use crate::error::Error;
use crate::query::{SingleDocumentDriveQuery, SingleDocumentDriveQueryContestedStatus};
use crate::verify::state_transition::state_transition_execution_path_queries::TryTransitionIntoPathQuery;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_keeps_history_rules::accessors::v0::TokenKeepsHistoryRulesV0Getters;
use dpp::data_contracts::SystemDataContract;
use dpp::group::GroupStateTransitionInfo;
use dpp::identifier::Identifier;
use dpp::prelude::DataContract;
use dpp::state_transition::batch_transition::batched_transition::token_transition::{
    TokenTransition, TokenTransitionV0Methods,
};
use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::token_freeze_transition::v0::v0_methods::TokenFreezeTransitionV0Methods;
use dpp::state_transition::batch_transition::token_mint_transition::v0::v0_methods::TokenMintTransitionV0Methods;
use dpp::state_transition::batch_transition::token_transfer_transition::v0::v0_methods::TokenTransferTransitionV0Methods;
use dpp::state_transition::batch_transition::token_unfreeze_transition::v0::v0_methods::TokenUnfreezeTransitionV0Methods;
use dpp::system_data_contracts::load_system_data_contract;
use grovedb::PathQuery;
use platform_version::version::PlatformVersion;

fn create_token_historical_document_query(
    token_transition: &TokenTransition,
    owner_id: Identifier,
    platform_version: &PlatformVersion,
) -> Result<PathQuery, Error> {
    let token_history_contract =
        load_system_data_contract(SystemDataContract::TokenHistory, platform_version)?;

    let query = SingleDocumentDriveQuery {
        contract_id: token_history_contract.id().into_buffer(),
        document_type_name: token_transition.historical_document_type_name().to_string(),
        document_type_keeps_history: false,
        document_id: token_transition
            .historical_document_id(owner_id)
            .to_buffer(),
        block_time_ms: None, //None because we want latest
        contested_status: SingleDocumentDriveQueryContestedStatus::NotContested,
    };

    query.construct_path_query(platform_version)
}

fn create_token_group_action_query(
    contract_id: [u8; 32],
    identity_id: [u8; 32],
    group_state_transition_info: GroupStateTransitionInfo,
) -> PathQuery {
    let GroupStateTransitionInfo {
        group_contract_position,
        action_id,
        ..
    } = group_state_transition_info;

    Drive::group_active_and_closed_action_single_signer_query(
        contract_id,
        group_contract_position,
        action_id.to_buffer(),
        identity_id,
    )
}

impl TryTransitionIntoPathQuery for TokenTransition {
    type Error = Error;

    fn try_transition_into_path_query_with_contract(
        &self,
        contract: &DataContract,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Self::Error> {
        let token_id = self.token_id();

        let token_config =
            contract.expected_token_configuration(self.base().token_contract_position())?;

        let keeps_historical_document = token_config.keeps_history();

        let action_success_query = match self {
            TokenTransition::Burn(_) => {
                if keeps_historical_document.keeps_burning_history() {
                    create_token_historical_document_query(self, owner_id, platform_version)?
                } else {
                    Drive::token_balance_for_identity_id_query(
                        token_id.to_buffer(),
                        owner_id.to_buffer(),
                    )
                }
            }
            TokenTransition::Mint(token_mint_transition) => {
                if keeps_historical_document.keeps_minting_history() {
                    create_token_historical_document_query(self, owner_id, platform_version)?
                } else {
                    let recipient_id = token_mint_transition.recipient_id(token_config)?;

                    Drive::token_balance_for_identity_id_query(
                        token_id.into_buffer(),
                        recipient_id.into_buffer(),
                    )
                }
            }
            TokenTransition::Transfer(token_transfer_transition) => {
                if keeps_historical_document.keeps_transfer_history() {
                    create_token_historical_document_query(self, owner_id, platform_version)?
                } else {
                    let recipient_id = token_transfer_transition.recipient_id();
                    let identity_ids = [owner_id.to_buffer(), recipient_id.to_buffer()];

                    Drive::token_balances_for_identity_ids_query(
                        token_id.into_buffer(),
                        &identity_ids,
                    )
                }
            }
            TokenTransition::Freeze(token_frozen_transition) => {
                if keeps_historical_document.keeps_freezing_history() {
                    create_token_historical_document_query(self, owner_id, platform_version)?
                } else {
                    Drive::token_info_for_identity_id_query(
                        token_id.to_buffer(),
                        token_frozen_transition.frozen_identity_id().to_buffer(),
                    )
                }
            }
            TokenTransition::Unfreeze(token_unfrozen_transition) => {
                if keeps_historical_document.keeps_freezing_history() {
                    create_token_historical_document_query(self, owner_id, platform_version)?
                } else {
                    Drive::token_info_for_identity_id_query(
                        token_id.to_buffer(),
                        token_unfrozen_transition.frozen_identity_id().to_buffer(),
                    )
                }
            }
            TokenTransition::DirectPurchase(_) => {
                if keeps_historical_document.keeps_direct_purchase_history() {
                    create_token_historical_document_query(self, owner_id, platform_version)?
                } else {
                    Drive::token_balance_for_identity_id_query(
                        token_id.to_buffer(),
                        owner_id.to_buffer(),
                    )
                }
            }
            TokenTransition::SetPriceForDirectPurchase(_) => {
                if keeps_historical_document.keeps_direct_pricing_history() {
                    create_token_historical_document_query(self, owner_id, platform_version)?
                } else {
                    Drive::token_direct_purchase_price_query(token_id.to_buffer())
                }
            }
            TokenTransition::DestroyFrozenFunds(_)
            | TokenTransition::EmergencyAction(_)
            | TokenTransition::ConfigUpdate(_)
            | TokenTransition::Claim(_) => {
                create_token_historical_document_query(self, owner_id, platform_version)?
            }
        };

        if let Some(group_state_transition_info) = self.base().using_group_info() {
            // if we are using group info, one of two things might have happened
            // either the action happened or it was initiated
            // we need to merge the action success query and the query for group signers
            let group_signer_query = create_token_group_action_query(
                self.data_contract_id().to_buffer(),
                owner_id.to_buffer(),
                group_state_transition_info,
            );
            let mut success_query = action_success_query;
            // We need to remove the limit here
            success_query.query.limit = None;
            PathQuery::merge(
                vec![&group_signer_query, &success_query],
                &platform_version.drive.grove_version,
            )
            .map_err(Error::from)
        } else {
            // if there is no group info that all we need is this query
            Ok(action_success_query)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
    use dpp::data_contract::associated_token::token_keeps_history_rules::v0::TokenKeepsHistoryRulesV0;
    use dpp::data_contract::associated_token::token_keeps_history_rules::TokenKeepsHistoryRules;
    use dpp::data_contract::config::v0::DataContractConfigV0;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::v1::DataContractV1;
    use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
    use dpp::state_transition::batch_transition::batched_transition::token_burn_transition::v0::TokenBurnTransitionV0;
    use dpp::state_transition::batch_transition::batched_transition::token_burn_transition::TokenBurnTransition;
    use dpp::state_transition::batch_transition::batched_transition::token_freeze_transition::v0::TokenFreezeTransitionV0;
    use dpp::state_transition::batch_transition::batched_transition::token_freeze_transition::TokenFreezeTransition;
    use dpp::state_transition::batch_transition::batched_transition::token_unfreeze_transition::v0::TokenUnfreezeTransitionV0;
    use dpp::state_transition::batch_transition::batched_transition::token_unfreeze_transition::TokenUnfreezeTransition;
    use dpp::state_transition::batch_transition::batched_transition::token_set_price_for_direct_purchase_transition::v0::TokenSetPriceForDirectPurchaseTransitionV0;
    use dpp::state_transition::batch_transition::batched_transition::token_set_price_for_direct_purchase_transition::TokenSetPriceForDirectPurchaseTransition;
    use std::collections::BTreeMap;

    /// Helper to create a data contract with a single token that does NOT keep any
    /// historical documents (all keeps_*_history flags are false).
    fn create_test_contract_and_ids() -> (DataContract, Identifier, Identifier) {
        // Start from default_most_restrictive and override keeps_history to all-false.
        let mut token_config = TokenConfigurationV0::default_most_restrictive();
        *token_config.keeps_history_mut() = TokenKeepsHistoryRules::V0(TokenKeepsHistoryRulesV0 {
            keeps_transfer_history: false,
            keeps_freezing_history: false,
            keeps_minting_history: false,
            keeps_burning_history: false,
            keeps_direct_pricing_history: false,
            keeps_direct_purchase_history: false,
        });

        let contract = DataContract::V1(DataContractV1 {
            id: Identifier::from([1u8; 32]),
            version: 0,
            owner_id: Default::default(),
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
            tokens: BTreeMap::from([(0, TokenConfiguration::V0(token_config))]),
            keywords: Vec::new(),
            description: None,
        });

        use dpp::data_contract::accessors::v1::DataContractV1Getters;
        let token_id = contract.token_id(0).expect("expected token at position 0");
        let owner_id = Identifier::from([5u8; 32]);
        (contract, token_id, owner_id)
    }

    fn make_base(contract_id: Identifier, token_id: Identifier) -> TokenBaseTransition {
        TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: 1,
            token_contract_position: 0,
            data_contract_id: contract_id,
            token_id,
            using_group_info: None,
        })
    }

    #[test]
    fn burn_transition_produces_balance_query_without_history() {
        let platform_version = PlatformVersion::latest();
        let (contract, token_id, owner_id) = create_test_contract_and_ids();

        let transition = TokenTransition::Burn(TokenBurnTransition::V0(TokenBurnTransitionV0 {
            base: make_base(contract.id(), token_id),
            burn_amount: 100,
            public_note: None,
        }));

        let path_query = transition
            .try_transition_into_path_query_with_contract(&contract, owner_id, platform_version)
            .expect("expected path query");

        // The burn transition (no history) should produce a balance query for the owner
        let expected_query =
            Drive::token_balance_for_identity_id_query(token_id.to_buffer(), owner_id.to_buffer());

        assert_eq!(path_query.path, expected_query.path);
        assert_eq!(path_query.query.limit, expected_query.query.limit);
    }

    #[test]
    fn freeze_transition_produces_info_query_without_history() {
        let platform_version = PlatformVersion::latest();
        let (contract, token_id, owner_id) = create_test_contract_and_ids();
        let frozen_identity_id = Identifier::from([10u8; 32]);

        let transition =
            TokenTransition::Freeze(TokenFreezeTransition::V0(TokenFreezeTransitionV0 {
                base: make_base(contract.id(), token_id),
                identity_to_freeze_id: frozen_identity_id,
                public_note: None,
            }));

        let path_query = transition
            .try_transition_into_path_query_with_contract(&contract, owner_id, platform_version)
            .expect("expected path query");

        // The freeze transition (no history) should produce an info query for the frozen identity
        let expected_query = Drive::token_info_for_identity_id_query(
            token_id.to_buffer(),
            frozen_identity_id.to_buffer(),
        );

        assert_eq!(path_query.path, expected_query.path);
        assert_eq!(path_query.query.limit, expected_query.query.limit);
    }

    #[test]
    fn unfreeze_transition_produces_info_query_without_history() {
        let platform_version = PlatformVersion::latest();
        let (contract, token_id, owner_id) = create_test_contract_and_ids();
        let frozen_identity_id = Identifier::from([11u8; 32]);

        let transition =
            TokenTransition::Unfreeze(TokenUnfreezeTransition::V0(TokenUnfreezeTransitionV0 {
                base: make_base(contract.id(), token_id),
                frozen_identity_id,
                public_note: None,
            }));

        let path_query = transition
            .try_transition_into_path_query_with_contract(&contract, owner_id, platform_version)
            .expect("expected path query");

        let expected_query = Drive::token_info_for_identity_id_query(
            token_id.to_buffer(),
            frozen_identity_id.to_buffer(),
        );

        assert_eq!(path_query.path, expected_query.path);
        assert_eq!(path_query.query.limit, expected_query.query.limit);
    }

    #[test]
    fn set_price_for_direct_purchase_produces_price_query_without_history() {
        let platform_version = PlatformVersion::latest();
        let (contract, token_id, owner_id) = create_test_contract_and_ids();

        let transition = TokenTransition::SetPriceForDirectPurchase(
            TokenSetPriceForDirectPurchaseTransition::V0(
                TokenSetPriceForDirectPurchaseTransitionV0 {
                    base: make_base(contract.id(), token_id),
                    price: None,
                    public_note: None,
                },
            ),
        );

        let path_query = transition
            .try_transition_into_path_query_with_contract(&contract, owner_id, platform_version)
            .expect("expected path query");

        let expected_query = Drive::token_direct_purchase_price_query(token_id.to_buffer());

        assert_eq!(path_query.path, expected_query.path);
        assert_eq!(path_query.query.limit, expected_query.query.limit);
    }
}
