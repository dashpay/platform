//! The digest an outbound DashPay account records of the contact request its
//! xpub was decrypted from. It sits beside the xpub on the account's row so the
//! two are always written together; rows written before it exist get NULL and
//! are verified by decrypting, as before.
pub fn migration() -> String {
    "ALTER TABLE account_registrations ADD COLUMN encrypted_xpub_digest BLOB \
     CHECK (encrypted_xpub_digest IS NULL OR \
            (typeof(encrypted_xpub_digest) = 'blob' AND length(encrypted_xpub_digest) = 32));"
        .to_owned()
}
