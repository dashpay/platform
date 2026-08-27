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
//! see [`DashPayView::drain_pending_contact_crypto_verified`] and
//! [`DashPayView::drain_auto_accepts_verified`], the two primitives every
//! provider-deriving pass goes through so no caller can forget the check, and
//! [`PlatformWallet::drain_pending_contact_crypto_verified`], the whole-wallet
//! wrapper that runs both.
//!
//! Each primitive gates itself and returns a [`ProviderBinding`] recording
//! whether the check actually ran, so a caller sequencing two of them carries
//! that evidence forward instead of re-deriving it from a queue probe that may
//! since have gone stale.

use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;

use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::identity::network::contact_requests::ContactCryptoProvider;
use crate::wallet::identity::network::dashpay_view::DashPayView;
use crate::wallet::identity::network::identity_handle::IdentityWallet;
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

/// Whether a contact-crypto provider has been put through the seed-binding
/// check *on this drain cycle*.
///
/// This exists because "the queue was empty a moment ago" is not the same
/// statement as "this provider owns this wallet", and the gated drain returns
/// the first while its callers were reading it as the second. Every gated
/// primitive in this module returns one, and every pass that would derive key
/// material through the provider takes one — so the fact of verification
/// travels with the value instead of being re-inferred from a queue probe that
/// has already gone stale.
///
/// It cannot be forged: the two constructors are private to this module, so
/// only a primitive that actually ran (or skipped) the check can mint one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderBinding(BindingState);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BindingState {
    /// `verify_seed_binds` passed for this provider.
    Verified,
    /// The check did not run — the gated drain found an empty queue and had
    /// nothing to derive. **Not** a licence to run further provider work: the
    /// queue can be refilled at any instant by the recurring contact sweep.
    NotEstablished,
}

impl ProviderBinding {
    /// The provider passed the seed-binding check.
    fn verified() -> Self {
        Self(BindingState::Verified)
    }

    /// The check did not run.
    fn not_established() -> Self {
        Self(BindingState::NotEstablished)
    }

    /// Whether the provider has been shown to resolve this wallet's seed.
    pub fn is_verified(self) -> bool {
        matches!(self.0, BindingState::Verified)
    }
}

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
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
            let guard = self.wallet_manager.read().await;
            let wallet = guard
                .get_wallet(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
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
                wallet_id: hex::encode(self.wallet_id),
            })
        }
    }
}

