//! In-place seed upgrade for a loaded external-signable wallet.
//!
//! The persisted-restore path (`Wallet::new_external_signable`, see
//! `rs-platform-wallet-ffi::persistence::build_wallet_start_state`)
//! rehydrates every wallet **watch-only**: it carries the per-account
//! xpubs (enough to track funds and generate addresses) but no root key
//! material. Any operation that needs a private key — DashPay contact
//! xpub derivation (`derive_contact_xpub`), identity-key signing, etc. —
//! fails with `External signable wallet has no private key` after every
//! app relaunch.
//!
//! [`PlatformWalletManager::attach_wallet_seed`] closes that gap: given
//! the seed (fetched by the host from its Keychain), it re-derives the
//! signing wallet from the seed and grafts the seed-bearing
//! [`WalletType`](key_wallet::wallet::WalletType) onto the already-loaded
//! wallet **in place** — preserving the persisted account set, the
//! `PlatformWalletInfo` (managed accounts, identity manager, tracked
//! asset locks), and every other piece of loaded state untouched.

use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::Wallet;

use crate::changeset::PlatformWalletPersistence;
use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::WalletId;

use super::PlatformWalletManager;

impl<P: PlatformWalletPersistence + 'static> PlatformWalletManager<P> {
    /// Upgrade an already-loaded external-signable wallet to a fully
    /// seeded signing wallet **in place**, preserving all loaded state.
    ///
    /// The persisted-restore path rehydrates wallets watch-only (no key
    /// material). This re-derives the signing wallet from `seed` and
    /// swaps the seed-bearing [`WalletType`](key_wallet::wallet::WalletType)
    /// onto the wallet held in the inner
    /// [`WalletManager`](key_wallet_manager::WalletManager) — the
    /// associated `PlatformWalletInfo` (managed accounts / address pools,
    /// identity manager, tracked asset locks) is **not** rebuilt or
    /// touched.
    ///
    /// ## Safety gate
    ///
    /// `seed` is verified to actually belong to `wallet_id`, accepting
    /// either of two cryptographic bindings:
    ///
    /// 1. **Id match** — the wallet id recomputed from the seed (the way
    ///    `create_wallet_from_seed_bytes` does it, network-scoped via
    ///    `Wallet::from_seed_bytes`) equals `wallet_id`; or
    /// 2. **Xpub match (legacy ids)** — the persisted BIP44 account 0
    ///    xpub equals the one re-derived from the seed. Wallets created
    ///    before the network-scoped wallet-id scheme carry ids today's
    ///    recompute can't reproduce, but their persisted xpubs still
    ///    bind the seed exactly (observed in the field 2026-06-12: a
    ///    2026-05-28 devnet wallet whose Keychain mnemonic was correct
    ///    yet failed the id gate).
    ///
    /// If neither binds, [`PlatformWalletError::SeedMismatch`] is
    /// returned — the wrong seed is never attached.
    ///
    /// ## Account preservation
    ///
    /// The persisted external-signable wallet's accounts were derived
    /// from this same seed when the wallet was first created, so their
    /// xpubs are authoritative. They are kept verbatim — only
    /// `wallet_type` changes — so address pools, used-flags, and every
    /// downstream `walletId`/xpub-keyed structure stay byte-identical. A
    /// debug-only sanity check confirms the persisted BIP44 account 0
    /// xpub matches the one re-derived from the seed.
    ///
    /// ## Idempotency
    ///
    /// A no-op (returns `Ok`) if the wallet is already seed-backed
    /// (`Mnemonic` / `Seed` variant) — e.g. a wallet created in-session
    /// from its mnemonic, or a second `attach` after the first.
    ///
    /// Returns [`PlatformWalletError::WalletNotFound`] if no wallet with
    /// `wallet_id` is registered.
    pub async fn attach_wallet_seed(
        &self,
        wallet_id: WalletId,
        seed: &[u8; 64],
    ) -> Result<(), PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;

        // Read the loaded wallet's network. The id is network-scoped, so
        // re-deriving from the seed must use the *same* network or the
        // recomputed id can't match. Also short-circuit the idempotent
        // case before doing any key derivation.
        let network = {
            let wallet = wm
                .get_wallet(&wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(wallet_id)))?;
            if wallet.has_seed() {
                // Already a signing wallet (created in-session from its
                // mnemonic, or a repeated attach). Nothing to do.
                return Ok(());
            }
            wallet.network
        };

        // Re-derive the signing wallet from the seed. `Default` account
        // options match `create_wallet_from_seed_bytes` so the derived
        // wallet id and account xpubs agree with what was first
        // persisted. This is the same construction
        // `create_wallet_from_seed_bytes` uses, so the network-scoped id
        // it stamps is exactly the safety gate's reference value.
        let seeded = Wallet::from_seed_bytes(*seed, network, WalletAccountCreationOptions::Default)
            .map_err(|e| {
                PlatformWalletError::WalletCreation(format!(
                    "Failed to re-derive wallet from seed: {}",
                    e
                ))
            })?;

        // Graft target. The only mutable-wallet accessor the inner
        // `WalletManager` exposes is the split-borrow
        // `get_wallet_mut_and_info_mut`; we mutate the wallet only and
        // leave `_info` alone.
        let (wallet, _info) = wm.get_wallet_mut_and_info_mut(&wallet_id).ok_or_else(|| {
            // The wallet vanished between the read above and here — only
            // possible under a concurrent `remove_wallet`, which would
            // need the same write lock we hold, so this is unreachable in
            // practice. Surface it as NotFound rather than panic.
            PlatformWalletError::WalletNotFound(hex::encode(wallet_id))
        })?;

        // SAFETY GATE: the seed must bind to this wallet by id OR by
        // xpub (see the doc comment — xpub covers pre-network-scoped-id
        // wallets whose stored id today's recompute can't reproduce).
        // Never graft a seed that satisfies neither.
        let id_matches = seeded.wallet_id == wallet_id;
        let xpub_matches = matches!(
            (wallet.get_bip44_account(0), seeded.get_bip44_account(0)),
            (Some(persisted), Some(derived)) if persisted.account_xpub == derived.account_xpub
        );
        if !id_matches && !xpub_matches {
            return Err(PlatformWalletError::SeedMismatch {
                wallet_id: hex::encode(wallet_id),
                derived_id: hex::encode(seeded.wallet_id),
            });
        }
        if !id_matches {
            tracing::info!(
                wallet_id = %hex::encode(wallet_id),
                derived_id = %hex::encode(seeded.wallet_id),
                "attach_wallet_seed: accepting via BIP44-0 xpub match \
                 (wallet predates the network-scoped id scheme)"
            );
        }

        // `Wallet` implements `Drop` (zeroizes key material), so a field
        // can't be moved out of `seeded`. Swap the two `wallet_type`
        // fields instead: the loaded wallet gains the seed-bearing type,
        // and `seeded` is left holding the old `ExternalSignable` unit
        // variant (nothing sensitive) and dropped at end of scope.
        let mut seeded = seeded;
        std::mem::swap(&mut wallet.wallet_type, &mut seeded.wallet_type);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use key_wallet::mnemonic::{Language, Mnemonic};
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::wallet::Wallet;
    use key_wallet::Network;

    use crate::changeset::{
        ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    use crate::error::PlatformWalletError;
    use crate::events::{EventHandler, PlatformEventHandler};
    use crate::wallet::platform_wallet::WalletId;
    use crate::PlatformWalletManager;

    // Canonical all-`abandon` BIP-39 test vector.
    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon \
         abandon abandon abandon abandon abandon about";

    struct NoopPersister;
    impl PlatformWalletPersistence for NoopPersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            Ok(())
        }
        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }
        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    struct NoopEventHandler;
    impl EventHandler for NoopEventHandler {}
    impl PlatformEventHandler for NoopEventHandler {}

    fn make_manager() -> Arc<PlatformWalletManager<NoopPersister>> {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let persister = Arc::new(NoopPersister);
        let event_handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
        Arc::new(PlatformWalletManager::new(sdk, persister, event_handler))
    }

    fn seed_for(phrase: &str) -> [u8; 64] {
        Mnemonic::from_phrase(phrase, Language::English)
            .expect("valid test mnemonic")
            .to_seed("")
    }

    /// Build a watch-only / external-signable replica of a seeded wallet:
    /// same id, same account xpubs, but no key material — exactly the
    /// shape the persisted-restore path produces. Registers it directly
    /// in the inner `WalletManager` so the manager holds an
    /// external-signable wallet to upgrade.
    async fn register_external_signable(
        manager: &PlatformWalletManager<NoopPersister>,
        network: Network,
        seed: &[u8; 64],
    ) -> WalletId {
        let seeded = Wallet::from_seed_bytes(*seed, network, WalletAccountCreationOptions::Default)
            .expect("seeded wallet");
        let external =
            Wallet::new_external_signable(network, seeded.wallet_id, seeded.accounts.clone());
        let info = crate::wallet::platform_wallet::PlatformWalletInfo {
            core_wallet: key_wallet::wallet::managed_wallet_info::ManagedWalletInfo::from_wallet(
                &external, 0,
            ),
            balance: Arc::new(crate::wallet::core::WalletBalance::new()),
            identity_manager: crate::wallet::identity::IdentityManager::new(),
            tracked_asset_locks: std::collections::BTreeMap::new(),
            pending_contact_crypto: Vec::new(),
        };
        let mut wm = manager.wallet_manager.write().await;
        wm.insert_wallet(external, info)
            .expect("insert external-signable")
    }

    /// Happy path: an external-signable wallet flips to signing-capable
    /// (`has_seed()` / `can_sign()` over a real key) after attach, and
    /// the persisted account xpubs are preserved verbatim.
    #[tokio::test]
    async fn attach_seed_upgrades_external_signable_in_place() {
        let manager = make_manager();
        let network = Network::Testnet;
        let seed = seed_for(TEST_MNEMONIC);

        let wallet_id = register_external_signable(&manager, network, &seed).await;

        // Snapshot the persisted BIP44-0 xpub before the upgrade.
        let xpub_before = {
            let wm = manager.wallet_manager.read().await;
            let w = wm.get_wallet(&wallet_id).unwrap();
            assert!(w.is_external_signable(), "precondition: external-signable");
            assert!(!w.has_seed(), "precondition: no key material");
            w.get_bip44_account(0).expect("bip44 0").account_xpub
        };

        manager
            .attach_wallet_seed(wallet_id, &seed)
            .await
            .expect("attach should succeed for the matching seed");

        let wm = manager.wallet_manager.read().await;
        let w = wm.get_wallet(&wallet_id).unwrap();
        assert!(w.has_seed(), "wallet must be seed-backed after attach");
        assert!(!w.is_external_signable(), "no longer external-signable");
        assert_eq!(
            w.get_bip44_account(0).expect("bip44 0").account_xpub,
            xpub_before,
            "persisted account xpub must be preserved across the upgrade"
        );
    }

    /// The safety gate: a seed that derives to a different wallet id must
    /// be rejected with `SeedMismatch`, leaving the wallet untouched.
    #[tokio::test]
    async fn attach_seed_rejects_mismatched_seed() {
        let manager = make_manager();
        let network = Network::Testnet;
        let real_seed = seed_for(TEST_MNEMONIC);

        let wallet_id = register_external_signable(&manager, network, &real_seed).await;

        // A different mnemonic → different network-scoped wallet id.
        let wrong_seed =
            seed_for("legal winner thank year wave sausage worth useful legal winner thank yellow");

        let err = manager
            .attach_wallet_seed(wallet_id, &wrong_seed)
            .await
            .expect_err("attach must reject a seed that derives to a different id");
        assert!(
            matches!(err, PlatformWalletError::SeedMismatch { .. }),
            "expected SeedMismatch, got: {err:?}"
        );

        // Wallet stays watch-only — the wrong seed was not grafted.
        let wm = manager.wallet_manager.read().await;
        assert!(
            !wm.get_wallet(&wallet_id).unwrap().has_seed(),
            "rejected attach must leave the wallet external-signable"
        );
    }

    /// Attaching to an unknown wallet id is `WalletNotFound`.
    #[tokio::test]
    async fn attach_seed_unknown_wallet_is_not_found() {
        let manager = make_manager();
        let seed = seed_for(TEST_MNEMONIC);
        let err = manager
            .attach_wallet_seed([0u8; 32], &seed)
            .await
            .expect_err("unknown wallet must fail");
        assert!(
            matches!(err, PlatformWalletError::WalletNotFound(_)),
            "expected WalletNotFound, got: {err:?}"
        );
    }

    /// Legacy-id fallback: a wallet registered under an id today's
    /// recompute can't reproduce (pre-network-scoped-id scheme) must
    /// still accept its true seed via the BIP44-0 xpub binding.
    #[tokio::test]
    async fn attach_seed_accepts_legacy_wallet_id_via_xpub_match() {
        let manager = make_manager();
        let network = Network::Testnet;
        let seed = seed_for(TEST_MNEMONIC);

        // Same account set as the real seed, but registered under a
        // synthetic legacy id that no recompute path will produce.
        let seeded = Wallet::from_seed_bytes(seed, network, WalletAccountCreationOptions::Default)
            .expect("seeded wallet");
        let legacy_id: WalletId = [0xAB; 32];
        let external = Wallet::new_external_signable(network, legacy_id, seeded.accounts.clone());
        {
            let info = crate::wallet::platform_wallet::PlatformWalletInfo {
                core_wallet:
                    key_wallet::wallet::managed_wallet_info::ManagedWalletInfo::from_wallet(
                        &external, 0,
                    ),
                balance: Arc::new(crate::wallet::core::WalletBalance::new()),
                identity_manager: crate::wallet::identity::IdentityManager::new(),
                tracked_asset_locks: std::collections::BTreeMap::new(),
                pending_contact_crypto: Vec::new(),
            };
            let mut wm = manager.wallet_manager.write().await;
            wm.insert_wallet(external, info)
                .expect("insert legacy external-signable");
        }

        manager
            .attach_wallet_seed(legacy_id, &seed)
            .await
            .expect("xpub binding must accept the true seed despite the legacy id");

        let wm = manager.wallet_manager.read().await;
        assert!(wm.get_wallet(&legacy_id).unwrap().has_seed());
    }

    /// Idempotency: attaching to a wallet that is already seed-backed is
    /// a no-op `Ok` (covers in-session-created wallets + repeated attach).
    #[tokio::test]
    async fn attach_seed_on_seeded_wallet_is_noop() {
        let manager = make_manager();
        let network = Network::Testnet;
        let seed = seed_for(TEST_MNEMONIC);

        // Register a fully-seeded wallet directly.
        let seeded = Wallet::from_seed_bytes(seed, network, WalletAccountCreationOptions::Default)
            .expect("seeded wallet");
        let wallet_id = seeded.wallet_id;
        {
            let info = crate::wallet::platform_wallet::PlatformWalletInfo {
                core_wallet:
                    key_wallet::wallet::managed_wallet_info::ManagedWalletInfo::from_wallet(
                        &seeded, 0,
                    ),
                balance: Arc::new(crate::wallet::core::WalletBalance::new()),
                identity_manager: crate::wallet::identity::IdentityManager::new(),
                tracked_asset_locks: std::collections::BTreeMap::new(),
                pending_contact_crypto: Vec::new(),
            };
            let mut wm = manager.wallet_manager.write().await;
            wm.insert_wallet(seeded, info).expect("insert seeded");
        }

        manager
            .attach_wallet_seed(wallet_id, &seed)
            .await
            .expect("attach on an already-seeded wallet is a no-op Ok");

        let wm = manager.wallet_manager.read().await;
        assert!(wm.get_wallet(&wallet_id).unwrap().has_seed());
    }
}
