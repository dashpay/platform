use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::{GetProofsRequest, GetProofsResponse};
use dpp::balances::credits::TokenAmount;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_keeps_history_rules::accessors::v0::TokenKeepsHistoryRulesV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::TokenConfiguration;
use dpp::data_contracts::SystemDataContract;
use dpp::document::property_names::PRICE;
use dpp::document::Document;
use dpp::fee::Credits;
use dpp::identity::PartialIdentity;
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
use dpp::state_transition::proof_result::StateTransitionProofResult::{
    VerifiedDocuments, VerifiedPartialIdentity, VerifiedTokenActionWithDocument,
    VerifiedTokenBalance, VerifiedTokenIdentitiesBalances, VerifiedTokenIdentityInfo,
};
use dpp::state_transition::{StateTransition, StateTransitionLike};
use dpp::system_data_contracts::load_system_data_contract;
use dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;
use dpp::voting::vote_polls::VotePoll;
use dpp::voting::votes::Vote;
use drive::drive::identity::key::fetch::IdentityKeysRequest;
use drive::drive::identity::{IdentityDriveQuery, IdentityProveRequestType};
use drive::drive::Drive;
use drive::error::proof::ProofError;
use drive::query::{PathQuery, SingleDocumentDriveQuery, SingleDocumentDriveQueryContestedStatus};
use drive::verify::RootHash;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

