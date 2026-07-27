//! Add the balance height pin to `platform_addresses`.
//!
//! `as_of_height` mirrors `AddressFunds::as_of_height` (see `dash-sdk`'s
//! `address_sync` module): the Platform block height a persisted balance
//! is current **as of**. It is the reconciliation rule between
//! proof-attested absolutes and the recent/compacted balance-change
//! delta stream — a delta recorded at or below the pin is already
//! included in the absolute and must not be re-applied (the ADDR-09
//! double-count).
//!
//! `DEFAULT 0` means "unknown provenance" for rows persisted before the
//! pin existed: every delta applies and any pinned absolute supersedes
//! them, which is exactly the pre-pin behavior plus self-healing.

pub fn migration() -> String {
    "ALTER TABLE platform_addresses ADD COLUMN as_of_height INTEGER NOT NULL DEFAULT 0;"
        .to_string()
}
