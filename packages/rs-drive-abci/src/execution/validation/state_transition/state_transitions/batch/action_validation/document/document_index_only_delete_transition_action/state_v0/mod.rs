use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::document::InvalidDocumentTypeError;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::document::document_not_found_error::DocumentNotFoundError;
use dpp::consensus::state::state_error::StateError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::identifier::Identifier;
use dpp::prelude::ConsensusValidationResult;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::fees::op::{FunctionOp, HashFunction, LowLevelDriveOperation};
use drive::grovedb::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_index_only_delete_transition_action::v0::DocumentIndexOnlyDeleteTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_index_only_delete_transition_action::DocumentIndexOnlyDeleteTransitionAction;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::batch::action_validation::document::document_base_transaction_action::DocumentBaseTransitionActionValidation;
use crate::platform_types::platform::PlatformStateRef;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait DocumentIndexOnlyDeleteTransitionActionStateValidationV0 {
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl DocumentIndexOnlyDeleteTransitionActionStateValidationV0
    for DocumentIndexOnlyDeleteTransitionAction
{
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let validation_result = self.base().validate_state(
            platform,
            owner_id,
            block_info,
            "indexOnlyDelete",
            execution_context,
            transaction,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        let contract_fetch_info = self.base().data_contract_fetch_info();

        let contract = &contract_fetch_info.contract;

        let document_type_name = self.base().document_type_name();

        let Some(document_type) = contract.document_type_optional_for_name(document_type_name)
        else {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(document_type_name.clone(), contract.id()).into(),
            ));
        };

        // An indexOnly document has no primary row to fetch. Reconstruct
        // the document from the carried values ONCE (this also fixes the
        // row commitment every probe compares against), then probe EVERY
        // index entry the values produce: each must exist AND carry that
        // commitment. Every index embeds $ownerId (ownership) and every
        // entry stores the commitment binding it to its document's full
        // tuple (row integrity), so a values tuple spliced from different
        // documents — even two by the same owner — fails a probe cleanly
        // here instead of mid-apply. These actions only exist for PV14+
        // indexOnly contracts, so this path is unreachable historically.
        let document = drive::drive::Drive::index_only_document_from_values(
            self.base().id(),
            owner_id,
            self.data().clone(),
        )
        .map_err(Error::Drive)?;

        // One double-SHA256 over the full tuple, shared by every probe —
        // billed as a function operation sized to its preimage alongside
        // the probes' grove reads.
        let (expected_commitment, preimage_size) =
            drive::drive::document::index_only_row_commitment_with_preimage_size(
                &document,
                document_type,
                platform_version,
            )
            .map_err(Error::Drive)?;

        let mut probe_operations = vec![LowLevelDriveOperation::FunctionOperation(
            FunctionOp::new_with_byte_count(HashFunction::Sha256_2, preimage_size),
        )];
        let mut missing_entry = false;
        for index in document_type.indexes().values() {
            let entry_exists = platform
                .drive
                .index_only_entry_commitment_matches(
                    contract.id(),
                    document_type,
                    index,
                    &document,
                    &expected_commitment,
                    transaction,
                    &mut probe_operations,
                    platform_version,
                )
                .map_err(Error::Drive)?;

            if !entry_exists {
                missing_entry = true;
                break;
            }
        }

        // Every probe is a stateful grove read (plus the row-commitment
        // hash) validators actually perform — bill them all, on the
        // not-found path included.
        let fee_result = drive::drive::Drive::calculate_fee(
            None,
            Some(probe_operations),
            &block_info.epoch,
            platform.drive.config.epochs_per_era,
            platform_version,
            None,
        )
        .map_err(Error::Drive)?;
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

        if missing_entry {
            return Ok(ConsensusValidationResult::new_with_error(
                ConsensusError::StateError(StateError::DocumentNotFoundError(
                    DocumentNotFoundError::new(self.base().id()),
                )),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
