//! What a state transition a dApp hands the wallet asks for, before the user
//! approves it (DashConnect `dash-st:` links / QRs, DashPay Connect `sign`).
//!
//! The wallet must never sign opaque bytes a web page hands it without showing
//! the user what they are. [`summarize_state_transition`] decodes any kind with
//! rs-dpp's exact untrusted decoder and returns a [`StateTransitionSummary`]:
//! the common fields every kind has, a typed summary for the kinds the approval
//! sheet describes, and a rendering of every material field the typed summary
//! does not cover. Nothing is refused on kind.
//!
//! This module does not sign and does not broadcast.

use std::fmt;

use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::identity::KeyID;
use dpp::prelude::{Identifier, UserFeeIncrease};
use dpp::serialization::{PlatformDeserializableUntrusted, PlatformSerializable};
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::batched_transition::document_purchase_transition::v0::v0_methods::DocumentPurchaseTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::v0::v0_methods::DocumentTransferTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::document_transition_action_type::DocumentTransitionActionTypeGetter;
use dpp::state_transition::batch_transition::batched_transition::document_update_price_transition::v0::v0_methods::DocumentUpdatePriceTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::token_transition::TokenTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::token_transition_action_type::TokenTransitionActionTypeGetter;
use dpp::state_transition::batch_transition::batched_transition::{
    BatchedTransitionRef, DocumentTransition, TokenTransition,
};
use dpp::state_transition::batch_transition::document_base_transition::v1::v1_methods::DocumentBaseTransitionV1Methods;
use dpp::state_transition::batch_transition::document_base_transition::v2::v2_methods::DocumentBaseTransitionV2Methods;
use dpp::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::token_burn_transition::v0::v0_methods::TokenBurnTransitionV0Methods;
use dpp::state_transition::batch_transition::token_claim_transition::v0::v0_methods::TokenClaimTransitionV0Methods;
use dpp::state_transition::batch_transition::token_config_update_transition::v0::v0_methods::TokenConfigUpdateTransitionV0Methods;
use dpp::state_transition::batch_transition::token_destroy_frozen_funds_transition::v0::v0_methods::TokenDestroyFrozenFundsTransitionV0Methods;
use dpp::state_transition::batch_transition::token_direct_purchase_transition::v0::v0_methods::TokenDirectPurchaseTransitionV0Methods;
use dpp::state_transition::batch_transition::token_emergency_action_transition::v0::v0_methods::TokenEmergencyActionTransitionV0Methods;
use dpp::state_transition::batch_transition::token_freeze_transition::v0::v0_methods::TokenFreezeTransitionV0Methods;
use dpp::state_transition::batch_transition::token_mint_transition::v0::v0_methods::TokenMintTransitionV0Methods;
use dpp::state_transition::batch_transition::token_set_price_for_direct_purchase_transition::v0::v0_methods::TokenSetPriceForDirectPurchaseTransitionV0Methods;
use dpp::state_transition::batch_transition::token_transfer_transition::v0::v0_methods::TokenTransferTransitionV0Methods;
use dpp::state_transition::batch_transition::token_unfreeze_transition::v0::v0_methods::TokenUnfreezeTransitionV0Methods;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
use dpp::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::state_transition::{
    StateTransition, StateTransitionOwned, StateTransitionType, STATE_TRANSITION_MAX_ENCODED_BYTES,
};

use crate::error::PlatformWalletError;

/// Kinds some dApps send serialized on their own, without the
/// `StateTransition` variant tag in front (Yappr sends identity updates that
/// way, from wasm-dpp2's inner `toBytes()`).
pub const UNTAGGED_STATE_TRANSITION_KINDS: &[StateTransitionType] = &[
    StateTransitionType::IdentityUpdate,
    StateTransitionType::Batch,
];

/// What a state transition asks for.
#[derive(Debug, Clone, PartialEq)]
pub struct StateTransitionSummary {
    /// `StateTransition::name()`, e.g. `IdentityUpdate`,
    /// `DocumentsBatch([Create, TokenTransfer])`, `MasternodeVote`.
    pub kind_name: String,
    /// The identity the transition acts for; `None` for the asset-lock-funded
    /// and shielded kinds.
    pub owner_id: Option<Identifier>,
    /// Whether the transition already carries a non-empty signature. A `sign`
    /// request must arrive unsigned.
    pub is_signed: bool,
    /// Percentage added to the processing fee Platform charges (`65535` is
    /// about 656 times the base). Part of the signed bytes, so an approval
    /// sheet must show a non-zero value.
    pub user_fee_increase: UserFeeIncrease,
    /// The bytes that decoded, always tagged. These, not the input, are what
    /// a caller signs after approval.
    pub serialized: Vec<u8>,
    pub kind: StateTransitionSummaryKind,
}

impl StateTransitionSummary {
    /// Whether the typed summary shows every material field. When it does not,
    /// a wallet must render the `details` it carries before approval.
    pub fn is_complete(&self) -> bool {
        match &self.kind {
            StateTransitionSummaryKind::Batch { transitions, .. } => transitions
                .iter()
                .all(BatchedTransitionSummary::is_complete),
            StateTransitionSummaryKind::IdentityUpdate { .. }
            | StateTransitionSummaryKind::CreditTransfer { .. } => true,
            StateTransitionSummaryKind::DataContractCreate(_)
            | StateTransitionSummaryKind::DataContractUpdate(_)
            | StateTransitionSummaryKind::Other { .. } => false,
        }
    }
}

/// The typed part of a [`StateTransitionSummary`].
#[derive(Debug, Clone, PartialEq)]
pub enum StateTransitionSummaryKind {
    IdentityUpdate {
        identity_id: Identifier,
        add_public_keys: Vec<IdentityPublicKeyInCreation>,
        disable_public_key_ids: Vec<KeyID>,
    },
    Batch {
        owner_id: Identifier,
        transitions: Vec<BatchedTransitionSummary>,
    },
    CreditTransfer {
        identity_id: Identifier,
        recipient_id: Identifier,
        amount: u64,
    },
    DataContractCreate(DataContractSummary),
    DataContractUpdate(DataContractSummary),
    /// A kind without a describer. `details` is the whole decoded transition.
    Other {
        details: String,
    },
}

