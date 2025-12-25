use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::prove::prove_state_transition::ProofCreationResult;
use crate::query::{
    IdentityBasedVoteDriveQuery, SingleDocumentDriveQuery, SingleDocumentDriveQueryContestedStatus,
};
use crate::verify::state_transition::state_transition_execution_path_queries::TryTransitionIntoPathQuery;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::identifier::Identifier;
use dpp::state_transition::address_credit_withdrawal_transition::accessors::AddressCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::address_funding_from_asset_lock_transition::accessors::AddressFundingFromAssetLockTransitionAccessorsV0;
use dpp::state_transition::address_funds_transfer_transition::accessors::AddressFundsTransferTransitionAccessorsV0;
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::batched_transition::document_transition::{
    DocumentTransition, DocumentTransitionV0Methods,
};
use dpp::state_transition::batch_transition::batched_transition::token_transition::TokenTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use dpp::state_transition::identity_create_from_addresses_transition::accessors::IdentityCreateFromAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_to_addresses_transition::accessors::IdentityCreditTransferToAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
use dpp::state_transition::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::identity_topup_from_addresses_transition::accessors::IdentityTopUpFromAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_topup_transition::accessors::IdentityTopUpTransitionAccessorsV0;
use dpp::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use dpp::state_transition::masternode_vote_transition::accessors::MasternodeVoteTransitionAccessorsV0;
use dpp::state_transition::StateTransitionIdentityIdFromInputs;
use dpp::state_transition::StateTransitionWitnessSigned;
use dpp::state_transition::{StateTransition, StateTransitionLike, StateTransitionOwned};
use dpp::voting::votes::resource_vote::accessors::v0::ResourceVoteGettersV0;
use dpp::voting::votes::Vote;
use grovedb::{PathQuery, TransactionArg};
use platform_version::version::PlatformVersion;

fn contract_ids_to_non_historical_path_query(contract_ids: &[Identifier]) -> PathQuery {
    let contract_ids: Vec<_> = contract_ids.iter().map(|id| id.to_buffer()).collect();

    let mut path_query = Drive::fetch_non_historical_contracts_query(&contract_ids);
    path_query.query.limit = None;
    path_query
}

