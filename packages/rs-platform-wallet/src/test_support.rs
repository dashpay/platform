//! Shared unit-test scaffolding: mock broadcasters, a seed-backed signer,
//! and a funded in-memory wallet manager.
//!
//! Used by the broadcast-failure regression tests in `wallet::core::broadcast`
//! and `wallet::asset_lock::build`.

use std::collections::BTreeMap;
#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use dashcore::hashes::Hash;
use dashcore::secp256k1::{ecdsa, Message, PublicKey, Secp256k1};
use dashcore::BlockHash;
#[cfg(test)]
use dashcore::Txid;
use dashcore::{Network, Transaction};
use key_wallet::account::account_type::StandardAccountType;
use key_wallet::bip32::ExtendedPubKey;
// Only the `#[cfg(test)]` CoinJoin fixture needs the trait (for
// `next_address_with_info` on a non-standard account); gate it to match so a
// `test-utils`-only build does not flag it unused.
#[cfg(test)]
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::signer::{ExtendedPubKeySigner, Signer, SignerMethod};
use key_wallet::test_utils::TestWalletContext;
use key_wallet::transaction_checking::{BlockInfo, TransactionContext};
use key_wallet::{DerivationPath, Wallet};
use key_wallet_manager::WalletManager;
use tokio::sync::RwLock;

#[cfg(test)]
use crate::broadcaster::{BroadcastError, TransactionBroadcaster};
use crate::wallet::core::WalletGeneration;
use crate::wallet::identity::IdentityManager;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};

/// Broadcaster whose first call fails with a definitive pre-send rejection
/// and which succeeds afterwards, to model a transient broadcast error
/// followed by a user retry.
///
/// Only consumed by this crate's own `#[cfg(test)]` unit tests (never via
/// the `test-utils` feature alone), so it's gated on `cfg(test)` directly —
/// otherwise `--all-features` builds without `--tests` compile this with no
/// consumer and clippy flags it as dead code.
#[cfg(test)]
pub(crate) struct RejectFirstBroadcaster {
    failed_once: AtomicBool,
}

#[cfg(test)]
impl RejectFirstBroadcaster {
    pub(crate) fn new() -> Self {
        Self {
            failed_once: AtomicBool::new(false),
        }
    }
}

#[cfg(test)]
#[async_trait]
impl TransactionBroadcaster for RejectFirstBroadcaster {
    async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, BroadcastError> {
        if self.failed_once.swap(true, Ordering::SeqCst) {
            Ok(transaction.txid())
        } else {
            Err(BroadcastError::Rejected {
                reason: "simulated pre-send rejection".to_string(),
            })
        }
    }
}

/// Broadcaster that always succeeds, for flows that must run past the
/// broadcast step (e.g. the broadcast half of the funded asset-lock flow).
#[cfg(test)]
pub(crate) struct AlwaysOkBroadcaster;

#[cfg(test)]
#[async_trait]
impl TransactionBroadcaster for AlwaysOkBroadcaster {
    async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, BroadcastError> {
        Ok(transaction.txid())
    }
}

/// Broadcaster that always fails with a definitive pre-send rejection.
#[cfg(test)]
pub(crate) struct AlwaysRejectedBroadcaster;

#[cfg(test)]
#[async_trait]
impl TransactionBroadcaster for AlwaysRejectedBroadcaster {
    async fn broadcast(&self, _transaction: &Transaction) -> Result<Txid, BroadcastError> {
        Err(BroadcastError::Rejected {
            reason: "simulated pre-send rejection".to_string(),
        })
    }
}

/// Broadcaster that always fails with an *ambiguous* result — the network
/// may already have accepted the transaction — so its inputs must NOT be
/// released on failure.
#[cfg(test)]
pub(crate) struct AlwaysMaybeSentBroadcaster;

#[cfg(test)]
#[async_trait]
impl TransactionBroadcaster for AlwaysMaybeSentBroadcaster {
    async fn broadcast(&self, _transaction: &Transaction) -> Result<Txid, BroadcastError> {
        Err(BroadcastError::MaybeSent {
            reason: "simulated ambiguous broadcast".to_string(),
        })
    }
}

