//! Who moderates a contract, as state has it now: the moderators its declaration names, or
//! the team of the charter seated on an elected contract. Read by the moderation transition,
//! and by a document create or replace that writes a field only moderators write.

use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::common::seated_moderation_charter::{
    fetch_seated_moderation_charter, SeatedModerationCharter, TeamSeat,
};
use dpp::block::epoch::Epoch;
use dpp::consensus::state::contract_moderation::{
    DocumentModeratorFieldNotWritableError, ModerationReasonNotListedError,
};
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::moderation::{
    ContractModerationConfig, ElectedModerators, ModerationAbility,
};
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::data_contract::DataContract;
use dpp::prelude::Identifier;
use dpp::state_transition::contract_user_moderation_transition::ContractUserModerationAction;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::contract::contract_user_moderation::ContractUserModerationTransitionAction;

/// How an identity moderates a contract
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModeratorSeat {
    /// As one of the moderators the declaration names
    Declared,
    /// From its seat on the seated team
    Team(TeamSeat),
}

/// Who moderates a contract, as state has it now.
///
/// Until an elected contract has a seated charter, the moderators its declaration names do: the
/// merged kinds, or the elected declaration's interim ([`ContractModerationConfig::may_moderate`]
/// and [`ContractModerationConfig::protects`]). Once a contest for its seat was awarded, the
/// team of the seated charter does, and only it (decentralized moderation teams): the
/// leader and the active members moderate, with the abilities the declaration gives the team
/// and no others, and they are protected, with the owner when the declaration says so. Interim
/// moderators are then neither.
pub(crate) enum Moderators<'a> {
    /// The moderators the declaration names
    Declared(&'a ContractModerationConfig),
    /// The team of the charter seated on an elected contract
    Seated {
        elected: &'a ElectedModerators,
        charter: SeatedModerationCharter,
    },
}

impl<'a> Moderators<'a> {
    /// Who moderates a contract declaring `moderation`: for an elected declaration, whether a
    /// charter is seated is read (billed); nothing is read for the merged kinds.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn read(
        moderation: &'a ContractModerationConfig,
        contract_id: Identifier,
        drive: &Drive,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let Some(elected) = moderation.moderators.elected() else {
            return Ok(Moderators::Declared(moderation));
        };
        Ok(
            match fetch_seated_moderation_charter(
                drive,
                contract_id,
                epoch,
                execution_context,
                tx,
                platform_version,
            )? {
                None => Moderators::Declared(moderation),
                Some(charter) => Moderators::Seated { elected, charter },
            },
        )
    }

    /// Whether `identity_id` may moderate the contract owned by `owner_id`. For a seated team,
    /// whether it is the leader or an active member, at most two point reads, billed.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn may_moderate(
        &self,
        owner_id: Identifier,
        identity_id: Identifier,
        drive: &Drive,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        Ok(self
            .seat_of(
                owner_id,
                identity_id,
                drive,
                epoch,
                execution_context,
                tx,
                platform_version,
            )?
            .is_some())
    }

    /// How `identity_id` moderates the contract owned by `owner_id`, `None` when it does not:
    /// [`Moderators::may_moderate`], by the same reads, with the seat of a member of a seated
    /// team ([`SeatedModerationCharter::seat_of`]).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn seat_of(
        &self,
        owner_id: Identifier,
        identity_id: Identifier,
        drive: &Drive,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ModeratorSeat>, Error> {
        match self {
            Moderators::Declared(moderation) => Ok(moderation
                .may_moderate(&owner_id, &identity_id)
                .then_some(ModeratorSeat::Declared)),
            Moderators::Seated { charter, .. } => Ok(charter
                .seat_of(
                    drive,
                    identity_id,
                    epoch,
                    execution_context,
                    tx,
                    platform_version,
                )?
                .map(ModeratorSeat::Team)),
        }
    }

    /// Whether `identity_id` is protected from moderation on the contract owned by `owner_id`:
    /// it can not be put on a list, and its documents can not be deleted. For a seated team,
    /// the owner when the declaration protects it, and the leader and the active members.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn protects(
        &self,
        owner_id: Identifier,
        identity_id: Identifier,
        drive: &Drive,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        match self {
            Moderators::Declared(moderation) => Ok(moderation.protects(&owner_id, &identity_id)),
            Moderators::Seated { elected, charter } => {
                if identity_id == owner_id && elected.owner_protected {
                    return Ok(true);
                }
                charter.seats(
                    drive,
                    identity_id,
                    epoch,
                    execution_context,
                    tx,
                    platform_version,
                )
            }
        }
    }

    /// The refusal of a seated team's ban, suspension, warning, document deletion, field change
    /// or proposal of a settled document's deletion whose
    /// reason names no reason document its proposal lists (decentralized moderation teams): a
    /// team acts only on the grounds it proposed, and a proposal that lists none can take no
    /// such action. The proposal is read, billed, only when the reason names a document. A
    /// reversal carries no reason and is not checked, and neither are the moderators a
    /// declaration names, the interim among them, whose reason document is stored as written.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn unlisted_reason(
        &self,
        action: &ContractUserModerationAction,
        contract_id: Identifier,
        drive: &Drive,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ConsensusError>, Error> {
        let Moderators::Seated { charter, .. } = self else {
            return Ok(None);
        };
        let reason = match action {
            ContractUserModerationAction::Ban { reason, .. }
            | ContractUserModerationAction::Suspend { reason, .. }
            | ContractUserModerationAction::Warn { reason, .. }
            | ContractUserModerationAction::DeleteDocument { reason, .. }
            | ContractUserModerationAction::ChangeDocumentFields { reason, .. }
            | ContractUserModerationAction::DeleteSettledDocument { reason, .. } => reason,
            ContractUserModerationAction::Unban { .. }
            | ContractUserModerationAction::Unsuspend { .. }
            | ContractUserModerationAction::ClearWarnings { .. }
            | ContractUserModerationAction::RestoreDocument { .. }
            | ContractUserModerationAction::ApproveTeamAction { .. } => return Ok(None),
        };
        let listed = match reason.reason_document_id {
            None => false,
            Some(reason_document_id) => charter
                .fetch_proposal(drive, epoch, execution_context, tx, platform_version)?
                .reasons
                .contains(&reason_document_id),
        };
        Ok((!listed).then(|| {
            ModerationReasonNotListedError::new(
                contract_id,
                charter.charter.submitted_charter_id,
                reason.reason_document_id,
            )
            .into()
        }))
    }

    /// `action`, counted for its signer when a member of a seated team signs a ban, a
    /// suspension, a warning or a document deletion: the signer's moderation action count since
    /// the moderators pot was last settled is read (one point read, billed) and the action
    /// carries it one higher, for Drive to write. What a settle splits the pot's action share
    /// by. A reversal (an unban, an unsuspension, a clearing, a restore) counts for nothing, and
    /// neither does a field change: changes of one document have no bound, as deletions of the
    /// content that exists do, so counting them would let a member farm the share. Neither does
    /// an action of the moderators a declaration names, who share the pot equally. A team
    /// action is counted by its own transforms, for every approver whose approval counts, and
    /// only once the approvals run it.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn count_for_signer(
        &self,
        action: ContractUserModerationTransitionAction,
        drive: &Drive,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ContractUserModerationTransitionAction, Error> {
        let Moderators::Seated { .. } = self else {
            return Ok(action);
        };
        let counts = matches!(
            action.action(),
            ContractUserModerationAction::Ban { .. }
                | ContractUserModerationAction::Suspend { .. }
                | ContractUserModerationAction::Warn { .. }
                | ContractUserModerationAction::DeleteDocument { .. }
        );
        if !counts {
            return Ok(action);
        }
        let count = next_moderation_action_count(
            drive,
            action.data_contract_id(),
            action.moderator_id(),
            epoch,
            execution_context,
            tx,
            platform_version,
        )?;
        Ok(match count {
            Some(count) => action.with_moderation_action_count(count),
            None => action,
        })
    }

    /// Whether a seated team lacks `ability`: on `document_type_name` for a deletion, a
    /// restore or a field change, on every moderated type for a list, which is contract-wide. The moderators a
    /// declaration names hold every ability the contract backs.
    pub(crate) fn lacks(
        &self,
        ability: ModerationAbility,
        document_type_name: Option<&str>,
    ) -> bool {
        match self {
            Moderators::Declared(_) => false,
            Moderators::Seated { elected, .. } => match document_type_name {
                Some(document_type_name) => !elected.allows(document_type_name, ability),
                None => !elected.allows_on_any_type(ability),
            },
        }
    }
}

