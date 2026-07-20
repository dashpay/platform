//! Established contact between identities in DashPay
//!
//! This module provides the `EstablishedContact` struct representing a bidirectional
//! relationship (friendship) between two identities where both have sent contact requests.

use super::contact_request::ContactRequest;
use dpp::prelude::Identifier;

/// An established contact represents a bidirectional relationship between two identities
///
/// This is formed when both identities have sent contact requests to each other.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EstablishedContact {
    /// The contact's identity unique identifier
    pub contact_identity_id: Identifier,

    /// The outgoing contact request (from us to them)
    pub outgoing_request: ContactRequest,

    /// The incoming contact request (from them to us)
    pub incoming_request: ContactRequest,

    /// Optional alias/nickname for this contact
    pub alias: Option<String>,

    /// Optional note about this contact
    pub note: Option<String>,

    /// Whether this contact is hidden from the contact list
    pub is_hidden: bool,

    /// List of accepted account references beyond the default
    pub accepted_accounts: Vec<u32>,

    /// Whether this contact's payment channel is **permanently** broken.
    ///
    /// Set by the account-building sweep when registering the
    /// counterparty's external sending account fails for a *permanent*
    /// reason — a decrypt/decode failure of the encrypted xpub, or an
    /// identity-key shape that can never satisfy the ECDH gate. A
    /// transient failure (network) leaves this `false` so the next sweep
    /// retries. Once `true`, the sweep skips this contact until the
    /// underlying request changes, and the FFI/UI surfaces "payment
    /// channel broken — ask the contact to send a new request" instead
    /// of an unbounded retry loop.
    ///
    /// Defaults to `false`; a freshly established contact is never broken.
    #[cfg_attr(feature = "serde", serde(default))]
    pub payment_channel_broken: bool,

    /// The contact's decrypted DIP-15 `encryptedAccountLabel` — the
    /// human-readable label the contact attached to the receiving account
    /// they shared (a payment-routing hint, e.g. "Main wallet"). Derived
    /// during the external-account build by decrypting
    /// **`incoming_request`**'s `encrypted_account_label` with the ECDH
    /// shared key — never the outgoing request, which carries a label *we*
    /// chose. `None` when the contact sent no label or it could not be
    /// decrypted to printable text.
    ///
    /// Cosmetic: a decrypt failure never breaks the payment channel. Reset
    /// to `None` on (re-)establish and on rotation so it is always
    /// re-derived from the current incoming request rather than going
    /// stale against new key material.
    #[cfg_attr(feature = "serde", serde(default))]
    pub contact_account_label: Option<String>,

    /// The `incoming_request.account_reference` the currently-registered
    /// `DashpayExternalAccount` (our outbound sending account for this
    /// contact) was built from, or `None` when no external account is known
    /// to be built for the current reference.
    ///
    /// Load-bearing for **rotation self-heal across restart**. When the
    /// contact rotates their receiving xpub the sweep tears down the stale
    /// external account and re-registers from the new xpub — but if the
    /// rebuild is deferred (no signer) and the app restarts before it drains,
    /// the persisted account-registration row (an upsert with no tombstone)
    /// rebuilds the STALE xpub while `incoming_request` carries the new
    /// reference. The account-build sweep would then skip the contact (the
    /// account exists) forever, sending to addresses the contact no longer
    /// watches. Comparing this marker against `incoming_request.account_reference`
    /// lets the sweep detect a stale account and tear it down + rebuild.
    ///
    /// Reset to `None` on (re-)establish and on rotation (the account must be
    /// rebuilt from the new key material). A cold restart that does not
    /// restore this field leaves it `None`, which conservatively forces one
    /// rebuild on the next sweep — self-healing either way.
    #[cfg_attr(feature = "serde", serde(default))]
    pub external_account_reference: Option<u32>,
}

impl EstablishedContact {
    /// Create a new established contact from bidirectional contact requests
    pub fn new(
        contact_identity_id: Identifier,
        outgoing_request: ContactRequest,
        incoming_request: ContactRequest,
    ) -> Self {
        Self {
            contact_identity_id,
            outgoing_request,
            incoming_request,
            alias: None,
            note: None,
            is_hidden: false,
            accepted_accounts: Vec::new(),
            payment_channel_broken: false,
            contact_account_label: None,
            external_account_reference: None,
        }
    }

