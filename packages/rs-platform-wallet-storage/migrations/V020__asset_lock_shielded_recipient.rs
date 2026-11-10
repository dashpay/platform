//! The Orchard recipient a shielded asset-lock funding was built to pay, so a
//! resume after restart pays the recipient the user confirmed. Legacy blobs
//! stay byte-identical; old rows receive NULL and keep accepting the
//! resuming caller's recipient.
pub fn migration() -> String {
    "ALTER TABLE asset_locks ADD COLUMN shielded_recipient BLOB \
     CHECK (shielded_recipient IS NULL OR (typeof(shielded_recipient) = 'blob' AND length(shielded_recipient) = 43)); \
     CREATE TRIGGER asset_locks_shielded_recipient_immutable \
     BEFORE UPDATE OF shielded_recipient ON asset_locks \
     WHEN OLD.shielded_recipient IS NOT NULL AND NEW.shielded_recipient IS NOT OLD.shielded_recipient \
     BEGIN SELECT RAISE(ABORT, 'asset lock shielded recipient cannot change'); END;".to_owned()
}
