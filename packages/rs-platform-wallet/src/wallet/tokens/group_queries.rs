//! Group-action discovery for token contracts.
//!
//! These helpers wrap rs-sdk's `GroupActionsQuery` and
//! `GroupActionSignersQuery` so the FFI layer can return typed,
//! Swift-friendly entries instead of raw `dpp` enums. They live here
//! (next to the rest of the token bookkeeping) rather than in
//! rs-sdk-ffi because querying pending proposals only makes sense in
//! the context of a wallet's `Sdk` reference and an identity's role
//! in a contract's groups — see `swift-sdk/CLAUDE.md`'s "high-level
//! operations go through platform-wallet" rule.
//!
//! Exposed as free functions taking `&Sdk` rather than methods on a
//! token wallet: these are read-only network queries that don't
//! touch any wallet bookkeeping (no balance cache, no watch list).
//! Folding them onto a wallet type would needlessly require a
//! wallet handle for an operation that only needs an SDK.

use dpp::balances::credits::TokenAmount;
use dpp::data_contract::{GroupContractPosition, TokenContractPosition};
use dpp::fee::Credits;
use dpp::group::group_action::GroupActionAccessors;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::prelude::Identifier;
use dpp::tokens::emergency_action::TokenEmergencyAction;

use dash_sdk::platform::group_actions::{GroupActionSignersQuery, GroupActionsQuery};
use dash_sdk::platform::FetchMany;

use crate::error::PlatformWalletError;

/// A pending or closed group-action proposal on a token contract,
/// flattened for cross-language consumers.
///
/// `params` carries only the variants Wave 6 ships co-sign UI for.
/// Anything else lives in `Other { name }` so the iOS list still has
/// something readable for proposals it can't replay yet (e.g. claim,
/// transfer, ConfigUpdate). Adding a co-sign entry point for one of
/// those just means promoting it out of `Other` here and lighting up
/// the matching arm in Swift.
#[derive(Debug, Clone)]
pub struct GroupActionEntry {
    pub action_id: Identifier,
    pub proposer: Identifier,
    pub token_contract_position: TokenContractPosition,
    pub status: GroupActionStatus,
    pub params: GroupActionParams,
}

/// Co-sign-relevant parameters extracted from each `TokenEvent`
/// variant. Only the fields a co-signer needs to replay the
/// proposal — encrypted-note blobs are dropped because Platform
/// validates the signature against the proposal's already-stored
/// payload, not against re-supplied notes.
#[derive(Debug, Clone)]
pub enum GroupActionParams {
    Mint {
        amount: TokenAmount,
        recipient: Identifier,
        public_note: Option<String>,
    },
    Burn {
        amount: TokenAmount,
        burn_from: Identifier,
        public_note: Option<String>,
    },
    Freeze {
        target: Identifier,
        public_note: Option<String>,
    },
    Unfreeze {
        target: Identifier,
        public_note: Option<String>,
    },
    DestroyFrozenFunds {
        target: Identifier,
        amount: TokenAmount,
        public_note: Option<String>,
    },
    EmergencyAction {
        action: TokenEmergencyAction,
        public_note: Option<String>,
    },
    SetPrice {
        price_per_token: Option<u64>,
        public_note: Option<String>,
    },
    DirectPurchase {
        amount: TokenAmount,
        total_cost: Credits,
    },
    /// Variants the co-sign UI doesn't replay yet — the `name` field
    /// is the `TokenEvent::associated_document_type_name()` so the
    /// Swift side can still display "Claim" / "Transfer" /
    /// "ConfigUpdate" / etc.
    Other { name: String },
}

/// A single signer's entry on a group-action proposal: the member's
/// identity id and their power in the group.
#[derive(Debug, Clone)]
pub struct GroupActionSignerEntry {
    pub identity_id: Identifier,
    pub power: u32,
}

