//! Add the `dashpay_backfill` table (durable record of the DashPay coreHeight
//! backfill) and the `contacts.external_account_reference` column
//! (outbound-account marker).
//!
//! The backfill record is the durable half of the guard that stops the
//! DIP-15 §12.6 rescan from re-lowering the cursor it lowers. Without it the
//! guard lived only in memory while the lowered cursor was durable, so every
//! launch restored a cursor inside the climb, saw no guard, and rewound again
//! (dashpay/platform#4302).
//!
//! One row per wallet holds the backfill's extent and the receival accounts it
//! covers. The record is only ever read and written whole, so the cover set is
//! one blob in `DashPayBackfillRecord::covered_bytes` form — the layout the FFI
//! hosts store — rather than a child table: decoding it canonicalizes order and
//! duplicates, and a blob that is not a whole number of entries reads as "no
//! record", which only costs a re-run.
//!
//! The outbound-account marker is the account reference a contact's outbound
//! account was last built for. A restored marker spares the cold-start rebuild
//! of that account; `NULL` restores "no marker", which rebuilds it once — the
//! behaviour every row had before this column existed.
//!
//! Purely additive: an upgraded database gains an empty table and a `NULL`
//! column, every wallet reads back "no backfill recorded", and upstream treats
//! that absence as "re-run the backfill" — slow, never lossy. Nothing is
//! backfilled because no earlier schema held either fact.

pub fn migration() -> String {
    "\
CREATE TABLE dashpay_backfill (
    wallet_id BLOB NOT NULL PRIMARY KEY,
    floor INTEGER NOT NULL CHECK (floor BETWEEN 0 AND 4294967295),
    rewound_from INTEGER NOT NULL CHECK (rewound_from BETWEEN 0 AND 4294967295),
    -- `DashPayBackfillRecord::covered_bytes`: 72 bytes per covered receival
    -- account (owner id, contact id, account index LE, covered_from LE).
    covered BLOB NOT NULL CHECK (length(covered) > 0 AND length(covered) % 72 = 0),
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
);

ALTER TABLE contacts ADD COLUMN external_account_reference INTEGER
    CHECK (external_account_reference IS NULL
        OR external_account_reference BETWEEN 0 AND 4294967295);
"
    .to_string()
}
