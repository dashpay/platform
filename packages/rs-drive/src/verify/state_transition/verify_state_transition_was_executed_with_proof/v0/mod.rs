use std::collections::BTreeMap;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::document::{Document, DocumentV0Getters};
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::property_names::PRICE;
use dpp::fee::Credits;
use dpp::identity::PartialIdentity;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::documents_batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::documents_batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::documents_batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use dpp::state_transition::documents_batch_transition::document_transition::{DocumentTransition, DocumentTransitionV0Methods};
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
use dpp::state_transition::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::identity_topup_transition::accessors::IdentityTopUpTransitionAccessorsV0;
use dpp::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use dpp::state_transition::{StateTransition, StateTransitionLike};
use dpp::state_transition::documents_batch_transition::document_create_transition::DocumentFromCreateTransition;
use dpp::state_transition::documents_batch_transition::document_delete_transition::v0::v0_methods::DocumentDeleteTransitionV0Methods;
use dpp::state_transition::documents_batch_transition::document_replace_transition::DocumentFromReplaceTransition;
use dpp::state_transition::documents_batch_transition::document_replace_transition::v0::v0_methods::DocumentReplaceTransitionV0Methods;
use dpp::state_transition::documents_batch_transition::document_transition::document_purchase_transition::v0::v0_methods::DocumentPurchaseTransitionV0Methods;
use dpp::state_transition::documents_batch_transition::document_transition::document_transfer_transition::v0::v0_methods::DocumentTransferTransitionV0Methods;
use dpp::state_transition::documents_batch_transition::document_transition::document_update_price_transition::v0::v0_methods::DocumentUpdatePriceTransitionV0Methods;
use dpp::state_transition::masternode_vote_transition::accessors::MasternodeVoteTransitionAccessorsV0;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::proof_result::StateTransitionProofResult::{VerifiedBalanceTransfer, VerifiedDataContract, VerifiedDocuments, VerifiedIdentity, VerifiedMasternodeVote, VerifiedPartialIdentity};
use dpp::voting::vote_polls::VotePoll;
use dpp::voting::votes::resource_vote::accessors::v0::ResourceVoteGettersV0;
use dpp::voting::votes::Vote;
use platform_version::TryIntoPlatformVersioned;
use platform_version::version::PlatformVersion;
use crate::drive::Drive;
use crate::drive::identity::key::fetch::IdentityKeysRequest;
use crate::verify::RootHash;
use crate::error::Error;
use crate::error::proof::ProofError;
use crate::query::{ContractLookupFn, SingleDocumentDriveQuery, SingleDocumentDriveQueryContestedStatus};

