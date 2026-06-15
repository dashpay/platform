//! DashPay sync-correctness schema additions.
//!
//! **Append-only — these MUST NOT be folded back into V001.** V001 shipped
//! in `v4.0.0-beta.4` / `rc.1` / `rc.2`, and refinery records a checksum of
//! every applied migration. Editing V001 in place would break the upgrade
//! for any database that already ran it (checksum mismatch on open, or the
//! new DDL silently skipped because V001 is already marked applied). So the
//! `payment_channel_broken` column and the `rejected_contact_requests`
//! table — both added after V001 was released — live here.
//!
//! - `contacts.payment_channel_broken` (G1c): set when external-account
//!   registration *permanently* fails for a contact, so the sync sweep
//!   stops retrying a poisoned channel; cleared when a superseding rotation
//!   re-establishes the contact.
//! - `rejected_contact_requests` (G5 stage 1): persisted rejection
//!   tombstones so a rejected sender's still-on-platform immutable
//!   `contactRequest` isn't re-ingested (and the contact resurrected) on the
//!   next sync sweep. Keyed by `(wallet_id, owner_id, sender_id,
//!   account_reference)` — NOT bare sender id — so a once-rejected sender
//!   can still re-request via a bumped `accountReference` (DIP-15 rotation),
//!   while a replay of the exact same immutable request stays suppressed.

pub fn migration() -> String {
    // `ALTER TABLE … ADD COLUMN` is the only schema change SQLite supports
    // in place; the new column is nullable (no default needed — readers
    // treat NULL as `false`). The tombstone table mirrors the
    // SwiftData/`ManagedIdentity.rejected_contact_requests` shape.
    String::from(
        "\
ALTER TABLE contacts ADD COLUMN payment_channel_broken INTEGER;

CREATE TABLE rejected_contact_requests (
    wallet_id BLOB NOT NULL,
    owner_id BLOB NOT NULL,
    sender_id BLOB NOT NULL,
    account_reference INTEGER NOT NULL,
    document_id BLOB,
    rejected_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (wallet_id, owner_id, sender_id, account_reference),
    FOREIGN KEY (wallet_id) REFERENCES wallet_metadata(wallet_id) ON DELETE CASCADE
);
",
    )
}
