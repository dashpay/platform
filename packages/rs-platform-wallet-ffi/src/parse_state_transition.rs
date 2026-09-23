//! FFI projection of [`summarize_state_transition`] for raw DPP state
//! transitions handed to the wallet by a dApp (DashConnect `dash-st:` links /
//! QRs, DashPay Connect `sign`).
//!
//! Decoding and deciding what the user has to see live in `platform-wallet`
//! ([`StateTransitionSummary`]); this module copies the summary into owned C
//! structs:
//!
//! - `Batch`: one row per batched transition (contract id, document type,
//!   action, and for token transitions the amount and recipient),
//! - `IdentityUpdate`: every key added (purpose, level, bounds, limits) and
//!   every key disabled,
//! - `IdentityCreditTransfer`: recipient and amount,
//! - `DataContractCreate` / `DataContractUpdate`: contract id and document
//!   type names,
//! - everything else: `OTHER`, with the whole decoded transition in `details`.
//!
//! **Completeness contract.** A batched row whose typed fields do not cover
//! every material field carries `complete == false` and renders the rest in
//! its `details`; a wallet must not approve such a row from the typed summary
//! alone. The `OTHER` kind likewise carries the whole decoded transition in
//! the common `details`.
//!
//! This module does not sign and does not broadcast.

use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;
use std::slice;

use dpp::prelude::Identifier;
use platform_wallet::error::PlatformWalletError;
use platform_wallet::wallet::identity::network::{
    summarize_state_transition, BatchedTransitionSummary, BatchedTransitionTarget,
    DataContractSummary, StateTransitionSummary, StateTransitionSummaryKind,
};

use crate::check_ptr;
use crate::error::*;
use crate::identity_update::{
    platform_wallet_parse_identity_update_transition_free, project_parsed_identity_update,
    ParsedIdentityUpdateFFI,
};
use crate::unwrap_result_or_return;

/// `ParsedStateTransitionFFI::kind`: nothing was parsed (default state).
pub const PARSED_STATE_TRANSITION_KIND_NONE: u8 = 0;
/// `ParsedStateTransitionFFI::kind`: `identity_update` is populated.
pub const PARSED_STATE_TRANSITION_KIND_IDENTITY_UPDATE: u8 = 1;
/// `ParsedStateTransitionFFI::kind`: `batch` is populated.
pub const PARSED_STATE_TRANSITION_KIND_BATCH: u8 = 2;
/// `ParsedStateTransitionFFI::kind`: `credit_transfer` is populated.
pub const PARSED_STATE_TRANSITION_KIND_CREDIT_TRANSFER: u8 = 3;
/// `ParsedStateTransitionFFI::kind`: `data_contract` is populated and the
/// transition creates the contract.
pub const PARSED_STATE_TRANSITION_KIND_DATA_CONTRACT_CREATE: u8 = 4;
/// `ParsedStateTransitionFFI::kind`: `data_contract` is populated and the
/// transition updates the contract.
pub const PARSED_STATE_TRANSITION_KIND_DATA_CONTRACT_UPDATE: u8 = 5;
/// `ParsedStateTransitionFFI::kind`: a kind without a describer; the common
/// fields are populated, with the whole transition in `details`.
pub const PARSED_STATE_TRANSITION_KIND_OTHER: u8 = 255;

/// `ParsedBatchedTransitionFFI::family`: a document transition.
pub const PARSED_BATCHED_TRANSITION_FAMILY_DOCUMENT: u8 = 0;
/// `ParsedBatchedTransitionFFI::family`: a token transition.
pub const PARSED_BATCHED_TRANSITION_FAMILY_TOKEN: u8 = 1;

/// One transition inside a parsed `BatchTransition`; the fields mirror
/// `BatchedTransitionSummary`. `action`, `document_type` (null for token rows)
/// and `details` are owned, NUL-terminated, and released by
/// [`platform_wallet_parse_state_transition_free`]. A row with
/// `complete == false` must not be approved without rendering `details`.
#[repr(C)]
pub struct ParsedBatchedTransitionFFI {
    /// `PARSED_BATCHED_TRANSITION_FAMILY_DOCUMENT` or `_TOKEN`.
    pub family: u8,
    /// Data contract the transition acts within.
    pub data_contract_id: [u8; 32],
    /// Action name, see the struct doc. Owned, NUL-terminated.
    pub action: *mut c_char,
    /// Document type name for document transitions; null for token ones.
    pub document_type: *mut c_char,
    /// Document id for document transitions; zero for token ones.
    pub document_id: [u8; 32],
    /// Position of the token within its contract (token transitions).
    pub token_contract_position: u16,
    /// Token id (token transitions); zero for document ones.
    pub token_id: [u8; 32],
    pub has_amount: bool,
    pub amount: u64,
    pub has_recipient: bool,
    pub recipient_id: [u8; 32],
    /// Whether a direct purchase's token count is set.
    pub has_token_count: bool,
    /// Tokens bought by a `DirectPurchase`.
    pub token_count: u64,
    /// Whether the typed fields describe every material field. See the
    /// struct doc.
    pub complete: bool,
    /// Rendering of the material fields the typed fields do not cover, when
    /// `complete` is false; null otherwise. Owned, NUL-terminated.
    pub details: *mut c_char,
}

impl Default for ParsedBatchedTransitionFFI {
    fn default() -> Self {
        Self {
            family: PARSED_BATCHED_TRANSITION_FAMILY_DOCUMENT,
            data_contract_id: [0u8; 32],
            action: ptr::null_mut(),
            document_type: ptr::null_mut(),
            document_id: [0u8; 32],
            token_contract_position: 0,
            token_id: [0u8; 32],
            has_amount: false,
            amount: 0,
            has_recipient: false,
            recipient_id: [0u8; 32],
            has_token_count: false,
            token_count: 0,
            complete: true,
            details: ptr::null_mut(),
        }
    }
}

