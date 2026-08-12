//! Network-facing handle — a single façade over the shared
//! `WalletManager` + the Platform SDK.
//!
//! `IdentityWallet<B>` covers the identity lifecycle (register,
//! top-up, transfer, withdraw, update, discovery, DPNS, loading); the
//! DashPay-contract operations that live on the same identity (contact
//! requests, contacts, profile, payments, account labels) are
//! namespaced behind the zero-cost borrowing view
//! [`DashPayView`] reached via `IdentityWallet::dashpay()`.
//!
//! Historically DashPay lived on a separate owned `DashPayWallet<B>`
//! facade, but both views operated on the same underlying state (a
//! single `ManagedIdentity` carries both identity fields and DashPay
//! fields). The two facades were merged to cut handle-juggling at the
//! FFI boundary and so DashPay ops can reuse the same signer /
//! asset-lock plumbing the identity lifecycle already owns; the
//! borrowing view restores the call-site namespace without
//! reintroducing a second handle. `B` picks the transaction
//! broadcaster used by DashPay `send_payment`; it defaults to
//! `SpvBroadcaster` so most call sites don't need to name it.

// Core handle + identity-lifecycle operations.
mod contract;
mod discovery;
mod document;
mod dpns;
mod dpns_marketplace;
mod identity_handle;
mod loading;
mod register_from_addresses;
mod registration;
mod top_up;
mod top_up_from_addresses;
mod transfer;
mod transfer_to_addresses;
mod update;
mod withdrawal;

// DashPay-contract operations, namespaced behind `IdentityWallet::dashpay()`.
mod contact_info;
mod contact_requests;
mod contacts;
mod dashpay_view;
mod invitation;
pub use invitation::{
    Invitation, MAX_INVITATION_DUFFS, MAX_INVITATION_TTL_SECS, MIN_INVITATION_DUFFS,
};
mod payment_handler;
pub(crate) use payment_handler::DashPayPaymentHandler;
// Re-exported for the payments unit tests, which drive the hooks
// directly; the handler itself calls it module-locally.
#[cfg(test)]
pub(crate) use payment_handler::run_dashpay_payment_hooks;
mod payments;
pub(crate) use payments::{
    confirm_sent_dashpay_payment, confirm_sent_dashpay_payment_by_txid,
    record_incoming_dashpay_payments,
};
mod profile;
pub(crate) mod sdk_writer;
mod seed_binding;
pub use seed_binding::SeedBindingVerification;

// Token state-transition operations (same `IdentityWallet` impl blocks).
// Bookkeeping (watch / sync / balance) lives on
// `crate::manager::identity_sync::IdentitySyncManager`.
mod tokens;

pub use contact_info::ContactInfoPublishOutcome;
pub use contact_requests::{
    AutoAcceptProofSource, ContactCryptoProvider, ContactInfoOpened, ContactInfoSealed,
};
pub use dashpay_view::DashPayView;
pub use discovery::IdentityDiscoveryOptions;
pub use dpns::{ContestContender, ContestVoteState, ContestWinner};
pub use dpns_marketplace::{
    DepartedDpnsName, DpnsDomainState, DpnsMarketplaceSyncSummary, DpnsNameHistoryEvent,
    DpnsNameHistoryEventKind, DpnsPriceChange, DOCUMENT_TRANSITION_FEE_RESERVE_CREDITS,
};
pub use identity_handle::{
    derive_ecdsa_identity_auth_keypair_from_master, derive_identity_auth_key_hash_from_master,
    derive_identity_auth_keypair, identity_auth_derivation_path_for_type, DerivedIdentityAuthKey,
    IdentityWallet, IDENTITY_GAP_LIMIT, MASTER_KEY_INDEX,
};

// Helpers declared on `identity_handle.rs` that siblings reach
// through `use super::*;`. Sibling-only — re-exporting here just
// avoids each sibling having to spell out `identity_handle::` on
// every call site.
pub(super) use identity_handle::derive_identity_auth_key_hash;

/// Process-wide cached DashPay data contract.
///
/// The bundled system contract is immutable for a given platform
/// version, so one parse serves every operation — the previous
/// per-call `load_system_data_contract` re-deserialized the contract
/// on every profile / contactInfo op.
pub(crate) fn dashpay_contract(
) -> Result<std::sync::Arc<dpp::prelude::DataContract>, crate::error::PlatformWalletError> {
    static CONTRACT: std::sync::OnceLock<std::sync::Arc<dpp::prelude::DataContract>> =
        std::sync::OnceLock::new();
    if let Some(contract) = CONTRACT.get() {
        return Ok(std::sync::Arc::clone(contract));
    }
    let contract = dpp::system_data_contracts::load_system_data_contract(
        dpp::data_contracts::SystemDataContract::Dashpay,
        dpp::version::PlatformVersion::latest(),
    )
    .map_err(|e| {
        crate::error::PlatformWalletError::InvalidIdentityData(format!(
            "Failed to load DashPay contract: {e}"
        ))
    })?;
    let arc = std::sync::Arc::new(contract);
    // A concurrent first call may have won the race — return whichever
    // Arc actually landed in the cell.
    let _ = CONTRACT.set(std::sync::Arc::clone(&arc));
    Ok(CONTRACT.get().map(std::sync::Arc::clone).unwrap_or(arc))
}
