//! FFI bindings for multiple choice proposal create and edit, and for
//! entering a choice, on `IdentityWallet` (protocol version 15).
//!
//! Thin: each function reads its C arguments, calls
//! `IdentityWallet::create_multiple_choice_proposal_with_signer`,
//! `IdentityWallet::edit_multiple_choice_proposal_with_signer` or
//! `IdentityWallet::enter_choice_with_signer` on the platform-wallet 8 MB
//! worker stack, and writes the confirmed document back. rs-sdk and the
//! wallet decide everything else: the documents' properties, the contest
//! fund a choice states, the signing key, refusing an edit that changes
//! nothing, and the revision an edit is put at.

use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;
use std::slice;

use dash_sdk::platform::decision_proposals::multiple_choice::{
    ChoiceInput, DescriptionDocument, MultipleChoiceProposalEdit, MultipleChoiceProposalInput,
};
use dpp::document::{Document, DocumentV0Getters};
use dpp::prelude::Identifier;
use platform_wallet::PlatformWalletError;
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::check_ptr;
use crate::dashpay_profile::decode_opt_c_str;
use crate::decision_proposal::required_string;
use crate::document::confirmed_document_to_json;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::read_identifier;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// The document a multiple choice proposal points at for what is being
/// decided. Every field is required. Platform refuses a document that is
/// not of a moderated type the elected team of its contract may delete.
#[repr(C)]
pub struct DescriptionDocumentFFI {
    /// The document's 32-byte id.
    pub id: *const u8,
    /// The 32-byte id of the contract it lives in.
    pub contract_id: *const u8,
    /// Its document type, NUL-terminated UTF-8.
    pub document_type: *const c_char,
}

/// What a new multiple choice proposal asks
/// (`platform_wallet_multiple_choice_proposal_create`). Platform refuses
/// values outside the limits noted here when the proposal is created, and
/// a proposal that gives neither a description document nor a url;
/// nothing is checked before.
#[repr(C)]
pub struct MultipleChoiceProposalParamsFFI {
    /// The question, in one line: 1 to 128 characters. NUL-terminated
    /// UTF-8, required.
    pub title: *const c_char,
    /// The document saying what is being decided; null for none.
    pub description_document: *const DescriptionDocumentFFI,
    /// Where what is being decided is described, for a proposal that
    /// points at no description document, or where it is discussed, for
    /// one that does: a URI of at most 512 characters. NUL-terminated
    /// UTF-8; null for none.
    pub url: *const c_char,
    /// Whether anyone may enter choices; only the proposal's owner may
    /// when it is false. It never changes.
    pub open: bool,
}

/// What a multiple choice proposal changes to
/// (`platform_wallet_multiple_choice_proposal_edit`). A field left null
/// keeps its value. Platform accepts the edit for two hours after the
/// proposal was created, and refuses one that leaves it with neither a
/// description document nor a url; whether it is open never changes.
#[repr(C)]
pub struct MultipleChoiceProposalEditFFI {
    /// The new title, NUL-terminated UTF-8; null keeps it.
    pub title: *const c_char,
    /// The new description document; null keeps it, unless
    /// `remove_description_document` is set.
    pub description_document: *const DescriptionDocumentFFI,
    /// Removes the description document, leaving the proposal described
    /// at its url alone. Setting it together with a non-null
    /// `description_document` is refused with `ErrorInvalidParameter`.
    pub remove_description_document: bool,
    /// The new url, NUL-terminated UTF-8; null keeps it, unless
    /// `remove_url` is set.
    pub url: *const c_char,
    /// Removes the url. Setting it together with a non-null `url` is
    /// refused with `ErrorInvalidParameter`.
    pub remove_url: bool,
}

/// An answer to enter for a multiple choice proposal
/// (`platform_wallet_multiple_choice_proposal_enter_choice`). Platform
/// refuses values outside the limits noted here; nothing is checked
/// before.
#[repr(C)]
pub struct ChoiceParamsFFI {
    /// The answer, in one line: 1 to 128 characters. NUL-terminated
    /// UTF-8, required.
    pub title: *const c_char,
    /// What voting for the answer means: 1 to 4096 characters.
    /// NUL-terminated UTF-8; null for none.
    pub description: *const c_char,
}