/// Fetch group-action proposals on `(contract_id, group_position)`
/// filtered by `status`. Returns flat [`GroupActionEntry`] rows
/// rather than rs-sdk's nested `IndexMap<Identifier, Option<GroupAction>>`
/// so the FFI shim doesn't need to special-case the `None`
/// (proof-says-it-existed-but-deleted) case for every field.
pub async fn pending_group_actions_external(
    sdk: &dash_sdk::Sdk,
    contract_id: Identifier,
    group_contract_position: GroupContractPosition,
    status: GroupActionStatus,
    start_at_action_id: Option<(Identifier, bool)>,
    limit: Option<u16>,
) -> Result<Vec<GroupActionEntry>, PlatformWalletError> {
    use dpp::group::action_event::GroupActionEvent;
    use dpp::group::group_action::GroupAction;
    use dpp::tokens::token_event::TokenEvent;

    let query = GroupActionsQuery {
        contract_id,
        group_contract_position,
        status,
        start_at_action_id,
        limit,
    };

    let rows = GroupAction::fetch_many(sdk, query).await.map_err(|e| {
        PlatformWalletError::TokenError(format!("Fetch group actions failed: {}", e))
    })?;

    let mut out = Vec::with_capacity(rows.len());
    for (action_id, maybe_action) in rows {
        let Some(action) = maybe_action else { continue };
        let proposer = action.proposer_id();
        let token_position = action.token_contract_position();
        let GroupActionEvent::TokenEvent(token_event) = action.event().clone();

        let params = match token_event {
            TokenEvent::Mint(amount, recipient, note) => GroupActionParams::Mint {
                amount,
                recipient,
                public_note: note,
            },
            TokenEvent::Burn(amount, burn_from, note) => GroupActionParams::Burn {
                amount,
                burn_from,
                public_note: note,
            },
            TokenEvent::Freeze(target, note) => GroupActionParams::Freeze {
                target,
                public_note: note,
            },
            TokenEvent::Unfreeze(target, note) => GroupActionParams::Unfreeze {
                target,
                public_note: note,
            },
            TokenEvent::DestroyFrozenFunds(target, amount, note) => {
                GroupActionParams::DestroyFrozenFunds {
                    target,
                    amount,
                    public_note: note,
                }
            }
            TokenEvent::EmergencyAction(action, note) => GroupActionParams::EmergencyAction {
                action,
                public_note: note,
            },
            TokenEvent::ChangePriceForDirectPurchase(schedule, note) => {
                let price_per_token = match schedule {
                    Some(
                        dpp::tokens::token_pricing_schedule::TokenPricingSchedule::SinglePrice(p),
                    ) => Some(p),
                    // Tiered schedules aren't surfaced to the simple-form
                    // co-sign UI yet — expose them as Other so the row at
                    // least renders, and a future wave can replay them.
                    Some(_) => {
                        out.push(GroupActionEntry {
                            action_id,
                            proposer,
                            token_contract_position: token_position,
                            status,
                            params: GroupActionParams::Other {
                                name: "directPricingTiered".to_string(),
                            },
                        });
                        continue;
                    }
                    None => None,
                };
                GroupActionParams::SetPrice {
                    price_per_token,
                    public_note: note,
                }
            }
            TokenEvent::DirectPurchase(amount, total_cost) => {
                GroupActionParams::DirectPurchase { amount, total_cost }
            }
            other => GroupActionParams::Other {
                name: other.associated_document_type_name().to_string(),
            },
        };

        out.push(GroupActionEntry {
            action_id,
            proposer,
            token_contract_position: token_position,
            status,
            params,
        });
    }

    Ok(out)
}

/// Fetch the signers (identity id + power) currently signed onto
/// `action_id` on `(contract_id, group_position)`. Status mirrors
/// the [`pending_group_actions_external`] filter — Platform only
/// indexes signers under one status at a time.
pub async fn group_action_signers_external(
    sdk: &dash_sdk::Sdk,
    contract_id: Identifier,
    group_contract_position: GroupContractPosition,
    status: GroupActionStatus,
    action_id: Identifier,
) -> Result<Vec<GroupActionSignerEntry>, PlatformWalletError> {
    use dpp::data_contract::group::GroupMemberPower;

    let query = GroupActionSignersQuery {
        contract_id,
        group_contract_position,
        status,
        action_id,
    };

    let rows = GroupMemberPower::fetch_many(sdk, query)
        .await
        .map_err(|e| {
            PlatformWalletError::TokenError(format!("Fetch group action signers failed: {}", e))
        })?;

    let mut out = Vec::with_capacity(rows.len());
    for (identity_id, maybe_power) in rows {
        let Some(power) = maybe_power else { continue };
        out.push(GroupActionSignerEntry { identity_id, power });
    }
    Ok(out)
}