/// Owned C representation of a parsed `BatchTransition`: its owner and one
/// [`ParsedBatchedTransitionFFI`] per batched transition, in order.
#[repr(C)]
pub struct ParsedBatchFFI {
    pub owner_id: [u8; 32],
    pub transitions: *mut ParsedBatchedTransitionFFI,
    pub transitions_count: usize,
}

impl Default for ParsedBatchFFI {
    fn default() -> Self {
        Self {
            owner_id: [0u8; 32],
            transitions: ptr::null_mut(),
            transitions_count: 0,
        }
    }
}

/// Owned C representation of a parsed `IdentityCreditTransferTransition`.
/// Plain old data.
#[repr(C)]
#[derive(Default)]
pub struct ParsedCreditTransferFFI {
    pub identity_id: [u8; 32],
    pub recipient_id: [u8; 32],
    pub amount: u64,
}

/// Owned C representation of the inspectable parts of a parsed
/// `DataContractCreateTransition` or `DataContractUpdateTransition`.
/// `document_type_names` is a Rust-owned array of NUL-terminated strings,
/// sorted as the contract stores them.
#[repr(C)]
pub struct ParsedDataContractFFI {
    pub contract_id: [u8; 32],
    pub owner_id: [u8; 32],
    pub document_type_names: *mut *mut c_char,
    pub document_type_names_count: usize,
}

impl Default for ParsedDataContractFFI {
    fn default() -> Self {
        Self {
            contract_id: [0u8; 32],
            owner_id: [0u8; 32],
            document_type_names: ptr::null_mut(),
            document_type_names_count: 0,
        }
    }
}

/// Owned C representation of one parsed state transition, discriminated by
/// `kind`. Exactly one of the kind-specific payload fields is populated
/// (none for `PARSED_STATE_TRANSITION_KIND_OTHER`); the rest stay in their
/// zeroed default state. The common fields are populated for every kind.
/// Must be released via [`platform_wallet_parse_state_transition_free`]
/// regardless of `kind`.
#[repr(C)]
pub struct ParsedStateTransitionFFI {
    /// One of the `PARSED_STATE_TRANSITION_KIND_*` constants.
    pub kind: u8,
    /// `StateTransition::name()` of the decoded transition, e.g.
    /// `IdentityUpdate`, `DocumentsBatch([Create, TokenTransfer])`,
    /// `MasternodeVote`. Owned, NUL-terminated.
    pub kind_name: *mut c_char,
    /// Whether the transition names an owner (every kind except the
    /// asset-lock-funded and shielded ones does).
    pub has_owner_id: bool,
    /// The identity the transition acts for, when `has_owner_id`.
    pub owner_id: [u8; 32],
    /// Whether the transition already carries a non-empty signature. A
    /// `sign` request must arrive unsigned.
    pub is_signed: bool,
    /// Percentage added to the processing fee; part of the signed bytes.
    pub user_fee_increase: u16,
    /// Whether the typed payload shows every material field
    /// (`StateTransitionSummary::is_complete`); when false, render `details`
    /// (or the rows' `details`) before approval.
    pub complete: bool,
    /// The decoded transition re-serialized, tagged: what a caller signs
    /// after approval.
    pub serialized: *mut u8,
    pub serialized_len: usize,
    /// `OTHER`: `Debug` dump of the whole transition; data contract create /
    /// update: of the whole contract. Null otherwise. Owned, NUL-terminated.
    pub details: *mut c_char,
    /// Populated when `kind == PARSED_STATE_TRANSITION_KIND_IDENTITY_UPDATE`.
    pub identity_update: ParsedIdentityUpdateFFI,
    /// Populated when `kind == PARSED_STATE_TRANSITION_KIND_BATCH`.
    pub batch: ParsedBatchFFI,
    /// Populated when `kind == PARSED_STATE_TRANSITION_KIND_CREDIT_TRANSFER`.
    pub credit_transfer: ParsedCreditTransferFFI,
    /// Populated when `kind` is `_DATA_CONTRACT_CREATE` or `_UPDATE`.
    pub data_contract: ParsedDataContractFFI,
}

impl Default for ParsedStateTransitionFFI {
    fn default() -> Self {
        Self {
            kind: PARSED_STATE_TRANSITION_KIND_NONE,
            kind_name: ptr::null_mut(),
            has_owner_id: false,
            owner_id: [0u8; 32],
            is_signed: false,
            user_fee_increase: 0,
            complete: false,
            serialized: ptr::null_mut(),
            serialized_len: 0,
            details: ptr::null_mut(),
            identity_update: ParsedIdentityUpdateFFI::default(),
            batch: ParsedBatchFFI::default(),
            credit_transfer: ParsedCreditTransferFFI::default(),
            data_contract: ParsedDataContractFFI::default(),
        }
    }
}

/// Bytes the summary could not decode.
pub(crate) fn deserialization_error(error: PlatformWalletError) -> PlatformWalletFFIResult {
    PlatformWalletFFIResult::err(
        PlatformWalletFFIResultCode::ErrorDeserialization,
        error.to_string(),
    )
}

/// A string that cannot cross the FFI as a C string is an error rather than
/// a fallback: showing the user a different name than the transition
/// declares would let them approve something other than what they saw.
///
/// Returns the owning [`CString`]; callers `into_raw()` it only once every
/// string of the row they are building has converted, so a failure partway
/// through drops what was built instead of leaking a raw pointer.
fn owned_c_string(value: &str, what: &str) -> Result<CString, PlatformWalletFFIResult> {
    CString::new(value).map_err(|error| {
        PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!("{what} cannot be represented as a C string: {error}"),
        )
    })
}

fn optional_c_string(
    value: Option<&str>,
    what: &str,
) -> Result<Option<CString>, PlatformWalletFFIResult> {
    value.map(|value| owned_c_string(value, what)).transpose()
}

unsafe fn free_c_string(ptr: &mut *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(*ptr));
        *ptr = ptr::null_mut();
    }
}

