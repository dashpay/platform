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
//! for it ([`ElectedCharter::active_members`]). An addition is taken back by deleting it and a
//! removal, which only names an elected member, by deleting it too, so both lists are the
//! documents that exist now. The moderation paths of the target read it from
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
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::moderation_charter::{
    property_names, ElectedCharter, SubmittedCharter, ADDED_MODERATOR_DOCUMENT_TYPE_NAME,
    ELECTED_CHARTER_DOCUMENT_TYPE_NAME, REMOVED_MODERATOR_DOCUMENT_TYPE_NAME,
    SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
};
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;
use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::query::{DriveDocumentQuery, InternalClauses, WhereClause, WhereOperator};
use drive::state_transition_action::contract::moderators_pot_settlement::ModeratorsPotSettlement;
use std::collections::BTreeSet;

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
    seated_charter_of(document).map(Some)
}

/// The elected charter stored at `elected_charter_id`, `None` when there is none. A stored
/// elected charter is a seated one: only a contest's winner is ever written to the type's
/// storage. One read by id, billed.
pub(crate) fn fetch_moderation_charter_by_id(
    drive: &Drive,
    elected_charter_id: Identifier,
    epoch: &Epoch,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<Option<SeatedModerationCharter>, Error> {
    let contract = drive
        .cache
        .system_data_contracts
        .load_moderation_charters(platform_version)?;
    let document_type = contract.document_type_for_name(ELECTED_CHARTER_DOCUMENT_TYPE_NAME)?;
    let Some(document) = fetch_document_with_id(
        drive,
        &contract,
        document_type,
        elected_charter_id,
        epoch,
        execution_context,
        transaction,
        platform_version,
    )?
    else {
        return Ok(None);
    };
    seated_charter_of(document).map(Some)
}

/// Reads a stored elected charter document.
fn seated_charter_of(document: Document) -> Result<SeatedModerationCharter, Error> {
    // The schema admitted the document when it was filed, so it reads.
    let charter = ElectedCharter::from_document_properties(document.properties())
        .into_data()
        .map_err(|_| {
            Error::Execution(ExecutionError::DriveIncoherence(
                "a stored elected charter does not read as one",
            ))
        })?;
    Ok(SeatedModerationCharter {
        id: document.id(),
        leader_id: document.owner_id(),
        charter,
    })
}

impl SeatedModerationCharter {
    /// Whether `identity_id` is on the seated team: the leader, or an active member. The
    /// leader costs nothing more. An elected member (one of the charter's `members`) is on
    /// it unless the leader filed a `removedModerator` for it, and anyone else only while the
    /// leader's `addedModerator` for it exists: one point read either way, since a removal
    /// can only name an elected member and an addition is taken back by deleting it. Both
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
        let (document_type_name, present_means_seated) =
            if self.charter.members.contains(&identity_id) {
                (REMOVED_MODERATOR_DOCUMENT_TYPE_NAME, false)
            } else {
                (ADDED_MODERATOR_DOCUMENT_TYPE_NAME, true)
            };
        let present = !query_charter_documents(
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
        .is_empty();
        Ok(present == present_means_seated)
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
        Ok(self
            .fetch_proposal(
                drive,
                epoch,
                execution_context,
                transaction,
                platform_version,
            )?
            .moderators_share_or_full())
    }

    /// The proposal the team runs on: its reasons, its share and its reward split. One read of
    /// the `submittedCharter` by id, billed.
    pub(crate) fn fetch_proposal(
        &self,
        drive: &Drive,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SubmittedCharter, Error> {
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
        // The schema admitted the proposal when it was filed, so it reads.
        SubmittedCharter::from_document_properties(proposal.properties())
            .into_data()
            .map_err(|_| {
                Error::Execution(ExecutionError::DriveIncoherence(
                    "a stored moderation charter proposal does not read as one",
                ))
            })
    }

    /// The active members of the team besides the leader ([`ElectedCharter::active_members`]):
    /// the charter's `members` less its `removedModerator` documents, plus its
    /// `addedModerator` documents. Two billed queries of the `byElectedCharterMember` indexes,
    /// each bounded: a removal names one of the charter's members, and the target's
    /// `maxAddedModerators` caps the additions that exist. A query that can find nothing is not
    /// made.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn fetch_active_members(
        &self,
        drive: &Drive,
        max_added_moderators: u16,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeSet<Identifier>, Error> {
        let removals_bound = u16::try_from(self.charter.members.len()).unwrap_or(u16::MAX);
        let member_ids = |document_type_name: &str,
                          limit: u16,
                          execution_context: &mut StateTransitionExecutionContext|
         -> Result<Vec<Identifier>, Error> {
            if limit == 0 {
                return Ok(vec![]);
            }
            query_charter_documents(
                drive,
                document_type_name,
                [(property_names::ELECTED_CHARTER_ID, self.id)],
                limit,
                epoch,
                execution_context,
                transaction,
                platform_version,
            )?
            .iter()
            .map(|document| {
                // The schema requires the member of every team change.
                document
                    .properties()
                    .get_identifier(property_names::MEMBER_ID)
                    .map_err(|_| {
                        Error::Execution(ExecutionError::DriveIncoherence(
                            "a stored moderation team change names its member",
                        ))
                    })
            })
            .collect()
        };
        let removed = member_ids(
            REMOVED_MODERATOR_DOCUMENT_TYPE_NAME,
            removals_bound,
            execution_context,
        )?;
        let added = member_ids(
            ADDED_MODERATOR_DOCUMENT_TYPE_NAME,
            max_added_moderators,
            execution_context,
        )?;
        Ok(self
            .charter
            .active_members(self.leader_id, &added, &removed))
    }

    /// The settle of the moderators pot of the elected contract `contract_id`, holding
    /// `pot_credits`, to the team as it is now: what the proposal's reward split pays the
    /// leader and each active member ([`dpp::moderation_charter::ModerationCharterRewardSplit::payouts`]), and the
    /// action counts it resets, every count that exists. Reads, all billed: the active members
    /// (see [`SeatedModerationCharter::fetch_active_members`]), the proposal, and the counts,
    /// at most one per identity the team can hold.
    ///
    /// Every settle resets every count, so a count only exists for a member of the team as it
    /// is at the settle: a change of the team settles first.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn settle_moderators_pot(
        &self,
        drive: &Drive,
        contract_id: Identifier,
        pot_credits: Credits,
        max_added_moderators: u16,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ModeratorsPotSettlement, Error> {
        let members = self.fetch_active_members(
            drive,
            max_added_moderators,
            epoch,
            execution_context,
            transaction,
            platform_version,
        )?;
        self.settle_moderators_pot_among(
            &members,
            drive,
            contract_id,
            pot_credits,
            max_added_moderators,
            epoch,
            execution_context,
            transaction,
            platform_version,
        )
    }

    /// [`SeatedModerationCharter::settle_moderators_pot`] with the active members already read
    /// ([`SeatedModerationCharter::fetch_active_members`]): reads the proposal and the counts.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn settle_moderators_pot_among(
        &self,
        members: &BTreeSet<Identifier>,
        drive: &Drive,
        contract_id: Identifier,
        pot_credits: Credits,
        max_added_moderators: u16,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ModeratorsPotSettlement, Error> {
        let proposal = self.fetch_proposal(
            drive,
            epoch,
            execution_context,
            transaction,
            platform_version,
        )?;
        // The leader, the elected members and the additions the target allows.
        let team_bound = u16::try_from(self.charter.members.len())
            .unwrap_or(u16::MAX)
            .saturating_add(max_added_moderators)
            .saturating_add(1);
        let (fee, action_counts) = drive.fetch_contract_moderation_action_counts_with_fee(
            contract_id,
            team_bound,
            epoch,
            transaction,
            platform_version,
        )?;
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
        let payouts = proposal
            .reward_split
            .payouts(pot_credits, self.leader_id, members, &action_counts)
            .map_err(|_| {
                Error::Execution(ExecutionError::DriveIncoherence(
                    "a stored moderation charter proposal's reward split adds up to 100",
                ))
            })?;
        Ok(ModeratorsPotSettlement {
            contract_id,
            payouts,
            settled_action_counts: action_counts.into_keys().collect(),
        })
    }
}

/// How many `addedModerator` documents name the elected charter `elected_charter_id`, counted up
/// to `up_to`: the members added to it now (deleting an addition takes the member off and frees
/// its slot), which the target's `maxAddedModerators` caps. One billed query of the
/// `byElectedCharterMember` index, limited to `up_to` documents, so the cost is bounded by the
/// cap.
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
