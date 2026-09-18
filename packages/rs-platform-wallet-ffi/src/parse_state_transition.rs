//! FFI parser for raw DPP state transitions handed to the wallet by a dApp
//! (DashConnect `dash-st:` links / QRs, DashPay Connect `sign`).
//!
//! The wallet must never sign opaque bytes a web page hands it without
//! showing the user what they are. This module decodes any state transition
//! kind with the bounded untrusted decoder and projects a typed summary the
//! approval sheet can describe:
//!
//! - `Batch`: one row per batched transition (contract id, document type,
//!   action, and for token transitions the amount and recipient),
//! - `IdentityUpdate`: every key added (purpose, level, bounds, limits) and
//!   every key disabled,
//! - `IdentityCreditTransfer`: recipient and amount,
//! - `DataContractCreate` / `DataContractUpdate`: contract id and document
//!   type names,
//! - everything else: the kind name only.
//!
//! Every result also carries the kind name, the owner id, whether the
//! transition is already signed, and the exact bytes that were decoded (with
//! the variant tag, so the caller can sign what it showed). Nothing is
//! refused on kind: the sheet shows what is asked and the user decides.
//! Kinds without a describer are shown as a structured dump behind an
//! advanced setting on the client side.
//!
//! This module does not sign and does not broadcast.

use std::borrow::Cow;
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;
use std::slice;

use dpp::prelude::Identifier;
use dpp::serialization::PlatformDeserializableUntrusted;
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::document_transition_action_type::DocumentTransitionActionTypeGetter;
use dpp::state_transition::batch_transition::batched_transition::token_transition::TokenTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::token_transition_action_type::TokenTransitionActionTypeGetter;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::batch_transition::batched_transition::document_purchase_transition::v0::v0_methods::DocumentPurchaseTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::v0::v0_methods::DocumentTransferTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::document_update_price_transition::v0::v0_methods::DocumentUpdatePriceTransitionV0Methods;
use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::token_burn_transition::v0::v0_methods::TokenBurnTransitionV0Methods;
use dpp::state_transition::batch_transition::token_destroy_frozen_funds_transition::v0::v0_methods::TokenDestroyFrozenFundsTransitionV0Methods;
use dpp::state_transition::batch_transition::token_direct_purchase_transition::v0::v0_methods::TokenDirectPurchaseTransitionV0Methods;
use dpp::state_transition::batch_transition::token_freeze_transition::v0::v0_methods::TokenFreezeTransitionV0Methods;
use dpp::state_transition::batch_transition::token_mint_transition::v0::v0_methods::TokenMintTransitionV0Methods;
use dpp::state_transition::batch_transition::token_transfer_transition::v0::v0_methods::TokenTransferTransitionV0Methods;
use dpp::state_transition::batch_transition::token_unfreeze_transition::v0::v0_methods::TokenUnfreezeTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::{DocumentTransition, TokenTransition};
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
use dpp::state_transition::{StateTransition, StateTransitionOwned};

use crate::check_ptr;
use crate::error::*;
use crate::identity_update::{
    platform_wallet_parse_identity_update_transition_free, project_parsed_identity_update,
    ParsedIdentityUpdateFFI, IDENTITY_UPDATE_VARIANT_TAG,
};
use crate::unwrap_result_or_return;

/// Positional bincode variant tag of `StateTransition::Batch`.
pub(crate) const BATCH_VARIANT_TAG: u8 = 2;

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
/// `ParsedStateTransitionFFI::kind`: a kind this module has no describer
/// for. Only the common fields (`kind_name`, `owner_id`, `is_signed`,
/// `serialized`) are populated.
pub const PARSED_STATE_TRANSITION_KIND_OTHER: u8 = 255;

/// `ParsedBatchedTransitionFFI::family`: a document transition.
pub const PARSED_BATCHED_TRANSITION_FAMILY_DOCUMENT: u8 = 0;
/// `ParsedBatchedTransitionFFI::family`: a token transition.
pub const PARSED_BATCHED_TRANSITION_FAMILY_TOKEN: u8 = 1;

