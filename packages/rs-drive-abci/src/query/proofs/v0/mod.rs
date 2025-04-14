use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::drive::v0::{GetProofsRequest, GetProofsResponse};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_keeps_history_rules::accessors::v0::TokenKeepsHistoryRulesV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contracts::SystemDataContract;
use dpp::prelude::Identifier;
use dpp::serialization::PlatformDeserializable;
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::batched_transition::document_transition::{
    DocumentTransition, DocumentTransitionV0Methods,
};
use dpp::state_transition::batch_transition::batched_transition::token_transition::{
    TokenTransition, TokenTransitionV0Methods,
};
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::token_mint_transition::v0::v0_methods::TokenMintTransitionV0Methods;
use dpp::state_transition::batch_transition::token_transfer_transition::v0::v0_methods::TokenTransferTransitionV0Methods;
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
use dpp::state_transition::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::identity_topup_transition::accessors::IdentityTopUpTransitionAccessorsV0;
use dpp::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use dpp::state_transition::masternode_vote_transition::accessors::MasternodeVoteTransitionAccessorsV0;
use dpp::state_transition::{StateTransition, StateTransitionLike};
use dpp::system_data_contracts::load_system_data_contract;
use dpp::version::PlatformVersion;
use dpp::voting::votes::resource_vote::accessors::v0::ResourceVoteGettersV0;
use dpp::voting::votes::Vote;
use drive::drive::Drive;
use drive::error::proof::ProofError;
use drive::query::{
    IdentityBasedVoteDriveQuery, PathQuery, SingleDocumentDriveQuery,
    SingleDocumentDriveQueryContestedStatus,
};

fn contract_ids_to_non_historical_path_query(contract_ids: &[Identifier]) -> PathQuery {
    let contract_ids: Vec<_> = contract_ids.iter().map(|id| id.to_buffer()).collect();

    let mut path_query = Drive::fetch_non_historical_contracts_query(&contract_ids);
    path_query.query.limit = None;
    path_query
}