impl Drive {
    #[inline(always)]
    pub(super) fn verify_state_transition_was_executed_with_proof_v0(
        state_transition: &StateTransition,
        block_info: &BlockInfo,
        proof: &[u8],
        known_contracts_provider_fn: &ContractLookupFn,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, StateTransitionProofResult), Error> {
        match state_transition {
            StateTransition::DataContractCreate(data_contract_create) => {
                // we expect to get a contract that matches the state transition
                let (root_hash, contract) = Drive::verify_contract(
                    proof,
                    None,
                    false,
                    true,
                    data_contract_create.data_contract().id().into_buffer(),
                    platform_version,
                )?;
                let contract = contract.ok_or(Error::Proof(ProofError::IncorrectProof(format!("proof did not contain contract with id {} expected to exist because of state transition (create)", data_contract_create.data_contract().id()))))?;
                let contract_for_serialization: DataContractInSerializationFormat = contract
                    .clone()
                    .try_into_platform_versioned(platform_version)?;
                if &contract_for_serialization != data_contract_create.data_contract() {
                    return Err(Error::Proof(ProofError::IncorrectProof(format!("proof of state transition execution did not contain exact expected contract after create with id {}", data_contract_create.data_contract().id()))));
                }
                Ok((root_hash, VerifiedDataContract(contract)))
            }
            StateTransition::DataContractUpdate(data_contract_update) => {
                // we expect to get a contract that matches the state transition
                let (root_hash, contract) = Drive::verify_contract(
                    proof,
                    None,
                    false,
                    true,
                    data_contract_update.data_contract().id().into_buffer(),
                    platform_version,
                )?;
                let contract = contract.ok_or(Error::Proof(ProofError::IncorrectProof(format!("proof did not contain contract with id {} expected to exist because of state transition (update", data_contract_update.data_contract().id()))))?;
                let contract_for_serialization: DataContractInSerializationFormat = contract
                    .clone()
                    .try_into_platform_versioned(platform_version)?;
                if &contract_for_serialization != data_contract_update.data_contract() {
                    return Err(Error::Proof(ProofError::IncorrectProof(format!("proof of state transition execution did not contain exact expected contract after update with id {}", data_contract_update.data_contract().id()))));
                }
                Ok((root_hash, VerifiedDataContract(contract)))
            }
            StateTransition::DocumentsBatch(documents_batch_transition) => {
                if documents_batch_transition.transitions().len() > 1 {
                    return Err(Error::Proof(ProofError::InvalidTransition(format!("version {} does not support more than one document in a document batch transition", platform_version.protocol_version))));
                }
                let Some(transition) = documents_batch_transition.transitions().first() else {
                    return Err(Error::Proof(ProofError::InvalidTransition(
                        "no transition in a document batch transition".to_string(),
                    )));
                };

                let owner_id = documents_batch_transition.owner_id();

                let data_contract_id = transition.data_contract_id();

                let contract = known_contracts_provider_fn(&data_contract_id)?.ok_or(
                    Error::Proof(ProofError::UnknownContract(format!(
                        "unknown contract with id {}",
                        data_contract_id
                    ))),
                )?;

                let document_type = contract
                    .document_type_for_name(transition.document_type_name())
                    .map_err(|e| {
                        Error::Proof(ProofError::UnknownContract(format!(
                            "cannot fetch contract for document {} with id {}: {}",
                            transition.document_type_name(),
                            transition.data_contract_id(),
                            e
                        )))
                    })?;

                let contested_status =
                    if let DocumentTransition::Create(create_transition) = transition {
                        if create_transition.prefunded_voting_balance().is_some() {
                            SingleDocumentDriveQueryContestedStatus::Contested
                        } else {
                            SingleDocumentDriveQueryContestedStatus::NotContested
                        }
                    } else {
                        SingleDocumentDriveQueryContestedStatus::NotContested
                    };

                match transition {
                    DocumentTransition::Create(_) => {}
                    DocumentTransition::Replace(_) => {}
                    DocumentTransition::Delete(_) => {}
                    DocumentTransition::Transfer(_) => {}
                    DocumentTransition::UpdatePrice(_) => {}
                    DocumentTransition::Purchase(_) => {}
                }

                let query = SingleDocumentDriveQuery {
                    contract_id: transition.data_contract_id().into_buffer(),
                    document_type_name: transition.document_type_name().clone(),
                    document_type_keeps_history: document_type.documents_keep_history(),
                    document_id: transition.base().id().into_buffer(),
                    block_time_ms: None, //None because we want latest
                    contested_status,
                };
                let (root_hash, document) =
                    query.verify_proof(false, proof, document_type, platform_version)?;

                match transition {
                    DocumentTransition::Create(create_transition) => {
                        let document = document.ok_or(Error::Proof(ProofError::IncorrectProof(format!("proof did not contain document with id {} expected to exist because of state transition (create)", create_transition.base().id()))))?;
                        let expected_document = Document::try_from_create_transition(
                            create_transition,
                            documents_batch_transition.owner_id(),
                            block_info,
                            &document_type,
                            platform_version,
                        )?;

                        let transient_fields = document_type
                            .transient_fields()
                            .iter()
                            .map(|a| a.as_str())
                            .collect();

                        if !document.is_equal_ignoring_time_based_fields(
                            &expected_document,
                            Some(transient_fields),
                            platform_version,
                        )? {
                            return Err(Error::Proof(ProofError::IncorrectProof(format!("proof of state transition execution did not contain expected document (time fields were not checked) after create with id {}", create_transition.base().id()))));
                        }
                        Ok((
                            root_hash,
                            VerifiedDocuments(BTreeMap::from([(document.id(), Some(document))])),
                        ))
                    }
                    DocumentTransition::Replace(replace_transition) => {
                        let document = document.ok_or(Error::Proof(ProofError::IncorrectProof(format!("proof did not contain document with id {} expected to exist because of state transition (replace)", replace_transition.base().id()))))?;
                        let expected_document = Document::try_from_replace_transition(
                            replace_transition,
                            documents_batch_transition.owner_id(),
                            document.created_at(), //we can trust the created at (as we don't care)
                            document.created_at_block_height(), //we can trust the created at block height (as we don't care)
                            document.created_at_core_block_height(), //we can trust the created at core block height (as we don't care)
                            document.created_at(), //we can trust the created at (as we don't care)
                            document.created_at_block_height(), //we can trust the created at block height (as we don't care)
                            document.created_at_core_block_height(), //we can trust the created at core block height (as we don't care)
                            block_info,
                            &document_type,
                            platform_version,
                        )?;

                        let transient_fields = document_type
                            .transient_fields()
                            .iter()
                            .map(|a| a.as_str())
                            .collect();

                        if !document.is_equal_ignoring_time_based_fields(
                            &expected_document,
                            Some(transient_fields),
                            platform_version,
                        )? {
                            return Err(Error::Proof(ProofError::IncorrectProof(format!("proof of state transition execution did not contain expected document (time fields were not checked) after replace with id {}", replace_transition.base().id()))));
                        }

                        Ok((
                            root_hash,
                            VerifiedDocuments(BTreeMap::from([(document.id(), Some(document))])),
                        ))
                    }
                    DocumentTransition::Transfer(transfer_transition) => {
                        let document = document.ok_or(Error::Proof(ProofError::IncorrectProof(format!("proof did not contain document with id {} expected to exist because of state transition (transfer)", transfer_transition.base().id()))))?;
                        let recipient_owner_id = transfer_transition.recipient_owner_id();

                        if document.owner_id() != recipient_owner_id {
                            return Err(Error::Proof(ProofError::IncorrectProof(format!("proof of state transition execution did not have the transfer executed after expected transfer with id {}", transfer_transition.base().id()))));
                        }

                        Ok((
                            root_hash,
                            VerifiedDocuments(BTreeMap::from([(document.id(), Some(document))])),
                        ))
                    }
                    DocumentTransition::Delete(delete_transition) => {
                        if document.is_some() {
                            return Err(Error::Proof(ProofError::IncorrectProof(format!("proof of state transition execution contained document after delete with id {}", delete_transition.base().id()))));
                        }
                        Ok((
                            root_hash,
                            VerifiedDocuments(BTreeMap::from([(
                                delete_transition.base().id(),
                                None,
                            )])),
                        ))
                    }
                    DocumentTransition::UpdatePrice(update_price_transition) => {
                        let document = document.ok_or(Error::Proof(ProofError::IncorrectProof(format!("proof did not contain document with id {} expected to exist because of state transition (update price)", update_price_transition.base().id()))))?;
                        let new_document_price : Credits = document.properties().get_integer(PRICE).map_err(|e| Error::Proof(ProofError::IncorrectProof(format!("proof did not contain a document that contained a price field with id {} expected to exist because of state transition (update price): {}", update_price_transition.base().id(), e))))?;
                        if new_document_price != update_price_transition.price() {
                            return Err(Error::Proof(ProofError::IncorrectProof(format!("proof of state transition execution did not contain expected document update of price after price update with id {}", update_price_transition.base().id()))));
                        }
                        Ok((
                            root_hash,
                            VerifiedDocuments(BTreeMap::from([(document.id(), Some(document))])),
                        ))
                    }
                    DocumentTransition::Purchase(purchase_transition) => {
                        let document = document.ok_or(Error::Proof(ProofError::IncorrectProof(format!("proof did not contain document with id {} expected to exist because of state transition (purchase)", purchase_transition.base().id()))))?;

                        if document.owner_id() != owner_id {
                            return Err(Error::Proof(ProofError::IncorrectProof(format!("proof of state transition execution did not have the transfer executed after expected transfer with id {}", purchase_transition.base().id()))));
                        }

                        Ok((
                            root_hash,
                            VerifiedDocuments(BTreeMap::from([(document.id(), Some(document))])),
                        ))
                    }
                }
            }
            StateTransition::IdentityCreate(identity_create_transition) => {
                // we expect to get an identity that matches the state transition
                let (root_hash, identity) = Drive::verify_full_identity_by_identity_id(
                    proof,
                    false,
                    identity_create_transition.identity_id().into_buffer(),
                    platform_version,
                )?;
                let identity = identity.ok_or(Error::Proof(ProofError::IncorrectProof(format!("proof did not contain identity {} expected to exist because of state transition (create)", identity_create_transition.identity_id()))))?;
                Ok((root_hash, VerifiedIdentity(identity)))
            }
            StateTransition::IdentityTopUp(identity_top_up_transition) => {
                // we expect to get a new balance and revision
                let identity_id = identity_top_up_transition.identity_id();
                let (root_hash, Some((balance, revision))) =
                    Drive::verify_identity_balance_and_revision_for_identity_id(
                        proof,
                        identity_id.into_buffer(),
                        false,
                        platform_version,
                    )?
                else {
                    return Err(Error::Proof(ProofError::IncorrectProof(
                        format!("proof did not contain balance for identity {} expected to exist because of state transition (top up)", identity_id))));
                };
                Ok((
                    root_hash,
                    VerifiedPartialIdentity(PartialIdentity {
                        id: *identity_top_up_transition.identity_id(),
                        loaded_public_keys: Default::default(),
                        balance: Some(balance),
                        revision: Some(revision),

                        not_found_public_keys: Default::default(),
                    }),
                ))
            }
            StateTransition::IdentityCreditWithdrawal(identity_credit_withdrawal_transition) => {
                // we expect to get an identity that matches the state transition
                let (root_hash, balance) = Drive::verify_identity_balance_for_identity_id(
                    proof,
                    identity_credit_withdrawal_transition
                        .identity_id()
                        .into_buffer(),
                    false,
                    platform_version,
                )?;
                let balance = balance.ok_or(Error::Proof(ProofError::IncorrectProof(format!("proof did not contain balance for identity {} expected to exist because of state transition (withdrawal)", identity_credit_withdrawal_transition.identity_id()))))?;
                Ok((
                    root_hash,
                    VerifiedPartialIdentity(PartialIdentity {
                        id: identity_credit_withdrawal_transition.identity_id(),
                        loaded_public_keys: Default::default(),
                        balance: Some(balance),
                        revision: None,

                        not_found_public_keys: Default::default(),
                    }),
                ))
            }
            StateTransition::IdentityUpdate(identity_update_transition) => {
                // we expect to get an identity that matches the state transition
                let (root_hash, identity) = Drive::verify_identity_keys_by_identity_id(
                    proof,
                    IdentityKeysRequest::new_all_keys_query(
                        &identity_update_transition.identity_id().into_buffer(),
                        None,
                    ),
                    true,
                    true,
                    false,
                    platform_version,
                )?;
                let identity = identity.ok_or(Error::Proof(ProofError::IncorrectProof(format!("proof did not contain update for identity {} expected to exist because of state transition (update)", identity_update_transition.identity_id()))))?;
                Ok((root_hash, VerifiedPartialIdentity(identity)))
            }
            StateTransition::IdentityCreditTransfer(identity_credit_transfer) => {
                // we expect to get an identity that matches the state transition
                let (root_hash_identity, balance_identity) =
                    Drive::verify_identity_balance_for_identity_id(
                        proof,
                        identity_credit_transfer.identity_id().into_buffer(),
                        true,
                        platform_version,
                    )?;

                let (root_hash_recipient, balance_recipient) =
                    Drive::verify_identity_balance_for_identity_id(
                        proof,
                        identity_credit_transfer.recipient_id().into_buffer(),
                        true,
                        platform_version,
                    )?;

                if root_hash_identity != root_hash_recipient {
                    return Err(Error::Proof(ProofError::CorruptedProof("proof is expected to have same root hash for all subsets (identity transfer)".to_string())));
                }

                let balance_identity = balance_identity.ok_or(Error::Proof(ProofError::IncorrectProof(format!("proof did not contain balance for identity sender {} expected to exist because of state transition (transfer)", identity_credit_transfer.identity_id()))))?;
                let balance_recipient = balance_recipient.ok_or(Error::Proof(ProofError::IncorrectProof(format!("proof did not contain balance for identity recipient {} expected to exist because of state transition (transfer)", identity_credit_transfer.recipient_id()))))?;

                Ok((
                    root_hash_identity,
                    VerifiedBalanceTransfer(
                        PartialIdentity {
                            id: identity_credit_transfer.identity_id(),
                            loaded_public_keys: Default::default(),
                            balance: Some(balance_identity),
                            revision: None,

                            not_found_public_keys: Default::default(),
                        },
                        PartialIdentity {
                            id: identity_credit_transfer.recipient_id(),
                            loaded_public_keys: Default::default(),
                            balance: Some(balance_recipient),
                            revision: None,

                            not_found_public_keys: Default::default(),
                        },
                    ),
                ))
            }
            StateTransition::MasternodeVote(masternode_vote) => {
                let pro_tx_hash = masternode_vote.pro_tx_hash();
                let vote = masternode_vote.vote();
                let contract = match vote {
                    Vote::ResourceVote(resource_vote) => match resource_vote.vote_poll() {
                        VotePoll::ContestedDocumentResourceVotePoll(
                            contested_document_resource_vote_poll,
                        ) => known_contracts_provider_fn(
                            &contested_document_resource_vote_poll.contract_id,
                        )?
                        .ok_or(Error::Proof(
                            ProofError::UnknownContract(format!(
                                "unknown contract with id {}",
                                contested_document_resource_vote_poll.contract_id
                            )),
                        ))?,
                    },
                };

                // we expect to get a vote that matches the state transition
                let (root_hash, vote) = Drive::verify_masternode_vote(
                    proof,
                    pro_tx_hash.to_buffer(),
                    vote,
                    &contract,
                    false,
                    platform_version,
                )?;
                let vote = vote.ok_or(Error::Proof(ProofError::IncorrectProof(format!("proof did not contain actual vote for masternode {} expected to exist because of state transition (masternode vote)", masternode_vote.pro_tx_hash()))))?;
                Ok((root_hash, VerifiedMasternodeVote(vote)))
            }
        }
    }
}