/// Variant tags tried when the payload appears to use a tagless framing.
/// `IdentityUpdate` first: it is the framing Yappr has actually been observed
/// to send tagless; `Batch` payloads have so far arrived properly tagged.
const TAGLESS_FRAMING_CANDIDATES: &[(u8, &str)] = &[
    (IDENTITY_UPDATE_VARIANT_TAG, "IdentityUpdate"),
    (BATCH_VARIANT_TAG, "Batch"),
];

/// One transition inside a parsed `BatchTransition`.
///
/// `action` is a Rust-owned NUL-terminated string naming the action as
/// `DocumentTransitionActionType` / `TokenTransitionActionType` spell it
/// (`Create`, `Replace`, `Delete`, `Transfer`, `Purchase`, `UpdatePrice`,
/// `IndexOnlyDelete`; `Burn`, `Mint`, `Transfer`, `Freeze`, `Unfreeze`,
/// `DestroyFrozenFunds`, `Claim`, `EmergencyAction`, `ConfigUpdate`,
/// `DirectPurchase`, `SetPriceForDirectPurchase`). `document_type` is set
/// for document transitions and null for token ones. Both are released by
/// [`platform_wallet_parse_state_transition_free`].
///
/// `has_amount` / `amount` carry the credits or tokens the transition
/// moves: a document purchase price, an update-price value, a token
/// transfer / mint / burn amount, or a direct purchase's total agreed
/// price. `has_recipient` / `recipient_id` carry the identity on the other
/// side: a document transfer's new owner, a token transfer's recipient, a
/// mint's issued-to identity, or the frozen identity of a freeze / unfreeze
/// / destroy. Both are interpreted against `action`: the sheet should say
/// "freeze the tokens of X" for a `Freeze`, not "send to X".
/// `token_contract_position` and `token_id` are set for every token
/// transition.
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
    /// The bytes that were decoded, always in tagged framing (the variant
    /// tag was prepended when the input arrived tagless). These, not the
    /// input, are what a caller should sign after approval.
    pub serialized: *mut u8,
    pub serialized_len: usize,
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
            serialized: ptr::null_mut(),
            serialized_len: 0,
            identity_update: ParsedIdentityUpdateFFI::default(),
            batch: ParsedBatchFFI::default(),
            credit_transfer: ParsedCreditTransferFFI::default(),
            data_contract: ParsedDataContractFFI::default(),
        }
    }
}

/// Deserializes `bytes` as a `StateTransition`, tolerating both normal
/// tagged DPP framing and the tagless framing Yappr sends, where the
/// positional bincode enum variant tag has to be prepended first.
///
/// Every framing is tried: the bytes as they are, and the bytes with each
/// of the `candidates` tags prepended. Exactly one must decode. A payload
/// that decodes under two framings is refused rather than described under
/// whichever was tried first: the caller signs what the user was shown,
/// so an ambiguous payload could otherwise be approved as one transition
/// and broadcast as another. In practice a tagless body only decodes with
/// its own tag prepended and a tagged body only decodes as-is, so the
/// happy path has exactly one hit.
///
/// Returns the transition together with the bytes that decoded it (tagged).
pub(crate) fn deserialize_transition_with_flexible_framing(
    bytes: &[u8],
    candidates: &[(u8, &str)],
) -> Result<(StateTransition, Vec<u8>), PlatformWalletFFIResult> {
    let as_is: (Cow<'_, [u8]>, String) = (Cow::Borrowed(bytes), "as-is".to_string());
    let prepended = candidates.iter().map(|(tag, name)| {
        let mut prefixed = Vec::with_capacity(bytes.len() + 1);
        prefixed.push(*tag);
        prefixed.extend_from_slice(bytes);
        (
            Cow::Owned(prefixed),
            format!("{name} variant tag prepended"),
        )
    });
    let attempts: Vec<(Cow<'_, [u8]>, String)> = std::iter::once(as_is).chain(prepended).collect();

    let mut decoded: Vec<(StateTransition, Vec<u8>, &str)> = Vec::new();
    let mut failures: Vec<String> = Vec::with_capacity(attempts.len());
    for (payload, label) in &attempts {
        match StateTransition::deserialize_from_bytes_untrusted(payload) {
            Ok(state_transition) => decoded.push((state_transition, payload.to_vec(), label)),
            Err(error) => failures.push(format!("{label}: {error}")),
        }
    }

    match decoded.len() {
        1 => {
            let (transition, payload, _) = decoded.remove(0);
            Ok((transition, payload))
        }
        0 => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorDeserialization,
            format!(
                "Failed to deserialize state transition in any supported framing ({})",
                failures.join("; ")
            ),
        )),
        _ => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorDeserialization,
            format!(
                "Ambiguous state transition framing: the bytes decode under more than one \
                 framing ({}); refusing to guess which one the sender meant",
                decoded
                    .iter()
                    .map(|(transition, _, label)| format!("{label} as {}", transition.name()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )),
    }
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

unsafe fn free_c_string(ptr: &mut *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(*ptr));
        *ptr = ptr::null_mut();
    }
}