impl<B: TransactionBroadcaster + ?Sized> DashPayView<'_, B> {
    /// Drain the deferred contact-crypto queue, but only through a provider
    /// that has been shown to resolve this wallet's seed.
    ///
    /// Runs the provider-only ops ([`Self::drain_pending_contact_crypto_until`])
    /// behind the gate and returns the completed count. `deadline` bounds the
    /// drain from the inside; `None` is unbounded.
    ///
    /// This is the **innermost** gated primitive — the one every drain reaches,
    /// whatever handle the caller is holding. The startup sequence and the FFI
    /// drain entry point arrive via
    /// [`PlatformWallet::drain_pending_contact_crypto_verified`], which adds
    /// the DIP-15 auto-accept pass behind the same gate; the payment path
    /// ([`Self::send_payment`]) calls this one directly, because a
    /// `DashPayView` is what it has in hand.
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
    /// inherited the bug rather than the rule. Review found the same shape a
    /// third time on the payment path, which drained unverified before any
    /// funding-input signing could fail on the wrong seed. Hosting the gate on
    /// `DashPayView` — the handle the drain itself lives on — is what removes
    /// the last place it could be omitted from: there is no way to reach the
    /// drain with a provider that has not been through it.
    ///
    /// # Cost
    ///
    /// Proportional to the risk: an empty queue would derive nothing, so there
    /// is no wrong-seed write to prevent and the check is skipped entirely —
    /// a warm launch with nothing queued resolves no key material at all. The
    /// auto-accept pass rides the same queue, so the outer wrapper's early-out
    /// on an empty queue covers both.
    ///
    /// # Errors
    ///
    /// Fails closed on **every** verification error, not only on
    /// [`PlatformWalletError::SeedMismatch`]: a provider that cannot answer has
    /// not been shown to own this wallet. Skipping costs nothing that is not
    /// recoverable — the queue is untouched, so the next signer-present drain
    /// completes exactly the work this one declined to guess at.
    pub async fn drain_pending_contact_crypto_verified<C>(
        &self,
        crypto: &C,
        deadline: Option<std::time::Instant>,
    ) -> Result<usize, PlatformWalletError>
    where
        C: ContactCryptoProvider + Sync,
    {
        self.drain_pending_contact_crypto_verified_reporting(crypto, deadline)
            .await
            .map(|(drained, _)| drained)
    }

    /// [`Self::drain_pending_contact_crypto_verified`], additionally reporting
    /// whether the provider was actually put through the check.
    ///
    /// The empty-queue early-out returns without verifying — correctly, since
    /// there is nothing to derive — but that makes the return value ambiguous
    /// to a caller that wants to run *more* provider work afterwards. Review
    /// found the whole-wallet wrapper doing exactly that: it probed the queue,
    /// saw it nonempty, delegated here, and this call re-probed and found the
    /// queue emptied by a concurrent drain, so it returned `Ok(0)` unverified —
    /// after which the wrapper ran the DIP-15 auto-accept pass anyway. That
    /// pass re-snapshots the queue at its own instant, and the recurring
    /// contact sweep can enqueue an `AutoAccept` inside that window, so a
    /// brand-new entry could be processed through a provider nobody had
    /// checked. With a wrong seed the re-derived auto-accept key does not match
    /// the proof, and the mapping treats a verify failure as *permanent*: the
    /// valid proof is marked failed and dropped, so the sweep's enqueue gate
    /// never offers it again.
    ///
    /// So the fact of verification is returned rather than inferred. See
    /// [`Self::drain_auto_accepts_verified`], which takes it.
    pub async fn drain_pending_contact_crypto_verified_reporting<C>(
        &self,
        crypto: &C,
        deadline: Option<std::time::Instant>,
    ) -> Result<(usize, ProviderBinding), PlatformWalletError>
    where
        C: ContactCryptoProvider + Sync,
    {
        if self.drainable_contact_crypto_count().await == 0 {
            return Ok((0, ProviderBinding::not_established()));
        }

        self.establish_provider_binding(crypto).await?;

        Ok((
            self.drain_pending_contact_crypto_until(crypto, deadline)
                .await,
            ProviderBinding::verified(),
        ))
    }

    /// Run the DIP-15 auto-accept pass behind the same gate, given whatever
    /// binding a previous pass on this cycle established.
    ///
    /// `binding` is an optimisation, not the gate: an already-`Verified`
    /// provider is not re-derived, and anything else is checked here before a
    /// single entry is touched. So the auto-accept pass cannot run through an
    /// unverified provider no matter how its caller is sequenced — which is
    /// the property the wrapper was silently relying on the drain to provide
    /// and, on the empty-queue path, not getting.
    ///
    /// There is deliberately **no** empty-queue early-out here. The queue this
    /// would probe is re-snapshotted inside
    /// [`Self::drain_auto_accepts_until`] anyway, so a probe here would only
    /// re-open the same window: seeing it empty and skipping the check would
    /// leave the pass unverified for an entry enqueued a moment later. The
    /// check is cheap next to the risk, and it only runs at all once the
    /// wrapper has already seen work queued.
    ///
    /// # Errors
    ///
    /// Fails closed on every verification error, exactly as the drain does.
    /// The queue is untouched, so the next signer-present pass auto-accepts
    /// everything this one declined to guess at.
    pub async fn drain_auto_accepts_verified<S, C>(
        &self,
        signer: &S,
        crypto: &C,
        deadline: Option<std::time::Instant>,
        binding: ProviderBinding,
    ) -> Result<usize, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
        C: ContactCryptoProvider + Sync,
    {
        if !binding.is_verified() {
            self.establish_provider_binding(crypto).await?;
        }

        Ok(self.drain_auto_accepts_until(signer, crypto, deadline).await)
    }

    /// The check itself, with the shared refusal log. Returns the binding so
    /// callers propagate evidence rather than re-deriving the conclusion.
    async fn establish_provider_binding<C>(
        &self,
        crypto: &C,
    ) -> Result<ProviderBinding, PlatformWalletError>
    where
        C: ContactCryptoProvider + Sync,
    {
        self.verify_seed_binds(crypto).await.inspect_err(|e| {
            tracing::error!(
                wallet_id = %hex::encode(self.wallet_id),
                error = %e,
                "the contact-crypto provider does not bind to this wallet's seed; skipping \
                 the drain rather than deriving contact addresses that could never be corrected"
            );
        })?;
        Ok(ProviderBinding::verified())
    }
}

