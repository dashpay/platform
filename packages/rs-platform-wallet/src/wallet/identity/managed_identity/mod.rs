//! Managed identity that combines a Platform Identity with wallet-specific metadata
//!
//! This module provides the `ManagedIdentity` struct which wraps a Platform Identity
//! with additional metadata for wallet management.

mod block_time;
mod contact_requests;
mod contacts;
mod identity_ops;
pub mod key_storage;
mod label;
mod sync;
mod tests;

pub use block_time::BlockTime;
pub use key_storage::{DpnsNameInfo, IdentityStatus, KeyStorage, PrivateKeyData, WatchedIdentity};

use crate::wallet::dashpay::{ContactRequest, DashPayProfile, EstablishedContact, PaymentEntry};
use dpp::identity::Identity;
use dpp::prelude::Identifier;
use std::collections::BTreeMap;

/// A managed identity that combines an Identity with wallet-specific metadata
#[derive(Debug, Clone)]
pub struct ManagedIdentity {
    /// The Platform identity
    pub identity: Identity,

    /// The BIP-9 HD identity index used during registration or discovery.
    ///
    /// This is the index in the derivation path `m/9'/coin'/5'/0'/key_type'/identity_index'/key_id'`.
    /// Recorded during identity registration or gap-limit discovery so that
    /// subsequent operations (signing, ECDH) can derive the correct keys.
    pub identity_index: u32,

    /// Last block time when balance was updated for this identity
    pub last_updated_balance_block_time: Option<BlockTime>,

    /// Last block time when keys were synced for this identity
    pub last_synced_keys_block_time: Option<BlockTime>,

    /// User-defined label for this identity
    pub label: Option<String>,

    /// Map of established contacts (bidirectional relationships) keyed by contact identity ID
    pub established_contacts: BTreeMap<Identifier, EstablishedContact>,

    /// Map of sent contact requests (outgoing, not yet reciprocated) keyed by recipient ID
    pub sent_contact_requests: BTreeMap<Identifier, ContactRequest>,

    /// Map of incoming contact requests (not yet accepted) keyed by sender ID
    pub incoming_contact_requests: BTreeMap<Identifier, ContactRequest>,

    /// Private key storage mapping KeyID to (public key, private key data).
    pub key_storage: KeyStorage,

    /// Identity lifecycle status on Platform.
    pub status: IdentityStatus,

    /// DPNS usernames associated with this identity.
    pub dpns_names: Vec<DpnsNameInfo>,

    /// Wallet identifier (`SHA256(root_pub_key || chain_code)`) of
    /// the wallet that owns this identity, if known. Set during
    /// gap-limit scan and identity recovery.
    pub wallet_id: Option<[u8; 32]>,

    /// Top-up history: maps top-up index to amount (in duffs).
    pub top_ups: BTreeMap<u32, u64>,

    /// DashPay profile (display name, bio, avatar, public message)
    /// published via the DashPay data contract. `None` until the
    /// profile has been fetched or set.
    pub dashpay_profile: Option<DashPayProfile>,

    /// DashPay payment history keyed by transaction id (hex string).
    /// Each entry records a single Dash payment to or from a contact
    /// identity, with direction, amount, memo, and status.
    pub dashpay_payments: BTreeMap<String, PaymentEntry>,
}