fn project_document_transition(
    transition: &DocumentTransition,
) -> Result<ParsedBatchedTransitionFFI, PlatformWalletFFIResult> {
    let (amount, recipient) = match transition {
        DocumentTransition::Transfer(t) => (None, Some(t.recipient_owner_id())),
        DocumentTransition::Purchase(t) => (Some(t.price()), None),
        DocumentTransition::UpdatePrice(t) => (Some(t.price()), None),
        DocumentTransition::Create(_)
        | DocumentTransition::Replace(_)
        | DocumentTransition::Delete(_)
        | DocumentTransition::IndexOnlyDelete(_) => (None, None),
    };
    let action = owned_c_string(
        &format!("{:?}", transition.action_type()),
        "Batched document transition action",
    )?;
    let document_type = owned_c_string(
        transition.document_type_name(),
        "Batched document transition document type",
    )?;

    Ok(ParsedBatchedTransitionFFI {
        family: PARSED_BATCHED_TRANSITION_FAMILY_DOCUMENT,
        data_contract_id: transition.data_contract_id().to_buffer(),
        action: action.into_raw(),
        document_type: document_type.into_raw(),
        document_id: transition.get_id().to_buffer(),
        has_amount: amount.is_some(),
        amount: amount.unwrap_or_default(),
        has_recipient: recipient.is_some(),
        recipient_id: recipient.map(|id| id.to_buffer()).unwrap_or_default(),
        ..ParsedBatchedTransitionFFI::default()
    })
}

fn project_token_transition(
    transition: &TokenTransition,
) -> Result<ParsedBatchedTransitionFFI, PlatformWalletFFIResult> {
    let (amount, recipient): (Option<u64>, Option<Identifier>) = match transition {
        TokenTransition::Transfer(t) => (Some(t.amount()), Some(t.recipient_id())),
        TokenTransition::Mint(t) => (Some(t.amount()), t.issued_to_identity_id()),
        TokenTransition::Burn(t) => (Some(t.burn_amount()), None),
        TokenTransition::Freeze(t) => (None, Some(t.frozen_identity_id())),
        TokenTransition::Unfreeze(t) => (None, Some(t.frozen_identity_id())),
        TokenTransition::DestroyFrozenFunds(t) => (None, Some(t.frozen_identity_id())),
        TokenTransition::DirectPurchase(t) => (Some(t.total_agreed_price()), None),
        TokenTransition::Claim(_)
        | TokenTransition::EmergencyAction(_)
        | TokenTransition::ConfigUpdate(_)
        | TokenTransition::SetPriceForDirectPurchase(_) => (None, None),
    };
    let action = owned_c_string(
        &transition.action_type().to_string(),
        "Batched token transition action",
    )?;
    let base = transition.base();

    Ok(ParsedBatchedTransitionFFI {
        family: PARSED_BATCHED_TRANSITION_FAMILY_TOKEN,
        data_contract_id: transition.data_contract_id().to_buffer(),
        action: action.into_raw(),
        document_type: ptr::null_mut(),
        token_contract_position: base.token_contract_position(),
        token_id: transition.token_id().to_buffer(),
        has_amount: amount.is_some(),
        amount: amount.unwrap_or_default(),
        has_recipient: recipient.is_some(),
        recipient_id: recipient.map(|id| id.to_buffer()).unwrap_or_default(),
        ..ParsedBatchedTransitionFFI::default()
    })
}

