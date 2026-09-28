use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::common::moderators::moderator_field_write_refusal;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::DataContract;
use dpp::platform_value::Identifier;
use dpp::prelude::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::DocumentCreateTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::DocumentReplaceTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;

/// Judges a create or a replace in `result`, by `writer`, that sets, changes or removes a
/// field its document type keeps for the contract's moderators
/// (`moderatorAbilities.changeFields`): a create that sets one, a replace whose changed fields
/// hold one (the first in name order is the one reported). When `writer` does not moderate
/// the contract the write is refused and the refusal returned; when it does, a replace is
/// stamped as a moderator's, so the document is written with `$moderatedAt` the block's time
/// and `$moderatedBy` the writer (a create needs no stamp here: the create action stamps the
/// document as its owner's whenever it sets such a field, which only a moderator's may).
/// `None` for any other action, a refused one included.
///
/// Judged in the transformer, so the mempool refuses such a write as a block does. Added in
/// place to the shipped transformer at protocol version 14 and inert before it: only meta-schema
/// v3 (protocol version 14) admits the keyword, so no earlier document type keeps such a field
/// and nothing is read or stamped.
#[allow(clippy::too_many_arguments)]
pub(super) fn judge_moderator_field_write(
    drive: &Drive,
    contract: &DataContract,
    result: &mut ConsensusValidationResult<BatchedTransitionAction>,
    writer: Identifier,
    block_info: &BlockInfo,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<Option<ConsensusError>, Error> {
    if !result.is_valid() {
        return Ok(None);
    }
    let Some(BatchedTransitionAction::DocumentAction(action)) = result.data.as_mut() else {
        return Ok(None);
    };
    let refusal = {
        let (base, mut written) = match &*action {
            DocumentTransitionAction::CreateAction(action) => (
                action.base(),
                Box::new(action.data().keys()) as Box<dyn Iterator<Item = &String>>,
            ),
            DocumentTransitionAction::ReplaceAction(action) => (
                action.base(),
                Box::new(action.changed_data_fields().iter()) as Box<dyn Iterator<Item = &String>>,
            ),
            _ => return Ok(None),
        };
        let document_type_name = base.document_type_name();
        let Some(document_type) = contract.document_type_optional_for_name(document_type_name)
        else {
            return Ok(None);
        };
        let moderator_fields = document_type.moderator_changeable_fields();
        let Some(field) = written.find(|field| moderator_fields.contains(*field)) else {
            return Ok(None);
        };
        moderator_field_write_refusal(
            drive,
            contract,
            document_type_name,
            base.id(),
            field,
            writer,
            &block_info.epoch,
            execution_context,
            transaction,
            platform_version,
        )?
    };
    if refusal.is_some() {
        return Ok(refusal);
    }
    if let DocumentTransitionAction::ReplaceAction(action) = action {
        action.set_moderated(block_info.time_ms, writer);
    }
    Ok(None)
}
