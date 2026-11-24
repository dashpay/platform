//! Fungible token state transitions

pub mod burn;
pub mod claim;
pub mod config_update;
pub mod destroy;
pub mod emergency_action;
pub mod freeze;
pub mod mint;
pub mod purchase;
pub mod set_price;
pub mod transfer;
pub mod unfreeze;

#[cfg(all(test, feature = "mocks"))]
mod tests;

use dpp::group::GroupStateTransitionInfoStatus;

/// The public note a group-capable token transition carries.
///
/// The proposer of a group action writes its note; a signer who joins an existing action
/// (`GroupStateTransitionInfoOtherSigner` with `action_is_proposer == false`) adds a signature
/// only, and Platform refuses a note from them (`TokenNoteOnlyAllowedWhenProposerError`). A
/// caller replaying a pending proposal naturally hands back every field it read, note included,
/// so the builders leave the note off here instead of building a transition Platform rejects.
/// The proposal keeps its own note either way.
pub(crate) fn public_note_for_signer(
    public_note: Option<String>,
    using_group_info: Option<&GroupStateTransitionInfoStatus>,
) -> Option<String> {
    match using_group_info {
        Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(info))
            if !info.action_is_proposer =>
        {
            None
        }
        _ => public_note,
    }
}
