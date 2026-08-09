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
            dpns_name_states: BTreeMap::new(),
    };

    let mut wm = WalletManager::<PlatformWalletInfo>::new(Network::Testnet);
    let wallet_id = wm.insert_wallet(ctx.wallet, info).expect("insert wallet");

    (Arc::new(RwLock::new(wm)), wallet_id, generation, signer)
}

/// Funds BOTH standard families — BIP44 account 0 and BIP32 account 0 — each
/// with its own chain-locked UTXO set, for the pooled-send tests: a spend
/// larger than either family's balance must draw from both.
///
/// `cfg(test)`-gated like [`RejectFirstBroadcaster`]: only this crate's own
/// unit tests consume it, so a `test-utils`-only build would flag it unused.
#[cfg(test)]
pub(crate) async fn funded_wallet_manager_dual_standard(
    bip44_outputs: &[u64],
    bip32_outputs: &[u64],
) -> (
    Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    WalletId,
    Arc<WalletGeneration>,
    WalletSigner,
) {
    let mut ctx = TestWalletContext::new_random();

    let bip44_address = ctx.receive_address.clone();
    let bip32_address = {
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
    };

    for (address, outputs, tag) in [
        (&bip44_address, bip44_outputs, 0..1),
        (&bip32_address, bip32_outputs, 1..2),
    ] {
        let funding_tx = Transaction::dummy(address, tag, outputs);
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
        assert!(result.is_relevant, "funding tx should be relevant");
    }

    let signer = WalletSigner {
        wallet: ctx.wallet.clone(),
    };
    let generation = Arc::new(WalletGeneration::new());
    let info = PlatformWalletInfo {
        core_wallet: ctx.managed_wallet,
        generation: Arc::clone(&generation),
        identity_manager: IdentityManager::new(),
        tracked_asset_locks: BTreeMap::new(),
            dpns_name_states: BTreeMap::new(),
    };
    let mut wm = WalletManager::<PlatformWalletInfo>::new(Network::Testnet);
    let wallet_id = wm.insert_wallet(ctx.wallet, info).expect("insert wallet");
    (Arc::new(RwLock::new(wm)), wallet_id, generation, signer)
}

/// Funds BIP44 account 0 and a real `DashpayReceivingFunds` contact account,
/// so the pooled-send tests can prove that contact funds are actually SPENT —
/// the point of pulling `AllDashpayReceivingFunds` into `SEND_FUNDING_SOURCES`.
///
/// The contact account is built exactly as `DashPayView::register_contact_account`
/// builds it (DIP-15 xpub, `Account` in the key collection, funds-bearing managed
/// account in the managed collection) minus the persistence round, which needs a
/// full manager the fixture does not have. Returns the contact's `AccountType`
/// so the test can assert on the recorded funding accounts by identity.
#[cfg(test)]
pub(crate) async fn funded_wallet_manager_with_contact(
    bip44_outputs: &[u64],
    contact_outputs: &[u64],
) -> (
    Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    WalletId,
    Arc<WalletGeneration>,
    WalletSigner,
    key_wallet::account::AccountType,
) {
    use dpp::identifier::Identifier;
    use key_wallet::account::AccountType;
    use key_wallet::managed_account::ManagedCoreFundsAccount;

    let mut ctx = TestWalletContext::new_random();
    let bip44_address = ctx.receive_address.clone();

    let owner = Identifier::from([0xAA; 32]);
    let contact = Identifier::from([0xBB; 32]);
    let account_type = AccountType::DashpayReceivingFunds {
        index: 0,
        user_identity_id: owner.to_buffer(),
        friend_identity_id: contact.to_buffer(),
    };
    let account_xpub = crate::wallet::identity::crypto::dip14::derive_contact_xpub(
        &ctx.wallet,
        Network::Testnet,
        0,
        &owner,
        &contact,
    )
    .expect("derive contact xpub")
    .xpub;

    let mut managed = ManagedCoreFundsAccount::from_account(&key_wallet::Account {
        parent_wallet_id: Some(ctx.wallet.wallet_id),
        account_type,
        network: Network::Testnet,
        account_xpub,
        is_watch_only: false,
    });
    // DashPay accounts are non-standard: one external pool via
    // `next_address_with_info`, not the standard receive/change split.
    let contact_address = managed
        .next_address_with_info(Some(&account_xpub), true)
        .expect("contact receive address")
        .address;
    ctx.wallet
        .add_account(account_type, Some(account_xpub))
        .expect("add contact account to key collection");
    ctx.managed_wallet
        .accounts
        .insert_funds_bearing_account(managed)
        .expect("insert managed contact account");

    for (address, outputs, tag) in [
        (&bip44_address, bip44_outputs, 0..1),
        (&contact_address, contact_outputs, 1..2),
    ] {
        let funding_tx = Transaction::dummy(address, tag, outputs);
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
            "funding tx should be relevant (the contact account's pool must be monitored)"
        );
    }

    let signer = WalletSigner {
        wallet: ctx.wallet.clone(),
    };
    let generation = Arc::new(WalletGeneration::new());
    let info = PlatformWalletInfo {
        core_wallet: ctx.managed_wallet,
        generation: Arc::clone(&generation),
        identity_manager: IdentityManager::new(),
        tracked_asset_locks: BTreeMap::new(),
            dpns_name_states: BTreeMap::new(),
    };
    let mut wm = WalletManager::<PlatformWalletInfo>::new(Network::Testnet);
    let wallet_id = wm.insert_wallet(ctx.wallet, info).expect("insert wallet");
    (
        Arc::new(RwLock::new(wm)),
        wallet_id,
        generation,
        signer,
        account_type,
    )
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
            dpns_name_states: BTreeMap::new(),
    };

    let mut wm = WalletManager::<PlatformWalletInfo>::new(Network::Testnet);
    let wallet_id = wm.insert_wallet(ctx.wallet, info).expect("insert wallet");

    (Arc::new(RwLock::new(wm)), wallet_id, generation, signer)
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

