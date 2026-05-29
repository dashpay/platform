//! DashPay payment history models.
//!
//! A [`PaymentEntry`] records a single DashPay-routed Dash payment —
//! either sent to or received from a contact identity. Entries live
//! on the owning [`ManagedIdentity`](crate::wallet::identity::ManagedIdentity)
//! as a `BTreeMap<String, PaymentEntry>` keyed by transaction id,
//! and are emitted through [`IdentityEntry`](crate::changeset::IdentityEntry)
//! so the persister can round-trip them.
//!
//! The model mirrors what evo-tool's `dashpay_payments` table stores
//! `(tx_id, from_identity_id, to_identity_id, amount, memo,
//! payment_type, status, created_at, confirmed_at)` — but in
//! platform-wallet-native shape: the counterparty is a single
//! `counterparty_id` field (the caller's direction tells us whether
//! the owner is the sender or the receiver), and direction / status
//! are strongly-typed enums instead of free-form strings.

use dpp::prelude::Identifier;

/// Direction of a DashPay payment, from the owner's point of view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PaymentDirection {
    /// The owner sent this payment to the counterparty.
    Sent,
    /// The owner received this payment from the counterparty.
    Received,
}

/// Status of a DashPay payment on Core chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PaymentStatus {
    /// Broadcast but not yet confirmed.
    #[default]
    Pending,
    /// Confirmed on Core chain.
    Confirmed,
    /// Broadcast failed or the transaction was dropped.
    Failed,
}

/// Match result from
/// [`IdentityWallet::match_incoming_dashpay_address`](crate::wallet::identity::IdentityWallet).
///
/// Returned when an on-chain address matches one of the DashPay
/// contact receival accounts registered in this wallet's
/// [`ManagedAccountCollection`]. Lets the SPV / backend task layer
/// classify an observed transaction output as a DashPay incoming
/// payment from a specific contact, at a specific BIP44 index,
/// without needing to maintain a separate reverse-lookup table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DashpayAddressMatch {
    /// Our (owner) identity ID — the recipient of the payment.
    pub user_identity_id: Identifier,
    /// The sending contact's identity ID.
    pub friend_identity_id: Identifier,
    /// Address index within the account's external pool.
    pub address_index: u32,
}

/// A single DashPay payment entry recorded on a
/// [`ManagedIdentity`](crate::wallet::identity::ManagedIdentity).
///
/// Keyed by transaction id (hex string, matching evo-tool's
/// `dashpay_payments.tx_id` column which is `TEXT UNIQUE NOT NULL`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PaymentEntry {
    /// The other identity in this payment. Whether they're the
    /// sender or receiver is encoded in `direction`.
    pub counterparty_id: Identifier,
    /// Amount in duffs. Always positive; `direction` tells us
    /// whether it was outgoing or incoming.
    pub amount_duffs: u64,
    /// Optional memo attached by the sender.
    pub memo: Option<String>,
    /// Direction from the owner's perspective.
    pub direction: PaymentDirection,
    /// Current status on Core chain.
    pub status: PaymentStatus,
}

impl PaymentEntry {
    /// Create a new sent-payment entry with status
    /// [`PaymentStatus::Pending`].
    pub fn new_sent(counterparty_id: Identifier, amount_duffs: u64, memo: Option<String>) -> Self {
        Self {
            counterparty_id,
            amount_duffs,
            memo,
            direction: PaymentDirection::Sent,
            status: PaymentStatus::Pending,
        }
    }

    /// Create a new received-payment entry with status
    /// [`PaymentStatus::Confirmed`] (incoming payments are only
    /// recorded once SPV sees the transaction on-chain).
    pub fn new_received(
        counterparty_id: Identifier,
        amount_duffs: u64,
        memo: Option<String>,
    ) -> Self {
        Self {
            counterparty_id,
            amount_duffs,
            memo,
            direction: PaymentDirection::Received,
            status: PaymentStatus::Confirmed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sent_starts_pending() {
        let entry =
            PaymentEntry::new_sent(Identifier::from([1u8; 32]), 12_000, Some("lunch".into()));
        assert_eq!(entry.direction, PaymentDirection::Sent);
        assert_eq!(entry.status, PaymentStatus::Pending);
        assert_eq!(entry.amount_duffs, 12_000);
        assert_eq!(entry.memo.as_deref(), Some("lunch"));
    }

    #[test]
    fn new_received_starts_confirmed() {
        let entry = PaymentEntry::new_received(Identifier::from([2u8; 32]), 7_500, None);
        assert_eq!(entry.direction, PaymentDirection::Received);
        assert_eq!(entry.status, PaymentStatus::Confirmed);
        assert_eq!(entry.amount_duffs, 7_500);
        assert!(entry.memo.is_none());
    }
}
