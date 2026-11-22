//! Contract moderation and contract fee pots (protocol version 14): the reads. Thin bindings
//! over `dash_sdk::platform::contract_moderation` and `dash_sdk::platform::contract_fee_pots`:
//! rs-sdk decides everything (which lists a contract keeps, the page caps, the cursor of the
//! next page, how a removal record's kept values read); these functions read C arguments, call
//! one rs-sdk entry point, and write the result as JSON.
//!
//! # Reading
//!
//! Every read returns a JSON object (freed with `dash_sdk_string_free`), or no data when rs-sdk
//! returns none. The shapes follow the wasm bindings (`getContractModerationStatus`,
//! `getContractModerationEntries`, `getContractDocumentRemovals`, `getContractTeamActions`,
//! `getContractTeamActionSigners`, `getContractModerationActionCounts`, `getContractFeePots`):
//! camelCase names, identifiers as base58 strings, times in milliseconds and credits as JSON
//! integers. A paged read carries the cursor of its next page, which the caller passes back as
//! it is; a page without one is the last.
//!
//! A refusal from the node (a list the contract does not keep, a document type that keeps no
//! removal records, a contract that is not elected) comes back as the error, not as an empty
//! result.
//!
//! Below protocol version 14 none of these requests is sent: the SDK's protocol gate refuses
//! it first (`Sdk::ensure_network_serves`, `dash_sdk::Error::ProtocolVersionNotActive`). The
//! error code tells why (`is_transient_protocol_version_refusal` decides it):
//! * `ProtocolError` when the network is proven below 14, or the SDK is pinned below 14:
//!   check the pin, or wait for the network to activate 14;
//! * `NetworkError` when the query for the network's version failed for a reason that can
//!   pass: retry.
//!
//! # Enumerated arguments
//!
//! A list and a team action status cross the boundary as a plain `uint8_t` holding a
//! [`ContractModerationListFFI`] or [`ContractTeamActionStatusFFI`] discriminant, not as the
//! enum itself: an out-of-range value read as a `#[repr(u8)]` enum is undefined behavior, so
//! the byte is compared with the discriminants and any other value is refused with
//! `InvalidParameter`. The enums are not exported to the C header for that reason; Swift and
//! Kotlin mirror their values.

mod json;
mod queries;

pub use queries::*;

use crate::FFIError;
use dash_sdk::platform::contract_moderation::{ContractModerationList, GroupActionStatus};

/// The moderation lists, as the `list` and `lists` arguments name them.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContractModerationListFFI {
    /// The banlist.
    Banlist = 0,
    /// The suspension list.
    Suspensions = 1,
    /// The warning list.
    Warnings = 2,
}

/// The list `value` names; any other byte is refused.
pub(crate) fn moderation_list(value: u8, field: &str) -> Result<ContractModerationList, FFIError> {
    match value {
        x if x == ContractModerationListFFI::Banlist as u8 => Ok(ContractModerationList::Banlist),
        x if x == ContractModerationListFFI::Suspensions as u8 => {
            Ok(ContractModerationList::Suspensions)
        }
        x if x == ContractModerationListFFI::Warnings as u8 => Ok(ContractModerationList::Warnings),
        other => Err(FFIError::InvalidParameter(format!(
            "{field} {other} is not a moderation list (0 banlist, 1 suspensions, 2 warnings)"
        ))),
    }
}

/// A team action's status, as the `status` argument names it.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContractTeamActionStatusFFI {
    /// Still gathering approvals.
    Active = 0,
    /// Its approvals met the rule and it ran.
    Closed = 1,
}

/// The status `value` names; any other byte is refused.
pub(crate) fn team_action_status(value: u8) -> Result<GroupActionStatus, FFIError> {
    match value {
        x if x == ContractTeamActionStatusFFI::Active as u8 => Ok(GroupActionStatus::ActionActive),
        x if x == ContractTeamActionStatusFFI::Closed as u8 => Ok(GroupActionStatus::ActionClosed),
        other => Err(FFIError::InvalidParameter(format!(
            "status {other} is not a team action status (0 active, 1 closed)"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_read_every_list_byte_and_refuse_any_other() {
        assert_eq!(
            moderation_list(0, "list").expect("banlist"),
            ContractModerationList::Banlist
        );
        assert_eq!(
            moderation_list(1, "list").expect("suspensions"),
            ContractModerationList::Suspensions
        );
        assert_eq!(
            moderation_list(2, "list").expect("warnings"),
            ContractModerationList::Warnings
        );
        for refused in [3, u8::MAX] {
            assert!(matches!(
                moderation_list(refused, "list"),
                Err(FFIError::InvalidParameter(_))
            ));
        }
    }

    #[test]
    fn should_read_every_status_byte_and_refuse_any_other() {
        assert_eq!(
            team_action_status(0).expect("active"),
            GroupActionStatus::ActionActive
        );
        assert_eq!(
            team_action_status(1).expect("closed"),
            GroupActionStatus::ActionClosed
        );
        for refused in [2, u8::MAX] {
            assert!(matches!(
                team_action_status(refused),
                Err(FFIError::InvalidParameter(_))
            ));
        }
    }
}