/// The description document `description_document` names, `None` when it
/// is null.
///
/// # Safety
/// `description_document` must be null or point to a
/// `DescriptionDocumentFFI` whose `id` and `contract_id` are null or point
/// to 32 readable bytes and whose `document_type` is null or
/// NUL-terminated.
unsafe fn description_document(
    description_document: *const DescriptionDocumentFFI,
) -> Result<Option<DescriptionDocument>, PlatformWalletFFIResult> {
    let Some(description_document) = description_document.as_ref() else {
        return Ok(None);
    };
    Ok(Some(DescriptionDocument {
        id: read_identifier(description_document.id)?,
        contract_id: read_identifier(description_document.contract_id)?,
        document_type: required_string(
            description_document.document_type,
            "description_document.document_type",
        )?,
    }))
}

/// `value` as an edit of an optional field: `Some(Some(value))` sets it,
/// `Some(None)` removes it and `None` keeps it. Both at once is refused.
fn set_or_remove<T>(
    value: Option<T>,
    remove: bool,
    field: &str,
) -> Result<Option<Option<T>>, PlatformWalletFFIResult> {
    match (value, remove) {
        (Some(_), true) => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!("{field} and remove_{field} cannot both be set"),
        )),
        (Some(value), false) => Ok(Some(Some(value))),
        (None, true) => Ok(Some(None)),
        (None, false) => Ok(None),
    }
}

/// The proposal input `params` holds.
///
/// # Safety
/// `params` must point to a `MultipleChoiceProposalParamsFFI` whose
/// pointer fields are null or valid as documented on it.
unsafe fn proposal_input(
    params: &MultipleChoiceProposalParamsFFI,
) -> Result<MultipleChoiceProposalInput, PlatformWalletFFIResult> {
    Ok(MultipleChoiceProposalInput {
        title: required_string(params.title, "title")?,
        description_document: description_document(params.description_document)?,
        url: decode_opt_c_str(params.url)?,
        open: params.open,
    })
}

/// The edit `edit` holds.
///
/// # Safety
/// `edit` must point to a `MultipleChoiceProposalEditFFI` whose pointer
/// fields are null or valid as documented on it.
unsafe fn proposal_edit(
    edit: &MultipleChoiceProposalEditFFI,
) -> Result<MultipleChoiceProposalEdit, PlatformWalletFFIResult> {
    Ok(MultipleChoiceProposalEdit {
        title: decode_opt_c_str(edit.title)?,
        description_document: set_or_remove(
            description_document(edit.description_document)?,
            edit.remove_description_document,
            "description_document",
        )?,
        url: set_or_remove(decode_opt_c_str(edit.url)?, edit.remove_url, "url")?,
    })
}