/// Soft signer that derives keys straight from a test wallet's seed. Stands
/// in for the FFI keychain-backed signer used in production.
#[derive(Clone)]
pub struct WalletSigner {
    wallet: Wallet,
}

#[async_trait]
impl Signer for WalletSigner {
    type Error = String;

    fn supported_methods(&self) -> &[SignerMethod] {
        &[SignerMethod::Digest]
    }

    async fn sign_ecdsa(
        &self,
        path: &DerivationPath,
        sighash: [u8; 32],
    ) -> Result<(ecdsa::Signature, PublicKey), Self::Error> {
        let secp = Secp256k1::new();
        let key = self
            .wallet
            .derive_private_key(path)
            .map_err(|e| e.to_string())?;
        let message = Message::from_digest(sighash);
        Ok((
            secp.sign_ecdsa(&message, &key),
            PublicKey::from_secret_key(&secp, &key),
        ))
    }

    async fn public_key(&self, path: &DerivationPath) -> Result<PublicKey, Self::Error> {
        let secp = Secp256k1::new();
        let key = self
            .wallet
            .derive_private_key(path)
            .map_err(|e| e.to_string())?;
        Ok(PublicKey::from_secret_key(&secp, &key))
    }
}

#[async_trait]
impl ExtendedPubKeySigner for WalletSigner {
    async fn extended_public_key(
        &self,
        path: &DerivationPath,
    ) -> Result<ExtendedPubKey, Self::Error> {
        // The test wallet is full-signable, so it can derive an extended
        // public key at a hardened path from its root xpriv — mirroring
        // what `MnemonicResolverCoreSigner` does via the Keychain mnemonic
        // in production.
        self.wallet
            .derive_extended_public_key(path)
            .map_err(|e| e.to_string())
    }
}

/// Builds a testnet wallet manager whose `account_type`/index-0 account
/// holds a single spendable UTXO (10_000_000 duffs) — the whole balance
/// rides on that one input, so a leaked reservation strands it. Returns
/// the manager, the wallet id, the shared balance handle, and a soft
/// signer over the wallet's seed.
pub(crate) async fn funded_wallet_manager(
    account_type: StandardAccountType,
) -> (
    Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    WalletId,
    Arc<WalletGeneration>,
    WalletSigner,
) {
    funded_wallet_manager_with_outputs(account_type, &[10_000_000]).await
}

/// Like [`funded_wallet_manager`] but with caller-chosen funding outputs —
/// multiple outputs yield multiple spendable UTXOs, letting tests run
/// concurrent asset-lock builds that each need their own input.
pub(crate) async fn funded_wallet_manager_with_outputs(
    account_type: StandardAccountType,
    outputs: &[u64],
) -> (
    Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    WalletId,
    Arc<WalletGeneration>,
    WalletSigner,
) {
    let mut ctx = TestWalletContext::new_random();

    // `new_random()` already derives a BIP44 receive address; only the
    // BIP32 arm needs a hand-rolled derivation.
    let receive_address = match account_type {
        StandardAccountType::BIP44Account => ctx.receive_address.clone(),
        StandardAccountType::BIP32Account => {
            let xpub = ctx
                .wallet
                .accounts
                .standard_bip32_accounts
                .get(&0)
                .expect("bip32 account")
                .account_xpub;
            ctx.managed_wallet
                .first_bip32_managed_account_mut()
                .expect("bip32 managed account")
                .next_receive_address(Some(&xpub), true)
                .expect("bip32 receive address")
        }
    };

    let funding_tx = Transaction::dummy(&receive_address, 0..1, outputs);
    // Chain-locked funding, not `Mempool`: asset-lock builders only
    // select final (confirmed / InstantSend-locked) inputs since
    // rust-dashcore#836, so a mempool-funded fixture leaves the
    // asset-lock tests with no eligible UTXO.
    let result = ctx
        .check_transaction(
            &funding_tx,
            TransactionContext::InChainLockedBlock(BlockInfo::new(
                1,
                BlockHash::all_zeros(),
                1_700_000_000,
            )),
        )
        .await;
    assert!(
        result.is_relevant,
        "funding tx should be relevant to {account_type:?}"
    );
    assert!(result.is_new_transaction);

    let signer = WalletSigner {
        wallet: ctx.wallet.clone(),
    };

    let generation = Arc::new(WalletGeneration::new());
    let info = PlatformWalletInfo {
        core_wallet: ctx.managed_wallet,
        generation: Arc::clone(&generation),
        identity_manager: IdentityManager::new(),
        tracked_asset_locks: BTreeMap::new(),
    };

    let mut wm = WalletManager::<PlatformWalletInfo>::new(Network::Testnet);
    let wallet_id = wm.insert_wallet(ctx.wallet, info).expect("insert wallet");

    (Arc::new(RwLock::new(wm)), wallet_id, generation, signer)
}

