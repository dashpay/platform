//! Add the `identity_scan_states` + `identity_scan_failed_indices` tables
//! (verdict of the last gap-limit identity scan).
//!
//! One row per wallet, holding what the last scan probed and what it could
//! not answer. Without it the verdict lived only for the life of the process:
//! a scan that found an identity while one of its probes went unanswered
//! reported success, the next launch saw an identity on file and took the
//! warm-launch shortcut, and the identity at the unanswered index stayed
//! invisible for the life of the installation (dashpay/platform#4365).
//!
//! The entry is all-primitive apart from its list of unanswered indices, so
//! everything maps to explicit columns and no opaque blob is needed (the
//! `dpns_name_states` precedent). The list gets its own child table rather
//! than a packed column: the composite primary key is what makes "ascending,
//! no duplicates" a schema invariant instead of a writer convention.
//!
//! `complete` is stored rather than derived because the two ways a scan ends
//! early differ — unanswered probes leave indices behind, while a scan
//! abandoned at the startup budget leaves none and is no more complete for
//! it. `unlocated_gap` records that second kind, which by definition has no
//! index to name it.
//!
//! Purely additive: an upgraded database gains two empty tables, every wallet
//! reads back "no verdict recorded", and upstream treats that absence as
//! "keep the existing behaviour" rather than "rescan". Nothing is backfilled
//! because nothing could be — no earlier schema held the fact.
//!
//! The intra-row half of the completeness invariant is a CHECK here; the
//! cross-table half (`complete` against a non-empty index list) cannot be,
//! and is enforced by the reader.

pub fn migration() -> String {
    "\
CREATE TABLE identity_scan_states (
    wallet_id BLOB NOT NULL PRIMARY KEY,
    complete INTEGER NOT NULL CHECK (complete IN (0, 1)),
    probed_from INTEGER NOT NULL CHECK (probed_from >= 0),
    probed_through INTEGER NOT NULL CHECK (probed_through >= probed_from),
    unlocated_gap INTEGER NOT NULL CHECK (unlocated_gap IN (0, 1)),
    -- A scan cannot both have answered everything and be sitting on a gap
    -- nobody could name.
    CHECK (complete = 0 OR unlocated_gap = 0),
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
);

-- Indices this wallet's scans probed without getting an answer. Parented on
-- the verdict rather than on `wallets` so clearing a verdict clears its gaps
-- in one statement; the wallet cascade still reaches here transitively.
CREATE TABLE identity_scan_failed_indices (
    wallet_id BLOB NOT NULL,
    failed_index INTEGER NOT NULL CHECK (failed_index >= 0),
    PRIMARY KEY (wallet_id, failed_index),
    FOREIGN KEY (wallet_id) REFERENCES identity_scan_states(wallet_id) ON DELETE CASCADE
);
"
    .to_string()
}
