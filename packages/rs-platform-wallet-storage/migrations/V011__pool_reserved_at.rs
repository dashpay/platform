//! Persist address-pool reservation timestamps (dashpay/platform#4188).
//!
//! A nullable timestamp preserves `AddressState::Reserved` while keeping
//! available and used rows unreserved.

pub fn migration() -> String {
    "ALTER TABLE core_address_pool ADD COLUMN reserved_at INTEGER NULL;".to_string()
}
