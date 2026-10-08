use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::document_type::property_constraints::{AggregateRead, SystemChange};
use dpp::data_contract::DataContract;
use dpp::document::{Document, DocumentV0Getters};
use dpp::platform_value::{Identifier, Value};
use dpp::prelude::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::DocumentCreateTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_purchase_transition_action::DocumentPurchaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::DocumentReplaceTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_transfer_transition_action::DocumentTransferTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_update_price_transition_action::DocumentUpdatePriceTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use std::collections::{BTreeMap, BTreeSet};

/// A document version a write stores, replaces or deletes: its id, its properties and its
/// owner.
#[derive(Clone, Copy)]
struct DocumentVersion<'a> {
    id: Identifier,
    properties: &'a BTreeMap<String, Value>,
    owner_id: Identifier,
}

impl<'a> DocumentVersion<'a> {
    /// `document` as it stands, with its own owner.
    fn of(document: &'a Document) -> Self {
        DocumentVersion {
            id: document.id(),
            properties: document.properties(),
            owner_id: document.owner_id(),
        }
    }
}

/// A document version as a total reads it: its id, its properties as one map value, and its
/// owner.
#[derive(Clone, Copy)]
struct VersionData<'a> {
    id: Identifier,
    data: &'a Value,
    owner_id: Identifier,
}

