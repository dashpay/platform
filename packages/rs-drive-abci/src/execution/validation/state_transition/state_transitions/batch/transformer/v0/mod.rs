use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::error::Error;
use crate::platform_types::platform::PlatformStateRef;
use dpp::consensus::basic::document::{DataContractNotPresentError, InvalidDocumentTypeError};
use dpp::consensus::basic::BasicError;

use dpp::consensus::state::document::document_not_found_error::DocumentNotFoundError;
use dpp::consensus::state::document::document_owner_id_mismatch_error::DocumentOwnerIdMismatchError;

use dpp::consensus::state::document::invalid_document_revision_error::InvalidDocumentRevisionError;
use dpp::consensus::state::state_error::StateError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;

use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::document::document_incorrect_purchase_price_error::DocumentIncorrectPurchasePriceError;
use dpp::consensus::state::document::document_not_for_sale_error::DocumentNotForSaleError;
use dpp::document::property_names::PRICE;
use dpp::document::{Document, DocumentV0Getters};
use dpp::fee::Credits;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::prelude::{Revision, UserFeeIncrease};
use dpp::validation::SimpleConsensusValidationResult;
use dpp::{consensus::ConsensusError, prelude::Identifier, validation::ConsensusValidationResult};
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::document_purchase_transition::v0::v0_methods::DocumentPurchaseTransitionV0Methods;
use dpp::state_transition::StateTransitionLike;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::DocumentCreateTransitionAction;
use drive::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
use drive::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::DocumentReplaceTransitionAction;
use drive::state_transition_action::batch::BatchTransitionAction;
use drive::state_transition_action::batch::v0::BatchTransitionActionV0;

use crate::execution::validation::state_transition::batch::state::v0::fetch_documents::fetch_documents_for_transitions_knowing_contract_and_document_type;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

use dpp::state_transition::batch_transition::batched_transition::document_replace_transition::v0::v0_methods::DocumentReplaceTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::v0::v0_methods::DocumentTransferTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::document_transition::{DocumentTransition, DocumentTransitionV0Methods};
use dpp::state_transition::batch_transition::batched_transition::document_update_price_transition::v0::v0_methods::DocumentUpdatePriceTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::token_transition::{TokenTransition, TokenTransitionV0Methods};
use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use drive::drive::contract::DataContractFetchInfo;
use drive::drive::Drive;
use drive::drive::subscriptions::DriveSubscriptionFilter;
use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use drive::state_transition_action::batch::batched_transition::document_transition::document_purchase_transition_action::DocumentPurchaseTransitionAction;
use drive::state_transition_action::batch::batched_transition::document_transition::document_transfer_transition_action::DocumentTransferTransitionAction;
use drive::state_transition_action::batch::batched_transition::document_transition::document_update_price_transition_action::DocumentUpdatePriceTransitionAction;
use drive::state_transition_action::batch::batched_transition::token_transition::token_burn_transition_action::TokenBurnTransitionAction;
use drive::state_transition_action::batch::batched_transition::token_transition::token_config_update_transition_action::TokenConfigUpdateTransitionAction;
use drive::state_transition_action::batch::batched_transition::token_transition::token_destroy_frozen_funds_transition_action::TokenDestroyFrozenFundsTransitionAction;
use drive::state_transition_action::batch::batched_transition::token_transition::token_emergency_action_transition_action::TokenEmergencyActionTransitionAction;
use drive::state_transition_action::batch::batched_transition::token_transition::token_freeze_transition_action::TokenFreezeTransitionAction;
use drive::state_transition_action::batch::batched_transition::token_transition::token_mint_transition_action::TokenMintTransitionAction;
use drive::state_transition_action::batch::batched_transition::token_transition::token_claim_transition_action::TokenClaimTransitionAction;
use drive::state_transition_action::batch::batched_transition::token_transition::token_direct_purchase_transition_action::TokenDirectPurchaseTransitionAction;
use drive::state_transition_action::batch::batched_transition::token_transition::token_set_price_for_direct_purchase_transition_action::TokenSetPriceForDirectPurchaseTransitionAction;
use drive::state_transition_action::batch::batched_transition::token_transition::token_transfer_transition_action::TokenTransferTransitionAction;
use drive::state_transition_action::batch::batched_transition::token_transition::token_unfreeze_transition_action::TokenUnfreezeTransitionAction;
use drive::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;
use drive::state_transition_action::transform_to_state_transition_action_result::TransformToStateTransitionActionResult;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