    /// (Re-)establish a contact while **preserving** any user metadata
    /// already attached to a prior `EstablishedContact` for the same pair.
    ///
    /// [`EstablishedContact::new`] resets `alias` / `note` / `is_hidden` /
    /// `accepted_accounts` / `payment_channel_broken` to their defaults —
    /// so a naive re-establish on every recurring sweep (the sent-side
    /// reconcile, or a re-ingested reciprocal) would wipe the user's alias,
    /// note, hide flag, and accepted-accounts list each pass. This
    /// constructor refreshes the two underlying [`ContactRequest`]s (the
    /// authoritative on-platform documents may have been re-fetched) but
    /// carries the metadata forward from `existing`.
    ///
    /// `payment_channel_broken` is **reset to `false`** on re-establish:
    /// re-establishment means a fresh request flowed in, which is exactly
    /// the "underlying request changed" condition under which the broken
    /// flag should clear so the account-building sweep retries.
    pub fn reestablish_preserving_metadata(
        existing: &EstablishedContact,
        outgoing_request: ContactRequest,
        incoming_request: ContactRequest,
    ) -> Self {
        Self {
            contact_identity_id: existing.contact_identity_id,
            outgoing_request,
            incoming_request,
            alias: existing.alias.clone(),
            note: existing.note.clone(),
            is_hidden: existing.is_hidden,
            accepted_accounts: existing.accepted_accounts.clone(),
            // A new request superseded the old relationship — clear the
            // broken flag so the sweep gives the rebuilt channel a chance.
            payment_channel_broken: false,
            // The label is a property of the (possibly new) incoming
            // request, not user metadata — drop it so it is re-derived
            // from the fresh request rather than carried over stale.
            contact_account_label: None,
            // The external account must be rebuilt from the (possibly new)
            // incoming request, so the marker resets — the sweep re-registers
            // and re-stamps it.
            external_account_reference: None,
        }
    }

    /// Set the alias for this contact
    pub fn set_alias(&mut self, alias: String) {
        self.alias = Some(alias);
    }

    /// Clear the alias for this contact
    pub fn clear_alias(&mut self) {
        self.alias = None;
    }

    /// Set a note for this contact
    pub fn set_note(&mut self, note: String) {
        self.note = Some(note);
    }

    /// Clear the note for this contact
    pub fn clear_note(&mut self) {
        self.note = None;
    }

    /// Hide this contact from the contact list
    pub fn hide(&mut self) {
        self.is_hidden = true;
    }

    /// Unhide this contact
    pub fn unhide(&mut self) {
        self.is_hidden = false;
    }

    /// Add an accepted account reference
    pub fn add_accepted_account(&mut self, account_reference: u32) {
        if !self.accepted_accounts.contains(&account_reference) {
            self.accepted_accounts.push(account_reference);
        }
    }

