//! Two deterministic Orchard test wallets used by the SDK genesis test-data
//! seeder.
//!
//! The whole module is gated behind `#[cfg(create_sdk_test_data)]`, so the
//! seed bytes never reach a release binary's `strings` output.
//!
//! The key derivation mirrors `rs-platform-wallet::OrchardKeySet::from_seed`
//! exactly — ZIP-32 with `coin_type = 1` (Dash testnet/regtest) and
//! `account_id = 0`. This means a wallet test that calls
//! `manager.platform_wallet.bind_shielded(&SEED_A, &[0], &coord)` ends up with
//! byte-identical IVK / payment address to the chain-side seeded notes, so
//! `sync_shielded_notes` actually decrypts what we put on-chain.

use grovedb_commitment_tree::{
    FullViewingKey, IncomingViewingKey, OutgoingViewingKey, PaymentAddress,
    PreparedIncomingViewingKey, Scope, SpendingKey,
};
use std::sync::OnceLock;
use zip32::AccountId;

/// ZIP-32 coin type for Dash testnet/regtest. Matches `DASH_COIN_TYPE_TESTNET`
/// in `rs-platform-wallet::wallet::shielded::keys`. The SDK_TEST_DATA path is
/// regtest-only, so we never need the mainnet coin type here.
const COIN_TYPE_TESTNET_REGTEST: u32 = 1;

/// Hard-coded 32-byte seed for shielded test wallet A.
///
/// The 32 bytes are interpreted as an Orchard `SpendingKey` directly (no ZIP-32
/// derivation) — this is regtest-only test data, the seed never leaves the
/// regtest binary, and the validity of the resulting key is pinned by
/// [`tests::seed_a_derives_a_valid_wallet`].
pub const SEED_A: [u8; 32] = [0x73; 32];

/// Hard-coded 32-byte seed for shielded test wallet B. See [`SEED_A`].
pub const SEED_B: [u8; 32] = [0x74; 32];

/// Cached viewing-grade key material + spending key for a deterministic test
/// wallet.
///
/// The spending key is retained on purpose: Phase-1 acceptance includes
/// building (but not submitting) an Orchard spend bundle for an owned note,
/// which requires the `SpendingKey`. In a real wallet the SK lives only in the
/// host signer; here the regtest-only cfg gate keeps it scoped to test data.
pub struct TestWallet {
    pub full_viewing_key: FullViewingKey,
    pub incoming_viewing_key: IncomingViewingKey,
    pub prepared_ivk: PreparedIncomingViewingKey,
    pub outgoing_viewing_key: OutgoingViewingKey,
    pub default_address: PaymentAddress,
    pub spending_key: SpendingKey,
}

impl TestWallet {
    fn derive(seed: [u8; 32]) -> Self {
        // ZIP-32 derivation — matches `rs-platform-wallet::OrchardKeySet::from_seed`
        // byte-for-byte for `network = Regtest` (coin_type = 1), `account = 0`.
        // If this ever drifts, the functional test in `rs-platform-wallet/tests/
        // shielded_sync.rs` will fail loudly with "decrypted 0 notes" because
        // the wallet-side IVK won't match the chain-side recipient address.
        let spending_key =
            SpendingKey::from_zip32_seed(&seed, COIN_TYPE_TESTNET_REGTEST, AccountId::ZERO)
                .expect("ZIP-32 derivation must succeed for the hardcoded test seeds");
        let full_viewing_key = FullViewingKey::from(&spending_key);
        let incoming_viewing_key = full_viewing_key.to_ivk(Scope::External);
        let prepared_ivk = PreparedIncomingViewingKey::new(&incoming_viewing_key);
        let outgoing_viewing_key = full_viewing_key.to_ovk(Scope::External);
        let default_address = full_viewing_key.address_at(0u32, Scope::External);
        Self {
            full_viewing_key,
            incoming_viewing_key,
            prepared_ivk,
            outgoing_viewing_key,
            default_address,
            spending_key,
        }
    }
}

/// Wallet A — the first shielded test wallet.
pub fn test_wallet_a() -> &'static TestWallet {
    static WALLET: OnceLock<TestWallet> = OnceLock::new();
    WALLET.get_or_init(|| TestWallet::derive(SEED_A))
}

/// Wallet B — the second shielded test wallet.
pub fn test_wallet_b() -> &'static TestWallet {
    static WALLET: OnceLock<TestWallet> = OnceLock::new();
    WALLET.get_or_init(|| TestWallet::derive(SEED_B))
}

/// Both test wallets in stable order — `[A, B]`. Used by the seeder when it
/// needs to round-robin or split owned-note counts across both.
pub fn test_wallets() -> [&'static TestWallet; 2] {
    [test_wallet_a(), test_wallet_b()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_a_derives_a_valid_wallet() {
        // Pins the assumption that SEED_A maps to a non-degenerate Orchard SK
        // (ask != 0). If a future orchard bump changes the field semantics
        // and this seed no longer works, swap the constant — the rest of the
        // seeded test data depends on the wallets resolving.
        let w = test_wallet_a();
        // Sanity: address is non-zero (default address with a real diversifier).
        let addr_bytes = w.default_address.to_raw_address_bytes();
        assert!(addr_bytes.iter().any(|&b| b != 0));
    }

    #[test]
    fn seed_b_derives_a_valid_wallet() {
        let w = test_wallet_b();
        let addr_bytes = w.default_address.to_raw_address_bytes();
        assert!(addr_bytes.iter().any(|&b| b != 0));
    }

    #[test]
    fn wallets_a_and_b_are_distinct() {
        // Cross-wallet privacy depends on A and B having different IVKs and
        // different default addresses. Confirm directly.
        let a = test_wallet_a();
        let b = test_wallet_b();
        assert_ne!(
            a.default_address.to_raw_address_bytes(),
            b.default_address.to_raw_address_bytes(),
            "wallet A and B must derive distinct default addresses"
        );
        // IVKs don't expose `==`, so compare via the FVK bytes which is stable.
        assert_ne!(
            a.full_viewing_key.to_bytes(),
            b.full_viewing_key.to_bytes(),
            "wallet A and B must derive distinct full viewing keys"
        );
    }

    #[test]
    fn derivation_is_deterministic() {
        // Two calls to the cached accessor return the same instance (OnceLock
        // semantics) and re-deriving from scratch produces an equal wallet.
        let cached = test_wallet_a();
        let fresh = TestWallet::derive(SEED_A);
        assert_eq!(
            cached.default_address.to_raw_address_bytes(),
            fresh.default_address.to_raw_address_bytes()
        );
        assert_eq!(
            cached.full_viewing_key.to_bytes(),
            fresh.full_viewing_key.to_bytes()
        );
    }

    #[test]
    fn test_wallets_returns_a_then_b() {
        let [first, second] = test_wallets();
        assert_eq!(
            first.full_viewing_key.to_bytes(),
            test_wallet_a().full_viewing_key.to_bytes()
        );
        assert_eq!(
            second.full_viewing_key.to_bytes(),
            test_wallet_b().full_viewing_key.to_bytes()
        );
    }
}