/// The choice input `params` holds for the proposal `proposal_id`.
///
/// # Safety
/// `params` must point to a `ChoiceParamsFFI` whose string fields are null
/// or NUL-terminated.
unsafe fn choice_input(
    proposal_id: Identifier,
    params: &ChoiceParamsFFI,
) -> Result<ChoiceInput, PlatformWalletFFIResult> {
    Ok(ChoiceInput {
        proposal_id,
        title: required_string(params.title, "title")?,
        description: decode_opt_c_str(params.description)?,
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

/// Create + broadcast a multiple choice proposal of `owner_identity_id`
/// asking what `params` says, signed via the external `signer_handle`.
///
/// Goes through `IdentityWallet::create_multiple_choice_proposal_with_signer`:
/// rs-sdk builds the proposal; Platform charges the owner the multiple
/// choice proposal processing fee (0.2 Dash,
/// `dash_sdk_multiple_choice_proposal_get_processing_fee`) on top of the
/// fees, which the create does not state. The wallet signs with the
/// owner's first AUTHENTICATION + ECDSA key at a security level the
/// `multipleChoiceProposal` type accepts that the signer holds, broadcasts
/// on the platform-wallet 8 MB worker stack, waits for the confirmed
/// document and keeps the owner's proven balance. A network below protocol
/// version 15 has no decision proposals, which is an error.
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
/// - `params` must point to a `MultipleChoiceProposalParamsFFI` whose
///   `title` is a NUL-terminated string, whose `url` is null or one, and
///   whose `description_document` is null or points to a
///   `DescriptionDocumentFFI` with 32 readable bytes at `id` and at
///   `contract_id` and a NUL-terminated `document_type`.
/// - `signer_handle` must be a live signer, which the caller keeps owning.
/// - `out_proposal_id` must point to 32 writable bytes and
///   `out_document_json` must be a valid, writable pointer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_multiple_choice_proposal_create(
    wallet_handle: Handle,
    owner_identity_id: *const u8,
    params: *const MultipleChoiceProposalParamsFFI,
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
                .create_multiple_choice_proposal_with_signer(&owner_id, &input, signer)
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

/// Give the multiple choice proposal `proposal_id` of `owner_identity_id`
/// what `edit` changes, and broadcast the replace, signed via the external
/// `signer_handle`.
///
/// Goes through `IdentityWallet::edit_multiple_choice_proposal_with_signer`,
/// which reads the proposal once, applies the edit and the revision after
/// the one read (rs-sdk refuses an edit that changes nothing), signs with a
/// key chosen as the create chooses it and puts that document. It never
/// reads the proposal again before the put: if another edit lands in
/// between, Platform refuses this one (`consensus_code` 40106,
/// `InvalidDocumentRevisionError`) rather than overwrite it, and the host
/// reads the proposal again before offering the edit anew. Platform also
/// refuses an edit more than two hours after the proposal was created, and
/// one that leaves it with neither a description document nor a url.
///
/// On success the confirmed document's canonical JSON, at its new
/// revision, is written to `*out_document_json`; release it with
/// `platform_wallet_string_free`. On any error it is left null.
///
/// # Errors
/// - `NotFound` when there is no such proposal;
/// - `ErrorInvalidParameter` when the proposal belongs to another
///   identity, the edit changes nothing, `url` and `remove_url` are both
///   set, or `description_document` and `remove_description_document` are
///   both set;
/// - `ErrorInvalidIdentifier` for a `description_document` whose `id` or
///   `contract_id` is null, and `ErrorNullPointer` for one whose
///   `document_type` is;
/// - `ErrorUtf8Conversion` for a string field that is not UTF-8;
/// - a refusal by Platform carries its `consensus_code`.
///
/// # Safety
/// - `owner_identity_id` and `proposal_id` must each point to 32 readable
///   bytes.
/// - `edit` must point to a `MultipleChoiceProposalEditFFI` whose string
///   fields are null or NUL-terminated and whose `description_document` is
///   null or valid as `platform_wallet_multiple_choice_proposal_create`
///   requires.
/// - `signer_handle` must be a live signer, which the caller keeps owning.
/// - `out_document_json` must be a valid, writable pointer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_multiple_choice_proposal_edit(
    wallet_handle: Handle,
    owner_identity_id: *const u8,
    proposal_id: *const u8,
    edit: *const MultipleChoiceProposalEditFFI,
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
                .edit_multiple_choice_proposal_with_signer(&owner_id, &proposal_id, &edit, signer)
                .await?;
            confirmed_document_to_json(&confirmed)
        })
    });
    let result = unwrap_option_or_return!(option);
    let document_json = unwrap_result_or_return!(result);

    unwrap_result_or_return!(write_document_json(document_json, out_document_json));
    PlatformWalletFFIResult::ok()
}

