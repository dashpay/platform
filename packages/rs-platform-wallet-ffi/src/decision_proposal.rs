//! FFI bindings for decision proposal create and edit on `IdentityWallet`
//! (protocol version 15).
//!
//! Thin: each function reads its C arguments, calls
//! `IdentityWallet::create_decision_proposal_with_signer` or
//! `IdentityWallet::edit_decision_proposal_with_signer` on the
//! platform-wallet 8 MB worker stack, and writes the confirmed proposal
//! back. rs-sdk and the wallet decide everything else: the proposal's
//! properties, the decision proposal fund the create states, the signing
//! key, refusing an edit that changes nothing, and the revision an edit
//! is put at.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::slice;

use dash_sdk::platform::decision_proposals::{DecisionProposalEdit, DecisionProposalInput};
use dpp::document::{Document, DocumentV0Getters};
use platform_wallet::PlatformWalletError;
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::check_ptr;
use crate::dashpay_profile::decode_opt_c_str;
use crate::document::confirmed_document_to_json;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::read_identifier;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// What a new decision proposal asks and the rule it is decided by
/// (`platform_wallet_decision_proposal_create`). Platform refuses values
/// outside the limits noted here when the proposal is created; nothing is
/// checked before.
#[repr(C)]
pub struct DecisionProposalParamsFFI {
    /// The question, in one line: 1 to 128 characters. NUL-terminated
    /// UTF-8, required.
    pub title: *const c_char,
    /// What a yes and a no mean: 1 to 4096 characters. NUL-terminated
    /// UTF-8, required.
    pub description: *const c_char,
    /// Where the proposal is discussed, a URI of at most 512 characters;
    /// null for none.
    pub url: *const c_char,
    /// The proposal passes when the yes voting power is strictly more than
    /// this percent of the yes plus no voting power: 50 to 80.
    pub pass_percent: u8,
    /// The least yes plus no voting power the proposal needs, as a percent
    /// of the masternode list's total voting power when it closes, rounded
    /// up: 0 to 50.
    pub minimum_turnout_percent: u8,
    /// When voting closes, in milliseconds since the Unix epoch: from a
    /// week to four weeks after the block that creates the proposal, which
    /// is a little later than now.
    pub end_date_ms: u64,
}

/// New text for a decision proposal
/// (`platform_wallet_decision_proposal_edit`). A field left null keeps
/// its value. Platform accepts the edit for a day after the proposal was
/// created (or a fifth of the time it runs, if that is shorter); the rule
/// and the end date never change.
#[repr(C)]
pub struct DecisionProposalEditFFI {
    /// The new title, NUL-terminated UTF-8; null keeps it.
    pub title: *const c_char,
    /// The new description, NUL-terminated UTF-8; null keeps it.
    pub description: *const c_char,
    /// The new url, NUL-terminated UTF-8; null keeps it, unless
    /// `remove_url` is set.
    pub url: *const c_char,
    /// Removes the url. Setting it together with a non-null `url` is
    /// refused with `ErrorInvalidParameter`.
    pub remove_url: bool,
}

/// The UTF-8 string `field` points to; a null pointer is refused.
///
/// # Safety
/// `ptr` must be null or point to a NUL-terminated string that outlives
/// the call.
unsafe fn required_string(
    ptr: *const c_char,
    field: &str,
) -> Result<String, PlatformWalletFFIResult> {
    if ptr.is_null() {
        return Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorNullPointer,
            format!("{field} is null"),
        ));
    }
    Ok(CStr::from_ptr(ptr).to_str()?.to_string())
}

/// The proposal input `params` holds.
///
/// # Safety
/// `params` must point to a `DecisionProposalParamsFFI` whose string
/// fields are null or NUL-terminated.
unsafe fn proposal_input(
    params: &DecisionProposalParamsFFI,
) -> Result<DecisionProposalInput, PlatformWalletFFIResult> {
    Ok(DecisionProposalInput {
        title: required_string(params.title, "title")?,
        description: required_string(params.description, "description")?,
        url: decode_opt_c_str(params.url)?,
        pass_percent: params.pass_percent,
        minimum_turnout_percent: params.minimum_turnout_percent,
        end_date: params.end_date_ms,
    })
}

