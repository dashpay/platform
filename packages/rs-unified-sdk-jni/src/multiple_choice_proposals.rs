//! JNI exports for multiple choice proposals (protocol version 15): questions whose answers
//! are the choices entered for them, as `multipleChoiceProposal` and `choice` documents of
//! the decision proposals system contract. Each export marshals ONE C function: a
//! `dash_sdk_multiple_choice_proposal_*` function of `rs-sdk-ffi` (the reads, the two amounts
//! and the masternode vote), or `platform_wallet_multiple_choice_proposal_create` / `_edit` /
//! `_enter_choice` of `platform-wallet-ffi` (the writes, which the platform wallet builds,
//! signs, broadcasts and confirms itself). It reads the JVM arguments, calls the C function,
//! copies the result into a JVM value and frees every Rust allocation the result carried
//! before returning.
//!
//! Errors and argument checks follow `crate::decision_proposals`, whose marshalling helpers
//! these exports share: a `DashSDKResult` error and a `PlatformWalletFFIResult` error each
//! throw `DashSDKException` by their crate's convention, and only an argument the JVM value
//! cannot carry into the C type (a null required string, a byte array of the wrong length, a
//! number out of the C type's range) is refused here, with code 1 (`InvalidParameter`).
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.MultipleChoiceProposalsNative`.

use crate::decision_proposals::{
    c_ptr, is_byte_array_of_len, read_bytes32, read_cstring, read_key32_zeroizing,
    read_optional_cstring, string_or_null, take_wallet_json, to_unsigned,
};
use crate::results::take_error;
use crate::support::{guard, net_from_ord, take_pwffi_error, throw_sdk_exception};
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jboolean, jbyte, jint, jlong, jstring};
use jni::JNIEnv;
use platform_wallet_ffi::handle::Handle;
use platform_wallet_ffi::{
    platform_wallet_multiple_choice_proposal_create, platform_wallet_multiple_choice_proposal_edit,
    platform_wallet_multiple_choice_proposal_enter_choice, ChoiceParamsFFI, DescriptionDocumentFFI,
    MultipleChoiceProposalEditFFI, MultipleChoiceProposalParamsFFI,
};
use rs_sdk_ffi::{
    dash_sdk_multiple_choice_proposal_cast_vote, dash_sdk_multiple_choice_proposal_fetch,
    dash_sdk_multiple_choice_proposal_get_choice_fund_to_join,
    dash_sdk_multiple_choice_proposal_get_kept_choice,
    dash_sdk_multiple_choice_proposal_get_masternode_vote,
    dash_sdk_multiple_choice_proposal_get_processing_fee,
    dash_sdk_multiple_choice_proposal_get_vote,
    dash_sdk_multiple_choice_proposal_list_by_created_at,
    dash_sdk_multiple_choice_proposal_list_by_owner, SDKHandle, SignerHandle,
};
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

// ── Marshalling helpers ──────────────────────────────────────────────────

/// Read a nullable 32-byte array.
///
/// `Some(None)` for a null array, `Some(Some(_))` for one of 32 bytes, and `None` after
/// throwing for any other length (the caller bails out).
fn read_optional_bytes32(
    env: &mut JNIEnv,
    arr: &JByteArray,
    field: &str,
) -> Option<Option<[u8; 32]>> {
    if arr.is_null() {
        return Some(None);
    }
    read_bytes32(env, arr, field).map(Some)
}

/// The three parts of a description document as the JVM handed them over, each absent when
/// its argument was null. They are kept here for as long as the C struct points into them.
struct DescriptionDocumentArgs {
    id: Option<[u8; 32]>,
    contract_id: Option<[u8; 32]>,
    document_type: Option<CString>,
}

impl DescriptionDocumentArgs {
    /// Read the three arguments; `None` after throwing for one the C type cannot carry.
    fn read(
        env: &mut JNIEnv,
        id: &JByteArray,
        contract_id: &JByteArray,
        document_type: &JString,
    ) -> Option<Self> {
        Some(Self {
            id: read_optional_bytes32(env, id, "descriptionDocumentId")?,
            contract_id: read_optional_bytes32(env, contract_id, "descriptionDocumentContractId")?,
            document_type: read_optional_cstring(env, document_type, "descriptionDocumentType")?,
        })
    }

    /// The C struct pointing at these parts: `None` (a null `description_document`) when all
    /// three are absent, and otherwise a struct whose pointer is null for each absent part,
    /// which the platform wallet refuses.
    fn as_ffi(&self) -> Option<DescriptionDocumentFFI> {
        if self.id.is_none() && self.contract_id.is_none() && self.document_type.is_none() {
            return None;
        }
        Some(DescriptionDocumentFFI {
            id: self.id.as_ref().map_or(ptr::null(), |id| id.as_ptr()),
            contract_id: self
                .contract_id
                .as_ref()
                .map_or(ptr::null(), |contract_id| contract_id.as_ptr()),
            document_type: c_ptr(&self.document_type),
        })
    }
}

/// The pointer the C structs take for an optional description document: null for `None`.
fn description_document_ptr(
    described: &Option<DescriptionDocumentFFI>,
) -> *const DescriptionDocumentFFI {
    described
        .as_ref()
        .map_or(ptr::null(), |described| described as *const _)
}

/// Write the id a paid create handed back into `out_id`, a `byte[32]` checked before the
/// create. Throws and returns `false` when the write still fails, with a message that says
/// `what` exists so that a caller does not retry a paid create.
fn write_created_id(env: &mut JNIEnv, out_id: &JByteArray, id: [u8; 32], what: &str) -> bool {
    let id = id.map(|b| b as jbyte);
    if env.set_byte_array_region(out_id, 0, &id).is_err() {
        let _ = env.exception_clear();
        throw_sdk_exception(
            env,
            99,
            &format!("{what} was created, but its id could not be returned"),
        );
        return false;
    }
    true
}

// ── Create, edit and enter a choice (platform wallet) ────────────────────

/// Create + broadcast a multiple choice proposal of `ownerId`, signed via `signerHandle`:
/// the JNI bridge over `platform_wallet_multiple_choice_proposal_create`. The wallet builds
/// the proposal, picks the signing key itself, broadcasts and waits for Platform to confirm;
/// Platform charges the owner the multiple choice proposal processing fee on top of the fees.
///
/// The description document is its three arguments together, all null for none; `url` may be
/// null (no url). `outProposalId` must be a non-null `byte[32]`, checked before anything is
/// broadcast so that the id of a paid create can always be written back; it receives the
/// proposal's id.
///
/// Returns the confirmed document's canonical JSON, its Rust string freed here. Null after
/// throwing; a refusal by Platform carries its consensus code.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_MultipleChoiceProposalsNative_create(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    owner_id: JByteArray,
    title: JString,
    description_document_id: JByteArray,
    description_document_contract_id: JByteArray,
    description_document_type: JString,
    url: JString,
    open: jboolean,
    signer_handle: jlong,
    out_proposal_id: JByteArray,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        // Checked before the paid create: once it succeeded, writing the id back into a
        // validated array allocates nothing and cannot go out of bounds.
        if !is_byte_array_of_len(env, &out_proposal_id, 32) {
            throw_sdk_exception(env, 1, "outProposalId must be a non-null byte[32]");
            return ptr::null_mut();
        }
        let Some(owner) = read_bytes32(env, &owner_id, "ownerId") else {
            return ptr::null_mut();
        };
        let Some(title) = read_cstring(env, &title, "title") else {
            return ptr::null_mut();
        };
        let Some(described) = DescriptionDocumentArgs::read(
            env,
            &description_document_id,
            &description_document_contract_id,
            &description_document_type,
        ) else {
            return ptr::null_mut();
        };
        let Some(url) = read_optional_cstring(env, &url, "url") else {
            return ptr::null_mut();
        };

        let described = described.as_ffi();
        let params = MultipleChoiceProposalParamsFFI {
            title: title.as_ptr(),
            description_document: description_document_ptr(&described),
            url: c_ptr(&url),
            open: open != 0,
        };
        let mut proposal_id = [0u8; 32];
        let mut out_json: *mut c_char = ptr::null_mut();
        // SAFETY: `owner` and `proposal_id` are 32 bytes, `params` and everything it points
        // to outlive the call, and the signer handle is the Kotlin manager's live signer.
        let result = unsafe {
            platform_wallet_multiple_choice_proposal_create(
                wallet_handle as Handle,
                owner.as_ptr(),
                &params,
                signer_handle as *mut SignerHandle,
                proposal_id.as_mut_ptr(),
                &mut out_json,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        // SAFETY: on success `out_json` is the wallet's string, freed by this call only.
        let Some(json) =
            (unsafe { take_wallet_json(env, out_json, "multiple choice proposal create") })
        else {
            return ptr::null_mut();
        };
        if !write_created_id(
            env,
            &out_proposal_id,
            proposal_id,
            "the multiple choice proposal",
        ) {
            return ptr::null_mut();
        }
        env.new_string(json)
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Give the multiple choice proposal `proposalId` of `ownerId` what the arguments change and
/// broadcast the replace, signed via `signerHandle`: the JNI bridge over
/// `platform_wallet_multiple_choice_proposal_edit`. The wallet reads the proposal once, edits
/// it at the revision after the one read and picks the signing key as the create does; if
/// another edit landed in between, Platform refuses this one (consensus code 40106) rather
/// than overwrite it.
///
/// `title` and `url` null keep their value, and so do the three description document
/// arguments when all are null; `removeDescriptionDocument` and `removeUrl` remove theirs
/// (the wallet refuses either together with a new value). `ownerId` and `proposalId` are
/// 32-byte ids.
///
/// Returns the confirmed document's canonical JSON at its new revision, its Rust string
/// freed here. Null after throwing; a proposal that does not exist is the wallet's
/// `NotFound`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_MultipleChoiceProposalsNative_edit(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    owner_id: JByteArray,
    proposal_id: JByteArray,
    title: JString,
    description_document_id: JByteArray,
    description_document_contract_id: JByteArray,
    description_document_type: JString,
    remove_description_document: jboolean,
    url: JString,
    remove_url: jboolean,
    signer_handle: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(owner) = read_bytes32(env, &owner_id, "ownerId") else {
            return ptr::null_mut();
        };
        let Some(proposal) = read_bytes32(env, &proposal_id, "proposalId") else {
            return ptr::null_mut();
        };
        let Some(title) = read_optional_cstring(env, &title, "title") else {
            return ptr::null_mut();
        };
        let Some(described) = DescriptionDocumentArgs::read(
            env,
            &description_document_id,
            &description_document_contract_id,
            &description_document_type,
        ) else {
            return ptr::null_mut();
        };
        let Some(url) = read_optional_cstring(env, &url, "url") else {
            return ptr::null_mut();
        };

        let described = described.as_ffi();
        let edit = MultipleChoiceProposalEditFFI {
            title: c_ptr(&title),
            description_document: description_document_ptr(&described),
            remove_description_document: remove_description_document != 0,
            url: c_ptr(&url),
            remove_url: remove_url != 0,
        };
        let mut out_json: *mut c_char = ptr::null_mut();
        // SAFETY: `owner` and `proposal` are 32 bytes, `edit` and everything it points to
        // outlive the call, and the signer handle is the Kotlin manager's live signer.
        let result = unsafe {
            platform_wallet_multiple_choice_proposal_edit(
                wallet_handle as Handle,
                owner.as_ptr(),
                proposal.as_ptr(),
                &edit,
                signer_handle as *mut SignerHandle,
                &mut out_json,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        // SAFETY: on success `out_json` is the wallet's string, freed by this call only.
        let Some(json) =
            (unsafe { take_wallet_json(env, out_json, "multiple choice proposal edit") })
        else {
            return ptr::null_mut();
        };
        env.new_string(json)
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Enter a choice of `ownerId` for the multiple choice proposal `proposalId` and broadcast
/// it, signed via `signerHandle`: the JNI bridge over
/// `platform_wallet_multiple_choice_proposal_enter_choice`. The wallet builds the choice,
/// whose put states the fund of the contest among the proposal's choices (paid by the owner
/// on top of the fees), picks the signing key, broadcasts and waits for Platform to confirm.
///
/// `description` may be null (none). `outChoiceId` must be a non-null `byte[32]`, checked
/// before anything is broadcast; it receives the choice's id, the id a masternode vote
/// towards it names.
///
/// Returns the confirmed document's canonical JSON, its Rust string freed here. Null after
/// throwing; a refusal by Platform carries its consensus code.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_MultipleChoiceProposalsNative_enterChoice(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    owner_id: JByteArray,
    proposal_id: JByteArray,
    title: JString,
    description: JString,
    signer_handle: jlong,
    out_choice_id: JByteArray,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        // Checked before the paid put, as the create checks its out-array.
        if !is_byte_array_of_len(env, &out_choice_id, 32) {
            throw_sdk_exception(env, 1, "outChoiceId must be a non-null byte[32]");
            return ptr::null_mut();
        }
        let Some(owner) = read_bytes32(env, &owner_id, "ownerId") else {
            return ptr::null_mut();
        };
        let Some(proposal) = read_bytes32(env, &proposal_id, "proposalId") else {
            return ptr::null_mut();
        };
        let Some(title) = read_cstring(env, &title, "title") else {
            return ptr::null_mut();
        };
        let Some(description) = read_optional_cstring(env, &description, "description") else {
            return ptr::null_mut();
        };

        let params = ChoiceParamsFFI {
            title: title.as_ptr(),
            description: c_ptr(&description),
        };
        let mut choice_id = [0u8; 32];
        let mut out_json: *mut c_char = ptr::null_mut();
        // SAFETY: `owner`, `proposal` and `choice_id` are 32 bytes, `params` and the strings
        // it points to outlive the call, and the signer handle is the Kotlin manager's live
        // signer.
        let result = unsafe {
            platform_wallet_multiple_choice_proposal_enter_choice(
                wallet_handle as Handle,
                owner.as_ptr(),
                proposal.as_ptr(),
                &params,
                signer_handle as *mut SignerHandle,
                choice_id.as_mut_ptr(),
                &mut out_json,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        // SAFETY: on success `out_json` is the wallet's string, freed by this call only.
        let Some(json) =
            (unsafe { take_wallet_json(env, out_json, "multiple choice proposal enter choice") })
        else {
            return ptr::null_mut();
        };
        if !write_created_id(env, &out_choice_id, choice_id, "the choice") {
            return ptr::null_mut();
        }
        env.new_string(json)
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

// ── Reads ────────────────────────────────────────────────────────────────

/// A multiple choice proposal by id as the proposal JSON object
/// (`dash_sdk_multiple_choice_proposal_fetch`); null when there is none, and after throwing.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_MultipleChoiceProposalsNative_fetch(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    proposal_id: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(id) = read_cstring(env, &proposal_id, "proposalId") else {
            return ptr::null_mut();
        };
        unsafe {
            let result =
                dash_sdk_multiple_choice_proposal_fetch(sdk as *const SDKHandle, id.as_ptr());
            string_or_null(env, result)
        }
    })
}

/// One page of an identity's multiple choice proposals in filing order
/// (`dash_sdk_multiple_choice_proposal_list_by_owner`), as a JSON array of proposals. `limit`
/// 0 is Platform's cap of 100; `startAfter` (base58) is null for the first page. Null after
/// throwing.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_MultipleChoiceProposalsNative_listByOwner(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    owner_id: JString,
    limit: jint,
    start_after: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(owner) = read_cstring(env, &owner_id, "ownerId") else {
            return ptr::null_mut();
        };
        let Some(limit) = to_unsigned(env, limit.into(), "limit") else {
            return ptr::null_mut();
        };
        let Some(start_after) = read_optional_cstring(env, &start_after, "startAfter") else {
            return ptr::null_mut();
        };
        unsafe {
            let result = dash_sdk_multiple_choice_proposal_list_by_owner(
                sdk as *const SDKHandle,
                owner.as_ptr(),
                limit,
                c_ptr(&start_after),
            );
            string_or_null(env, result)
        }
    })
}

/// One page of every multiple choice proposal, newest first
/// (`dash_sdk_multiple_choice_proposal_list_by_created_at`), as a JSON array of proposals.
/// Paging as for `listByOwner`. Null after throwing.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_MultipleChoiceProposalsNative_listByCreatedAt(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    limit: jint,
    start_after: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(limit) = to_unsigned(env, limit.into(), "limit") else {
            return ptr::null_mut();
        };
        let Some(start_after) = read_optional_cstring(env, &start_after, "startAfter") else {
            return ptr::null_mut();
        };
        unsafe {
            let result = dash_sdk_multiple_choice_proposal_list_by_created_at(
                sdk as *const SDKHandle,
                limit,
                c_ptr(&start_after),
            );
            string_or_null(env, result)
        }
    })
}

/// The vote among a proposal's choices as JSON (`dash_sdk_multiple_choice_proposal_get_vote`):
/// every choice with its tally, the other tallies, and how the vote ended once it did. Null
/// after throwing.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_MultipleChoiceProposalsNative_getVote(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    proposal_id: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(id) = read_cstring(env, &proposal_id, "proposalId") else {
            return ptr::null_mut();
        };
        unsafe {
            let result =
                dash_sdk_multiple_choice_proposal_get_vote(sdk as *const SDKHandle, id.as_ptr());
            string_or_null(env, result)
        }
    })
}

/// The choice a proposal's finished vote kept, as the choice JSON object
/// (`dash_sdk_multiple_choice_proposal_get_kept_choice`); null while the vote runs and when
/// it kept none, and after throwing.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_MultipleChoiceProposalsNative_getKeptChoice(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    proposal_id: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(id) = read_cstring(env, &proposal_id, "proposalId") else {
            return ptr::null_mut();
        };
        unsafe {
            let result = dash_sdk_multiple_choice_proposal_get_kept_choice(
                sdk as *const SDKHandle,
                id.as_ptr(),
            );
            string_or_null(env, result)
        }
    })
}

/// A masternode's answer in the vote among a proposal's choices, `{"choice", "choiceId"}`
/// (`dash_sdk_multiple_choice_proposal_get_masternode_vote`); null when it has not voted or
/// the vote ended, and after throwing. `voterProTxHash` is the 32-byte pro_tx_hash in wire
/// order, the same bytes `castVote` takes.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_MultipleChoiceProposalsNative_getMasternodeVote(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    voter_pro_tx_hash: JByteArray,
    proposal_id: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(pro_tx_hash) = read_bytes32(env, &voter_pro_tx_hash, "voterProTxHash") else {
            return ptr::null_mut();
        };
        let Some(id) = read_cstring(env, &proposal_id, "proposalId") else {
            return ptr::null_mut();
        };
        unsafe {
            let result = dash_sdk_multiple_choice_proposal_get_masternode_vote(
                sdk as *const SDKHandle,
                pro_tx_hash.as_ptr(),
                id.as_ptr(),
            );
            string_or_null(env, result)
        }
    })
}

/// What creating one multiple choice proposal adds to the processing fee of its batch, in
/// credits as a decimal string, not JSON
/// (`dash_sdk_multiple_choice_proposal_get_processing_fee`). Null after throwing.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_MultipleChoiceProposalsNative_getProcessingFee(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| unsafe {
        let result = dash_sdk_multiple_choice_proposal_get_processing_fee(sdk as *const SDKHandle);
        string_or_null(env, result)
    })
}

/// The fund a choice entered now for a proposal pays, in credits as a decimal string, not
/// JSON (`dash_sdk_multiple_choice_proposal_get_choice_fund_to_join`). Null after throwing.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_MultipleChoiceProposalsNative_getChoiceFundToJoin(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    proposal_id: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(id) = read_cstring(env, &proposal_id, "proposalId") else {
            return ptr::null_mut();
        };
        unsafe {
            let result = dash_sdk_multiple_choice_proposal_get_choice_fund_to_join(
                sdk as *const SDKHandle,
                id.as_ptr(),
            );
            string_or_null(env, result)
        }
    })
}

// ── Vote ─────────────────────────────────────────────────────────────────

/// Vote among a multiple choice proposal's choices for a masternode and wait for the
/// response (`dash_sdk_multiple_choice_proposal_cast_vote`).
///
/// * `voteChoice` - `0` for the choice `choiceId` names, `1` none of the choices, `2`
///   abstain; rs-sdk-ffi refuses any other value. A value outside `u8` is refused here
///   rather than wrapped onto a valid answer.
/// * `choiceId` - the document id (base58) of the choice voted for, null for the answers
///   that name none; rs-sdk-ffi refuses a missing or a superfluous one.
/// * `voterProTxHash` / `votingPrivateKey` - each a 32-byte `byte[]`; the pro_tx_hash in wire
///   order, as for `DecisionProposalsNative.castVote`. The key is read through a zeroizing
///   buffer.
/// * `networkOrd` - `Network.ffiValue`, used for the signer's WIF encoding only.
///
/// Returns nothing on success; throws on error. A vote from a wallet that is not a
/// masternode is refused by Platform (no voter identity), the expected outcome.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_MultipleChoiceProposalsNative_castVote(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    proposal_id: JString,
    vote_choice: jint,
    choice_id: JString,
    voter_pro_tx_hash: JByteArray,
    voting_private_key: JByteArray,
    network_ord: jint,
) {
    guard(&mut env, (), |env| {
        let Some(id) = read_cstring(env, &proposal_id, "proposalId") else {
            return;
        };
        let Some(choice) = to_unsigned(env, vote_choice.into(), "voteChoice") else {
            return;
        };
        let Some(choice_id) = read_optional_cstring(env, &choice_id, "choiceId") else {
            return;
        };
        let Some(pro_tx_hash) = read_bytes32(env, &voter_pro_tx_hash, "voterProTxHash") else {
            return;
        };
        let Some(voting_key) = read_key32_zeroizing(env, &voting_private_key, "votingPrivateKey")
        else {
            return;
        };
        let result = unsafe {
            dash_sdk_multiple_choice_proposal_cast_vote(
                sdk as *const SDKHandle,
                id.as_ptr(),
                choice,
                c_ptr(&choice_id),
                pro_tx_hash.as_ptr(),
                voting_key.as_ptr(),
                net_from_ord(network_ord),
            )
        };
        // Success carries no payload; take_error throws and frees on error.
        unsafe { take_error(env, &result) };
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision_proposals::tests::kotlin_signature_in;
    use std::ffi::CStr;

    const MULTIPLE_CHOICE_PROPOSALS_NATIVE_KT: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../kotlin-sdk/sdk/src/main/kotlin/org/dashfoundation/dashsdk/ffi/MultipleChoiceProposalsNative.kt"
    ));

    /// The parameter types and the return type of `external fun {name}(...)` in
    /// `MultipleChoiceProposalsNative.kt`.
    fn kotlin_signature(name: &str) -> (Vec<String>, String) {
        kotlin_signature_in(MULTIPLE_CHOICE_PROPOSALS_NATIVE_KT, name)
    }

    fn types<const N: usize>(types: [&str; N]) -> Vec<String> {
        types.map(String::from).to_vec()
    }

    /// JNI binds an export by its name alone: a Kotlin declaration whose parameters drifted
    /// from the export's would hand it the wrong JVM values at run time, which only an
    /// instrumented test would otherwise catch. The wallet writes take the most arguments,
    /// and the last of the two that create a document is the id out-array.
    #[test]
    fn should_declare_the_multiple_choice_wallet_writes_as_the_exports_take_them() {
        assert_eq!(
            kotlin_signature("create"),
            (
                types([
                    "Long",       // wallet_handle
                    "ByteArray",  // owner_id
                    "String",     // title
                    "ByteArray?", // description_document_id
                    "ByteArray?", // description_document_contract_id
                    "String?",    // description_document_type
                    "String?",    // url
                    "Boolean",    // open
                    "Long",       // signer_handle
                    "ByteArray",  // out_proposal_id
                ]),
                "String".to_string()
            )
        );
        assert_eq!(
            kotlin_signature("edit"),
            (
                types([
                    "Long",       // wallet_handle
                    "ByteArray",  // owner_id
                    "ByteArray",  // proposal_id
                    "String?",    // title
                    "ByteArray?", // description_document_id
                    "ByteArray?", // description_document_contract_id
                    "String?",    // description_document_type
                    "Boolean",    // remove_description_document
                    "String?",    // url
                    "Boolean",    // remove_url
                    "Long",       // signer_handle
                ]),
                "String".to_string()
            )
        );
        assert_eq!(
            kotlin_signature("enterChoice"),
            (
                types([
                    "Long",      // wallet_handle
                    "ByteArray", // owner_id
                    "ByteArray", // proposal_id
                    "String",    // title
                    "String?",   // description
                    "Long",      // signer_handle
                    "ByteArray", // out_choice_id
                ]),
                "String".to_string()
            )
        );
    }

    /// The vote takes the choice id between the answer and the masternode's credentials.
    #[test]
    fn should_declare_the_multiple_choice_vote_as_the_export_takes_it() {
        assert_eq!(
            kotlin_signature("castVote"),
            (
                types([
                    "Long",      // sdk
                    "String",    // proposal_id
                    "Int",       // vote_choice
                    "String?",   // choice_id
                    "ByteArray", // voter_pro_tx_hash
                    "ByteArray", // voting_private_key
                    "Int",       // network_ord
                ]),
                "Unit".to_string()
            )
        );
    }

    /// No description document is a null struct pointer, never a struct of null fields: the
    /// platform wallet reads the first as "none" and refuses the second.
    #[test]
    fn should_pass_no_multiple_choice_description_document_as_a_null_pointer() {
        let absent = DescriptionDocumentArgs {
            id: None,
            contract_id: None,
            document_type: None,
        };
        let described = absent.as_ffi();
        assert!(described.is_none());
        assert!(description_document_ptr(&described).is_null());
    }

    #[test]
    fn should_point_the_multiple_choice_description_document_at_each_part_given() {
        let complete = DescriptionDocumentArgs {
            id: Some([5; 32]),
            contract_id: Some([6; 32]),
            document_type: Some(CString::new("post").expect("no NUL")),
        };
        let described = complete.as_ffi();
        let pointer = description_document_ptr(&described);
        assert!(!pointer.is_null());
        // SAFETY: `pointer` points into `described`, whose fields point into `complete`;
        // both are alive here.
        unsafe {
            let read = &*pointer;
            assert_eq!(std::slice::from_raw_parts(read.id, 32), [5u8; 32]);
            assert_eq!(std::slice::from_raw_parts(read.contract_id, 32), [6u8; 32]);
            assert_eq!(CStr::from_ptr(read.document_type).to_bytes(), b"post");
        }

        // A part left out reaches the platform wallet as a null field, which it refuses.
        let partial = DescriptionDocumentArgs {
            id: Some([5; 32]),
            contract_id: None,
            document_type: None,
        };
        let described = partial.as_ffi().expect("a struct for a partial document");
        assert!(!described.id.is_null());
        assert!(described.contract_id.is_null());
        assert!(described.document_type.is_null());
    }
}
