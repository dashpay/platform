//! Wrong-seed / wrong-wallet self-check for a seedless wallet at unlock.
//!
//! The persisted-restore path rehydrates every wallet external-signable —
//! per-account xpubs only, no resident key material. Signing runs through the
//! host's Keychain-backed signer rather than a seed grafted onto the wallet.
//! Before trusting that signer, the host verifies it actually resolves *this*
//! wallet's seed: derive the BIP44 account-0 extended public key through the
//! signer and compare it to the wallet's persisted account xpub. A mis-mapped
//! Keychain slot — the signer resolving some other wallet's mnemonic — derives
//! a different xpub and is refused, so it can never sign for the wrong wallet.
//! This is the wrong-seed detection without ever holding a resident seed.

use crate::error::PlatformWalletError;
use crate::wallet::identity::network::contact_requests::ContactCryptoProvider;
use crate::wallet::platform_wallet::PlatformWallet;

/// How [`PlatformWallet::verify_seed_binds_with_marker`] established the
/// seed binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedBindingVerification {
    /// The host's persisted verified-binding marker equals the wallet's
    /// current account-0 xpub combined with the current Keychain-item stamp —
    /// the binding was proven on an earlier launch and neither the xpub nor
    /// the mnemonic item has changed since, so the signer was not consulted.
    MarkerMatched,
    /// The full check ran: the account-0 xpub was derived through the signer
    /// and matched the persisted one. The host should persist the returned
    /// marker (when one is handed back) so later launches can skip the
    /// derivation.
    Verified,
}

impl PlatformWallet {
    /// Verify the signer behind `crypto` resolves the seed that owns this wallet.
    ///
    /// Reads the wallet's persisted BIP44 account-0 xpub and the path it was
    /// rooted at, derives the xpub at that same path through `crypto` (the host
    /// signer, which holds the seed), and compares. The wallet itself may be
    /// watch-only / external-signable — it only supplies its stored xpub, never
    /// a private key.
    ///
    /// Returns [`PlatformWalletError::SeedMismatch`] if the derived xpub differs
    /// from the persisted one (the signer is mapped to the wrong wallet), so a
    /// wrong-seed signer fails loud at unlock instead of silently signing for a
    /// wallet it does not own.
    pub async fn verify_seed_binds<C: ContactCryptoProvider + Sync>(
        &self,
        crypto: &C,
    ) -> Result<(), PlatformWalletError> {
        self.verify_seed_binds_with_marker(crypto, None, None)
            .await
            .map(|_| ())
    }

