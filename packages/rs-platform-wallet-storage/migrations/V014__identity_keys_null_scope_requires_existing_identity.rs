//! Close the asymmetry in the `identity_keys` NULL-scope guard.
//!
//! V001's trigger pair aborts a NULL-scoped key whose identity is
//! wallet-owned, but accepts one naming an identity that does not exist at
//! all — `EXISTS(... AND wallet_id IS NOT NULL)` is false for a missing row
//! just as it is for an unowned one. SQLite's MATCH SIMPLE leaves both of
//! the table's foreign keys dormant whenever any child-key column is NULL,
//! so nothing else constrains a NULL-scoped row either: the triggers are
//! the whole guard, and that gap admits a key belonging to no identity.
//!
//! The condition is inverted to require the named identity to exist AND be
//! unowned, which covers the wallet-owned case the original caught and the
//! missing-identity case it did not.
//!
//! Recreated rather than edited into V001: refinery never re-runs an
//! applied migration, so editing V001 would tighten only freshly created
//! databases and leave every existing wallet on the permissive trigger.
//! `V007__drop_core_utxo_metadata` sets the same precedent.
//!
//! The UPDATE twin is as load-bearing as the INSERT one — the writer's
//! upsert resolves an existing key to `DO UPDATE`, and an UPDATE never
//! fires a BEFORE INSERT trigger — so both move together.

pub fn migration() -> String {
    "\
DROP TRIGGER identity_keys_null_scope_requires_unowned_identity;
DROP TRIGGER identity_keys_null_scope_requires_unowned_identity_on_update;

CREATE TRIGGER identity_keys_null_scope_requires_unowned_identity
BEFORE INSERT ON identity_keys
FOR EACH ROW WHEN NEW.wallet_id IS NULL
BEGIN
    SELECT RAISE(ABORT, 'identity_keys.wallet_id is NULL but the named identity is missing or wallet-owned')
    WHERE NOT EXISTS (
        SELECT 1 FROM identities i
        WHERE i.identity_id = NEW.identity_id AND i.wallet_id IS NULL
    );
END;

CREATE TRIGGER identity_keys_null_scope_requires_unowned_identity_on_update
BEFORE UPDATE ON identity_keys
FOR EACH ROW WHEN NEW.wallet_id IS NULL
BEGIN
    SELECT RAISE(ABORT, 'identity_keys.wallet_id is NULL but the named identity is missing or wallet-owned')
    WHERE NOT EXISTS (
        SELECT 1 FROM identities i
        WHERE i.identity_id = NEW.identity_id AND i.wallet_id IS NULL
    );
END;
"
    .to_string()
}
