//! The seated moderation charters the state validation of one batch reads for the team changes
//! it validates (`addedModerator` and `removedModerator` of the moderation charters contract),
//! each with its target's `maxAddedModerators`: the cap on additions and the settle a change
//! forces read them once between them.

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::common::seated_moderation_charter::{
    fetch_moderation_charter_by_id, SeatedModerationCharter,
};
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

/// A seated charter with the `maxAddedModerators` its target declares.
pub(super) struct SeatedCharterRead {
    pub(super) charter: SeatedModerationCharter,
    pub(super) max_added_moderators: u16,
}

/// The seated charters read by one batch state validation, by elected charter id.
#[derive(Default)]
pub(super) struct SeatedCharterReads {
    reads: BTreeMap<Identifier, SeatedCharterRead>,
}

impl SeatedCharterReads {
    /// The seated charter `elected_charter_id` a team change of the batch names, read the
    /// first time with its target contract (both billed), and from this batch's reads after.
    /// Call it only for a change whose own state validation passed: its reference then proved
    /// the charter stored, which means seated (only a contest's winner is written to the type's
    /// storage).
    pub(super) fn read(
        &mut self,
        elected_charter_id: Identifier,
        platform: &PlatformStateRef,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<&SeatedCharterRead, Error> {
        let entry = match self.reads.entry(elected_charter_id) {
            Entry::Occupied(entry) => return Ok(entry.into_mut()),
            Entry::Vacant(entry) => entry,
        };
        let charter = fetch_moderation_charter_by_id(
            platform.drive,
            elected_charter_id,
            epoch,
            execution_context,
            transaction,
            platform_version,
        )?
        .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
            "the elected charter a moderation team change refers to was found by its reference",
        )))?;
        // The fee this call returns is billed, never the one a cached fetch info carries, which
        // depends on the cache.
        let (fee, target_contract) = platform.drive.get_contract_with_fetch_info_and_fee(
            charter.charter.target_contract_id.to_buffer(),
            Some(epoch),
            false,
            transaction,
            platform_version,
        )?;
        let fee = fee.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
            "fee must exist when fetching a contract with an epoch",
        )))?;
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
        // A charter is only filed for a contract that declares elected moderation (its
        // `electionOpen` requirement), which is fixed at the contract's creation, and a contract
        // is never deleted.
        let max_added_moderators = target_contract
            .as_ref()
            .and_then(|fetch_info| {
                fetch_info
                    .contract
                    .config()
                    .moderation()
                    .and_then(|moderation| moderation.moderators.elected())
                    .map(|elected| elected.max_added_moderators)
            })
            .ok_or(Error::Execution(ExecutionError::DriveIncoherence(
                "the target of a stored elected charter declares elected moderation",
            )))?;
        Ok(entry.insert(SeatedCharterRead {
            charter,
            max_added_moderators,
        }))
    }
}
