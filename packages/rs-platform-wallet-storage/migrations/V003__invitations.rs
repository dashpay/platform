//! Add the `invitations` table (DIP-13 DashPay invitations).
//!
//! Inviter-side records of created invitations, powering the "Sent invitations"
//! status list and (future) reclaim of an unclaimed voucher. **No key material
//! is stored** — the one-time voucher key is HD-derived and re-derivable from
//! `funding_index` on demand.
//!
//! All fields map to explicit columns (the entry is all-primitive), so no
//! opaque lifecycle blob is needed — the row reconstructs directly.

pub fn migration() -> String {
    "CREATE TABLE invitations (
        wallet_id BLOB NOT NULL,
        outpoint BLOB NOT NULL,
        status TEXT NOT NULL CHECK (status IN ('created', 'claimed', 'reclaimed')),
        funding_index INTEGER NOT NULL,
        amount_duffs INTEGER NOT NULL,
        expiry_unix INTEGER NOT NULL,
        created_at_secs INTEGER NOT NULL,
        has_inviter INTEGER NOT NULL,
        PRIMARY KEY (wallet_id, outpoint),
        FOREIGN KEY (wallet_id) REFERENCES wallet_metadata(wallet_id) ON DELETE CASCADE
    );"
    .to_string()
}