/// Like [`funded_wallet_manager`] but funds the wallet's CoinJoin account 0
/// (created by `WalletAccountCreationOptions::Default`) with a single spendable
/// UTXO. Lets the deferred-payment tests exercise a CoinJoin-funded reservation,
/// which has no `StandardAccountType` yet must still be released immediately on
/// rejection/abandon rather than stranded until the TTL backstop.
///
/// Only the crate's own `#[cfg(test)]` unit tests consume it, so it is gated on
/// `cfg(test)` directly — under the `test-utils` feature alone (the FFI crate's
/// build) it would compile with no user and trip `dead_code`.
#[cfg(test)]
pub(crate) async fn funded_coinjoin_wallet_manager() -> (
    Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    WalletId,
    Arc<WalletGeneration>,
    WalletSigner,
) {
    let mut ctx = TestWalletContext::new_random();

    let coinjoin_xpub = ctx
        .wallet
        .accounts
        .coinjoin_accounts
        .get(&0)
        .expect("default wallet has CoinJoin account 0")
        .account_xpub;
    // CoinJoin is a non-standard account type: its addresses come from the
    // single external pool via `next_address_with_info`, not the standard
    // receive/change split that `next_receive_address` serves.
    let receive_address = ctx
        .managed_wallet
        .first_coinjoin_managed_account_mut()
        .expect("coinjoin managed account")
        .next_address_with_info(Some(&coinjoin_xpub), true)
        .expect("coinjoin receive address")
        .address;

    let funding_tx = Transaction::dummy(&receive_address, 0..1, &[10_000_000]);
    let result = ctx
        .check_transaction(
            &funding_tx,
            TransactionContext::InChainLockedBlock(BlockInfo::new(
                1,
                BlockHash::all_zeros(),
                1_700_000_000,
            )),
        )
        .await;
    assert!(
        result.is_relevant,
        "funding tx should be relevant to the CoinJoin account"
    );
    assert!(result.is_new_transaction);

    let signer = WalletSigner {
        wallet: ctx.wallet.clone(),
    };

    let generation = Arc::new(WalletGeneration::new());
    let info = PlatformWalletInfo {
        core_wallet: ctx.managed_wallet,
        generation: Arc::clone(&generation),
        identity_manager: IdentityManager::new(),
        tracked_asset_locks: BTreeMap::new(),
    };

    let mut wm = WalletManager::<PlatformWalletInfo>::new(Network::Testnet);
    let wallet_id = wm.insert_wallet(ctx.wallet, info).expect("insert wallet");

    (Arc::new(RwLock::new(wm)), wallet_id, generation, signer)
}