fn project_batched_transition(
    row: &BatchedTransitionSummary,
) -> Result<ParsedBatchedTransitionFFI, PlatformWalletFFIResult> {
    let action = owned_c_string(&row.action, "Batched transition action")?;
    let details = optional_c_string(row.details.as_deref(), "Batched transition details")?;
    let mut out = ParsedBatchedTransitionFFI {
        data_contract_id: row.data_contract_id.to_buffer(),
        has_amount: row.amount.is_some(),
        amount: row.amount.unwrap_or_default(),
        has_recipient: row.recipient_id.is_some(),
        recipient_id: row
            .recipient_id
            .map(|id| id.to_buffer())
            .unwrap_or_default(),
        has_token_count: row.token_count.is_some(),
        token_count: row.token_count.unwrap_or_default(),
        complete: row.is_complete(),
        ..ParsedBatchedTransitionFFI::default()
    };
    match &row.target {
        BatchedTransitionTarget::Document {
            document_type,
            document_id,
        } => {
            let document_type =
                owned_c_string(document_type, "Batched document transition document type")?;
            out.family = PARSED_BATCHED_TRANSITION_FAMILY_DOCUMENT;
            out.document_type = document_type.into_raw();
            out.document_id = document_id.to_buffer();
        }
        BatchedTransitionTarget::Token {
            token_id,
            token_contract_position,
        } => {
            out.family = PARSED_BATCHED_TRANSITION_FAMILY_TOKEN;
            out.token_id = token_id.to_buffer();
            out.token_contract_position = *token_contract_position;
        }
    }
    out.action = action.into_raw();
    out.details = details.map(CString::into_raw).unwrap_or(ptr::null_mut());
    Ok(out)
}

unsafe fn free_batched_transitions(rows: &mut [ParsedBatchedTransitionFFI]) {
    for row in rows.iter_mut() {
        free_c_string(&mut row.action);
        free_c_string(&mut row.document_type);
        free_c_string(&mut row.details);
    }
}

fn project_parsed_batch(
    owner_id: &Identifier,
    transitions: &[BatchedTransitionSummary],
) -> Result<ParsedBatchFFI, PlatformWalletFFIResult> {
    let mut rows: Vec<ParsedBatchedTransitionFFI> = Vec::with_capacity(transitions.len());
    for transition in transitions {
        match project_batched_transition(transition) {
            Ok(row) => rows.push(row),
            Err(error) => {
                // The caller never receives this struct, so nothing else will
                // free the rows projected so far.
                unsafe { free_batched_transitions(&mut rows) };
                return Err(error);
            }
        }
    }

    let transitions_count = rows.len();
    let transitions = if transitions_count == 0 {
        ptr::null_mut()
    } else {
        Box::into_raw(rows.into_boxed_slice()) as *mut ParsedBatchedTransitionFFI
    };

    Ok(ParsedBatchFFI {
        owner_id: owner_id.to_buffer(),
        transitions,
        transitions_count,
    })
}

unsafe fn free_parsed_batch(batch: &mut ParsedBatchFFI) {
    if !batch.transitions.is_null() && batch.transitions_count > 0 {
        let rows = slice::from_raw_parts_mut(batch.transitions, batch.transitions_count);
        free_batched_transitions(rows);
        drop(Box::from_raw(rows as *mut [ParsedBatchedTransitionFFI]));
    }
    *batch = ParsedBatchFFI::default();
}

fn project_parsed_data_contract(
    contract: &DataContractSummary,
) -> Result<ParsedDataContractFFI, PlatformWalletFFIResult> {
    // `Vec<CString>` owns every name until all of them have converted, so a
    // failure partway through frees what was built with no manual cleanup.
    let names = contract
        .document_type_names
        .iter()
        .map(|name| owned_c_string(name, "Data contract document type name"))
        .collect::<Result<Vec<CString>, _>>()?;

    let document_type_names_count = names.len();
    let document_type_names = if document_type_names_count == 0 {
        ptr::null_mut()
    } else {
        let raw: Vec<*mut c_char> = names.into_iter().map(CString::into_raw).collect();
        Box::into_raw(raw.into_boxed_slice()) as *mut *mut c_char
    };

    Ok(ParsedDataContractFFI {
        contract_id: contract.contract_id.to_buffer(),
        owner_id: contract.owner_id.to_buffer(),
        document_type_names,
        document_type_names_count,
    })
}

unsafe fn free_parsed_data_contract(contract: &mut ParsedDataContractFFI) {
    if !contract.document_type_names.is_null() && contract.document_type_names_count > 0 {
        let names = slice::from_raw_parts_mut(
            contract.document_type_names,
            contract.document_type_names_count,
        );
        for name in names.iter_mut() {
            free_c_string(name);
        }
        drop(Box::from_raw(names as *mut [*mut c_char]));
    }
    *contract = ParsedDataContractFFI::default();
}