/// The edit `edit` holds.
///
/// # Safety
/// `edit` must point to a `DecisionProposalEditFFI` whose string fields
/// are null or NUL-terminated.
unsafe fn proposal_edit(
    edit: &DecisionProposalEditFFI,
) -> Result<DecisionProposalEdit, PlatformWalletFFIResult> {
    let url = match (decode_opt_c_str(edit.url)?, edit.remove_url) {
        (Some(_), true) => {
            return Err(PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                "url and remove_url cannot both be set",
            ))
        }
        (Some(url), false) => Some(Some(url)),
        (None, true) => Some(None),
        (None, false) => None,
    };
    Ok(DecisionProposalEdit {
        title: decode_opt_c_str(edit.title)?,
        description: decode_opt_c_str(edit.description)?,
        url,
    })
}

/// Write `json` to `*out_document_json`, handing its ownership to the
/// caller.
///
/// # Safety
/// `out_document_json` must be a valid, writable pointer.
unsafe fn write_document_json(
    json: String,
    out_document_json: *mut *mut c_char,
) -> Result<(), PlatformWalletFFIResult> {
    // A NUL byte inside the JSON would be a serializer bug; refuse it
    // rather than panic across the FFI boundary.
    *out_document_json = CString::new(json)?.into_raw();
    Ok(())
}

/// Create + broadcast a decision proposal of `owner_identity_id` asking
/// what `params` says, signed via the external `signer_handle`.
///
/// Goes through `IdentityWallet::create_decision_proposal_with_signer`:
/// rs-sdk builds the proposal, whose create states the decision proposal
/// fund (0.5 Dash), paid by the owner on top of the fees; the wallet signs
/// with the owner's first AUTHENTICATION + ECDSA key at a security level
/// the `proposal` type accepts that the signer holds, broadcasts on the
/// platform-wallet 8 MB worker stack, waits for the confirmed document and
/// keeps the owner's proven balance. Platform refuses a rule outside the
/// contract's limits and an end date less than a week or more than four
/// weeks after the block that creates the proposal. A network below
/// protocol version 15 has no decision proposals, which is an error.
///
/// On success the proposal's 32-byte id is written to `out_proposal_id`,
/// and the confirmed document's canonical JSON (the shape
/// [`platform_wallet_create_document_with_signer`](crate::platform_wallet_create_document_with_signer)
/// returns) to `*out_document_json`; release it with
/// `platform_wallet_string_free`. On any error `*out_document_json` is
/// left null. A refusal by Platform carries its `consensus_code`.
///
/// # Safety
/// - `owner_identity_id` must point to 32 readable bytes.
/// - `params` must point to a `DecisionProposalParamsFFI` whose `title`
///   and `description` are NUL-terminated strings and whose `url` is null
///   or one.
/// - `signer_handle` must be a live signer, which the caller keeps owning.
/// - `out_proposal_id` must point to 32 writable bytes and
///   `out_document_json` must be a valid, writable pointer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_decision_proposal_create(
    wallet_handle: Handle,
    owner_identity_id: *const u8,
    params: *const DecisionProposalParamsFFI,
    signer_handle: *mut SignerHandle,
    out_proposal_id: *mut u8,
    out_document_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(params);
    check_ptr!(signer_handle);
    check_ptr!(out_proposal_id);
    check_ptr!(out_document_json);
    *out_document_json = ptr::null_mut();

    let owner_id = unwrap_result_or_return!(read_identifier(owner_identity_id));
    let input = unwrap_result_or_return!(proposal_input(&*params));
    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            let confirmed: Document = identity_wallet
                .create_decision_proposal_with_signer(&owner_id, &input, signer)
                .await?;
            let json = confirmed_document_to_json(&confirmed)?;
            Ok::<_, PlatformWalletError>((confirmed.id(), json))
        })
    });
    let result = unwrap_option_or_return!(option);
    let (proposal_id, document_json) = unwrap_result_or_return!(result);

    unwrap_result_or_return!(write_document_json(document_json, out_document_json));
    slice::from_raw_parts_mut(out_proposal_id, 32).copy_from_slice(proposal_id.as_slice());
    PlatformWalletFFIResult::ok()
}