/// Builds a testnet wallet manager whose balance is split across TWO privacy
/// domains: `bip44_duffs` on BIP44 account 0 and `coinjoin_duffs` on the DIP-9
/// CoinJoin account 0. Lets the funding-domain tests prove that coin selection
/// never crosses from one account into the other.
///
/// Returns the manager, the wallet id, and a soft signer over the wallet's seed
/// (which can derive keys for BOTH accounts, so per-account signing can be
/// exercised end-to-end).
#[cfg(test)]
pub(crate) async fn split_funded_wallet_manager(
    bip44_duffs: u64,
    coinjoin_duffs: u64,
) -> (
    Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    WalletId,
    Arc<WalletGeneration>,
    WalletSigner,
) {
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait as _;

    let mut ctx = TestWalletContext::new_random();

    // Fund BIP44 account 0 (the default funding account) at its pre-derived
    // receive address.
    let bip44_tx = Transaction::dummy(&ctx.receive_address, 0..1, &[bip44_duffs]);
    let bip44_result = ctx
        .check_transaction(
            &bip44_tx,
            TransactionContext::InChainLockedBlock(BlockInfo::new(
                1,
                BlockHash::all_zeros(),
                1_700_000_000,
            )),
        )
        .await;
    assert!(
        bip44_result.is_relevant && bip44_result.is_new_transaction,
        "BIP44 funding tx should be recognized"
    );

    // Derive a fresh CoinJoin receive address (registering it in the CoinJoin
    // pool so the checker recognizes the funding), then fund CoinJoin account 0.
    let coinjoin_xpub = ctx
        .wallet
        .get_coinjoin_account(0)
        .expect("default wallet has CoinJoin account 0")
        .account_xpub;
    // CoinJoin is a single-pool (non-standard) account, so it derives via
    // `next_address` rather than `next_receive_address`.
    let coinjoin_address = ctx
        .managed_wallet
        .first_coinjoin_managed_account_mut()
        .expect("default wallet has a managed CoinJoin account 0")
        .next_address(Some(&coinjoin_xpub), true)
        .expect("CoinJoin receive address");
    let coinjoin_tx = Transaction::dummy(&coinjoin_address, 0..1, &[coinjoin_duffs]);
    let coinjoin_result = ctx
        .check_transaction(
            &coinjoin_tx,
            TransactionContext::InChainLockedBlock(BlockInfo::new(
                2,
                BlockHash::all_zeros(),
                1_700_000_100,
            )),
        )
        .await;
    assert!(
        coinjoin_result.is_relevant && coinjoin_result.is_new_transaction,
        "CoinJoin funding tx should be recognized"
    );

    let signer = WalletSigner {
        wallet: ctx.wallet.clone(),
    };

    // Returned, not discarded: a `CoreWallet` built over a *different*
    // `WalletGeneration` than the one registered here is a foreign generation,
    // and every generation-bound path (above all the owner-guarded reservation
    // release) correctly no-ops against it. Handing the real handle back is what
    // keeps callers from silently testing a no-op.
    let generation = Arc::new(WalletGeneration::new());
    let info = PlatformWalletInfo {
        core_wallet: ctx.managed_wallet,
        generation: Arc::clone(&generation),
        identity_manager: IdentityManager::new(),
        tracked_asset_locks: BTreeMap::new(),
    };

    let mut wm = WalletManager::<PlatformWalletInfo>::new(Network::Testnet);
    let wallet_id = wm.insert_wallet(ctx.wallet, info).expect("insert wallet");

    (Arc::new(RwLock::new(wm)), wallet_id, generation, signer)
}

/// Which DashPay funds account arm a [`split_funded_wallet_manager_dashpay`]
/// fixture provisions. The two arms differ only in which collection they land
/// in and the DIP-15 derivation order (user/friend vs friend/user), but both
/// are fund-bearing and both are covered by the vendored asset-lock router fix
/// (`get_relevant_account_types(AssetLock)` lists `DashpayReceivingFunds` AND
/// `DashpayExternalAccount` alongside `CoinJoin`).
///
/// Kept verbatim from #4184 so the two copies reconcile cleanly when both
/// land: only the `ReceivingFunds` arm has a consumer in THIS crate's tests
/// (the finalize funding-path tests), while #4184's asset-lock tests drive
/// the watch-only `ExternalAccount` arm — hence the `dead_code` allowance
/// rather than trimming the variant.
#[cfg(test)]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub(crate) enum DashpayLeg {
    /// Incoming DashPay funds account (`user_id/friend_id`).
    ReceivingFunds,
    /// DashPay external (watch-only-style) account (`friend_id/user_id`).
    ExternalAccount,
}

