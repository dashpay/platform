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
            .map_err(Error::GroveDB)
        } else {
            // if there is no group info that all we need is this query
            Ok(action_success_query)
        }
    }
}