/// Give the decision proposal `proposal_id` of `owner_identity_id` the new
/// text `edit` sets, and broadcast the replace, signed via the external
/// `signer_handle`.
///
/// Goes through `IdentityWallet::edit_decision_proposal_with_signer`,
/// which reads the proposal once, gives it the new text and the revision
/// after the one read (rs-sdk refuses an edit that changes nothing), signs
/// with a key chosen as the create chooses it and puts that document. It
/// never reads the proposal again before the put: if another edit lands in
/// between, Platform refuses this one (`consensus_code` 40106,
/// `InvalidDocumentRevisionError`) rather than overwrite it, and the host
/// reads the proposal again before offering the edit anew. Platform also
/// refuses an edit more than a day after the proposal was created (or a
/// fifth of the time it runs, if that is shorter).
///
/// On success the confirmed document's canonical JSON, at its new
/// revision, is written to `*out_document_json`; release it with
/// `platform_wallet_string_free`. On any error it is left null.
///
/// # Errors
/// - `NotFound` when there is no such proposal;
/// - `ErrorInvalidParameter` when the proposal belongs to another
///   identity, the edit changes nothing, or `url` and `remove_url` are
///   both set;
/// - `ErrorUtf8Conversion` for a string field that is not UTF-8;
/// - a refusal by Platform carries its `consensus_code`.
///
/// # Safety
/// - `owner_identity_id` and `proposal_id` must each point to 32 readable
///   bytes.
/// - `edit` must point to a `DecisionProposalEditFFI` whose string fields
///   are null or NUL-terminated.
/// - `signer_handle` must be a live signer, which the caller keeps owning.
/// - `out_document_json` must be a valid, writable pointer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_decision_proposal_edit(
    wallet_handle: Handle,
    owner_identity_id: *const u8,
    proposal_id: *const u8,
    edit: *const DecisionProposalEditFFI,
    signer_handle: *mut SignerHandle,
    out_document_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(edit);
    check_ptr!(signer_handle);
    check_ptr!(out_document_json);
    *out_document_json = ptr::null_mut();

    let owner_id = unwrap_result_or_return!(read_identifier(owner_identity_id));
    let proposal_id = unwrap_result_or_return!(read_identifier(proposal_id));
    let edit = unwrap_result_or_return!(proposal_edit(&*edit));
    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            let confirmed: Document = identity_wallet
                .edit_decision_proposal_with_signer(&owner_id, &proposal_id, &edit, signer)
                .await?;
            confirmed_document_to_json(&confirmed)
        })
    });
    let result = unwrap_option_or_return!(option);
    let document_json = unwrap_result_or_return!(result);

    unwrap_result_or_return!(write_document_json(document_json, out_document_json));
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::error::StateTransitionBroadcastError;
    use dpp::consensus::state::document::invalid_document_revision_error::InvalidDocumentRevisionError;
    use dpp::consensus::ConsensusError;
    use dpp::prelude::Identifier;

    fn c_string(value: &str) -> CString {
        CString::new(value).expect("no NUL")
    }

    fn no_edit() -> DecisionProposalEditFFI {
        DecisionProposalEditFFI {
            title: ptr::null(),
            description: ptr::null(),
            url: ptr::null(),
            remove_url: false,
        }
    }

    #[test]
    fn should_read_the_proposal_params() {
        let title = c_string("Raise the block size?");
        let description = c_string("Yes doubles it; no keeps it.");
        let url = c_string("https://forum.dash.org/t/block-size");
        let mut params = DecisionProposalParamsFFI {
            title: title.as_ptr(),
            description: description.as_ptr(),
            url: url.as_ptr(),
            pass_percent: 60,
            minimum_turnout_percent: 10,
            end_date_ms: 1_701_209_600_000,
        };
        unsafe {
            assert_eq!(
                proposal_input(&params).expect("reads"),
                DecisionProposalInput {
                    title: "Raise the block size?".to_string(),
                    description: "Yes doubles it; no keeps it.".to_string(),
                    url: Some("https://forum.dash.org/t/block-size".to_string()),
                    pass_percent: 60,
                    minimum_turnout_percent: 10,
                    end_date: 1_701_209_600_000,
                }
            );
            params.url = ptr::null();
            assert_eq!(proposal_input(&params).expect("reads").url, None);
            params.title = ptr::null();
            let refused = proposal_input(&params).expect_err("no title");
            assert_eq!(refused.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        }
    }

    #[test]
    fn should_read_the_edit_and_refuse_a_url_also_removed() {
        let title = c_string("Raise the block size to 4 MB?");
        let url = c_string("https://example.com");
        unsafe {
            assert_eq!(
                proposal_edit(&no_edit()).expect("reads"),
                DecisionProposalEdit::default()
            );
            assert_eq!(
                proposal_edit(&DecisionProposalEditFFI {
                    title: title.as_ptr(),
                    url: url.as_ptr(),
                    ..no_edit()
                })
                .expect("reads"),
                DecisionProposalEdit {
                    title: Some("Raise the block size to 4 MB?".to_string()),
                    description: None,
                    url: Some(Some("https://example.com".to_string())),
                }
            );
            assert_eq!(
                proposal_edit(&DecisionProposalEditFFI {
                    remove_url: true,
                    ..no_edit()
                })
                .expect("reads")
                .url,
                Some(None)
            );
            let refused = proposal_edit(&DecisionProposalEditFFI {
                url: url.as_ptr(),
                remove_url: true,
                ..no_edit()
            })
            .expect_err("contradictory");
            assert_eq!(
                refused.code,
                PlatformWalletFFIResultCode::ErrorInvalidParameter
            );
        }
    }

    #[test]
    fn should_refuse_a_string_that_is_not_utf8() {
        let not_utf8 = CString::new(vec![0xff, 0xfe]).expect("no NUL");
        let description = c_string("Yes doubles it; no keeps it.");
        unsafe {
            let refused = proposal_input(&DecisionProposalParamsFFI {
                title: not_utf8.as_ptr(),
                description: description.as_ptr(),
                url: ptr::null(),
                pass_percent: 60,
                minimum_turnout_percent: 10,
                end_date_ms: 1_701_209_600_000,
            })
            .expect_err("not UTF-8");
            assert_eq!(
                refused.code,
                PlatformWalletFFIResultCode::ErrorUtf8Conversion
            );
            for edit in [
                DecisionProposalEditFFI {
                    title: not_utf8.as_ptr(),
                    ..no_edit()
                },
                DecisionProposalEditFFI {
                    description: not_utf8.as_ptr(),
                    ..no_edit()
                },
                DecisionProposalEditFFI {
                    url: not_utf8.as_ptr(),
                    ..no_edit()
                },
            ] {
                let refused = proposal_edit(&edit).expect_err("not UTF-8");
                assert_eq!(
                    refused.code,
                    PlatformWalletFFIResultCode::ErrorUtf8Conversion
                );
            }
        }
    }

    /// Every pointer is checked before the wallet is looked up, and the
    /// JSON out-param is left null on the way out.
    #[test]
    fn should_refuse_null_pointers() {
        let id = [0u8; 32];
        let mut out_id = [0u8; 32];
        let mut out_json: *mut c_char = ptr::null_mut();
        let title = c_string("Raise the block size?");
        let description = c_string("Yes doubles it; no keeps it.");
        let params = DecisionProposalParamsFFI {
            title: title.as_ptr(),
            description: description.as_ptr(),
            url: ptr::null(),
            pass_percent: 60,
            minimum_turnout_percent: 10,
            end_date_ms: 1_701_209_600_000,
        };
        let edit = no_edit();
        // A dangling but non-null signer: every call below returns at a
        // pointer check before the signer is read.
        let signer = ptr::NonNull::<SignerHandle>::dangling().as_ptr();
        unsafe {
            for result in [
                platform_wallet_decision_proposal_create(
                    0,
                    id.as_ptr(),
                    ptr::null(),
                    signer,
                    out_id.as_mut_ptr(),
                    &mut out_json,
                ),
                platform_wallet_decision_proposal_create(
                    0,
                    id.as_ptr(),
                    &params,
                    ptr::null_mut(),
                    out_id.as_mut_ptr(),
                    &mut out_json,
                ),
                platform_wallet_decision_proposal_create(
                    0,
                    id.as_ptr(),
                    &params,
                    signer,
                    ptr::null_mut(),
                    &mut out_json,
                ),
                platform_wallet_decision_proposal_create(
                    0,
                    id.as_ptr(),
                    &params,
                    signer,
                    out_id.as_mut_ptr(),
                    ptr::null_mut(),
                ),
                platform_wallet_decision_proposal_edit(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    ptr::null(),
                    signer,
                    &mut out_json,
                ),
                platform_wallet_decision_proposal_edit(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    &edit,
                    ptr::null_mut(),
                    &mut out_json,
                ),
                platform_wallet_decision_proposal_edit(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    &edit,
                    signer,
                    ptr::null_mut(),
                ),
            ] {
                assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
            }
            assert!(out_json.is_null());
        }
    }

    /// An argument the wallet cannot read is refused before the wallet is
    /// looked up, and the JSON out-param is left null.
    #[test]
    fn should_refuse_unreadable_arguments_before_the_wallet_is_looked_up() {
        let id = [0u8; 32];
        let mut out_id = [0u8; 32];
        let mut out_json: *mut c_char = ptr::null_mut();
        let description = c_string("Yes doubles it; no keeps it.");
        let no_title = DecisionProposalParamsFFI {
            title: ptr::null(),
            description: description.as_ptr(),
            url: ptr::null(),
            pass_percent: 60,
            minimum_turnout_percent: 10,
            end_date_ms: 1_701_209_600_000,
        };
        let url = c_string("https://example.com");
        let contradictory = DecisionProposalEditFFI {
            url: url.as_ptr(),
            remove_url: true,
            ..no_edit()
        };
        let signer = ptr::NonNull::<SignerHandle>::dangling().as_ptr();
        unsafe {
            let created = platform_wallet_decision_proposal_create(
                0,
                id.as_ptr(),
                &no_title,
                signer,
                out_id.as_mut_ptr(),
                &mut out_json,
            );
            assert_eq!(created.code, PlatformWalletFFIResultCode::ErrorNullPointer);
            let edited = platform_wallet_decision_proposal_edit(
                0,
                id.as_ptr(),
                id.as_ptr(),
                &contradictory,
                signer,
                &mut out_json,
            );
            assert_eq!(
                edited.code,
                PlatformWalletFFIResultCode::ErrorInvalidParameter
            );
            assert!(out_json.is_null());
        }
    }

    /// A proposal the edit names and does not find is the "does not
    /// exist" case.
    #[test]
    fn should_report_a_missing_proposal_as_not_found() {
        let result: PlatformWalletFFIResult = PlatformWalletError::DecisionProposalNotFound {
            proposal_id: Identifier::from([4; 32]),
        }
        .into();
        assert_eq!(result.code, PlatformWalletFFIResultCode::NotFound);
    }

    /// Platform refuses an edit whose proposal another edit moved on since
    /// it was read; the host gets the refusal's consensus code beside the
    /// result code, to read the proposal again.
    #[test]
    fn should_carry_the_consensus_code_of_a_stale_edit() {
        let cause = ConsensusError::from(InvalidDocumentRevisionError::new(
            Identifier::from([4; 32]),
            Some(2),
            2,
        ));
        let result: PlatformWalletFFIResult = PlatformWalletError::Sdk(
            dash_sdk::Error::StateTransitionBroadcastError(StateTransitionBroadcastError {
                code: 40106,
                message: cause.to_string(),
                cause: Some(cause),
            }),
        )
        .into();
        assert_eq!(result.consensus_code, 40106);
        assert_eq!(
            result.consensus_kind,
            PlatformWalletFFIConsensusErrorKind::State
        );
    }
}
