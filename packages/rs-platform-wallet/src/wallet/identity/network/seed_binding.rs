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
//!
//! The check is also the gate in front of the deferred contact-crypto drain —
//! see [`PlatformWallet::drain_pending_contact_crypto_verified`], the primitive
//! every client drains through so none of them can forget it.

use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;

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

    /// Drain the deferred contact-crypto queue, but only through a provider
    /// that has been shown to resolve this wallet's seed.
    ///
    /// Runs the provider-only ops
    /// ([`drain_pending_contact_crypto_until`]) and, when an identity signer is
    /// supplied, the DIP-15 auto-accept pass
    /// ([`drain_auto_accepts_until`]) — the same pair every drain entry point
    /// runs — and returns their combined completed count. `deadline` bounds
    /// both from the inside; `None` is unbounded.
    ///
    /// # Why the gate lives here
    ///
    /// Everything the drain derives comes from whatever seed the provider
    /// resolves, and none of it is authenticated. A provider mapped to the
    /// wrong wallet derives contact receiving xpubs from the wrong seed, and
    /// `register_contact_account` keys its existence check on `(index, us,
    /// them)` rather than on the xpub — so the wrong addresses are written
    /// once and every later correct-seed pass no-ops. The corruption is
    /// permanent and its only symptom is payments that never arrive.
    ///
    /// Putting the check in each client is what lets a client forget it: iOS
    /// enforced it in its Swift wrapper while the FFI drain entry point had no
    /// gate at all, so a JNI binding written against that entry point
    /// inherited the bug rather than the rule. This is the one primitive both
    /// the startup sequence and the FFI drain call, so there is a single place
    /// the gate can be removed from and none where it can be omitted.
    ///
    /// # Cost
    ///
    /// Proportional to the risk: an empty queue would derive nothing, so there
    /// is no wrong-seed write to prevent and the check is skipped entirely —
    /// a warm launch with nothing queued resolves no key material at all. Both
    /// drains ride the same queue, so one count covers both.
    ///
    /// # Errors
    ///
    /// Fails closed on **every** verification error, not only on
    /// [`PlatformWalletError::SeedMismatch`]: a provider that cannot answer has
    /// not been shown to own this wallet. Skipping costs nothing that is not
    /// recoverable — the queue is untouched, so the next signer-present drain
    /// completes exactly the work this one declined to guess at.
    ///
    /// [`drain_pending_contact_crypto_until`]: crate::wallet::identity::network::DashPayView::drain_pending_contact_crypto_until
    /// [`drain_auto_accepts_until`]: crate::wallet::identity::network::DashPayView::drain_auto_accepts_until
    pub async fn drain_pending_contact_crypto_verified<C, S>(
        &self,
        crypto: &C,
        identity_signer: Option<&S>,
        deadline: Option<std::time::Instant>,
    ) -> Result<usize, PlatformWalletError>
    where
        C: ContactCryptoProvider + Sync,
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let dashpay = self.identity().dashpay();
        if dashpay.drainable_contact_crypto_count().await == 0 {
            return Ok(0);
        }

        self.verify_seed_binds(crypto).await.inspect_err(|e| {
            tracing::error!(
                wallet_id = %hex::encode(self.wallet_id()),
                error = %e,
                "the contact-crypto provider does not bind to this wallet's seed; skipping \
                 the drain rather than deriving contact addresses that could never be corrected"
            );
        })?;

        let drained = dashpay
            .drain_pending_contact_crypto_until(crypto, deadline)
            .await;
        let accepted = match identity_signer {
            Some(signer) => {
                dashpay
                    .drain_auto_accepts_until(signer, crypto, deadline)
                    .await
            }
            None => 0,
        };
        Ok(drained + accepted)
    }
}

#[cfg(test)]
mod tests {
    use super::SeedBindingVerification;
    use std::sync::Arc;

    use key_wallet::mnemonic::{Language, Mnemonic};
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
        Mnemonic::from_phrase(phrase, Language::English)
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

    // -----------------------------------------------------------------------
    // The gate in front of the drain.
    //
    // `verify_seed_binds` above proves the check itself is right. These prove
    // the drain cannot run without it — the property that matters, because the
    // FFI entry point every JNI client binds to used to call the drains
    // directly and skip the check entirely.
    // -----------------------------------------------------------------------

    /// A different valid BIP-39 vector: the mis-mapped Keychain slot.
    const FOREIGN_MNEMONIC: &str =
        "legal winner thank year wave sausage worth useful legal winner thank yellow";

    /// Only ever passed as `None`, so the auto-accept pass is skipped — but the
    /// generic still has to be named.
    #[derive(Debug)]
    struct UnusedSigner;

    #[async_trait::async_trait]
    impl dpp::identity::signer::Signer<dpp::identity::IdentityPublicKey> for UnusedSigner {
        async fn sign(
            &self,
            _key: &dpp::identity::IdentityPublicKey,
            _data: &[u8],
        ) -> Result<dpp::platform_value::BinaryData, dpp::ProtocolError> {
            unreachable!("the auto-accept pass is never reached with a None signer")
        }

        async fn sign_create_witness(
            &self,
            _key: &dpp::identity::IdentityPublicKey,
            _data: &[u8],
        ) -> Result<dpp::address_funds::AddressWitness, dpp::ProtocolError> {
            unreachable!("the auto-accept pass is never reached with a None signer")
        }

        fn can_sign_with(&self, _key: &dpp::identity::IdentityPublicKey) -> bool {
            false
        }
    }

