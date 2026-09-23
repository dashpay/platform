//! The seated moderation charter of an elected contract (protocol version 14).
//!
//! Seating writes nothing. An application for a contract's moderation seat is an
//! `electedCharter` create of the moderation charters system contract on its contested unique
//! index `byTargetContract`, keyed by the target contract, and awarding the contest writes the
//! winner's document to the charter contract's storage. It is the only `electedCharter` ever
//! written there for that target: contenders live in the contest until it is awarded, and in
//! protocol version 14 a seat is never replaced. So the charter seated on a contract is the one
//! `byTargetContract` finds, and its team is its owner, the leader, plus its `members` and the
//! `memberId` of every `addedModerator` for it, less the `memberId` of every `removedModerator`
//! for it ([`ElectedCharter::active_members`]). The moderation paths of the target read it from
//! there: there is no block-end seating hook and no copy under the moderated contract.
//!
//! Every read is a query of the charter contract, served from the system contract cache (it
//! can not change: its owner is the zero identity), whose processing cost is billed to the
//! transition that needed it. These helpers are only reached for a contract with an elected
//! moderation declaration, which exists from protocol version 14, so they carry no version of
//! their own, as `fetch_document_through_lookup` does not.

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::state_transitions::batch::fetch_document_with_id;
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::{Document, DocumentV0Getters};
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::moderation_charter::{
    property_names, ElectedCharter, ADDED_MODERATOR_DOCUMENT_TYPE_NAME,
    ELECTED_CHARTER_DOCUMENT_TYPE_NAME, FULL_MODERATORS_SHARE,
    REMOVED_MODERATOR_DOCUMENT_TYPE_NAME, SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
};
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;
use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::query::{DriveDocumentQuery, InternalClauses, WhereClause, WhereOperator};

/// The charter seated on an elected contract, as stored by the moderation charters contract
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SeatedModerationCharter {
    /// The id of the `electedCharter` document
    pub(crate) id: Identifier,
    /// The leader: the charter's owner, who filed the proposal it runs on
    pub(crate) leader_id: Identifier,
    /// The charter
    pub(crate) charter: ElectedCharter,
}