    /// Marker-aware variant of [`Self::verify_seed_binds`]: the derivation is a
    /// pure function of (seed, network) against a fixed persisted xpub, so its
    /// outcome cannot change between launches *while the underlying Keychain
    /// mnemonic item is untouched*. After one successful verify the host
    /// persists the returned marker and passes it back on later launches
    /// together with `keychain_stamp`, an opaque identity/generation stamp of
    /// the mnemonic Keychain item (e.g. its creation+modification dates) that
    /// changes on every write to that item. The marker binds BOTH the wallet's
    /// account-0 xpub AND the stamp, so a match proves the previously verified
    /// mnemonic item is still the one in the Keychain — only then is the
    /// signer skipped (no mnemonic resolution, no derivation). A rewritten or
    /// re-created mnemonic item changes the stamp and falls through to the
    /// full signer check, exactly like a first launch or wallet re-import.
    ///
    /// Returns the outcome plus the marker to persist: `Some` only when a
    /// full verification ran, bound, and a stamp was supplied (without a
    /// stamp there is nothing safe to cache — the caller keeps verifying
    /// every launch). Errors exactly like [`Self::verify_seed_binds`] when
    /// the full check runs and fails.
    pub async fn verify_seed_binds_with_marker<C: ContactCryptoProvider + Sync>(
        &self,
        crypto: &C,
        marker: Option<&str>,
        keychain_stamp: Option<&str>,
    ) -> Result<(SeedBindingVerification, Option<String>), PlatformWalletError> {
        // Read the binding xpub and its exact derivation path from the same
        // account, so the two can never drift. Drop the lock before awaiting the
        // signer — the guard is not held across `.await`.
        let (path, expected) = {
            let guard = self.state().await;
            let wallet = guard.wallet();
            let account = wallet.get_bip44_account(0).ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "wallet has no BIP44 account 0 to verify the seed against".to_string(),
                )
            })?;
            let path = account
                .account_type
                .derivation_path(wallet.network)
                .map_err(|e| PlatformWalletError::KeyDerivation(e.to_string()))?;
            (path, account.account_xpub)
        };

        // The only marker that can match is the one for the CURRENT xpub and
        // the CURRENT Keychain item stamp. No stamp → no candidate → the full
        // check below always runs (fail-safe: cache disabled, not bypassed).
        let candidate = keychain_stamp.map(|stamp| format!("{expected}|{stamp}"));
        if let (Some(marker), Some(candidate)) = (marker, candidate.as_deref()) {
            if marker == candidate {
                return Ok((SeedBindingVerification::MarkerMatched, None));
            }
        }

        let derived = crypto.receiving_xpub(&path).await?;
        if derived == expected {
            Ok((SeedBindingVerification::Verified, candidate))
        } else {
            Err(PlatformWalletError::SeedMismatch {
                wallet_id: hex::encode(self.wallet_id()),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SeedBindingVerification;
    use std::sync::Arc;

    use key_wallet::mnemonic::Mnemonic;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::Network;

    use crate::changeset::{
        ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    use crate::error::PlatformWalletError;
    use crate::events::{EventHandler, PlatformEventHandler};
    use crate::wallet::identity::network::contact_requests::SeedCryptoProvider;
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
        Mnemonic::from_phrase(phrase)
            .expect("valid test mnemonic")
            .to_seed("")
    }

    /// The signer that resolves the wallet's own seed binds: the BIP44
    /// account-0 xpub it derives matches the wallet's persisted account xpub.
    #[tokio::test]
    async fn verify_seed_binds_accepts_matching_signer() {
        let manager = make_manager();
        let network = Network::Testnet;
        let seed = seed_for(TEST_MNEMONIC);

        // Created external-signable (no resident key material), exactly the
        // persisted-restore posture the unlock check runs against.
        let wallet = manager
            .create_wallet_from_seed_bytes(
                network,
                &seed,
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet creation");
        assert!(
            !wallet.state().await.wallet().has_seed(),
            "precondition: wallet must be seedless / external-signable"
        );

        let crypto = SeedCryptoProvider::from_seed(seed, network);
        wallet
            .verify_seed_binds(&crypto)
            .await
            .expect("the wallet's own seed must bind");
    }

    /// A signer resolving a *different* seed derives a different BIP44
    /// account-0 xpub and is rejected with `SeedMismatch` — the wrong-seed
    /// detection that protects against a mis-mapped Keychain slot.
    #[tokio::test]
    async fn verify_seed_binds_rejects_wrong_signer() {
        let manager = make_manager();
        let network = Network::Testnet;
        let seed = seed_for(TEST_MNEMONIC);

        let wallet = manager
            .create_wallet_from_seed_bytes(
                network,
                &seed,
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet creation");

        // A different mnemonic → a signer for the wrong wallet.
        let wrong_seed =
            seed_for("legal winner thank year wave sausage worth useful legal winner thank yellow");
        let wrong_crypto = SeedCryptoProvider::from_seed(wrong_seed, network);

        let err = wallet
            .verify_seed_binds(&wrong_crypto)
            .await
            .expect_err("a signer for a different seed must be rejected");
        assert!(
            matches!(err, PlatformWalletError::SeedMismatch { .. }),
            "expected SeedMismatch, got: {err:?}"
        );
    }

    /// First launch (no marker): the full signer check runs, binds, and
    /// returns `Verified` plus the marker for the host to persist — the
    /// wallet's account-0 xpub bound to the supplied Keychain-item stamp.
    #[tokio::test]
    async fn verify_with_no_marker_runs_full_check_and_returns_marker() {
        let manager = make_manager();
        let network = Network::Testnet;
        let seed = seed_for(TEST_MNEMONIC);

        let wallet = manager
            .create_wallet_from_seed_bytes(
                network,
                &seed,
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet creation");

        let crypto = SeedCryptoProvider::from_seed(seed, network);
        let (outcome, marker) = wallet
            .verify_seed_binds_with_marker(&crypto, None, Some("stamp-1"))
            .await
            .expect("the wallet's own seed must bind");
        assert_eq!(outcome, SeedBindingVerification::Verified);

        let expected_xpub = wallet
            .state()
            .await
            .wallet()
            .get_bip44_account(0)
            .expect("account 0")
            .account_xpub
            .to_string();
        assert_eq!(
            marker.as_deref(),
            Some(format!("{expected_xpub}|stamp-1").as_str()),
            "marker binds the account-0 xpub to the Keychain-item stamp"
        );
    }

    /// Later launch (marker matches the wallet's current xpub AND the current
    /// Keychain-item stamp): the signer is not consulted at all — proven by
    /// passing a signer for the WRONG seed, which would fail the full check,
    /// and still getting `MarkerMatched`. Nothing new to persist.
    #[tokio::test]
    async fn verify_with_matching_marker_skips_signer() {
        let manager = make_manager();
        let network = Network::Testnet;
        let seed = seed_for(TEST_MNEMONIC);

        let wallet = manager
            .create_wallet_from_seed_bytes(
                network,
                &seed,
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet creation");
        let xpub = wallet
            .state()
            .await
            .wallet()
            .get_bip44_account(0)
            .expect("account 0")
            .account_xpub
            .to_string();
        let marker = format!("{xpub}|stamp-1");

        let wrong_seed =
            seed_for("legal winner thank year wave sausage worth useful legal winner thank yellow");
        let wrong_crypto = SeedCryptoProvider::from_seed(wrong_seed, network);

        let (outcome, returned) = wallet
            .verify_seed_binds_with_marker(&wrong_crypto, Some(&marker), Some("stamp-1"))
            .await
            .expect("a matching marker must short-circuit before the signer runs");
        assert_eq!(outcome, SeedBindingVerification::MarkerMatched);
        assert_eq!(returned, None, "a matched marker has nothing to persist");
    }

    /// The mnemonic Keychain item was rewritten since the marker was
    /// persisted (its stamp changed): the marker must NOT match even though
    /// the xpub part is current, and the full check must re-run against the
    /// item's actual content — a replaced/mis-mapped mnemonic is rejected
    /// with `SeedMismatch`; the original mnemonic re-verifies and hands back
    /// a marker carrying the new stamp.
    #[tokio::test]
    async fn verify_with_changed_keychain_stamp_reruns_full_check() {
        let manager = make_manager();
        let network = Network::Testnet;
        let seed = seed_for(TEST_MNEMONIC);

        let wallet = manager
            .create_wallet_from_seed_bytes(
                network,
                &seed,
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet creation");
        let xpub = wallet
            .state()
            .await
            .wallet()
            .get_bip44_account(0)
            .expect("account 0")
            .account_xpub
            .to_string();
        // Marker verified while the item carried stamp-1; the item has since
        // been rewritten and now carries stamp-2.
        let marker = format!("{xpub}|stamp-1");

        // The rewrite put a DIFFERENT mnemonic in the slot → must be caught.
        let wrong_seed =
            seed_for("legal winner thank year wave sausage worth useful legal winner thank yellow");
        let wrong_crypto = SeedCryptoProvider::from_seed(wrong_seed, network);
        let err = wallet
            .verify_seed_binds_with_marker(&wrong_crypto, Some(&marker), Some("stamp-2"))
            .await
            .expect_err("a stamp change must force the full check, catching a replaced mnemonic");
        assert!(
            matches!(err, PlatformWalletError::SeedMismatch { .. }),
            "expected SeedMismatch, got: {err:?}"
        );

        // The rewrite re-stored the SAME mnemonic → re-verifies, and the new
        // marker carries the new stamp.
        let crypto = SeedCryptoProvider::from_seed(seed, network);
        let (outcome, returned) = wallet
            .verify_seed_binds_with_marker(&crypto, Some(&marker), Some("stamp-2"))
            .await
            .expect("the wallet's own seed must re-verify after a stamp change");
        assert_eq!(outcome, SeedBindingVerification::Verified);
        assert_eq!(returned, Some(format!("{xpub}|stamp-2")));
    }

    /// No Keychain stamp available: there is no safe candidate to match, so
    /// the full check always runs and nothing is handed back to cache — the
    /// caller keeps verifying every launch (fail-safe: cache disabled, never
    /// bypassed).
    #[tokio::test]
    async fn verify_without_stamp_never_matches_and_returns_no_marker() {
        let manager = make_manager();
        let network = Network::Testnet;
        let seed = seed_for(TEST_MNEMONIC);

        let wallet = manager
            .create_wallet_from_seed_bytes(
                network,
                &seed,
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet creation");
        let xpub = wallet
            .state()
            .await
            .wallet()
            .get_bip44_account(0)
            .expect("account 0")
            .account_xpub
            .to_string();

        let crypto = SeedCryptoProvider::from_seed(seed, network);
        let (outcome, returned) = wallet
            .verify_seed_binds_with_marker(&crypto, Some(&format!("{xpub}|stamp-1")), None)
            .await
            .expect("the wallet's own seed must bind");
        assert_eq!(
            outcome,
            SeedBindingVerification::Verified,
            "without a stamp the marker must not match"
        );
        assert_eq!(returned, None, "without a stamp there is nothing to cache");
    }

    /// A stale marker (wallet re-imported / xpub changed since it was written)
    /// falls through to the full signer check: the right signer re-verifies
    /// and hands back the fresh marker; the wrong signer is still rejected.
    #[tokio::test]
    async fn verify_with_stale_marker_falls_through_to_full_check() {
        let manager = make_manager();
        let network = Network::Testnet;
        let seed = seed_for(TEST_MNEMONIC);

        let wallet = manager
            .create_wallet_from_seed_bytes(
                network,
                &seed,
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet creation");
        let current_xpub = wallet
            .state()
            .await
            .wallet()
            .get_bip44_account(0)
            .expect("account 0")
            .account_xpub
            .to_string();

        let crypto = SeedCryptoProvider::from_seed(seed, network);
        let (outcome, marker) = wallet
            .verify_seed_binds_with_marker(
                &crypto,
                Some("stale-marker-from-a-previous-import"),
                Some("stamp-1"),
            )
            .await
            .expect("the wallet's own seed must bind after a stale marker");
        assert_eq!(outcome, SeedBindingVerification::Verified);
        assert_eq!(marker, Some(format!("{current_xpub}|stamp-1")));

        let wrong_seed =
            seed_for("legal winner thank year wave sausage worth useful legal winner thank yellow");
        let wrong_crypto = SeedCryptoProvider::from_seed(wrong_seed, network);
        let err = wallet
            .verify_seed_binds_with_marker(
                &wrong_crypto,
                Some("stale-marker-from-a-previous-import"),
                Some("stamp-1"),
            )
            .await
            .expect_err("a stale marker must not let a wrong signer through");
        assert!(
            matches!(err, PlatformWalletError::SeedMismatch { .. }),
            "expected SeedMismatch, got: {err:?}"
        );
    }

    /// A wallet with no BIP44 account 0 (created without the default account
    /// set) is refused with `InvalidIdentityData` — the gate has no persisted
    /// xpub to bind against, so it must fail closed rather than pass silently.
    #[tokio::test]
    async fn verify_seed_binds_rejects_wallet_without_bip44_account() {
        let manager = make_manager();
        let network = Network::Testnet;
        let seed = seed_for(TEST_MNEMONIC);

        let wallet = manager
            .create_wallet_from_seed_bytes(
                network,
                &seed,
                WalletAccountCreationOptions::None,
                Some(0),
            )
            .await
            .expect("wallet creation");
        assert!(
            wallet.state().await.wallet().get_bip44_account(0).is_none(),
            "precondition: wallet has no BIP44 account 0"
        );

        let crypto = SeedCryptoProvider::from_seed(seed, network);
        let err = wallet
            .verify_seed_binds(&crypto)
            .await
            .expect_err("a wallet with no BIP44 account 0 cannot be bound");
        assert!(
            matches!(err, PlatformWalletError::InvalidIdentityData(_)),
            "expected InvalidIdentityData, got: {err:?}"
        );
    }
}
