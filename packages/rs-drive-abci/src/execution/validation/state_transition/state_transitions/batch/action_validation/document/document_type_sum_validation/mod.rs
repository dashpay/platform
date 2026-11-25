use std::collections::BTreeMap;

use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::document::document_sum_overflow_error::DocumentSumOverflowError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::platform_value::Value;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{
    DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0,
};

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::platform_types::platform::PlatformStateRef;

/// Holds the total a document type keeps of its summed property (`documentsSummable` or
/// `documentsAverageable`) to what the type's sums can hold, for the document create and
/// replace state validation of protocol version 15.
pub(crate) trait DocumentTypeSumValidation {
    /// Refuses a write that raises the type's total of its summed property past the most the
    /// type's sums hold: `i64::MAX` over the entries one document adds to a summed index (1
    /// unless an index keys the document by the elements of a typed array or the windows of
    /// a range grid). Each node of a sum tree holds the sum of its own branch; with values
    /// that are never negative, which registration requires of a property whose values may
    /// be large, no branch of any of the type's sums holds more than that total times those
    /// entries, so a write the total admits is one GroveDB applies.
    ///
    /// `written` is the properties the write stores. `replaced` is `None` for a create, and
    /// for a replace the stored value of each property the replace changes, so a replace
    /// that leaves the summed property as it was reads nothing. A write that does not raise
    /// the total reads nothing either: a type registered before the rule may hold a total
    /// above the limit, and stays writable downward.
    ///
    /// A type that keeps no total is not checked. From protocol version 15 registration
    /// admits one only with summed values within
    /// `SystemLimits::max_summed_value_magnitude_without_type_total`, which keep its sums in
    /// range by themselves.
    #[allow(clippy::too_many_arguments)]
    fn validate_document_type_sum(
        &self,
        written: &BTreeMap<String, Value>,
        replaced: Option<&BTreeMap<String, Value>>,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentTypeSumValidation for DocumentBaseTransitionAction {
    fn validate_document_type_sum(
        &self,
        written: &BTreeMap<String, Value>,
        replaced: Option<&BTreeMap<String, Value>>,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let document_type = self.document_type()?;
        let stored = match (replaced, document_type.documents_summable()) {
            (Some(replaced), Some(summed_property)) => match replaced.get(summed_property) {
                // The replace leaves the property as it was.
                None => return Ok(SimpleConsensusValidationResult::new()),
                stored => stored,
            },
            _ => None,
        };
        validate_summed_value_within_type_total(
            &self.data_contract_fetch_info_ref().contract,
            document_type,
            written,
            stored,
            platform.drive,
            block_info,
            transaction,
            execution_context,
            platform_version,
        )
    }
}

/// The check behind [`DocumentTypeSumValidation`], for every write that puts a document's
/// summed value in its type's sums: a create or a replace of a batch, and a moderator's
/// restore.
///
/// `written` is the properties the write stores. `stored` is the value of the summed
/// property the write takes out of the sums: `None` for a write that adds a document, and
/// for a write over a stored document the value that document holds.
#[allow(clippy::too_many_arguments)]
pub(crate) fn validate_summed_value_within_type_total(
    contract: &DataContract,
    document_type: DocumentTypeRef,
    written: &BTreeMap<String, Value>,
    stored: Option<&Value>,
    drive: &Drive,
    block_info: &BlockInfo,
    transaction: TransactionArg,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    let Some(summed_property) = document_type.documents_summable() else {
        return Ok(SimpleConsensusValidationResult::new());
    };
    // The property is required, so structure validation has refused a write without it.
    let Some(written_value) = written.get(summed_property) else {
        return Ok(SimpleConsensusValidationResult::new());
    };
    let removed = match stored {
        None => 0,
        Some(stored_value) => stored_value.to_integer::<i64>().map_err(|_| {
            Error::Execution(ExecutionError::CorruptedCodeExecution(
                "a stored document's summed property is not a value its sums hold",
            ))
        })?,
    };
    let entries = document_type
        .index_structure()
        .max_summed_entries_per_document();
    let limit = i64::MAX / i64::try_from(entries).unwrap_or(i64::MAX).max(1);
    let refusal = || {
        Ok(SimpleConsensusValidationResult::new_with_error(
            DocumentSumOverflowError::new(
                contract.id(),
                document_type.name().clone(),
                summed_property.to_string(),
                limit,
            )
            .into(),
        ))
    };
    // A value no sum holds, which only a type registered before unsigned 64-bit summed
    // properties were refused can admit.
    let Ok(added) = written_value.to_integer::<i64>() else {
        return refusal();
    };
    if added <= removed {
        return Ok(SimpleConsensusValidationResult::new());
    }

    let (total, fee_result) = drive.fetch_document_type_sum(
        contract,
        document_type,
        Some(&block_info.epoch),
        transaction,
        platform_version,
    )?;
    execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

    if total - i128::from(removed) + i128::from(added) > i128::from(limit) {
        return refusal();
    }
    Ok(SimpleConsensusValidationResult::new())
}
