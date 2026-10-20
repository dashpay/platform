//! Persist the DashPay coreHeight backfill record and the outbound-account
//! marker as plain columns on the tables they describe.
//!
//! The backfill record is the durable half of the guard that stops the
//! DIP-15 §12.6 rescan from re-lowering the cursor it lowers. Without it the
//! guard lived only in memory while the lowered cursor was durable, so every
//! launch restored a cursor inside the climb, saw no guard, and rewound again
//! (dashpay/platform#4302).
//!
//! - `core_sync_state.dashpay_backfill_floor` / `dashpay_backfill_rewound_from`:
//!   the backfill's extent, beside the `synced_height` it lowered. Both set or
//!   both NULL.
//! - `account_registrations.dashpay_covered_from`: the height a DashPay
//!   receiving account is covered from. A covered entry names exactly one
//!   receiving account (owner, contact, index), so its height is a property of
//!   that account's row; only `dashpay_receiving` rows may carry one.
//! - `contacts.external_account_reference`: the account reference a contact's
//!   outbound account was last built for. A restored marker spares the
//!   cold-start rebuild of that account; `NULL` restores "no marker", which
//!   rebuilds it once — the behaviour every row had before this column existed.
//!
//! Purely additive: every existing row reads back NULL, which upstream treats
//! as "no backfill recorded" and re-runs once — slow, never lossy.

pub fn migration() -> String {
    "\
ALTER TABLE core_sync_state ADD COLUMN dashpay_backfill_floor INTEGER
    CHECK (dashpay_backfill_floor IS NULL
        OR dashpay_backfill_floor BETWEEN 0 AND 4294967295);

ALTER TABLE core_sync_state ADD COLUMN dashpay_backfill_rewound_from INTEGER
    CHECK ((dashpay_backfill_rewound_from IS NULL) = (dashpay_backfill_floor IS NULL)
        AND (dashpay_backfill_rewound_from IS NULL
            OR dashpay_backfill_rewound_from BETWEEN 0 AND 4294967295));

ALTER TABLE account_registrations ADD COLUMN dashpay_covered_from INTEGER
    CHECK (dashpay_covered_from IS NULL
        OR (account_type = 'dashpay_receiving'
            AND dashpay_covered_from BETWEEN 0 AND 4294967295));

ALTER TABLE contacts ADD COLUMN external_account_reference INTEGER
    CHECK (external_account_reference IS NULL
        OR external_account_reference BETWEEN 0 AND 4294967295);
"
    .to_string()
}
