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
    #[inline(always)]
    pub(super) fn verify_state_transition_was_executed_with_proof_v0(
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
                let keeps_history = data_contract_update
                    .data_contract()
                    .config()
                    .keeps_history();
                let (root_hash, contract) = Drive::verify_contract(
                    proof,
                    Some(keeps_history),
                    false,
                    true,
                    data_contract_update.data_contract().id().into_buffer(),
                    platform_version,
                )?;
                let contract = contract.ok_or(Error::Proof(ProofError::IncorrectProof(format!("proof did not contain contract with id {} expected to exist because of state transition (update", data_contract_update.data_contract().id()))))?;
                let contract_for_serialization: DataContractInSerializationFormat = contract
                    .clone()
                    .try_into_platform_versioned(platform_version)?;
                if let Some(mismatch) =
                    contract_for_serialization.first_mismatch(data_contract_update.data_contract())
                {
                    return Err(Error::Proof(ProofError::IncorrectProof(format!("proof of state transition execution did not contain exact expected contract after update with id {}: {}", data_contract_update.data_contract().id(), mismatch))));
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
                use dpp::serialization::PlatformDeserializable;
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
                                        AssetLockValue::deserialize_from_bytes(&bytes)?,
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
                                            AssetLockValue::deserialize_from_bytes(&bytes)?,
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
                use dpp::serialization::PlatformDeserializable;
                use dpp::state_transition::identity_create_from_shielded_pool_transition::accessors::IdentityCreateFromShieldedPoolTransitionAccessorsV0;
                use dpp::state_transition::identity_create_from_shielded_pool_transition::derive_identity_id_from_actions;
                use dpp::state_transition::proof_result::StateTransitionProofResult::VerifiedIdentityWithShieldedNullifiers;
                use std::collections::{BTreeMap, BTreeSet};

                // Recompute the id from the actions (the canonical value) instead of trusting the
                // wire field, and reject a tampered transition whose wire id doesn't match — so a
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
                // all-keys sub-query is an unbounded RangeFull — and
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

                // Partition the proved key/values by PATH (NOT key length — nullifier keys and the
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
                        let public_key = IdentityPublicKey::deserialize_from_bytes(&item_bytes)?;
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

    /// Whether a valid proof for this state transition binds the execution of
    /// this specific transition (`ExecutionProved`) or can only authenticate
    /// the state the transition affects at the committed block
    /// (`AffectedState`).
    ///
    /// This classifier is deliberately an exhaustive `match` over every
    /// transition family — and, for tokens, over the contract's keeps-history
    /// configuration and group usage — so adding a new variant fails to
    /// compile until it is classified here: no transition can silently
    /// inherit the stronger `ExecutionProved` guarantee.
    fn state_transition_proof_binds_execution(
        state_transition: &StateTransition,
        known_contracts_provider_fn: &ContractLookupFn,
    ) -> Result<bool, Error> {
        let binds = match state_transition {
            // The created contract's id derives from its owner and entropy,
            // and the proven body must equal the transition's declared
            // contract, which cannot exist without this create executing.
            StateTransition::DataContractCreate(_) => true,
            // The proven contract body equaling the update's target does not
            // prove that THIS update executed: the proven version may predate
            // the request, whose signature and nonce are not in the proof.
            StateTransition::DataContractUpdate(_) => false,
            StateTransition::Batch(batch) => match batch.first_transition() {
                // The verifier errors on an empty batch before the tag is
                // consulted; classify fail-closed regardless.
                None => false,
                // Document proofs bind the exact document (or its absence
                // after deletion), including contested status and history —
                // EXCEPT indexOnly types: their snapshot authenticates the
                // commitment-checked entry (or its absence), which carries
                // neither id, entropy nor nonce and so cannot bind one
                // specific transition's execution.
                Some(BatchedTransitionRef::Document(document_transition)) => {
                    use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
                    let data_contract_id = document_transition.data_contract_id();
                    let contract = known_contracts_provider_fn(&data_contract_id)?.ok_or(
                        Error::Proof(ProofError::UnknownContract(format!(
                            "unknown contract with id {} in document verification",
                            data_contract_id
                        ))),
                    )?;
                    !contract
                        .document_type_for_name(document_transition.document_type_name())
                        .map_err(|e| Error::Proof(ProofError::UnknownContract(e.to_string())))?
                        .index_only()
                }
                Some(BatchedTransitionRef::Token(token_transition)) => {
                    let data_contract_id = token_transition.data_contract_id();
                    let contract = known_contracts_provider_fn(&data_contract_id)?.ok_or(
                        Error::Proof(ProofError::UnknownContract(format!(
                            "unknown contract with id {} in token verification",
                            data_contract_id
                        ))),
                    )?;
                    let token_config = contract.expected_token_configuration(
                        token_transition.base().token_contract_position(),
                    )?;
                    let keeps = token_config.keeps_history();
                    // A group action's proof binds the action id and the
                    // signer's recorded participation; a history-keeping
                    // token's proof binds the exact historical document.
                    // Without either, only the resulting state is proven.
                    let grouped = token_transition.base().using_group_info().is_some();
                    match token_transition {
                        TokenTransition::Burn(_) => keeps.keeps_burning_history() || grouped,
                        TokenTransition::Mint(_) => keeps.keeps_minting_history() || grouped,
                        TokenTransition::Transfer(_) => keeps.keeps_transfer_history(),
                        TokenTransition::Freeze(_) | TokenTransition::Unfreeze(_) => {
                            keeps.keeps_freezing_history() || grouped
                        }
                        TokenTransition::DirectPurchase(_) => keeps.keeps_direct_purchase_history(),
                        TokenTransition::SetPriceForDirectPurchase(_) => {
                            keeps.keeps_direct_pricing_history() || grouped
                        }
                        TokenTransition::DestroyFrozenFunds(_)
                        | TokenTransition::EmergencyAction(_)
                        | TokenTransition::ConfigUpdate(_)
                        | TokenTransition::Claim(_) => true,
                    }
                }
            },
            // The proven identity holds exactly the transition's declared
            // key set, which cannot exist without this create executing.
            StateTransition::IdentityCreate(_) => true,
            // Balance/revision snapshots: values as of the proof's block,
            // not bindable to one specific top-up/withdrawal/transfer.
            StateTransition::IdentityTopUp(_) => false,
            StateTransition::IdentityCreditWithdrawal(_) => false,
            StateTransition::IdentityCreditTransfer(_) => false,
            // Binds the transition's revision and its exact key additions
            // and disabling timestamps.
            StateTransition::IdentityUpdate(_) => true,
            // The proven vote is stored under the masternode's identity and
            // must equal the transition's declared vote.
            StateTransition::MasternodeVote(_) => true,
            // Current identity and address balances, without the requested
            // amounts, nonces, or the submitted request itself.
            StateTransition::IdentityCreditTransferToAddresses(_) => false,
            StateTransition::IdentityTopUpFromAddresses(_) => false,
            // Binds the created identity's declared public keys.
            StateTransition::IdentityCreateFromAddresses(_) => true,
            // Address funds families: post-state address balances only.
            StateTransition::AddressFundsTransfer(_) => false,
            StateTransition::AddressFundingFromAssetLock(_) => false,
            StateTransition::AddressCreditWithdrawal(_) => false,
            StateTransition::Shield(_) => false,
            // Nullifier-spend proofs bind the exact Orchard actions of this
            // transition.
            StateTransition::Unshield(_) => true,
            StateTransition::ShieldedTransfer(_) => true,
            StateTransition::ShieldedWithdrawal(_) => true,
            // Only the consumed outpoint (and current surplus-address state)
            // is proven; competing shields over the same outpoint share the
            // same query and result.
            StateTransition::ShieldFromAssetLock(_) => false,
            // Spent nullifiers plus the resulting identity do not bind the
            // complete Orchard request, denomination context, or fallback
            // address.
            StateTransition::IdentityCreateFromShieldedPool(_) => false,
        };

        Ok(binds)
    }

    /// Reconstruct the prove side's merged multi-root query and verify it STRICTLY.
    ///
    /// The shielded prove paths merge several sub-queries into a SINGLE multi-root
    /// proof. [`grovedb::PathQuery::merge`] rejects sub-queries that still carry a
    /// limit, so every sub-query's limit is cleared here before the merge. The strict
    /// [`grovedb::GroveDb::verify_query_with_absence_proof`] in turn *requires* a
    /// limit, but it must never be exhausted by the legitimate result set: the
    /// per-layer succinctness check that rejects extra proof branches runs AFTER that
    /// layer's result loop, and the result loop only breaks early once the limit hits
    /// 0 — so a limit smaller than the real result count could break before a layer's
    /// succinctness check runs and falsely reject an honest proof. Every shielded
    /// merged query targets a fixed, tiny set of explicit keys ({nullifiers} plus one
    /// address/document/outpoint), so an unreachable `u16::MAX` limit is sound: it
    /// guarantees every layer is fully traversed while the (limit-independent)
    /// succinctness check still rejects any proof padded with extra subtree branches.
    ///
    /// Returns the proof root hash and every proved `(path, key, element)` trio, left
    /// for the caller to partition against the sub-query paths.
    #[allow(clippy::type_complexity)]
    fn verify_merged_query_strict(
        proof: &[u8],
        mut sub_queries: Vec<grovedb::PathQuery>,
        platform_version: &PlatformVersion,
    ) -> Result<
        (
            RootHash,
            Vec<grovedb::query_result_type::PathKeyOptionalElementTrio>,
        ),
        Error,
    > {
        for sub_query in &mut sub_queries {
            sub_query.query.limit = None;
        }

        let mut merged_pq = grovedb::PathQuery::merge(
            sub_queries.iter().collect(),
            &platform_version.drive.grove_version,
        )?;
        merged_pq.query.limit = Some(u16::MAX);

        Ok(grovedb::GroveDb::verify_query_with_absence_proof(
            proof,
            &merged_pq,
            &platform_version.drive.grove_version,
        )?)
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::Drive;
    use crate::query::ContractLookupFn;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
    use dpp::document::DocumentV0Getters;
    use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
    use dpp::identity::Identity;
    use dpp::prelude::DataContract;
    use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
    use dpp::state_transition::proof_result::{
        StateTransitionProofOutcome, StateTransitionProofResult,
    };
    use dpp::state_transition::StateTransition;
    use dpp::tests::fixtures::get_dpns_data_contract_fixture;
    use dpp::version::PlatformVersion;
    use platform_version::TryIntoPlatformVersioned;
    use std::sync::Arc;

    /// Helper: set up drive, insert a DPNS contract, return (drive, contract)
    fn setup_drive_and_contract() -> (Drive, DataContract) {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let created_contract =
            get_dpns_data_contract_fixture(None, 0, platform_version.protocol_version);
        let contract = created_contract.data_contract_owned();
        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");
        (drive, contract)
    }

    /// Helper: set up drive and add an identity, return (drive, identity)
    fn setup_drive_and_identity() -> (Drive, Identity) {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let identity = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add identity");
        (drive, identity)
    }

    // -----------------------------------------------------------------------
    // DataContractCreate
    // -----------------------------------------------------------------------

    #[test]
    fn verify_data_contract_create_happy_path() {
        let (drive, contract) = setup_drive_and_contract();
        let platform_version = PlatformVersion::latest();
        let contract_id = contract.id().to_buffer();

        // Generate a proof for this contract
        let proof = drive
            .prove_contract(contract_id, None, platform_version)
            .expect("expected to prove contract");

        // Build the DataContractCreate state transition from the contract
        let data_contract_serialized: DataContractInSerializationFormat = contract
            .clone()
            .try_into_platform_versioned(platform_version)
            .expect("expected to serialize contract");

        use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
        use dpp::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
        let st = StateTransition::DataContractCreate(DataContractCreateTransition::V0(
            DataContractCreateTransitionV0 {
                data_contract: data_contract_serialized,
                identity_nonce: 0,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &proof,
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            result.is_ok(),
            "expected verification to succeed, got: {:?}",
            result.err()
        );
        let (root_hash, proof_result) = result.unwrap();
        assert_ne!(root_hash, [0u8; 32], "root hash should not be all zeros");
        match proof_result {
            StateTransitionProofOutcome::ExecutionProved(
                StateTransitionProofResult::VerifiedDataContract(verified_contract),
            ) => {
                assert_eq!(
                    verified_contract.id(),
                    contract.id(),
                    "verified contract id should match"
                );
            }
            other => panic!("expected VerifiedDataContract, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // DataContractUpdate
    // -----------------------------------------------------------------------

    #[test]
    fn verify_data_contract_update_happy_path() {
        let (drive, contract) = setup_drive_and_contract();
        let platform_version = PlatformVersion::latest();
        let contract_id = contract.id().to_buffer();

        // For the update transition, we use the same contract (version hasn't changed
        // in the fixture, but the proof verifies the contract is as expected).
        let proof = drive
            .prove_contract(contract_id, None, platform_version)
            .expect("expected to prove contract");

        let data_contract_serialized: DataContractInSerializationFormat = contract
            .clone()
            .try_into_platform_versioned(platform_version)
            .expect("expected to serialize contract");

        use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0;
        let st = StateTransition::DataContractUpdate(DataContractUpdateTransition::V0(
            DataContractUpdateTransitionV0 {
                identity_contract_nonce: 0,
                data_contract: data_contract_serialized,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &proof,
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            result.is_ok(),
            "expected verification to succeed, got: {:?}",
            result.err()
        );
        let (_root_hash, proof_result) = result.unwrap();
        // The proven contract body equaling the update's target does not
        // prove that THIS update executed (the version may predate the
        // request), so the outcome is a snapshot, not execution evidence.
        match proof_result {
            StateTransitionProofOutcome::AffectedState(
                StateTransitionProofResult::VerifiedDataContract(verified_contract),
            ) => {
                assert_eq!(verified_contract.id(), contract.id());
            }
            other => panic!(
                "expected AffectedState(VerifiedDataContract), got {:?}",
                other
            ),
        }
    }

    // -----------------------------------------------------------------------
    // IdentityCreate
    // -----------------------------------------------------------------------

    #[test]
    fn verify_identity_create_happy_path() {
        let (drive, identity) = setup_drive_and_identity();
        let platform_version = PlatformVersion::latest();
        let identity_id = identity.id().to_buffer();

        let proof = drive
            .prove_full_identity(identity_id, None, &platform_version.drive)
            .expect("expected to prove full identity");

        use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
        use dpp::state_transition::state_transitions::identity::identity_create_transition::v0::IdentityCreateTransitionV0;
        let st = StateTransition::IdentityCreate(IdentityCreateTransition::V0(
            IdentityCreateTransitionV0 {
                identity_id: identity.id(),
                public_keys: identity.public_keys().values().map(Into::into).collect(),
                ..Default::default()
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &proof,
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            result.is_ok(),
            "expected verification to succeed, got: {:?}",
            result.err()
        );
        let (_root_hash, proof_result) = result.unwrap();
        match proof_result {
            StateTransitionProofOutcome::ExecutionProved(
                StateTransitionProofResult::VerifiedIdentity(verified_identity),
            ) => {
                assert_eq!(verified_identity.id(), identity.id());
            }
            other => panic!("expected VerifiedIdentity, got {:?}", other),
        }
    }

    #[test]
    fn verify_identity_create_rejects_unrelated_public_keys() {
        let (drive, identity) = setup_drive_and_identity();
        let platform_version = PlatformVersion::latest();
        let proof = drive
            .prove_full_identity(identity.id().to_buffer(), None, &platform_version.drive)
            .expect("expected to prove full identity");

        use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
        use dpp::state_transition::state_transitions::identity::identity_create_transition::v0::IdentityCreateTransitionV0;
        let st = StateTransition::IdentityCreate(IdentityCreateTransition::V0(
            IdentityCreateTransitionV0 {
                identity_id: identity.id(),
                public_keys: vec![],
                ..Default::default()
            },
        ));

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &proof,
            &|_id| Ok(None),
            platform_version,
        );

        assert!(matches!(
            result,
            Err(Error::Proof(ProofError::IncorrectProof(_)))
        ));
    }

    // -----------------------------------------------------------------------
    // IdentityTopUp
    // -----------------------------------------------------------------------

    #[test]
    fn verify_identity_top_up_returns_affected_state_snapshot() {
        let (drive, identity) = setup_drive_and_identity();
        let platform_version = PlatformVersion::latest();
        let identity_id = identity.id().to_buffer();

        let proof = drive
            .prove_identity_balance_and_revision(identity_id, None, &platform_version.drive)
            .expect("expected to prove identity balance and revision");

        use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
        use dpp::state_transition::state_transitions::identity::identity_topup_transition::v0::IdentityTopUpTransitionV0;
        let st = StateTransition::IdentityTopUp(IdentityTopUpTransition::V0(
            IdentityTopUpTransitionV0 {
                identity_id: identity.id(),
                ..Default::default()
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        // A top-up proof cannot be bound to the transition's execution, so
        // the outcome must be tagged as an affected-state snapshot, never as
        // execution evidence.
        let (_root_hash, outcome) = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &proof,
            known_contracts_provider_fn,
            platform_version,
        )
        .expect("expected verification to succeed");

        match outcome {
            StateTransitionProofOutcome::AffectedState(
                StateTransitionProofResult::VerifiedPartialIdentity(partial_identity),
            ) => {
                assert_eq!(partial_identity.id, identity.id());
                assert!(partial_identity.balance.is_some());
                assert!(partial_identity.revision.is_some());
            }
            other => panic!(
                "expected AffectedState(VerifiedPartialIdentity), got {:?}",
                other
            ),
        }
    }

    // -----------------------------------------------------------------------
    // IdentityCreditWithdrawal
    // -----------------------------------------------------------------------

    #[test]
    fn verify_identity_credit_withdrawal_returns_affected_state_snapshot() {
        let (drive, identity) = setup_drive_and_identity();
        let platform_version = PlatformVersion::latest();
        let identity_id = identity.id().to_buffer();

        let proof = drive
            .prove_identity_balance(identity_id, None, &platform_version.drive)
            .expect("expected to prove identity balance");

        use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
        use dpp::state_transition::state_transitions::identity::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
        let st = StateTransition::IdentityCreditWithdrawal(IdentityCreditWithdrawalTransition::V0(
            IdentityCreditWithdrawalTransitionV0 {
                identity_id: identity.id(),
                amount: 100,
                ..Default::default()
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let (_root_hash, outcome) = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &proof,
            known_contracts_provider_fn,
            platform_version,
        )
        .expect("expected verification to succeed");

        match outcome {
            StateTransitionProofOutcome::AffectedState(
                StateTransitionProofResult::VerifiedPartialIdentity(partial_identity),
            ) => {
                assert_eq!(partial_identity.id, identity.id());
                assert!(partial_identity.balance.is_some());
            }
            other => panic!(
                "expected AffectedState(VerifiedPartialIdentity), got {:?}",
                other
            ),
        }
    }

    // -----------------------------------------------------------------------
    // IdentityUpdate
    // -----------------------------------------------------------------------

    #[test]
    fn verify_identity_update_happy_path() {
        let (drive, identity) = setup_drive_and_identity();
        let platform_version = PlatformVersion::latest();
        let identity_id = identity.id().to_buffer();

        // The verify function for IdentityUpdate uses verify_identity_keys_by_identity_id
        // with with_revision=true, which creates a merged query of keys + revision.
        // We need a proof that contains both. Use the same path queries merged together.
        use crate::drive::identity::key::fetch::IdentityKeysRequest;
        let key_request = IdentityKeysRequest::new_all_keys_query(&identity_id, None);
        let keys_path_query = key_request.into_path_query();
        let revision_path_query = Drive::identity_revision_query(&identity_id);
        let merged = grovedb::PathQuery::merge(
            vec![&keys_path_query, &revision_path_query],
            &platform_version.drive.grove_version,
        )
        .expect("expected to merge path queries");
        let proof = drive
            .grove_get_proved_path_query(&merged, None, &mut vec![], &platform_version.drive)
            .expect("expected to prove identity keys and revision");

        use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
        use dpp::state_transition::state_transitions::identity::identity_update_transition::v0::IdentityUpdateTransitionV0;
        let st = StateTransition::IdentityUpdate(IdentityUpdateTransition::V0(
            IdentityUpdateTransitionV0 {
                identity_id: identity.id(),
                revision: identity.revision(),
                ..Default::default()
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &proof,
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            result.is_ok(),
            "expected verification to succeed, got: {:?}",
            result.err()
        );
        let (_root_hash, proof_result) = result.unwrap();
        match proof_result {
            StateTransitionProofOutcome::ExecutionProved(
                StateTransitionProofResult::VerifiedPartialIdentity(partial_identity),
            ) => {
                assert_eq!(partial_identity.id, identity.id());
                assert!(
                    !partial_identity.loaded_public_keys.is_empty(),
                    "loaded public keys should not be empty"
                );
            }
            other => panic!("expected VerifiedPartialIdentity, got {:?}", other),
        }
    }

    #[test]
    fn verify_identity_update_rejects_unrelated_revision() {
        let (drive, identity) = setup_drive_and_identity();
        let platform_version = PlatformVersion::latest();
        let identity_id = identity.id().to_buffer();
        let key_request = IdentityKeysRequest::new_all_keys_query(&identity_id, None);
        let keys_path_query = key_request.into_path_query();
        let revision_path_query = Drive::identity_revision_query(&identity_id);
        let merged = grovedb::PathQuery::merge(
            vec![&keys_path_query, &revision_path_query],
            &platform_version.drive.grove_version,
        )
        .expect("expected to merge path queries");
        let proof = drive
            .grove_get_proved_path_query(&merged, None, &mut vec![], &platform_version.drive)
            .expect("expected to prove identity keys and revision");

        use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
        use dpp::state_transition::state_transitions::identity::identity_update_transition::v0::IdentityUpdateTransitionV0;
        let st = StateTransition::IdentityUpdate(IdentityUpdateTransition::V0(
            IdentityUpdateTransitionV0 {
                identity_id: identity.id(),
                revision: identity.revision() + 1,
                ..Default::default()
            },
        ));

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &proof,
            &|_id| Ok(None),
            platform_version,
        );

        assert!(matches!(
            result,
            Err(Error::Proof(ProofError::IncorrectProof(_)))
        ));
    }

    #[test]
    fn verify_identity_update_rejects_active_key_as_disabled() {
        let (drive, identity) = setup_drive_and_identity();
        let platform_version = PlatformVersion::latest();
        let identity_id = identity.id().to_buffer();
        let key_request = IdentityKeysRequest::new_all_keys_query(&identity_id, None);
        let keys_path_query = key_request.into_path_query();
        let revision_path_query = Drive::identity_revision_query(&identity_id);
        let merged = grovedb::PathQuery::merge(
            vec![&keys_path_query, &revision_path_query],
            &platform_version.drive.grove_version,
        )
        .expect("expected to merge path queries");
        let proof = drive
            .grove_get_proved_path_query(&merged, None, &mut vec![], &platform_version.drive)
            .expect("expected to prove identity keys and revision");
        let key_id = *identity
            .public_keys()
            .keys()
            .next()
            .expect("identity should have a public key");

        use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
        use dpp::state_transition::state_transitions::identity::identity_update_transition::v0::IdentityUpdateTransitionV0;
        let st = StateTransition::IdentityUpdate(IdentityUpdateTransition::V0(
            IdentityUpdateTransitionV0 {
                identity_id: identity.id(),
                revision: identity.revision(),
                disable_public_keys: vec![key_id],
                ..Default::default()
            },
        ));

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &proof,
            &|_id| Ok(None),
            platform_version,
        );

        assert!(matches!(
            result,
            Err(Error::Proof(ProofError::IncorrectProof(_)))
        ));
    }

    #[test]
    fn verify_identity_update_accepts_key_disabled_before_proof_block() {
        let (drive, identity) = setup_drive_and_identity();
        let platform_version = PlatformVersion::latest();
        let identity_id = identity.id().to_buffer();
        let key_id = *identity
            .public_keys()
            .keys()
            .next()
            .expect("identity should have a public key");

        // Disable the key at time 50, then verify against a proof served at a
        // later block: `disabled_at` never changes afterwards, so retried or
        // later proofs must still verify.
        let disabled_at: u64 = 50;
        drive
            .disable_identity_keys(
                identity_id,
                vec![key_id],
                disabled_at,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to disable identity key");

        let key_request = IdentityKeysRequest::new_all_keys_query(&identity_id, None);
        let keys_path_query = key_request.into_path_query();
        let revision_path_query = Drive::identity_revision_query(&identity_id);
        let merged = grovedb::PathQuery::merge(
            vec![&keys_path_query, &revision_path_query],
            &platform_version.drive.grove_version,
        )
        .expect("expected to merge path queries");
        let proof = drive
            .grove_get_proved_path_query(&merged, None, &mut vec![], &platform_version.drive)
            .expect("expected to prove identity keys and revision");

        use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
        use dpp::state_transition::state_transitions::identity::identity_update_transition::v0::IdentityUpdateTransitionV0;
        let st = StateTransition::IdentityUpdate(IdentityUpdateTransition::V0(
            IdentityUpdateTransitionV0 {
                identity_id: identity.id(),
                revision: identity.revision(),
                disable_public_keys: vec![key_id],
                ..Default::default()
            },
        ));

        let later_block = BlockInfo {
            time_ms: disabled_at + 10_000,
            ..Default::default()
        };
        let (_root_hash, result) = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &later_block,
            &proof,
            &|_id| Ok(None),
            platform_version,
        )
        .expect("expected verification to accept a key disabled before the proof block");

        match result {
            StateTransitionProofOutcome::ExecutionProved(
                StateTransitionProofResult::VerifiedPartialIdentity(partial_identity),
            ) => {
                assert_eq!(partial_identity.id, identity.id());
            }
            other => panic!(
                "expected ExecutionProved(VerifiedPartialIdentity), got {:?}",
                other
            ),
        }
    }

    // -----------------------------------------------------------------------
    // IdentityCreditTransfer
    // -----------------------------------------------------------------------

    #[test]
    fn verify_identity_credit_transfer_returns_affected_state_snapshot() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let sender = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a sender identity");
        let recipient = Identity::random_identity(3, Some(15), platform_version)
            .expect("expected a recipient identity");

        drive
            .add_new_identity(
                sender.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add sender identity");
        drive
            .add_new_identity(
                recipient.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add recipient identity");

        let proof = drive
            .prove_many_identity_balances(
                &[sender.id().to_buffer(), recipient.id().to_buffer()],
                None,
                &platform_version.drive,
            )
            .expect("expected to prove both balances");

        use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
        use dpp::state_transition::state_transitions::identity::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
        let st = StateTransition::IdentityCreditTransfer(IdentityCreditTransferTransition::V0(
            IdentityCreditTransferTransitionV0 {
                identity_id: sender.id(),
                recipient_id: recipient.id(),
                amount: 50,
                nonce: 1,
                ..Default::default()
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let (_root_hash, outcome) = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &proof,
            known_contracts_provider_fn,
            platform_version,
        )
        .expect("expected verification to succeed");

        match outcome {
            StateTransitionProofOutcome::AffectedState(
                StateTransitionProofResult::VerifiedBalanceTransfer(
                    sender_identity,
                    recipient_identity,
                ),
            ) => {
                assert_eq!(sender_identity.id, sender.id());
                assert_eq!(recipient_identity.id, recipient.id());
                assert!(sender_identity.balance.is_some());
                assert!(recipient_identity.balance.is_some());
            }
            other => panic!(
                "expected AffectedState(VerifiedBalanceTransfer), got {:?}",
                other
            ),
        }
    }

    // -----------------------------------------------------------------------
    // Batch: empty transitions error
    // -----------------------------------------------------------------------

    #[test]
    fn verify_batch_empty_transitions_returns_error() {
        let platform_version = PlatformVersion::latest();

        use dpp::state_transition::batch_transition::BatchTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV0;
        let st = StateTransition::Batch(BatchTransition::V0(BatchTransitionV0 {
            owner_id: Default::default(),
            transitions: vec![],
            ..Default::default()
        }));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected Error::Proof, got: {:?}",
            result
        );
        let err = result.unwrap_err();
        match &err {
            Error::Proof(ProofError::InvalidTransition(msg)) => {
                assert!(
                    msg.contains("no transition"),
                    "expected error about no transition, got: {}",
                    msg
                );
            }
            other => panic!("expected InvalidTransition error, got: {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Batch: too many transitions error
    // -----------------------------------------------------------------------

    #[test]
    fn verify_batch_too_many_transitions_returns_error() {
        let platform_version = PlatformVersion::latest();

        use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
        use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
        use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
        use dpp::state_transition::batch_transition::document_delete_transition::DocumentDeleteTransition;
        use dpp::state_transition::batch_transition::document_delete_transition::DocumentDeleteTransitionV0;
        use dpp::state_transition::batch_transition::BatchTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV0;

        let base1 = DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Default::default(),
            identity_contract_nonce: 1,
            document_type_name: "test".to_string(),
            data_contract_id: Default::default(),
        });
        let base2 = DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Default::default(),
            identity_contract_nonce: 2,
            document_type_name: "test".to_string(),
            data_contract_id: Default::default(),
        });

        let st = StateTransition::Batch(BatchTransition::V0(BatchTransitionV0 {
            owner_id: Default::default(),
            transitions: vec![
                DocumentTransition::Delete(DocumentDeleteTransition::V0(
                    DocumentDeleteTransitionV0 { base: base1 },
                )),
                DocumentTransition::Delete(DocumentDeleteTransition::V0(
                    DocumentDeleteTransitionV0 { base: base2 },
                )),
            ],
            ..Default::default()
        }));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected Error::Proof, got: {:?}",
            result
        );
        let err = result.unwrap_err();
        match &err {
            Error::Proof(ProofError::InvalidTransition(msg)) => {
                assert!(
                    msg.contains("does not support more than one document"),
                    "expected error about too many transitions, got: {}",
                    msg
                );
            }
            other => panic!("expected InvalidTransition error, got: {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Batch: document delete happy path
    // -----------------------------------------------------------------------

    #[test]
    fn verify_batch_document_delete_happy_path() {
        let (drive, contract) = setup_drive_and_contract();
        let platform_version = PlatformVersion::latest();

        let document_type = contract
            .document_type_for_name("preorder")
            .expect("expected preorder document type");

        let document = document_type
            .random_document(Some(99), platform_version)
            .expect("expected a random document");
        let doc_id = document.id();

        // Insert the document
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, None)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to insert document");

        // Delete the document
        drive
            .delete_document_for_contract(
                doc_id,
                &contract,
                "preorder",
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to delete document");

        // Generate a proof for the now-absent document
        use crate::query::{SingleDocumentDriveQuery, SingleDocumentDriveQueryContestedStatus};
        let single_query = SingleDocumentDriveQuery {
            contract_id: contract.id().to_buffer(),
            document_type_name: "preorder".to_string(),
            document_type_keeps_history: document_type.documents_keep_history(),
            document_id: doc_id.to_buffer(),
            block_time_ms: None,
            contested_status: SingleDocumentDriveQueryContestedStatus::NotContested,
        };
        let path_query = single_query
            .construct_path_query(platform_version)
            .expect("expected to build path query");
        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("expected to get proof");

        // Build a document delete batch transition
        use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
        use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
        use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
        use dpp::state_transition::batch_transition::document_delete_transition::DocumentDeleteTransition;
        use dpp::state_transition::batch_transition::document_delete_transition::DocumentDeleteTransitionV0;
        use dpp::state_transition::batch_transition::BatchTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV0;

        let base = DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: doc_id,
            identity_contract_nonce: 1,
            document_type_name: "preorder".to_string(),
            data_contract_id: contract.id(),
        });

        let st = StateTransition::Batch(BatchTransition::V0(BatchTransitionV0 {
            owner_id: Default::default(),
            transitions: vec![DocumentTransition::Delete(DocumentDeleteTransition::V0(
                DocumentDeleteTransitionV0 { base },
            ))],
            ..Default::default()
        }));

        let contract_arc = Arc::new(contract.clone());
        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(Some(contract_arc.clone()));

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &proof,
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            result.is_ok(),
            "expected verification to succeed, got: {:?}",
            result.err()
        );
        let (_root_hash, proof_result) = result.unwrap();
        match proof_result {
            StateTransitionProofOutcome::ExecutionProved(
                StateTransitionProofResult::VerifiedDocuments(docs),
            ) => {
                assert_eq!(docs.len(), 1, "expected exactly one document entry");
                let (returned_id, maybe_doc) = docs.into_iter().next().unwrap();
                assert_eq!(returned_id, doc_id);
                assert!(
                    maybe_doc.is_none(),
                    "document should be None after deletion"
                );
            }
            other => panic!("expected VerifiedDocuments, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Batch: document create happy path
    // -----------------------------------------------------------------------

    #[test]
    fn verify_batch_document_create_happy_path() {
        let (drive, contract) = setup_drive_and_contract();
        let platform_version = PlatformVersion::latest();

        let document_type = contract
            .document_type_for_name("preorder")
            .expect("expected preorder document type");

        let document = document_type
            .random_document(Some(99), platform_version)
            .expect("expected a random document");
        let doc_id = document.id();
        let owner_id = document.owner_id();

        let block_info = BlockInfo::default();

        // Insert the document
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, None)),
                        owner_id: Some(owner_id.to_buffer()),
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                block_info,
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to insert document");

        // Generate a proof for the existing document
        use crate::query::{SingleDocumentDriveQuery, SingleDocumentDriveQueryContestedStatus};
        let single_query = SingleDocumentDriveQuery {
            contract_id: contract.id().to_buffer(),
            document_type_name: "preorder".to_string(),
            document_type_keeps_history: document_type.documents_keep_history(),
            document_id: doc_id.to_buffer(),
            block_time_ms: None,
            contested_status: SingleDocumentDriveQueryContestedStatus::NotContested,
        };
        let path_query = single_query
            .construct_path_query(platform_version)
            .expect("expected to build path query");
        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("expected to get proof");

        // Build a document create batch transition
        use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
        use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
        use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
        use dpp::state_transition::batch_transition::document_create_transition::DocumentCreateTransition;
        use dpp::state_transition::batch_transition::document_create_transition::DocumentCreateTransitionV0;
        use dpp::state_transition::batch_transition::BatchTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV0;

        let base = DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: doc_id,
            identity_contract_nonce: 1,
            document_type_name: "preorder".to_string(),
            data_contract_id: contract.id(),
        });

        // Build the create transition from document properties
        let create_transition = DocumentCreateTransition::V0(DocumentCreateTransitionV0 {
            base,
            entropy: [100u8; 32],
            data: document.properties().clone(),
            prefunded_voting_balance: None,
        });

        let st = StateTransition::Batch(BatchTransition::V0(BatchTransitionV0 {
            owner_id,
            transitions: vec![DocumentTransition::Create(create_transition)],
            ..Default::default()
        }));

        let contract_arc = Arc::new(contract.clone());
        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(Some(contract_arc.clone()));

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &block_info,
            &proof,
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            result.is_ok(),
            "expected verification to succeed, got: {:?}",
            result.err()
        );
        let (_root_hash, proof_result) = result.unwrap();
        match proof_result {
            StateTransitionProofOutcome::ExecutionProved(
                StateTransitionProofResult::VerifiedDocuments(docs),
            ) => {
                assert_eq!(docs.len(), 1, "expected exactly one document entry");
                let (returned_id, maybe_doc) = docs.into_iter().next().unwrap();
                assert_eq!(returned_id, doc_id);
                assert!(maybe_doc.is_some(), "document should exist after creation");
            }
            other => panic!("expected VerifiedDocuments, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Batch: unknown contract returns error
    // -----------------------------------------------------------------------

    #[test]
    fn verify_batch_document_unknown_contract_returns_error() {
        let platform_version = PlatformVersion::latest();

        use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
        use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
        use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
        use dpp::state_transition::batch_transition::document_delete_transition::DocumentDeleteTransition;
        use dpp::state_transition::batch_transition::document_delete_transition::DocumentDeleteTransitionV0;
        use dpp::state_transition::batch_transition::BatchTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV0;

        let base = DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Default::default(),
            identity_contract_nonce: 1,
            document_type_name: "test".to_string(),
            data_contract_id: Default::default(),
        });

        let st = StateTransition::Batch(BatchTransition::V0(BatchTransitionV0 {
            owner_id: Default::default(),
            transitions: vec![DocumentTransition::Delete(DocumentDeleteTransition::V0(
                DocumentDeleteTransitionV0 { base },
            ))],
            ..Default::default()
        }));

        // Provider returns None for all contracts
        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected Error::Proof, got: {:?}",
            result
        );
        let err = result.unwrap_err();
        match &err {
            Error::Proof(ProofError::UnknownContract(msg)) => {
                assert!(
                    msg.contains("unknown contract"),
                    "expected error about unknown contract, got: {}",
                    msg
                );
            }
            other => panic!("expected UnknownContract error, got: {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // DataContractCreate: empty proof returns error
    // -----------------------------------------------------------------------

    #[test]
    fn verify_data_contract_create_empty_proof_returns_error() {
        let platform_version = PlatformVersion::latest();

        // Build a minimal contract create transition
        let created_contract =
            get_dpns_data_contract_fixture(None, 0, platform_version.protocol_version);
        let contract = created_contract.data_contract();
        let data_contract_serialized: DataContractInSerializationFormat = contract
            .clone()
            .try_into_platform_versioned(platform_version)
            .expect("expected to serialize contract");

        use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
        use dpp::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
        let st = StateTransition::DataContractCreate(DataContractCreateTransition::V0(
            DataContractCreateTransitionV0 {
                data_contract: data_contract_serialized,
                identity_nonce: 0,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        // Empty proof should cause a GroveDB/proof error
        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected Error::Proof or Error::GroveDB for empty proof, got: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // IdentityCreate: empty proof returns error
    // -----------------------------------------------------------------------

    #[test]
    fn verify_identity_create_empty_proof_returns_error() {
        let platform_version = PlatformVersion::latest();

        use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
        use dpp::state_transition::state_transitions::identity::identity_create_transition::v0::IdentityCreateTransitionV0;
        let st = StateTransition::IdentityCreate(IdentityCreateTransition::V0(
            IdentityCreateTransitionV0 {
                identity_id: dpp::prelude::Identifier::random(),
                ..Default::default()
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected Error::Proof or Error::GroveDB for empty proof, got: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // IdentityTopUp: empty proof returns error
    // -----------------------------------------------------------------------

    #[test]
    fn verify_identity_top_up_empty_proof_returns_error() {
        let platform_version = PlatformVersion::latest();

        use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
        use dpp::state_transition::state_transitions::identity::identity_topup_transition::v0::IdentityTopUpTransitionV0;
        let st = StateTransition::IdentityTopUp(IdentityTopUpTransition::V0(
            IdentityTopUpTransitionV0 {
                identity_id: dpp::prelude::Identifier::random(),
                ..Default::default()
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected Error::Proof or Error::GroveDB for empty proof, got: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // IdentityCreditWithdrawal: empty proof returns error
    // -----------------------------------------------------------------------

    #[test]
    fn verify_identity_credit_withdrawal_empty_proof_returns_error() {
        let platform_version = PlatformVersion::latest();

        use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
        use dpp::state_transition::state_transitions::identity::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
        let st = StateTransition::IdentityCreditWithdrawal(IdentityCreditWithdrawalTransition::V0(
            IdentityCreditWithdrawalTransitionV0 {
                identity_id: dpp::prelude::Identifier::random(),
                amount: 100,
                ..Default::default()
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected Error::Proof or Error::GroveDB for empty proof, got: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // IdentityUpdate: empty proof returns error
    // -----------------------------------------------------------------------

    #[test]
    fn verify_identity_update_empty_proof_returns_error() {
        let platform_version = PlatformVersion::latest();

        use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
        use dpp::state_transition::state_transitions::identity::identity_update_transition::v0::IdentityUpdateTransitionV0;
        let st = StateTransition::IdentityUpdate(IdentityUpdateTransition::V0(
            IdentityUpdateTransitionV0 {
                identity_id: dpp::prelude::Identifier::random(),
                revision: 1,
                ..Default::default()
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected Error::Proof or Error::GroveDB for empty proof, got: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // IdentityCreditTransfer: empty proof returns error
    // -----------------------------------------------------------------------

    #[test]
    fn verify_identity_credit_transfer_empty_proof_returns_error() {
        let platform_version = PlatformVersion::latest();

        use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
        use dpp::state_transition::state_transitions::identity::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
        let st = StateTransition::IdentityCreditTransfer(IdentityCreditTransferTransition::V0(
            IdentityCreditTransferTransitionV0 {
                identity_id: dpp::prelude::Identifier::random(),
                recipient_id: dpp::prelude::Identifier::random(),
                amount: 50,
                nonce: 1,
                ..Default::default()
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected Error::Proof or Error::GroveDB for empty proof, got: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // Additional coverage for rarely-executed error branches.
    //
    // The following tests exercise variants that were not previously
    // covered by the inline suite, and in particular focus on error arms
    // (unknown-contract, invalid-transition, malformed-input) that are
    // otherwise hard to exercise through happy-path tests.
    // -----------------------------------------------------------------------

    // --- DataContractUpdate: empty proof returns Error::Proof or Error::GroveDB.
    #[test]
    fn verify_data_contract_update_empty_proof_returns_error() {
        let (_drive, contract) = setup_drive_and_contract();
        let platform_version = PlatformVersion::latest();
        let data_contract_serialized: DataContractInSerializationFormat = contract
            .clone()
            .try_into_platform_versioned(platform_version)
            .expect("expected to serialize contract");

        use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0;
        let st = StateTransition::DataContractUpdate(DataContractUpdateTransition::V0(
            DataContractUpdateTransitionV0 {
                identity_contract_nonce: 1,
                data_contract: data_contract_serialized,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        // Empty proof → must error (either Error::Proof or Error::GroveDB).
        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected Error::Proof or Error::GroveDB for empty proof, got: {:?}",
            result
        );
    }

    // --- MasternodeVote: unknown contract returns a specific
    // `ProofError::UnknownContract` because the provider returns None.
    #[test]
    fn verify_masternode_vote_unknown_contract_returns_error() {
        let platform_version = PlatformVersion::latest();
        use dpp::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
        use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
        use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
        use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
        use dpp::voting::vote_polls::VotePoll;
        use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
        use dpp::voting::votes::resource_vote::ResourceVote;
        use dpp::voting::votes::Vote;

        let st = StateTransition::MasternodeVote(MasternodeVoteTransition::V0(
            MasternodeVoteTransitionV0 {
                pro_tx_hash: dpp::prelude::Identifier::from([1u8; 32]),
                voter_identity_id: dpp::prelude::Identifier::from([2u8; 32]),
                vote: Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
                    vote_poll: VotePoll::ContestedDocumentResourceVotePoll(
                        ContestedDocumentResourceVotePoll {
                            contract_id: dpp::prelude::Identifier::from([7u8; 32]),
                            document_type_name: "some_type".to_string(),
                            index_name: "idx".to_string(),
                            index_values: vec![],
                        },
                    ),
                    resource_vote_choice: ResourceVoteChoice::Abstain,
                })),
                nonce: 1,
                signature_public_key_id: 0,
                signature: Default::default(),
            },
        ));

        // Provider always returns None – triggers UnknownContract.
        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        let err = result.expect_err("expected UnknownContract error, got Ok");
        match err {
            crate::error::Error::Proof(ProofError::UnknownContract(msg)) => {
                assert!(
                    msg.contains("unknown contract") && msg.contains("resource vote"),
                    "unexpected UnknownContract message: {msg}"
                );
            }
            other => panic!("expected Error::Proof(UnknownContract), got {:?}", other),
        }
    }

    // --- ShieldedWithdrawal: empty proof + no nullifiers → error.
    //
    // The `first_nullifier` guard in the verifier is only reachable once
    // `verify_shielded_nullifiers` returns Ok. With an empty proof, grove-db
    // proof verification errors out first, so this test cannot reach the
    // `InvalidTransition` guard specifically — we only assert that some
    // Error::Proof / Error::GroveDB is returned on the failing path.
    #[test]
    fn verify_shielded_withdrawal_empty_proof_error() {
        let platform_version = PlatformVersion::latest();
        use dpp::identity::core_script::CoreScript;
        use dpp::state_transition::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;
        use dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
        use dpp::withdrawal::Pooling;

        let st = StateTransition::ShieldedWithdrawal(ShieldedWithdrawalTransition::V0(
            ShieldedWithdrawalTransitionV0 {
                actions: vec![],
                unshielding_amount: 0,
                anchor: [0u8; 32],
                proof: vec![],
                binding_signature: [0u8; 64],
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: CoreScript::from_bytes(vec![]),
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected error for shielded withdrawal with empty proof, got: {:?}",
            result
        );
    }

    // --- ShieldedTransfer: empty proof leads to an error (Error::Proof
    // or Error::GroveDB) through the verify_shielded_nullifiers path.
    #[test]
    fn verify_shielded_transfer_empty_proof_returns_error() {
        let platform_version = PlatformVersion::latest();
        use dpp::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;
        use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;

        let st = StateTransition::ShieldedTransfer(ShieldedTransferTransition::V0(
            ShieldedTransferTransitionV0 {
                actions: vec![],
                value_balance: 0,
                anchor: [0u8; 32],
                proof: vec![],
                binding_signature: [0u8; 64],
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected error for shielded transfer with empty proof, got: {:?}",
            result
        );
    }

    // --- Unshield: empty proof leads to an error.
    #[test]
    fn verify_unshield_empty_proof_returns_error() {
        let platform_version = PlatformVersion::latest();
        use dpp::state_transition::unshield_transition::v0::UnshieldTransitionV0;
        use dpp::state_transition::unshield_transition::UnshieldTransition;

        let st = StateTransition::Unshield(UnshieldTransition::V0(UnshieldTransitionV0 {
            output_address: Default::default(),
            actions: vec![],
            unshielding_amount: 0,
            anchor: [0u8; 32],
            proof: vec![],
            binding_signature: [0u8; 64],
        }));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected error for unshield with empty proof, got: {:?}",
            result
        );
    }

    // --- IdentityCreateFromShieldedPool: empty proof returns error.
    //
    // Exercises the STRICT merged-query verify arm: an empty proof cannot satisfy the strict
    // `verify_query` over the merged {nullifier-tree, identity} query, so the verifier must reject
    // (rather than silently accepting). The positive prove→verify roundtrip lives in drive-abci's
    // identity_create_from_shielded_pool tests (synthetic-action execution).
    #[test]
    fn verify_identity_create_from_shielded_pool_empty_proof_returns_error() {
        let platform_version = PlatformVersion::latest();
        use dpp::shielded::SerializedAction;
        use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::derive_identity_id_from_actions;
        use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::v0::IdentityCreateFromShieldedPoolTransitionV0;
        use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;

        let actions = vec![SerializedAction {
            nullifier: [0x11; 32],
            rk: [0x22; 32],
            cmx: [0x33; 32],
            encrypted_note: vec![0x44; 216],
            cv_net: [0x55; 32],
            spend_auth_sig: [0x66; 64],
        }];
        let identity_id = derive_identity_id_from_actions(&actions);

        let st = StateTransition::IdentityCreateFromShieldedPool(
            IdentityCreateFromShieldedPoolTransition::V0(
                IdentityCreateFromShieldedPoolTransitionV0 {
                    public_keys: vec![],
                    denomination: 10_000_000_000,
                    actions,
                    anchor: [0u8; 32],
                    proof: vec![],
                    binding_signature: [0u8; 64],
                    send_to_address_on_creation_failure: PlatformAddress::P2pkh([0u8; 20]),
                    identity_id,
                },
            ),
        );

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected error for identity create from shielded pool with empty proof, got: {:?}",
            result
        );
    }

    // --- IdentityCreditTransferToAddresses: empty proof returns error.
    #[test]
    fn verify_identity_credit_transfer_to_addresses_empty_proof_returns_error() {
        let platform_version = PlatformVersion::latest();
        use dpp::state_transition::identity_credit_transfer_to_addresses_transition::v0::IdentityCreditTransferToAddressesTransitionV0;
        use dpp::state_transition::identity_credit_transfer_to_addresses_transition::IdentityCreditTransferToAddressesTransition;

        let st = StateTransition::IdentityCreditTransferToAddresses(
            IdentityCreditTransferToAddressesTransition::V0(
                IdentityCreditTransferToAddressesTransitionV0 {
                    identity_id: dpp::prelude::Identifier::from([3u8; 32]),
                    ..Default::default()
                },
            ),
        );

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected error for identity credit transfer to addresses with empty proof, got: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // Additional error-path coverage for state transitions whose error arms
    // were not previously tested. These tests target proof decoding / early
    // validation errors on state transition variants that were not covered
    // by the existing test suite.
    // -----------------------------------------------------------------------

    // --- AddressFundsTransfer: empty proof returns error.
    #[test]
    fn verify_address_funds_transfer_empty_proof_returns_error() {
        let platform_version = PlatformVersion::latest();
        use dpp::address_funds::PlatformAddress;
        use dpp::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
        use dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
        use std::collections::BTreeMap;

        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([1u8; 20]), (1u32, 1000u64));
        let mut outputs = BTreeMap::new();
        outputs.insert(PlatformAddress::P2pkh([2u8; 20]), 500u64);

        let st = StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
            AddressFundsTransferTransitionV0 {
                inputs,
                outputs,
                fee_strategy: vec![],
                user_fee_increase: 0,
                input_witnesses: vec![],
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(result, Err(Error::Proof(_)) | Err(Error::GroveDB(_))),
            "expected a proof error for an empty proof, got: {:?}",
            result
        );
    }

    // --- AddressCreditWithdrawal: empty proof returns error.
    #[test]
    fn verify_address_credit_withdrawal_empty_proof_returns_error() {
        let platform_version = PlatformVersion::latest();
        use dpp::address_funds::PlatformAddress;
        use dpp::identity::core_script::CoreScript;
        use dpp::state_transition::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;
        use dpp::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
        use dpp::withdrawal::Pooling;
        use std::collections::BTreeMap;

        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([3u8; 20]), (1u32, 2000u64));

        let st = StateTransition::AddressCreditWithdrawal(AddressCreditWithdrawalTransition::V0(
            AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: vec![],
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: CoreScript::from_bytes(vec![]),
                user_fee_increase: 0,
                input_witnesses: vec![],
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(result, Err(Error::Proof(_)) | Err(Error::GroveDB(_))),
            "expected a proof error for an empty proof, got: {:?}",
            result
        );
    }

    // --- AddressCreditWithdrawal with change output: empty proof returns error.
    #[test]
    fn verify_address_credit_withdrawal_with_change_output_empty_proof_returns_error() {
        let platform_version = PlatformVersion::latest();
        use dpp::address_funds::PlatformAddress;
        use dpp::identity::core_script::CoreScript;
        use dpp::state_transition::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;
        use dpp::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
        use dpp::withdrawal::Pooling;
        use std::collections::BTreeMap;

        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([3u8; 20]), (1u32, 2000u64));

        let st = StateTransition::AddressCreditWithdrawal(AddressCreditWithdrawalTransition::V0(
            AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: Some((PlatformAddress::P2pkh([4u8; 20]), 1000u64)),
                fee_strategy: vec![],
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: CoreScript::from_bytes(vec![]),
                user_fee_increase: 0,
                input_witnesses: vec![],
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(result, Err(Error::Proof(_)) | Err(Error::GroveDB(_))),
            "expected a proof error for an empty proof, got: {:?}",
            result
        );
    }

    // --- AddressFundingFromAssetLock: empty proof returns error.
    #[test]
    fn verify_address_funding_from_asset_lock_empty_proof_returns_error() {
        let platform_version = PlatformVersion::latest();
        use dpp::address_funds::PlatformAddress;
        use dpp::identity::state_transition::asset_lock_proof::AssetLockProof;
        use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
        use dpp::platform_value::BinaryData;
        use dpp::state_transition::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
        use dpp::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
        use std::collections::BTreeMap;

        let mut outputs = BTreeMap::new();
        outputs.insert(PlatformAddress::P2pkh([7u8; 20]), None);

        let st = StateTransition::AddressFundingFromAssetLock(
            AddressFundingFromAssetLockTransition::V0(AddressFundingFromAssetLockTransitionV0 {
                asset_lock_proof: AssetLockProof::Instant(InstantAssetLockProof::default()),
                inputs: BTreeMap::new(),
                outputs,
                fee_strategy: vec![],
                user_fee_increase: 0,
                signature: BinaryData::new(vec![]),
                input_witnesses: vec![],
            }),
        );

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(result, Err(Error::Proof(_)) | Err(Error::GroveDB(_))),
            "expected a proof error for an empty proof, got: {:?}",
            result
        );
    }

    // --- Shield: empty proof returns error.
    #[test]
    fn verify_shield_empty_proof_returns_error() {
        let platform_version = PlatformVersion::latest();
        use dpp::address_funds::PlatformAddress;
        use dpp::state_transition::shield_transition::v0::ShieldTransitionV0;
        use dpp::state_transition::shield_transition::ShieldTransition;
        use std::collections::BTreeMap;

        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([8u8; 20]), (1u32, 1000u64));

        let st = StateTransition::Shield(ShieldTransition::V0(ShieldTransitionV0 {
            inputs,
            actions: vec![],
            amount: 500,
            anchor: [0u8; 32],
            proof: vec![],
            binding_signature: [0u8; 64],
            fee_strategy: vec![],
            user_fee_increase: 0,
            input_witnesses: vec![],
        }));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(result, Err(Error::Proof(_)) | Err(Error::GroveDB(_))),
            "expected a proof error for an empty proof, got: {:?}",
            result
        );
    }

    // --- ShieldFromAssetLock with invalid (no-output) Instant asset lock proof
    // should return InvalidTransition("shield from asset lock has no outpoint").
    // Default InstantAssetLockProof has no AssetLockPayload, so out_point() returns None.
    #[test]
    fn verify_shield_from_asset_lock_missing_outpoint_returns_invalid_transition() {
        let platform_version = PlatformVersion::latest();
        use dpp::identity::state_transition::asset_lock_proof::AssetLockProof;
        use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
        use dpp::platform_value::BinaryData;
        use dpp::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
        use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;

        let st = StateTransition::ShieldFromAssetLock(ShieldFromAssetLockTransition::V0(
            ShieldFromAssetLockTransitionV0 {
                asset_lock_proof: AssetLockProof::Instant(InstantAssetLockProof::default()),
                actions: vec![],
                value_balance: 0,
                anchor: [0u8; 32],
                proof: vec![],
                binding_signature: [0u8; 64],
                surplus_output: None,
                signature: BinaryData::new(vec![]),
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        let err = result.expect_err("expected InvalidTransition error, got Ok");
        match err {
            crate::error::Error::Proof(ProofError::InvalidTransition(msg)) => {
                assert!(
                    msg.contains("shield from asset lock has no outpoint"),
                    "unexpected InvalidTransition message: {msg}"
                );
            }
            other => panic!("expected Error::Proof(InvalidTransition), got {:?}", other),
        }
    }

    // --- ShieldFromAssetLock with Chain asset lock proof (has outpoint) and
    // empty GroveDB proof: should return an error from grove-db verification.
    #[test]
    fn verify_shield_from_asset_lock_chain_empty_proof_returns_error() {
        let platform_version = PlatformVersion::latest();
        use dpp::dashcore::OutPoint;
        use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
        use dpp::identity::state_transition::asset_lock_proof::AssetLockProof;
        use dpp::platform_value::BinaryData;
        use dpp::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
        use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;

        let st = StateTransition::ShieldFromAssetLock(ShieldFromAssetLockTransition::V0(
            ShieldFromAssetLockTransitionV0 {
                asset_lock_proof: AssetLockProof::Chain(ChainAssetLockProof {
                    core_chain_locked_height: 100,
                    out_point: OutPoint::from([11u8; 36]),
                }),
                actions: vec![],
                value_balance: 0,
                anchor: [0u8; 32],
                proof: vec![],
                binding_signature: [0u8; 64],
                surplus_output: None,
                signature: BinaryData::new(vec![]),
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected error for shield from asset lock (chain) with empty proof, got: {:?}",
            result
        );
    }

    // --- IdentityCreateFromAddresses: empty proof returns error.
    #[test]
    fn verify_identity_create_from_addresses_empty_proof_returns_error() {
        let platform_version = PlatformVersion::latest();
        use dpp::address_funds::PlatformAddress;
        use dpp::state_transition::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
        use dpp::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
        use std::collections::BTreeMap;

        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([9u8; 20]), (1u32, 2000u64));

        let st = StateTransition::IdentityCreateFromAddresses(
            IdentityCreateFromAddressesTransition::V0(IdentityCreateFromAddressesTransitionV0 {
                public_keys: vec![],
                inputs,
                output: None,
                fee_strategy: vec![],
                user_fee_increase: 0,
                input_witnesses: vec![],
            }),
        );

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected error for identity create from addresses with empty proof, got: {:?}",
            result
        );
    }

    #[test]
    fn verify_identity_create_from_addresses_binds_public_keys() {
        use dpp::address_funds::PlatformAddress;
        use dpp::state_transition::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
        use dpp::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
        use dpp::state_transition::StateTransitionIdentityIdFromInputs;
        use std::collections::BTreeMap;

        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([9u8; 20]), (1u32, 2000u64));
        let id_transition =
            IdentityCreateFromAddressesTransition::V0(IdentityCreateFromAddressesTransitionV0 {
                public_keys: vec![],
                inputs: inputs.clone(),
                output: None,
                fee_strategy: vec![],
                user_fee_increase: 0,
                input_witnesses: vec![],
            });
        let identity_id = id_transition
            .identity_id_from_inputs()
            .expect("expected identity id from inputs");

        let mut identity = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        identity.set_id(identity_id);
        drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add identity");

        let matching = StateTransition::IdentityCreateFromAddresses(
            IdentityCreateFromAddressesTransition::V0(IdentityCreateFromAddressesTransitionV0 {
                public_keys: identity.public_keys().values().map(Into::into).collect(),
                inputs: inputs.clone(),
                output: None,
                fee_strategy: vec![],
                user_fee_increase: 0,
                input_witnesses: vec![],
            }),
        );
        let proof = drive
            .prove_state_transition(&matching, None, platform_version)
            .expect("expected proof creation")
            .into_data()
            .expect("expected proof data");

        let matching_result = Drive::verify_state_transition_was_executed_with_proof(
            &matching,
            &BlockInfo::default(),
            &proof,
            &|_id| Ok(None),
            platform_version,
        );
        assert!(matching_result.is_ok());

        let mismatched = StateTransition::IdentityCreateFromAddresses(
            IdentityCreateFromAddressesTransition::V0(IdentityCreateFromAddressesTransitionV0 {
                public_keys: vec![],
                inputs,
                output: None,
                fee_strategy: vec![],
                user_fee_increase: 0,
                input_witnesses: vec![],
            }),
        );
        let mismatched_result = Drive::verify_state_transition_was_executed_with_proof(
            &mismatched,
            &BlockInfo::default(),
            &proof,
            &|_id| Ok(None),
            platform_version,
        );
        assert!(matches!(
            mismatched_result,
            Err(Error::Proof(ProofError::IncorrectProof(_)))
        ));
    }

    // --- IdentityTopUpFromAddresses: empty proof returns error.
    #[test]
    fn verify_identity_top_up_from_addresses_empty_proof_returns_error() {
        let platform_version = PlatformVersion::latest();
        use dpp::address_funds::PlatformAddress;
        use dpp::state_transition::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;
        use dpp::state_transition::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
        use std::collections::BTreeMap;

        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([10u8; 20]), (1u32, 500u64));

        let st = StateTransition::IdentityTopUpFromAddresses(
            IdentityTopUpFromAddressesTransition::V0(IdentityTopUpFromAddressesTransitionV0 {
                inputs,
                output: None,
                identity_id: dpp::prelude::Identifier::from([11u8; 32]),
                fee_strategy: vec![],
                user_fee_increase: 0,
                input_witnesses: vec![],
            }),
        );

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected error for identity top up from addresses with empty proof, got: {:?}",
            result
        );
    }

    // --- IdentityCreditWithdrawal V1 empty proof returns error.
    #[test]
    fn verify_identity_credit_withdrawal_v1_empty_proof_returns_error() {
        let platform_version = PlatformVersion::latest();
        use dpp::state_transition::identity_credit_withdrawal_transition::v1::IdentityCreditWithdrawalTransitionV1;
        use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
        use dpp::withdrawal::Pooling;

        let st = StateTransition::IdentityCreditWithdrawal(IdentityCreditWithdrawalTransition::V1(
            IdentityCreditWithdrawalTransitionV1 {
                identity_id: dpp::prelude::Identifier::random(),
                amount: 100,
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: None,
                nonce: 1,
                user_fee_increase: 0,
                ..Default::default()
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected error for identity credit withdrawal v1 with empty proof, got: {:?}",
            result
        );
    }

    // --- MasternodeVote: empty proof (but with a known contract) returns
    // an Error::Proof / Error::GroveDB because the proof can't be decoded.
    #[test]
    fn verify_masternode_vote_empty_proof_with_known_contract_returns_error() {
        let (_drive, contract) = setup_drive_and_contract();
        let platform_version = PlatformVersion::latest();
        use dpp::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
        use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
        use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
        use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
        use dpp::voting::vote_polls::VotePoll;
        use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
        use dpp::voting::votes::resource_vote::ResourceVote;
        use dpp::voting::votes::Vote;

        let st = StateTransition::MasternodeVote(MasternodeVoteTransition::V0(
            MasternodeVoteTransitionV0 {
                pro_tx_hash: dpp::prelude::Identifier::from([1u8; 32]),
                voter_identity_id: dpp::prelude::Identifier::from([2u8; 32]),
                vote: Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
                    vote_poll: VotePoll::ContestedDocumentResourceVotePoll(
                        ContestedDocumentResourceVotePoll {
                            contract_id: contract.id(),
                            document_type_name: "preorder".to_string(),
                            index_name: "idx".to_string(),
                            index_values: vec![],
                        },
                    ),
                    resource_vote_choice: ResourceVoteChoice::Abstain,
                })),
                nonce: 1,
                signature_public_key_id: 0,
                signature: Default::default(),
            },
        ));

        let contract_arc = Arc::new(contract.clone());
        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(Some(contract_arc.clone()));

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected error for masternode vote with empty proof, got: {:?}",
            result
        );
    }

    // --- ShieldedWithdrawal with no nullifiers and a known contract that
    // triggers InvalidTransition("shielded withdrawal has no nullifiers") via
    // the first_nullifier guard.
    //
    // We route past the grove-db verification by providing `actions: vec![]`
    // (so nullifiers() is empty) and a valid empty proof for nullifiers.
    // Because proof is empty, grove-db errors out first — we only check for
    // some Error variant here (this is a defensive test that ensures the
    // known-contract branch doesn't crash).
    #[test]
    fn verify_shielded_withdrawal_with_known_contract_empty_proof_returns_error() {
        let platform_version = PlatformVersion::latest();
        use dpp::identity::core_script::CoreScript;
        use dpp::state_transition::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;
        use dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
        use dpp::withdrawal::Pooling;

        let st = StateTransition::ShieldedWithdrawal(ShieldedWithdrawalTransition::V0(
            ShieldedWithdrawalTransitionV0 {
                actions: vec![],
                unshielding_amount: 0,
                anchor: [0u8; 32],
                proof: vec![],
                binding_signature: [0u8; 64],
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: CoreScript::from_bytes(vec![]),
            },
        ));

        // Load the real withdrawals contract so the known_contracts_provider
        // returns Some. Empty proof will still fail, but we exercise the
        // contract-lookup path.
        use dpp::data_contracts::withdrawals_contract;
        use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
        let withdrawals =
            load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
                .expect("expected withdrawals system contract");
        let withdrawals_arc = Arc::new(withdrawals);
        let expected_id = withdrawals_contract::ID;
        let known_contracts_provider_fn: &ContractLookupFn = &|id| {
            if id == &expected_id {
                Ok(Some(withdrawals_arc.clone()))
            } else {
                Ok(None)
            }
        };

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected error for shielded withdrawal (known contract, empty proof), got: {:?}",
            result
        );
    }

    // --- Batch V1 with Token Mint transition + unknown contract returns
    // UnknownContract error (token transition branch).
    /// The classifier is the single authority on which transition families'
    /// proofs bind execution; this table pins the snapshot-only families so
    /// a reclassification (accidental or deliberate) fails a test instead of
    /// silently upgrading a snapshot into execution evidence.
    #[test]
    fn classifier_tags_snapshot_only_families_as_affected_state() {
        use dpp::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
        use dpp::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
        use dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
        use dpp::state_transition::identity_credit_transfer_to_addresses_transition::IdentityCreditTransferToAddressesTransition;
        use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
        use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
        use dpp::state_transition::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
        use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;

        let no_contracts: &ContractLookupFn = &|_id| Ok(None);

        let snapshot_only: Vec<(&str, StateTransition)> = vec![
            (
                "identity top up",
                StateTransition::IdentityTopUp(IdentityTopUpTransition::V0(Default::default())),
            ),
            (
                "identity credit withdrawal",
                StateTransition::IdentityCreditWithdrawal(IdentityCreditWithdrawalTransition::V0(
                    Default::default(),
                )),
            ),
            (
                "identity credit transfer",
                StateTransition::IdentityCreditTransfer(IdentityCreditTransferTransition::V0(
                    Default::default(),
                )),
            ),
            (
                "identity credit transfer to addresses",
                StateTransition::IdentityCreditTransferToAddresses(
                    IdentityCreditTransferToAddressesTransition::V0(Default::default()),
                ),
            ),
            (
                "identity top up from addresses",
                StateTransition::IdentityTopUpFromAddresses(
                    IdentityTopUpFromAddressesTransition::V0(Default::default()),
                ),
            ),
            (
                "address funds transfer",
                StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                    Default::default(),
                )),
            ),
            (
                "address funding from asset lock",
                StateTransition::AddressFundingFromAssetLock(
                    AddressFundingFromAssetLockTransition::V0(Default::default()),
                ),
            ),
            (
                "address credit withdrawal",
                StateTransition::AddressCreditWithdrawal(AddressCreditWithdrawalTransition::V0(
                    Default::default(),
                )),
            ),
        ];

        for (name, st) in snapshot_only {
            let binds = Drive::state_transition_proof_binds_execution(&st, no_contracts)
                .expect("classifier should not error for contract-independent families");
            assert!(
                !binds,
                "{name}: proof must be classified as an affected-state snapshot"
            );
        }
    }

    #[test]
    fn verify_no_history_token_empty_proof_returns_error() {
        use dpp::data_contract::accessors::v1::DataContractV1Setters;
        use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
        use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
        use dpp::data_contract::associated_token::token_keeps_history_rules::v0::TokenKeepsHistoryRulesV0;
        use dpp::state_transition::batch_transition::batched_transition::token_burn_transition::v0::TokenBurnTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::token_burn_transition::TokenBurnTransition;
        use dpp::state_transition::batch_transition::batched_transition::token_mint_transition::v0::TokenMintTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::token_mint_transition::TokenMintTransition;
        use dpp::state_transition::batch_transition::batched_transition::token_transfer_transition::v0::TokenTransferTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::token_transfer_transition::TokenTransferTransition;
        use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
        use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
        use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
        use dpp::state_transition::batch_transition::{BatchTransition, BatchTransitionV1};

        let platform_version = PlatformVersion::latest();
        let mut contract =
            get_dpns_data_contract_fixture(None, 0, platform_version.protocol_version)
                .data_contract_owned();
        let mut token_configuration = TokenConfigurationV0::default_most_restrictive();
        token_configuration.keeps_history =
            TokenKeepsHistoryRulesV0::default_for_keeping_all_history(false).into();
        contract.set_tokens(BTreeMap::from([(
            0,
            TokenConfiguration::V0(token_configuration),
        )]));
        let contract_id = contract.id();
        let token_id = contract.token_id(0).expect("expected token id");
        let owner_id = Identifier::from([9u8; 32]);
        let recipient_id = Identifier::from([10u8; 32]);

        let base = || {
            TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce: 1,
                token_contract_position: 0,
                data_contract_id: contract_id,
                token_id,
                using_group_info: None,
            })
        };
        let transitions = vec![
            TokenTransition::Burn(TokenBurnTransition::V0(TokenBurnTransitionV0 {
                base: base(),
                burn_amount: 42,
                public_note: None,
            })),
            TokenTransition::Mint(TokenMintTransition::V0(TokenMintTransitionV0 {
                base: base(),
                amount: 42,
                issued_to_identity_id: Some(recipient_id),
                public_note: None,
            })),
            TokenTransition::Transfer(TokenTransferTransition::V0(TokenTransferTransitionV0 {
                base: base(),
                amount: 42,
                recipient_id,
                public_note: None,
                shared_encrypted_note: None,
                private_encrypted_note: None,
            })),
        ];

        let contract = Arc::new(contract);
        let known_contracts_provider_fn: &ContractLookupFn = &|id| {
            if id == &contract_id {
                Ok(Some(contract.clone()))
            } else {
                Ok(None)
            }
        };

        for transition in transitions {
            let st = StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
                owner_id,
                transitions: vec![BatchedTransition::Token(transition)],
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }));
            let result = Drive::verify_state_transition_was_executed_with_proof(
                &st,
                &BlockInfo::default(),
                &[],
                known_contracts_provider_fn,
                platform_version,
            );
            assert!(
                matches!(result, Err(Error::Proof(_)) | Err(Error::GroveDB(_))),
                "expected a proof error for an empty proof, got: {:?}",
                result
            );
        }
    }

    // --- Batch V1 with Token Mint transition + unknown contract returns
    // UnknownContract error (token transition branch).
    #[test]
    fn verify_batch_token_mint_unknown_contract_returns_error() {
        let platform_version = PlatformVersion::latest();

        use dpp::state_transition::batch_transition::batched_transition::token_mint_transition::v0::TokenMintTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::token_mint_transition::TokenMintTransition;
        use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
        use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
        use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
        use dpp::state_transition::batch_transition::BatchTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV1;

        let token_base = TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: 1,
            token_contract_position: 0,
            data_contract_id: dpp::prelude::Identifier::from([44u8; 32]),
            token_id: dpp::prelude::Identifier::from([45u8; 32]),
            using_group_info: None,
        });
        let token_transition =
            TokenTransition::Mint(TokenMintTransition::V0(TokenMintTransitionV0 {
                base: token_base,
                amount: 99,
                issued_to_identity_id: Some(dpp::prelude::Identifier::from([77u8; 32])),
                public_note: None,
            }));

        let st = StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
            owner_id: dpp::prelude::Identifier::from([2u8; 32]),
            transitions: vec![BatchedTransition::Token(token_transition)],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        let err = result.expect_err("expected UnknownContract, got Ok");
        match err {
            crate::error::Error::Proof(ProofError::UnknownContract(msg)) => {
                assert!(
                    msg.contains("unknown contract") && msg.contains("token verification"),
                    "unexpected UnknownContract message: {msg}"
                );
            }
            other => panic!("expected Error::Proof(UnknownContract), got {:?}", other),
        }
    }

    // --- Batch V1 with Token Transfer transition + unknown contract.
    #[test]
    fn verify_batch_token_transfer_unknown_contract_returns_error() {
        let platform_version = PlatformVersion::latest();

        use dpp::state_transition::batch_transition::batched_transition::token_transfer_transition::v0::TokenTransferTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::token_transfer_transition::TokenTransferTransition;
        use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
        use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
        use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
        use dpp::state_transition::batch_transition::BatchTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV1;

        let token_base = TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: 1,
            token_contract_position: 0,
            data_contract_id: dpp::prelude::Identifier::from([58u8; 32]),
            token_id: dpp::prelude::Identifier::from([59u8; 32]),
            using_group_info: None,
        });
        let token_transition =
            TokenTransition::Transfer(TokenTransferTransition::V0(TokenTransferTransitionV0 {
                base: token_base,
                amount: 100,
                recipient_id: dpp::prelude::Identifier::from([88u8; 32]),
                public_note: None,
                shared_encrypted_note: None,
                private_encrypted_note: None,
            }));

        let st = StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
            owner_id: dpp::prelude::Identifier::from([3u8; 32]),
            transitions: vec![BatchedTransition::Token(token_transition)],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        let err = result.expect_err("expected UnknownContract, got Ok");
        assert!(
            matches!(
                err,
                crate::error::Error::Proof(ProofError::UnknownContract(_))
            ),
            "expected Error::Proof(UnknownContract), got: {:?}",
            err
        );
    }

    // --- Batch V1 with Token Freeze transition + unknown contract.
    #[test]
    fn verify_batch_token_freeze_unknown_contract_returns_error() {
        let platform_version = PlatformVersion::latest();

        use dpp::state_transition::batch_transition::batched_transition::token_freeze_transition::v0::TokenFreezeTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::token_freeze_transition::TokenFreezeTransition;
        use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
        use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
        use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
        use dpp::state_transition::batch_transition::BatchTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV1;

        let token_base = TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: 1,
            token_contract_position: 0,
            data_contract_id: dpp::prelude::Identifier::from([71u8; 32]),
            token_id: dpp::prelude::Identifier::from([72u8; 32]),
            using_group_info: None,
        });
        let token_transition =
            TokenTransition::Freeze(TokenFreezeTransition::V0(TokenFreezeTransitionV0 {
                base: token_base,
                identity_to_freeze_id: dpp::prelude::Identifier::from([91u8; 32]),
                public_note: None,
            }));

        let st = StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
            owner_id: dpp::prelude::Identifier::from([4u8; 32]),
            transitions: vec![BatchedTransition::Token(token_transition)],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        let err = result.expect_err("expected UnknownContract, got Ok");
        assert!(
            matches!(
                err,
                crate::error::Error::Proof(ProofError::UnknownContract(_))
            ),
            "expected Error::Proof(UnknownContract), got: {:?}",
            err
        );
    }

    // --- Batch V1 with Token Unfreeze transition + unknown contract.
    #[test]
    fn verify_batch_token_unfreeze_unknown_contract_returns_error() {
        let platform_version = PlatformVersion::latest();

        use dpp::state_transition::batch_transition::batched_transition::token_unfreeze_transition::v0::TokenUnfreezeTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::token_unfreeze_transition::TokenUnfreezeTransition;
        use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
        use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
        use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
        use dpp::state_transition::batch_transition::BatchTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV1;

        let token_base = TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: 1,
            token_contract_position: 0,
            data_contract_id: dpp::prelude::Identifier::from([81u8; 32]),
            token_id: dpp::prelude::Identifier::from([82u8; 32]),
            using_group_info: None,
        });
        let token_transition =
            TokenTransition::Unfreeze(TokenUnfreezeTransition::V0(TokenUnfreezeTransitionV0 {
                base: token_base,
                frozen_identity_id: dpp::prelude::Identifier::from([92u8; 32]),
                public_note: None,
            }));

        let st = StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
            owner_id: dpp::prelude::Identifier::from([5u8; 32]),
            transitions: vec![BatchedTransition::Token(token_transition)],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        let err = result.expect_err("expected UnknownContract, got Ok");
        assert!(
            matches!(
                err,
                crate::error::Error::Proof(ProofError::UnknownContract(_))
            ),
            "expected Error::Proof(UnknownContract), got: {:?}",
            err
        );
    }

    // --- Batch V1 with Token DestroyFrozenFunds transition + unknown contract.
    #[test]
    fn verify_batch_token_destroy_frozen_funds_unknown_contract_returns_error() {
        let platform_version = PlatformVersion::latest();

        use dpp::state_transition::batch_transition::batched_transition::token_destroy_frozen_funds_transition::v0::TokenDestroyFrozenFundsTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::token_destroy_frozen_funds_transition::TokenDestroyFrozenFundsTransition;
        use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
        use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
        use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
        use dpp::state_transition::batch_transition::BatchTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV1;

        let token_base = TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: 1,
            token_contract_position: 0,
            data_contract_id: dpp::prelude::Identifier::from([101u8; 32]),
            token_id: dpp::prelude::Identifier::from([102u8; 32]),
            using_group_info: None,
        });
        let token_transition = TokenTransition::DestroyFrozenFunds(
            TokenDestroyFrozenFundsTransition::V0(TokenDestroyFrozenFundsTransitionV0 {
                base: token_base,
                frozen_identity_id: dpp::prelude::Identifier::from([103u8; 32]),
                public_note: None,
            }),
        );

        let st = StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
            owner_id: dpp::prelude::Identifier::from([6u8; 32]),
            transitions: vec![BatchedTransition::Token(token_transition)],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        let err = result.expect_err("expected UnknownContract, got Ok");
        assert!(
            matches!(
                err,
                crate::error::Error::Proof(ProofError::UnknownContract(_))
            ),
            "expected Error::Proof(UnknownContract), got: {:?}",
            err
        );
    }

    // --- Batch V1 with Token Claim transition + unknown contract.
    #[test]
    fn verify_batch_token_claim_unknown_contract_returns_error() {
        let platform_version = PlatformVersion::latest();

        use dpp::state_transition::batch_transition::batched_transition::token_claim_transition::v0::TokenClaimTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::token_claim_transition::TokenClaimTransition;
        use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
        use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
        use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
        use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
        use dpp::state_transition::batch_transition::BatchTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV1;

        let token_base = TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: 1,
            token_contract_position: 0,
            data_contract_id: dpp::prelude::Identifier::from([111u8; 32]),
            token_id: dpp::prelude::Identifier::from([112u8; 32]),
            using_group_info: None,
        });
        let token_transition =
            TokenTransition::Claim(TokenClaimTransition::V0(TokenClaimTransitionV0 {
                base: token_base,
                distribution_type: TokenDistributionType::PreProgrammed,
                public_note: None,
            }));

        let st = StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
            owner_id: dpp::prelude::Identifier::from([7u8; 32]),
            transitions: vec![BatchedTransition::Token(token_transition)],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        let err = result.expect_err("expected UnknownContract, got Ok");
        assert!(
            matches!(
                err,
                crate::error::Error::Proof(ProofError::UnknownContract(_))
            ),
            "expected Error::Proof(UnknownContract), got: {:?}",
            err
        );
    }

    // --- Batch with a single document Replace transition + unknown contract
    // returns UnknownContract.
    #[test]
    fn verify_batch_document_replace_unknown_contract_returns_error() {
        let platform_version = PlatformVersion::latest();

        use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
        use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
        use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
        use dpp::state_transition::batch_transition::document_replace_transition::DocumentReplaceTransition;
        use dpp::state_transition::batch_transition::document_replace_transition::DocumentReplaceTransitionV0;
        use dpp::state_transition::batch_transition::BatchTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV0;

        let base = DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Default::default(),
            identity_contract_nonce: 1,
            document_type_name: "test".to_string(),
            data_contract_id: Default::default(),
        });

        let st = StateTransition::Batch(BatchTransition::V0(BatchTransitionV0 {
            owner_id: Default::default(),
            transitions: vec![DocumentTransition::Replace(DocumentReplaceTransition::V0(
                DocumentReplaceTransitionV0 {
                    base,
                    revision: 2,
                    data: Default::default(),
                },
            ))],
            ..Default::default()
        }));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        let err = result.expect_err("expected UnknownContract, got Ok");
        assert!(
            matches!(
                err,
                crate::error::Error::Proof(ProofError::UnknownContract(_))
            ),
            "expected Error::Proof(UnknownContract), got: {:?}",
            err
        );
    }

    // --- Batch with a single document Transfer transition + unknown contract.
    #[test]
    fn verify_batch_document_transfer_unknown_contract_returns_error() {
        let platform_version = PlatformVersion::latest();

        use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::DocumentTransferTransition;
        use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::DocumentTransferTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
        use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
        use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
        use dpp::state_transition::batch_transition::BatchTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV0;

        let base = DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Default::default(),
            identity_contract_nonce: 1,
            document_type_name: "test".to_string(),
            data_contract_id: Default::default(),
        });

        let st = StateTransition::Batch(BatchTransition::V0(BatchTransitionV0 {
            owner_id: Default::default(),
            transitions: vec![DocumentTransition::Transfer(
                DocumentTransferTransition::V0(DocumentTransferTransitionV0 {
                    base,
                    revision: 1,
                    recipient_owner_id: dpp::prelude::Identifier::from([77u8; 32]),
                }),
            )],
            ..Default::default()
        }));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        let err = result.expect_err("expected UnknownContract, got Ok");
        assert!(
            matches!(
                err,
                crate::error::Error::Proof(ProofError::UnknownContract(_))
            ),
            "expected Error::Proof(UnknownContract), got: {:?}",
            err
        );
    }

    // --- Batch with a single document UpdatePrice transition + unknown contract.
    #[test]
    fn verify_batch_document_update_price_unknown_contract_returns_error() {
        let platform_version = PlatformVersion::latest();

        use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
        use dpp::state_transition::batch_transition::batched_transition::document_update_price_transition::DocumentUpdatePriceTransition;
        use dpp::state_transition::batch_transition::batched_transition::document_update_price_transition::DocumentUpdatePriceTransitionV0;
        use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
        use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
        use dpp::state_transition::batch_transition::BatchTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV0;

        let base = DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Default::default(),
            identity_contract_nonce: 1,
            document_type_name: "test".to_string(),
            data_contract_id: Default::default(),
        });

        let st = StateTransition::Batch(BatchTransition::V0(BatchTransitionV0 {
            owner_id: Default::default(),
            transitions: vec![DocumentTransition::UpdatePrice(
                DocumentUpdatePriceTransition::V0(DocumentUpdatePriceTransitionV0 {
                    base,
                    revision: 1,
                    price: 500,
                }),
            )],
            ..Default::default()
        }));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        let err = result.expect_err("expected UnknownContract, got Ok");
        assert!(
            matches!(
                err,
                crate::error::Error::Proof(ProofError::UnknownContract(_))
            ),
            "expected Error::Proof(UnknownContract), got: {:?}",
            err
        );
    }

    // --- Batch with a single document Purchase transition + unknown contract.
    #[test]
    fn verify_batch_document_purchase_unknown_contract_returns_error() {
        let platform_version = PlatformVersion::latest();

        use dpp::state_transition::batch_transition::batched_transition::document_purchase_transition::DocumentPurchaseTransition;
        use dpp::state_transition::batch_transition::batched_transition::document_purchase_transition::DocumentPurchaseTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
        use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
        use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
        use dpp::state_transition::batch_transition::BatchTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV0;

        let base = DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Default::default(),
            identity_contract_nonce: 1,
            document_type_name: "test".to_string(),
            data_contract_id: Default::default(),
        });

        let st = StateTransition::Batch(BatchTransition::V0(BatchTransitionV0 {
            owner_id: Default::default(),
            transitions: vec![DocumentTransition::Purchase(
                DocumentPurchaseTransition::V0(DocumentPurchaseTransitionV0 {
                    base,
                    revision: 1,
                    price: 500,
                }),
            )],
            ..Default::default()
        }));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        let err = result.expect_err("expected UnknownContract, got Ok");
        assert!(
            matches!(
                err,
                crate::error::Error::Proof(ProofError::UnknownContract(_))
            ),
            "expected Error::Proof(UnknownContract), got: {:?}",
            err
        );
    }

    // --- Batch V1 with an empty transitions vec returns InvalidTransition
    // (covers the V1 batch "no transition" arm).
    #[test]
    fn verify_batch_v1_empty_transitions_returns_error() {
        let platform_version = PlatformVersion::latest();

        use dpp::state_transition::batch_transition::BatchTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV1;
        let st = StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
            owner_id: Default::default(),
            transitions: vec![],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        let err = result.expect_err("expected InvalidTransition, got Ok");
        match err {
            Error::Proof(ProofError::InvalidTransition(msg)) => {
                assert!(
                    msg.contains("no transition"),
                    "expected 'no transition' message, got: {msg}"
                );
            }
            other => panic!("expected InvalidTransition, got: {:?}", other),
        }
    }

    // --- Batch V1 with two transitions returns InvalidTransition (too-many arm).
    #[test]
    fn verify_batch_v1_too_many_transitions_returns_error() {
        let platform_version = PlatformVersion::latest();

        use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
        use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
        use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
        use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
        use dpp::state_transition::batch_transition::document_delete_transition::DocumentDeleteTransition;
        use dpp::state_transition::batch_transition::document_delete_transition::DocumentDeleteTransitionV0;
        use dpp::state_transition::batch_transition::BatchTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV1;

        let make_delete = |nonce: u64| -> BatchedTransition {
            let base = DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
                id: Default::default(),
                identity_contract_nonce: nonce,
                document_type_name: "x".to_string(),
                data_contract_id: Default::default(),
            });
            BatchedTransition::Document(DocumentTransition::Delete(DocumentDeleteTransition::V0(
                DocumentDeleteTransitionV0 { base },
            )))
        };

        let st = StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
            owner_id: Default::default(),
            transitions: vec![make_delete(1), make_delete(2)],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        let err = result.expect_err("expected InvalidTransition, got Ok");
        match err {
            Error::Proof(ProofError::InvalidTransition(msg)) => {
                assert!(
                    msg.contains("does not support more than one document"),
                    "expected too-many-document message, got: {msg}"
                );
            }
            other => panic!("expected InvalidTransition, got: {:?}", other),
        }
    }

    // --- IdentityCreditTransferToAddresses with address recipients returns
    // error for empty proof (exercises the recipient_addresses()-iteration arm).
    #[test]
    fn verify_identity_credit_transfer_to_addresses_with_recipients_empty_proof_errors() {
        let platform_version = PlatformVersion::latest();
        use dpp::address_funds::PlatformAddress;
        use dpp::state_transition::identity_credit_transfer_to_addresses_transition::v0::IdentityCreditTransferToAddressesTransitionV0;
        use dpp::state_transition::identity_credit_transfer_to_addresses_transition::IdentityCreditTransferToAddressesTransition;
        use std::collections::BTreeMap;

        let mut recipient_addresses = BTreeMap::new();
        recipient_addresses.insert(PlatformAddress::P2pkh([21u8; 20]), 100u64);

        let st = StateTransition::IdentityCreditTransferToAddresses(
            IdentityCreditTransferToAddressesTransition::V0(
                IdentityCreditTransferToAddressesTransitionV0 {
                    identity_id: dpp::prelude::Identifier::from([22u8; 32]),
                    recipient_addresses,
                    ..Default::default()
                },
            ),
        );

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected error for ICTTA with recipients empty proof, got: {:?}",
            result
        );
    }

    // --- Non-empty, malformed proof bytes → decode / verification error.
    //
    // Exercises the decoder path inside grove-db for a non-empty but
    // garbage proof, which should error out (either Error::GroveDB or
    // Error::Proof).
    #[test]
    fn verify_identity_create_garbage_proof_returns_error() {
        let platform_version = PlatformVersion::latest();
        use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
        use dpp::state_transition::state_transitions::identity::identity_create_transition::v0::IdentityCreateTransitionV0;
        let st = StateTransition::IdentityCreate(IdentityCreateTransition::V0(
            IdentityCreateTransitionV0 {
                identity_id: dpp::prelude::Identifier::random(),
                ..Default::default()
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        // A short, non-empty garbage proof triggers the decoder error path.
        let bad_proof: [u8; 8] = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11];

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &bad_proof,
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected error for garbage proof bytes, got: {:?}",
            result
        );
    }

    // --- DataContractCreate with a larger garbage proof returns an error.
    #[test]
    fn verify_data_contract_create_garbage_proof_returns_error() {
        let platform_version = PlatformVersion::latest();
        let created_contract =
            get_dpns_data_contract_fixture(None, 0, platform_version.protocol_version);
        let contract = created_contract.data_contract();
        let data_contract_serialized: DataContractInSerializationFormat = contract
            .clone()
            .try_into_platform_versioned(platform_version)
            .expect("expected to serialize contract");

        use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
        use dpp::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
        let st = StateTransition::DataContractCreate(DataContractCreateTransition::V0(
            DataContractCreateTransitionV0 {
                data_contract: data_contract_serialized,
                identity_nonce: 0,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);
        let garbage_proof: [u8; 32] = [0x5A; 32];

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &garbage_proof,
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected error for data contract create with garbage proof, got: {:?}",
            result
        );
    }

    // --- IdentityCreditTransfer: garbage proof returns error.
    #[test]
    fn verify_identity_credit_transfer_garbage_proof_returns_error() {
        let platform_version = PlatformVersion::latest();

        use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
        use dpp::state_transition::state_transitions::identity::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
        let st = StateTransition::IdentityCreditTransfer(IdentityCreditTransferTransition::V0(
            IdentityCreditTransferTransitionV0 {
                identity_id: dpp::prelude::Identifier::random(),
                recipient_id: dpp::prelude::Identifier::random(),
                amount: 50,
                nonce: 1,
                ..Default::default()
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);
        let garbage_proof = vec![0xFFu8; 20];

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &garbage_proof,
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected error for identity credit transfer with garbage proof, got: {:?}",
            result
        );
    }

    // --- IdentityCreate: garbage proof with a contract provider that errors
    // internally does not affect the Identity path (provider is not queried).
    // This sanity-checks that an IdentityCreate with an erroring provider
    // still fails on the proof (not on the provider call).
    #[test]
    fn verify_identity_create_erroring_provider_ignored_for_identity_flow() {
        let platform_version = PlatformVersion::latest();
        use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
        use dpp::state_transition::state_transitions::identity::identity_create_transition::v0::IdentityCreateTransitionV0;
        let st = StateTransition::IdentityCreate(IdentityCreateTransition::V0(
            IdentityCreateTransitionV0 {
                identity_id: dpp::prelude::Identifier::random(),
                ..Default::default()
            },
        ));

        // Provider returns an error, but the IdentityCreate branch never
        // consults the provider. The error surface is the empty proof.
        let known_contracts_provider_fn: &ContractLookupFn = &|_id| {
            Err(Error::Drive(
                crate::error::drive::DriveError::CorruptedDriveState(
                    "unused provider error".to_string(),
                ),
            ))
        };

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected proof/grovedb error (not provider error), got: {:?}",
            result
        );
    }

    // --- MasternodeVote: provider callback errors → error is propagated.
    #[test]
    fn verify_masternode_vote_provider_errors_is_propagated() {
        let platform_version = PlatformVersion::latest();
        use dpp::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
        use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
        use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
        use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
        use dpp::voting::vote_polls::VotePoll;
        use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
        use dpp::voting::votes::resource_vote::ResourceVote;
        use dpp::voting::votes::Vote;

        let st = StateTransition::MasternodeVote(MasternodeVoteTransition::V0(
            MasternodeVoteTransitionV0 {
                pro_tx_hash: dpp::prelude::Identifier::from([1u8; 32]),
                voter_identity_id: dpp::prelude::Identifier::from([2u8; 32]),
                vote: Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
                    vote_poll: VotePoll::ContestedDocumentResourceVotePoll(
                        ContestedDocumentResourceVotePoll {
                            contract_id: dpp::prelude::Identifier::from([7u8; 32]),
                            document_type_name: "some_type".to_string(),
                            index_name: "idx".to_string(),
                            index_values: vec![],
                        },
                    ),
                    resource_vote_choice: ResourceVoteChoice::Abstain,
                })),
                nonce: 1,
                signature_public_key_id: 0,
                signature: Default::default(),
            },
        ));

        // Provider returns Err — the error should be propagated as-is.
        let known_contracts_provider_fn: &ContractLookupFn = &|_id| {
            Err(Error::Drive(
                crate::error::drive::DriveError::CorruptedDriveState(
                    "synthetic provider failure".to_string(),
                ),
            ))
        };

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        let err = result.expect_err("expected provider error, got Ok");
        match err {
            Error::Drive(crate::error::drive::DriveError::CorruptedDriveState(msg)) => {
                assert_eq!(msg, "synthetic provider failure");
            }
            other => panic!("expected CorruptedDriveState, got: {:?}", other),
        }
    }

    // --- Batch with document transition: provider error is propagated.
    #[test]
    fn verify_batch_document_provider_error_is_propagated() {
        let platform_version = PlatformVersion::latest();

        use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
        use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
        use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
        use dpp::state_transition::batch_transition::document_delete_transition::DocumentDeleteTransition;
        use dpp::state_transition::batch_transition::document_delete_transition::DocumentDeleteTransitionV0;
        use dpp::state_transition::batch_transition::BatchTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV0;

        let base = DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Default::default(),
            identity_contract_nonce: 1,
            document_type_name: "x".to_string(),
            data_contract_id: Default::default(),
        });

        let st = StateTransition::Batch(BatchTransition::V0(BatchTransitionV0 {
            owner_id: Default::default(),
            transitions: vec![DocumentTransition::Delete(DocumentDeleteTransition::V0(
                DocumentDeleteTransitionV0 { base },
            ))],
            ..Default::default()
        }));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| {
            Err(Error::Drive(
                crate::error::drive::DriveError::CorruptedDriveState(
                    "synthetic batch provider failure".to_string(),
                ),
            ))
        };

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        let err = result.expect_err("expected provider error, got Ok");
        match err {
            Error::Drive(crate::error::drive::DriveError::CorruptedDriveState(msg)) => {
                assert_eq!(msg, "synthetic batch provider failure");
            }
            other => panic!("expected CorruptedDriveState, got: {:?}", other),
        }
    }

    // --- DataContractCreate empty proof + keeps_history=true: exercises the
    // happy-path arm up to proof verification but with an empty proof so it
    // errors out.
    #[test]
    fn verify_data_contract_create_keeps_history_empty_proof_returns_error() {
        let platform_version = PlatformVersion::latest();
        // Build a contract with keeps_history=true
        use dpp::data_contract::config::v0::DataContractConfigV0;
        use dpp::data_contract::config::DataContractConfig;
        use dpp::data_contract::v1::DataContractV1;
        use dpp::prelude::DataContract;
        let contract = DataContract::V1(DataContractV1 {
            id: dpp::prelude::Identifier::from([99u8; 32]),
            version: 1,
            owner_id: Default::default(),
            document_types: Default::default(),
            config: DataContractConfig::V0(DataContractConfigV0 {
                can_be_deleted: false,
                readonly: false,
                keeps_history: true,
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
            tokens: Default::default(),
            keywords: Vec::new(),
            description: None,
        });
        let data_contract_serialized: DataContractInSerializationFormat = contract
            .clone()
            .try_into_platform_versioned(platform_version)
            .expect("expected to serialize contract");
        use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
        use dpp::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
        let st = StateTransition::DataContractCreate(DataContractCreateTransition::V0(
            DataContractCreateTransitionV0 {
                data_contract: data_contract_serialized,
                identity_nonce: 0,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected error for keeps_history contract create with empty proof, got: {:?}",
            result
        );
    }

    // --- DataContractUpdate keeps_history=true + empty proof: errors out.
    #[test]
    fn verify_data_contract_update_keeps_history_empty_proof_returns_error() {
        let platform_version = PlatformVersion::latest();
        use dpp::data_contract::config::v0::DataContractConfigV0;
        use dpp::data_contract::config::DataContractConfig;
        use dpp::data_contract::v1::DataContractV1;
        use dpp::prelude::DataContract;
        let contract = DataContract::V1(DataContractV1 {
            id: dpp::prelude::Identifier::from([98u8; 32]),
            version: 2,
            owner_id: Default::default(),
            document_types: Default::default(),
            config: DataContractConfig::V0(DataContractConfigV0 {
                can_be_deleted: false,
                readonly: false,
                keeps_history: true,
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
            tokens: Default::default(),
            keywords: Vec::new(),
            description: None,
        });
        let data_contract_serialized: DataContractInSerializationFormat = contract
            .clone()
            .try_into_platform_versioned(platform_version)
            .expect("expected to serialize contract");
        use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0;
        let st = StateTransition::DataContractUpdate(DataContractUpdateTransition::V0(
            DataContractUpdateTransitionV0 {
                identity_contract_nonce: 1,
                data_contract: data_contract_serialized,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            },
        ));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(
            matches!(
                result,
                Err(crate::error::Error::Proof(_)) | Err(crate::error::Error::GroveDB(_))
            ),
            "expected error for keeps_history contract update with empty proof, got: {:?}",
            result
        );
    }

    // --- Batch V0 with a Document Create transition in contested status
    // + unknown contract returns UnknownContract. The contested_status arm
    // is a distinct branch that other tests don't cover.
    #[test]
    fn verify_batch_document_contested_create_unknown_contract_returns_error() {
        let platform_version = PlatformVersion::latest();

        use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
        use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
        use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
        use dpp::state_transition::batch_transition::document_create_transition::DocumentCreateTransition;
        use dpp::state_transition::batch_transition::document_create_transition::DocumentCreateTransitionV0;
        use dpp::state_transition::batch_transition::BatchTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV0;

        let base = DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Default::default(),
            identity_contract_nonce: 1,
            document_type_name: "t".to_string(),
            data_contract_id: Default::default(),
        });

        let create_transition = DocumentCreateTransition::V0(DocumentCreateTransitionV0 {
            base,
            entropy: [0u8; 32],
            data: Default::default(),
            // Contested (prefunded) create → separate branch in verify()
            prefunded_voting_balance: Some(("dash".to_string(), 1000)),
        });

        let st = StateTransition::Batch(BatchTransition::V0(BatchTransitionV0 {
            owner_id: Default::default(),
            transitions: vec![DocumentTransition::Create(create_transition)],
            ..Default::default()
        }));

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        let err = result.expect_err("expected UnknownContract, got Ok");
        assert!(
            matches!(
                err,
                crate::error::Error::Proof(ProofError::UnknownContract(_))
            ),
            "expected Error::Proof(UnknownContract), got: {:?}",
            err
        );
    }

    // --- Batch with a single Token transition + unknown contract returns
    // UnknownContract error (covers the token transition branch).
    #[test]
    fn verify_batch_token_transition_unknown_contract_returns_error() {
        let platform_version = PlatformVersion::latest();

        use dpp::state_transition::batch_transition::batched_transition::token_burn_transition::v0::TokenBurnTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::token_burn_transition::TokenBurnTransition;
        use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
        use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
        use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
        use dpp::state_transition::batch_transition::BatchTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV1;

        let token_base = TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: 1,
            token_contract_position: 0,
            data_contract_id: dpp::prelude::Identifier::from([55u8; 32]),
            token_id: dpp::prelude::Identifier::from([66u8; 32]),
            using_group_info: None,
        });
        let token_transition =
            TokenTransition::Burn(TokenBurnTransition::V0(TokenBurnTransitionV0 {
                base: token_base,
                burn_amount: 42,
                public_note: None,
            }));

        // V1 supports BatchedTransition::Token. V0 only supports document
        // transitions — use V1 to include a token transition.
        let st = StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
            owner_id: dpp::prelude::Identifier::from([9u8; 32]),
            transitions: vec![BatchedTransition::Token(token_transition)],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }));

        // Provider returns None — triggers UnknownContract for the token.
        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            platform_version,
        );

        let err = result.expect_err("expected UnknownContract error, got Ok");
        match err {
            crate::error::Error::Proof(ProofError::UnknownContract(msg)) => {
                assert!(
                    msg.contains("unknown contract") && msg.contains("token verification"),
                    "unexpected UnknownContract message: {msg}"
                );
            }
            other => panic!("expected Error::Proof(UnknownContract), got {:?}", other),
        }
    }
}
