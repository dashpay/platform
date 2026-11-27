//! The Platform addresses an external Platform-address funding was built to
//! pay, so a resume after restart pays them or is refused instead of
//! refunding the sender. Stored as the wallet's opaque
//! `PlatformAddressRecipients::to_bytes` form. Legacy blobs stay
//! byte-identical; old rows receive NULL: an owned funding, or an external
//! payment tracked before this column, whose payees only the host recorded.
pub fn migration() -> String {
    "ALTER TABLE asset_locks ADD COLUMN platform_address_recipients BLOB \
     CHECK (platform_address_recipients IS NULL OR (typeof(platform_address_recipients) = 'blob' AND length(platform_address_recipients) BETWEEN 1 AND 4096)); \
     CREATE TRIGGER asset_locks_platform_address_recipients_immutable \
     BEFORE UPDATE OF platform_address_recipients ON asset_locks \
     WHEN OLD.platform_address_recipients IS NOT NULL AND NEW.platform_address_recipients IS NOT OLD.platform_address_recipients \
     BEGIN SELECT RAISE(ABORT, 'asset lock platform address recipients cannot change'); END;".to_owned()
}