impl<C> Platform<C> {
    pub(super) fn query_proofs_v0(
        &self,
        GetProofsRequest {
            state_transition: state_transition_bytes,
        }: GetProofsRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetProofsResponse>, Error> {
        let state_transition =
            match StateTransition::deserialize_from_bytes(&state_transition_bytes) {
                Ok(state_transition) => state_transition,
                Err(e) => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Protocol(
                        e,
                    )))
                }
            };

        let path_query = match state_transition {
            StateTransition::DataContractCreate(st) => {
                contract_ids_to_non_historical_path_query(&st.modified_data_ids())
            }
            StateTransition::DataContractUpdate(st) => {
                contract_ids_to_non_historical_path_query(&st.modified_data_ids())
            }
            StateTransition::Batch(st) => {
                if st.transitions_len() > 1 {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Proof(
                        ProofError::InvalidTransition(
                            "batch state transition must have only one batched transition"
                                .to_string(),
                        ),
                    )));
                }
                let Some(transition) = st.first_transition() else {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Proof(
                        ProofError::InvalidTransition(
                            "batch state transition must have one batched transition".to_string(),
                        ),
                    )));
                };

                let owner_id = st.owner_id();

                match transition {
                    BatchedTransitionRef::Document(document_transition) => {
                        let data_contract_id = document_transition.data_contract_id();

                        let Some(contract_fetch_info) = self.drive.get_contract_with_fetch_info(
                            data_contract_id.to_buffer(),
                            false,
                            None,
                            platform_version,
                        )?
                        else {
                            return Err(drive::error::Error::Proof(ProofError::UnknownContract(
                                format!("unknown contract with id {}", data_contract_id),
                            ))
                            .into());
                        };

                        let contract = &contract_fetch_info.contract;

                        let document_type = contract
                            .document_type_for_name(document_transition.document_type_name())
                            .map_err(|e| {
                                drive::error::Error::Proof(ProofError::UnknownContract(format!(
                                    "cannot fetch contract for document {} with id {}: {}",
                                    document_transition.document_type_name(),
                                    document_transition.data_contract_id(),
                                    e
                                )))
                            })?;

                        let contested_status =
                            if let DocumentTransition::Create(create_transition) =
                                document_transition
                            {
                                if create_transition.prefunded_voting_balance().is_some() {
                                    SingleDocumentDriveQueryContestedStatus::Contested
                                } else {
                                    SingleDocumentDriveQueryContestedStatus::NotContested
                                }
                            } else {
                                SingleDocumentDriveQueryContestedStatus::NotContested
                            };

                        let query = SingleDocumentDriveQuery {
                            contract_id: document_transition.data_contract_id().into_buffer(),
                            document_type_name: document_transition.document_type_name().clone(),
                            document_type_keeps_history: document_type.documents_keep_history(),
                            document_id: document_transition.base().id().into_buffer(),
                            block_time_ms: None, //None because we want latest
                            contested_status,
                        };

                        let mut path_query = query.construct_path_query(platform_version)?;
                        path_query.query.limit = None;
                        path_query
                    }
                    BatchedTransitionRef::Token(token_transition) => {
                        //todo group actions
                        let data_contract_id = token_transition.data_contract_id();
                        let token_id = token_transition.token_id();

                        let Some(contract_fetch_info) = self.drive.get_contract_with_fetch_info(
                            data_contract_id.to_buffer(),
                            false,
                            None,
                            platform_version,
                        )?
                        else {
                            return Ok(QueryValidationResult::new_with_error(QueryError::Proof(
                                ProofError::UnknownContract(format!(
                                    "unknown contract with id {}",
                                    data_contract_id
                                )),
                            )));
                        };

                        let contract = &contract_fetch_info.contract;

                        let token_config = contract.expected_token_configuration(
                            token_transition.base().token_contract_position(),
                        )?;

                        let keeps_historical_document = token_config.keeps_history();

                        match token_transition {
                            TokenTransition::Burn(_) => {
                                if keeps_historical_document.keeps_burning_history() {
                                    create_token_historical_document_query(
                                        token_transition,
                                        owner_id,
                                        platform_version,
                                    )?
                                } else {
                                    Drive::token_balance_for_identity_id_query(
                                        token_id.to_buffer(),
                                        owner_id.to_buffer(),
                                    )
                                }
                            }
                            TokenTransition::Mint(token_mint_transition) => {
                                if keeps_historical_document.keeps_minting_history() {
                                    create_token_historical_document_query(
                                        token_transition,
                                        owner_id,
                                        platform_version,
                                    )?
                                } else {
                                    let recipient_id =
                                        token_mint_transition.recipient_id(token_config)?;

                                    Drive::token_balance_for_identity_id_query(
                                        token_id.into_buffer(),
                                        recipient_id.into_buffer(),
                                    )
                                }
                            }
                            TokenTransition::Transfer(token_transfer_transition) => {
                                if keeps_historical_document.keeps_transfer_history() {
                                    create_token_historical_document_query(
                                        token_transition,
                                        owner_id,
                                        platform_version,
                                    )?
                                } else {
                                    let recipient_id = token_transfer_transition.recipient_id();
                                    let identity_ids =
                                        [owner_id.to_buffer(), recipient_id.to_buffer()];

                                    Drive::token_balances_for_identity_ids_query(
                                        token_id.into_buffer(),
                                        &identity_ids,
                                    )
                                }
                            }
                            TokenTransition::Freeze(_) => {
                                if keeps_historical_document.keeps_freezing_history() {
                                    create_token_historical_document_query(
                                        token_transition,
                                        owner_id,
                                        platform_version,
                                    )?
                                } else {
                                    Drive::token_info_for_identity_id_query(
                                        token_id.to_buffer(),
                                        owner_id.to_buffer(),
                                    )
                                }
                            }
                            TokenTransition::Unfreeze(_) => {
                                if keeps_historical_document.keeps_freezing_history() {
                                    create_token_historical_document_query(
                                        token_transition,
                                        owner_id,
                                        platform_version,
                                    )?
                                } else {
                                    Drive::token_info_for_identity_id_query(
                                        token_id.to_buffer(),
                                        owner_id.to_buffer(),
                                    )
                                }
                            }
                            TokenTransition::DirectPurchase(_) => {
                                if keeps_historical_document.keeps_direct_purchase_history() {
                                    create_token_historical_document_query(
                                        token_transition,
                                        owner_id,
                                        platform_version,
                                    )?
                                } else {
                                    Drive::token_balance_for_identity_id_query(
                                        token_id.to_buffer(),
                                        owner_id.to_buffer(),
                                    )
                                }
                            }
                            TokenTransition::SetPriceForDirectPurchase(_) => {
                                if keeps_historical_document.keeps_direct_pricing_history() {
                                    create_token_historical_document_query(
                                        token_transition,
                                        owner_id,
                                        platform_version,
                                    )?
                                } else {
                                    Drive::token_direct_purchase_price_query(token_id.to_buffer())
                                }
                            }
                            TokenTransition::DestroyFrozenFunds(_)
                            | TokenTransition::EmergencyAction(_)
                            | TokenTransition::ConfigUpdate(_)
                            | TokenTransition::Claim(_) => create_token_historical_document_query(
                                token_transition,
                                owner_id,
                                platform_version,
                            )?,
                        }
                    }
                }
            }
            StateTransition::IdentityCreate(st) => Drive::full_identity_query(
                &st.identity_id().into_buffer(),
                &platform_version.drive.grove_version,
            )?,
            StateTransition::IdentityTopUp(st) => {
                // we expect to get a new balance and revision
                Drive::revision_and_balance_path_query(
                    st.identity_id().to_buffer(),
                    &platform_version.drive.grove_version,
                )?
            }
            StateTransition::IdentityCreditWithdrawal(st) => {
                Drive::identity_balance_query(&st.identity_id().to_buffer())
            }
            StateTransition::IdentityUpdate(st) => Drive::identity_all_keys_query(
                &st.identity_id().to_buffer(),
                &platform_version.drive.grove_version,
            )?,
            StateTransition::IdentityCreditTransfer(st) => {
                let sender_query = Drive::identity_balance_query(&st.identity_id().into_buffer());
                let recipient_query =
                    Drive::identity_balance_query(&st.recipient_id().into_buffer());

                PathQuery::merge(
                    vec![&sender_query, &recipient_query],
                    &platform_version.drive.grove_version,
                )?
            }
            StateTransition::MasternodeVote(st) => {
                let pro_tx_hash = st.pro_tx_hash();

                match st.vote_owned() {
                    Vote::ResourceVote(resource_vote) => {
                        let query = IdentityBasedVoteDriveQuery {
                            identity_id: pro_tx_hash,
                            vote_poll: resource_vote.vote_poll_owned(),
                        };

                        // The path query construction can only fail if the serialization fails.
                        // Because the serialization will pretty much never fail, we can do this.
                        let mut path_query = query.construct_path_query()?;
                        path_query.query.limit = None;
                        path_query
                    }
                }
            }
        };

        let proof = self.drive.grove_get_proved_path_query(
            &path_query,
            None,
            &mut vec![],
            &platform_version.drive,
        )?;

        let response = GetProofsResponse {
            proof: Some(self.response_proof_v0(platform_state, proof)),
            metadata: Some(self.response_metadata_v0(platform_state)),
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

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

    query
        .construct_path_query(platform_version)
        .map_err(Error::Drive)
}
