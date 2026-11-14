//! What an asset lock pays when that is not the wallet itself (the recipients
//! of an external Platform-address payment), so a resume after restart pays
//! them or is refused instead of refunding the sender. Stored as the wallet's
//! opaque `AssetLockPaymentIntent::to_bytes` form. Legacy blobs stay
//! byte-identical; old rows receive NULL and resume as before.
pub fn migration() -> String {
    "ALTER TABLE asset_locks ADD COLUMN payment_intent BLOB \
     CHECK (payment_intent IS NULL OR (typeof(payment_intent) = 'blob' AND length(payment_intent) BETWEEN 1 AND 4096)); \
     CREATE TRIGGER asset_locks_payment_intent_immutable \
     BEFORE UPDATE OF payment_intent ON asset_locks \
     WHEN OLD.payment_intent IS NOT NULL AND NEW.payment_intent IS NOT OLD.payment_intent \
     BEGIN SELECT RAISE(ABORT, 'asset lock payment intent cannot change'); END;".to_owned()
}