/// The moderation action count of `identity_id`, a member of the seated team of the elected
/// contract `contract_id`, with one more action counted: its count since the moderators pot was
/// last settled (0 when it has none), plus one, for Drive to write. One point read, billed.
/// `None` when the contract has no counts tree, being stored elected before the counts existed:
/// its team's actions go uncounted, and a settle splits the action share equally.
#[allow(clippy::too_many_arguments)]
pub(crate) fn next_moderation_action_count(
    drive: &Drive,
    contract_id: Identifier,
    identity_id: Identifier,
    epoch: &Epoch,
    execution_context: &mut StateTransitionExecutionContext,
    tx: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<Option<u32>, Error> {
    let (fee, count) = drive.fetch_contract_moderation_action_count_with_fee(
        contract_id,
        identity_id,
        epoch,
        tx,
        platform_version,
    )?;
    execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
    Ok(count.map(|count| count.saturating_add(1)))
}

/// The refusal of a document create or replace by `writer` that sets, changes or removes
/// `field`, one of the fields the document type `document_type_name` keeps for the contract's
/// moderators (`moderatorAbilities.changeFields`): `None` when `writer` moderates the contract,
/// holding `changeDocumentFields` on the type when a team is seated. Such a field is the
/// moderators' to write, and a document's owner writes it only when the owner is one of them:
/// the contract owner under the merged kinds and during an interim that names it, a member of
/// the seated team once one is. Who moderates is read, billed, only when such a field is
/// written.
#[allow(clippy::too_many_arguments)]
pub(crate) fn moderator_field_write_refusal(
    drive: &Drive,
    contract: &DataContract,
    document_type_name: &str,
    document_id: Identifier,
    field: &str,
    writer: Identifier,
    epoch: &Epoch,
    execution_context: &mut StateTransitionExecutionContext,
    tx: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<Option<ConsensusError>, Error> {
    let refusal = || {
        Some(
            DocumentModeratorFieldNotWritableError::new(
                contract.id(),
                document_type_name.to_string(),
                document_id,
                field.to_string(),
                writer,
            )
            .into(),
        )
    };
    // The keyword is only admitted on a contract that declares moderation
    let Some(moderation) = contract.config().moderation() else {
        return Ok(refusal());
    };
    let moderators = Moderators::read(
        moderation,
        contract.id(),
        drive,
        epoch,
        execution_context,
        tx,
        platform_version,
    )?;
    let moderates = moderators.may_moderate(
        contract.owner_id(),
        writer,
        drive,
        epoch,
        execution_context,
        tx,
        platform_version,
    )? && !moderators.lacks(
        ModerationAbility::ChangeDocumentFields,
        Some(document_type_name),
    );
    Ok(if moderates { None } else { refusal() })
}