impl Drive {
    pub(super) fn prove_state_transition_v0(
        &self,
        state_transition: &StateTransition,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ProofCreationResult<Vec<u8>>, Error> {
        let path_query = match state_transition {
            StateTransition::DataContractCreate(st) => {
                contract_ids_to_non_historical_path_query(&st.modified_data_ids())
            }
            StateTransition::DataContractUpdate(st) => {
                contract_ids_to_non_historical_path_query(&st.modified_data_ids())
            }
            StateTransition::Batch(st) => {
                if st.transitions_len() > 1 {
                    return Ok(ProofCreationResult::new_with_error(
                        ProofError::InvalidTransition(
                            "batch state transition must have only one batched transition"
                                .to_string(),
                        ),
                    ));
                }
                let Some(transition) = st.first_transition() else {
                    return Ok(ProofCreationResult::new_with_error(
                        ProofError::InvalidTransition(
                            "batch state transition must have one batched transition".to_string(),
                        ),
                    ));
                };

                let owner_id = st.owner_id();

                match transition {
                    BatchedTransitionRef::Document(document_transition) => {
                        let data_contract_id = document_transition.data_contract_id();

                        let Some(contract_fetch_info) = self.get_contract_with_fetch_info(
                            data_contract_id.to_buffer(),
                            false,
                            None,
                            platform_version,
                        )?
                        else {
                            return Err(Error::Proof(ProofError::UnknownContract(format!(
                                "unknown contract with id {} in document proving",
                                data_contract_id
                            ))));
                        };

                        let contract = &contract_fetch_info.contract;

                        let document_type = contract
                            .document_type_for_name(document_transition.document_type_name())
                            .map_err(|e| {
                                Error::Proof(ProofError::UnknownContract(format!(
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
                        let data_contract_id = token_transition.data_contract_id();

                        let Some(contract_fetch_info) = self.get_contract_with_fetch_info(
                            data_contract_id.to_buffer(),
                            false,
                            None,
                            platform_version,
                        )?
                        else {
                            return Ok(ProofCreationResult::new_with_error(
                                ProofError::UnknownContract(format!(
                                    "unknown contract with id {} in token proving",
                                    data_contract_id
                                )),
                            ));
                        };

                        let contract = &contract_fetch_info.contract;
                        token_transition.try_transition_into_path_query_with_contract(
                            contract,
                            owner_id,
                            platform_version,
                        )?
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

                match st.vote() {
                    Vote::ResourceVote(resource_vote) => {
                        let query = IdentityBasedVoteDriveQuery {
                            identity_id: pro_tx_hash,
                            vote_poll: resource_vote.vote_poll().clone(),
                        };

                        // The path query construction can only fail if the serialization fails.
                        // Because the serialization will pretty much never fail, we can do this.
                        let mut path_query = query.construct_path_query()?;
                        path_query.query.limit = None;
                        path_query
                    }
                }
            }
            StateTransition::IdentityCreditTransferToAddresses(st) => {
                let identity_query = Drive::revision_and_balance_path_query(
                    st.identity_id().to_buffer(),
                    &platform_version.drive.grove_version,
                )?;
                let mut addresses_query =
                    Drive::balances_for_clear_addresses_query(st.recipient_addresses().keys());

                // TODO: fix this limit setting - "can not merge pathqueries with limits, consider setting the limit after the merge"
                addresses_query.query.limit = None;

                PathQuery::merge(
                    vec![&identity_query, &addresses_query],
                    &platform_version.drive.grove_version,
                )?
            }
            StateTransition::IdentityCreateFromAddresses(st) => {
                let identity_id = st.identity_id_from_inputs().map_err(|e| {
                    Error::Proof(ProofError::CorruptedProof(format!(
                        "Failed to calculate identity_id from inputs: {}",
                        e
                    )))
                })?;
                let identity_query = Drive::full_identity_query(
                    &identity_id.into_buffer(),
                    &platform_version.drive.grove_version,
                )?;
                let change_output = st.output().into_iter().map(|(address, _)| address);
                let addresses_to_check = st.inputs().keys().chain(change_output);

                let mut addresses_query =
                    Drive::balances_for_clear_addresses_query(addresses_to_check);

                // TODO: fix this limit setting - "can not merge pathqueries with limits, consider setting the limit after the merge"
                addresses_query.query.limit = None;

                PathQuery::merge(
                    vec![&identity_query, &addresses_query],
                    &platform_version.drive.grove_version,
                )?
            }
            StateTransition::IdentityTopUpFromAddresses(st) => {
                // we expect to get a new balance and revision
                let identity_query = Drive::revision_and_balance_path_query(
                    st.identity_id().to_buffer(),
                    &platform_version.drive.grove_version,
                )?;
                let change_output = st.output().into_iter().map(|(address, _)| address);
                let addresses_to_check = st.inputs().keys().chain(change_output);
                let addresses_query = Drive::balances_for_clear_addresses_query(addresses_to_check);

                // TODO: not sure if just setting this to unlimited is correct
                let mut addresses_query = addresses_query;
                addresses_query.query.limit = None;

                PathQuery::merge(
                    vec![&identity_query, &addresses_query],
                    &platform_version.drive.grove_version,
                )?
            }
            StateTransition::AddressFundsTransfer(st) => Drive::balances_for_clear_addresses_query(
                st.inputs().keys().chain(st.outputs().keys()),
            ),
            StateTransition::AddressFundingFromAssetLock(st) => {
                Drive::balances_for_clear_addresses_query(
                    st.inputs().keys().chain(st.outputs().keys()),
                )
            }
            StateTransition::AddressCreditWithdrawal(st) => {
                let addresses_to_check = st
                    .inputs()
                    .keys()
                    .chain(st.output().into_iter().map(|(address, _)| address));
                Drive::balances_for_clear_addresses_query(addresses_to_check)
            }
        };

        let proof = self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )?;

        Ok(ProofCreationResult::new_with_data(proof))
    }
}