/// Enter the choice `params` describes, owned by `owner_identity_id`, for
/// the multiple choice proposal `proposal_id`, and broadcast it, signed
/// via the external `signer_handle`.
///
/// Goes through `IdentityWallet::enter_choice_with_signer`: rs-sdk builds
/// the choice, and its put reads the fund of the contest among the
/// proposal's choices just before it is signed and states it
/// (`dash_sdk_multiple_choice_proposal_get_choice_fund_to_join` reads the
/// same amount), which the owner pays on top of the fees. The first choice
/// opens the vote. Signed, broadcast and kept like
/// `platform_wallet_multiple_choice_proposal_create`. Platform refuses a
/// choice for a proposal that is not open from anyone but its owner, and
/// one entered once the vote's join window closed. A choice is never
/// edited.
///
/// On success the choice's 32-byte id, the id a masternode vote towards it
/// names, is written to `out_choice_id`, and the confirmed document's
/// canonical JSON to `*out_document_json`; release it with
/// `platform_wallet_string_free`. On any error `*out_document_json` is
/// left null. A refusal by Platform carries its `consensus_code`.
///
/// # Safety
/// - `owner_identity_id` and `proposal_id` must each point to 32 readable
///   bytes.
/// - `params` must point to a `ChoiceParamsFFI` whose `title` is a
///   NUL-terminated string and whose `description` is null or one.
/// - `signer_handle` must be a live signer, which the caller keeps owning.
/// - `out_choice_id` must point to 32 writable bytes and
///   `out_document_json` must be a valid, writable pointer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_multiple_choice_proposal_enter_choice(
    wallet_handle: Handle,
    owner_identity_id: *const u8,
    proposal_id: *const u8,
    params: *const ChoiceParamsFFI,
    signer_handle: *mut SignerHandle,
    out_choice_id: *mut u8,
    out_document_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(params);
    check_ptr!(signer_handle);
    check_ptr!(out_choice_id);
    check_ptr!(out_document_json);
    *out_document_json = ptr::null_mut();

    let owner_id = unwrap_result_or_return!(read_identifier(owner_identity_id));
    let proposal_id = unwrap_result_or_return!(read_identifier(proposal_id));
    let input = unwrap_result_or_return!(choice_input(proposal_id, &*params));
    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            let confirmed: Document = identity_wallet
                .enter_choice_with_signer(&owner_id, &input, signer)
                .await?;
            let json = confirmed_document_to_json(&confirmed)?;
            Ok::<_, PlatformWalletError>((confirmed.id(), json))
        })
    });
    let result = unwrap_option_or_return!(option);
    let (choice_id, document_json) = unwrap_result_or_return!(result);

    unwrap_result_or_return!(write_document_json(document_json, out_document_json));
    slice::from_raw_parts_mut(out_choice_id, 32).copy_from_slice(choice_id.as_slice());
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c_string(value: &str) -> CString {
        CString::new(value).expect("no NUL")
    }

    fn no_edit() -> MultipleChoiceProposalEditFFI {
        MultipleChoiceProposalEditFFI {
            title: ptr::null(),
            description_document: ptr::null(),
            remove_description_document: false,
            url: ptr::null(),
            remove_url: false,
        }
    }

    const DOCUMENT_ID: [u8; 32] = [5; 32];
    const CONTRACT_ID: [u8; 32] = [6; 32];

    fn expected_description_document() -> DescriptionDocument {
        DescriptionDocument {
            id: Identifier::from(DOCUMENT_ID),
            contract_id: Identifier::from(CONTRACT_ID),
            document_type: "post".to_string(),
        }
    }

    #[test]
    fn should_read_the_proposal_params() {
        let title = c_string("Which block size?");
        let url = c_string("https://forum.dash.org/t/block-size");
        let document_type = c_string("post");
        let described = DescriptionDocumentFFI {
            id: DOCUMENT_ID.as_ptr(),
            contract_id: CONTRACT_ID.as_ptr(),
            document_type: document_type.as_ptr(),
        };
        let mut params = MultipleChoiceProposalParamsFFI {
            title: title.as_ptr(),
            description_document: &described,
            url: url.as_ptr(),
            open: true,
        };
        unsafe {
            assert_eq!(
                proposal_input(&params).expect("reads"),
                MultipleChoiceProposalInput {
                    title: "Which block size?".to_string(),
                    description_document: Some(expected_description_document()),
                    url: Some("https://forum.dash.org/t/block-size".to_string()),
                    open: true,
                }
            );
            params.url = ptr::null();
            params.description_document = ptr::null();
            params.open = false;
            let read = proposal_input(&params).expect("reads");
            assert_eq!(read.url, None);
            assert_eq!(read.description_document, None);
            assert!(!read.open);
            params.title = ptr::null();
            let refused = proposal_input(&params).expect_err("no title");
            assert_eq!(refused.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        }
    }

    /// A description document is all three of its fields or nothing: one
    /// with a null field cannot be read.
    #[test]
    fn should_refuse_a_description_document_with_a_null_field() {
        let document_type = c_string("post");
        let complete = DescriptionDocumentFFI {
            id: DOCUMENT_ID.as_ptr(),
            contract_id: CONTRACT_ID.as_ptr(),
            document_type: document_type.as_ptr(),
        };
        unsafe {
            for no_id in [
                DescriptionDocumentFFI {
                    id: ptr::null(),
                    ..complete
                },
                DescriptionDocumentFFI {
                    contract_id: ptr::null(),
                    ..complete
                },
            ] {
                let refused = description_document(&no_id).expect_err("no id");
                assert_eq!(
                    refused.code,
                    PlatformWalletFFIResultCode::ErrorInvalidIdentifier
                );
            }
            let refused = description_document(&DescriptionDocumentFFI {
                document_type: ptr::null(),
                ..complete
            })
            .expect_err("no document type");
            assert_eq!(refused.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        }
    }

    #[test]
    fn should_read_the_edit_and_refuse_a_field_both_set_and_removed() {
        let title = c_string("Which block size, then?");
        let url = c_string("https://example.com");
        let document_type = c_string("post");
        let described = DescriptionDocumentFFI {
            id: DOCUMENT_ID.as_ptr(),
            contract_id: CONTRACT_ID.as_ptr(),
            document_type: document_type.as_ptr(),
        };
        unsafe {
            assert_eq!(
                proposal_edit(&no_edit()).expect("reads"),
                MultipleChoiceProposalEdit::default()
            );
            assert_eq!(
                proposal_edit(&MultipleChoiceProposalEditFFI {
                    title: title.as_ptr(),
                    description_document: &described,
                    url: url.as_ptr(),
                    ..no_edit()
                })
                .expect("reads"),
                MultipleChoiceProposalEdit {
                    title: Some("Which block size, then?".to_string()),
                    description_document: Some(Some(expected_description_document())),
                    url: Some(Some("https://example.com".to_string())),
                }
            );
            assert_eq!(
                proposal_edit(&MultipleChoiceProposalEditFFI {
                    remove_description_document: true,
                    remove_url: true,
                    ..no_edit()
                })
                .expect("reads"),
                MultipleChoiceProposalEdit {
                    title: None,
                    description_document: Some(None),
                    url: Some(None),
                }
            );
            for contradictory in [
                MultipleChoiceProposalEditFFI {
                    url: url.as_ptr(),
                    remove_url: true,
                    ..no_edit()
                },
                MultipleChoiceProposalEditFFI {
                    description_document: &described,
                    remove_description_document: true,
                    ..no_edit()
                },
            ] {
                let refused = proposal_edit(&contradictory).expect_err("contradictory");
                assert_eq!(
                    refused.code,
                    PlatformWalletFFIResultCode::ErrorInvalidParameter
                );
            }
        }
    }

    #[test]
    fn should_read_the_choice_params() {
        let proposal_id = Identifier::from([7; 32]);
        let title = c_string("Double it");
        let description = c_string("Twice the size it is today.");
        let mut params = ChoiceParamsFFI {
            title: title.as_ptr(),
            description: description.as_ptr(),
        };
        unsafe {
            assert_eq!(
                choice_input(proposal_id, &params).expect("reads"),
                ChoiceInput {
                    proposal_id,
                    title: "Double it".to_string(),
                    description: Some("Twice the size it is today.".to_string()),
                }
            );
            params.description = ptr::null();
            assert_eq!(
                choice_input(proposal_id, &params)
                    .expect("reads")
                    .description,
                None
            );
            params.title = ptr::null();
            let refused = choice_input(proposal_id, &params).expect_err("no title");
            assert_eq!(refused.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        }
    }

    #[test]
    fn should_refuse_a_string_that_is_not_utf8() {
        let not_utf8 = CString::new(vec![0xff, 0xfe]).expect("no NUL");
        let proposal_id = Identifier::from([7; 32]);
        unsafe {
            let refused = proposal_input(&MultipleChoiceProposalParamsFFI {
                title: not_utf8.as_ptr(),
                description_document: ptr::null(),
                url: ptr::null(),
                open: true,
            })
            .expect_err("not UTF-8");
            assert_eq!(
                refused.code,
                PlatformWalletFFIResultCode::ErrorUtf8Conversion
            );
            let not_utf8_type = DescriptionDocumentFFI {
                id: DOCUMENT_ID.as_ptr(),
                contract_id: CONTRACT_ID.as_ptr(),
                document_type: not_utf8.as_ptr(),
            };
            for edit in [
                MultipleChoiceProposalEditFFI {
                    title: not_utf8.as_ptr(),
                    ..no_edit()
                },
                MultipleChoiceProposalEditFFI {
                    url: not_utf8.as_ptr(),
                    ..no_edit()
                },
                MultipleChoiceProposalEditFFI {
                    description_document: &not_utf8_type,
                    ..no_edit()
                },
            ] {
                let refused = proposal_edit(&edit).expect_err("not UTF-8");
                assert_eq!(
                    refused.code,
                    PlatformWalletFFIResultCode::ErrorUtf8Conversion
                );
            }
            for params in [
                ChoiceParamsFFI {
                    title: not_utf8.as_ptr(),
                    description: ptr::null(),
                },
                ChoiceParamsFFI {
                    title: not_utf8.as_ptr(),
                    description: not_utf8.as_ptr(),
                },
            ] {
                let refused = choice_input(proposal_id, &params).expect_err("not UTF-8");
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
        let title = c_string("Which block size?");
        let url = c_string("https://forum.dash.org/t/block-size");
        let params = MultipleChoiceProposalParamsFFI {
            title: title.as_ptr(),
            description_document: ptr::null(),
            url: url.as_ptr(),
            open: true,
        };
        let edit = no_edit();
        let choice = ChoiceParamsFFI {
            title: title.as_ptr(),
            description: ptr::null(),
        };
        // A dangling but non-null signer: every call below returns at a
        // pointer check before the signer is read.
        let signer = ptr::NonNull::<SignerHandle>::dangling().as_ptr();
        unsafe {
            for result in [
                platform_wallet_multiple_choice_proposal_create(
                    0,
                    id.as_ptr(),
                    ptr::null(),
                    signer,
                    out_id.as_mut_ptr(),
                    &mut out_json,
                ),
                platform_wallet_multiple_choice_proposal_create(
                    0,
                    id.as_ptr(),
                    &params,
                    ptr::null_mut(),
                    out_id.as_mut_ptr(),
                    &mut out_json,
                ),
                platform_wallet_multiple_choice_proposal_create(
                    0,
                    id.as_ptr(),
                    &params,
                    signer,
                    ptr::null_mut(),
                    &mut out_json,
                ),
                platform_wallet_multiple_choice_proposal_create(
                    0,
                    id.as_ptr(),
                    &params,
                    signer,
                    out_id.as_mut_ptr(),
                    ptr::null_mut(),
                ),
                platform_wallet_multiple_choice_proposal_edit(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    ptr::null(),
                    signer,
                    &mut out_json,
                ),
                platform_wallet_multiple_choice_proposal_edit(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    &edit,
                    ptr::null_mut(),
                    &mut out_json,
                ),
                platform_wallet_multiple_choice_proposal_edit(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    &edit,
                    signer,
                    ptr::null_mut(),
                ),
                platform_wallet_multiple_choice_proposal_enter_choice(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    ptr::null(),
                    signer,
                    out_id.as_mut_ptr(),
                    &mut out_json,
                ),
                platform_wallet_multiple_choice_proposal_enter_choice(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    &choice,
                    ptr::null_mut(),
                    out_id.as_mut_ptr(),
                    &mut out_json,
                ),
                platform_wallet_multiple_choice_proposal_enter_choice(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    &choice,
                    signer,
                    ptr::null_mut(),
                    &mut out_json,
                ),
                platform_wallet_multiple_choice_proposal_enter_choice(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    &choice,
                    signer,
                    out_id.as_mut_ptr(),
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
        let no_title = MultipleChoiceProposalParamsFFI {
            title: ptr::null(),
            description_document: ptr::null(),
            url: ptr::null(),
            open: true,
        };
        let url = c_string("https://example.com");
        let contradictory = MultipleChoiceProposalEditFFI {
            url: url.as_ptr(),
            remove_url: true,
            ..no_edit()
        };
        let untitled_choice = ChoiceParamsFFI {
            title: ptr::null(),
            description: ptr::null(),
        };
        let signer = ptr::NonNull::<SignerHandle>::dangling().as_ptr();
        unsafe {
            let created = platform_wallet_multiple_choice_proposal_create(
                0,
                id.as_ptr(),
                &no_title,
                signer,
                out_id.as_mut_ptr(),
                &mut out_json,
            );
            assert_eq!(created.code, PlatformWalletFFIResultCode::ErrorNullPointer);
            let edited = platform_wallet_multiple_choice_proposal_edit(
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
            let entered = platform_wallet_multiple_choice_proposal_enter_choice(
                0,
                id.as_ptr(),
                id.as_ptr(),
                &untitled_choice,
                signer,
                out_id.as_mut_ptr(),
                &mut out_json,
            );
            assert_eq!(entered.code, PlatformWalletFFIResultCode::ErrorNullPointer);
            assert!(out_json.is_null());
        }
    }
}