/// An account-level xpub whose private keys the wallet under test does NOT
/// hold — derived from a SEPARATE random wallet. Models a DashPay contact's
/// decrypted xpub, from which production builds the watch-only
/// `DashpayExternalAccount` (`is_watch_only: true`,
/// `wallet/identity/network/contacts.rs`). Any well-formed testnet account
/// xpub serves as a single-pool account key; using a FOREIGN one makes the
/// account unsignable by the local seed exactly as it is in production, so an
/// asset-lock builder that (wrongly) selected its UTXOs would sign them with
/// the local mnemonic's key and produce an invalid input signature.
#[cfg(test)]
fn foreign_contact_account_xpub() -> ExtendedPubKey {
    let foreign = TestWalletContext::new_random();
    foreign
        .wallet
        .accounts
        .standard_bip44_accounts
        .get(&0)
        .expect("foreign wallet has BIP44 account 0")
        .account_xpub
}

/// Builds a testnet wallet manager whose balance is SPLIT across BIP44 account
/// 0 (`bip44_duffs`) and a DashPay funds account (`dashpay_duffs`) — the
/// DashPay analogue of [`split_funded_wallet_manager`]'s BIP44 + CoinJoin split.
/// `leg` selects which DashPay account type carries the mixed slice.
///
/// This exercises the DashPay legs of the vendored asset-lock router fix that
/// the CoinJoin fixture does not reach: `get_relevant_account_types(AssetLock)`
/// covers `CoinJoin`, `DashpayReceivingFunds`, AND `DashpayExternalAccount`, so
/// an asset lock funded from a DashPay UTXO must have that input debited by the
/// `check_core_transaction` scan (dashpay/platform#4073, dashpay/dash-wallet#1507).
///
/// `WalletAccountCreationOptions::Default` does not create DashPay accounts, so
/// this provisions one — identity ids are arbitrary-but-distinct test vectors —
/// on BOTH the signing `Wallet` and the `ManagedWalletInfo`, derives a fresh
/// receive address from its single pool (registering it so the checker
/// recognizes the funding), and funds it, mirroring how
/// [`split_funded_wallet_manager`] funds the CoinJoin account.
///
/// Signability of the DashPay input matches production per arm (see the inline
/// note in the body): the `ReceivingFunds` account is derived from our own
/// seed and is signable end-to-end; the `ExternalAccount` account is watch-only
/// (its xpub is a contact's, from a foreign seed), so the local signer CANNOT
/// sign its UTXOs — the asset-lock builder must exclude them.
#[cfg(test)]
pub(crate) async fn split_funded_wallet_manager_dashpay(
    bip44_duffs: u64,
    dashpay_duffs: u64,
    leg: DashpayLeg,
) -> (
    Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    WalletId,
    WalletSigner,
) {
    use key_wallet::account::account_collection::DashpayAccountKey;
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    use key_wallet::wallet::managed_wallet_info::managed_account_operations::ManagedAccountOperations;
    use key_wallet::AccountType;

    let mut ctx = TestWalletContext::new_random();

    // Fund BIP44 account 0 (the primary) at its pre-derived receive address.
    let bip44_tx = Transaction::dummy(&ctx.receive_address, 0..1, &[bip44_duffs]);
    let bip44_result = ctx
        .check_transaction(
            &bip44_tx,
            TransactionContext::InChainLockedBlock(BlockInfo::new(
                1,
                BlockHash::all_zeros(),
                1_700_000_000,
            )),
        )
        .await;
    assert!(
        bip44_result.is_relevant && bip44_result.is_new_transaction,
        "BIP44 funding tx should be recognized"
    );

    // Provision a DashPay funds account of the requested arm on the wallet and
    // mirror it into the managed side. The identity ids are arbitrary distinct
    // test vectors; distinct ids keep the receiving (user/friend) and external
    // (friend/user) derivations on different keys/paths.
    let user_identity_id = [0x11u8; 32];
    let friend_identity_id = [0x22u8; 32];
    let account_type = match leg {
        DashpayLeg::ReceivingFunds => AccountType::DashpayReceivingFunds {
            index: 0,
            user_identity_id,
            friend_identity_id,
        },
        DashpayLeg::ExternalAccount => AccountType::DashpayExternalAccount {
            index: 0,
            user_identity_id,
            friend_identity_id,
        },
    };
    // Provision the DashPay funds account of the requested arm, matching how
    // production derives each so the local signer's capability is faithful:
    //
    //  * `ReceivingFunds` is OURS. Production derives it from our own
    //    friendship xpub (`register_contact_account`, `is_watch_only: false`),
    //    so the local seed CAN sign it. `add_account(_, None)` models that by
    //    deriving the account from this wallet's own root xpriv.
    //
    //  * `ExternalAccount` is the CONTACT's, WATCH-ONLY. Production builds it
    //    from the contact's decrypted xpub (`register_dashpay_external_account`,
    //    `is_watch_only: true`), whose private keys live under a DIFFERENT seed
    //    the wallet does not hold. Model that faithfully: derive the account
    //    xpub from a SEPARATE random wallet and insert it via
    //    `add_account(_, Some(xpub))`, which stores the account
    //    `is_watch_only: true`. The old shortcut — `add_account(_, None)` for
    //    BOTH arms — derived the external account from our OWN seed, making it
    //    locally signable and MASKING the union-funding bug (an asset lock
    //    would silently spend the contact's coins with a wrong-key, invalid
    //    signature). This arm now proves the builder excludes it.
    match leg {
        DashpayLeg::ReceivingFunds => {
            ctx.wallet
                .add_account(account_type, None)
                .expect("add DashPay receiving account to wallet");
        }
        DashpayLeg::ExternalAccount => {
            let foreign_xpub = foreign_contact_account_xpub();
            ctx.wallet
                .add_account(account_type, Some(foreign_xpub))
                .expect("add watch-only DashPay external account to wallet");
        }
    }
    ctx.managed_wallet
        .add_managed_account(&ctx.wallet, account_type)
        .expect("mirror DashPay account into managed wallet");

    // Derive a fresh DashPay receive address from the single-pool managed
    // account, then fund it. DashPay accounts are single-pool (like CoinJoin),
    // so they derive via `next_address` rather than `next_receive_address`.
    let key = DashpayAccountKey {
        index: 0,
        user_identity_id,
        friend_identity_id,
    };
    let dashpay_xpub = match leg {
        DashpayLeg::ReceivingFunds => ctx.wallet.accounts.dashpay_receival_accounts.get(&key),
        DashpayLeg::ExternalAccount => ctx.wallet.accounts.dashpay_external_accounts.get(&key),
    }
    .expect("DashPay account present in wallet")
    .account_xpub;
    let dashpay_address = {
        let managed = match leg {
            DashpayLeg::ReceivingFunds => ctx
                .managed_wallet
                .accounts
                .dashpay_receival_accounts
                .get_mut(&key),
            DashpayLeg::ExternalAccount => ctx
                .managed_wallet
                .accounts
                .dashpay_external_accounts
                .get_mut(&key),
        }
        .expect("managed DashPay account present");
        managed
            .next_address(Some(&dashpay_xpub), true)
            .expect("DashPay receive address")
    };
    let dashpay_tx = Transaction::dummy(&dashpay_address, 0..1, &[dashpay_duffs]);
    let dashpay_result = ctx
        .check_transaction(
            &dashpay_tx,
            TransactionContext::InChainLockedBlock(BlockInfo::new(
                2,
                BlockHash::all_zeros(),
                1_700_000_100,
            )),
        )
        .await;
    assert!(
        dashpay_result.is_relevant && dashpay_result.is_new_transaction,
        "DashPay funding tx should be recognized"
    );

    let signer = WalletSigner {
        wallet: ctx.wallet.clone(),
    };

    let generation = Arc::new(WalletGeneration::new());
    let info = PlatformWalletInfo {
        core_wallet: ctx.managed_wallet,
        generation,
        identity_manager: IdentityManager::new(),
        tracked_asset_locks: BTreeMap::new(),
    };

    let mut wm = WalletManager::<PlatformWalletInfo>::new(Network::Testnet);
    let wallet_id = wm.insert_wallet(ctx.wallet, info).expect("insert wallet");

    (Arc::new(RwLock::new(wm)), wallet_id, signer)
}