pub(in crate::execution::validation::state_transition::state_transitions::batch) trait BatchTransitionTransformerV0
{
    fn try_into_action_v0<'a>(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        full_validation: bool,
        // These are the filters that have already shown that this transition is a match
        passing_filters_for_transition: &[&'a DriveSubscriptionFilter],
        // These are the filters that might still pass, if the original passes
        requiring_original_filters_for_transition: &[&'a DriveSubscriptionFilter],
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
    ) -> Result<ConsensusValidationResult<TransformToStateTransitionActionResult<'a>>, Error>;
}

trait BatchTransitionInternalTransformerV0 {
    #[allow(clippy::too_many_arguments)]
    fn transform_document_transitions_within_contract_v0(
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        full_validation: bool,
        data_contract_id: &Identifier,
        owner_id: Identifier,
        document_transitions: &BTreeMap<&String, Vec<&DocumentTransition>>,
        user_fee_increase: UserFeeIncrease,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Vec<BatchedTransitionAction>>, Error>;
    #[allow(clippy::too_many_arguments)]
    fn transform_document_transitions_within_document_type_v0(
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        full_validation: bool,
        data_contract_fetch_info: Arc<DataContractFetchInfo>,
        document_type_name: &str,
        owner_id: Identifier,
        document_transitions: &[&DocumentTransition],
        user_fee_increase: UserFeeIncrease,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Vec<BatchedTransitionAction>>, Error>;
    #[allow(clippy::too_many_arguments)]
    fn transform_token_transitions_within_contract_v0(
        platform: &PlatformStateRef,
        data_contract_id: &Identifier,
        block_info: &BlockInfo,
        validate_against_state: bool,
        owner_id: Identifier,
        token_transitions: &[&TokenTransition],
        user_fee_increase: UserFeeIncrease,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Vec<BatchedTransitionAction>>, Error>;
    #[allow(clippy::too_many_arguments)]
    /// Transfer token transition
    fn transform_token_transition_v0(
        drive: &Drive,
        transaction: TransactionArg,
        block_info: &BlockInfo,
        validate_against_state: bool,
        data_contract_fetch_info: Arc<DataContractFetchInfo>,
        transition: &TokenTransition,
        owner_id: Identifier,
        user_fee_increase: UserFeeIncrease,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<BatchedTransitionAction>, Error>;
    /// The data contract can be of multiple difference versions
    #[allow(clippy::too_many_arguments)]
    fn transform_document_transition_v0(
        drive: &Drive,
        transaction: TransactionArg,
        full_validation: bool,
        block_info: &BlockInfo,
        data_contract_fetch_info: Arc<DataContractFetchInfo>,
        transition: &DocumentTransition,
        replaced_documents: &[Document],
        user_fee_increase: UserFeeIncrease,
        owner_id: Identifier,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<BatchedTransitionAction>, Error>;
    fn find_replaced_document_v0<'a>(
        document_transition: &'a DocumentTransition,
        fetched_documents: &'a [Document],
    ) -> ConsensusValidationResult<&'a Document>;
    fn check_ownership_of_old_replaced_document_v0(
        document_id: Identifier,
        fetched_document: &Document,
        owner_id: &Identifier,
    ) -> SimpleConsensusValidationResult;
    fn check_revision_is_bumped_by_one_during_replace_v0(
        transition_revision: Revision,
        document_id: Identifier,
        original_document: &Document,
    ) -> SimpleConsensusValidationResult;
}

impl BatchTransitionTransformerV0 for BatchTransition {
    fn try_into_action_v0<'a>(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        validate_against_state: bool,
        // These are the filters that have already shown that this transition is a match
        passing_filters_for_transition: &[&'a DriveSubscriptionFilter],
        // These are the filters that might still pass, if the original passes
        requiring_original_filters_for_transition: &[&'a DriveSubscriptionFilter],
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
    ) -> Result<ConsensusValidationResult<TransformToStateTransitionActionResult<'a>>, Error> {
        let owner_id = self.owner_id();
        let user_fee_increase = self.user_fee_increase();
        let platform_version = platform.state.current_platform_version()?;
        let mut document_transitions_by_contracts_and_types: BTreeMap<
            &Identifier,
            BTreeMap<&String, Vec<&DocumentTransition>>,
        > = BTreeMap::new();

        let mut token_transitions_by_contracts: BTreeMap<&Identifier, Vec<&TokenTransition>> =
            BTreeMap::new();

        // We want to validate by contract, and then for each document type within a contract
        for transition in self.transitions_iter() {
            match transition {
                BatchedTransitionRef::Document(document_transition) => {
                    let document_type = document_transition.base().document_type_name();
                    let data_contract_id = document_transition.base().data_contract_id_ref();

                    match document_transitions_by_contracts_and_types.entry(data_contract_id) {
                        Entry::Vacant(v) => {
                            v.insert(BTreeMap::from([(document_type, vec![document_transition])]));
                        }
                        Entry::Occupied(mut transitions_by_types_in_contract) => {
                            match transitions_by_types_in_contract
                                .get_mut()
                                .entry(document_type)
                            {
                                Entry::Vacant(v) => {
                                    v.insert(vec![document_transition]);
                                }
                                Entry::Occupied(mut o) => o.get_mut().push(document_transition),
                            }
                        }
                    }
                }
                BatchedTransitionRef::Token(token_transition) => {
                    let data_contract_id = token_transition.base().data_contract_id_ref();

                    match token_transitions_by_contracts.entry(data_contract_id) {
                        Entry::Vacant(v) => {
                            v.insert(vec![token_transition]);
                        }
                        Entry::Occupied(mut transitions_by_tokens_in_contract) => {
                            transitions_by_tokens_in_contract
                                .get_mut()
                                .push(token_transition)
                        }
                    }
                }
            }
        }

        let validation_result_documents = document_transitions_by_contracts_and_types
            .iter()
            .map(
                |(data_contract_id, document_transitions_by_document_type)| {
                    Self::transform_document_transitions_within_contract_v0(
                        platform,
                        block_info,
                        validate_against_state,
                        data_contract_id,
                        owner_id,
                        document_transitions_by_document_type,
                        user_fee_increase,
                        execution_context,
                        transaction,
                        platform_version,
                    )
                },
            )
            .collect::<Result<Vec<ConsensusValidationResult<Vec<BatchedTransitionAction>>>, Error>>(
            )?;

        let mut validation_result_tokens = token_transitions_by_contracts
            .iter()
            .map(|(data_contract_id, token_transitions)| {
                Self::transform_token_transitions_within_contract_v0(
                    platform,
                    data_contract_id,
                    block_info,
                    validate_against_state,
                    owner_id,
                    token_transitions,
                    user_fee_increase,
                    transaction,
                    execution_context,
                    platform_version,
                )
            })
            .collect::<Result<Vec<ConsensusValidationResult<Vec<BatchedTransitionAction>>>, Error>>(
            )?;

        let mut validation_results = validation_result_documents;

        validation_results.append(&mut validation_result_tokens);

        let validation_result = ConsensusValidationResult::flatten(validation_results);

        if validation_result.has_data() {
            let (transitions, errors) = validation_result.into_data_and_errors()?;
            let batch_transition_action = BatchTransitionActionV0 {
                owner_id,
                transitions,
                user_fee_increase,
            }
            .into();
            Ok(ConsensusValidationResult::new_with_data_and_errors(
                batch_transition_action,
                errors,
            ))
        } else {
            Ok(ConsensusValidationResult::new_with_errors(
                validation_result.errors,
            ))
        }
    }
}

impl BatchTransitionInternalTransformerV0 for BatchTransition {
    fn transform_token_transitions_within_contract_v0(
        platform: &PlatformStateRef,
        data_contract_id: &Identifier,
        block_info: &BlockInfo,
        validate_against_state: bool,
        owner_id: Identifier,
        token_transitions: &[&TokenTransition],
        user_fee_increase: UserFeeIncrease,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Vec<BatchedTransitionAction>>, Error> {
        let drive = platform.drive;
        // Data Contract must exist
        let Some(data_contract_fetch_info) = drive
            .get_contract_with_fetch_info_and_fee(
                data_contract_id.to_buffer(),
                None,
                false,
                transaction,
                platform_version,
            )?
            .1
        else {
            return Ok(ConsensusValidationResult::new_with_error(
                BasicError::DataContractNotPresentError(DataContractNotPresentError::new(
                    *data_contract_id,
                ))
                .into(),
            ));
        };

        let validation_result = token_transitions
            .iter()
            .map(|token_transition| {
                Self::transform_token_transition_v0(
                    platform.drive,
                    transaction,
                    block_info,
                    validate_against_state,
                    data_contract_fetch_info.clone(),
                    token_transition,
                    owner_id,
                    user_fee_increase,
                    execution_context,
                    platform_version,
                )
            })
            .collect::<Result<Vec<ConsensusValidationResult<BatchedTransitionAction>>, Error>>()?;
        let validation_result = ConsensusValidationResult::merge_many(validation_result);
        Ok(validation_result)
    }
    fn transform_document_transitions_within_contract_v0(
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        validate_against_state: bool,
        data_contract_id: &Identifier,
        owner_id: Identifier,
        document_transitions: &BTreeMap<&String, Vec<&DocumentTransition>>,
        user_fee_increase: UserFeeIncrease,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Vec<BatchedTransitionAction>>, Error> {
        let drive = platform.drive;
        // Data Contract must exist
        let Some(data_contract_fetch_info) = drive
            .get_contract_with_fetch_info_and_fee(
                data_contract_id.0 .0,
                None,
                false,
                transaction,
                platform_version,
            )?
            .1
        else {
            return Ok(ConsensusValidationResult::new_with_error(
                BasicError::DataContractNotPresentError(DataContractNotPresentError::new(
                    *data_contract_id,
                ))
                .into(),
            ));
        };

        let validation_result = document_transitions
            .iter()
            .map(|(document_type_name, document_transitions)| {
                Self::transform_document_transitions_within_document_type_v0(
                    platform,
                    block_info,
                    validate_against_state,
                    data_contract_fetch_info.clone(),
                    document_type_name,
                    owner_id,
                    document_transitions,
                    user_fee_increase,
                    execution_context,
                    transaction,
                    platform_version,
                )
            })
            .collect::<Result<Vec<ConsensusValidationResult<Vec<BatchedTransitionAction>>>, Error>>(
            )?;
        Ok(ConsensusValidationResult::flatten(validation_result))
    }

    fn transform_document_transitions_within_document_type_v0(
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        validate_against_state: bool,
        data_contract_fetch_info: Arc<DataContractFetchInfo>,
        document_type_name: &str,
        owner_id: Identifier,
        document_transitions: &[&DocumentTransition],
        user_fee_increase: UserFeeIncrease,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Vec<BatchedTransitionAction>>, Error> {
        // We use temporary execution context without dry run,
        // because despite the dryRun, we need to get the
        // data contract to proceed with following logic
        // let tmp_execution_context = StateTransitionExecutionContext::default_for_platform_version(platform_version)?;
        //
        // execution_context.add_operations(tmp_execution_context.operations_slice());

        let dry_run = false; //maybe re-enable

        let data_contract = &data_contract_fetch_info.contract;

        let Some(document_type) = data_contract.document_type_optional_for_name(document_type_name)
        else {
            return Ok(ConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(document_type_name.to_owned(), data_contract.id())
                    .into(),
            ));
        };

        let replace_and_transfer_transitions = document_transitions
            .iter()
            .filter(|transition| {
                matches!(
                    transition,
                    DocumentTransition::Replace(_)
                        | DocumentTransition::Transfer(_)
                        | DocumentTransition::Purchase(_)
                        | DocumentTransition::UpdatePrice(_)
                )
            })
            .copied()
            .collect::<Vec<_>>();

        // We fetch documents only for replace and transfer transitions
        // since we need them to create transition actions
        // Below we also perform state validation for replace and transfer transitions only
        // other transitions are validated in their validate_state functions
        // TODO: Think more about this architecture
        let fetched_documents_validation_result =
            fetch_documents_for_transitions_knowing_contract_and_document_type(
                platform.drive,
                data_contract,
                document_type,
                replace_and_transfer_transitions.as_slice(),
                transaction,
                platform_version,
            )?;

        if !fetched_documents_validation_result.is_valid() {
            return Ok(ConsensusValidationResult::new_with_errors(
                fetched_documents_validation_result.errors,
            ));
        }

        let replaced_documents = fetched_documents_validation_result.into_data()?;

        Ok(if !dry_run {
            let document_transition_actions_validation_result = document_transitions
                .iter()
                .map(|transition| {
                    // we validate every transition in this document type
                    Self::transform_document_transition_v0(
                        platform.drive,
                        transaction,
                        validate_against_state,
                        block_info,
                        data_contract_fetch_info.clone(),
                        transition,
                        &replaced_documents,
                        user_fee_increase,
                        owner_id,
                        execution_context,
                        platform_version,
                    )
                })
                .collect::<Result<Vec<ConsensusValidationResult<BatchedTransitionAction>>, Error>>(
                )?;

            let result = ConsensusValidationResult::merge_many(
                document_transition_actions_validation_result,
            );

            if !result.is_valid() {
                return Ok(result);
            }
            result
        } else {
            ConsensusValidationResult::default()
        })
    }

    /// The data contract can be of multiple difference versions
    fn transform_token_transition_v0(
        drive: &Drive,
        transaction: TransactionArg,
        block_info: &BlockInfo,
        validate_against_state: bool,
        data_contract_fetch_info: Arc<DataContractFetchInfo>,
        transition: &TokenTransition,
        owner_id: Identifier,
        user_fee_increase: UserFeeIncrease,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<BatchedTransitionAction>, Error> {
        let approximate_for_costs = !validate_against_state;
        match transition {
            TokenTransition::Burn(token_burn_transition) => {
                let (batched_action, fee_result) = TokenBurnTransitionAction::try_from_borrowed_token_burn_transition_with_contract_lookup(drive, owner_id, token_burn_transition, approximate_for_costs, transaction, block_info,user_fee_increase, |_identifier| {
                    Ok(data_contract_fetch_info.clone())
                }, platform_version)?;

                execution_context
                    .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

                Ok(batched_action)
            }
            TokenTransition::Mint(token_mint_transition) => {
                let (batched_action, fee_result) = TokenMintTransitionAction::try_from_borrowed_token_mint_transition_with_contract_lookup(drive, owner_id, token_mint_transition, approximate_for_costs, transaction, block_info, user_fee_increase, |_identifier| {
                    Ok(data_contract_fetch_info.clone())
                }, platform_version)?;

                execution_context
                    .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

                Ok(batched_action)
            }
            TokenTransition::Transfer(token_transfer_transition) => {
                let (token_transfer_action, fee_result) = TokenTransferTransitionAction::try_from_borrowed_token_transfer_transition_with_contract_lookup(drive, owner_id, token_transfer_transition, approximate_for_costs, transaction, block_info, |_identifier| {
                    Ok(data_contract_fetch_info.clone())
                }, platform_version)?;

                execution_context
                    .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

                let batched_action = BatchedTransitionAction::TokenAction(
                    TokenTransitionAction::TransferAction(token_transfer_action),
                );
                Ok(batched_action.into())
            }
            TokenTransition::Freeze(token_freeze_transition) => {
                let (batched_action, fee_result) = TokenFreezeTransitionAction::try_from_borrowed_token_freeze_transition_with_contract_lookup(drive, owner_id, token_freeze_transition, approximate_for_costs, transaction, block_info, user_fee_increase, |_identifier| {
                    Ok(data_contract_fetch_info.clone())
                }, platform_version)?;

                execution_context
                    .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

                Ok(batched_action)
            }
            TokenTransition::Unfreeze(token_unfreeze_transition) => {
                let (batched_action, fee_result) = TokenUnfreezeTransitionAction::try_from_borrowed_token_unfreeze_transition_with_contract_lookup(drive, owner_id, token_unfreeze_transition, approximate_for_costs, transaction, block_info, user_fee_increase, |_identifier| {
                    Ok(data_contract_fetch_info.clone())
                }, platform_version)?;

                execution_context
                    .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

                Ok(batched_action)
            }
            TokenTransition::DestroyFrozenFunds(destroy_frozen_funds) => {
                let (batched_action, fee_result) = TokenDestroyFrozenFundsTransitionAction::try_from_borrowed_token_destroy_frozen_funds_transition_with_contract_lookup(drive, owner_id, destroy_frozen_funds, approximate_for_costs, transaction, block_info, user_fee_increase, |_identifier| {
                    Ok(data_contract_fetch_info.clone())
                }, platform_version)?;

                execution_context
                    .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

                Ok(batched_action)
            }
            TokenTransition::EmergencyAction(emergency_action) => {
                let (batched_action, fee_result) = TokenEmergencyActionTransitionAction::try_from_borrowed_token_emergency_action_transition_with_contract_lookup(drive, owner_id, emergency_action, approximate_for_costs, transaction, block_info, user_fee_increase, |_identifier| {
                    Ok(data_contract_fetch_info.clone())
                }, platform_version)?;

                execution_context
                    .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

                Ok(batched_action)
            }
            TokenTransition::ConfigUpdate(token_config_update) => {
                let (batched_action, fee_result) = TokenConfigUpdateTransitionAction::try_from_borrowed_token_config_update_transition_with_contract_lookup(drive, owner_id, token_config_update, approximate_for_costs, transaction, block_info, user_fee_increase, |_identifier| {
                    Ok(data_contract_fetch_info.clone())
                }, platform_version)?;

                execution_context
                    .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

                Ok(batched_action)
            }
            TokenTransition::Claim(claim) => {
                let (batched_action, fee_result) = TokenClaimTransitionAction::try_from_borrowed_token_claim_transition_with_contract_lookup(drive, owner_id, claim, approximate_for_costs, transaction, block_info, user_fee_increase, |_identifier| {
                    Ok(data_contract_fetch_info.clone())
                }, platform_version)?;

                execution_context
                    .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

                Ok(batched_action)
            }
            TokenTransition::DirectPurchase(direct_purchase) => {
                let (batched_action, fee_result) = TokenDirectPurchaseTransitionAction::try_from_borrowed_token_direct_purchase_transition_with_contract_lookup(drive, owner_id, direct_purchase, approximate_for_costs, transaction, block_info, user_fee_increase, |_identifier| {
                    Ok(data_contract_fetch_info.clone())
                }, platform_version)?;

                execution_context
                    .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

                Ok(batched_action)
            }
            TokenTransition::SetPriceForDirectPurchase(set_price_for_direct_purchase) => {
                let (batched_action, fee_result) = TokenSetPriceForDirectPurchaseTransitionAction::try_from_borrowed_token_set_price_for_direct_purchase_transition_with_contract_lookup(drive, owner_id, set_price_for_direct_purchase, approximate_for_costs, transaction, block_info, user_fee_increase, |_identifier| {
                    Ok(data_contract_fetch_info.clone())
                }, platform_version)?;

                execution_context
                    .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

                Ok(batched_action)
            }
        }
    }

    /// The data contract can be of multiple difference versions
    fn transform_document_transition_v0<'a>(
        drive: &Drive,
        transaction: TransactionArg,
        validate_against_state: bool,
        block_info: &BlockInfo,
        data_contract_fetch_info: Arc<DataContractFetchInfo>,
        transition: &DocumentTransition,
        replaced_documents: &[Document],
        user_fee_increase: UserFeeIncrease,
        owner_id: Identifier,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<BatchedTransitionAction>, Error> {
        match transition {
            DocumentTransition::Create(document_create_transition) => {
                let (document_create_action, fee_result) = DocumentCreateTransitionAction::try_from_document_borrowed_create_transition_with_contract_lookup(
                    drive, owner_id, transaction,
                    document_create_transition, block_info, user_fee_increase, |_identifier| {
                        Ok(data_contract_fetch_info.clone())
                    }, platform_version)?;

                execution_context
                    .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));
                Ok(document_create_action)
            }
            DocumentTransition::Replace(document_replace_transition) => {
                let mut result = ConsensusValidationResult::<BatchedTransitionAction>::new();

                let validation_result =
                    Self::find_replaced_document_v0(transition, replaced_documents);

                if !validation_result.is_valid_with_data() {
                    // We can set the user fee increase to 0 here because it is decided by the Documents Batch instead
                    let bump_action =
                        BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition(
                            document_replace_transition.base(),
                            owner_id,
                            0,
                        );
                    let batched_action =
                        BatchedTransitionAction::BumpIdentityDataContractNonce(bump_action);

                    return Ok(ConsensusValidationResult::new_with_data_and_errors(
                        batched_action,
                        validation_result.errors,
                    ));
                }

                let original_document = validation_result.into_data()?;

                let validation_result = Self::check_ownership_of_old_replaced_document_v0(
                    document_replace_transition.base().id(),
                    original_document,
                    &owner_id,
                );

                if !validation_result.is_valid() {
                    result.merge(validation_result);
                    return Ok(result);
                }

                if validate_against_state {
                    //there are situations where we don't want to validate this against the state
                    // for example when we already applied the state transition action,
                    // and we are just validating it happened
                    let validation_result = Self::check_revision_is_bumped_by_one_during_replace_v0(
                        document_replace_transition.revision(),
                        document_replace_transition.base().id(),
                        original_document,
                    );

                    if !validation_result.is_valid() {
                        result.merge(validation_result);
                        return Ok(result);
                    }
                }

                let (document_replace_action, fee_result) =
                    DocumentReplaceTransitionAction::try_from_borrowed_document_replace_transition(
                        document_replace_transition,
                        owner_id,
                        original_document,
                        block_info,
                        user_fee_increase,
                        |_identifier| Ok(data_contract_fetch_info.clone()),
                    )?;

                execution_context
                    .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

                if result.is_valid() {
                    Ok(document_replace_action)
                } else {
                    Ok(result)
                }
            }
            DocumentTransition::Delete(document_delete_transition) => {
                let (batched_action, fee_result) = DocumentDeleteTransitionAction::try_from_document_borrowed_delete_transition_with_contract_lookup(document_delete_transition, owner_id, user_fee_increase, |_identifier| {
                    Ok(data_contract_fetch_info.clone())
                })?;

                execution_context
                    .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

                Ok(batched_action)
            }
            DocumentTransition::Transfer(document_transfer_transition) => {
                let mut result = ConsensusValidationResult::<BatchedTransitionAction>::new();

                let validation_result =
                    Self::find_replaced_document_v0(transition, replaced_documents);

                if !validation_result.is_valid_with_data() {
                    result.merge(validation_result);
                    return Ok(result);
                }

                let original_document = validation_result.into_data()?;

                let validation_result = Self::check_ownership_of_old_replaced_document_v0(
                    document_transfer_transition.base().id(),
                    original_document,
                    &owner_id,
                );

                if !validation_result.is_valid() {
                    result.merge(validation_result);
                    return Ok(result);
                }

                if validate_against_state {
                    //there are situations where we don't want to validate this against the state
                    // for example when we already applied the state transition action,
                    // and we are just validating it happened
                    let validation_result = Self::check_revision_is_bumped_by_one_during_replace_v0(
                        document_transfer_transition.revision(),
                        document_transfer_transition.base().id(),
                        original_document,
                    );

                    if !validation_result.is_valid() {
                        result.merge(validation_result);
                        return Ok(result);
                    }
                }

                let (document_transfer_action, fee_result) =
                    DocumentTransferTransitionAction::try_from_borrowed_document_transfer_transition(
                        document_transfer_transition,
                        owner_id,
                        original_document.clone(), //todo: remove clone
                        block_info,
                        user_fee_increase,
                        |_identifier| Ok(data_contract_fetch_info.clone()),
                    )?;

                execution_context
                    .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

                if result.is_valid() {
                    Ok(document_transfer_action)
                } else {
                    Ok(result)
                }
            }
            DocumentTransition::UpdatePrice(document_update_price_transition) => {
                let mut result = ConsensusValidationResult::<BatchedTransitionAction>::new();

                let validation_result =
                    Self::find_replaced_document_v0(transition, replaced_documents);

                if !validation_result.is_valid_with_data() {
                    result.merge(validation_result);
                    return Ok(result);
                }

                let original_document = validation_result.into_data()?;

                let validation_result = Self::check_ownership_of_old_replaced_document_v0(
                    document_update_price_transition.base().id(),
                    original_document,
                    &owner_id,
                );

                if !validation_result.is_valid() {
                    result.merge(validation_result);
                    return Ok(result);
                }

                if validate_against_state {
                    //there are situations where we don't want to validate this against the state
                    // for example when we already applied the state transition action,
                    // and we are just validating it happened
                    let validation_result = Self::check_revision_is_bumped_by_one_during_replace_v0(
                        document_update_price_transition.revision(),
                        document_update_price_transition.base().id(),
                        original_document,
                    );

                    if !validation_result.is_valid() {
                        result.merge(validation_result);
                        return Ok(result);
                    }
                }

                let (document_update_price_action, fee_result) =
                    DocumentUpdatePriceTransitionAction::try_from_borrowed_document_update_price_transition(
                        document_update_price_transition,
                        owner_id,
                        original_document.clone(), //todo: find a way to not have to use cloning
                        block_info,
                        user_fee_increase,
                        |_identifier| Ok(data_contract_fetch_info.clone()),
                    )?;

                execution_context
                    .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

                if result.is_valid() {
                    Ok(document_update_price_action)
                } else {
                    Ok(result)
                }
            }
            DocumentTransition::Purchase(document_purchase_transition) => {
                let mut result = ConsensusValidationResult::<BatchedTransitionAction>::new();

                let validation_result =
                    Self::find_replaced_document_v0(transition, replaced_documents);

                if !validation_result.is_valid_with_data() {
                    result.merge(validation_result);
                    return Ok(result);
                }

                let original_document = validation_result.into_data()?;

                let Some(listed_price) = original_document
                    .properties()
                    .get_optional_integer::<Credits>(PRICE)?
                else {
                    result.add_error(StateError::DocumentNotForSaleError(
                        DocumentNotForSaleError::new(original_document.id()),
                    ));
                    return Ok(result);
                };

                if listed_price != document_purchase_transition.price() {
                    result.add_error(StateError::DocumentIncorrectPurchasePriceError(
                        DocumentIncorrectPurchasePriceError::new(
                            original_document.id(),
                            document_purchase_transition.price(),
                            listed_price,
                        ),
                    ));
                    return Ok(result);
                }

                if validate_against_state {
                    //there are situations where we don't want to validate this against the state
                    // for example when we already applied the state transition action,
                    // and we are just validating it happened
                    let validation_result = Self::check_revision_is_bumped_by_one_during_replace_v0(
                        document_purchase_transition.revision(),
                        document_purchase_transition.base().id(),
                        original_document,
                    );

                    if !validation_result.is_valid() {
                        result.merge(validation_result);
                        return Ok(result);
                    }
                }

                let (document_purchase_action, fee_result) =
                    DocumentPurchaseTransitionAction::try_from_borrowed_document_purchase_transition(
                        document_purchase_transition,
                        owner_id,
                        original_document.clone(), //todo: find a way to not have to use cloning
                        owner_id,
                        block_info,
                        user_fee_increase,
                        |_identifier| Ok(data_contract_fetch_info.clone()),
                    )?;

                execution_context
                    .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

                if result.is_valid() {
                    Ok(document_purchase_action)
                } else {
                    Ok(result)
                }
            }
        }
    }

    fn find_replaced_document_v0<'a>(
        document_transition: &'a DocumentTransition,
        fetched_documents: &'a [Document],
    ) -> ConsensusValidationResult<&'a Document> {
        let maybe_fetched_document = fetched_documents
            .iter()
            .find(|d| d.id() == document_transition.base().id());

        if let Some(document) = maybe_fetched_document {
            ConsensusValidationResult::new_with_data(document)
        } else {
            ConsensusValidationResult::new_with_error(ConsensusError::StateError(
                StateError::DocumentNotFoundError(DocumentNotFoundError::new(
                    document_transition.base().id(),
                )),
            ))
        }
    }

    fn check_ownership_of_old_replaced_document_v0(
        document_id: Identifier,
        fetched_document: &Document,
        owner_id: &Identifier,
    ) -> SimpleConsensusValidationResult {
        let mut result = SimpleConsensusValidationResult::default();
        if fetched_document.owner_id() != owner_id {
            result.add_error(ConsensusError::StateError(
                StateError::DocumentOwnerIdMismatchError(DocumentOwnerIdMismatchError::new(
                    document_id,
                    owner_id.to_owned(),
                    fetched_document.owner_id(),
                )),
            ));
        }
        result
    }
    fn check_revision_is_bumped_by_one_during_replace_v0(
        transition_revision: Revision,
        document_id: Identifier,
        original_document: &Document,
    ) -> SimpleConsensusValidationResult {
        let mut result = SimpleConsensusValidationResult::default();

        // If there was no previous revision this means that the document_type is not update-able
        // However this should have been caught earlier
        let Some(previous_revision) = original_document.revision() else {
            result.add_error(ConsensusError::StateError(
                StateError::InvalidDocumentRevisionError(InvalidDocumentRevisionError::new(
                    document_id,
                    None,
                    transition_revision,
                )),
            ));
            return result;
        };
        // no need to check bounds here, because it would be impossible to hit the end on a u64
        let expected_revision = previous_revision + 1;
        if transition_revision != expected_revision {
            result.add_error(ConsensusError::StateError(
                StateError::InvalidDocumentRevisionError(InvalidDocumentRevisionError::new(
                    document_id,
                    Some(previous_revision),
                    transition_revision,
                )),
            ))
        }
        result
    }
}
