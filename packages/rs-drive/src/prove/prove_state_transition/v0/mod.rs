use crate::drive::contract::moderation::types::{
    ContractDocumentRemovalsQuery, ContractDocumentRemovalsSelection,
};
use crate::drive::identity::key::fetch::IdentityKeysRequest;
use crate::drive::{Drive, RootTree};
use crate::error::drive::DriveError;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::prove::prove_state_transition::ProofCreationResult;
use crate::query::{
    IdentityBasedVoteDriveQuery, SingleDocumentDriveQuery, SingleDocumentDriveQueryContestedStatus,
};
use crate::verify::state_transition::state_transition_execution_path_queries::TryTransitionIntoPathQuery;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::moderation::ContractModerationList;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
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
use dpp::state_transition::contract_fee_claim_transition::accessors::ContractFeeClaimTransitionAccessorsV0;
use dpp::state_transition::contract_user_moderation_transition::accessors::ContractUserModerationTransitionAccessorsV0;
use dpp::state_transition::contract_user_moderation_transition::ContractUserModerationAction;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::identity_create_from_addresses_transition::accessors::IdentityCreateFromAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_to_addresses_transition::accessors::IdentityCreditTransferToAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
use dpp::state_transition::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::identity_key_limits_update_transition::accessors::IdentityKeyLimitsUpdateTransitionAccessorsV0;
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