/// The contract a data contract create or update registers.
#[derive(Debug, Clone, PartialEq)]
pub struct DataContractSummary {
    pub contract_id: Identifier,
    pub owner_id: Identifier,
    /// Document type names, in the contract's order.
    pub document_type_names: Vec<String>,
    /// The whole transition: the contract (schemas, tokens and their
    /// distribution rules, groups, keywords), and for a create the contract
    /// group it registers and the memberships it declares. Everything the
    /// names above do not show.
    pub details: String,
}

/// One transition inside a batch.
#[derive(Debug, Clone, PartialEq)]
pub struct BatchedTransitionSummary {
    pub data_contract_id: Identifier,
    /// The rs-dpp action name (`Create`, `Transfer`, `DirectPurchase`, ...).
    pub action: String,
    pub target: BatchedTransitionTarget,
    /// Credits or tokens moved: a document purchase price or update-price
    /// value, a token transfer / mint / burn amount, or a direct purchase's
    /// total agreed price.
    pub amount: Option<u64>,
    /// The identity on the other side, read against `action`: a document
    /// transfer's new owner, a token transfer's recipient, a mint's issued-to
    /// identity, or the frozen identity of a freeze / unfreeze / destroy.
    pub recipient_id: Option<Identifier>,
    /// Tokens bought by a direct purchase.
    pub token_count: Option<u64>,
    /// The material fields `amount` / `recipient_id` / `token_count` do not
    /// cover, one per line (document data, a config change item, a price
    /// schedule, notes, group action info). `None` when there are none.
    pub details: Option<String>,
}