/// Canonical all-`abandon` BIP-39 test vector. Fixed (not
/// `TestWalletContext::new_random`) so every key it derives is a stable golden —
/// which is what lets the signed-message tests pin an RFC6979-deterministic
/// signature and cross-check it against dashj.
#[cfg(test)]
pub(crate) const MESSAGE_SIGNING_TEST_MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon \
     abandon abandon abandon abandon abandon about";

/// Builds a testnet wallet manager from a KNOWN mnemonic with one derived BIP44
/// external address, and NO funding.
///
/// The unfunded counterpart of [`funded_wallet_manager`], for operations that
/// prove key ownership rather than move value — signing a message needs a
/// derivation path and a signer, never a UTXO. Returns the manager, the wallet
/// id, a soft signer over the wallet's seed, and that first receive address.
///
/// `#[cfg(test)]` rather than `test-utils`-gated: only this crate's own unit
/// tests consume it, so under the FFI crate's `test-utils` build it would
/// compile with no user and trip `dead_code`.
#[cfg(test)]
pub(crate) async fn mnemonic_wallet_manager(
    phrase: &str,
) -> (
    Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    WalletId,
    WalletSigner,
    dashcore::Address,
) {
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::wallet::ManagedWalletInfo;
    use key_wallet::{Language, Mnemonic};

    let mnemonic = Mnemonic::from_phrase(phrase, Language::English).expect("valid test mnemonic");
    let wallet = Wallet::from_mnemonic(
        mnemonic,
        Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("wallet construction from a known mnemonic");
    let mut managed_wallet =
        ManagedWalletInfo::from_wallet_with_name(&wallet, "SignMessage".to_string(), 0);

    let xpub = wallet
        .accounts
        .standard_bip44_accounts
        .get(&0)
        .expect("default options create BIP44 account 0")
        .account_xpub;
    // `true` registers the address in the external pool, which is what makes it
    // findable by `address_derivation_path` — an unregistered gap-limit address
    // is deliberately not signable.
    let receive_address = managed_wallet
        .first_bip44_managed_account_mut()
        .expect("managed BIP44 account 0")
        .next_receive_address(Some(&xpub), true)
        .expect("first BIP44 receive address");

    let signer = WalletSigner {
        wallet: wallet.clone(),
    };
    let info = PlatformWalletInfo {
        core_wallet: managed_wallet,
        generation: Arc::new(WalletGeneration::new()),
        identity_manager: IdentityManager::new(),
        tracked_asset_locks: BTreeMap::new(),
            dpns_name_states: BTreeMap::new(),
    };

    let mut wm = WalletManager::<PlatformWalletInfo>::new(Network::Testnet);
    let wallet_id = wm.insert_wallet(wallet, info).expect("insert wallet");

    (
        Arc::new(RwLock::new(wm)),
        wallet_id,
        signer,
        receive_address,
    )
}
