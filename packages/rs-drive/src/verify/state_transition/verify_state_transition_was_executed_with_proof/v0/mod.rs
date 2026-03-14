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
use dpp::identity::PartialIdentity;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::prelude::{AddressNonce, Identifier};
use dpp::state_transition::address_credit_withdrawal_transition::accessors::AddressCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::identity_create_from_addresses_transition::accessors::IdentityCreateFromAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_to_addresses_transition::accessors::IdentityCreditTransferToAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
use dpp::state_transition::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::identity_topup_transition::accessors::IdentityTopUpTransitionAccessorsV0;
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
use dpp::state_transition::masternode_vote_transition::accessors::MasternodeVoteTransitionAccessorsV0;
use dpp::state_transition::proof_result::StateTransitionProofResult;
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
    ) -> Result<(RootHash, StateTransitionProofResult), Error> {
        match state_transition {
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
                    false,
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
                // Verify balances for output addresses after funding
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
                // Verify balances for input addresses after withdrawal
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
                use dpp::state_transition::proof_result::StateTransitionProofResult::VerifiedShieldedNullifiersWithAddressInfos;
                use dpp::state_transition::unshield_transition::accessors::UnshieldTransitionAccessorsV0;

                let nullifier_keys: Vec<Vec<u8>> = st.nullifiers();

                let (root_hash_nf, statuses) = Drive::verify_shielded_nullifiers(
                    proof,
                    &nullifier_keys,
                    true,
                    platform_version,
                )?;

                for (nf, is_spent) in &statuses {
                    if !is_spent {
                        return Err(Error::Proof(ProofError::IncorrectProof(format!(
                            "nullifier {} was not found as spent in the unshield proof",
                            hex::encode(nf)
                        ))));
                    }
                }

                let (root_hash_addr, balances): (
                    RootHash,
                    BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>,
                ) = Drive::verify_addresses_infos(
                    proof,
                    std::iter::once(st.output_address()),
                    true,
                    platform_version,
                )?;

                if root_hash_nf != root_hash_addr {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "unshield proof root hashes do not match between nullifiers and address"
                            .to_string(),
                    )));
                }

                Ok((
                    root_hash_nf,
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
                use dpp::data_contracts::withdrawals_contract;
                use dpp::data_contracts::withdrawals_contract::v1::document_types::withdrawal;
                use dpp::document::Document;
                use dpp::state_transition::proof_result::StateTransitionProofResult::VerifiedShieldedNullifiersWithWithdrawalDocument;
                use dpp::state_transition::shielded_withdrawal_transition::accessors::ShieldedWithdrawalTransitionAccessorsV0;

                let nullifier_keys: Vec<Vec<u8>> = st.nullifiers();

                let (root_hash_nf, statuses) = Drive::verify_shielded_nullifiers(
                    proof,
                    &nullifier_keys,
                    true,
                    platform_version,
                )?;

                for (nf, is_spent) in &statuses {
                    if !is_spent {
                        return Err(Error::Proof(ProofError::IncorrectProof(format!(
                            "nullifier {} was not found as spent in the shielded withdrawal proof",
                            hex::encode(nf)
                        ))));
                    }
                }

                // Compute withdrawal document ID deterministically (same as prove side)
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

                let (root_hash_doc, maybe_doc) =
                    doc_query.verify_proof(true, proof, document_type, platform_version)?;

                if root_hash_nf != root_hash_doc {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "shielded withdrawal proof root hashes do not match between nullifiers and document"
                            .to_string(),
                    )));
                }

                let doc = maybe_doc.ok_or_else(|| {
                    Error::Proof(ProofError::CorruptedProof(
                        "shielded withdrawal was executed but withdrawal document is missing from proof".to_string(),
                    ))
                })?;
                let documents = BTreeMap::from([(document_id, Some(doc))]);

                Ok((
                    root_hash_nf,
                    VerifiedShieldedNullifiersWithWithdrawalDocument(statuses, documents),
                ))
            }
            StateTransition::ShieldFromAssetLock(st) => {
                use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
                use dpp::asset_lock::StoredAssetLockInfo;
                use dpp::identity::state_transition::AssetLockProved;
                use dpp::serialization::PlatformDeserializable;
                use dpp::state_transition::proof_result::StateTransitionProofResult::VerifiedAssetLockConsumed;
                use grovedb::Element;

                let outpoint = st.asset_lock_proof().out_point().ok_or_else(|| {
                    Error::Proof(ProofError::InvalidTransition(
                        "shield from asset lock has no outpoint".to_string(),
                    ))
                })?;
                let outpoint_bytes: [u8; 36] = outpoint.into();

                // Build the same PathQuery as the prove side
                let mut query = grovedb::Query::new();
                query.insert_key(outpoint_bytes.to_vec());
                let path_query = grovedb::PathQuery::new(
                    vec![vec![72u8]], // RootTree::SpentAssetLockTransactions
                    grovedb::SizedQuery::new(query, Some(1), None),
                );

                let (root_hash, mut proved_key_values) =
                    grovedb::GroveDb::verify_query_with_absence_proof(
                        proof,
                        &path_query,
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
                                "expected an item element for asset lock outpoint".to_string(),
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
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::prelude::DataContract;
    use dpp::state_transition::proof_result::StateTransitionProofResult;
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
        let st = StateTransition::DataContractCreate(
            DataContractCreateTransition::V0(DataContractCreateTransitionV0 {
                data_contract: data_contract_serialized,
                identity_nonce: 0,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }),
        );

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &proof,
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(result.is_ok(), "expected verification to succeed, got: {:?}", result.err());
        let (root_hash, proof_result) = result.unwrap();
        assert_ne!(root_hash, [0u8; 32], "root hash should not be all zeros");
        match proof_result {
            StateTransitionProofResult::VerifiedDataContract(verified_contract) => {
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

        use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
        use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0;
        let st = StateTransition::DataContractUpdate(
            DataContractUpdateTransition::V0(DataContractUpdateTransitionV0 {
                identity_contract_nonce: 0,
                data_contract: data_contract_serialized,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }),
        );

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &proof,
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(result.is_ok(), "expected verification to succeed, got: {:?}", result.err());
        let (_root_hash, proof_result) = result.unwrap();
        match proof_result {
            StateTransitionProofResult::VerifiedDataContract(verified_contract) => {
                assert_eq!(verified_contract.id(), contract.id());
            }
            other => panic!("expected VerifiedDataContract, got {:?}", other),
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

        assert!(result.is_ok(), "expected verification to succeed, got: {:?}", result.err());
        let (_root_hash, proof_result) = result.unwrap();
        match proof_result {
            StateTransitionProofResult::VerifiedIdentity(verified_identity) => {
                assert_eq!(verified_identity.id(), identity.id());
            }
            other => panic!("expected VerifiedIdentity, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // IdentityTopUp
    // -----------------------------------------------------------------------

    #[test]
    fn verify_identity_top_up_happy_path() {
        let (drive, identity) = setup_drive_and_identity();
        let platform_version = PlatformVersion::latest();
        let identity_id = identity.id().to_buffer();

        let proof = drive
            .prove_identity_balance_and_revision(
                identity_id,
                None,
                &platform_version.drive,
            )
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

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &proof,
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(result.is_ok(), "expected verification to succeed, got: {:?}", result.err());
        let (_root_hash, proof_result) = result.unwrap();
        match proof_result {
            StateTransitionProofResult::VerifiedPartialIdentity(partial_identity) => {
                assert_eq!(partial_identity.id, identity.id());
                assert!(
                    partial_identity.balance.is_some(),
                    "balance should be present"
                );
                assert!(
                    partial_identity.revision.is_some(),
                    "revision should be present"
                );
            }
            other => panic!("expected VerifiedPartialIdentity, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // IdentityCreditWithdrawal
    // -----------------------------------------------------------------------

    #[test]
    fn verify_identity_credit_withdrawal_happy_path() {
        let (drive, identity) = setup_drive_and_identity();
        let platform_version = PlatformVersion::latest();
        let identity_id = identity.id().to_buffer();

        let proof = drive
            .prove_identity_balance(identity_id, None, &platform_version.drive)
            .expect("expected to prove identity balance");

        use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
        use dpp::state_transition::state_transitions::identity::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
        let st = StateTransition::IdentityCreditWithdrawal(
            IdentityCreditWithdrawalTransition::V0(IdentityCreditWithdrawalTransitionV0 {
                identity_id: identity.id(),
                amount: 100,
                ..Default::default()
            }),
        );

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &proof,
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(result.is_ok(), "expected verification to succeed, got: {:?}", result.err());
        let (_root_hash, proof_result) = result.unwrap();
        match proof_result {
            StateTransitionProofResult::VerifiedPartialIdentity(partial_identity) => {
                assert_eq!(partial_identity.id, identity.id());
                assert!(
                    partial_identity.balance.is_some(),
                    "balance should be present after withdrawal verification"
                );
            }
            other => panic!("expected VerifiedPartialIdentity, got {:?}", other),
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
        let key_request =
            IdentityKeysRequest::new_all_keys_query(&identity_id, None);
        let keys_path_query = key_request.into_path_query();
        let revision_path_query =
            Drive::identity_revision_query(&identity_id);
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
                revision: 1,
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

        assert!(result.is_ok(), "expected verification to succeed, got: {:?}", result.err());
        let (_root_hash, proof_result) = result.unwrap();
        match proof_result {
            StateTransitionProofResult::VerifiedPartialIdentity(partial_identity) => {
                assert_eq!(partial_identity.id, identity.id());
                assert!(
                    !partial_identity.loaded_public_keys.is_empty(),
                    "loaded public keys should not be empty"
                );
            }
            other => panic!("expected VerifiedPartialIdentity, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // IdentityCreditTransfer
    // -----------------------------------------------------------------------

    #[test]
    fn verify_identity_credit_transfer_happy_path() {
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

        // The transfer verification calls verify_identity_balance_for_identity_id twice
        // with verify_subset_of_proof=true, so we need a merged proof of both balances.
        let proof = drive
            .prove_many_identity_balances(
                &[sender.id().to_buffer(), recipient.id().to_buffer()],
                None,
                &platform_version.drive,
            )
            .expect("expected to prove both balances");

        use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
        use dpp::state_transition::state_transitions::identity::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
        let st = StateTransition::IdentityCreditTransfer(
            IdentityCreditTransferTransition::V0(IdentityCreditTransferTransitionV0 {
                identity_id: sender.id(),
                recipient_id: recipient.id(),
                amount: 50,
                nonce: 1,
                ..Default::default()
            }),
        );

        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &proof,
            known_contracts_provider_fn,
            platform_version,
        );

        assert!(result.is_ok(), "expected verification to succeed, got: {:?}", result.err());
        let (_root_hash, proof_result) = result.unwrap();
        match proof_result {
            StateTransitionProofResult::VerifiedBalanceTransfer(
                sender_partial,
                recipient_partial,
            ) => {
                assert_eq!(sender_partial.id, sender.id());
                assert_eq!(recipient_partial.id, recipient.id());
                assert!(sender_partial.balance.is_some());
                assert!(recipient_partial.balance.is_some());
            }
            other => panic!("expected VerifiedBalanceTransfer, got {:?}", other),
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

        assert!(result.is_err());
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

        use dpp::state_transition::batch_transition::BatchTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
        use dpp::state_transition::batch_transition::document_delete_transition::DocumentDeleteTransition;
        use dpp::state_transition::batch_transition::document_delete_transition::DocumentDeleteTransitionV0;
        use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
        use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;

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

        assert!(result.is_err());
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
            .grove_get_proved_path_query(
                &path_query,
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to get proof");

        // Build a document delete batch transition
        use dpp::state_transition::batch_transition::BatchTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
        use dpp::state_transition::batch_transition::document_delete_transition::DocumentDeleteTransition;
        use dpp::state_transition::batch_transition::document_delete_transition::DocumentDeleteTransitionV0;
        use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
        use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;

        let base = DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: doc_id,
            identity_contract_nonce: 1,
            document_type_name: "preorder".to_string(),
            data_contract_id: contract.id(),
        });

        let st = StateTransition::Batch(BatchTransition::V0(BatchTransitionV0 {
            owner_id: Default::default(),
            transitions: vec![DocumentTransition::Delete(
                DocumentDeleteTransition::V0(DocumentDeleteTransitionV0 { base }),
            )],
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

        assert!(result.is_ok(), "expected verification to succeed, got: {:?}", result.err());
        let (_root_hash, proof_result) = result.unwrap();
        match proof_result {
            StateTransitionProofResult::VerifiedDocuments(docs) => {
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
                block_info.clone(),
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
            .grove_get_proved_path_query(
                &path_query,
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to get proof");

        // Build a document create batch transition
        use dpp::state_transition::batch_transition::BatchTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
        use dpp::state_transition::batch_transition::document_create_transition::DocumentCreateTransition;
        use dpp::state_transition::batch_transition::document_create_transition::DocumentCreateTransitionV0;
        use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
        use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;

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

        assert!(result.is_ok(), "expected verification to succeed, got: {:?}", result.err());
        let (_root_hash, proof_result) = result.unwrap();
        match proof_result {
            StateTransitionProofResult::VerifiedDocuments(docs) => {
                assert_eq!(docs.len(), 1, "expected exactly one document entry");
                let (returned_id, maybe_doc) = docs.into_iter().next().unwrap();
                assert_eq!(returned_id, doc_id);
                assert!(
                    maybe_doc.is_some(),
                    "document should exist after creation"
                );
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

        use dpp::state_transition::batch_transition::BatchTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
        use dpp::state_transition::batch_transition::document_delete_transition::DocumentDeleteTransition;
        use dpp::state_transition::batch_transition::document_delete_transition::DocumentDeleteTransitionV0;
        use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
        use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;

        let base = DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Default::default(),
            identity_contract_nonce: 1,
            document_type_name: "test".to_string(),
            data_contract_id: Default::default(),
        });

        let st = StateTransition::Batch(BatchTransition::V0(BatchTransitionV0 {
            owner_id: Default::default(),
            transitions: vec![DocumentTransition::Delete(
                DocumentDeleteTransition::V0(DocumentDeleteTransitionV0 { base }),
            )],
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

        assert!(result.is_err());
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
        let st = StateTransition::DataContractCreate(
            DataContractCreateTransition::V0(DataContractCreateTransitionV0 {
                data_contract: data_contract_serialized,
                identity_nonce: 0,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
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

        // Empty proof should cause a GroveDB/proof error
        assert!(
            result.is_err(),
            "expected error with empty proof, but got success"
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
            result.is_err(),
            "expected error with empty proof for identity create"
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
            result.is_err(),
            "expected error with empty proof for identity top up"
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
        let st = StateTransition::IdentityCreditWithdrawal(
            IdentityCreditWithdrawalTransition::V0(IdentityCreditWithdrawalTransitionV0 {
                identity_id: dpp::prelude::Identifier::random(),
                amount: 100,
                ..Default::default()
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
            result.is_err(),
            "expected error with empty proof for identity credit withdrawal"
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
            result.is_err(),
            "expected error with empty proof for identity update"
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
        let st = StateTransition::IdentityCreditTransfer(
            IdentityCreditTransferTransition::V0(IdentityCreditTransferTransitionV0 {
                identity_id: dpp::prelude::Identifier::random(),
                recipient_id: dpp::prelude::Identifier::random(),
                amount: 50,
                nonce: 1,
                ..Default::default()
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
            result.is_err(),
            "expected error with empty proof for identity credit transfer"
        );
    }
}