impl BatchedTransitionSummary {
    /// Whether `amount` / `recipient_id` / `token_count` show every material
    /// field; when not, `details` must be rendered before approval.
    pub fn is_complete(&self) -> bool {
        self.details.is_none()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BatchedTransitionTarget {
    Document {
        document_type: String,
        document_id: Identifier,
    },
    Token {
        token_id: Identifier,
        token_contract_position: u16,
    },
}

/// Decodes `bytes` and summarizes what they ask for.
///
/// Tagged `StateTransition` bytes and the untagged kinds in
/// [`UNTAGGED_STATE_TRANSITION_KINDS`] are accepted. Every framing is tried
/// with the exact decoder, which refuses leftover bytes, and exactly one must
/// decode: a payload that decodes under two is refused rather than described
/// under whichever was tried first, since the caller signs what the user was
/// shown.
pub fn summarize_state_transition(
    bytes: &[u8],
) -> Result<StateTransitionSummary, PlatformWalletError> {
    let (transition, serialized) = decode_state_transition(bytes, UNTAGGED_STATE_TRANSITION_KINDS)?;
    summarize(&transition, serialized)
}

/// Decodes `bytes` as a tagged `StateTransition` or as one of `untagged_kinds`
/// serialized on its own. Returns the transition and its tagged bytes.
pub fn decode_state_transition(
    bytes: &[u8],
    untagged_kinds: &[StateTransitionType],
) -> Result<(StateTransition, Vec<u8>), PlatformWalletError> {
    let mut decoded: Vec<(StateTransition, String)> = Vec::new();
    let mut failures: Vec<String> = Vec::new();

    match StateTransition::deserialize_from_bytes_untrusted_exact(bytes) {
        Ok(transition) => decoded.push((transition, "tagged".to_string())),
        Err(error) => failures.push(format!("tagged: {error}")),
    }
    for kind in untagged_kinds {
        match StateTransition::deserialize_untagged_untrusted_exact(*kind, bytes) {
            Ok(transition) => decoded.push((transition, format!("untagged {kind}"))),
            Err(error) => failures.push(format!("untagged {kind}: {error}")),
        }
    }

    match decoded.len() {
        0 => Err(PlatformWalletError::InvalidParameter(format!(
            "Failed to deserialize state transition in any supported framing ({})",
            failures.join("; ")
        ))),
        1 => {
            let (transition, _) = decoded.pop().expect("one decoded framing");
            let serialized = transition.serialize_to_bytes().map_err(|error| {
                PlatformWalletError::InvalidParameter(format!(
                    "Decoded state transition does not re-serialize: {error}"
                ))
            })?;
            Ok((transition, serialized))
        }
        _ => Err(PlatformWalletError::InvalidParameter(format!(
            "Ambiguous state transition framing: the bytes decode as {}; refusing to guess \
             which one the sender meant",
            decoded
                .iter()
                .map(|(transition, framing)| format!("{} ({framing})", transition.name()))
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

/// Document type names reach the approval sheet as their own field, unescaped. Consensus only
/// accepts `^[a-zA-Z0-9-_]{1,64}$`, so anything else is refused here rather than shown.
fn check_document_type_name(name: &str) -> Result<(), PlatformWalletError> {
    let valid = (1..=64).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    if valid {
        Ok(())
    } else {
        Err(PlatformWalletError::InvalidParameter(format!(
            "Invalid document type name {name:?}"
        )))
    }
}

fn summarize(
    transition: &StateTransition,
    serialized: Vec<u8>,
) -> Result<StateTransitionSummary, PlatformWalletError> {
    let mut budget = DetailsBudget::default();
    let kind = match transition {
        StateTransition::IdentityUpdate(update) => StateTransitionSummaryKind::IdentityUpdate {
            identity_id: update.identity_id(),
            add_public_keys: update.public_keys_to_add().to_vec(),
            disable_public_key_ids: update.public_key_ids_to_disable().to_vec(),
        },
        StateTransition::Batch(batch) => StateTransitionSummaryKind::Batch {
            owner_id: batch.owner_id(),
            transitions: summarize_batch(batch, &mut budget)?,
        },
        StateTransition::IdentityCreditTransfer(transfer) => {
            StateTransitionSummaryKind::CreditTransfer {
                identity_id: transfer.identity_id(),
                recipient_id: transfer.recipient_id(),
                amount: transfer.amount(),
            }
        }
        StateTransition::DataContractCreate(create) => {
            StateTransitionSummaryKind::DataContractCreate(summarize_contract(
                create.data_contract(),
                budget.render(create)?,
            )?)
        }
        StateTransition::DataContractUpdate(update) => {
            StateTransitionSummaryKind::DataContractUpdate(summarize_contract(
                update.data_contract(),
                budget.render(update)?,
            )?)
        }
        other => StateTransitionSummaryKind::Other {
            details: budget.render(other)?,
        },
    };

    Ok(StateTransitionSummary {
        kind_name: transition.name(),
        owner_id: transition.owner_id(),
        is_signed: transition.signature().is_some_and(|sig| !sig.is_empty()),
        user_fee_increase: transition.user_fee_increase(),
        serialized,
        kind,
    })
}

/// Total bytes of `details` one summary may render: 64 times the largest transition the decoder
/// accepts. Compact `Debug` of the densest values measured (a minimal token configuration) is
/// about 42 times its encoding, so a valid request fits; nested values cannot grow past the bound.
pub const MAX_DETAILS_BYTES: usize = 64 * STATE_TRANSITION_MAX_ENCODED_BYTES;

/// Renders `details` with compact `Debug`, which quotes and escapes every string a dApp controls,
/// under one byte budget for the whole summary. Formatting stops as soon as the budget is spent,
/// and the summary is refused rather than shown truncated.
struct DetailsBudget {
    limit: usize,
    used: usize,
}

impl Default for DetailsBudget {
    fn default() -> Self {
        Self {
            limit: MAX_DETAILS_BYTES,
            used: 0,
        }
    }
}

impl DetailsBudget {
    fn render(&mut self, value: &dyn fmt::Debug) -> Result<String, PlatformWalletError> {
        struct Bounded<'a> {
            out: String,
            remaining: &'a mut usize,
        }
        impl fmt::Write for Bounded<'_> {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                *self.remaining = self.remaining.checked_sub(s.len()).ok_or(fmt::Error)?;
                self.out.push_str(s);
                Ok(())
            }
        }

        let mut remaining = self.limit - self.used;
        let mut sink = Bounded {
            out: String::new(),
            remaining: &mut remaining,
        };
        fmt::write(&mut sink, format_args!("{value:?}")).map_err(|_| {
            PlatformWalletError::InvalidParameter(format!(
                "State transition details exceed {} bytes; refusing to summarize it",
                self.limit
            ))
        })?;
        let out = base58_identifiers(&sink.out);
        self.used = self.limit - remaining;
        Ok(out)
    }

    /// `label: <Debug of value>`.
    fn line(&mut self, label: &str, value: &dyn fmt::Debug) -> Result<String, PlatformWalletError> {
        Ok(format!("{label}: {}", self.render(value)?))
    }
}

/// Rewrites every `Identifier(IdentifierBytes32([b0, .., b31]))` that `Debug` produces as
/// `Identifier(<base58>)`, the form a user can compare with other tools. Only exact 32-byte runs
/// are rewritten, and the output is never longer than the input.
fn base58_identifiers(rendered: &str) -> String {
    const PREFIX: &str = "Identifier(IdentifierBytes32([";
    const SUFFIX: &str = "]))";
    let mut out = String::with_capacity(rendered.len());
    let mut rest = rendered;
    while let Some(start) = rest.find(PREFIX) {
        out.push_str(&rest[..start]);
        let after = &rest[start + PREFIX.len()..];
        let parsed = after.find(SUFFIX).and_then(|end| {
            let bytes: Vec<u8> = after[..end]
                .split(", ")
                .map(str::parse)
                .collect::<Result<_, _>>()
                .ok()?;
            let bytes: [u8; 32] = bytes.try_into().ok()?;
            Some((Identifier::from(bytes), end))
        });
        match parsed {
            Some((id, end)) => {
                out.push_str(&format!("Identifier({id})"));
                rest = &after[end + SUFFIX.len()..];
            }
            None => {
                out.push_str(PREFIX);
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// `details` is the whole transition, not only its contract: a create transition also carries
/// the contract group it registers and the memberships it declares.
fn summarize_contract(
    contract: &DataContractInSerializationFormat,
    details: String,
) -> Result<DataContractSummary, PlatformWalletError> {
    let document_type_names: Vec<String> = contract.document_schemas().keys().cloned().collect();
    for name in &document_type_names {
        check_document_type_name(name)?;
    }
    Ok(DataContractSummary {
        contract_id: contract.id(),
        owner_id: contract.owner_id(),
        document_type_names,
        details,
    })
}

fn summarize_batch(
    batch: &BatchTransition,
    budget: &mut DetailsBudget,
) -> Result<Vec<BatchedTransitionSummary>, PlatformWalletError> {
    batch
        .transitions_iter()
        .map(|transition| match transition {
            BatchedTransitionRef::Document(document) => {
                summarize_document_transition(document, budget)
            }
            BatchedTransitionRef::Token(token) => summarize_token_transition(token, budget),
        })
        .collect()
}

fn join_details(lines: Vec<String>) -> Option<String> {
    (!lines.is_empty()).then(|| lines.join("\n"))
}

fn summarize_document_transition(
    transition: &DocumentTransition,
    budget: &mut DetailsBudget,
) -> Result<BatchedTransitionSummary, PlatformWalletError> {
    check_document_type_name(transition.document_type_name())?;
    let (amount, recipient_id) = match transition {
        DocumentTransition::Transfer(t) => (None, Some(t.recipient_owner_id())),
        DocumentTransition::Purchase(t) => (Some(t.price()), None),
        DocumentTransition::UpdatePrice(t) => (Some(t.price()), None),
        DocumentTransition::Create(_)
        | DocumentTransition::Replace(_)
        | DocumentTransition::Delete(_)
        | DocumentTransition::IndexOnlyDelete(_) => (None, None),
    };

    let mut lines = Vec::new();
    if let Some(data) = transition.data() {
        lines.push(budget.line("data", data)?);
    }
    if let DocumentTransition::Create(create) = transition {
        if let Some((index_name, credits)) = create.prefunded_voting_balance() {
            lines.push(format!(
                "prefunded voting balance: {credits} credits for index {}",
                budget.render(index_name)?
            ));
        }
    }
    let base = transition.base();
    if let Some(payment) = base.token_payment_info() {
        lines.push(budget.line("token payment", &payment)?);
    }
    if let Some(fees) = base.action_fee_agreement() {
        lines.push(budget.line("action fees", &fees)?);
    }

    Ok(BatchedTransitionSummary {
        data_contract_id: transition.data_contract_id(),
        action: format!("{:?}", transition.action_type()),
        target: BatchedTransitionTarget::Document {
            document_type: transition.document_type_name().clone(),
            document_id: transition.get_id(),
        },
        amount,
        recipient_id,
        token_count: None,
        details: join_details(lines),
    })
}

fn summarize_token_transition(
    transition: &TokenTransition,
    budget: &mut DetailsBudget,
) -> Result<BatchedTransitionSummary, PlatformWalletError> {
    let mut lines = Vec::new();
    let (amount, recipient_id, token_count, public_note) = match transition {
        TokenTransition::Transfer(t) => {
            if t.shared_encrypted_note().is_some() {
                lines.push("shared encrypted note: present".to_string());
            }
            if t.private_encrypted_note().is_some() {
                lines.push("private encrypted note: present".to_string());
            }
            (
                Some(t.amount()),
                Some(t.recipient_id()),
                None,
                t.public_note(),
            )
        }
        TokenTransition::Mint(t) => {
            if t.issued_to_identity_id().is_none() {
                lines.push("issued to: the token's default destination".to_string());
            }
            (
                Some(t.amount()),
                t.issued_to_identity_id(),
                None,
                t.public_note(),
            )
        }
        TokenTransition::Burn(t) => (Some(t.burn_amount()), None, None, t.public_note()),
        TokenTransition::Freeze(t) => (None, Some(t.frozen_identity_id()), None, t.public_note()),
        TokenTransition::Unfreeze(t) => (None, Some(t.frozen_identity_id()), None, t.public_note()),
        TokenTransition::DestroyFrozenFunds(t) => {
            (None, Some(t.frozen_identity_id()), None, t.public_note())
        }
        TokenTransition::DirectPurchase(t) => (
            Some(t.total_agreed_price()),
            None,
            Some(t.token_count()),
            None,
        ),
        TokenTransition::Claim(t) => {
            lines.push(budget.line("distribution type", &t.distribution_type())?);
            (None, None, None, t.public_note())
        }
        TokenTransition::EmergencyAction(t) => {
            lines.push(budget.line("emergency action", &t.emergency_action())?);
            (None, None, None, t.public_note())
        }
        TokenTransition::ConfigUpdate(t) => {
            lines.push(budget.line("config update", t.update_token_configuration_item())?);
            (None, None, None, t.public_note())
        }
        TokenTransition::SetPriceForDirectPurchase(t) => {
            match t.price() {
                Some(schedule) => lines.push(budget.line("price schedule", schedule)?),
                None => lines.push("price schedule: removed (not for sale)".to_string()),
            }
            (None, None, None, t.public_note())
        }
    };
    if let Some(note) = public_note {
        lines.push(budget.line("public note", note)?);
    }
    let base = transition.base();
    if let Some(group) = base.using_group_info() {
        lines.push(format!(
            "group action: position {}, action id {}, {}",
            group.group_contract_position,
            group.action_id,
            if group.action_is_proposer {
                "proposing"
            } else {
                "signing an existing proposal"
            }
        ));
    }

    Ok(BatchedTransitionSummary {
        data_contract_id: transition.data_contract_id(),
        action: transition.action_type().to_string(),
        target: BatchedTransitionTarget::Token {
            token_id: transition.token_id(),
            token_contract_position: base.token_contract_position(),
        },
        amount,
        recipient_id,
        token_count,
        details: join_details(lines),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::document_type::action_fees::agreement::v0::DocumentActionFeeAgreementV0;
    use dpp::state_transition::batch_transition::document_base_transition::v1::DocumentBaseTransitionV1;
    use dpp::state_transition::batch_transition::document_base_transition::v2::DocumentBaseTransitionV2;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use dpp::tokens::token_payment_info::v0::TokenPaymentInfoV0;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::{PlatformVersion, TryFromPlatformVersioned};
    use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
    use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
    use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
    use dpp::identity::core_script::CoreScript;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::{platform_value, BinaryData};
    use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::v0::DocumentTransferTransitionV0;
    use dpp::state_transition::batch_transition::batched_transition::{
        BatchedTransition, DocumentTransferTransition,
    };
    use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
    use dpp::state_transition::batch_transition::document_create_transition::v0::DocumentCreateTransitionV0;
    use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
    use dpp::state_transition::batch_transition::token_claim_transition::v0::TokenClaimTransitionV0;
    use dpp::state_transition::batch_transition::token_config_update_transition::v0::TokenConfigUpdateTransitionV0;
    use dpp::state_transition::batch_transition::token_direct_purchase_transition::v0::TokenDirectPurchaseTransitionV0;
    use dpp::state_transition::batch_transition::token_emergency_action_transition::v0::TokenEmergencyActionTransitionV0;
    use dpp::state_transition::batch_transition::token_set_price_for_direct_purchase_transition::v0::TokenSetPriceForDirectPurchaseTransitionV0;
    use dpp::state_transition::batch_transition::token_transfer_transition::v0::TokenTransferTransitionV0;
    use dpp::state_transition::batch_transition::{
        BatchTransitionV1, DocumentCreateTransition, TokenClaimTransition,
        TokenConfigUpdateTransition, TokenDirectPurchaseTransition,
        TokenEmergencyActionTransition, TokenSetPriceForDirectPurchaseTransition,
        TokenTransferTransition,
    };
    use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use dpp::state_transition::identity_credit_withdrawal_transition::v1::IdentityCreditWithdrawalTransitionV1;
    use dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
    use dpp::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0;
    use dpp::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
    use dpp::data_contract::associated_token::token_configuration_localization::v0::TokenConfigurationLocalizationV0;
    use dpp::data_contract::associated_token::token_configuration_localization::TokenConfigurationLocalization;
    use dpp::platform_value::Value;
    use dpp::contract_group::{ContractGroupMember, ContractGroupMembership, ContractGroupRegistration};
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransitionV1;
    use std::collections::BTreeSet;
    use dpp::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV1Getters;
    use dpp::state_transition::public_key_in_creation::v1::IdentityPublicKeyInCreationV1;
    use dpp::tokens::emergency_action::TokenEmergencyAction;
    use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
    use dpp::withdrawal::Pooling;
    use std::collections::BTreeMap;

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

    fn document_base(document_type_name: &str) -> DocumentBaseTransition {
        DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Identifier::from([0x0D; 32]),
            identity_contract_nonce: 1,
            document_type_name: document_type_name.to_string(),
            data_contract_id: Identifier::from(CONTRACT),
        })
    }

    fn batch(transitions: Vec<BatchedTransition>) -> StateTransition {
        StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
            owner_id: Identifier::from(OWNER),
            transitions,
            user_fee_increase: 1,
            signature_public_key_id: 2,
            signature: BinaryData::new(vec![0x88; 65]),
        }))
    }

    fn credit_transfer(user_fee_increase: u16) -> StateTransition {
        IdentityCreditTransferTransitionV0 {
            identity_id: Identifier::from([0x11; 32]),
            recipient_id: Identifier::from(RECIPIENT),
            amount: 1_000,
            nonce: 1,
            user_fee_increase,
            signature_public_key_id: 0,
            signature: BinaryData::new(vec![]),
        }
        .into()
    }

    fn summarize_bytes(transition: &StateTransition) -> StateTransitionSummary {
        summarize_state_transition(&transition.serialize_to_bytes().unwrap()).expect("summarizes")
    }

    #[test]
    fn a_mixed_batch_summarizes_one_row_per_transition() {
        let transition = batch(vec![
            BatchedTransition::Document(DocumentTransition::Create(DocumentCreateTransition::V0(
                DocumentCreateTransitionV0 {
                    base: document_base("post"),
                    entropy: [0xEE; 32],
                    data: BTreeMap::from([("message".to_string(), platform_value!("hi"))]),
                    prefunded_voting_balance: None,
                },
            ))),
            BatchedTransition::Document(DocumentTransition::Transfer(
                DocumentTransferTransition::V0(DocumentTransferTransitionV0 {
                    base: document_base("profile"),
                    revision: 2,
                    recipient_owner_id: Identifier::from(RECIPIENT),
                }),
            )),
            BatchedTransition::Token(TokenTransition::Transfer(TokenTransferTransition::V0(
                TokenTransferTransitionV0 {
                    base: token_base(),
                    amount: 250,
                    recipient_id: Identifier::from(RECIPIENT),
                    public_note: None,
                    shared_encrypted_note: None,
                    private_encrypted_note: None,
                },
            ))),
            BatchedTransition::Token(TokenTransition::DirectPurchase(
                TokenDirectPurchaseTransition::V0(TokenDirectPurchaseTransitionV0 {
                    base: token_base(),
                    token_count: 100,
                    total_agreed_price: 100_000_000,
                }),
            )),
        ]);
        let summary = summarize_bytes(&transition);

        assert_eq!(
            summary.kind_name,
            "DocumentsBatch([Create, Transfer, TokenTransfer, TokenDirectPurchase])"
        );
        assert_eq!(summary.owner_id, Some(Identifier::from(OWNER)));
        assert!(summary.is_signed);
        assert_eq!(summary.user_fee_increase, 1);
        assert_eq!(summary.serialized, transition.serialize_to_bytes().unwrap());
        // The document data has no typed projection.
        assert!(!summary.is_complete());

        let StateTransitionSummaryKind::Batch {
            owner_id,
            transitions,
        } = summary.kind
        else {
            panic!("expected a batch");
        };
        assert_eq!(owner_id, Identifier::from(OWNER));
        let [create, transfer, token_transfer, purchase] = transitions.as_slice() else {
            panic!("expected four rows");
        };

        assert_eq!(create.action, "Create");
        assert_eq!(
            create.target,
            BatchedTransitionTarget::Document {
                document_type: "post".to_string(),
                document_id: Identifier::from([0x0D; 32]),
            }
        );
        let create_details = create.details.as_deref().expect("data is rendered");
        assert!(create_details.contains("\"message\""), "{create_details}");

        assert_eq!(transfer.action, "Transfer");
        assert_eq!(transfer.recipient_id, Some(Identifier::from(RECIPIENT)));
        assert_eq!(transfer.details, None);

        assert_eq!(token_transfer.action, "Transfer");
        assert_eq!(
            token_transfer.target,
            BatchedTransitionTarget::Token {
                token_id: Identifier::from(TOKEN),
                token_contract_position: 3,
            }
        );
        assert_eq!(token_transfer.amount, Some(250));
        assert_eq!(
            token_transfer.recipient_id,
            Some(Identifier::from(RECIPIENT))
        );

        assert_eq!(purchase.action, "DirectPurchase");
        assert_eq!(purchase.amount, Some(100_000_000));
        assert_eq!(purchase.token_count, Some(100));
        assert_eq!(purchase.details, None);
    }

    #[test]
    fn an_identity_update_carries_every_added_key_with_its_limits() {
        let transition: StateTransition = IdentityUpdateTransitionV0 {
            signature: BinaryData::new(vec![0x99; 65]),
            signature_public_key_id: 3,
            identity_id: Identifier::from([0x11; 32]),
            revision: 7,
            nonce: 9,
            add_public_keys: vec![IdentityPublicKeyInCreationV1 {
                id: 18,
                key_type: KeyType::ECDSA_SECP256K1,
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::HIGH,
                read_only: false,
                data: BinaryData::new(vec![0x03; 33]),
                signature: BinaryData::new(vec![0xbb; 65]),
                contract_bounds: None,
                total_budget: Some(10_000_000_000),
                expires_at: Some(1_800_000_000_000),
            }
            .into()],
            disable_public_keys: vec![4, 8],
            user_fee_increase: 2,
        }
        .into();
        let summary = summarize_bytes(&transition);

        assert_eq!(summary.kind_name, "IdentityUpdate");
        assert!(summary.is_complete());
        let StateTransitionSummaryKind::IdentityUpdate {
            identity_id,
            add_public_keys,
            disable_public_key_ids,
        } = summary.kind
        else {
            panic!("expected an identity update");
        };
        assert_eq!(identity_id, Identifier::from([0x11; 32]));
        assert_eq!(disable_public_key_ids, vec![4, 8]);
        assert_eq!(add_public_keys[0].total_budget(), Some(10_000_000_000));
        assert_eq!(add_public_keys[0].expires_at(), Some(1_800_000_000_000));
    }

    /// Identity updates arrive untagged from some dApps; the summary carries
    /// the tagged bytes, which are what the wallet signs.
    #[test]
    fn an_untagged_identity_update_is_summarized_with_tagged_bytes() {
        let transition: StateTransition = IdentityUpdateTransitionV0 {
            identity_id: Identifier::from([0x11; 32]),
            revision: 1,
            nonce: 1,
            disable_public_keys: vec![3],
            ..Default::default()
        }
        .into();
        let tagged = transition.serialize_to_bytes().unwrap();
        let StateTransition::IdentityUpdate(inner) = &transition else {
            unreachable!()
        };
        let untagged = inner.serialize_to_bytes().unwrap();
        assert_ne!(untagged, tagged);

        let summary = summarize_state_transition(&untagged).expect("summarizes");
        assert_eq!(summary.kind_name, "IdentityUpdate");
        assert_eq!(summary.serialized, tagged);
    }

    #[test]
    fn a_credit_transfer_summary() {
        let summary = summarize_bytes(&credit_transfer(0));
        assert_eq!(
            summary.kind,
            StateTransitionSummaryKind::CreditTransfer {
                identity_id: Identifier::from([0x11; 32]),
                recipient_id: Identifier::from(RECIPIENT),
                amount: 1_000,
            }
        );
        assert!(!summary.is_signed);
    }

    /// The fee multiplier is part of the signed bytes and scales the
    /// processing fee, so two otherwise identical transfers must not summarize
    /// to the same thing.
    #[test]
    fn the_user_fee_increase_is_part_of_the_summary() {
        let plain = summarize_bytes(&credit_transfer(0));
        let maxed = summarize_bytes(&credit_transfer(u16::MAX));
        assert_eq!(plain.kind, maxed.kind);
        assert_eq!(plain.user_fee_increase, 0);
        assert_eq!(maxed.user_fee_increase, u16::MAX);
    }

    /// Two config updates granting manual minting to different takers must not
    /// summarize the same; neither is complete from the typed fields alone.
    #[test]
    fn token_details_cover_config_update_emergency_price_and_claim() {
        let config_update = |takers: AuthorizedActionTakers| {
            BatchedTransition::Token(TokenTransition::ConfigUpdate(
                TokenConfigUpdateTransition::V0(TokenConfigUpdateTransitionV0 {
                    base: token_base(),
                    update_token_configuration_item: TokenConfigurationChangeItem::ManualMinting(
                        takers,
                    ),
                    public_note: Some("grant".to_string()),
                }),
            ))
        };
        let summary = summarize_bytes(&batch(vec![
            config_update(AuthorizedActionTakers::ContractOwner),
            config_update(AuthorizedActionTakers::Identity(Identifier::from(
                RECIPIENT,
            ))),
            BatchedTransition::Token(TokenTransition::EmergencyAction(
                TokenEmergencyActionTransition::V0(TokenEmergencyActionTransitionV0 {
                    base: token_base(),
                    emergency_action: TokenEmergencyAction::Pause,
                    public_note: None,
                }),
            )),
            BatchedTransition::Token(TokenTransition::SetPriceForDirectPurchase(
                TokenSetPriceForDirectPurchaseTransition::V0(
                    TokenSetPriceForDirectPurchaseTransitionV0 {
                        base: token_base(),
                        price: Some(TokenPricingSchedule::SinglePrice(4_200)),
                        public_note: None,
                    },
                ),
            )),
            BatchedTransition::Token(TokenTransition::Claim(TokenClaimTransition::V0(
                TokenClaimTransitionV0 {
                    base: token_base(),
                    distribution_type: TokenDistributionType::Perpetual,
                    public_note: None,
                },
            ))),
        ]));
        assert!(!summary.is_complete());
        let StateTransitionSummaryKind::Batch { transitions, .. } = summary.kind else {
            panic!("expected a batch");
        };
        let details: Vec<String> = transitions
            .into_iter()
            .map(|row| row.details.expect("every row renders details"))
            .collect();

        assert_ne!(details[0], details[1]);
        assert!(details[0].contains("ManualMinting"), "{}", details[0]);
        assert!(details[0].contains("ContractOwner"), "{}", details[0]);
        assert!(
            details[0].contains("public note: \"grant\""),
            "{}",
            details[0]
        );
        assert!(
            details[1].contains(&format!("Identifier({})", Identifier::from(RECIPIENT))),
            "{}",
            details[1]
        );
        assert!(
            details[2].contains("emergency action: Pause"),
            "{}",
            details[2]
        );
        assert!(details[3].contains("SinglePrice(4200)"), "{}", details[3]);
        assert!(
            details[4].contains("distribution type: Perpetual"),
            "{}",
            details[4]
        );
    }

    #[test]
    fn a_kind_without_a_describer_carries_the_whole_transition() {
        let transition: StateTransition = IdentityCreditWithdrawalTransitionV1 {
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
        .into();
        let summary = summarize_bytes(&transition);

        assert_eq!(summary.kind_name, "IdentityCreditWithdrawal");
        assert!(!summary.is_complete());
        let StateTransitionSummaryKind::Other { details } = summary.kind else {
            panic!("expected other");
        };
        assert!(details.contains("amount: 123456789"), "{details}");
        assert!(details.contains("Never"), "{details}");
        assert!(details.contains("output_script"), "{details}");
    }

    #[test]
    fn trailing_bytes_and_garbage_are_refused() {
        let mut padded = credit_transfer(0).serialize_to_bytes().unwrap();
        padded.push(0);
        assert!(matches!(
            summarize_state_transition(&padded),
            Err(PlatformWalletError::InvalidParameter(message)) if message.contains("left over")
        ));
        assert!(summarize_state_transition(&[0xde, 0xad, 0xbe, 0xef]).is_err());
    }

    /// Tagged bytes of every described kind, and the untagged kinds' own
    /// bytes, decode under exactly one framing.
    #[test]
    fn every_described_kind_decodes_under_one_framing() {
        for transition in [
            credit_transfer(0),
            batch(vec![]),
            IdentityUpdateTransitionV0::default().into(),
        ] {
            let tagged = transition.serialize_to_bytes().unwrap();
            let (decoded, serialized) =
                decode_state_transition(&tagged, UNTAGGED_STATE_TRANSITION_KINDS)
                    .expect("one framing");
            assert_eq!(decoded, transition);
            assert_eq!(serialized, tagged);
        }
    }

    /// Asking for the same untagged kind twice makes one payload decode under
    /// two framings, which is refused.
    #[test]
    fn ambiguous_framing_is_refused() {
        let transition: StateTransition = IdentityUpdateTransitionV0::default().into();
        let StateTransition::IdentityUpdate(inner) = &transition else {
            unreachable!()
        };
        let untagged = inner.serialize_to_bytes().unwrap();
        let result = decode_state_transition(
            &untagged,
            &[
                StateTransitionType::IdentityUpdate,
                StateTransitionType::IdentityUpdate,
            ],
        );
        assert!(matches!(
            result,
            Err(PlatformWalletError::InvalidParameter(message)) if message.contains("Ambiguous")
        ));
    }

    /// A transfer or delete names no amount of its own, but its base can
    /// commit the owner to a token cost (V1) or to action fees (V2); the row
    /// must render them and not claim to be complete.
    #[test]
    fn document_rows_render_token_payment_and_action_fees() {
        let payment = TokenPaymentInfoV0 {
            payment_token_contract_id: None,
            token_contract_position: 0,
            minimum_token_cost: None,
            maximum_token_cost: Some(5_000),
            gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
        }
        .into();
        let transfer = |base: DocumentBaseTransition| {
            BatchedTransition::Document(DocumentTransition::Transfer(
                DocumentTransferTransition::V0(DocumentTransferTransitionV0 {
                    base,
                    revision: 2,
                    recipient_owner_id: Identifier::from(RECIPIENT),
                }),
            ))
        };
        let summary = summarize_bytes(&batch(vec![
            transfer(DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
                id: Identifier::from([0x0D; 32]),
                identity_contract_nonce: 1,
                document_type_name: "card".to_string(),
                data_contract_id: Identifier::from(CONTRACT),
                token_payment_info: Some(payment),
            })),
            transfer(DocumentBaseTransition::V2(DocumentBaseTransitionV2 {
                id: Identifier::from([0x0D; 32]),
                identity_contract_nonce: 2,
                document_type_name: "card".to_string(),
                data_contract_id: Identifier::from(CONTRACT),
                token_payment_info: None,
                action_fee_agreement: Some(
                    DocumentActionFeeAgreementV0 {
                        owner: 700,
                        moderators: 300,
                        fee_multiplier: None,
                    }
                    .into(),
                ),
            })),
        ]));
        assert!(!summary.is_complete());
        let StateTransitionSummaryKind::Batch { transitions, .. } = summary.kind else {
            panic!("expected a batch");
        };
        let token = transitions[0]
            .details
            .as_deref()
            .expect("token payment rendered");
        assert!(token.contains("token payment:"), "{token}");
        assert!(token.contains("5000"), "{token}");
        let fees = transitions[1]
            .details
            .as_deref()
            .expect("action fees rendered");
        assert!(fees.contains("action fees:"), "{fees}");
        assert!(fees.contains("700") && fees.contains("300"), "{fees}");
    }

    /// Tokens, groups and schemas are what a contract registration asks for
    /// and pays on; the names alone do not show them.
    #[test]
    fn a_data_contract_create_is_not_complete_from_its_names() {
        let created = get_data_contract_fixture(
            Some(Identifier::from(OWNER)),
            1,
            PlatformVersion::latest().protocol_version,
        );
        let contract = DataContractInSerializationFormat::try_from_platform_versioned(
            created.data_contract(),
            PlatformVersion::latest(),
        )
        .expect("serialization format");
        let transition: StateTransition = DataContractCreateTransitionV0 {
            data_contract: contract,
            identity_nonce: 1,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: BinaryData::new(vec![]),
        }
        .into();
        let summary = summarize_bytes(&transition);
        assert!(!summary.is_complete());
        let StateTransitionSummaryKind::DataContractCreate(contract) = summary.kind else {
            panic!("expected a contract create");
        };
        assert_eq!(contract.owner_id, Identifier::from(OWNER));
        assert!(
            contract.details.contains("niceDocument"),
            "{}",
            contract.details
        );
    }

    /// A V1 create also registers a contract group and declares memberships
    /// outside its contract; two creations that differ only there must not
    /// summarize the same.
    #[test]
    fn a_data_contract_create_details_show_its_group_declarations() {
        let created = get_data_contract_fixture(
            Some(Identifier::from(OWNER)),
            1,
            PlatformVersion::latest().protocol_version,
        );
        let contract = DataContractInSerializationFormat::try_from_platform_versioned(
            created.data_contract(),
            PlatformVersion::latest(),
        )
        .expect("serialization format");
        let create = |admin: [u8; 32], group: [u8; 32]| -> StateTransition {
            DataContractCreateTransitionV1 {
                data_contract: contract.clone(),
                identity_nonce: 1,
                contract_group: Some(ContractGroupRegistration {
                    admins: BTreeSet::from([Identifier::from(admin)]),
                    name: None,
                    description: None,
                }),
                contract_group_memberships: vec![ContractGroupMembership {
                    contract_group_id: Identifier::from(group),
                    member: ContractGroupMember::Contract,
                }],
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: BinaryData::new(vec![]),
            }
            .into()
        };
        let details = |transition: &StateTransition| {
            let StateTransitionSummaryKind::DataContractCreate(contract) =
                summarize_bytes(transition).kind
            else {
                panic!("expected a contract create");
            };
            contract.details
        };

        let base = details(&create([0xA1; 32], [0xB1; 32]));
        assert_ne!(base, details(&create([0xA2; 32], [0xB1; 32])), "admins");
        assert_ne!(base, details(&create([0xA1; 32], [0xB2; 32])), "membership");
        assert!(base.contains("contract_group_memberships"), "{base}");
    }

    /// dApp-chosen strings inside a config change (token names) and a
    /// prefunded voting index name stay quoted inside their own field, so
    /// delimiters and newlines cannot make two changes read the same or forge
    /// a line.
    #[test]
    fn dapp_strings_in_details_stay_inside_their_field() {
        let conventions = |singular: &str, plural: &str| {
            BatchedTransition::Token(TokenTransition::ConfigUpdate(
                TokenConfigUpdateTransition::V0(TokenConfigUpdateTransitionV0 {
                    base: token_base(),
                    update_token_configuration_item: TokenConfigurationChangeItem::Conventions(
                        TokenConfigurationConvention::V0(TokenConfigurationConventionV0 {
                            localizations: BTreeMap::from([(
                                "en".to_string(),
                                TokenConfigurationLocalization::V0(
                                    TokenConfigurationLocalizationV0 {
                                        should_capitalize: false,
                                        singular_form: singular.to_string(),
                                        plural_form: plural.to_string(),
                                    },
                                ),
                            )]),
                            decimals: 8,
                        }),
                    ),
                    public_note: None,
                }),
            ))
        };
        let details = |transition: BatchedTransition| {
            let StateTransitionSummaryKind::Batch { transitions, .. } =
                summarize_bytes(&batch(vec![transition])).kind
            else {
                panic!("expected a batch");
            };
            transitions[0].details.clone().expect("details rendered")
        };

        let a = details(conventions("a', Plural: 'b", "c"));
        let b = details(conventions("a", "b', Plural: 'c"));
        assert_ne!(a, b);
        let forged = details(conventions("x\npublic note: \"trusted\"", "y"));
        assert_eq!(forged.lines().count(), 1, "{forged}");

        let create = BatchedTransition::Document(DocumentTransition::Create(
            DocumentCreateTransition::V0(DocumentCreateTransitionV0 {
                base: document_base("post"),
                entropy: [0xEE; 32],
                data: BTreeMap::new(),
                prefunded_voting_balance: Some(("idx\npublic note: \"x\"".to_string(), 1)),
            }),
        ));
        let index = details(create);
        assert!(
            !index.lines().any(|line| line.starts_with("public note")),
            "{index}"
        );
    }

    /// Rendering stops at the first write past the budget and the summary is
    /// refused, without formatting the rest or truncating it.
    #[test]
    fn details_beyond_the_budget_are_refused() {
        use std::cell::Cell;

        /// Writes `chunks` pieces of 100 bytes, counting how many it wrote.
        struct Chunks<'a> {
            chunks: usize,
            written: &'a Cell<usize>,
        }
        impl fmt::Debug for Chunks<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                for _ in 0..self.chunks {
                    f.write_str(&"x".repeat(100))?;
                    self.written.set(self.written.get() + 1);
                }
                Ok(())
            }
        }

        let written = Cell::new(0);
        let mut budget = DetailsBudget {
            limit: 1_000,
            used: 0,
        };
        let refused = budget.render(&Chunks {
            chunks: 1_000,
            written: &written,
        });
        assert!(matches!(
            refused,
            Err(PlatformWalletError::InvalidParameter(message)) if message.contains("exceed")
        ));
        assert_eq!(written.get(), 10, "formatting stopped at the budget");

        let mut budget = DetailsBudget::default();
        let data = Value::Array((0..1_000).map(Value::U64).collect());
        let rendered = budget.render(&data).expect("fits the default budget");
        assert_eq!(budget.used, rendered.len());
    }

    /// Identifiers inside details read as base58, as everywhere else a user
    /// compares them.
    #[test]
    fn identifiers_in_details_render_as_base58() {
        let id = Identifier::from(core::array::from_fn::<u8, 32, _>(|i| i as u8));
        let mut budget = DetailsBudget::default();
        let rendered = budget
            .render(&AuthorizedActionTakers::Identity(id))
            .expect("renders");
        assert_eq!(rendered, format!("Identity(Identifier({id}))"));
    }

    /// A document type name outside the consensus pattern is refused rather
    /// than shown raw in its own field.
    #[test]
    fn invalid_document_type_names_are_refused() {
        for name in ["post\u{202e}", "a\nb", "", &"x".repeat(65)] {
            let transition = batch(vec![BatchedTransition::Document(
                DocumentTransition::Create(DocumentCreateTransition::V0(
                    DocumentCreateTransitionV0 {
                        base: document_base(name),
                        entropy: [0xEE; 32],
                        data: BTreeMap::new(),
                        prefunded_voting_balance: None,
                    },
                )),
            )]);
            assert!(
                summarize_state_transition(&transition.serialize_to_bytes().unwrap()).is_err(),
                "{name:?}"
            );
        }
    }

    /// A public note is quoted, so a newline cannot forge another line and a
    /// NUL cannot break the C projection.
    #[test]
    fn public_notes_are_quoted() {
        let summary = summarize_bytes(&batch(vec![BatchedTransition::Token(
            TokenTransition::Transfer(TokenTransferTransition::V0(TokenTransferTransitionV0 {
                base: token_base(),
                amount: 1,
                recipient_id: Identifier::from(RECIPIENT),
                public_note: Some("hi\nrecipient: someone else\0".to_string()),
                shared_encrypted_note: None,
                private_encrypted_note: None,
            })),
        )]));
        let StateTransitionSummaryKind::Batch { transitions, .. } = summary.kind else {
            panic!("expected a batch");
        };
        let details = transitions[0].details.as_deref().expect("note rendered");
        assert_eq!(details.lines().count(), 1, "{details}");
        assert!(!details.contains('\0'), "{details}");
    }
}