/// Funded SPV-backed Core wallet for downstream FFI lifecycle tests. The SPV
/// runtime is intentionally not started; abandon/free only need wallet state.
pub async fn funded_spv_core_wallet(
    account_type: StandardAccountType,
) -> (
    crate::CoreWallet<crate::broadcaster::SpvBroadcaster>,
    WalletSigner,
) {
    let (manager, wallet_id, generation, signer) = funded_wallet_manager(account_type).await;
    let spv = Arc::new(crate::spv::SpvRuntime::new(
        Arc::clone(&manager),
        Arc::new(crate::events::PlatformEventManager::new(Vec::new())),
    ));
    let broadcaster = Arc::new(crate::broadcaster::SpvBroadcaster::new(spv));
    let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
    (
        crate::CoreWallet::new(sdk, manager, wallet_id, broadcaster, generation),
        signer,
    )
}

/// No-op persister satisfying [`PlatformWalletManager`] construction for tests
/// that need a full [`PlatformWallet`] but no real persistence pipeline.
pub struct NoopTestPersister;

impl crate::changeset::PlatformWalletPersistence for NoopTestPersister {
    fn store(
        &self,
        _wallet_id: WalletId,
        _changeset: crate::changeset::PlatformWalletChangeSet,
    ) -> Result<(), crate::changeset::PersistenceError> {
        Ok(())
    }