/// The charter seated on the elected contract `target_contract_id`, `None` while no contest for
/// its seat was awarded. One query of the `byTargetContract` index, billed.
pub(crate) fn fetch_seated_moderation_charter(
    drive: &Drive,
    target_contract_id: Identifier,
    epoch: &Epoch,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<Option<SeatedModerationCharter>, Error> {
    let Some(document) = query_charter_documents(
        drive,
        ELECTED_CHARTER_DOCUMENT_TYPE_NAME,
        [(property_names::TARGET_CONTRACT_ID, target_contract_id)],
        1,
        epoch,
        execution_context,
        transaction,
        platform_version,
    )?
    .into_iter()
    .next() else {
        return Ok(None);
    };
    // The schema admitted the document when it was filed, so it reads.
    let charter = ElectedCharter::from_document_properties(document.properties())
        .into_data()
        .map_err(|_| {
            Error::Execution(ExecutionError::DriveIncoherence(
                "a stored elected charter does not read as one",
            ))
        })?;
    Ok(Some(SeatedModerationCharter {
        id: document.id(),
        leader_id: document.owner_id(),
        charter,
    }))
}

impl SeatedModerationCharter {
    /// Whether `identity_id` is on the seated team: the leader, or an active member. The
    /// leader costs nothing more; an elected member costs a point read of its removal, and
    /// anyone else a point read of its addition and, when there is one, of its removal. Both
    /// types are unique on the charter and the member, so the team is never listed whole.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn seats(
        &self,
        drive: &Drive,
        identity_id: Identifier,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        if identity_id == self.leader_id {
            return Ok(true);
        }
        let member_document = |document_type_name: &str,
                               execution_context: &mut StateTransitionExecutionContext|
         -> Result<bool, Error> {
            Ok(!query_charter_documents(
                drive,
                document_type_name,
                [
                    (property_names::ELECTED_CHARTER_ID, self.id),
                    (property_names::MEMBER_ID, identity_id),
                ],
                1,
                epoch,
                execution_context,
                transaction,
                platform_version,
            )?
            .is_empty())
        };
        let joined = self.charter.members.contains(&identity_id)
            || member_document(ADDED_MODERATOR_DOCUMENT_TYPE_NAME, execution_context)?;
        // A removal is final: whoever it names is off the team for good.
        Ok(joined && !member_document(REMOVED_MODERATOR_DOCUMENT_TYPE_NAME, execution_context)?)
    }

    /// The share of each moderated type's declared moderators fee the team takes: its
    /// proposal's `moderatorsShare`, the full amount when it declares none. One read of the
    /// `submittedCharter` by id, billed.
    pub(crate) fn fetch_moderators_share(
        &self,
        drive: &Drive,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<u8, Error> {
        let contract = drive
            .cache
            .system_data_contracts
            .load_moderation_charters(platform_version)?;
        let document_type =
            contract.document_type_for_name(SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME)?;
        // The elected charter's reference proved the proposal when the charter was filed, and
        // a proposal can not be deleted.
        let proposal = fetch_document_with_id(
            drive,
            &contract,
            document_type,
            self.charter.submitted_charter_id,
            epoch,
            execution_context,
            transaction,
            platform_version,
        )?
        .ok_or(Error::Execution(ExecutionError::DriveIncoherence(
            "the proposal of a seated charter is not stored",
        )))?;
        // The share alone is read: nothing else of the proposal decides the discount. The
        // schema bounds it to 0 to 100 and leaves it out for the full amount.
        let share = proposal
            .properties()
            .get_optional_integer::<u8>(property_names::MODERATORS_SHARE)
            .map_err(|_| {
                Error::Execution(ExecutionError::DriveIncoherence(
                    "a stored moderation charter proposal's share is not a percentage",
                ))
            })?;
        Ok(share.unwrap_or(FULL_MODERATORS_SHARE))
    }
}

/// How many `addedModerator` documents name the elected charter `elected_charter_id`, counted up
/// to `up_to`: the additions ever filed for it (the type is immutable and undeletable), which the
/// target's `maxAddedModerators` caps. One billed query of the `byElectedCharterMember` index,
/// limited to `up_to` documents, so the cost is bounded by the cap.
pub(crate) fn count_added_moderators(
    drive: &Drive,
    elected_charter_id: Identifier,
    up_to: u16,
    epoch: &Epoch,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<u16, Error> {
    if up_to == 0 {
        return Ok(0);
    }
    let additions = query_charter_documents(
        drive,
        ADDED_MODERATOR_DOCUMENT_TYPE_NAME,
        [(property_names::ELECTED_CHARTER_ID, elected_charter_id)],
        up_to,
        epoch,
        execution_context,
        transaction,
        platform_version,
    )?;
    // At most `up_to` documents come back.
    Ok(u16::try_from(additions.len()).unwrap_or(up_to))
}

/// The documents of a moderation charters contract type whose identifier properties equal the
/// given ones, at most `limit`, with the processing cost of the query billed.
#[allow(clippy::too_many_arguments)]
fn query_charter_documents<const N: usize>(
    drive: &Drive,
    document_type_name: &str,
    equal_to: [(&str, Identifier); N],
    limit: u16,
    epoch: &Epoch,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<Vec<Document>, Error> {
    let contract = drive
        .cache
        .system_data_contracts
        .load_moderation_charters(platform_version)?;
    let document_type = contract.document_type_for_name(document_type_name)?;
    let equal_clauses = equal_to
        .into_iter()
        .map(|(field, value)| {
            (
                field.to_string(),
                WhereClause {
                    field: field.to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Identifier(value.to_buffer()),
                },
            )
        })
        .collect();
    let drive_query = DriveDocumentQuery {
        contract: &contract,
        document_type,
        internal_clauses: InternalClauses {
            primary_key_in_clause: None,
            primary_key_equal_clause: None,
            in_clauses: Vec::new(),
            range_clause: None,
            equal_clauses,
        },
        offset: None,
        limit: Some(limit),
        order_by: Default::default(),
        start_at: None,
        start_at_included: false,
        block_time_ms: None,
        resolved_time_ranges: vec![],
        sub_queries: vec![],
    };
    let outcome = drive.query_documents(
        drive_query,
        Some(epoch),
        false,
        transaction,
        Some(platform_version.protocol_version),
    )?;
    execution_context.add_operation(ValidationOperation::PrecalculatedOperation(FeeResult {
        storage_fee: 0,
        processing_fee: outcome.cost(),
        fee_refunds: Default::default(),
        removed_bytes_from_system: 0,
    }));
    Ok(outcome.documents_owned())
}
