//! Retire identity tombstoning: a removed identity is deleted outright.
//!
//! The `tombstoned` flag kept a logically-deleted row on disk so its
//! dependents were not wiped, at the cost of a permanent divergence: the
//! in-memory `IdentityManager` drops the whole `ManagedIdentity` on
//! removal, so a re-added identity was empty in memory while the next
//! `load()` handed it the removed one's keys back.
//!
//! Deleting the row instead needs a broom for the dependents no foreign
//! key reaches:
//!
//! - `identity_keys`' FK to `identities` is compound
//!   (`wallet_id, identity_id`), and SQLite's MATCH SIMPLE skips FK
//!   enforcement entirely once ANY child key column is NULL — so for an
//!   out-of-wallet identity (`wallet_id IS NULL` on both sides) the
//!   cascade is dormant and its keys would survive the delete.
//! - `contacts`, `ignored_senders`, and `pending_contact_crypto` carry
//!   identity owners but no FK to `identities`. Orphan contacts fail a
//!   strict load; orphan ignored-sender rows are omitted by the loader,
//!   and the deferred-crypto queue has no production reader.
//!
//! `token_balances`, `dashpay_profiles` and `dashpay_payments_overlay`
//! need nothing new: their FK column is `identity_id NOT NULL`, so it is
//! never dormant. `meta_identity` / `meta_token` keep riding V001's
//! `cascade_meta_on_identity_delete`, which this migration leaves alone.
//! V017's `identity_scan_states` / `identity_scan_failed_indices` are
//! wallet-scoped (FK to `wallets`, no `identity_id`), so a scan verdict
//! outliving one identity is the intended reading: it records how far the
//! wallet's index space was probed, not which identities came back.

pub fn migration() -> String {
    "\
CREATE TRIGGER cascade_children_on_identity_delete
AFTER DELETE ON identities
FOR EACH ROW
BEGIN
    DELETE FROM identity_keys   WHERE identity_id = OLD.identity_id;
    DELETE FROM contacts        WHERE owner_id    = OLD.identity_id;
    DELETE FROM ignored_senders WHERE owner_id    = OLD.identity_id;
    DELETE FROM pending_contact_crypto WHERE owner_identity_id = OLD.identity_id;
END;

-- Owner columns follow wallet_id in these primary keys. Owner-leading
-- indexes avoid scanning each child table once per cascaded identity.
CREATE INDEX idx_contacts_owner ON contacts(owner_id);
CREATE INDEX idx_ignored_senders_owner ON ignored_senders(owner_id);
CREATE INDEX idx_pending_contact_crypto_owner ON pending_contact_crypto(owner_identity_id);

-- Purge what earlier schemas only flagged. Spelled out per table rather
-- than left to the cascade and the trigger above, so the outcome does
-- not depend on the migrating connection's `foreign_keys` pragma.
DELETE FROM identity_keys
    WHERE identity_id IN (SELECT identity_id FROM identities WHERE tombstoned = 1);
DELETE FROM contacts
    WHERE owner_id IN (SELECT identity_id FROM identities WHERE tombstoned = 1);
DELETE FROM ignored_senders
    WHERE owner_id IN (SELECT identity_id FROM identities WHERE tombstoned = 1);
DELETE FROM pending_contact_crypto
    WHERE owner_identity_id IN (SELECT identity_id FROM identities WHERE tombstoned = 1);
DELETE FROM token_balances
    WHERE identity_id IN (SELECT identity_id FROM identities WHERE tombstoned = 1);
DELETE FROM dashpay_profiles
    WHERE identity_id IN (SELECT identity_id FROM identities WHERE tombstoned = 1);
DELETE FROM dashpay_payments_overlay
    WHERE identity_id IN (SELECT identity_id FROM identities WHERE tombstoned = 1);
DELETE FROM meta_identity
    WHERE identity_id IN (SELECT identity_id FROM identities WHERE tombstoned = 1);
DELETE FROM meta_token
    WHERE identity_id IN (SELECT identity_id FROM identities WHERE tombstoned = 1);
DELETE FROM identities WHERE tombstoned = 1;

ALTER TABLE identities DROP COLUMN tombstoned;
"
    .to_string()
}
