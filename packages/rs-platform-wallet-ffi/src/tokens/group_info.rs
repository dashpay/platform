//! FFI helpers for decoding `Option<GroupStateTransitionInfoStatus>`
//! from a flat (kind tag, payload) tuple supplied by the caller.
//!
//! Kind tag:
//!   0 = none (no group action)
//!   1 = proposer (caller is proposing a new group action at `position`)
//!   2 = other-signer (caller is signing on an existing proposal — the
//!       proposal's `action_id` and `action_is_proposer` flag are
//!       supplied; the caller is at `position`)
//!
//! For kind == 2 the caller must hand us a 32-byte `action_id`. For
//! kind == 1 the `action_id` pointer is ignored. For kind == 0 every
//! payload field is ignored.

use dpp::group::{GroupStateTransitionInfo, GroupStateTransitionInfoStatus};

use crate::error::{PlatformWalletFFIResult, PlatformWalletFFIResultCode};
use crate::types::read_identifier;

/// Decode a flat `(kind, position, action_id, action_is_proposer)`
/// tuple from an FFI caller into an `Option<GroupStateTransitionInfoStatus>`.
///
/// On error returns `Err(PlatformWalletFFIResult)` carrying the FFI
/// error the caller should bubble up.
///
/// # Safety
/// - `action_id` may be NULL when `kind != 2`. When `kind == 2` it must
///   point at a 32-byte buffer for the duration of the call.
pub(crate) unsafe fn decode_group_info(
    kind: u8,
    position: u16,
    action_id: *const u8,
    action_is_proposer: bool,
) -> Result<Option<GroupStateTransitionInfoStatus>, PlatformWalletFFIResult> {
    match kind {
        0 => Ok(None),
        1 => Ok(Some(
            GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(position),
        )),
        2 => {
            if action_id.is_null() {
                return Err(PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorNullPointer,
                    "group_info_action_id is null but kind == 2 (other-signer)",
                ));
            }
            let action_identifier = read_identifier(action_id)?;
            Ok(Some(
                GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                    GroupStateTransitionInfo {
                        group_contract_position: position,
                        action_id: action_identifier,
                        action_is_proposer,
                    },
                ),
            ))
        }
        other => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!("Invalid group_info_kind: {other} (expected 0, 1, or 2)"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_kind_zero() {
        unsafe {
            let result = decode_group_info(0, 0, std::ptr::null(), false);
            match result {
                Ok(None) => {}
                _ => panic!("expected Ok(None)"),
            }
        }
    }

    #[test]
    fn test_decode_proposer() {
        unsafe {
            let result = decode_group_info(1, 7, std::ptr::null(), false);
            match result {
                Ok(Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(pos))) => {
                    assert_eq!(pos, 7);
                }
                _ => panic!("expected Proposer(7)"),
            }
        }
    }

    #[test]
    fn test_decode_other_signer_null_action_id() {
        unsafe {
            let result = decode_group_info(2, 0, std::ptr::null(), false);
            assert!(result.is_err(), "expected Err(NullPointer)");
        }
    }

    #[test]
    fn test_decode_other_signer_ok() {
        unsafe {
            let action_id_bytes = [9u8; 32];
            let result = decode_group_info(2, 3, action_id_bytes.as_ptr(), true);
            match result {
                Ok(Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                    info,
                ))) => {
                    assert_eq!(info.group_contract_position, 3);
                    assert!(info.action_is_proposer);
                    assert_eq!(info.action_id.to_buffer(), action_id_bytes);
                }
                _ => panic!("expected OtherSigner"),
            }
        }
    }

    #[test]
    fn test_decode_invalid_kind() {
        unsafe {
            let result = decode_group_info(99, 0, std::ptr::null(), false);
            assert!(result.is_err(), "expected Err(InvalidParameter)");
        }
    }
}