impl PlatformWallet {
    /// Verify the signer behind `crypto` resolves the seed that owns this
    /// wallet. Wallet-level entry point for
    /// [`IdentityWallet::verify_seed_binds`].
    pub async fn verify_seed_binds<C: ContactCryptoProvider + Sync>(
        &self,
        crypto: &C,
    ) -> Result<(), PlatformWalletError> {
        self.identity().verify_seed_binds(crypto).await
    }

    /// Marker-aware variant — see
    /// [`IdentityWallet::verify_seed_binds_with_marker`].
    pub async fn verify_seed_binds_with_marker<C: ContactCryptoProvider + Sync>(
        &self,
        crypto: &C,
        marker: Option<&str>,
        keychain_stamp: Option<&str>,
    ) -> Result<(SeedBindingVerification, Option<String>), PlatformWalletError> {
        self.identity()
            .verify_seed_binds_with_marker(crypto, marker, keychain_stamp)
            .await
    }

    /// The gated drain plus the DIP-15 auto-accept pass, for callers holding a
    /// whole wallet: the startup sequence and the FFI drain entry point.
    ///
    /// Both passes ride the same queue, so one emptiness check decides whether
    /// this wrapper does anything at all — but *not* whether the provider was
    /// checked. Each pass is separately gated, and the auto-accept pass is
    /// handed the binding the drain established rather than inheriting the
    /// assumption that it ran: the drain re-probes the queue and returns
    /// unverified when a concurrent drain emptied it first, and the auto-accept
    /// pass re-snapshots the queue at its own instant, so the two observations
    /// can disagree. [`DashPayView::drain_auto_accepts_verified`] runs the
    /// check itself in that case, so there is no path to an auto-accept
    /// through an unverified provider on any interleaving.
    ///
    /// Returns the combined completed count; `deadline` bounds both from the
    /// inside, `None` is unbounded. Errors exactly as the inner primitives do.
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

