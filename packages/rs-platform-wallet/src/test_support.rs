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
use key_wallet::signer::{ExtendedPubKeySigner, Signer, SignerMethod};
use key_wallet::test_utils::TestWalletContext;
use key_wallet::transaction_checking::{BlockInfo, TransactionContext};
use key_wallet::{DerivationPath, Wallet};
use key_wallet_manager::WalletManager;
use tokio::sync::RwLock;

#[cfg(test)]
use crate::broadcaster::{BroadcastError, TransactionBroadcaster};
use crate::wallet::core::WalletBalance;
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
    Arc<WalletBalance>,
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
    Arc<WalletBalance>,
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

    let balance = Arc::new(WalletBalance::new());
    let info = PlatformWalletInfo {
        core_wallet: ctx.managed_wallet,
        balance: Arc::clone(&balance),
        identity_manager: IdentityManager::new(),
        tracked_asset_locks: BTreeMap::new(),
    };

    let mut wm = WalletManager::<PlatformWalletInfo>::new(Network::Testnet);
    let wallet_id = wm.insert_wallet(ctx.wallet, info).expect("insert wallet");

    (Arc::new(RwLock::new(wm)), wallet_id, balance, signer)
}

/// Funded SPV-backed Core wallet for downstream FFI lifecycle tests. The SPV
/// runtime is intentionally not started; abandon/free only need wallet state.
pub async fn funded_spv_core_wallet(
    account_type: StandardAccountType,
) -> (
    crate::CoreWallet<crate::broadcaster::SpvBroadcaster>,
    WalletSigner,
) {
    let (manager, wallet_id, balance, signer) = funded_wallet_manager(account_type).await;
    let spv = Arc::new(crate::spv::SpvRuntime::new(
        Arc::clone(&manager),
        Arc::new(crate::events::PlatformEventManager::new(Vec::new())),
    ));
    let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
    let broadcaster = Arc::new(crate::broadcaster::SpvBroadcaster::new(spv));
    (
        crate::CoreWallet::new(sdk, manager, wallet_id, broadcaster, balance),
        signer,
    )
}
