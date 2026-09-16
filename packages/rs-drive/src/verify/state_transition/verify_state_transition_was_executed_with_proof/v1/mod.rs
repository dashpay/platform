use std::collections::BTreeMap;
use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::TokenAmount;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_keeps_history_rules::accessors::v0::TokenKeepsHistoryRulesV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::data_contract::update_values::DataContractUpdateValues;
use dpp::document::{Document, DocumentV0Getters};
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::property_names::PRICE;
use dpp::fee::Credits;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{IdentityPublicKey, KeyID, PartialIdentity};
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::prelude::{AddressNonce, Identifier};
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::identity_create_from_addresses_transition::accessors::IdentityCreateFromAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_to_addresses_transition::accessors::IdentityCreditTransferToAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use dpp::state_transition::{StateTransition, StateTransitionOwned, StateTransitionWitnessSigned};
use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use dpp::state_transition::batch_transition::document_create_transition::DocumentFromCreateTransition;
use dpp::state_transition::batch_transition::document_replace_transition::DocumentFromReplaceTransition;
use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::v0::v0_methods::DocumentTransferTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::document_transition::{DocumentTransition, DocumentTransitionV0Methods};
use dpp::state_transition::batch_transition::batched_transition::document_update_price_transition::v0::v0_methods::DocumentUpdatePriceTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::token_transition::{TokenTransition, TokenTransitionV0Methods};
use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::token_freeze_transition::v0::v0_methods::TokenFreezeTransitionV0Methods;
use dpp::state_transition::batch_transition::token_mint_transition::v0::v0_methods::TokenMintTransitionV0Methods;
use dpp::state_transition::batch_transition::token_transfer_transition::v0::v0_methods::TokenTransferTransitionV0Methods;
use dpp::state_transition::batch_transition::token_unfreeze_transition::v0::v0_methods::TokenUnfreezeTransitionV0Methods;
use dpp::state_transition::address_credit_withdrawal_transition::accessors::AddressCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
use dpp::state_transition::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::identity_topup_transition::accessors::IdentityTopUpTransitionAccessorsV0;
use dpp::state_transition::masternode_vote_transition::accessors::MasternodeVoteTransitionAccessorsV0;
use dpp::state_transition::proof_result::StateTransitionProofOutcome;
use dpp::state_transition::proof_result::StateTransitionProofResult::{VerifiedAddressInfos, VerifiedBalanceTransfer, VerifiedDataContract, VerifiedDocuments, VerifiedIdentity, VerifiedIdentityFullWithAddressInfos, VerifiedIdentityWithAddressInfos, VerifiedMasternodeVote, VerifiedPartialIdentity, VerifiedTokenActionWithDocument, VerifiedTokenBalance, VerifiedTokenGroupActionWithDocument, VerifiedTokenGroupActionWithTokenBalance, VerifiedTokenGroupActionWithTokenIdentityInfo, VerifiedTokenGroupActionWithTokenPricingSchedule, VerifiedTokenIdentitiesBalances, VerifiedTokenIdentityInfo, VerifiedTokenPricingSchedule};
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;
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
    /// Generation 1 adds the delta-based (V1) data contract update: the
    /// delta is materialized on the pre-update contract the known-contracts
    /// provider supplies and the whole result is compared with the proven
    /// contract. Everything else matches generation 0.
    #[inline(always)]
    pub(super) fn verify_state_transition_was_executed_with_proof_v1(
        state_transition: &StateTransition,
        block_info: &BlockInfo,
        proof: &[u8],
        known_contracts_provider_fn: &ContractLookupFn,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, StateTransitionProofOutcome), Error> {
        let (root_hash, result) = match state_transition {
            StateTransition::DataContractCreate(data_contract_create) => {
                // we expect to get a contract that matches the state transition
                let keeps_history = data_contract_create
                    .data_contract()
                    .config()
                    .keeps_history();
                let (root_hash, contract) = Drive::verify_contract(
                    proof,
                    Some(keeps_history),
                    false,
                    true,
                    data_contract_create.data_contract().id().into_buffer(),
                    platform_version,
                )?;
                let contract = contract.ok_or(Error::Proof(ProofError::IncorrectProof(format!("proof did not contain contract with id {} expected to exist because of state transition (create)", data_contract_create.data_contract().id()))))?;
                let contract_for_serialization: DataContractInSerializationFormat = contract
                    .clone()
                    .try_into_platform_versioned(platform_version)?;

                if let Some(mismatch) =
                    contract_for_serialization.first_mismatch(data_contract_create.data_contract())
                {
                    return Err(Error::Proof(ProofError::IncorrectProof(format!("proof of state transition execution did not contain exact expected contract after create with id {}: {}", data_contract_create.data_contract().id(), mismatch))));
                }

                Ok((root_hash, VerifiedDataContract(contract)))
            }
            StateTransition::DataContractUpdate(data_contract_update) => {
                // we expect to get a contract that matches the state transition
                let contract_id = data_contract_update.data_contract_id();
                // A full-contract update says whether the contract keeps
                // history; a delta does not, so both layouts are tried.
                let keeps_history = data_contract_update
                    .data_contract()
                    .map(|data_contract| data_contract.config().keeps_history());
                let (root_hash, contract) = Drive::verify_contract(
                    proof,
                    keeps_history,
                    false,
                    true,
                    contract_id.into_buffer(),
                    platform_version,
                )?;
                let contract = contract.ok_or(Error::Proof(ProofError::IncorrectProof(format!("proof did not contain contract with id {} expected to exist because of state transition (update", contract_id))))?;
                let mismatch = match data_contract_update {
                    DataContractUpdateTransition::V0(v0) => {
                        let contract_for_serialization: DataContractInSerializationFormat =
                            contract
                                .clone()
                                .try_into_platform_versioned(platform_version)?;
                        contract_for_serialization
                            .first_mismatch(&v0.data_contract)
                            .map(|mismatch| mismatch.to_string())
                    }
                    // A delta embeds no contract, so the contract it must have
                    // produced is learnt by applying it to the contract it was
                    // built against. The client that built the delta holds that
                    // contract; the known-contracts provider supplies it, the way
                    // it supplies the schema for a document transition. The
                    // whole materialized contract is then compared, so a proven
                    // state produced by some other update does not pass as this
                    // one.
                    DataContractUpdateTransition::V1(v1) => {
                        if let Some(error) = v1.overlapping_entry() {
                            return Err(Error::Proof(ProofError::InvalidTransition(format!(
                                "delta-based update of contract {} is malformed: {}",
                                contract_id, error
                            ))));
                        }
                        let Some(stored_version) = v1.version.checked_sub(1) else {
                            return Err(Error::Proof(ProofError::InvalidTransition(format!(
                                "delta-based update of contract {} can not produce version 0",
                                contract_id
                            ))));
                        };
                        let stored = known_contracts_provider_fn(&contract_id)?.ok_or(
                            Error::Proof(ProofError::MissingContextRequirement(format!(
                                "verifying a delta-based update of contract {} requires the contract at version {} before the update",
                                contract_id, stored_version
                            ))),
                        )?;
                        if stored.version() != stored_version {
                            return Err(Error::Proof(ProofError::MissingContextRequirement(format!(
                                "verifying a delta-based update of contract {} requires the contract at version {} before the update, the known contract is at version {}",
                                contract_id, stored_version, stored.version()
                            ))));
                        }
                        let expected = DataContractUpdateValues::from(v1)
                            .merge_onto(&stored, block_info)
                            .map_err(|error| {
                                Error::Proof(ProofError::InvalidTransition(format!(
                                    "delta-based update of contract {} does not apply to the known contract: {}",
                                    contract_id, error
                                )))
                            })?;
                        let contract_for_serialization: DataContractInSerializationFormat =
                            contract
                                .clone()
                                .try_into_platform_versioned(platform_version)?;
                        contract_for_serialization
                            .first_mismatch(&DataContractInSerializationFormat::V1(expected))
                            .map(|mismatch| mismatch.to_string())
                    }
                };
                if let Some(mismatch) = mismatch {
                    return Err(Error::Proof(ProofError::IncorrectProof(format!("proof of state transition execution did not contain exact expected contract after update with id {}: {}", contract_id, mismatch))));
                }
                Ok((root_hash, VerifiedDataContract(contract)))
            }
            StateTransition::Batch(documents_batch_transition) => {
                if documents_batch_transition.transitions_len() > 1 {
                    return Err(Error::Proof(ProofError::InvalidTransition(format!("version {} does not support more than one document in a document batch transition", platform_version.protocol_version))));
                }
                let Some(transition) = documents_batch_transition.first_transition() else {
                    return Err(Error::Proof(ProofError::InvalidTransition(
                        "no transition in a document batch transition".to_string(),
                    )));
                };

                let owner_id = documents_batch_transition.owner_id();

                match transition {
                    BatchedTransitionRef::Document(document_transition) => {
                        let data_contract_id = document_transition.data_contract_id();

                        let contract = known_contracts_provider_fn(&data_contract_id)?.ok_or(
                            Error::Proof(ProofError::UnknownContract(format!(
                                "unknown contract with id {} in document verification",
                                data_contract_id
                            ))),
                        )?;

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

                        // indexOnly documents have no primary row: the executed
                        // transition is verified against the single entry its
                        // values produce under the proof index — rebuilt here
                        // from the transition through the same builder the
                        // prover used.
                        {
                            use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
                            use dpp::state_transition::batch_transition::batched_transition::document_index_only_delete_transition::v0::v0_methods::DocumentIndexOnlyDeleteTransitionV0Methods;
                            if document_type.index_only() {
                                let values = match document_transition {
                                    DocumentTransition::Create(create_transition) => {
                                        create_transition.data().clone()
                                    }
                                    // The indexOnlyDelete kind always
                                    // carries its values.
                                    DocumentTransition::IndexOnlyDelete(delete_transition) => {
                                        delete_transition.data().clone()
                                    }
                                    _ => {
                                        return Err(Error::Proof(ProofError::IncorrectProof(
                                            "indexOnly documents only support create and \
                                             indexOnlyDelete"
                                                .to_string(),
                                        )));
                                    }
                                };
                                let path_query = crate::query::index_only_synthesis::index_only_transition_entry_path_query(
                                    contract.id(),
                                    document_type,
                                    &values,
                                    documents_batch_transition.owner_id(),
                                    platform_version,
                                )?;
                                let (root_hash, mut proved) = grovedb::GroveDb::verify_query(
                                    proof,
                                    &path_query,
                                    &platform_version.drive.grove_version,
                                )?;
                                let entry_element =
                                    proved.pop().and_then(|(_path, _key, element)| element);

                                let (root_hash, result) = match document_transition {
                                    DocumentTransition::Create(create_transition) => {
                                        let expected_document =
                                            Document::try_from_create_transition(
                                                create_transition,
                                                documents_batch_transition.owner_id(),
                                                block_info,
                                                &contract,
                                                &document_type,
                                                platform_version,
                                            )?;
                                        // The entry's element shape follows the
                                        // proof index's sum axis: a summable
                                        // index stores `ItemWithSumItem(
                                        // commitment, amount)`, a plain one
                                        // stores `Item(commitment)`. The shapes
                                        // are checked strictly — an element of
                                        // the wrong shape is a wrong proof, and
                                        // for the sum-bearing shape the proved
                                        // amount must equal the created
                                        // document's contribution (the value
                                        // the write path froze into the
                                        // element).
                                        let proof_index = crate::query::index_only_synthesis::index_only_proof_index(&document_type)?;
                                        let payload = match (
                                            entry_element,
                                            proof_index.summable.as_deref(),
                                        ) {
                                            (Some(grovedb::Element::Item(payload, _)), None) => {
                                                payload
                                            }
                                            (
                                                Some(grovedb::Element::ItemWithSumItem(
                                                    payload,
                                                    sum_value,
                                                    _,
                                                )),
                                                Some(sum_property),
                                            ) => {
                                                let expected_sum =
                                                    crate::drive::document::read_document_sum_contribution(
                                                        &expected_document,
                                                        sum_property,
                                                    )?;
                                                if sum_value != expected_sum {
                                                    return Err(Error::Proof(ProofError::IncorrectProof(format!(
                                                        "the proved indexOnly entry's sum contribution does not match the created document {}",
                                                        create_transition.base().id()
                                                    ))));
                                                }
                                                payload
                                            }
                                            _ => {
                                                return Err(Error::Proof(ProofError::IncorrectProof(format!(
                                                    "proof did not contain the indexOnly entry item expected to exist after create of {}",
                                                    create_transition.base().id()
                                                ))));
                                            }
                                        };
                                        // Entry presence alone only proves that
                                        // SOME row projects onto this position;
                                        // the stored row commitment binds the
                                        // entry to its document's full tuple,
                                        // so a pre-existing row with the same
                                        // projection but different values in
                                        // other indexes cannot masquerade as
                                        // this create. (`$createdAt`, when the
                                        // type requires it, enters the
                                        // commitment from the caller-supplied
                                        // block info — the same execution-block
                                        // assumption the expected document is
                                        // built under.)
                                        let expected_commitment =
                                            crate::drive::document::index_only_row_commitment(
                                                &expected_document,
                                                document_type,
                                                platform_version,
                                            )?;
                                        if payload != expected_commitment {
                                            return Err(Error::Proof(ProofError::IncorrectProof(format!(
                                                "the proved indexOnly entry's row commitment does not match the created document {}: the entry belongs to a different row",
                                                create_transition.base().id()
                                            ))));
                                        }
                                        (
                                            root_hash,
                                            VerifiedDocuments(BTreeMap::from([(
                                                expected_document.id(),
                                                Some(expected_document),
                                            )])),
                                        )
                                    }
                                    DocumentTransition::IndexOnlyDelete(delete_transition) => {
                                        if entry_element.is_some() {
                                            return Err(Error::Proof(ProofError::IncorrectProof(format!(
                                                "proof still contained the indexOnly entry after delete of {}",
                                                delete_transition.base().id()
                                            ))));
                                        }
                                        (
                                            root_hash,
                                            VerifiedDocuments(BTreeMap::from([(
                                                delete_transition.base().id(),
                                                None,
                                            )])),
                                        )
                                    }
                                    _ => {
                                        return Err(Error::Proof(ProofError::IncorrectProof(
                                            "indexOnly documents only support create and \
                                             indexOnlyDelete"
                                                .to_string(),
                                        )))
                                    }
                                };

                                // The classifier below is the single
                                // authority on binding, and it returns false
                                // for every indexOnly document transition: a
                                // second create with identical owner/values
                                // (different entropy) shares the proven entry
                                // while only one of them executed, and a
                                // delete's absence proof may describe state
                                // that was already absent — the proof attests
                                // the resulting STATE (`AffectedState`), not
                                // the execution.
                                let outcome = if Self::state_transition_proof_binds_execution(
                                    state_transition,
                                    known_contracts_provider_fn,
                                )? {
                                    StateTransitionProofOutcome::ExecutionProved(result)
                                } else {
                                    StateTransitionProofOutcome::AffectedState(result)
                                };
                                return Ok((root_hash, outcome));
                            }
                        }

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
                        let (root_hash, document) =
                            query.verify_proof(false, proof, document_type, platform_version)?;

                        match document_transition {
                            DocumentTransition::Create(create_transition) => {
                                let document = document.ok_or(Error::Proof(ProofError::IncorrectProof(format!("proof did not contain document with id {} expected to exist because of state transition (create)", create_transition.base().id()))))?;
                                let expected_document = Document::try_from_create_transition(
                                    create_transition,
                                    documents_batch_transition.owner_id(),
                                    block_info,
                                    &contract,
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
                                    return Err(Error::Proof(ProofError::IncorrectProof(format!("proof of state transition execution did not contain expected document (time fields were not checked) after create, got: [{}] vs expected: [{}], state transition is [{}]", document, expected_document, create_transition))));
                                }
                                Ok((
                                    root_hash,
                                    VerifiedDocuments(BTreeMap::from([(
                                        document.id(),
                                        Some(document),
                                    )])),
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
                                    document.creator_id(),
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
                                    return Err(Error::Proof(ProofError::IncorrectProof(format!("proof of state transition execution did not contain expected document (time fields were not checked) after replace, got: [{}] vs expected: [{}], state transition is [{}]", document, expected_document, replace_transition))));
                                }

                                Ok((
                                    root_hash,
                                    VerifiedDocuments(BTreeMap::from([(
                                        document.id(),
                                        Some(document),
                                    )])),
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
                                    VerifiedDocuments(BTreeMap::from([(
                                        document.id(),
                                        Some(document),
                                    )])),
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
                                    VerifiedDocuments(BTreeMap::from([(
                                        document.id(),
                                        Some(document),
                                    )])),
                                ))
                            }
                            DocumentTransition::Purchase(purchase_transition) => {
                                let document = document.ok_or(Error::Proof(ProofError::IncorrectProof(format!("proof did not contain document with id {} expected to exist because of state transition (purchase)", purchase_transition.base().id()))))?;

                                if document.owner_id() != owner_id {
                                    return Err(Error::Proof(ProofError::IncorrectProof(format!("proof of state transition execution did not have the transfer executed after expected transfer with id {}", purchase_transition.base().id()))));
                                }

                                Ok((
                                    root_hash,
                                    VerifiedDocuments(BTreeMap::from([(
                                        document.id(),
                                        Some(document),
                                    )])),
                                ))
                            }
                            DocumentTransition::IndexOnlyDelete(_) => {
                                // Only reachable when the doctype is NOT
                                // indexOnly (the indexOnly branch above
                                // returns first): an indexOnlyDelete aimed at
                                // a stored type can never have executed, so
                                // there is nothing a proof could attest.
                                Err(Error::Proof(ProofError::InvalidTransition(
                                    "an indexOnlyDelete cannot execute against a stored \
                                     document type"
                                        .to_string(),
                                )))
                            }
                        }
                    }
                    BatchedTransitionRef::Token(token_transition) => {
                        let data_contract_id = token_transition.data_contract_id();
                        let token_id = token_transition.token_id();

                        let contract = known_contracts_provider_fn(&data_contract_id)?.ok_or(
                            Error::Proof(ProofError::UnknownContract(format!(
                                "unknown contract with id {} in token verification",
                                data_contract_id
                            ))),
                        )?;

                        let identity_contract_nonce =
                            token_transition.base().identity_contract_nonce();

                        let token_history_document_type_name =
                            token_transition.historical_document_type_name().to_string();

                        let token_history_contract = load_system_data_contract(
                            SystemDataContract::TokenHistory,
                            platform_version,
                        )?;

                        let token_history_document_type =
                            token_transition.historical_document_type(&token_history_contract)?;

                        let token_config = contract.expected_token_configuration(
                            token_transition.base().token_contract_position(),
                        )?;
                        let keeps_historical_document = token_config.keeps_history();

                        let historical_query = || {
                            let query = SingleDocumentDriveQuery {
                                contract_id: token_history_contract.id().into_buffer(),
                                document_type_name: token_history_document_type_name,
                                document_type_keeps_history: false,
                                document_id: token_transition
                                    .historical_document_id(owner_id)
                                    .to_buffer(),
                                block_time_ms: None, //None because we want latest
                                contested_status:
                                    SingleDocumentDriveQueryContestedStatus::NotContested,
                            };

                            let is_group_action =
                                token_transition.base().using_group_info().is_some();

                            let (root_hash, document) = query.verify_proof(
                                is_group_action, // it will be a subset if it is a group action
                                proof,
                                token_history_document_type,
                                platform_version,
                            )?;

                            if let Some(document) = &document {
                                let expected_document = token_transition
                                    .build_historical_document(
                                        token_id,
                                        owner_id,
                                        identity_contract_nonce,
                                        &BlockInfo::default(),
                                        token_config,
                                        platform_version,
                                    )?;

                                // Some fields are populated by the drive,
                                // so we need to ignore them
                                let ignore_fields = match token_transition {
                                    TokenTransition::DestroyFrozenFunds(_) => {
                                        Some(vec!["destroyedAmount", "note"])
                                    }
                                    TokenTransition::Burn(_) => Some(vec!["burnFromId", "note"]),
                                    TokenTransition::Claim(_) => Some(vec!["amount"]),
                                    TokenTransition::DirectPurchase(_) => {
                                        let purchase_cost: Credits =
                                            document.properties().get_integer("purchaseCost")?;
                                        let agreed_to_purchase_cost: Credits = expected_document
                                            .properties()
                                            .get_integer("purchaseCost")?;
                                        if purchase_cost > agreed_to_purchase_cost {
                                            return Err(Error::Proof(ProofError::UnexpectedResultProof(format!("proof of state transition execution showed a purchase price of {}, whereas we only agreed to {}, state transition is [{}]", purchase_cost, agreed_to_purchase_cost, token_transition))));
                                        }
                                        Some(vec!["purchaseCost"])
                                    }
                                    TokenTransition::Mint(_)
                                    | TokenTransition::Freeze(_)
                                    | TokenTransition::Unfreeze(_)
                                    | TokenTransition::EmergencyAction(_)
                                    | TokenTransition::ConfigUpdate(_)
                                    | TokenTransition::SetPriceForDirectPurchase(_)
                                        if token_transition.base().using_group_info().is_some() =>
                                    {
                                        Some(vec!["note"])
                                    }
                                    _ => None,
                                };

                                if !document.is_equal_ignoring_time_based_fields(
                                    &expected_document,
                                    ignore_fields,
                                    platform_version,
                                )? {
                                    return Err(Error::Proof(ProofError::UnexpectedResultProof(format!("proof of state transition execution did not show the correct historical document got: [{}] vs expected: [{}], state transition is [{}]", document, expected_document, token_transition))));
                                }
                            }

                            if let Some(group_state_transition_info) =
                                token_transition.base().using_group_info()
                            {
                                let action_status = if document.is_some() {
                                    GroupActionStatus::ActionClosed
                                } else {
                                    GroupActionStatus::ActionActive
                                };
                                let sum_power = Drive::verify_action_signer_and_total_power(
                                    proof,
                                    data_contract_id,
                                    group_state_transition_info.group_contract_position,
                                    Some(action_status),
                                    group_state_transition_info.action_id,
                                    owner_id,
                                    true,
                                    platform_version,
                                )?
                                .2;
                                Ok((
                                    root_hash,
                                    VerifiedTokenGroupActionWithDocument(sum_power, document),
                                ))
                            } else {
                                let document = document.ok_or(Error::Proof(ProofError::IncorrectProof(format!("proof did not contain document of type `{}` expected to exist because the token keeps historical documents", token_transition.historical_document_type_name()))))?;
                                Ok((root_hash, VerifiedTokenActionWithDocument(document)))
                            }
                        };
                        match token_transition {
                            TokenTransition::Burn(_) => {
                                if keeps_historical_document.keeps_burning_history() {
                                    historical_query()
                                } else if let Some(group_state_transition_info) =
                                    token_transition.base().using_group_info()
                                {
                                    let (_root_hash, status, sum_power) =
                                        Drive::verify_action_signer_and_total_power(
                                            proof,
                                            data_contract_id,
                                            group_state_transition_info.group_contract_position,
                                            None,
                                            group_state_transition_info.action_id,
                                            owner_id,
                                            true,
                                            platform_version,
                                        )?;

                                    let (root_hash, balance) =
                                        Drive::verify_token_balance_for_identity_id(
                                            proof,
                                            token_id.into_buffer(),
                                            owner_id.into_buffer(),
                                            true,
                                            platform_version,
                                        )?;
                                    if status == GroupActionStatus::ActionClosed
                                        && balance.is_none()
                                    {
                                        return Err(Error::Proof(ProofError::IncorrectProof(
                                            format!("proof did not contain token balance for identity {} expected to exist because of state transition (token burn)", owner_id))));
                                    };

                                    Ok((
                                        root_hash,
                                        VerifiedTokenGroupActionWithTokenBalance(
                                            sum_power, status, balance,
                                        ),
                                    ))
                                } else {
                                    {
                                        let (root_hash, Some(balance)) =
                                            Drive::verify_token_balance_for_identity_id(
                                                proof,
                                                token_id.into_buffer(),
                                                owner_id.into_buffer(),
                                                false,
                                                platform_version,
                                            )?
                                        else {
                                            return Err(Error::Proof(ProofError::IncorrectProof(
                                                    format!("proof did not contain token balance for identity {} expected to exist because of state transition (token burn)", owner_id))));
                                        };
                                        Ok((root_hash, VerifiedTokenBalance(owner_id, balance)))
                                    }
                                }
                            }
                            TokenTransition::Mint(token_mint_transition) => {
                                if keeps_historical_document.keeps_minting_history() {
                                    historical_query()
                                } else if let Some(group_state_transition_info) =
                                    token_transition.base().using_group_info()
                                {
                                    let (_root_hash, status, sum_power) =
                                        Drive::verify_action_signer_and_total_power(
                                            proof,
                                            data_contract_id,
                                            group_state_transition_info.group_contract_position,
                                            None,
                                            group_state_transition_info.action_id,
                                            owner_id,
                                            true,
                                            platform_version,
                                        )?;

                                    let recipient_id =
                                        token_mint_transition.recipient_id(token_config)?;

                                    let (root_hash, balance) =
                                        Drive::verify_token_balance_for_identity_id(
                                            proof,
                                            token_id.into_buffer(),
                                            recipient_id.into_buffer(),
                                            true,
                                            platform_version,
                                        )?;
                                    if status == GroupActionStatus::ActionClosed
                                        && balance.is_none()
                                    {
                                        return Err(Error::Proof(ProofError::IncorrectProof(
                                            format!("proof did not contain token balance for identity {} expected to exist because of state transition (token mint)", owner_id))));
                                    };

                                    Ok((
                                        root_hash,
                                        VerifiedTokenGroupActionWithTokenBalance(
                                            sum_power, status, balance,
                                        ),
                                    ))
                                } else {
                                    {
                                        let recipient_id =
                                            token_mint_transition.recipient_id(token_config)?;
                                        let (root_hash, Some(balance)) =
                                            Drive::verify_token_balance_for_identity_id(
                                                proof,
                                                token_id.into_buffer(),
                                                recipient_id.into_buffer(),
                                                false,
                                                platform_version,
                                            )?
                                        else {
                                            return Err(Error::Proof(ProofError::IncorrectProof(
                                                    format!("proof did not contain token balance for identity {} expected to exist because of state transition (token mint)", recipient_id))));
                                        };
                                        Ok((root_hash, VerifiedTokenBalance(recipient_id, balance)))
                                    }
                                }
                            }
                            TokenTransition::Transfer(token_transfer_transition) => {
                                if keeps_historical_document.keeps_transfer_history() {
                                    historical_query()
                                } else {
                                    {
                                        let recipient_id = token_transfer_transition.recipient_id();
                                        let identity_ids =
                                            [owner_id.to_buffer(), recipient_id.to_buffer()];
                                        let (root_hash, balances): (
                                            RootHash,
                                            BTreeMap<Identifier, Option<TokenAmount>>,
                                        ) = Drive::verify_token_balances_for_identity_ids(
                                            proof,
                                            token_id.into_buffer(),
                                            &identity_ids,
                                            false,
                                            platform_version,
                                        )?;

                                        let balances = balances.into_iter().map(|(id, maybe_balance)| {
                                                    let balance = maybe_balance.ok_or(Error::Proof(ProofError::IncorrectProof(
                                                        format!("proof did not contain token balance for identity {} expected to exist because of state transition (token transfer)", id))))?;
                                                    Ok((id, balance))
                                                }).collect::<Result<_, Error>>()?;

                                        Ok((root_hash, VerifiedTokenIdentitiesBalances(balances)))
                                    }
                                }
                            }
                            TokenTransition::Freeze(token_freeze_transition) => {
                                if keeps_historical_document.keeps_freezing_history() {
                                    historical_query()
                                } else if let Some(group_state_transition_info) =
                                    token_transition.base().using_group_info()
                                {
                                    let (_root_hash, status, sum_power) =
                                        Drive::verify_action_signer_and_total_power(
                                            proof,
                                            data_contract_id,
                                            group_state_transition_info.group_contract_position,
                                            None,
                                            group_state_transition_info.action_id,
                                            owner_id,
                                            true,
                                            platform_version,
                                        )?;

                                    let (root_hash, identity_token_info) =
                                        Drive::verify_token_info_for_identity_id(
                                            proof,
                                            token_id.into_buffer(),
                                            token_freeze_transition
                                                .frozen_identity_id()
                                                .into_buffer(),
                                            true,
                                            platform_version,
                                        )?;
                                    if status == GroupActionStatus::ActionClosed
                                        && identity_token_info.is_none()
                                    {
                                        return Err(Error::Proof(ProofError::IncorrectProof(
                                            format!("proof did not contain token identity info for identity {} expected to exist because of state transition (token freeze)", owner_id))));
                                    };

                                    Ok((
                                        root_hash,
                                        VerifiedTokenGroupActionWithTokenIdentityInfo(
                                            sum_power,
                                            status,
                                            identity_token_info,
                                        ),
                                    ))
                                } else {
                                    let (root_hash, Some(identity_token_info)) =
                                        Drive::verify_token_info_for_identity_id(
                                            proof,
                                            token_id.into_buffer(),
                                            token_freeze_transition
                                                .frozen_identity_id()
                                                .into_buffer(),
                                            false,
                                            platform_version,
                                        )?
                                    else {
                                        return Err(Error::Proof(ProofError::IncorrectProof(
                                                format!("proof did not contain token info for identity {} expected to exist because of state transition (token freeze)", token_freeze_transition.frozen_identity_id()))));
                                    };
                                    if !identity_token_info.frozen() {
                                        return Err(Error::Proof(ProofError::IncorrectProof(
                                                format!("proof contained token info saying this token was not frozen for identity {}", token_freeze_transition.frozen_identity_id()))));
                                    }
                                    Ok((
                                        root_hash,
                                        VerifiedTokenIdentityInfo(owner_id, identity_token_info),
                                    ))
                                }
                            }
                            TokenTransition::Unfreeze(token_unfreeze_transition) => {
                                if keeps_historical_document.keeps_freezing_history() {
                                    historical_query()
                                } else if let Some(group_state_transition_info) =
                                    token_transition.base().using_group_info()
                                {
                                    let (_root_hash, status, sum_power) =
                                        Drive::verify_action_signer_and_total_power(
                                            proof,
                                            data_contract_id,
                                            group_state_transition_info.group_contract_position,
                                            None,
                                            group_state_transition_info.action_id,
                                            owner_id,
                                            true,
                                            platform_version,
                                        )?;

                                    let (root_hash, identity_token_info) =
                                        Drive::verify_token_info_for_identity_id(
                                            proof,
                                            token_id.into_buffer(),
                                            token_unfreeze_transition
                                                .frozen_identity_id()
                                                .into_buffer(),
                                            true,
                                            platform_version,
                                        )?;
                                    if status == GroupActionStatus::ActionClosed
                                        && identity_token_info.is_none()
                                    {
                                        return Err(Error::Proof(ProofError::IncorrectProof(
                                            format!("proof did not contain token identity info for identity {} expected to exist because of state transition (token unfreeze)", owner_id))));
                                    };

                                    Ok((
                                        root_hash,
                                        VerifiedTokenGroupActionWithTokenIdentityInfo(
                                            sum_power,
                                            status,
                                            identity_token_info,
                                        ),
                                    ))
                                } else {
                                    let (root_hash, Some(identity_token_info)) =
                                        Drive::verify_token_info_for_identity_id(
                                            proof,
                                            token_id.into_buffer(),
                                            token_unfreeze_transition
                                                .frozen_identity_id()
                                                .into_buffer(),
                                            false,
                                            platform_version,
                                        )?
                                    else {
                                        return Err(Error::Proof(ProofError::IncorrectProof(
                                                format!("proof did not contain token info for identity {} expected to exist because of state transition (token freeze)", token_unfreeze_transition.frozen_identity_id()))));
                                    };
                                    if identity_token_info.frozen() {
                                        return Err(Error::Proof(ProofError::IncorrectProof(
                                                format!("proof contained token info saying this token was frozen for identity {} when we just unfroze it", token_unfreeze_transition.frozen_identity_id()))));
                                    }
                                    Ok((
                                        root_hash,
                                        VerifiedTokenIdentityInfo(owner_id, identity_token_info),
                                    ))
                                }
                            }
                            TokenTransition::DirectPurchase(_) => {
                                if keeps_historical_document.keeps_direct_purchase_history() {
                                    historical_query()
                                } else {
                                    let (root_hash, Some(balance)) =
                                        Drive::verify_token_balance_for_identity_id(
                                            proof,
                                            token_id.into_buffer(),
                                            owner_id.into_buffer(),
                                            false,
                                            platform_version,
                                        )?
                                    else {
                                        return Err(Error::Proof(ProofError::IncorrectProof(
                                            format!("proof did not contain token balance for identity {} expected to exist because of state transition (token direct purchase)", owner_id))));
                                    };
                                    Ok((root_hash, VerifiedTokenBalance(owner_id, balance)))
                                }
                            }
                            TokenTransition::SetPriceForDirectPurchase(_) => {
                                if keeps_historical_document.keeps_direct_pricing_history() {
                                    historical_query()
                                } else if let Some(group_state_transition_info) =
                                    token_transition.base().using_group_info()
                                {
                                    let (_root_hash, status, sum_power) =
                                        Drive::verify_action_signer_and_total_power(
                                            proof,
                                            data_contract_id,
                                            group_state_transition_info.group_contract_position,
                                            None,
                                            group_state_transition_info.action_id,
                                            owner_id,
                                            true,
                                            platform_version,
                                        )?;

                                    let (root_hash, token_pricing_schedule) =
                                        Drive::verify_token_direct_selling_price(
                                            proof,
                                            token_id.into_buffer(),
                                            true,
                                            platform_version,
                                        )?;
                                    if status == GroupActionStatus::ActionClosed
                                        && token_pricing_schedule.is_none()
                                    {
                                        return Err(Error::Proof(ProofError::IncorrectProof(
                                            format!("proof did not contain token identity info for identity {} expected to exist because of state transition (token set price for direct purchase)", owner_id))));
                                    };

                                    Ok((
                                        root_hash,
                                        VerifiedTokenGroupActionWithTokenPricingSchedule(
                                            sum_power,
                                            status,
                                            token_pricing_schedule,
                                        ),
                                    ))
                                } else {
                                    let (root_hash, token_pricing_schedule) =
                                        Drive::verify_token_direct_selling_price(
                                            proof,
                                            token_id.into_buffer(),
                                            false,
                                            platform_version,
                                        )?;
                                    Ok((
                                        root_hash,
                                        VerifiedTokenPricingSchedule(
                                            owner_id,
                                            token_pricing_schedule,
                                        ),
                                    ))
                                }
                            }
                            TokenTransition::DestroyFrozenFunds(_)
                            | TokenTransition::EmergencyAction(_)
                            | TokenTransition::ConfigUpdate(_)
                            | TokenTransition::Claim(_) => historical_query(),
                        }
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
                let expected_keys: BTreeMap<KeyID, IdentityPublicKey> = identity_create_transition
                    .public_keys()
                    .iter()
                    .map(|key| {
                        let stored_key: IdentityPublicKey = key.into();
                        (stored_key.id(), stored_key)
                    })
                    .collect();
                if expected_keys.len() != identity_create_transition.public_keys().len() {
                    return Err(Error::Proof(ProofError::InvalidTransition(
                        "identity create transition contains duplicate public key ids".to_string(),
                    )));
                }
                if identity.public_keys() != &expected_keys {
                    return Err(Error::Proof(ProofError::IncorrectProof(
                        "identity create proof contains public keys that do not match the transition"
                            .to_string(),
                    )));
                }
                Ok((root_hash, VerifiedIdentity(identity)))
            }
            StateTransition::IdentityTopUp(identity_top_up_transition) => {
                // snapshot of the identity's balance and revision at the proof's block
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
                {
                    // snapshot of the identity's balance at the proof's block
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
                    false,
                    false,
                    platform_version,
                )?;
                let identity = identity.ok_or(Error::Proof(ProofError::IncorrectProof(format!("proof did not contain update for identity {} expected to exist because of state transition (update)", identity_update_transition.identity_id()))))?;

                if identity.revision != Some(identity_update_transition.revision()) {
                    return Err(Error::Proof(ProofError::IncorrectProof(format!(
                        "identity update proof contains revision {:?}, expected {}",
                        identity.revision,
                        identity_update_transition.revision()
                    ))));
                }

                for key in identity_update_transition.public_keys_to_add() {
                    let expected_key: IdentityPublicKey = key.into();
                    if identity.loaded_public_keys.get(&expected_key.id()) != Some(&expected_key) {
                        return Err(Error::Proof(ProofError::IncorrectProof(format!(
                            "identity update proof does not contain the expected state for added key {}",
                            expected_key.id()
                        ))));
                    }
                }

                for key_id in identity_update_transition.public_key_ids_to_disable() {
                    let Some(proved_key) = identity.loaded_public_keys.get(key_id) else {
                        return Err(Error::Proof(ProofError::IncorrectProof(format!(
                            "identity update proof does not contain disabled key {}",
                            key_id
                        ))));
                    };
                    // The proof may be generated at a block later than the one that
                    // executed the update; `disabled_at` never changes afterwards, so
                    // require the key to be disabled at or before the proof's block
                    // rather than exactly at it.
                    match proved_key.disabled_at() {
                        Some(disabled_at) if disabled_at <= block_info.time_ms => {}
                        _ => {
                            return Err(Error::Proof(ProofError::IncorrectProof(format!(
                                "identity update proof contains an unexpected disabled timestamp for key {}",
                                key_id
                            ))));
                        }
                    }
                }
                Ok((root_hash, VerifiedPartialIdentity(identity)))
            }
            StateTransition::IdentityCreditTransfer(identity_credit_transfer) => {
                // snapshot of the sender's and recipient's balances at the proof's block
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
                                "unknown contract with id {} in resource vote verification",
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
            StateTransition::IdentityCreditTransferToAddresses(st) => {
                let identity_id = st.identity_id();
                // verify_subset_of_proof=true because we verify identity balance/revision
                // and address balances as separate subsets of the same merged proof
                let (root_hash_identity, Some((balance, revision)), address_balances) =
                    Drive::verify_identity_balance_revision_and_addresses_from_inputs(
                        proof,
                        identity_id.to_buffer(),
                        st.recipient_addresses().keys(),
                        true,
                        platform_version,
                    )?
                else {
                    return Err(Error::Proof(ProofError::IncorrectProof(
                        format!("proof did not contain balance for identity {} expected to exist because of state transition (identity credit transfer to addresses)", identity_id)
                    )));
                };

                Ok((
                    root_hash_identity,
                    VerifiedIdentityWithAddressInfos(
                        PartialIdentity {
                            id: identity_id,
                            loaded_public_keys: Default::default(),
                            balance: Some(balance),
                            revision: Some(revision),
                            not_found_public_keys: Default::default(),
                        },
                        address_balances,
                    ),
                ))
            }
            StateTransition::IdentityCreateFromAddresses(st) => {
                use dpp::state_transition::StateTransitionIdentityIdFromInputs;
                let identity_id = st.identity_id_from_inputs().map_err(|e| {
                    Error::Proof(ProofError::CorruptedProof(format!(
                        "Failed to calculate identity id from inputs: {}",
                        e
                    )))
                })?;
                let (root_hash_identity, identity) = Drive::verify_full_identity_by_identity_id(
                    proof,
                    true,
                    identity_id.into_buffer(),
                    platform_version,
                )?;
                let identity = identity.ok_or(Error::Proof(ProofError::IncorrectProof(format!("proof did not contain identity {} expected to exist because of state transition (create from addresses)", identity_id))))?;

                let expected_keys: BTreeMap<KeyID, IdentityPublicKey> = st
                    .public_keys()
                    .iter()
                    .map(|key| {
                        let stored_key: IdentityPublicKey = key.into();
                        (stored_key.id(), stored_key)
                    })
                    .collect();
                if expected_keys.len() != st.public_keys().len() {
                    return Err(Error::Proof(ProofError::InvalidTransition(
                        "identity create from addresses transition contains duplicate public key ids"
                            .to_string(),
                    )));
                }
                if identity.public_keys() != &expected_keys {
                    return Err(Error::Proof(ProofError::IncorrectProof(
                        "identity create from addresses proof contains public keys that do not match the transition"
                            .to_string(),
                    )));
                }

                let addresses_to_check = st
                    .inputs()
                    .keys()
                    .chain(st.output().into_iter().map(|(address, _)| address));

                let (root_hash_addresses, address_balances): (
                    RootHash,
                    BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>,
                ) = Drive::verify_addresses_infos(
                    proof,
                    addresses_to_check,
                    true,
                    platform_version,
                )?;

                if root_hash_identity != root_hash_addresses {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "proof is expected to have same root hash for identity and address infos"
                            .to_string(),
                    )));
                }

                Ok((
                    root_hash_identity,
                    VerifiedIdentityFullWithAddressInfos(identity, address_balances),
                ))
            }
            StateTransition::IdentityTopUpFromAddresses(st) => {
                // Verify revision and balance for the identity
                use dpp::state_transition::identity_topup_from_addresses_transition::accessors::IdentityTopUpFromAddressesTransitionAccessorsV0;
                let identity_id = st.identity_id();
                let addresses_to_check = st
                    .inputs()
                    .keys()
                    .chain(st.output().into_iter().map(|(address, _)| address));
                // verify_subset_of_proof=true because we verify identity balance/revision
                // and address balances as separate subsets of the same merged proof
                let (root_hash_identity, Some((balance, revision)), address_balances) =
                    Drive::verify_identity_balance_revision_and_addresses_from_inputs(
                        proof,
                        identity_id.to_buffer(),
                        addresses_to_check,
                        true,
                        platform_version,
                    )?
                else {
                    return Err(Error::Proof(ProofError::IncorrectProof(
                        format!("proof did not contain balance for identity {} expected to exist because of state transition (top up from addresses)", identity_id))));
                };

                Ok((
                    root_hash_identity,
                    VerifiedIdentityWithAddressInfos(
                        PartialIdentity {
                            id: *identity_id,
                            loaded_public_keys: Default::default(),
                            balance: Some(balance),
                            revision: Some(revision),
                            not_found_public_keys: Default::default(),
                        },
                        address_balances,
                    ),
                ))
            }
            StateTransition::AddressFundsTransfer(st) => {
                // snapshot of the input and output addresses at the proof's block
                use dpp::state_transition::address_funds_transfer_transition::accessors::AddressFundsTransferTransitionAccessorsV0;
                use dpp::state_transition::StateTransitionWitnessSigned;
                let (root_hash, address_balances): (
                    RootHash,
                    BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>,
                ) = Drive::verify_addresses_infos(
                    proof,
                    st.inputs().keys().chain(st.outputs().keys()),
                    false,
                    platform_version,
                )?;

                Ok((root_hash, VerifiedAddressInfos(address_balances)))
            }
            StateTransition::AddressFundingFromAssetLock(st) => {
                // snapshot of the input and output addresses at the proof's block
                use dpp::state_transition::address_funding_from_asset_lock_transition::accessors::AddressFundingFromAssetLockTransitionAccessorsV0;
                let (root_hash, balances): (
                    RootHash,
                    BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>,
                ) = Drive::verify_addresses_infos(
                    proof,
                    st.inputs().keys().chain(st.outputs().keys()),
                    false,
                    platform_version,
                )?;

                Ok((root_hash, VerifiedAddressInfos(balances)))
            }
            StateTransition::AddressCreditWithdrawal(st) => {
                // snapshot of the input and output addresses at the proof's block
                use dpp::state_transition::StateTransitionWitnessSigned;
                let addresses_to_check = st
                    .inputs()
                    .keys()
                    .chain(st.output().into_iter().map(|(address, _)| address));
                let (root_hash, balances): (
                    RootHash,
                    BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>,
                ) = Drive::verify_addresses_infos(
                    proof,
                    addresses_to_check,
                    false,
                    platform_version,
                )?;

                Ok((root_hash, VerifiedAddressInfos(balances)))
            }
            StateTransition::Shield(st) => {
                // snapshot of the input addresses at the proof's block
                use dpp::state_transition::StateTransitionWitnessSigned;
                let (root_hash, balances): (
                    RootHash,
                    BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>,
                ) = Drive::verify_addresses_infos(
                    proof,
                    st.inputs().keys(),
                    false,
                    platform_version,
                )?;
                Ok((root_hash, VerifiedAddressInfos(balances)))
            }
            StateTransition::Unshield(st) => {
                use crate::drive::shielded::paths::shielded_credit_pool_nullifiers_path_vec;
                use dpp::state_transition::proof_result::StateTransitionProofResult::VerifiedShieldedNullifiersWithAddressInfos;
                use dpp::state_transition::unshield_transition::accessors::UnshieldTransitionAccessorsV0;
                use grovedb::Element;

                let nullifier_keys: Vec<Vec<u8>> = st.nullifiers();

                // Reconstruct the prove side's merged query — nullifier spend-status ∪
                // output-address balance — and verify it strictly. See
                // `verify_merged_query_strict` for why a single strict merged verify is
                // sound and rejects proofs padded with extra subtree branches.
                let mut nf_query = grovedb::Query::new();
                nf_query.insert_keys(nullifier_keys);
                let nullifier_pq = grovedb::PathQuery::new(
                    shielded_credit_pool_nullifiers_path_vec(),
                    grovedb::SizedQuery::new(nf_query, None, None),
                );

                let address_pq =
                    Drive::balances_for_clear_addresses_query(std::iter::once(st.output_address()));

                let (root_hash, proved_key_values) = Self::verify_merged_query_strict(
                    proof,
                    vec![nullifier_pq, address_pq],
                    platform_version,
                )?;

                // Partition the proved key/values by path: entries under the nullifiers tree are
                // spend statuses, entries under the clear-address pool are address balances.
                let nullifiers_path = shielded_credit_pool_nullifiers_path_vec();
                let addresses_path = Drive::clear_addresses_path();

                let mut statuses: Vec<(Vec<u8>, bool)> = Vec::new();
                let mut balances: BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>> =
                    BTreeMap::new();

                for (path, key, element) in proved_key_values {
                    if path == nullifiers_path {
                        // A present element means the nullifier is spent; absence means unspent.
                        statuses.push((key, element.is_some()));
                    } else if path == addresses_path {
                        // Mirror `verify_addresses_infos_v0`: reconstruct the address from the key
                        // and decode the `ItemWithSumItem` (nonce, balance) element.
                        let address = PlatformAddress::from_bytes(&key).map_err(|e| {
                            Error::Proof(ProofError::CorruptedProof(format!(
                                "failed to deserialize output PlatformAddress: {}",
                                e
                            )))
                        })?;

                        let balance_info = element
                            .map(|element| {
                                let Element::ItemWithSumItem(nonce_vec, balance_i64, _) = element
                                else {
                                    return Err(Error::Proof(ProofError::CorruptedProof(
                                        "expected an item with sum item element".to_string(),
                                    )));
                                };

                                let nonce_bytes: [u8; 4] = nonce_vec.try_into().map_err(|_| {
                                    Error::Proof(ProofError::IncorrectValueSize(
                                        "nonce should be 4 bytes",
                                    ))
                                })?;
                                let nonce = AddressNonce::from_be_bytes(nonce_bytes);

                                if balance_i64 < 0 {
                                    return Err(Error::Proof(ProofError::CorruptedProof(
                                        "balance cannot be negative".to_string(),
                                    )));
                                }

                                Ok((nonce, balance_i64 as Credits))
                            })
                            .transpose()?;

                        // Mirror ShieldedWithdrawal's singleton-subtree invariant: the
                        // address sub-query targets exactly one key, so a second entry under
                        // the clear-address pool is a malformed proof, not last-write-wins.
                        if balances.contains_key(&address) {
                            return Err(Error::Proof(ProofError::CorruptedProof(
                                "unshield proof contained more than one output-address entry"
                                    .to_string(),
                            )));
                        }
                        balances.insert(address, balance_info);
                    } else {
                        return Err(Error::Proof(ProofError::CorruptedProof(
                            "unshield proof contained an entry outside the nullifier and address subtrees".to_string(),
                        )));
                    }
                }

                // Every nullifier must be present and marked spent.
                for (nf, is_spent) in &statuses {
                    if !is_spent {
                        return Err(Error::Proof(ProofError::IncorrectProof(format!(
                            "nullifier {} was not found as spent in the unshield proof",
                            hex::encode(nf)
                        ))));
                    }
                }

                Ok((
                    root_hash,
                    VerifiedShieldedNullifiersWithAddressInfos(statuses, balances),
                ))
            }
            StateTransition::ShieldedTransfer(st) => {
                use dpp::state_transition::proof_result::StateTransitionProofResult::VerifiedShieldedNullifiers;
                use dpp::state_transition::shielded_transfer_transition::accessors::ShieldedTransferTransitionAccessorsV0;

                let nullifier_keys: Vec<Vec<u8>> = st.nullifiers();

                let (root_hash, statuses) = Drive::verify_shielded_nullifiers(
                    proof,
                    &nullifier_keys,
                    false,
                    platform_version,
                )?;

                // All nullifiers must be marked as spent
                for (nf, is_spent) in &statuses {
                    if !is_spent {
                        return Err(Error::Proof(ProofError::IncorrectProof(format!(
                            "nullifier {} was not found as spent in the proof",
                            hex::encode(nf)
                        ))));
                    }
                }

                Ok((root_hash, VerifiedShieldedNullifiers(statuses)))
            }
            StateTransition::ShieldedWithdrawal(st) => {
                use crate::drive::shielded::paths::shielded_credit_pool_nullifiers_path_vec;
                use dpp::data_contracts::withdrawals_contract;
                use dpp::data_contracts::withdrawals_contract::v1::document_types::withdrawal;
                use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
                use dpp::document::Document;
                use dpp::state_transition::proof_result::StateTransitionProofResult::VerifiedShieldedNullifiersWithWithdrawalDocument;
                use dpp::state_transition::shielded_withdrawal_transition::accessors::ShieldedWithdrawalTransitionAccessorsV0;
                use grovedb::Element;

                let nullifier_keys: Vec<Vec<u8>> = st.nullifiers();

                // Compute withdrawal document ID deterministically (same as prove side).
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

                let contract =
                    known_contracts_provider_fn(&withdrawals_contract::ID)?.ok_or_else(|| {
                        Error::Proof(ProofError::UnknownContract(
                            "withdrawals contract not available for shielded withdrawal verification"
                                .to_string(),
                        ))
                    })?;
                let document_type =
                    contract
                        .document_type_for_name(withdrawal::NAME)
                        .map_err(|e| {
                            Error::Proof(ProofError::UnknownContract(format!(
                                "cannot fetch withdrawal document type: {}",
                                e
                            )))
                        })?;

                let doc_query = SingleDocumentDriveQuery {
                    contract_id: withdrawals_contract::ID.to_buffer(),
                    document_type_name: withdrawal::NAME.to_string(),
                    document_type_keeps_history: false,
                    document_id: document_id.to_buffer(),
                    block_time_ms: None,
                    contested_status: SingleDocumentDriveQueryContestedStatus::NotContested,
                };

                // Reconstruct the prove side's merged query — nullifier spend-status ∪
                // withdrawal document — and verify it strictly. See
                // `verify_merged_query_strict` for why a single strict merged verify is
                // sound and rejects proofs padded with extra subtree branches.
                let mut nf_query = grovedb::Query::new();
                nf_query.insert_keys(nullifier_keys);
                let nullifier_pq = grovedb::PathQuery::new(
                    shielded_credit_pool_nullifiers_path_vec(),
                    grovedb::SizedQuery::new(nf_query, None, None),
                );

                let doc_pq = doc_query.construct_path_query(platform_version)?;
                let document_path = doc_pq.path.clone();

                let (root_hash, proved_key_values) = Self::verify_merged_query_strict(
                    proof,
                    vec![nullifier_pq, doc_pq],
                    platform_version,
                )?;

                // Partition the proved key/values by path: entries under the nullifiers tree are
                // spend statuses; the single entry under the withdrawal-document tree is the
                // proven document.
                let nullifiers_path = shielded_credit_pool_nullifiers_path_vec();

                let mut statuses: Vec<(Vec<u8>, bool)> = Vec::new();
                let mut document_element: Option<Option<Element>> = None;

                for (path, key, element) in proved_key_values {
                    if path == nullifiers_path {
                        statuses.push((key, element.is_some()));
                    } else if path == document_path {
                        if document_element.is_some() {
                            return Err(Error::Proof(ProofError::CorruptedProof(
                                "shielded withdrawal proof contained more than one withdrawal document".to_string(),
                            )));
                        }
                        document_element = Some(element);
                    } else {
                        return Err(Error::Proof(ProofError::CorruptedProof(
                            "shielded withdrawal proof contained an entry outside the nullifier and document subtrees".to_string(),
                        )));
                    }
                }

                // Every nullifier must be present and marked spent.
                for (nf, is_spent) in &statuses {
                    if !is_spent {
                        return Err(Error::Proof(ProofError::IncorrectProof(format!(
                            "nullifier {} was not found as spent in the shielded withdrawal proof",
                            hex::encode(nf)
                        ))));
                    }
                }

                let document_element = document_element.ok_or_else(|| {
                    Error::Proof(ProofError::CorruptedProof(
                        "shielded withdrawal document key absent from proof".to_string(),
                    ))
                })?;

                let doc = match document_element {
                    Some(Element::Item(serialized, _)) => Document::from_bytes(
                        serialized.as_slice(),
                        document_type,
                        platform_version,
                    )?,
                    Some(_) => {
                        return Err(Error::Proof(ProofError::CorruptedProof(
                            "expected an item element for withdrawal document".to_string(),
                        )));
                    }
                    None => {
                        return Err(Error::Proof(ProofError::CorruptedProof(
                            "shielded withdrawal was executed but withdrawal document is missing from proof".to_string(),
                        )));
                    }
                };
                let documents = BTreeMap::from([(document_id, Some(doc))]);

                Ok((
                    root_hash,
                    VerifiedShieldedNullifiersWithWithdrawalDocument(statuses, documents),
                ))
            }
            StateTransition::ShieldFromAssetLock(st) => {
                use crate::drive::RootTree;
                use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
                use dpp::asset_lock::StoredAssetLockInfo;
                use dpp::identity::state_transition::AssetLockProved;
                use dpp::serialization::PlatformDeserializableUntrusted;
                use dpp::state_transition::proof_result::StateTransitionProofResult::{
                    VerifiedAssetLockConsumed, VerifiedAssetLockConsumedWithAddressInfos,
                };
                use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
                use grovedb::Element;

                let outpoint = st.asset_lock_proof().out_point().ok_or_else(|| {
                    Error::Proof(ProofError::InvalidTransition(
                        "shield from asset lock has no outpoint".to_string(),
                    ))
                })?;
                let outpoint_bytes: [u8; 36] = outpoint.into();

                // No accessor trait exposes `surplus_output`, so read it directly off the V0 body.
                let ShieldFromAssetLockTransition::V0(v0) = st;
                let surplus_output = &v0.surplus_output;

                // Build the outpoint sub-query exactly as the prove side does (same path, same key).
                let mut outpoint_query = grovedb::Query::new();
                outpoint_query.insert_key(outpoint_bytes.to_vec());
                let outpoint_pq = grovedb::PathQuery::new(
                    vec![vec![RootTree::SpentAssetLockTransactions as u8]],
                    grovedb::SizedQuery::new(outpoint_query, Some(1), None),
                );

                match surplus_output {
                    Some(surplus_address) => {
                        // Reconstruct the prove side's merged query — asset-lock outpoint ∪
                        // surplus-address balance — and verify it strictly. See
                        // `verify_merged_query_strict` for why a single strict merged verify
                        // is sound and rejects proofs padded with extra subtree branches.
                        let address_pq = Drive::balances_for_clear_addresses_query(
                            std::iter::once(surplus_address),
                        );

                        let (root_hash, proved_key_values) = Self::verify_merged_query_strict(
                            proof,
                            vec![outpoint_pq, address_pq],
                            platform_version,
                        )?;

                        // Partition the proved key/values: exactly one entry is the asset-lock
                        // outpoint (36-byte key), every other entry is a surplus-address balance
                        // (21-byte key). Anything else is a corrupted proof.
                        let outpoint_key = outpoint_bytes.to_vec();
                        let mut outpoint_element: Option<Option<Element>> = None;
                        let mut balances: BTreeMap<
                            PlatformAddress,
                            Option<(AddressNonce, Credits)>,
                        > = BTreeMap::new();

                        for (_path, key, element) in proved_key_values {
                            if key == outpoint_key {
                                outpoint_element = Some(element);
                                continue;
                            }

                            // Mirror `verify_addresses_infos_v0`: reconstruct the address from the
                            // key and decode the `ItemWithSumItem` (nonce, balance) element.
                            let address = PlatformAddress::from_bytes(&key).map_err(|e| {
                                Error::Proof(ProofError::CorruptedProof(format!(
                                    "failed to deserialize surplus PlatformAddress: {}",
                                    e
                                )))
                            })?;

                            let balance_info = element
                                .map(|element| {
                                    let Element::ItemWithSumItem(nonce_vec, balance_i64, _) =
                                        element
                                    else {
                                        return Err(Error::Proof(ProofError::CorruptedProof(
                                            "expected an item with sum item element".to_string(),
                                        )));
                                    };

                                    let nonce_bytes: [u8; 4] =
                                        nonce_vec.try_into().map_err(|_| {
                                            Error::Proof(ProofError::IncorrectValueSize(
                                                "nonce should be 4 bytes",
                                            ))
                                        })?;
                                    let nonce = AddressNonce::from_be_bytes(nonce_bytes);

                                    if balance_i64 < 0 {
                                        return Err(Error::Proof(ProofError::CorruptedProof(
                                            "balance cannot be negative".to_string(),
                                        )));
                                    }
                                    let balance = balance_i64 as Credits;

                                    Ok((nonce, balance))
                                })
                                .transpose()?;

                            balances.insert(address, balance_info);
                        }

                        // The asset-lock outpoint MUST be present in the merged proof.
                        let outpoint_element = outpoint_element.ok_or_else(|| {
                            Error::Proof(ProofError::CorruptedProof(
                                "shield from asset lock was executed but asset lock outpoint is absent from proof".to_string(),
                            ))
                        })?;

                        let info = match outpoint_element {
                            Some(Element::Item(bytes, _)) => {
                                if bytes.is_empty() {
                                    StoredAssetLockInfo::FullyConsumed
                                } else {
                                    StoredAssetLockInfo::PartiallyConsumed(
                                        AssetLockValue::deserialize_from_bytes_untrusted(&bytes)?,
                                    )
                                }
                            }
                            Some(_) => {
                                return Err(Error::Proof(ProofError::CorruptedProof(
                                    "expected an item element for asset lock outpoint".to_string(),
                                )));
                            }
                            None => {
                                return Err(Error::Proof(ProofError::CorruptedProof(
                                    "shield from asset lock was executed but asset lock outpoint is absent from proof".to_string(),
                                )));
                            }
                        };

                        Ok((
                            root_hash,
                            VerifiedAssetLockConsumedWithAddressInfos(info, balances),
                        ))
                    }
                    None => {
                        // No surplus output: the proof covers only the outpoint, verified strictly
                        // with the `Some(1)` limit exactly as before (unchanged behavior).
                        let (root_hash, mut proved_key_values) =
                            grovedb::GroveDb::verify_query_with_absence_proof(
                                proof,
                                &outpoint_pq,
                                &platform_version.drive.grove_version,
                            )?;

                        if proved_key_values.len() > 1 {
                            return Err(Error::Proof(ProofError::TooManyElements(
                                "expected at most 1 element for asset lock outpoint",
                            )));
                        }

                        let info = if let Some(proved) = proved_key_values.pop() {
                            match proved.2 {
                                Some(Element::Item(bytes, _)) => {
                                    if bytes.is_empty() {
                                        StoredAssetLockInfo::FullyConsumed
                                    } else {
                                        StoredAssetLockInfo::PartiallyConsumed(
                                            AssetLockValue::deserialize_from_bytes_untrusted(
                                                &bytes,
                                            )?,
                                        )
                                    }
                                }
                                Some(_) => {
                                    return Err(Error::Proof(ProofError::CorruptedProof(
                                        "expected an item element for asset lock outpoint"
                                            .to_string(),
                                    )));
                                }
                                None => {
                                    return Err(Error::Proof(ProofError::CorruptedProof(
                                        "shield from asset lock was executed but asset lock outpoint is absent from proof".to_string(),
                                    )));
                                }
                            }
                        } else {
                            return Err(Error::Proof(ProofError::CorruptedProof(
                                "shield from asset lock was executed but no proved key values returned"
                                    .to_string(),
                            )));
                        };

                        Ok((root_hash, VerifiedAssetLockConsumed(info)))
                    }
                }
            }
            StateTransition::IdentityCreateFromShieldedPool(st) => {
                use crate::drive::balances::balance_path;
                use crate::drive::identity::IdentityRootStructure::IdentityTreeRevision;
                use crate::drive::identity::{identity_key_tree_path, identity_path};
                use crate::drive::shielded::paths::shielded_credit_pool_nullifiers_path_vec;
                use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
                use dpp::identity::{IdentityPublicKey, IdentityV0, KeyID};
                use dpp::prelude::Revision;
                use dpp::serialization::PlatformDeserializableUntrusted;
                use dpp::state_transition::identity_create_from_shielded_pool_transition::accessors::IdentityCreateFromShieldedPoolTransitionAccessorsV0;
                use dpp::state_transition::identity_create_from_shielded_pool_transition::derive_identity_id_from_actions;
                use dpp::state_transition::proof_result::StateTransitionProofResult::VerifiedIdentityWithShieldedNullifiers;
                use std::collections::{BTreeMap, BTreeSet};

                // Recompute the id from the actions (the canonical value) instead of trusting the
                // wire field, and reject a tampered transition whose wire id doesn't match: so a
                // client verifying a proof cannot be fed a transition that reuses these nullifiers
                // while pointing `identity_id` at a different identity. (Consensus enforces the same
                // equality in `validate_structure`; this independently re-checks it here so the
                // SDK proof path is sound even on a hand-constructed transition object.)
                let derived_id = derive_identity_id_from_actions(st.actions());
                if st.identity_id() != derived_id {
                    return Err(Error::Proof(ProofError::IncorrectProof(
                        "identity create from shielded pool: identity_id does not match the value derived from the spend nullifiers".to_string(),
                    )));
                }
                let identity_id = derived_id.to_buffer();
                let nullifier_keys: Vec<Vec<u8>> = st.nullifiers();

                // Rebuild the BYTE-IDENTICAL merged query the prove side built: the nullifier
                // sub-query over the nullifier tree + the full-identity sub-query, each with its
                // limit cleared (PathQuery::merge rejects limited sub-queries).
                let mut nf_query = grovedb::Query::new();
                nf_query.insert_keys(nullifier_keys.clone());
                let nullifier_pq = grovedb::PathQuery::new(
                    shielded_credit_pool_nullifiers_path_vec(),
                    grovedb::SizedQuery::new(nf_query, None, None),
                );

                let mut identity_pq = Drive::full_identity_query(
                    &identity_id,
                    &platform_version.drive.grove_version,
                )?;
                identity_pq.query.limit = None;

                let merged_pq = grovedb::PathQuery::merge(
                    vec![&nullifier_pq, &identity_pq],
                    &platform_version.drive.grove_version,
                )?;

                // STRICT verification via `verify_query` (succinctness on). Unlike the other
                // shielded merged queries (which target only explicit keys and go through
                // `verify_merged_query_strict`), this one embeds `full_identity_query`, whose
                // all-keys sub-query is an unbounded RangeFull: and
                // `verify_query_with_absence_proof` enumerates the query's terminal keys, which
                // is impossible for unbounded ranges ("terminal keys are not supported with
                // unbounded ranges"). Absence synthesis isn't needed here anyway: every queried
                // element (the spent nullifiers and the created identity) must be PRESENT, so
                // presence is checked directly against the result set below. The succinctness
                // check still rejects proofs padded with branches beyond {nullifiers, identity}
                // (the strict-from-day-one guarantee of #3812), and the limit stays None exactly
                // as the prove side built it, so no layer's result loop can break early.
                let (root_hash, proved_key_values) = grovedb::GroveDb::verify_query(
                    proof,
                    &merged_pq,
                    &platform_version.drive.grove_version,
                )?;

                // Partition the proved key/values by PATH (NOT key length: nullifier keys and the
                // identity id are both 32 bytes): nullifier-tree entries vs the identity subtrees
                // (balance / revision / keys). Reconstruct the identity exactly as
                // `verify_full_identity_by_identity_id_v0` does.
                let nullifier_path = shielded_credit_pool_nullifiers_path_vec();
                let balance_path = balance_path();
                let identity_path = identity_path(identity_id.as_slice());
                let identity_keys_path = identity_key_tree_path(identity_id.as_slice());

                let mut spent_nullifiers = BTreeSet::<Vec<u8>>::new();
                let mut balance: Option<Credits> = None;
                let mut revision: Option<Revision> = None;
                let mut keys = BTreeMap::<KeyID, IdentityPublicKey>::new();

                for (path, key, maybe_element) in proved_key_values {
                    if path == nullifier_path {
                        if !nullifier_keys.contains(&key) {
                            return Err(Error::Proof(ProofError::CorruptedProof(
                                "identity create from shielded pool proof contains a nullifier \
                                 entry that was not requested"
                                    .to_string(),
                            )));
                        }
                        if maybe_element.is_some() {
                            spent_nullifiers.insert(key);
                        }
                    } else if path == balance_path && key == identity_id {
                        let element = maybe_element.ok_or_else(|| {
                            Error::Proof(ProofError::IncompleteProof(
                                "balance wasn't provided for the created identity",
                            ))
                        })?;
                        let signed_balance = element.as_sum_item_value().map_err(Error::from)?;
                        if signed_balance < 0 {
                            return Err(Error::Proof(ProofError::Overflow(
                                "balance can't be negative",
                            )));
                        }
                        balance = Some(signed_balance as Credits);
                    } else if path == identity_path && key == vec![IdentityTreeRevision as u8] {
                        let element = maybe_element.ok_or_else(|| {
                            Error::Proof(ProofError::IncompleteProof(
                                "revision wasn't provided for the created identity",
                            ))
                        })?;
                        let item_bytes = element.into_item_bytes().map_err(Error::from)?;
                        revision = Some(Revision::from_be_bytes(item_bytes.try_into().map_err(
                            |_| {
                                Error::Proof(ProofError::IncorrectValueSize(
                                    "revision should be 8 bytes",
                                ))
                            },
                        )?));
                    } else if path == identity_keys_path {
                        let element = maybe_element.ok_or_else(|| {
                            Error::Proof(ProofError::CorruptedProof(
                                "received an absence proof for a key but didn't request one"
                                    .to_string(),
                            ))
                        })?;
                        let item_bytes = element.into_item_bytes().map_err(Error::from)?;
                        let public_key =
                            IdentityPublicKey::deserialize_from_bytes_untrusted(&item_bytes)?;
                        keys.insert(public_key.id(), public_key);
                    } else {
                        return Err(Error::Proof(ProofError::TooManyElements(
                            "identity create from shielded pool proof contains an element outside \
                             the nullifier tree and the created identity",
                        )));
                    }
                }

                // Without absence synthesis an unspent nullifier yields no result entry (or a
                // bare absence entry), so each expected nullifier's spend status is its
                // membership in the proved-present set.
                let statuses: Vec<(Vec<u8>, bool)> = nullifier_keys
                    .iter()
                    .map(|nf| (nf.clone(), spent_nullifiers.contains(nf)))
                    .collect();

                // Every funding nullifier must be present (spent) in the post-execution state.
                for (nf, is_spent) in &statuses {
                    if !is_spent {
                        return Err(Error::Proof(ProofError::IncorrectProof(format!(
                            "nullifier {} was not found as spent in the identity-create-from-shielded-pool proof",
                            hex::encode(nf)
                        ))));
                    }
                }

                // The created identity MUST be fully present.
                let (balance, revision) = match (balance, revision, keys.is_empty()) {
                    (Some(balance), Some(revision), false) => (balance, revision),
                    _ => {
                        return Err(Error::Proof(ProofError::IncompleteProof(
                            "identity create from shielded pool was executed but the created identity is absent or incomplete in the proof",
                        )))
                    }
                };

                // Bind the proof to the transition's declared key set: the proven identity must hold
                // EXACTLY the keys the transition created (the same conversion the action transformer
                // used to build the identity). This stops a tampered transition from swapping in a
                // different key set while reusing a valid {nullifiers, identity} proof.
                //
                // The balance is deliberately NOT checked against `denomination`: the identity holds
                // `denomination - total_fee`, and `total_fee` is metered at execution and not
                // recoverable here, so a balance/denomination equality check would reject every
                // honest proof. (`denomination` is bound into the Orchard `extra_sighash_data` at
                // consensus, which is where that binding is enforced.)
                let expected_keys: BTreeMap<KeyID, IdentityPublicKey> = st
                    .public_keys()
                    .iter()
                    .map(|key| {
                        let public_key: IdentityPublicKey = key.into();
                        (public_key.id(), public_key)
                    })
                    .collect();
                if keys != expected_keys {
                    return Err(Error::Proof(ProofError::IncorrectProof(
                        "identity create from shielded pool: the proven identity's keys do not match the transition's declared public keys".to_string(),
                    )));
                }

                let identity: dpp::prelude::Identity = IdentityV0 {
                    id: Identifier::from(identity_id),
                    public_keys: keys,
                    balance,
                    revision,
                }
                .into();

                Ok((
                    root_hash,
                    VerifiedIdentityWithShieldedNullifiers(identity, statuses),
                ))
            }
            StateTransition::IdentityTopUpFromShieldedPool(st) => {
                use crate::drive::balances::balance_path;
                use crate::drive::identity::IdentityRootStructure::IdentityTreeRevision;
                use crate::drive::identity::{identity_key_tree_path, identity_path};
                use crate::drive::shielded::paths::shielded_credit_pool_nullifiers_path_vec;
                use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
                use dpp::identity::{IdentityPublicKey, IdentityV0, KeyID};
                use dpp::prelude::Revision;
                use dpp::serialization::PlatformDeserializableUntrusted;
                use dpp::state_transition::identity_top_up_from_shielded_pool_transition::accessors::IdentityTopUpFromShieldedPoolTransitionAccessorsV0;
                use dpp::state_transition::proof_result::StateTransitionProofResult::VerifiedIdentityWithShieldedNullifiers;
                use std::collections::{BTreeMap, BTreeSet};

                // The credited identity is the transition's declared `identity_id`; consensus
                // binds it into the Orchard sighash, so a proof for these nullifiers can only
                // have been produced for a transition crediting this identity.
                let identity_id = st.identity_id().to_buffer();
                let nullifier_keys: Vec<Vec<u8>> = st.nullifiers();

                // Rebuild the BYTE-IDENTICAL merged query the prove side built: the nullifier
                // sub-query over the nullifier tree + the full-identity sub-query, each with its
                // limit cleared (PathQuery::merge rejects limited sub-queries).
                let mut nf_query = grovedb::Query::new();
                nf_query.insert_keys(nullifier_keys.clone());
                let nullifier_pq = grovedb::PathQuery::new(
                    shielded_credit_pool_nullifiers_path_vec(),
                    grovedb::SizedQuery::new(nf_query, None, None),
                );

                let mut identity_pq = Drive::full_identity_query(
                    &identity_id,
                    &platform_version.drive.grove_version,
                )?;
                identity_pq.query.limit = None;

                let merged_pq = grovedb::PathQuery::merge(
                    vec![&nullifier_pq, &identity_pq],
                    &platform_version.drive.grove_version,
                )?;

                // STRICT verification via `verify_query` (succinctness on). Unlike the other
                // shielded merged queries (which target only explicit keys and go through
                // `verify_merged_query_strict`), this one embeds `full_identity_query`, whose
                // all-keys sub-query is an unbounded RangeFull: and
                // `verify_query_with_absence_proof` enumerates the query's terminal keys, which
                // is impossible for unbounded ranges ("terminal keys are not supported with
                // unbounded ranges"). Absence synthesis isn't needed here anyway: every queried
                // element (the spent nullifiers and the credited identity) must be PRESENT, so
                // presence is checked directly against the result set below. The succinctness
                // check still rejects proofs padded with branches beyond {nullifiers, identity}
                // (the strict-from-day-one guarantee of #3812), and the limit stays None exactly
                // as the prove side built it, so no layer's result loop can break early.
                let (root_hash, proved_key_values) = grovedb::GroveDb::verify_query(
                    proof,
                    &merged_pq,
                    &platform_version.drive.grove_version,
                )?;

                // Partition the proved key/values by PATH (NOT key length: nullifier keys and the
                // identity id are both 32 bytes): nullifier-tree entries vs the identity subtrees
                // (balance / revision / keys). Reconstruct the identity exactly as
                // `verify_full_identity_by_identity_id_v0` does.
                let nullifier_path = shielded_credit_pool_nullifiers_path_vec();
                let balance_path = balance_path();
                let identity_path = identity_path(identity_id.as_slice());
                let identity_keys_path = identity_key_tree_path(identity_id.as_slice());

                let mut spent_nullifiers = BTreeSet::<Vec<u8>>::new();
                let mut balance: Option<Credits> = None;
                let mut revision: Option<Revision> = None;
                let mut keys = BTreeMap::<KeyID, IdentityPublicKey>::new();

                for (path, key, maybe_element) in proved_key_values {
                    if path == nullifier_path {
                        if !nullifier_keys.contains(&key) {
                            return Err(Error::Proof(ProofError::CorruptedProof(
                                "identity top up from shielded pool proof contains a nullifier \
                                 entry that was not requested"
                                    .to_string(),
                            )));
                        }
                        if maybe_element.is_some() {
                            spent_nullifiers.insert(key);
                        }
                    } else if path == balance_path && key == identity_id {
                        let element = maybe_element.ok_or_else(|| {
                            Error::Proof(ProofError::IncompleteProof(
                                "balance wasn't provided for the topped-up identity",
                            ))
                        })?;
                        let signed_balance = element.as_sum_item_value().map_err(Error::from)?;
                        if signed_balance < 0 {
                            return Err(Error::Proof(ProofError::Overflow(
                                "balance can't be negative",
                            )));
                        }
                        balance = Some(signed_balance as Credits);
                    } else if path == identity_path && key == vec![IdentityTreeRevision as u8] {
                        let element = maybe_element.ok_or_else(|| {
                            Error::Proof(ProofError::IncompleteProof(
                                "revision wasn't provided for the topped-up identity",
                            ))
                        })?;
                        let item_bytes = element.into_item_bytes().map_err(Error::from)?;
                        revision = Some(Revision::from_be_bytes(item_bytes.try_into().map_err(
                            |_| {
                                Error::Proof(ProofError::IncorrectValueSize(
                                    "revision should be 8 bytes",
                                ))
                            },
                        )?));
                    } else if path == identity_keys_path {
                        let element = maybe_element.ok_or_else(|| {
                            Error::Proof(ProofError::CorruptedProof(
                                "received an absence proof for a key but didn't request one"
                                    .to_string(),
                            ))
                        })?;
                        let item_bytes = element.into_item_bytes().map_err(Error::from)?;
                        let public_key =
                            IdentityPublicKey::deserialize_from_bytes_untrusted(&item_bytes)?;
                        keys.insert(public_key.id(), public_key);
                    } else {
                        return Err(Error::Proof(ProofError::TooManyElements(
                            "identity top up from shielded pool proof contains an element outside \
                             the nullifier tree and the topped-up identity",
                        )));
                    }
                }

                // Without absence synthesis an unspent nullifier yields no result entry (or a
                // bare absence entry), so each expected nullifier's spend status is its
                // membership in the proved-present set.
                let statuses: Vec<(Vec<u8>, bool)> = nullifier_keys
                    .iter()
                    .map(|nf| (nf.clone(), spent_nullifiers.contains(nf)))
                    .collect();

                // Every funding nullifier must be present (spent) in the post-execution state.
                for (nf, is_spent) in &statuses {
                    if !is_spent {
                        return Err(Error::Proof(ProofError::IncorrectProof(format!(
                            "nullifier {} was not found as spent in the identity-top-up-from-shielded-pool proof",
                            hex::encode(nf)
                        ))));
                    }
                }

                // The credited identity MUST be fully present (it existed before the top-up).
                let (balance, revision) = match (balance, revision, keys.is_empty()) {
                    (Some(balance), Some(revision), false) => (balance, revision),
                    _ => {
                        return Err(Error::Proof(ProofError::IncompleteProof(
                            "identity top up from shielded pool was executed but the topped-up identity is absent or incomplete in the proof",
                        )))
                    }
                };

                // The balance is deliberately NOT checked against `top_up_amount`: the proof is a
                // post-execution snapshot of an identity that already held credits, so the
                // pre-top-up balance and the flat fee are not recoverable here. (`top_up_amount`
                // is bound into the Orchard `extra_sighash_data` at consensus.)
                let identity: dpp::prelude::Identity = IdentityV0 {
                    id: Identifier::from(identity_id),
                    public_keys: keys,
                    balance,
                    revision,
                }
                .into();

                Ok((
                    root_hash,
                    VerifiedIdentityWithShieldedNullifiers(identity, statuses),
                ))
            }
            StateTransition::ShieldFromIdentity(st) => {
                use dpp::state_transition::shield_from_identity_transition::accessors::ShieldFromIdentityTransitionAccessorsV0;
                // snapshot of the identity's balance at the proof's block
                let identity_id = st.identity_id();
                let (root_hash, balance) = Drive::verify_identity_balance_for_identity_id(
                    proof,
                    identity_id.into_buffer(),
                    false,
                    platform_version,
                )?;
                let balance = balance.ok_or(Error::Proof(ProofError::IncorrectProof(format!(
                    "proof did not contain balance for identity {} expected to exist because of state transition (shield from identity)",
                    identity_id
                ))))?;
                Ok((
                    root_hash,
                    VerifiedPartialIdentity(PartialIdentity {
                        id: identity_id,
                        loaded_public_keys: Default::default(),
                        balance: Some(balance),
                        revision: None,
                        not_found_public_keys: Default::default(),
                    }),
                ))
            }
        }?;

        let outcome = if Self::state_transition_proof_binds_execution(
            state_transition,
            known_contracts_provider_fn,
        )? {
            StateTransitionProofOutcome::ExecutionProved(result)
        } else {
            StateTransitionProofOutcome::AffectedState(result)
        };

        Ok((root_hash, outcome))
    }
}