fn contract_ids_to_non_historical_path_query(contract_ids: &[Identifier]) -> PathQuery {
    let contract_ids = contract_ids.iter().map(|id| id.0).collect();

    let mut path_query = Drive::fetch_non_historical_contracts_query(contract_ids);
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
        let state_transition: StateTransition =
            StateTransition::deserialize_from_bytes(&state_transition_bytes)?;

        let path_query = match state_transition {
            StateTransition::DataContractCreate(st) => {
                contract_ids_to_non_historical_path_query(&st.modified_data_ids())
            }
            StateTransition::DataContractUpdate(st) => {
                contract_ids_to_non_historical_path_query(&st.modified_data_ids())
            }
            StateTransition::Batch(st) => {
                if st.transitions_len() > 1 {
                    return Err(QueryError::Proof(ProofError::InvalidTransition(
                        "batch state transition must have only one batched transition".to_string(),
                    ))
                    .into());
                }
                let Some(transition) = st.first_transition() else {
                    return Err(QueryError::Proof(ProofError::InvalidTransition(
                        "batch state transition must have one batched transition".to_string(),
                    ))
                    .into());
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

                        let mut path_query = query.construct_path_query(platform_version).ok()?;
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
                            return Err(drive::error::Error::Proof(ProofError::UnknownContract(
                                format!("unknown contract with id {}", data_contract_id),
                            ))
                            .into());
                        };

                        let contract = &contract_fetch_info.contract;

                        let identity_contract_nonce =
                            token_transition.base().identity_contract_nonce();

                        let token_history_document_type_name =
                            token_transition.historical_document_type_name().to_string();

                        let token_history_contract =
                            self.drive.cache.system_data_contracts.load_token_history();

                        let token_history_document_type =
                            token_transition.historical_document_type(&token_history_contract)?;

                        let token_config = contract.expected_token_configuration(
                            token_transition.base().token_contract_position(),
                        )?;

                        let keeps_historical_document = token_config.keeps_history();

                        match token_transition {
                            TokenTransition::Burn(_) => {
                                if keeps_historical_document.keeps_burning_history() {
                                    create_token_historical_document_query(
                                        token_transition,
                                        token_config,
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
                                        token_config,
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
                                        token_config,
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
                            TokenTransition::Freeze(token_freeze_transition) => {
                                if keeps_historical_document.keeps_freezing_history() {
                                    create_token_historical_document_query(
                                        token_transition,
                                        token_config,
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
                            TokenTransition::Unfreeze(token_unfreeze_transition) => {
                                if keeps_historical_document.keeps_freezing_history() {
                                    create_token_historical_document_query(
                                        token_transition,
                                        token_config,
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
                            TokenTransition::DestroyFrozenFunds(_)
                            | TokenTransition::EmergencyAction(_)
                            | TokenTransition::ConfigUpdate(_)
                            | TokenTransition::Claim(_) => create_token_historical_document_query(
                                token_transition,
                                token_config,
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
                let pro_tx_hash = masternode_vote.pro_tx_hash();
                let vote = masternode_vote.vote();
                let contract = match vote {
                    Vote::ResourceVote(resource_vote) => match resource_vote.vote_poll() {
                        VotePoll::ContestedDocumentResourceVotePoll(
                            contested_document_resource_vote_poll,
                        ) => known_contracts_provider_fn(
                            &contested_document_resource_vote_poll.contract_id,
                        )?
                        .ok_or(drive::error::Error::Proof(
                            ProofError::UnknownContract(format!(
                                "unknown contract with id {}",
                                contested_document_resource_vote_poll.contract_id
                            )),
                        ))?,
                    },
                };

                Drive::

                // we expect to get a vote that matches the state transition
                πlet(root_hash, vote) = Drive::verify_masternode_vote(
                    proof,
                    pro_tx_hash.to_buffer(),
                    vote,
                    &contract,
                    false,
                    platform_version,
                )?;
            }
        };

        let identity_requests = check_validation_result_with_data!(identities
            .into_iter()
            .map(|identity_request| {
                Ok(IdentityDriveQuery {
                    identity_id: Bytes32::from_vec(identity_request.identity_id)
                        .map(|bytes| bytes.0)
                        .map_err(|_| {
                            QueryError::InvalidArgument(
                                "id must be a valid identifier (32 bytes long)".to_string(),
                            )
                        })?,
                    prove_request_type: IdentityProveRequestType::try_from(
                        identity_request.request_type as u8,
                    )
                    .map_err(|_| {
                        QueryError::InvalidArgument(
                            format!(
                                "invalid prove request type '{}'",
                                identity_request.request_type
                            )
                            .to_string(),
                        )
                    })?,
                })
            })
            .collect::<Result<Vec<IdentityDriveQuery>, QueryError>>());

        let vote_queries = check_validation_result_with_data!(votes
            .into_iter()
            .filter_map(|vote_proof_request| {
                if let Some(request_type) = vote_proof_request.request_type {
                    match request_type {
                        RequestType::ContestedResourceVoteStatusRequest(contested_resource_vote_status_request) => {
                            let identity_id = match contested_resource_vote_status_request.voter_identifier.try_into() {
                                Ok(identity_id) => identity_id,
                                Err(_) => return Some(Err(QueryError::InvalidArgument(
                                    "voter_identifier must be a valid identifier (32 bytes long)".to_string(),
                                ))),
                            };
                            let contract_id = match contested_resource_vote_status_request.contract_id.try_into() {
                                Ok(contract_id) => contract_id,
                                Err(_) => return Some(Err(QueryError::InvalidArgument(
                                    "contract_id must be a valid identifier (32 bytes long)".to_string(),
                                ))),
                            };
                            let document_type_name = contested_resource_vote_status_request.document_type_name;
                            let index_name = contested_resource_vote_status_request.index_name;
                            let index_values = match contested_resource_vote_status_request.index_values.into_iter().enumerate().map(|(pos, serialized_value)|
                                Ok(bincode::decode_from_slice(serialized_value.as_slice(), bincode::config::standard().with_big_endian()
                                    .with_no_limit()).map_err(|_| QueryError::InvalidArgument(
                                    format!("could not convert {:?} to a value in the index values at position {}", serialized_value, pos),
                                ))?.0)
                            ).collect::<Result<Vec<_>, QueryError>>() {
                                Ok(index_values) => index_values,
                                Err(e) => return Some(Err(e)),
                            };
                            let vote_poll = ContestedDocumentResourceVotePoll {
                                contract_id,
                                document_type_name,
                                index_name,
                                index_values,
                            }.into();
                            Some(Ok(IdentityBasedVoteDriveQuery {
                                identity_id,
                                vote_poll,
                            }))
                        }
                    }
                } else {
                    None
                }
            })
            .collect::<Result<Vec<IdentityBasedVoteDriveQuery>, QueryError>>());

        let token_balance_queries = check_validation_result_with_data!(identity_token_balances
            .into_iter()
            .map(|identity_token_balance_request| {
                let IdentityTokenBalanceRequest {
                    token_id,
                    identity_id,
                } = identity_token_balance_request;
                Ok(IdentityTokenBalanceDriveQuery {
                    identity_id: Identifier::try_from(identity_id).map_err(|_| {
                        QueryError::InvalidArgument(
                            "identity_id must be a valid identifier (32 bytes long)".to_string(),
                        )
                    })?,
                    token_id: Identifier::try_from(token_id).map_err(|_| {
                        QueryError::InvalidArgument(
                            "token_id must be a valid identifier (32 bytes long)".to_string(),
                        )
                    })?,
                })
            })
            .collect::<Result<Vec<IdentityTokenBalanceDriveQuery>, QueryError>>());

        let token_info_queries = check_validation_result_with_data!(identity_token_infos
            .into_iter()
            .map(|identity_token_info_request| {
                let IdentityTokenInfoRequest {
                    token_id,
                    identity_id,
                } = identity_token_info_request;
                Ok(IdentityTokenInfoDriveQuery {
                    identity_id: Identifier::try_from(identity_id).map_err(|_| {
                        QueryError::InvalidArgument(
                            "identity_id must be a valid identifier (32 bytes long)".to_string(),
                        )
                    })?,
                    token_id: Identifier::try_from(token_id).map_err(|_| {
                        QueryError::InvalidArgument(
                            "token_id must be a valid identifier (32 bytes long)".to_string(),
                        )
                    })?,
                })
            })
            .collect::<Result<Vec<IdentityTokenInfoDriveQuery>, QueryError>>());

        let token_status_queries = check_validation_result_with_data!(token_statuses
            .into_iter()
            .map(|token_status_request| {
                let TokenStatusRequest { token_id } = token_status_request;
                Ok(TokenStatusDriveQuery {
                    token_id: Identifier::try_from(token_id).map_err(|_| {
                        QueryError::InvalidArgument(
                            "token_id must be a valid identifier (32 bytes long)".to_string(),
                        )
                    })?,
                })
            })
            .collect::<Result<Vec<TokenStatusDriveQuery>, QueryError>>());

        let proof = self.drive.prove_multiple_state_transition_results(
            &identity_requests,
            &contract_ids,
            &document_queries,
            &vote_queries,
            &token_balance_queries,
            &token_info_queries,
            &token_status_queries,
            None,
            platform_version,
        )?;

        let response = GetProofsResponseV0 {
            result: Some(get_proofs_response_v0::Result::Proof(
                self.response_proof_v0(platform_state, proof),
            )),
            metadata: Some(self.response_metadata_v0(platform_state)),
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

fn create_token_historical_document_query(
    token_transition: &TokenTransition,
    token_config: &TokenConfiguration,
    owner_id: Identifier,
    platform_version: &PlatformVersion,
) -> Result<PathQuery, Error> {
    let token_history_contract =
        load_system_data_contract(SystemDataContract::TokenHistory, platform_version)?;

    let Some(token_event) = token_transition.associated_token_event(token_config, owner_id)? else {
        unreachable!("verify_token_historical_event must be called only for token transitions that have an associated token event");
    };

    let token_id = token_transition.token_id();
    let owner_nonce = token_transition.base().identity_contract_nonce();

    let query = SingleDocumentDriveQuery {
        contract_id: token_history_contract.id().into_buffer(),
        document_type_name: token_event.associated_document_type_name().to_string(),
        document_type_keeps_history: false,
        document_id: token_event
            .associated_document_id(token_id, owner_id, owner_nonce)
            .to_buffer(),
        block_time_ms: None, //None because we want latest
        contested_status: SingleDocumentDriveQueryContestedStatus::NotContested,
    };

    let token_history_document_type = token_history_contract
        .document_type_for_name(token_event.associated_document_type_name())?;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::{assert_invalid_identifier, setup_platform};
    use dapi_grpc::platform::v0::get_proofs_request::get_proofs_request_v0::vote_status_request::ContestedResourceVoteStatusRequest;
    use dapi_grpc::platform::v0::get_proofs_request::get_proofs_request_v0::{
        ContractRequest, DocumentRequest, IdentityRequest, VoteStatusRequest,
    };
    use dpp::dashcore::Network;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::platform_value::Value;
    use dpp::util::strings::convert_to_homograph_safe_chars;

    #[test]
    fn test_invalid_identity_ids() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetProofsRequestV0 {
            identities: vec![IdentityRequest {
                identity_id: vec![0; 8],
                request_type: 0,
            }],
            contracts: vec![],
            documents: vec![],
            votes: vec![],
            identity_token_balances: vec![],
            identity_token_infos: vec![],
            token_statuses: vec![],
        };

        let result = platform
            .query_proofs_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_invalid_identity_prove_request_type() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request_type = 10;

        let request = GetProofsRequestV0 {
            identities: vec![IdentityRequest {
                identity_id: vec![0; 32],
                request_type,
            }],
            contracts: vec![],
            documents: vec![],
            votes: vec![],
            identity_token_balances: vec![],
            identity_token_infos: vec![],
            token_statuses: vec![],
        };

        let result = platform
            .query_proofs_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg == &format!(
                "invalid prove request type '{}'",
                request_type
            )
        ))
    }

    #[test]
    fn test_invalid_contract_ids() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetProofsRequestV0 {
            identities: vec![],
            contracts: vec![ContractRequest {
                contract_id: vec![0; 8],
            }],
            documents: vec![],
            votes: vec![],
            identity_token_balances: vec![],
            identity_token_infos: vec![],
            token_statuses: vec![],
        };

        let result = platform
            .query_proofs_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_invalid_contract_id_for_documents_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetProofsRequestV0 {
            identities: vec![],
            contracts: vec![],
            documents: vec![DocumentRequest {
                contract_id: vec![0; 8],
                document_type: "niceDocument".to_string(),
                document_type_keeps_history: false,
                document_id: vec![0; 32],
                document_contested_status: 0,
            }],
            votes: vec![],
            identity_token_balances: vec![],
            identity_token_infos: vec![],
            token_statuses: vec![],
        };

        let result = platform
            .query_proofs_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_invalid_document_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetProofsRequestV0 {
            identities: vec![],
            contracts: vec![],
            documents: vec![DocumentRequest {
                contract_id: vec![0; 32],
                document_type: "niceDocument".to_string(),
                document_type_keeps_history: false,
                document_id: vec![0; 8],
                document_contested_status: 0,
            }],
            votes: vec![],
            identity_token_balances: vec![],
            identity_token_infos: vec![],
            token_statuses: vec![],
        };

        let result = platform
            .query_proofs_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_proof_of_absence() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetProofsRequestV0 {
            identities: vec![],
            contracts: vec![],
            documents: vec![DocumentRequest {
                contract_id: vec![0; 32],
                document_type: "niceDocument".to_string(),
                document_type_keeps_history: false,
                document_id: vec![0; 32],
                document_contested_status: 0,
            }],
            votes: vec![],
            identity_token_balances: vec![],
            identity_token_infos: vec![],
            token_statuses: vec![],
        };

        let validation_result = platform
            .query_proofs_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(validation_result.data, Some(GetProofsResponseV0 {
            result: Some(get_proofs_response_v0::Result::Proof(proof)),
            metadata: Some(_),
        }) if !proof.grovedb_proof.is_empty()));
    }

    #[test]
    fn test_proof_of_absence_of_vote() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let dpns_contract = platform
            .drive
            .cache
            .system_data_contracts
            .load_dpns()
            .as_ref()
            .clone();

        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        let serialized_index_values = [
            Value::Text("dash".to_string()),
            Value::Text(convert_to_homograph_safe_chars("quantum")),
        ]
        .iter()
        .map(|value| {
            bincode::encode_to_vec(value, config).expect("expected to encode value in path")
        })
        .collect();

        let request = GetProofsRequestV0 {
            identities: vec![],
            contracts: vec![],
            documents: vec![],
            votes: vec![VoteStatusRequest {
                request_type: Some(RequestType::ContestedResourceVoteStatusRequest(
                    ContestedResourceVoteStatusRequest {
                        contract_id: dpns_contract.id().to_vec(),
                        document_type_name: "domain".to_string(),
                        index_name: "parentNameAndLabel".to_string(),
                        index_values: serialized_index_values,
                        voter_identifier: [0u8; 32].to_vec(),
                    },
                )),
            }],
            identity_token_balances: vec![],
            identity_token_infos: vec![],
            token_statuses: vec![],
        };

        let validation_result = platform
            .query_proofs_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(validation_result.data, Some(GetProofsResponseV0 {
            result: Some(get_proofs_response_v0::Result::Proof(proof)),
            metadata: Some(_),
        }) if !proof.grovedb_proof.is_empty()));
    }

    #[test]
    fn test_prove_all() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetProofsRequestV0 {
            identities: vec![IdentityRequest {
                identity_id: vec![0; 32],
                request_type: 0,
            }],
            contracts: vec![ContractRequest {
                contract_id: vec![0; 32],
            }],
            documents: vec![DocumentRequest {
                contract_id: vec![0; 32],
                document_type: "niceDocument".to_string(),
                document_type_keeps_history: false,
                document_id: vec![1; 32],
                document_contested_status: 0,
            }],
            votes: vec![],
            identity_token_balances: vec![],
            identity_token_infos: vec![],
            token_statuses: vec![],
        };

        let validation_result = platform
            .query_proofs_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(validation_result.data, Some(GetProofsResponseV0 {
            result: Some(get_proofs_response_v0::Result::Proof(proof)),
            metadata: Some(_),
        }) if !proof.grovedb_proof.is_empty()));
    }
}