fn contract_ids_to_historical_path_query(contract_ids: &[Identifier]) -> PathQuery {
    let contract_ids: Vec<_> = contract_ids.iter().map(|id| id.to_buffer()).collect();

    let mut path_query = Drive::fetch_historical_contracts_query(&contract_ids);
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
        self.prove_state_transition_internal(state_transition, transaction, false, platform_version)
    }

    /// The proof of a state transition's execution, shared by every version of
    /// `prove_state_transition`. With `carries_owner_balance` (from version 1) the
    /// proof of an owned, fee-paying transition also carries the owner's credit
    /// balance.
    pub(in crate::prove::prove_state_transition) fn prove_state_transition_internal(
        &self,
        state_transition: &StateTransition,
        transaction: TransactionArg,
        carries_owner_balance: bool,
        platform_version: &PlatformVersion,
    ) -> Result<ProofCreationResult<Vec<u8>>, Error> {
        let path_query = match state_transition {
            StateTransition::DataContractCreate(st) => {
                if st.data_contract().config().keeps_history() {
                    contract_ids_to_historical_path_query(&st.modified_data_ids())
                } else {
                    contract_ids_to_non_historical_path_query(&st.modified_data_ids())
                }
            }
            StateTransition::DataContractUpdate(st) => {
                if st.data_contract().config().keeps_history() {
                    contract_ids_to_historical_path_query(&st.modified_data_ids())
                } else {
                    contract_ids_to_non_historical_path_query(&st.modified_data_ids())
                }
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

                        // indexOnly documents have no primary row to prove by
                        // id: the executed transition is proven against the
                        // entry its values produce under the proof index —
                        // the same single-entry path query the verifier
                        // rebuilds from the transition.
                        let document_path_query = {
                            use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
                            use dpp::state_transition::batch_transition::batched_transition::document_index_only_delete_transition::v0::v0_methods::DocumentIndexOnlyDeleteTransitionV0Methods;
                            if document_type.index_only() {
                                let values = match document_transition {
                                    DocumentTransition::Create(create_transition) => {
                                        create_transition.data().clone()
                                    }
                                    // The indexOnlyDelete kind always carries
                                    // its values — no fallible access needed.
                                    DocumentTransition::IndexOnlyDelete(delete_transition) => {
                                        delete_transition.data().clone()
                                    }
                                    _ => {
                                        return Ok(ProofCreationResult::new_with_error(
                                            ProofError::InvalidTransition(
                                                "indexOnly documents only support create and \
                                                 indexOnlyDelete"
                                                    .to_string(),
                                            ),
                                        ));
                                    }
                                };
                                crate::query::index_only_synthesis::index_only_transition_entry_path_query(
                                    contract.id(),
                                    document_type,
                                    &values,
                                    owner_id,
                                    platform_version,
                                )?
                            } else {
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
                                    contract_id: document_transition
                                        .data_contract_id()
                                        .into_buffer(),
                                    document_type_name: document_transition
                                        .document_type_name()
                                        .clone(),
                                    document_type_keeps_history: document_type
                                        .documents_keep_history(),
                                    document_id: document_transition.base().id().into_buffer(),
                                    block_time_ms: None, //None because we want latest
                                    contested_status,
                                };

                                let mut path_query =
                                    query.construct_path_query(platform_version)?;
                                path_query.query.limit = None;
                                path_query
                            }
                        };

                        document_path_query
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
            // The lists the moderation touched: a ban also removes a suspension, so it proves
            // every list the contract keeps (the banlist entry present, the suspension absent);
            // an unban, a suspend and an unsuspend prove the one entry they edit.
            // A document deletion proves the record it left: a document id is produced at most
            // once, so the record is of that document and the document is gone. A document
            // restore proves the same record, now marked restored; the document's id is inside
            // the bytes the transition carries, read under the contract's document type.
            StateTransition::ContractUserModeration(st) => {
                let contract_id = st.data_contract_id();
                if let Some((document_type_name, document_id)) = st.action().document() {
                    // The query the verifier rebuilds from the transition.
                    Drive::contract_document_removals_query(
                        contract_id.to_buffer(),
                        &ContractDocumentRemovalsQuery {
                            document_type_name: document_type_name.to_string(),
                            selection: ContractDocumentRemovalsSelection::DocumentIds(vec![
                                document_id,
                            ]),
                        },
                    )
                } else if let Some((document_type_name, document_bytes)) =
                    st.action().restored_document()
                {
                    let Some(contract_fetch_info) = self.get_contract_with_fetch_info(
                        contract_id.to_buffer(),
                        false,
                        None,
                        platform_version,
                    )?
                    else {
                        return Err(Error::Proof(ProofError::UnknownContract(format!(
                            "unknown contract with id {} in contract moderation proving",
                            contract_id
                        ))));
                    };
                    let document_type = contract_fetch_info
                        .contract
                        .document_type_for_name(document_type_name)?;
                    let document =
                        Document::from_bytes(document_bytes, document_type, platform_version)?;
                    Drive::contract_document_removals_query(
                        contract_id.to_buffer(),
                        &ContractDocumentRemovalsQuery {
                            document_type_name: document_type_name.to_string(),
                            selection: ContractDocumentRemovalsSelection::DocumentIds(vec![
                                document.id(),
                            ]),
                        },
                    )
                } else {
                    let target_identity_id = st.target_identity_id().ok_or(Error::Drive(
                        DriveError::CorruptedCodeExecution(
                            "a moderation that names no document names an identity",
                        ),
                    ))?;
                    let lists = match st.action() {
                        ContractUserModerationAction::Ban { .. } => {
                            let Some(contract_fetch_info) = self.get_contract_with_fetch_info(
                                contract_id.to_buffer(),
                                false,
                                None,
                                platform_version,
                            )?
                            else {
                                return Err(Error::Proof(ProofError::UnknownContract(format!(
                                    "unknown contract with id {} in contract moderation proving",
                                    contract_id
                                ))));
                            };
                            // A ban removes a suspension too, so its proof covers the lists
                            // that bar; the warning list, which a ban leaves alone, is not
                            // among them.
                            contract_fetch_info
                                .contract
                                .config()
                                .moderation()
                                .map(|moderation| moderation.barring_lists().collect::<Vec<_>>())
                                .unwrap_or_else(|| vec![ContractModerationList::Banlist])
                        }
                        ContractUserModerationAction::Unban { .. } => {
                            vec![ContractModerationList::Banlist]
                        }
                        ContractUserModerationAction::Suspend { .. }
                        | ContractUserModerationAction::Unsuspend { .. } => {
                            vec![ContractModerationList::Suspensions]
                        }
                        ContractUserModerationAction::Warn { .. }
                        | ContractUserModerationAction::ClearWarnings { .. } => {
                            vec![ContractModerationList::Warnings]
                        }
                        ContractUserModerationAction::DeleteDocument { .. }
                        | ContractUserModerationAction::RestoreDocument { .. } => {
                            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                                "a document deletion or restore is proved by the arms above",
                            )))
                        }
                    };
                    Drive::contract_moderation_status_query(
                        contract_id.to_buffer(),
                        target_identity_id.to_buffer(),
                        &lists,
                        &platform_version.drive.grove_version,
                    )?
                }
            }
            // The pot the claim paid out with its last claim (epoch, time, claimant), and the balance
            // of every identity the contract names as a recipient of that pot, or the claimant's
            // alone when the contract does not name it (a seated moderation team's member).
            StateTransition::ContractFeeClaim(st) => {
                let contract_id = st.data_contract_id();
                let Some(contract_fetch_info) = self.get_contract_with_fetch_info(
                    contract_id.to_buffer(),
                    false,
                    None,
                    platform_version,
                )?
                else {
                    return Err(Error::Proof(ProofError::UnknownContract(format!(
                        "unknown contract with id {} in contract fee claim proving",
                        contract_id
                    ))));
                };
                // A claimant the contract does not name as a recipient is on a seated
                // moderation team, which the contract does not name either: its balance alone
                // is proved. Only a contract fee claim takes this arm, a transition protocol
                // version 14 introduced, so no earlier proof changes.
                let recipients: Vec<[u8; 32]> = st
                    .pot()
                    .claim_proof_identities(&contract_fetch_info.contract, st.owner_id())
                    .into_iter()
                    .map(|recipient| recipient.to_buffer())
                    .collect();
                let pot_query = Drive::contract_fee_pots_query(
                    contract_id.to_buffer(),
                    &[st.pot()],
                    &platform_version.drive.grove_version,
                )?;
                let balances_query = Drive::balances_for_identity_ids_query(&recipients);
                PathQuery::merge(
                    vec![&pot_query, &balances_query],
                    &platform_version.drive.grove_version,
                )?
            }
            // Only the rewritten key: the verifier compares that one key.
            StateTransition::IdentityKeyLimitsUpdate(st) => {
                IdentityKeysRequest::new_specific_key_query_without_limit(
                    &st.identity_id().to_buffer(),
                    st.key_id(),
                )
                .into_path_query()
            }
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
            StateTransition::ShieldedTransfer(st) => {
                use crate::drive::shielded::paths::shielded_credit_pool_nullifiers_path_vec;
                use dpp::state_transition::shielded_transfer_transition::accessors::ShieldedTransferTransitionAccessorsV0;

                let nullifier_keys: Vec<Vec<u8>> = st.nullifiers();
                let count = nullifier_keys.len() as u16;

                let mut query = grovedb::Query::new();
                query.insert_keys(nullifier_keys);

                PathQuery::new(
                    shielded_credit_pool_nullifiers_path_vec(),
                    grovedb::SizedQuery::new(query, Some(count), None),
                )
            }
            StateTransition::Shield(st) => {
                Drive::balances_for_clear_addresses_query(st.inputs().keys())
            }
            StateTransition::Unshield(st) => {
                use crate::drive::shielded::paths::shielded_credit_pool_nullifiers_path_vec;
                use dpp::state_transition::unshield_transition::accessors::UnshieldTransitionAccessorsV0;

                let nullifier_keys: Vec<Vec<u8>> = st.nullifiers();
                let mut nf_query = grovedb::Query::new();
                nf_query.insert_keys(nullifier_keys);
                let nullifier_pq = PathQuery::new(
                    shielded_credit_pool_nullifiers_path_vec(),
                    grovedb::SizedQuery::new(nf_query, None, None),
                );

                let mut address_pq =
                    Drive::balances_for_clear_addresses_query(std::iter::once(st.output_address()));
                address_pq.query.limit = None;

                PathQuery::merge(
                    vec![&nullifier_pq, &address_pq],
                    &platform_version.drive.grove_version,
                )?
            }
            StateTransition::ShieldedWithdrawal(st) => {
                use crate::drive::shielded::paths::shielded_credit_pool_nullifiers_path_vec;
                use dpp::data_contracts::withdrawals_contract;
                use dpp::data_contracts::withdrawals_contract::v1::document_types::withdrawal;
                use dpp::document::Document;
                use dpp::state_transition::shielded_withdrawal_transition::accessors::ShieldedWithdrawalTransitionAccessorsV0;

                let nullifier_keys: Vec<Vec<u8>> = st.nullifiers();
                let mut nf_query = grovedb::Query::new();
                nf_query.insert_keys(nullifier_keys.clone());
                let nullifier_pq = PathQuery::new(
                    shielded_credit_pool_nullifiers_path_vec(),
                    grovedb::SizedQuery::new(nf_query, None, None),
                );

                // Compute withdrawal document ID deterministically
                let first_nullifier = nullifier_keys.first().ok_or_else(|| {
                    Error::Proof(ProofError::InvalidTransition(
                        "shielded withdrawal has no nullifiers".to_string(),
                    ))
                })?;
                let mut entropy = Vec::new();
                entropy.extend_from_slice(first_nullifier);
                entropy.extend_from_slice(st.output_script().as_bytes());
                let document_id = Document::generate_document_id_v0(
                    &withdrawals_contract::ID,
                    &withdrawals_contract::OWNER_ID,
                    withdrawal::NAME,
                    &entropy,
                );

                let doc_query = SingleDocumentDriveQuery {
                    contract_id: withdrawals_contract::ID.to_buffer(),
                    document_type_name: withdrawal::NAME.to_string(),
                    document_type_keeps_history: false,
                    document_id: document_id.to_buffer(),
                    block_time_ms: None,
                    contested_status: SingleDocumentDriveQueryContestedStatus::NotContested,
                };
                let mut doc_pq = doc_query.construct_path_query(platform_version)?;
                doc_pq.query.limit = None;

                PathQuery::merge(
                    vec![&nullifier_pq, &doc_pq],
                    &platform_version.drive.grove_version,
                )?
            }
            StateTransition::ShieldFromAssetLock(st) => {
                use dpp::identity::state_transition::AssetLockProved;
                use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;

                let outpoint = st.asset_lock_proof().out_point().ok_or_else(|| {
                    Error::Proof(ProofError::InvalidTransition(
                        "shield from asset lock has no outpoint".to_string(),
                    ))
                })?;
                let outpoint_bytes: [u8; 36] = outpoint.into();

                let mut query = grovedb::Query::new();
                query.insert_key(outpoint_bytes.to_vec());

                let outpoint_pq = PathQuery::new(
                    vec![vec![RootTree::SpentAssetLockTransactions as u8]],
                    grovedb::SizedQuery::new(query, Some(1), None),
                );

                // No accessor trait exposes `surplus_output`, so read it directly off the V0 body.
                let ShieldFromAssetLockTransition::V0(v0) = st;
                match &v0.surplus_output {
                    Some(surplus_address) => {
                        // Mirror the Unshield arm: also prove the balance of the signed
                        // surplus-output address so a light client can confirm the surplus
                        // credit landed there. `PathQuery::merge` rejects sub-queries that carry
                        // limits, so clear both before merging. The verifier rebuilds this exact
                        // merged query (same sub-queries, same cleared limits, same merge) and
                        // verifies it STRICTLY, so the proof cannot carry any extra data beyond
                        // {outpoint, surplus-address}.
                        let mut outpoint_pq = outpoint_pq;
                        outpoint_pq.query.limit = None;

                        let mut address_pq = Drive::balances_for_clear_addresses_query(
                            std::iter::once(surplus_address),
                        );
                        address_pq.query.limit = None;

                        PathQuery::merge(
                            vec![&outpoint_pq, &address_pq],
                            &platform_version.drive.grove_version,
                        )?
                    }
                    None => outpoint_pq,
                }
            }
            StateTransition::IdentityCreateFromShieldedPool(st) => {
                use crate::drive::shielded::paths::shielded_credit_pool_nullifiers_path_vec;
                use dpp::state_transition::identity_create_from_shielded_pool_transition::accessors::IdentityCreateFromShieldedPoolTransitionAccessorsV0;

                // Prove BOTH the spent nullifiers AND the newly-created identity in a single merged
                // multi-root proof. Built STRICT from day one (per #3812): the verifier rebuilds this
                // exact merged query and verifies it with `verify_query` (succinctness on), so the
                // proof cannot carry any branch beyond {nullifiers, identity}. The absence-proof
                // variant is unusable here: it enumerates the query's terminal keys, which is
                // impossible for `full_identity_query`'s unbounded all-keys range.
                let nullifier_keys: Vec<Vec<u8>> = st.nullifiers();
                let mut nf_query = grovedb::Query::new();
                nf_query.insert_keys(nullifier_keys);
                // `PathQuery::merge` rejects sub-queries that carry a limit, so leave it None.
                let nullifier_pq = PathQuery::new(
                    shielded_credit_pool_nullifiers_path_vec(),
                    grovedb::SizedQuery::new(nf_query, None, None),
                );

                let mut identity_pq = Drive::full_identity_query(
                    &st.identity_id().to_buffer(),
                    &platform_version.drive.grove_version,
                )?;
                identity_pq.query.limit = None;

                PathQuery::merge(
                    vec![&nullifier_pq, &identity_pq],
                    &platform_version.drive.grove_version,
                )?
            }
            StateTransition::IdentityTopUpFromShieldedPool(st) => {
                use crate::drive::shielded::paths::shielded_credit_pool_nullifiers_path_vec;
                use dpp::state_transition::identity_top_up_from_shielded_pool_transition::accessors::IdentityTopUpFromShieldedPoolTransitionAccessorsV0;

                // Spent nullifiers AND the credited identity in one STRICT merged proof,
                // exactly the IdentityCreateFromShieldedPool shape.
                let nullifier_keys: Vec<Vec<u8>> = st.nullifiers();
                let mut nf_query = grovedb::Query::new();
                nf_query.insert_keys(nullifier_keys);
                let nullifier_pq = PathQuery::new(
                    shielded_credit_pool_nullifiers_path_vec(),
                    grovedb::SizedQuery::new(nf_query, None, None),
                );

                let mut identity_pq = Drive::full_identity_query(
                    &st.identity_id().to_buffer(),
                    &platform_version.drive.grove_version,
                )?;
                identity_pq.query.limit = None;

                PathQuery::merge(
                    vec![&nullifier_pq, &identity_pq],
                    &platform_version.drive.grove_version,
                )?
            }
            StateTransition::ShieldFromIdentity(st) => {
                // The identity's post-debit balance; the shielded note is not proven
                // (the client learns it through shielded sync, as after `Shield`).
                use dpp::state_transition::shield_from_identity_transition::accessors::ShieldFromIdentityTransitionAccessorsV0;
                Drive::identity_balance_query(&st.identity_id().to_buffer())
            }
        };

        // From version 1 the proof of an owned, fee-paying transition carries
        // the owner's credit balance next to its result, so a wallet learns
        // what the write left it with without a second query. The verifier
        // rebuilds a document batch's merged query and verifies it strictly,
        // and reads the other kinds' result and balance as subsets of the
        // merged proof.
        let path_query =
            if carries_owner_balance && Self::proof_merges_owner_balance_after(state_transition) {
                let owner_id = state_transition.owner_id().ok_or(Error::Proof(
                    ProofError::InvalidTransition(
                        "an owned transition names its owner".to_string(),
                    ),
                ))?;
                let mut path_query = path_query;
                path_query.query.limit = None;
                let owner_balance_query = Drive::identity_balance_query(&owner_id.to_buffer());
                PathQuery::merge(
                    vec![&path_query, &owner_balance_query],
                    &platform_version.drive.grove_version,
                )?
            } else {
                path_query
            };

        let proof = self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )?;

        Ok(ProofCreationResult::new_with_data(proof))
    }

    /// The owned, fee-paying transitions whose version 1 proof gains the owner's
    /// balance by a merge after their own path query is built: document and token
    /// batches, contract creates and updates, identity updates and key limit
    /// updates, and contract moderation.
    fn proof_merges_owner_balance_after(state_transition: &StateTransition) -> bool {
        matches!(
            state_transition,
            StateTransition::Batch(_)
                | StateTransition::DataContractCreate(_)
                | StateTransition::DataContractUpdate(_)
                | StateTransition::IdentityUpdate(_)
                | StateTransition::IdentityKeyLimitsUpdate(_)
                | StateTransition::ContractUserModeration(_)
        )
    }
}
