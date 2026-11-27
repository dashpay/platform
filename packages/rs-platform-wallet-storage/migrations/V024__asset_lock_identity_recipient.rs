//! The identity an unbound identity top-up was built (or later assigned) to
//! pay, so a resume after restart pays that identity and refuses another.
//! Legacy blobs stay byte-identical; old rows receive NULL.
pub fn migration() -> String {
    "ALTER TABLE asset_locks ADD COLUMN identity_recipient BLOB \
     CHECK (identity_recipient IS NULL OR (typeof(identity_recipient) = 'blob' AND length(identity_recipient) = 32)); \
     CREATE TRIGGER asset_locks_identity_recipient_immutable \
     BEFORE UPDATE OF identity_recipient ON asset_locks \
     WHEN OLD.identity_recipient IS NOT NULL AND NEW.identity_recipient IS NOT OLD.identity_recipient \
     BEGIN SELECT RAISE(ABORT, 'asset lock identity recipient cannot change'); END;".to_owned()
}