        let (drained, binding) = dashpay
            .drain_pending_contact_crypto_verified_reporting(crypto, deadline)
            .await?;
        let accepted = match identity_signer {
            Some(signer) => {
                dashpay
                    .drain_auto_accepts_verified(signer, crypto, deadline, binding)
                    .await?
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

    // -----------------------------------------------------------------------
    // The payment path's pre-drain.
    //
    // `send_payment` drains the deferred contact-crypto queue before it takes
    // the write guard and before any funding input is signed. Review found it
    // doing so UNVERIFIED — the third entry point to reach these ops with no
    // gate, after the FFI drain and before it the iOS-only Swift check. A
    // wrong-seed provider registered the contact account first and only then
    // failed the send on bad signatures, so the corruption outlived the error.
    // -----------------------------------------------------------------------

    /// Funding signer for the send path. Must never be reached: the
    /// seed-binding gate fires on the pre-drain, before the write guard, coin
    /// selection or any signature.
    struct UnreachableCoreSigner;

    #[async_trait::async_trait]
    impl key_wallet::signer::Signer for UnreachableCoreSigner {
        type Error = String;

        fn supported_methods(&self) -> &[key_wallet::signer::SignerMethod] {
            &[key_wallet::signer::SignerMethod::Digest]
        }

        async fn sign_ecdsa(
            &self,
            _path: &key_wallet::DerivationPath,
            _sighash: [u8; 32],
        ) -> Result<
            (
                dashcore::secp256k1::ecdsa::Signature,
                dashcore::secp256k1::PublicKey,
            ),
            Self::Error,
        > {
            unreachable!("the seed-binding gate must fire before any funding input is signed")
        }

        async fn public_key(
            &self,
            _path: &key_wallet::DerivationPath,
        ) -> Result<key_wallet::bip32::PublicKey, Self::Error> {
            unreachable!("the seed-binding gate must fire before any key derivation")
        }
    }

    /// The defect: a payment made through a wrong-seed provider used to drain
    /// first and fail second. The drain runs `RegisterReceiving` against
    /// whatever seed the provider resolves, and `register_contact_account`
    /// keys its existence check on `(index, us, them)` rather than the xpub —
    /// so the wrong account is written once, never revisited, and the wallet
    /// permanently watches addresses nobody pays to. The failed payment is
    /// recoverable; that account is not.
    #[tokio::test]
    async fn send_payment_refuses_a_wrong_seed_provider_before_the_drain() {
        use dpp::prelude::Identifier;

        let (manager, wallet, wallet_id) = wallet_with_queued_contact_crypto().await;
        assert_eq!(receiving_account_count(&manager, &wallet_id).await, 0);
        assert_eq!(drainable(&wallet).await, 1);

        let foreign = SeedCryptoProvider::from_seed(seed_for(FOREIGN_MNEMONIC), Network::Testnet);
        let err = wallet
            .identity()
            .dashpay()
            .send_payment(
                &Identifier::from([1u8; 32]),
                &Identifier::from([2u8; 32]),
                10_000,
                None,
                &UnreachableCoreSigner,
                &foreign,
            )
            .await
            .expect_err("a payment through a provider that does not own the wallet must fail");

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

    /// The other half: the wallet's own seed passes the gate on the payment
    /// path too, so the pre-drain still does its job (building the contact
    /// account the send needs). Without this the test above would also pass if
    /// the gate simply refused every payment.
    ///
    /// The send then fails on the missing DashPay *external* account — this
    /// fixture has no contact xpub to build one from — which is precisely the
    /// point: a failure PAST the gate, and a different one.
    #[tokio::test]
    async fn send_payment_lets_the_owning_seed_through_to_the_drain() {
        use dpp::prelude::Identifier;

        let (manager, wallet, wallet_id) = wallet_with_queued_contact_crypto().await;

        let owning = SeedCryptoProvider::from_seed(seed_for(TEST_MNEMONIC), Network::Testnet);
        let err = wallet
            .identity()
            .dashpay()
            .send_payment(
                &Identifier::from([1u8; 32]),
                &Identifier::from([2u8; 32]),
                10_000,
                None,
                &UnreachableCoreSigner,
                &owning,
            )
            .await
            .expect_err("this fixture has no external account, so the send cannot complete");

        assert!(
            !matches!(err, PlatformWalletError::SeedMismatch { .. }),
            "the owning seed must not be refused by the gate, got: {err:?}"
        );
        assert_eq!(
            receiving_account_count(&manager, &wallet_id).await,
            1,
            "the verified pre-drain must still build the contact receiving account"
        );
        assert_eq!(
            drainable(&wallet).await,
            0,
            "the queued op completed rather than being skipped"
        );
    }

    // -----------------------------------------------------------------------
    // The auto-accept pass and the queue-probe race.
    //
    // The whole-wallet wrapper probes the queue, delegates to the gated drain,
    // then runs the DIP-15 auto-accept pass. Review found that the drain
    // re-probes the queue and returns `Ok(0)` WITHOUT verifying when a
    // concurrent drain emptied it in between — after which the wrapper ran the
    // auto-accept pass anyway. That pass re-snapshots the queue at its own
    // instant, so an `AutoAccept` the recurring sweep enqueued inside the
    // window was processed through a provider nobody had checked.
    //
    // The damage is not a failed pass: `drain_auto_accepts_until` maps a proof
    // that does not verify against our re-derived key to a PERMANENT verdict
    // and clears it, and marks it so the sweep's enqueue gate will not offer it
    // again. A wrong seed re-derives the wrong key, so a perfectly valid proof
    // is destroyed.
    // -----------------------------------------------------------------------

    /// Identity signer for the auto-accept pass. Must never be reached: the
    /// gate fires before a single queue entry is touched.
    #[derive(Debug)]
    struct UnreachableIdentitySigner;

    #[async_trait::async_trait]
    impl dpp::identity::signer::Signer<dpp::identity::IdentityPublicKey>
        for UnreachableIdentitySigner
    {
        async fn sign(
            &self,
            _key: &dpp::identity::IdentityPublicKey,
            _data: &[u8],
        ) -> Result<dpp::platform_value::BinaryData, dpp::ProtocolError> {
            unreachable!("the seed-binding gate must fire before any auto-accept is signed")
        }

        async fn sign_create_witness(
            &self,
            _key: &dpp::identity::IdentityPublicKey,
            _data: &[u8],
        ) -> Result<dpp::address_funds::AddressWitness, dpp::ProtocolError> {
            unreachable!("the seed-binding gate must fire before any auto-accept is signed")
        }

        fn can_sign_with(&self, _key: &dpp::identity::IdentityPublicKey) -> bool {
            unreachable!("the seed-binding gate must fire before the signer is consulted")
        }
    }

    /// Queue a DIP-15 `AutoAccept` op — what the recurring contact sweep
    /// enqueues when it ingests an inbound request carrying an auto-accept
    /// proof.
    async fn enqueue_auto_accept(
        manager: &PlatformWalletManager<NoopPersister>,
        wallet_id: &WalletId,
        contact: u8,
    ) {
        use crate::changeset::{
            upsert_pending_contact_crypto, PendingContactCrypto, PendingContactCryptoOp,
        };
        use dpp::prelude::Identifier;

        let mut wm = manager.wallet_manager.write().await;
        let info = wm.get_wallet_info_mut(wallet_id).expect("wallet info");
        let managed = info
            .identity_manager
            .managed_identity_mut(&Identifier::from([1u8; 32]))
            .expect("managed identity");
        upsert_pending_contact_crypto(
            managed.dashpay_pending_contact_crypto_mut(),
            PendingContactCrypto {
                owner_identity_id: Identifier::from([1u8; 32]),
                contact_id: Identifier::from([contact; 32]),
                op: PendingContactCryptoOp::AutoAccept,
                enqueued_at_ms: 0,
            },
        );
    }

    /// Stand-in for a concurrent drain on another task completing its work:
    /// the queue this wallet's next probe will read is empty.
    async fn empty_the_queue(manager: &PlatformWalletManager<NoopPersister>, wallet_id: &WalletId) {
        use dpp::prelude::Identifier;

        let mut wm = manager.wallet_manager.write().await;
        let info = wm.get_wallet_info_mut(wallet_id).expect("wallet info");
        let managed = info
            .identity_manager
            .managed_identity_mut(&Identifier::from([1u8; 32]))
            .expect("managed identity");
        managed.dashpay_pending_contact_crypto_mut().clear();
    }

    /// The gated drain must say whether it actually checked the provider. Its
    /// empty-queue early-out is correct on its own terms — nothing to derive,
    /// nothing to protect — but a caller reading `Ok(0)` as "the provider is
    /// good" is reading something the drain never said.
    #[tokio::test]
    async fn the_gated_drain_reports_whether_it_verified_the_provider() {
        let (manager, wallet, wallet_id) = wallet_with_queued_contact_crypto().await;
        let owning = SeedCryptoProvider::from_seed(seed_for(TEST_MNEMONIC), Network::Testnet);

        // Work queued → the check runs and the binding is reported.
        let (drained, binding) = wallet
            .identity()
            .dashpay()
            .drain_pending_contact_crypto_verified_reporting(&owning, None)
            .await
            .expect("the owning seed binds");
        assert_eq!(drained, 1);
        assert!(
            binding.is_verified(),
            "a drain that ran must report the provider verified"
        );

        // Queue now empty → the early-out returns without checking, and must
        // report exactly that rather than an unqualified success.
        let (drained, binding) = wallet
            .identity()
            .dashpay()
            .drain_pending_contact_crypto_verified_reporting(&owning, None)
            .await
            .expect("an empty queue is not an error");
        assert_eq!(drained, 0);
        assert!(
            !binding.is_verified(),
            "an empty-queue early-out never consulted the provider, so it must not \
             report a binding it did not establish"
        );
        let _ = (&manager, &wallet_id);
    }

    /// The auto-accept pass gates itself. Handed a binding that was never
    /// established, it must run the check rather than trust the caller's
    /// sequencing — so no interleaving of the wrapper's two passes can reach
    /// an auto-accept through an unverified provider.
    #[tokio::test]
    async fn the_auto_accept_pass_refuses_an_unestablished_binding() {
        let (manager, wallet, wallet_id) = wallet_with_queued_contact_crypto().await;
        enqueue_auto_accept(&manager, &wallet_id, 3).await;
        let queued_before = drainable(&wallet).await;

        let foreign = SeedCryptoProvider::from_seed(seed_for(FOREIGN_MNEMONIC), Network::Testnet);
        let err = wallet
            .identity()
            .dashpay()
            .drain_auto_accepts_verified(
                &UnreachableIdentitySigner,
                &foreign,
                None,
                super::ProviderBinding::not_established(),
            )
            .await
            .expect_err("an unverified provider must not reach the auto-accept pass");

        assert!(
            matches!(err, PlatformWalletError::SeedMismatch { .. }),
            "the refusal must be the typed wrong-seed error, got: {err:?}"
        );
        assert_eq!(
            drainable(&wallet).await,
            queued_before,
            "not one queue entry may be touched — a proof cleared on a wrong-seed verify \
             is destroyed permanently, the sweep's enqueue gate never re-offers it"
        );
    }

    /// **The race, driven end to end.** The exact interleaving review
    /// described, in the order the whole-wallet wrapper composes its two
    /// passes:
    ///
    /// 1. the wrapper probes the queue and sees work;
    /// 2. a concurrent drain empties it before the gated drain looks;
    /// 3. the gated drain therefore returns without verifying;
    /// 4. the recurring sweep enqueues a fresh `AutoAccept` in that window;
    /// 5. the auto-accept pass runs — and must refuse.
    ///
    /// Step 2 is a real second task rather than a hope about scheduling: it is
    /// awaited to completion, so the interleaving is the one under test on
    /// every run instead of the one that happened to be scheduled.
    ///
    /// Before the fix this reached `drain_auto_accepts_until` with the foreign
    /// provider, because `Ok(0)` from step 3 was indistinguishable from a
    /// verified drain.
    #[tokio::test]
    async fn a_queue_emptied_between_the_probes_does_not_let_an_auto_accept_through() {
        let (manager, wallet, wallet_id) = wallet_with_queued_contact_crypto().await;
        let foreign = SeedCryptoProvider::from_seed(seed_for(FOREIGN_MNEMONIC), Network::Testnet);

        // 1. The wrapper's probe: nonempty, so it does not early-out.
        assert_eq!(
            drainable(&wallet).await,
            1,
            "precondition: the wrapper must see work queued"
        );

        // 2. A concurrent drain on another task empties the queue.
        {
            let manager = manager.clone();
            tokio::spawn(async move {
                empty_the_queue(&manager, &wallet_id).await;
            })
            .await
            .expect("the concurrent drain completes");
        }

        // 3. The gated drain re-probes, finds nothing, and returns UNVERIFIED.
        let (drained, binding) = wallet
            .identity()
            .dashpay()
            .drain_pending_contact_crypto_verified_reporting(&foreign, None)
            .await
            .expect("an empty queue is not an error, even for a foreign provider");
        assert_eq!(drained, 0);
        assert!(
            !binding.is_verified(),
            "the empty-queue path cannot have verified anything"
        );

        // 4. The recurring sweep enqueues a new AutoAccept inside the window.
        enqueue_auto_accept(&manager, &wallet_id, 4).await;
        assert_eq!(drainable(&wallet).await, 1);

        // 5. The auto-accept pass must refuse the unverified provider.
        let err = wallet
            .identity()
            .dashpay()
            .drain_auto_accepts_verified(&UnreachableIdentitySigner, &foreign, None, binding)
            .await
            .expect_err(
                "an auto-accept enqueued after the queue probe must not be processed \
                 through a provider that was never checked",
            );
        assert!(
            matches!(err, PlatformWalletError::SeedMismatch { .. }),
            "the refusal must be the typed wrong-seed error, got: {err:?}"
        );
        assert_eq!(
            drainable(&wallet).await,
            1,
            "the freshly enqueued auto-accept must survive for the next correct-seed pass"
        );
    }

    /// The other half: a verified binding is not re-derived. Without this the
    /// gate could be satisfied by refusing every auto-accept pass.
    #[tokio::test]
    async fn a_verified_binding_carries_into_the_auto_accept_pass() {
        let (manager, wallet, wallet_id) = wallet_with_queued_contact_crypto().await;
        let owning = SeedCryptoProvider::from_seed(seed_for(TEST_MNEMONIC), Network::Testnet);

        let (_, binding) = wallet
            .identity()
            .dashpay()
            .drain_pending_contact_crypto_verified_reporting(&owning, None)
            .await
            .expect("the owning seed binds");
        assert!(binding.is_verified());

        // Nothing is queued for the auto-accept pass to do, so it completes at
        // zero — the point is that it is reached at all.
        let accepted = wallet
            .identity()
            .dashpay()
            .drain_auto_accepts_verified(&UnreachableIdentitySigner, &owning, None, binding)
            .await
            .expect("a verified binding must carry into the auto-accept pass");
        assert_eq!(accepted, 0);
        let _ = (&manager, &wallet_id);
    }
}