/// Reads the `countOf` and `sumOf` totals the rules judging the write in `result`
/// read ([`read_property_constraint_aggregates`]) and hands them to its action: a
/// create or a replace of a document of `writer`'s, stored as `stored` before
/// (`None` for a create), or a transfer, a purchase or a price update of `stored`,
/// judged by the rules the change can break. Any other action, a refused write
/// included, reads nothing.
#[allow(clippy::too_many_arguments)]
pub(super) fn attach_property_constraint_aggregates(
    drive: &Drive,
    contract: &DataContract,
    result: &mut ConsensusValidationResult<BatchedTransitionAction>,
    stored: Option<&Document>,
    writer: Identifier,
    block_info: &BlockInfo,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<(), Error> {
    let Some(BatchedTransitionAction::DocumentAction(action)) = result.data.as_mut() else {
        return Ok(());
    };
    let stored = stored.map(DocumentVersion::of);
    let mut read = |document_type_name: &str, written: DocumentVersion, change| {
        read_property_constraint_aggregates(
            drive,
            contract,
            document_type_name,
            written,
            stored,
            change,
            block_info,
            execution_context,
            transaction,
            platform_version,
        )
    };
    match action {
        DocumentTransitionAction::CreateAction(action) => {
            let written = DocumentVersion {
                id: action.base().id(),
                properties: action.data(),
                owner_id: writer,
            };
            let aggregates = read(action.base().document_type_name(), written, None)?;
            action.set_property_constraint_aggregates(aggregates);
        }
        DocumentTransitionAction::ReplaceAction(action) => {
            let written = DocumentVersion {
                id: action.base().id(),
                properties: action.data(),
                owner_id: writer,
            };
            let aggregates = read(action.base().document_type_name(), written, None)?;
            action.set_property_constraint_aggregates(aggregates);
        }
        // The action's document carries its new owner
        DocumentTransitionAction::TransferAction(action) => {
            let aggregates = read(
                action.base().document_type_name(),
                DocumentVersion::of(action.document()),
                Some(SystemChange::Transfer),
            )?;
            action.set_property_constraint_aggregates(aggregates);
        }
        DocumentTransitionAction::PurchaseAction(action) => {
            let aggregates = read(
                action.base().document_type_name(),
                DocumentVersion::of(action.document()),
                Some(SystemChange::Transfer),
            )?;
            action.set_property_constraint_aggregates(aggregates);
        }
        DocumentTransitionAction::UpdatePriceAction(action) => {
            let aggregates = read(
                action.base().document_type_name(),
                DocumentVersion::of(action.document()),
                Some(SystemChange::PriceUpdate),
            )?;
            action.set_property_constraint_aggregates(aggregates);
        }
        DocumentTransitionAction::DeleteAction(_)
        | DocumentTransitionAction::IndexOnlyDeleteAction(_) => {}
    }
    Ok(())
}

/// Reads the `countOf` and `sumOf` totals the `propertyConstraints` rules of the document type
/// `document_type_name` read for a moderator's field change, which stores `written` in place of
/// `stored`, the same document under the same owner: judged as a replace is, by every rule.
#[allow(clippy::too_many_arguments)]
pub(crate) fn read_property_constraint_aggregates_for_moderator_change(
    drive: &Drive,
    contract: &DataContract,
    document_type_name: &str,
    written: &Document,
    stored: &Document,
    block_info: &BlockInfo,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<BTreeMap<AggregateRead, i128>, Error> {
    read_property_constraint_aggregates(
        drive,
        contract,
        document_type_name,
        DocumentVersion::of(written),
        Some(DocumentVersion::of(stored)),
        None,
        block_info,
        execution_context,
        transaction,
        platform_version,
    )
}

/// Reads from state the `countOf` and `sumOf` totals the `propertyConstraints` rules of
/// the document type `document_type_name` read, for a write storing `written` in place of
/// `stored` (`None` for a create), each as it will be once the write is done: the total the
/// count or sum tree keeps now, less what `stored` adds to it, plus what `written` adds. For
/// a transfer, a purchase or a price update (`change`), only the rules the change can break
/// are judged, so only their totals are read. The reads are billed to `execution_context`.
///
/// Consensus reads them when it builds the action, before judging the rules
/// ([`DocumentSystemValues::aggregates`]), so that the rules stay a structure check. Added
/// in place to the shipped transformer at protocol version 14 and inert before it: a rule
/// reads a total only where `parse_property_constraints` is `Some(_)`, so earlier versions
/// read nothing and bill nothing here.
///
/// The document batch carries one transition (`SystemLimits::max_transitions_in_documents_batch`),
/// and every state transition of a block is applied before the next is validated, so no
/// write of the same block is missing from the totals read here. Raising that limit needs
/// the batch's own earlier writes added to them.
///
/// [`DocumentSystemValues::aggregates`]: dpp::data_contract::document_type::property_constraints::DocumentSystemValues::aggregates
#[allow(clippy::too_many_arguments)]
fn read_property_constraint_aggregates(
    drive: &Drive,
    contract: &DataContract,
    document_type_name: &str,
    written: DocumentVersion,
    stored: Option<DocumentVersion>,
    change: Option<SystemChange>,
    block_info: &BlockInfo,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<BTreeMap<AggregateRead, i128>, Error> {
    let Some(document_type) = contract.document_type_optional_for_name(document_type_name) else {
        // The action refuses a document type the contract lacks
        return Ok(BTreeMap::new());
    };
    let reads = document_type
        .property_constraints()
        .values()
        .filter(|rule| change.is_none_or(|change| rule.reads_change(change)))
        .flat_map(|rule| rule.aggregate_reads())
        .collect::<BTreeSet<_>>();
    if reads.is_empty() {
        return Ok(BTreeMap::new());
    }
    let written_data = Value::from(written.properties.clone());
    let stored_data = stored.map(|stored| (stored, Value::from(stored.properties.clone())));
    read_aggregates(
        drive,
        contract,
        reads,
        Some(VersionData {
            id: written.id,
            data: &written_data,
            owner_id: written.owner_id,
        }),
        stored_data.as_ref().map(|(stored, data)| VersionData {
            id: stored.id,
            data,
            owner_id: stored.owner_id,
        }),
        block_info,
        execution_context,
        transaction,
        platform_version,
    )
}

/// Reads from state the `countOf` and `sumOf` totals the `deleteConstraints` rules of the
/// document type `document_type_name` read, for its owner's delete of `stored`, whose
/// properties are `stored_data` (one map value, which the caller judges the rules on too), each
/// as it will be once the document is gone: the total the count or sum tree keeps now, less
/// what `stored` adds to it. A filter reads its values (its id, its owner, its properties) from
/// `stored`. The reads are billed to `execution_context`.
///
/// Only document delete state validation 1 calls it (protocol version 14), after the stored
/// document is fetched and its owner checked; a type without delete rules reads nothing.
///
/// Delete state validation reads committed state, and the batch's transitions are all
/// validated before any is applied, so these totals leave out the batch's own earlier writes.
/// That holds only while the document batch carries one transition
/// (`SystemLimits::max_transitions_in_documents_batch`): raising the limit would let a batch
/// create a vote pointing at a poll and delete the poll, the delete reading no vote. Raising it
/// needs the batch's earlier writes added to these totals, as to the write path's.
#[allow(clippy::too_many_arguments)]
pub(crate) fn read_delete_constraint_aggregates(
    drive: &Drive,
    contract: &DataContract,
    document_type_name: &str,
    stored: &Document,
    stored_data: &Value,
    block_info: &BlockInfo,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<BTreeMap<AggregateRead, i128>, Error> {
    let Some(document_type) = contract.document_type_optional_for_name(document_type_name) else {
        // The action refuses a document type the contract lacks
        return Ok(BTreeMap::new());
    };
    let reads = document_type
        .delete_constraints()
        .values()
        .flat_map(|rule| rule.aggregate_reads())
        .collect::<BTreeSet<_>>();
    read_aggregates(
        drive,
        contract,
        reads,
        None,
        Some(VersionData {
            id: stored.id(),
            data: stored_data,
            owner_id: stored.owner_id(),
        }),
        block_info,
        execution_context,
        transaction,
        platform_version,
    )
}

/// Reads each total of `reads` from state as it will be once a write storing `written` in place
/// of `stored` is done (`stored` `None` for a create, `written` `None` for a delete): the total
/// the count or sum tree keeps now, less what `stored` adds to it, plus what `written` adds. A
/// filter reads its values from the document the write leaves, `written`, or from `stored` for a
/// delete. The reads are billed to `execution_context`.
#[allow(clippy::too_many_arguments)]
fn read_aggregates(
    drive: &Drive,
    contract: &DataContract,
    reads: BTreeSet<&AggregateRead>,
    written: Option<VersionData>,
    stored: Option<VersionData>,
    block_info: &BlockInfo,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<BTreeMap<AggregateRead, i128>, Error> {
    let Some(bound) = written.or(stored) else {
        return Ok(BTreeMap::new());
    };
    if reads.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut drive_operations = vec![];
    let mut aggregates = BTreeMap::new();
    for read in reads {
        let Some(counted) = contract.document_type_optional_for_name(&read.document_type) else {
            return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "a propertyConstraints or deleteConstraints rule totals a document type its \
                 contract lacks, which registration refuses",
            )));
        };
        let total = match read.filter_values(
            counted,
            bound.id,
            bound.data,
            bound.owner_id,
            platform_version,
        )? {
            // No document can match a value its key cannot hold
            None => 0,
            Some(filter_values) => {
                let kept = drive.fetch_property_constraint_aggregate(
                    contract.id().to_buffer(),
                    counted,
                    read,
                    &filter_values,
                    transaction,
                    &mut drive_operations,
                    platform_version,
                )?;
                let contribution = |version: Option<VersionData>| match version {
                    Some(version) => read.contribution(
                        counted,
                        &filter_values,
                        version.data,
                        version.owner_id,
                        platform_version,
                    ),
                    None => Ok(0),
                };
                let removed = contribution(stored)?;
                let added = contribution(written)?;
                kept.checked_sub(removed)
                    .and_then(|total| total.checked_add(added))
                    .ok_or(Error::Execution(ExecutionError::Overflow(
                        "a propertyConstraints or deleteConstraints total overflowed an i128",
                    )))?
            }
        };
        aggregates.insert(read.clone(), total);
    }

    let fee = Drive::calculate_fee(
        None,
        Some(drive_operations),
        &block_info.epoch,
        drive.config.epochs_per_era,
        platform_version,
        None,
    )?;
    execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
    Ok(aggregates)
}