    fn flush(&self, _wallet_id: WalletId) -> Result<(), crate::changeset::PersistenceError> {
        Ok(())
    }

    fn load(
        &self,
    ) -> Result<crate::changeset::ClientStartState, crate::changeset::PersistenceError> {
        Ok(crate::changeset::ClientStartState::default())
    }
}

struct NoopTestEventHandler;
impl crate::events::EventHandler for NoopTestEventHandler {}
impl crate::events::PlatformEventHandler for NoopTestEventHandler {}

/// Build a full [`PlatformWallet`] over a mock SDK and a no-op persister, wired
/// through a real [`PlatformWalletManager`] so its `wallet_manager` `Arc` and
/// `wallet_id` are production-shaped. Returns the manager (which the caller must
/// keep alive — it owns the wallet-event adapter task and the registered
/// `Arc<PlatformWallet>`) alongside the wallet id.
///
/// Used by FFI-layer tests that need genuine `PlatformWallet` aliases, e.g. the
/// `platform_wallet_destroy` regression asserting that destroying wrapper
/// aliases never sweeps an independently-owned deferred-payment token.
pub async fn test_platform_wallet_manager() -> (
    Arc<crate::PlatformWalletManager<NoopTestPersister>>,
    WalletId,
) {
    use key_wallet::mnemonic::{Language, Mnemonic};
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;

    // Canonical all-`abandon` BIP-39 test vector.
    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon \
         abandon abandon abandon abandon abandon about";

    let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
    let persister = Arc::new(NoopTestPersister);
    let event_handler: Arc<dyn crate::events::PlatformEventHandler> =
        Arc::new(NoopTestEventHandler);
    let manager = Arc::new(crate::PlatformWalletManager::new(
        sdk,
        persister,
        event_handler,
    ));

    let mnemonic =
        Mnemonic::from_phrase(TEST_MNEMONIC, Language::English).expect("valid test mnemonic");
    let seed_bytes = mnemonic.to_seed("");
    // `Some(0)` skips the SPV birth-height lookup so the create never hits the
    // network.
    let wallet = manager
        .create_wallet_from_seed_bytes(
            Network::Testnet,
            &seed_bytes,
            WalletAccountCreationOptions::Default,
            Some(0),
        )
        .await
        .expect("create test wallet");
    let wallet_id = wallet.wallet_id();
    (manager, wallet_id)
}