    /// Remove an accepted account reference
    pub fn remove_accepted_account(&mut self, account_reference: u32) {
        self.accepted_accounts.retain(|&a| a != account_reference);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_outgoing_request() -> ContactRequest {
        ContactRequest::new(
            Identifier::from([1u8; 32]), // sender (us)
            Identifier::from([2u8; 32]), // recipient (them)
            0,
            0,
            0,
            vec![0u8; 96],
            100000,
            1234567890,
        )
    }

    fn create_test_incoming_request() -> ContactRequest {
        ContactRequest::new(
            Identifier::from([2u8; 32]), // sender (them)
            Identifier::from([1u8; 32]), // recipient (us)
            0,
            0,
            0,
            vec![0u8; 96],
            100000,
            1234567891,
        )
    }

    #[test]
    fn test_established_contact_creation() {
        let contact = EstablishedContact::new(
            Identifier::from([2u8; 32]),
            create_test_outgoing_request(),
            create_test_incoming_request(),
        );

        assert_eq!(contact.contact_identity_id, Identifier::from([2u8; 32]));
        assert_eq!(contact.alias, None);
        assert_eq!(contact.note, None);
        assert_eq!(contact.is_hidden, false);
        assert_eq!(contact.accepted_accounts.len(), 0);
        // A freshly established contact is never broken.
        assert_eq!(contact.payment_channel_broken, false);
    }

    /// `reestablish_preserving_metadata` must carry alias/note/is_hidden/
    /// accepted_accounts forward from the prior contact — the sweep
    /// re-establishes on every pass, and `EstablishedContact::new` would
    /// wipe the user's metadata each time. This pins that the
    /// metadata-preserving path does NOT reset it.
    #[test]
    fn test_reestablish_preserves_user_metadata() {
        let mut existing = EstablishedContact::new(
            Identifier::from([2u8; 32]),
            create_test_outgoing_request(),
            create_test_incoming_request(),
        );
        existing.set_alias("Best Friend".to_string());
        existing.set_note("Met at conference".to_string());
        existing.hide();
        existing.add_accepted_account(7);
        existing.payment_channel_broken = true;
        existing.contact_account_label = Some("Stale label".to_string());

        // Re-establish with fresh request docs (newer timestamps).
        let mut newer_outgoing = create_test_outgoing_request();
        newer_outgoing.created_at = 2_000_000_000;
        let mut newer_incoming = create_test_incoming_request();
        newer_incoming.created_at = 2_000_000_001;

        let reestablished = EstablishedContact::reestablish_preserving_metadata(
            &existing,
            newer_outgoing.clone(),
            newer_incoming.clone(),
        );

        // Metadata survives.
        assert_eq!(reestablished.alias, Some("Best Friend".to_string()));
        assert_eq!(reestablished.note, Some("Met at conference".to_string()));
        assert_eq!(reestablished.is_hidden, true);
        assert_eq!(reestablished.accepted_accounts, vec![7]);
        // Fresh requests are adopted.
        assert_eq!(reestablished.outgoing_request.created_at, 2_000_000_000);
        assert_eq!(reestablished.incoming_request.created_at, 2_000_000_001);
        // A superseding request clears the broken flag so the sweep retries.
        assert_eq!(reestablished.payment_channel_broken, false);
        // The label is re-derived from the fresh incoming request, not
        // carried over (it is a property of the request, not user metadata).
        assert_eq!(reestablished.contact_account_label, None);
    }

    #[test]
    fn test_alias_management() {
        let mut contact = EstablishedContact::new(
            Identifier::from([2u8; 32]),
            create_test_outgoing_request(),
            create_test_incoming_request(),
        );

        contact.set_alias("Best Friend".to_string());
        assert_eq!(contact.alias, Some("Best Friend".to_string()));

        contact.clear_alias();
        assert_eq!(contact.alias, None);
    }

    #[test]
    fn test_note_management() {
        let mut contact = EstablishedContact::new(
            Identifier::from([2u8; 32]),
            create_test_outgoing_request(),
            create_test_incoming_request(),
        );

        contact.set_note("Met at conference".to_string());
        assert_eq!(contact.note, Some("Met at conference".to_string()));

        contact.clear_note();
        assert_eq!(contact.note, None);
    }

    #[test]
    fn test_hide_unhide() {
        let mut contact = EstablishedContact::new(
            Identifier::from([2u8; 32]),
            create_test_outgoing_request(),
            create_test_incoming_request(),
        );

        assert_eq!(contact.is_hidden, false);

        contact.hide();
        assert_eq!(contact.is_hidden, true);

        contact.unhide();
        assert_eq!(contact.is_hidden, false);
    }

    #[test]
    fn test_accepted_accounts() {
        let mut contact = EstablishedContact::new(
            Identifier::from([2u8; 32]),
            create_test_outgoing_request(),
            create_test_incoming_request(),
        );

        // Add accounts
        contact.add_accepted_account(1);
        contact.add_accepted_account(2);
        assert_eq!(contact.accepted_accounts.len(), 2);
        assert!(contact.accepted_accounts.contains(&1));
        assert!(contact.accepted_accounts.contains(&2));

        // Adding duplicate should not increase count
        contact.add_accepted_account(1);
        assert_eq!(contact.accepted_accounts.len(), 2);

        // Remove account
        contact.remove_accepted_account(1);
        assert_eq!(contact.accepted_accounts.len(), 1);
        assert!(!contact.accepted_accounts.contains(&1));
        assert!(contact.accepted_accounts.contains(&2));
    }
}