    /// A wallet owning one identity with a single queued `RegisterReceiving`
    /// op — the smallest state in which the drain has real work, and the op
    /// that derives a contact receiving xpub straight from the provider with
    /// no network round trip.
    async fn wallet_with_queued_contact_crypto() -> (
        Arc<PlatformWalletManager<NoopPersister>>,
        Arc<crate::PlatformWallet>,
        WalletId,
    ) {
        use crate::changeset::{
            upsert_pending_contact_crypto, PendingContactCrypto, PendingContactCryptoOp,
        };
        use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};
        use dpp::identity::v0::IdentityV0;
        use dpp::prelude::Identifier;

        let manager = make_manager();
        let wallet = manager
            .create_wallet_from_seed_bytes(
                Network::Testnet,
                &seed_for(TEST_MNEMONIC),
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet creation");
        let wallet_id = wallet.wallet_id();
        let persister = WalletPersister::new(wallet_id, Arc::new(NoPlatformPersistence));

        let mut wm = manager.wallet_manager.write().await;
        let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet info");
        info.identity_manager
            .add_identity(
                dpp::identity::Identity::V0(IdentityV0 {
                    id: Identifier::from([1u8; 32]),
                    public_keys: std::collections::BTreeMap::new(),
                    balance: 0,
                    revision: 0,
                }),
                0,
                wallet_id,
                &persister,
            )
            .expect("add identity");
        let managed = info
            .identity_manager
            .managed_identity_mut(&Identifier::from([1u8; 32]))
            .expect("managed identity");
        upsert_pending_contact_crypto(
            managed.dashpay_pending_contact_crypto_mut(),
            PendingContactCrypto {
                owner_identity_id: Identifier::from([1u8; 32]),
                contact_id: Identifier::from([2u8; 32]),
                op: PendingContactCryptoOp::RegisterReceiving,
                enqueued_at_ms: 0,
            },
        );
        drop(wm);

        (manager, wallet, wallet_id)
    }

    /// The DashPay receiving accounts the wallet is watching — the thing a
    /// wrong-seed drain corrupts. `register_contact_account` keys its
    /// existence check on `(index, us, them)` and NOT on the xpub, so an
    /// account written from the wrong seed is never revisited.
    async fn receiving_account_count(
        manager: &PlatformWalletManager<NoopPersister>,
        wallet_id: &WalletId,
    ) -> usize {
        let wm = manager.wallet_manager.read().await;
        wm.get_wallet_info(wallet_id)
            .map(|info| info.core_wallet.accounts.dashpay_receival_accounts.len())
            .unwrap_or(0)
    }

    async fn drainable(wallet: &crate::PlatformWallet) -> usize {
        wallet
            .identity()
            .dashpay()
            .drainable_contact_crypto_count()
            .await
    }

    /// The defect: a provider resolving someone else's seed derives contact
    /// receiving xpubs that are written once and never corrected, so the
    /// wallet watches addresses nobody pays to. The drain must not run at all,
    /// and the queue must survive intact for the next signer-present attempt.
    #[tokio::test]
    async fn a_wrong_seed_provider_is_refused_before_the_drain() {
        let (manager, wallet, wallet_id) = wallet_with_queued_contact_crypto().await;
        assert_eq!(receiving_account_count(&manager, &wallet_id).await, 0);
        assert_eq!(drainable(&wallet).await, 1);

        let foreign = SeedCryptoProvider::from_seed(seed_for(FOREIGN_MNEMONIC), Network::Testnet);
        let err = wallet
            .drain_pending_contact_crypto_verified(&foreign, None::<&UnusedSigner>, None)
            .await
            .expect_err("a provider that does not own the wallet must be refused");

        assert!(
            matches!(err, PlatformWalletError::SeedMismatch { .. }),
            "the refusal must be the typed wrong-seed error, got: {err:?}"
        );
        assert_eq!(
            receiving_account_count(&manager, &wallet_id).await,
            0,
            "not one contact account may be registered from the wrong seed"
        );
        assert_eq!(
            drainable(&wallet).await,
            1,
            "the queue must survive so the next correct-seed drain can do the work"
        );
    }

    /// The other half: the wallet's own seed passes the gate and the drain
    /// runs. Without this the test above would also pass if the gate simply
    /// refused everything.
    #[tokio::test]
    async fn the_owning_seed_passes_the_gate_and_the_drain_runs() {
        let (manager, wallet, wallet_id) = wallet_with_queued_contact_crypto().await;

        let owning = SeedCryptoProvider::from_seed(seed_for(TEST_MNEMONIC), Network::Testnet);
        let drained = wallet
            .drain_pending_contact_crypto_verified(&owning, None::<&UnusedSigner>, None)
            .await
            .expect("the wallet's own seed must bind");

        assert_eq!(drained, 1, "the queued RegisterReceiving op must complete");
        assert_eq!(
            receiving_account_count(&manager, &wallet_id).await,
            1,
            "the contact receiving account must exist after a verified drain"
        );
    }

    /// The gate is paid for only when there is something to protect. An empty
    /// queue would derive nothing, so no key material is resolved — which is
    /// what keeps this affordable on a warm launch. Proven with a provider
    /// that would FAIL the check: an `Ok` shows it was never consulted.
    #[tokio::test]
    async fn an_empty_queue_never_consults_the_provider() {
        let manager = make_manager();
        let wallet = manager
            .create_wallet_from_seed_bytes(
                Network::Testnet,
                &seed_for(TEST_MNEMONIC),
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet creation");
        assert_eq!(drainable(&wallet).await, 0);

        let foreign = SeedCryptoProvider::from_seed(seed_for(FOREIGN_MNEMONIC), Network::Testnet);
        let drained = wallet
            .drain_pending_contact_crypto_verified(&foreign, None::<&UnusedSigner>, None)
            .await
            .expect("with nothing to drain the binding check must not run at all");
        assert_eq!(drained, 0);
    }
}
