//! Persist typed public keys carried by address-pool rows (dashpay/platform#4113).
//!
//! This closes the gap where pre-derived platform-node Ed25519 keys were silently
//! dropped during SQLite round-trips. SLIP-10 supports hardened derivation only,
//! so a watch-only account xpub cannot regenerate them. Nullable key bytes and a
//! curve discriminator preserve typed entries; existing rows and snapshots whose
//! `AddressInfo` has no public key remain NULL in both columns.

pub fn migration() -> String {
    "ALTER TABLE core_address_pool ADD COLUMN public_key BLOB NULL;
ALTER TABLE core_address_pool ADD COLUMN key_type INTEGER NULL CHECK (key_type IS NULL OR key_type IN (0, 1, 2));"
        .to_string()
}