unsafe fn free_batched_transitions(rows: &mut [ParsedBatchedTransitionFFI]) {
    for row in rows.iter_mut() {
        free_c_string(&mut row.action);
        free_c_string(&mut row.document_type);
    }
}

fn project_parsed_batch(
    batch: &BatchTransition,
) -> Result<ParsedBatchFFI, PlatformWalletFFIResult> {
    let mut rows: Vec<ParsedBatchedTransitionFFI> = Vec::with_capacity(batch.transitions_len());
    for transition in batch.transitions_iter() {
        let projected = match transition {
            BatchedTransitionRef::Document(document) => project_document_transition(document),
            BatchedTransitionRef::Token(token) => project_token_transition(token),
        };
        match projected {
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
        owner_id: batch.owner_id().to_buffer(),
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
    contract: &dpp::data_contract::serialized_version::DataContractInSerializationFormat,
) -> Result<ParsedDataContractFFI, PlatformWalletFFIResult> {
    // `Vec<CString>` owns every name until all of them have converted, so a
    // failure partway through frees what was built with no manual cleanup.
    let mut names: Vec<CString> = Vec::with_capacity(contract.document_schemas().len());
    for name in contract.document_schemas().keys() {
        names.push(owned_c_string(name, "Data contract document type name")?);
    }

    let document_type_names_count = names.len();
    let document_type_names = if document_type_names_count == 0 {
        ptr::null_mut()
    } else {
        let raw: Vec<*mut c_char> = names.into_iter().map(CString::into_raw).collect();
        Box::into_raw(raw.into_boxed_slice()) as *mut *mut c_char
    };

    Ok(ParsedDataContractFFI {
        contract_id: contract.id().to_buffer(),
        owner_id: contract.owner_id().to_buffer(),
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

/// Projects a decoded transition into the C struct. Everything allocated
/// is owned by `out` on success; on failure nothing is left allocated.
fn project_parsed_state_transition(
    transition: &StateTransition,
    serialized: Vec<u8>,
) -> Result<ParsedStateTransitionFFI, PlatformWalletFFIResult> {
    let mut out = ParsedStateTransitionFFI::default();

    let payload = match transition {
        StateTransition::IdentityUpdate(identity_update) => {
            project_parsed_identity_update(identity_update).map(|parsed| {
                out.identity_update = parsed;
                PARSED_STATE_TRANSITION_KIND_IDENTITY_UPDATE
            })
        }
        StateTransition::Batch(batch) => project_parsed_batch(batch).map(|parsed| {
            out.batch = parsed;
            PARSED_STATE_TRANSITION_KIND_BATCH
        }),
        StateTransition::IdentityCreditTransfer(transfer) => {
            out.credit_transfer = ParsedCreditTransferFFI {
                identity_id: transfer.identity_id().to_buffer(),
                recipient_id: transfer.recipient_id().to_buffer(),
                amount: transfer.amount(),
            };
            Ok(PARSED_STATE_TRANSITION_KIND_CREDIT_TRANSFER)
        }
        StateTransition::DataContractCreate(create) => {
            project_parsed_data_contract(create.data_contract()).map(|parsed| {
                out.data_contract = parsed;
                PARSED_STATE_TRANSITION_KIND_DATA_CONTRACT_CREATE
            })
        }
        StateTransition::DataContractUpdate(update) => {
            project_parsed_data_contract(update.data_contract()).map(|parsed| {
                out.data_contract = parsed;
                PARSED_STATE_TRANSITION_KIND_DATA_CONTRACT_UPDATE
            })
        }
        _ => Ok(PARSED_STATE_TRANSITION_KIND_OTHER),
    };
    let kind = match payload {
        Ok(kind) => kind,
        Err(error) => {
            // Nothing kind-specific was stored on a failure, so only the
            // default (empty) payloads are released here.
            unsafe { free_parsed_state_transition_payloads(&mut out) };
            return Err(error);
        }
    };

    let kind_name = match owned_c_string(&transition.name(), "State transition kind name") {
        Ok(name) => name.into_raw(),
        Err(error) => {
            unsafe { free_parsed_state_transition_payloads(&mut out) };
            return Err(error);
        }
    };

    let owner_id = transition.owner_id();
    let serialized_len = serialized.len();
    let serialized_ptr = Box::into_raw(serialized.into_boxed_slice()) as *mut u8;

    out.kind = kind;
    out.kind_name = kind_name;
    out.has_owner_id = owner_id.is_some();
    out.owner_id = owner_id.map(|id| id.to_buffer()).unwrap_or_default();
    out.is_signed = transition.signature().is_some_and(|sig| !sig.is_empty());
    out.serialized = serialized_ptr;
    out.serialized_len = serialized_len;
    Ok(out)
}

unsafe fn free_parsed_state_transition_payloads(parsed: &mut ParsedStateTransitionFFI) {
    platform_wallet_parse_identity_update_transition_free(&mut parsed.identity_update);
    free_parsed_batch(&mut parsed.batch);
    free_parsed_data_contract(&mut parsed.data_contract);
    parsed.credit_transfer = ParsedCreditTransferFFI::default();
}

/// Deserializes a raw DPP state transition (as carried by a DashConnect
/// `dash-st:` link / QR or a DashPay Connect `sign` request) into its
/// inspectable parts, reporting which kind it found in `out.kind` so the
/// caller can branch without probing kind-specific parsers.
///
/// Every kind decodes. `IdentityUpdate`, `Batch`, `IdentityCreditTransfer`
/// and the two data contract transitions get a typed summary (see the
/// module doc); every other kind is reported as
/// `PARSED_STATE_TRANSITION_KIND_OTHER` with the common fields only
/// (`kind_name`, `owner_id`, `is_signed`, `serialized`), so the client can
/// show a structured dump rather than refuse.
///
/// Accepts both normal tagged DPP state-transition bytes and Yappr's
/// tagless framing, where the positional bincode enum variant tag has to be
/// prepended before deserialization; `out.serialized` always holds the
/// tagged bytes that actually decoded.
///
/// Does NOT sign and does NOT broadcast.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_parse_state_transition(
    transition_bytes: *const u8,
    transition_len: usize,
    out: *mut ParsedStateTransitionFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(transition_bytes);
    check_ptr!(out);

    *out = ParsedStateTransitionFFI::default();

    let bytes = slice::from_raw_parts(transition_bytes, transition_len);
    let (transition, serialized) = unwrap_result_or_return!(
        deserialize_transition_with_flexible_framing(bytes, TAGLESS_FRAMING_CANDIDATES)
    );

    *out = unwrap_result_or_return!(project_parsed_state_transition(&transition, serialized));

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
    free_parsed_state_transition_payloads(parsed);
    free_c_string(&mut parsed.kind_name);
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
        StateTransition::IdentityCreditTransfer(
            IdentityCreditTransferTransitionV0 {
                identity_id: Identifier::from([0x11; 32]),
                recipient_id: Identifier::from(RECIPIENT),
                amount: 1_000,
                nonce: 1,
                user_fee_increase: 0,
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

        let transfer = &rows[1];
        assert_eq!(unsafe { c_str(transfer.action) }, "Transfer");
        assert_eq!(unsafe { c_str(transfer.document_type) }, "profile");
        assert!(transfer.has_recipient);
        assert_eq!(transfer.recipient_id, RECIPIENT);

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
    fn parses_a_tagless_batch_by_prepending_the_batch_tag_and_reports_tagged_bytes() {
        let tagged = batch_transition_bytes(vec![direct_purchase()], vec![0x88; 65]);
        assert_eq!(
            tagged[0], BATCH_VARIANT_TAG,
            "StateTransition::Batch variant tag drifted"
        );
        let tagless = tagged[1..].to_vec();

        let (result, mut out) = parse(&tagless);

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.kind, PARSED_STATE_TRANSITION_KIND_BATCH);
        assert_eq!(unsafe { serialized(&out) }, tagged.as_slice());

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

        unsafe { platform_wallet_parse_state_transition_free(&mut out) };
        assert!(out.kind_name.is_null());
    }

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
        ];
        let expected = [
            FIXTURE_IDENTITY_UPDATE_HEX,
            FIXTURE_MIXED_BATCH_HEX,
            FIXTURE_CREDIT_TRANSFER_HEX,
            FIXTURE_DATA_CONTRACT_CREATE_HEX,
            FIXTURE_DATA_CONTRACT_UPDATE_HEX,
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

    /// Tagged bytes of every described kind decode as themselves: the
    /// tagless candidates are also tried, and none may happen to decode as
    /// well, or the parse would be refused as ambiguous. This is the
    /// guarantee the approval sheet rests on: what is described is the
    /// transition the bytes carry.
    #[test]
    fn tagged_bytes_of_every_kind_decode_unambiguously_as_their_own_kind() {
        let (contract_create, _, _) = data_contract_create_bytes();
        let (contract_update, _, _) = data_contract_update_bytes();
        let fixtures = [
            (
                identity_update_transition_bytes(),
                PARSED_STATE_TRANSITION_KIND_IDENTITY_UPDATE,
            ),
            (
                batch_transition_bytes(vec![document_create("post"), token_transfer()], vec![]),
                PARSED_STATE_TRANSITION_KIND_BATCH,
            ),
            (
                credit_transfer_bytes(),
                PARSED_STATE_TRANSITION_KIND_CREDIT_TRANSFER,
            ),
            (
                contract_create,
                PARSED_STATE_TRANSITION_KIND_DATA_CONTRACT_CREATE,
            ),
            (
                contract_update,
                PARSED_STATE_TRANSITION_KIND_DATA_CONTRACT_UPDATE,
            ),
        ];
        for (bytes, expected_kind) in fixtures {
            let (result, mut out) = parse(&bytes);
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert_eq!(out.kind, expected_kind);
            assert_eq!(unsafe { serialized(&out) }, bytes.as_slice());
            unsafe { platform_wallet_parse_state_transition_free(&mut out) };
        }
    }

    /// A payload that decodes under two framings is refused. Built by
    /// hand: a tagless identity update whose first body byte happens to be
    /// a valid tag would be the real-world case; here the ambiguity is
    /// forced by asking the helper to try a candidate that reproduces the
    /// as-is bytes.
    #[test]
    fn ambiguous_framing_is_refused() {
        let tagged = credit_transfer_bytes();
        let tagless = tagged[1..].to_vec();
        // Candidate `7` (IdentityCreditTransfer's tag) makes the tagless body
        // decode; asking for the same tag twice means two framings decode.
        let result = deserialize_transition_with_flexible_framing(
            &tagless,
            &[
                (tagged[0], "CreditTransfer"),
                (tagged[0], "CreditTransfer again"),
            ],
        );
        let mut error = match result {
            Ok(_) => panic!("two decoding framings must be refused"),
            Err(error) => error,
        };
        assert_eq!(
            error.code,
            PlatformWalletFFIResultCode::ErrorDeserialization
        );
        let message = unsafe { CStr::from_ptr(error.message) }.to_str().unwrap();
        assert!(message.contains("Ambiguous"), "{message}");
        unsafe { platform_wallet_ffi_result_free(&mut error) };
    }

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
