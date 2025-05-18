use std::collections::BTreeMap;
use dpp::balances::credits::TokenAmount;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_keeps_history_rules::accessors::v0::TokenKeepsHistoryRulesV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::document::{Document, DocumentV0Getters};
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::property_names::PRICE;
use dpp::fee::Credits;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identity::PartialIdentity;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::prelude::Identifier;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
use dpp::state_transition::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::identity_topup_transition::accessors::IdentityTopUpTransitionAccessorsV0;
use dpp::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use dpp::state_transition::{StateTransition, StateTransitionLike};
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
use dpp::state_transition::masternode_vote_transition::accessors::MasternodeVoteTransitionAccessorsV0;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::proof_result::StateTransitionProofResult::{VerifiedBalanceTransfer, VerifiedDataContract, VerifiedDocuments, VerifiedIdentity, VerifiedMasternodeVote, VerifiedPartialIdentity, VerifiedTokenActionWithDocument, VerifiedTokenBalance, VerifiedTokenGroupActionWithDocument, VerifiedTokenGroupActionWithTokenBalance, VerifiedTokenGroupActionWithTokenIdentityInfo, VerifiedTokenGroupActionWithTokenPricingSchedule, VerifiedTokenIdentitiesBalances, VerifiedTokenIdentityInfo, VerifiedTokenPricingSchedule};
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

                if !contract_for_serialization
                    .eq_without_auto_fields(data_contract_create.data_contract())
                {
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
                                    TokenTransition::Burn(_) => {
                                        Some(vec!["burnFromId", "note"])
                                    }
                                    TokenTransition::Claim(_) => Some(vec!["amount"]),
                                    | TokenTransition::Mint(_)
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
                            TokenTransition::Transfer(token_transfer_transition) => {
                                if keeps_historical_document.keeps_transfer_history() {
                                    historical_query()
                                } else {
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
        }
    }
}
