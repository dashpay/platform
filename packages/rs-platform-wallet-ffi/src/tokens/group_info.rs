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

use crate::error::{PlatformWalletFFIError, PlatformWalletFFIResult};
use crate::types::read_identifier;

/// Outcome of decoding a group-info FFI tuple.
pub(crate) enum GroupInfoDecode {
    /// Successfully decoded — may be `None` for kind == 0.
    Ok(Option<GroupStateTransitionInfoStatus>),
    /// Decoding failed; `out_error` has been stamped (when non-null)
    /// and the caller should bail out with this code.
    Err(PlatformWalletFFIResult),
}

/// Decode a flat `(kind, position, action_id, action_is_proposer)`
/// tuple from an FFI caller into an `Option<GroupStateTransitionInfoStatus>`.
///
/// # Safety
/// - `action_id` may be NULL when `kind != 2`. When `kind == 2` it must
///   point at a 32-byte buffer for the duration of the call.
/// - `out_error` may be NULL.
pub(crate) unsafe fn decode_group_info(
    kind: u8,
    position: u16,
    action_id: *const u8,
    action_is_proposer: bool,
    out_error: *mut PlatformWalletFFIError,
) -> GroupInfoDecode {
    match kind {
        0 => GroupInfoDecode::Ok(None),
        1 => GroupInfoDecode::Ok(Some(
            GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(position),
        )),
        2 => {
            if action_id.is_null() {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorNullPointer,
                        "group_info_action_id is null but kind == 2 (other-signer)",
                    );
                }
                return GroupInfoDecode::Err(PlatformWalletFFIResult::ErrorNullPointer);
            }
            let action_identifier = match read_identifier(action_id) {
                Ok(i) => i,
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorInvalidIdentifier,
                            format!("Invalid group_info_action_id: {e}"),
                        );
                    }
                    return GroupInfoDecode::Err(PlatformWalletFFIResult::ErrorInvalidIdentifier);
                }
            };
            GroupInfoDecode::Ok(Some(
                GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                    GroupStateTransitionInfo {
                        group_contract_position: position,
                        action_id: action_identifier,
                        action_is_proposer,
                    },
                ),
            ))
        }
        other => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidParameter,
                    format!("Invalid group_info_kind: {other} (expected 0, 1, or 2)"),
                );
            }
            GroupInfoDecode::Err(PlatformWalletFFIResult::ErrorInvalidParameter)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_kind_zero() {
        unsafe {
            let result = decode_group_info(0, 0, std::ptr::null(), false, std::ptr::null_mut());
            match result {
                GroupInfoDecode::Ok(None) => {}
                _ => panic!("expected Ok(None)"),
            }
        }
    }

    #[test]
    fn test_decode_proposer() {
        unsafe {
            let result = decode_group_info(1, 7, std::ptr::null(), false, std::ptr::null_mut());
            match result {
                GroupInfoDecode::Ok(Some(
                    GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(pos),
                )) => {
                    assert_eq!(pos, 7);
                }
                _ => panic!("expected Proposer(7)"),
            }
        }
    }

    #[test]
    fn test_decode_other_signer_null_action_id() {
        unsafe {
            let mut err = PlatformWalletFFIError::success();
            let result = decode_group_info(2, 0, std::ptr::null(), false, &mut err);
            match result {
                GroupInfoDecode::Err(code) => {
                    assert_eq!(code, PlatformWalletFFIResult::ErrorNullPointer);
                }
                _ => panic!("expected Err(NullPointer)"),
            }
        }
    }

    #[test]
    fn test_decode_other_signer_ok() {
        unsafe {
            let action_id_bytes = [9u8; 32];
            let result =
                decode_group_info(2, 3, action_id_bytes.as_ptr(), true, std::ptr::null_mut());
            match result {
                GroupInfoDecode::Ok(Some(
                    GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(info),
                )) => {
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
            let mut err = PlatformWalletFFIError::success();
            let result = decode_group_info(99, 0, std::ptr::null(), false, &mut err);
            match result {
                GroupInfoDecode::Err(code) => {
                    assert_eq!(code, PlatformWalletFFIResult::ErrorInvalidParameter);
                }
                _ => panic!("expected Err(InvalidParameter)"),
            }
        }
    }
}