/// Copies a summary into the C struct. Everything allocated is owned by the
/// result on success; on failure nothing is left allocated.
fn project_parsed_state_transition(
    summary: StateTransitionSummary,
) -> Result<ParsedStateTransitionFFI, PlatformWalletFFIResult> {
    // Kept as owned `CString`s until nothing below can fail, so an error
    // return drops them.
    let kind_name = owned_c_string(&summary.kind_name, "State transition kind name")?;
    let details = match &summary.kind {
        StateTransitionSummaryKind::Other { details }
        | StateTransitionSummaryKind::DataContractCreate(DataContractSummary { details, .. })
        | StateTransitionSummaryKind::DataContractUpdate(DataContractSummary { details, .. }) => {
            Some(owned_c_string(details, "State transition details")?)
        }
        _ => None,
    };

    // Each arm stores at most one payload, and only on success, so an error
    // leaves nothing to free.
    let mut out = ParsedStateTransitionFFI::default();
    out.kind = match &summary.kind {
        StateTransitionSummaryKind::IdentityUpdate {
            identity_id,
            add_public_keys,
            disable_public_key_ids,
        } => {
            out.identity_update = project_parsed_identity_update(
                identity_id,
                add_public_keys,
                disable_public_key_ids,
            )?;
            PARSED_STATE_TRANSITION_KIND_IDENTITY_UPDATE
        }
        StateTransitionSummaryKind::Batch {
            owner_id,
            transitions,
        } => {
            out.batch = project_parsed_batch(owner_id, transitions)?;
            PARSED_STATE_TRANSITION_KIND_BATCH
        }
        StateTransitionSummaryKind::CreditTransfer {
            identity_id,
            recipient_id,
            amount,
        } => {
            out.credit_transfer = ParsedCreditTransferFFI {
                identity_id: identity_id.to_buffer(),
                recipient_id: recipient_id.to_buffer(),
                amount: *amount,
            };
            PARSED_STATE_TRANSITION_KIND_CREDIT_TRANSFER
        }
        StateTransitionSummaryKind::DataContractCreate(contract) => {
            out.data_contract = project_parsed_data_contract(contract)?;
            PARSED_STATE_TRANSITION_KIND_DATA_CONTRACT_CREATE
        }
        StateTransitionSummaryKind::DataContractUpdate(contract) => {
            out.data_contract = project_parsed_data_contract(contract)?;
            PARSED_STATE_TRANSITION_KIND_DATA_CONTRACT_UPDATE
        }
        StateTransitionSummaryKind::Other { .. } => PARSED_STATE_TRANSITION_KIND_OTHER,
    };

    out.complete = summary.is_complete();
    out.kind_name = kind_name.into_raw();
    out.has_owner_id = summary.owner_id.is_some();
    out.owner_id = summary
        .owner_id
        .map(|id| id.to_buffer())
        .unwrap_or_default();
    out.is_signed = summary.is_signed;
    out.user_fee_increase = summary.user_fee_increase;
    out.serialized_len = summary.serialized.len();
    out.serialized = Box::into_raw(summary.serialized.into_boxed_slice()) as *mut u8;
    out.details = details.map(CString::into_raw).unwrap_or(ptr::null_mut());
    Ok(out)
}

/// Decodes a raw DPP state transition (a DashConnect `dash-st:` link / QR or a
/// DashPay Connect `sign` request) and copies [`summarize_state_transition`]'s
/// summary into `out`. Accepts tagged bytes and an identity update or batch
/// serialized without the `StateTransition` variant tag; trailing bytes are
/// refused. Does NOT sign and does NOT broadcast.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_parse_state_transition(
    transition_bytes: *const u8,
    transition_len: usize,
    out: *mut ParsedStateTransitionFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out);
    *out = ParsedStateTransitionFFI::default();
    check_ptr!(transition_bytes);

    let bytes = slice::from_raw_parts(transition_bytes, transition_len);
    let summary =
        unwrap_result_or_return!(summarize_state_transition(bytes).map_err(deserialization_error));

    *out = unwrap_result_or_return!(project_parsed_state_transition(summary));

    PlatformWalletFFIResult::ok()
}

/// Frees a parsed transition previously returned by
/// [`platform_wallet_parse_state_transition`]. Safe to call for any `kind`,
/// including the zeroed default, and idempotent.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_parse_state_transition_free(
    out: *mut ParsedStateTransitionFFI,
) {
    if out.is_null() {
        return;
    }

    let parsed = &mut *out;
    platform_wallet_parse_identity_update_transition_free(&mut parsed.identity_update);
    free_parsed_batch(&mut parsed.batch);
    free_parsed_data_contract(&mut parsed.data_contract);
    free_c_string(&mut parsed.kind_name);
    free_c_string(&mut parsed.details);
    if !parsed.serialized.is_null() && parsed.serialized_len > 0 {
        drop(Box::from_raw(ptr::slice_from_raw_parts_mut(
            parsed.serialized,
            parsed.serialized_len,
        )));
    }
    *parsed = ParsedStateTransitionFFI::default();
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::state_transition::batch_transition::batched_transition::{
        DocumentTransition, TokenTransition,
    };
    use dpp::state_transition::batch_transition::BatchTransition;
    use dpp::state_transition::StateTransition;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
    use dpp::identity::identity_public_key::contract_bounds::ContractBounds;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::{platform_value, BinaryData};
    use dpp::prelude::Identifier;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
    use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
    use dpp::state_transition::batch_transition::document_create_transition::v0::DocumentCreateTransitionV0;
    use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::v0::DocumentTransferTransitionV0;
    use dpp::state_transition::batch_transition::batched_transition::DocumentTransferTransition;
    use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
    use dpp::state_transition::batch_transition::token_direct_purchase_transition::v0::TokenDirectPurchaseTransitionV0;
    use dpp::state_transition::batch_transition::token_transfer_transition::v0::TokenTransferTransitionV0;
    use dpp::state_transition::batch_transition::{
        BatchTransitionV1, DocumentCreateTransition, TokenDirectPurchaseTransition,
        TokenTransferTransition,
    };
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
    use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::TryFromPlatformVersioned;
    use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
    use dpp::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
    use dpp::state_transition::public_key_in_creation::v1::IdentityPublicKeyInCreationV1;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;
    use std::ffi::CStr;

    const OWNER: [u8; 32] = [0x21; 32];
    const CONTRACT: [u8; 32] = [0x42; 32];
    const TOKEN: [u8; 32] = [0x77; 32];
    const RECIPIENT: [u8; 32] = [0x22; 32];

    fn token_base() -> TokenBaseTransition {
        TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: 4,
            token_contract_position: 3,
            data_contract_id: Identifier::from(CONTRACT),
            token_id: Identifier::from(TOKEN),
            using_group_info: None,
        })
    }

    fn direct_purchase() -> BatchedTransition {
        BatchedTransition::Token(TokenTransition::DirectPurchase(
            TokenDirectPurchaseTransition::V0(TokenDirectPurchaseTransitionV0 {
                base: token_base(),
                token_count: 100,
                total_agreed_price: 100_000_000,
            }),
        ))
    }

    fn token_transfer() -> BatchedTransition {
        BatchedTransition::Token(TokenTransition::Transfer(TokenTransferTransition::V0(
            TokenTransferTransitionV0 {
                base: token_base(),
                amount: 250,
                recipient_id: Identifier::from(RECIPIENT),
                public_note: None,
                shared_encrypted_note: None,
                private_encrypted_note: None,
            },
        )))
    }

    fn document_base(document_type_name: &str) -> DocumentBaseTransition {
        DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Identifier::from([0x0D; 32]),
            identity_contract_nonce: 1,
            document_type_name: document_type_name.to_string(),
            data_contract_id: Identifier::from(CONTRACT),
        })
    }

    fn document_create(document_type_name: &str) -> BatchedTransition {
        BatchedTransition::Document(DocumentTransition::Create(DocumentCreateTransition::V0(
            DocumentCreateTransitionV0 {
                base: document_base(document_type_name),
                entropy: [0xEE; 32],
                data: BTreeMap::from([("message".to_string(), platform_value!("hi"))]),
                prefunded_voting_balance: None,
            },
        )))
    }

    fn document_transfer() -> BatchedTransition {
        BatchedTransition::Document(DocumentTransition::Transfer(
            DocumentTransferTransition::V0(DocumentTransferTransitionV0 {
                base: document_base("profile"),
                revision: 2,
                recipient_owner_id: Identifier::from(RECIPIENT),
            }),
        ))
    }

    fn batch_transition_bytes(transitions: Vec<BatchedTransition>, signature: Vec<u8>) -> Vec<u8> {
        StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
            owner_id: Identifier::from(OWNER),
            transitions,
            user_fee_increase: 1,
            signature_public_key_id: 2,
            signature: BinaryData::new(signature),
        }))
        .serialize_to_bytes()
        .expect("fixture batch serializes")
    }

    fn identity_update_transition_bytes() -> Vec<u8> {
        StateTransition::IdentityUpdate(
            IdentityUpdateTransitionV0 {
                signature: BinaryData::new(vec![0x99; 65]),
                signature_public_key_id: 3,
                identity_id: Identifier::from([0x11; 32]),
                revision: 7,
                nonce: 9,
                add_public_keys: vec![
                    IdentityPublicKeyInCreationV0 {
                        id: 17,
                        key_type: KeyType::ECDSA_SECP256K1,
                        purpose: Purpose::AUTHENTICATION,
                        security_level: SecurityLevel::HIGH,
                        read_only: false,
                        data: BinaryData::new(vec![0x02; 33]),
                        signature: BinaryData::new(vec![0xaa; 65]),
                        contract_bounds: None,
                    }
                    .into(),
                    // A DashPay Connect session key: HIGH auth key bound to a
                    // contract group with a budget and an expiry.
                    IdentityPublicKeyInCreationV1 {
                        id: 18,
                        key_type: KeyType::ECDSA_SECP256K1,
                        purpose: Purpose::AUTHENTICATION,
                        security_level: SecurityLevel::HIGH,
                        read_only: false,
                        data: BinaryData::new(vec![0x03; 33]),
                        signature: BinaryData::new(vec![0xbb; 65]),
                        contract_bounds: Some(ContractBounds::ContractGroup {
                            id: Identifier::from([0x66; 32]),
                        }),
                        total_budget: Some(10_000_000_000),
                        expires_at: Some(1_800_000_000_000),
                    }
                    .into(),
                ],
                disable_public_keys: vec![4, 8],
                user_fee_increase: 2,
            }
            .into(),
        )
        .serialize_to_bytes()
        .expect("fixture identity update serializes")
    }

    fn credit_transfer_bytes() -> Vec<u8> {
        credit_transfer_bytes_with_fee_increase(0)
    }

    fn credit_transfer_bytes_with_fee_increase(user_fee_increase: u16) -> Vec<u8> {
        StateTransition::IdentityCreditTransfer(
            IdentityCreditTransferTransitionV0 {
                identity_id: Identifier::from([0x11; 32]),
                recipient_id: Identifier::from(RECIPIENT),
                amount: 1_000,
                nonce: 1,
                user_fee_increase,
                signature_public_key_id: 0,
                signature: BinaryData::new(vec![]),
            }
            .into(),
        )
        .serialize_to_bytes()
        .expect("fixture credit transfer serializes")
    }

    /// The rs-dpp fixture contract (owner `OWNER`), in the wire form a
    /// create / update transition carries. Its id and document type names
    /// are read back from the fixture rather than hard-coded so the test
    /// follows the fixture if it changes.
    fn contract_format() -> (DataContractInSerializationFormat, Identifier, Vec<String>) {
        let created = get_data_contract_fixture(
            Some(Identifier::from(OWNER)),
            1,
            PlatformVersion::latest().protocol_version,
        );
        let contract = created.data_contract();
        let id = contract.id();
        let names: Vec<String> = contract.document_types().keys().cloned().collect();
        let format = DataContractInSerializationFormat::try_from_platform_versioned(
            contract,
            PlatformVersion::latest(),
        )
        .expect("fixture contract converts to its serialization format");
        (format, id, names)
    }

    fn data_contract_create_bytes() -> (Vec<u8>, Identifier, Vec<String>) {
        let (format, id, names) = contract_format();
        let bytes = StateTransition::DataContractCreate(
            DataContractCreateTransitionV0 {
                data_contract: format,
                identity_nonce: 1,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: BinaryData::new(vec![]),
            }
            .into(),
        )
        .serialize_to_bytes()
        .expect("fixture contract create serializes");
        (bytes, id, names)
    }

    fn data_contract_update_bytes() -> (Vec<u8>, Identifier, Vec<String>) {
        let (format, id, names) = contract_format();
        let bytes = StateTransition::DataContractUpdate(
            DataContractUpdateTransitionV0 {
                identity_contract_nonce: 2,
                data_contract: format,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: BinaryData::new(vec![]),
            }
            .into(),
        )
        .serialize_to_bytes()
        .expect("fixture contract update serializes");
        (bytes, id, names)
    }

    fn parse(bytes: &[u8]) -> (PlatformWalletFFIResult, ParsedStateTransitionFFI) {
        let mut out = ParsedStateTransitionFFI::default();
        let result = unsafe {
            platform_wallet_parse_state_transition(bytes.as_ptr(), bytes.len(), &mut out)
        };
        (result, out)
    }

    unsafe fn c_str(ptr: *const c_char) -> String {
        CStr::from_ptr(ptr).to_str().expect("utf8").to_string()
    }

    unsafe fn batched_rows(out: &ParsedStateTransitionFFI) -> &[ParsedBatchedTransitionFFI] {
        slice::from_raw_parts(out.batch.transitions, out.batch.transitions_count)
    }

    unsafe fn serialized(out: &ParsedStateTransitionFFI) -> &[u8] {
        slice::from_raw_parts(out.serialized, out.serialized_len)
    }

    #[test]
    fn parses_a_mixed_batch_with_one_row_per_transition() {
        let bytes = batch_transition_bytes(
            vec![
                document_create("post"),
                document_transfer(),
                token_transfer(),
                direct_purchase(),
            ],
            vec![0x88; 65],
        );
        let (result, mut out) = parse(&bytes);

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.kind, PARSED_STATE_TRANSITION_KIND_BATCH);
        assert_eq!(
            unsafe { c_str(out.kind_name) },
            "DocumentsBatch([Create, Transfer, TokenTransfer, TokenDirectPurchase])"
        );
        assert!(out.has_owner_id);
        assert_eq!(out.owner_id, OWNER);
        assert!(out.is_signed);
        assert_eq!(unsafe { serialized(&out) }, bytes.as_slice());
        assert_eq!(out.batch.owner_id, OWNER);
        assert_eq!(out.batch.transitions_count, 4);

        let rows = unsafe { batched_rows(&out) };

        let create = &rows[0];
        assert_eq!(create.family, PARSED_BATCHED_TRANSITION_FAMILY_DOCUMENT);
        assert_eq!(unsafe { c_str(create.action) }, "Create");
        assert_eq!(unsafe { c_str(create.document_type) }, "post");
        assert_eq!(create.data_contract_id, CONTRACT);
        assert_eq!(create.document_id, [0x0D; 32]);
        assert!(!create.has_amount);
        assert!(!create.has_recipient);
        // The document data has no typed projection, so the row is
        // incomplete and renders it.
        assert!(!create.complete);
        let create_details = unsafe { c_str(create.details) };
        assert!(create_details.contains("\"message\""), "{create_details}");
        assert!(create_details.contains("\"hi\""), "{create_details}");

        let transfer = &rows[1];
        assert_eq!(unsafe { c_str(transfer.action) }, "Transfer");
        assert_eq!(unsafe { c_str(transfer.document_type) }, "profile");
        assert!(transfer.has_recipient);
        assert_eq!(transfer.recipient_id, RECIPIENT);
        assert!(transfer.complete);
        assert!(transfer.details.is_null());

        let token_transfer = &rows[2];
        assert_eq!(
            token_transfer.family,
            PARSED_BATCHED_TRANSITION_FAMILY_TOKEN
        );
        assert_eq!(unsafe { c_str(token_transfer.action) }, "Transfer");
        assert!(token_transfer.document_type.is_null());
        assert_eq!(token_transfer.data_contract_id, CONTRACT);
        assert_eq!(token_transfer.token_id, TOKEN);
        assert_eq!(token_transfer.token_contract_position, 3);
        assert!(token_transfer.has_amount);
        assert_eq!(token_transfer.amount, 250);
        assert!(token_transfer.has_recipient);
        assert_eq!(token_transfer.recipient_id, RECIPIENT);

        let purchase = &rows[3];
        assert_eq!(unsafe { c_str(purchase.action) }, "DirectPurchase");
        assert!(purchase.has_amount);
        assert_eq!(purchase.amount, 100_000_000);
        assert!(!purchase.has_recipient);
        assert!(purchase.has_token_count);
        assert_eq!(purchase.token_count, 100);
        assert!(purchase.complete);
        assert_eq!(out.user_fee_increase, 1);
        assert!(!out.complete, "the create row renders its data");

        // The unused payloads stay in their default state.
        assert!(out.identity_update.add_public_keys.is_null());
        assert_eq!(out.credit_transfer.amount, 0);
        assert!(out.data_contract.document_type_names.is_null());

        unsafe { platform_wallet_parse_state_transition_free(&mut out) };
        assert_eq!(out.kind, PARSED_STATE_TRANSITION_KIND_NONE);
        assert!(out.kind_name.is_null());
        assert!(out.serialized.is_null());
        assert!(out.batch.transitions.is_null());
        assert_eq!(out.batch.transitions_count, 0);
    }

    #[test]
    fn an_empty_batch_and_an_unsigned_batch_both_decode() {
        let (result, mut out) = parse(&batch_transition_bytes(vec![], vec![]));
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.kind, PARSED_STATE_TRANSITION_KIND_BATCH);
        assert_eq!(out.batch.transitions_count, 0);
        assert!(out.batch.transitions.is_null());
        assert!(!out.is_signed);
        unsafe { platform_wallet_parse_state_transition_free(&mut out) };
    }

    #[test]
    fn parses_an_identity_update_with_key_limits_and_bounds() {
        let bytes = identity_update_transition_bytes();
        let (result, mut out) = parse(&bytes);

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.kind, PARSED_STATE_TRANSITION_KIND_IDENTITY_UPDATE);
        assert_eq!(unsafe { c_str(out.kind_name) }, "IdentityUpdate");
        assert_eq!(out.owner_id, [0x11; 32]);
        assert!(out.is_signed);
        assert_eq!(out.identity_update.identity_id, [0x11; 32]);
        assert_eq!(out.identity_update.add_public_keys_count, 2);
        assert_eq!(out.identity_update.disable_public_key_ids_count, 2);

        let keys = unsafe {
            slice::from_raw_parts(
                out.identity_update.add_public_keys,
                out.identity_update.add_public_keys_count,
            )
        };
        assert_eq!(keys[0].key_id, 17);
        assert!(!keys[0].has_total_budget);
        assert!(!keys[0].has_expires_at);

        assert_eq!(keys[1].key_id, 18);
        assert_eq!(keys[1].purpose, Purpose::AUTHENTICATION as u8);
        assert_eq!(keys[1].security_level, SecurityLevel::HIGH as u8);
        assert_eq!(keys[1].contract_bounds_kind, 3);
        assert_eq!(keys[1].contract_bounds_id, [0x66; 32]);
        assert!(keys[1].has_total_budget);
        assert_eq!(keys[1].total_budget, 10_000_000_000);
        assert!(keys[1].has_expires_at);
        assert_eq!(keys[1].expires_at, 1_800_000_000_000);

        unsafe { platform_wallet_parse_state_transition_free(&mut out) };
        assert_eq!(out.kind, PARSED_STATE_TRANSITION_KIND_NONE);
        assert!(out.identity_update.add_public_keys.is_null());
    }

    #[test]
    fn parses_a_credit_transfer() {
        let bytes = credit_transfer_bytes();
        let (result, mut out) = parse(&bytes);

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.kind, PARSED_STATE_TRANSITION_KIND_CREDIT_TRANSFER);
        assert_eq!(unsafe { c_str(out.kind_name) }, "IdentityCreditTransfer");
        assert_eq!(out.owner_id, [0x11; 32]);
        assert!(!out.is_signed);
        assert_eq!(out.credit_transfer.identity_id, [0x11; 32]);
        assert_eq!(out.credit_transfer.recipient_id, RECIPIENT);
        assert_eq!(out.credit_transfer.amount, 1_000);

        unsafe { platform_wallet_parse_state_transition_free(&mut out) };
        assert_eq!(out.credit_transfer.amount, 0);
    }

    #[test]
    fn parses_data_contract_create_and_update_with_document_type_names() {
        for (fixture, kind, name) in [
            (
                data_contract_create_bytes(),
                PARSED_STATE_TRANSITION_KIND_DATA_CONTRACT_CREATE,
                "DataContractCreate",
            ),
            (
                data_contract_update_bytes(),
                PARSED_STATE_TRANSITION_KIND_DATA_CONTRACT_UPDATE,
                "DataContractUpdate",
            ),
        ] {
            let (bytes, contract_id, expected_names) = fixture;
            assert!(
                !expected_names.is_empty(),
                "fixture contract has document types"
            );
            let (result, mut out) = parse(&bytes);

            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert_eq!(out.kind, kind);
            assert_eq!(unsafe { c_str(out.kind_name) }, name);
            assert_eq!(out.owner_id, OWNER);
            assert_eq!(out.data_contract.contract_id, contract_id.to_buffer());
            assert_eq!(out.data_contract.owner_id, OWNER);
            assert_eq!(
                out.data_contract.document_type_names_count,
                expected_names.len()
            );
            let names: Vec<String> = unsafe {
                slice::from_raw_parts(
                    out.data_contract.document_type_names,
                    out.data_contract.document_type_names_count,
                )
                .iter()
                .map(|name| c_str(*name))
                .collect()
            };
            assert_eq!(names, expected_names);
            assert!(!out.complete);
            assert!(unsafe { c_str(out.details) }.contains("niceDocument"));

            unsafe { platform_wallet_parse_state_transition_free(&mut out) };
            assert!(out.data_contract.document_type_names.is_null());
            assert_eq!(out.data_contract.document_type_names_count, 0);
        }
    }

    #[test]
    fn reports_kinds_without_a_describer_as_other_with_the_common_fields() {
        use dpp::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
        use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
        use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
        use dpp::voting::vote_polls::VotePoll;
        use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
        use dpp::voting::votes::resource_vote::ResourceVote;
        use dpp::voting::votes::Vote;

        let bytes = StateTransition::MasternodeVote(
            MasternodeVoteTransitionV0 {
                pro_tx_hash: Identifier::from([0x33; 32]),
                voter_identity_id: Identifier::from([0x11; 32]),
                vote: Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
                    vote_poll: VotePoll::ContestedDocumentResourceVotePoll(
                        ContestedDocumentResourceVotePoll {
                            contract_id: Identifier::from(CONTRACT),
                            document_type_name: "domain".to_string(),
                            index_name: "parentNameAndLabel".to_string(),
                            index_values: vec![],
                        },
                    ),
                    resource_vote_choice: ResourceVoteChoice::Abstain,
                })),
                nonce: 1,
                signature_public_key_id: 0,
                signature: BinaryData::new(vec![0x99; 96]),
            }
            .into(),
        )
        .serialize_to_bytes()
        .expect("fixture masternode vote serializes");

        let (result, mut out) = parse(&bytes);

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.kind, PARSED_STATE_TRANSITION_KIND_OTHER);
        assert_eq!(unsafe { c_str(out.kind_name) }, "MasternodeVote");
        assert!(out.has_owner_id);
        assert!(out.is_signed);
        assert_eq!(unsafe { serialized(&out) }, bytes.as_slice());
        // The structured dump names the vote's material fields.
        let details = unsafe { c_str(out.details) };
        assert!(details.contains("parentNameAndLabel"), "{details}");
        assert!(details.contains("Abstain"), "{details}");

        unsafe { platform_wallet_parse_state_transition_free(&mut out) };
        assert!(out.kind_name.is_null());
        assert!(out.details.is_null());
    }

    /// An `OTHER` kind the wallet is likely to be asked to sign; shared with
    /// the client suites as the `credit_withdrawal` fixture.
    fn credit_withdrawal_bytes() -> Vec<u8> {
        use dpp::identity::core_script::CoreScript;
        use dpp::state_transition::identity_credit_withdrawal_transition::v1::IdentityCreditWithdrawalTransitionV1;
        use dpp::withdrawal::Pooling;

        StateTransition::IdentityCreditWithdrawal(
            IdentityCreditWithdrawalTransitionV1 {
                identity_id: Identifier::from([0x11; 32]),
                amount: 123_456_789,
                core_fee_per_byte: 7,
                pooling: Pooling::Never,
                output_script: Some(CoreScript::from_bytes(vec![0x76, 0xa9, 0x14, 0xAB])),
                nonce: 3,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: BinaryData::new(vec![]),
            }
            .into(),
        )
        .serialize_to_bytes()
        .expect("fixture withdrawal serializes")
    }

    /// The user fee increase is part of the signed bytes and scales the
    /// processing fee, so two otherwise identical transfers must not parse
    /// to the same approval fields.
    /// A token config update naming one action taker must not parse to the
    /// same row as one naming another; the change item is rendered in
    /// `details` and the row is marked incomplete.
    /// Emergency action, price schedule and claim distribution type were
    /// exposed by the old single-purchase parser's predecessor and must not
    /// be lost: each renders in `details` with the row marked incomplete.
    /// The tagged 2,340-byte contract create also decodes as a 47-byte
    /// identity update when the IdentityUpdate tag is prepended; only the
    /// framing that consumes every byte counts, so the contract parses.
    /// Trailing bytes after a complete transition are refused, since the
    /// summary would not show whatever they carry.
    /// The serialized fixtures the Swift (`ParseStateTransitionTests`) and
    /// Kotlin (`StateTransitionParserTest`) suites decode through the same
    /// FFI. Pinned as hex so a change to the fixtures or to DPP's wire
    /// format is caught here first and the client vectors are updated
    /// together.
    #[test]
    fn fixture_bytes_are_pinned_for_the_client_suites() {
        let (contract_create, _, _) = data_contract_create_bytes();
        let (contract_update, _, _) = data_contract_update_bytes();
        let actual = [
            hex::encode(identity_update_transition_bytes()),
            hex::encode(batch_transition_bytes(
                vec![
                    document_create("post"),
                    document_transfer(),
                    token_transfer(),
                    direct_purchase(),
                ],
                vec![],
            )),
            hex::encode(credit_transfer_bytes()),
            hex::encode(contract_create),
            hex::encode(contract_update),
            hex::encode(credit_transfer_bytes_with_fee_increase(65_535)),
            hex::encode(credit_withdrawal_bytes()),
        ];
        let expected = [
            FIXTURE_IDENTITY_UPDATE_HEX,
            FIXTURE_MIXED_BATCH_HEX,
            FIXTURE_CREDIT_TRANSFER_HEX,
            FIXTURE_DATA_CONTRACT_CREATE_HEX,
            FIXTURE_DATA_CONTRACT_UPDATE_HEX,
            FIXTURE_CREDIT_TRANSFER_MAX_FEE_HEX,
            FIXTURE_CREDIT_WITHDRAWAL_HEX,
        ]
        .map(|hex| hex.trim().to_string());
        assert_eq!(
            actual, expected,
            "fixture bytes drifted; update the Swift and Kotlin vectors with these"
        );
    }

    /// The canonical copies live with the Swift tests (`swift test` copies
    /// the `Fixtures` directory into the test bundle); the Kotlin test reads
    /// them through a relative path. Referencing them from here means the
    /// three suites can never disagree about the bytes. Each file is one
    /// hex line plus a trailing newline.
    macro_rules! client_fixture {
        ($name:literal) => {
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../swift-sdk/SwiftTests/SwiftDashSDKTests/Fixtures/StateTransitions/",
                $name,
                ".hex"
            ))
        };
    }
    const FIXTURE_IDENTITY_UPDATE_HEX: &str = client_fixture!("identity_update");
    const FIXTURE_MIXED_BATCH_HEX: &str = client_fixture!("mixed_batch");
    const FIXTURE_CREDIT_TRANSFER_HEX: &str = client_fixture!("credit_transfer");
    const FIXTURE_DATA_CONTRACT_CREATE_HEX: &str = client_fixture!("data_contract_create");
    const FIXTURE_DATA_CONTRACT_UPDATE_HEX: &str = client_fixture!("data_contract_update");
    const FIXTURE_CREDIT_TRANSFER_MAX_FEE_HEX: &str = client_fixture!("credit_transfer_max_fee");
    const FIXTURE_CREDIT_WITHDRAWAL_HEX: &str = client_fixture!("credit_withdrawal");

    #[test]
    fn rejects_malformed_state_transition_bytes() {
        let bytes = [0xde, 0xad, 0xbe, 0xef];
        let (result, out) = parse(&bytes);

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorDeserialization
        );
        assert_eq!(out.kind, PARSED_STATE_TRANSITION_KIND_NONE);
        assert!(out.kind_name.is_null());
        assert!(out.serialized.is_null());
    }

    #[test]
    fn rejects_a_document_type_that_cannot_cross_the_ffi_without_leaking() {
        // The first row projects and owns two C strings; the second fails, so
        // the error path has to release the first row.
        let bytes = batch_transition_bytes(
            vec![document_create("post"), document_create("pro\0file")],
            vec![],
        );
        let (result, out) = parse(&bytes);

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        assert_eq!(out.kind, PARSED_STATE_TRANSITION_KIND_NONE);
        assert!(out.batch.transitions.is_null());
    }

    #[test]
    fn freeing_a_default_value_is_a_safe_no_op() {
        let mut out = ParsedStateTransitionFFI::default();
        unsafe { platform_wallet_parse_state_transition_free(&mut out) };
        assert_eq!(out.kind, PARSED_STATE_TRANSITION_KIND_NONE);
        // Double free of an already-freed value must also be safe.
        unsafe { platform_wallet_parse_state_transition_free(&mut out) };
        unsafe { platform_wallet_parse_state_transition_free(ptr::null_mut()) };
    }
}
