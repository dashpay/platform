//! The main PlatformWallet struct combining core, identity (+DashPay), and platform sub-wallets.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use dashcore::{OutPoint, Txid};
use dpp::prelude::CoreBlockHeight;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
#[cfg(feature = "shielded")]
use key_wallet::PlatformP2PKHAddress;
use key_wallet_manager::WalletManager;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use super::asset_lock::manager::AssetLockManager;
use super::asset_lock::tracked::TrackedAssetLock;
use super::core::{CoreWallet, WalletBalance, WalletGeneration};
use super::identity::{IdentityManager, IdentityWallet};
#[cfg(feature = "shielded")]
use super::persister::PrivatePaymentRestoreRetry;
use super::persister::WalletPersister;
#[cfg(feature = "shielded")]
use super::platform_addresses::merge_platform_payment_candidate_addresses;
use super::platform_addresses::PlatformAddressWallet;
#[cfg(feature = "shielded")]
use super::shielded::note_selection::ensure_positive_amount;
#[cfg(feature = "shielded")]
use super::shielded::operations::shield_fee_reserve_credits;
#[cfg(feature = "shielded")]
use crate::BlockTime;
// Phase 4d.3 deleted the `ShieldedWallet` wrapper; per-account
// keysets now live in `self.shielded_keys` directly. Spend
// operations source the shared commitment-tree store from
// `NetworkShieldedCoordinator` at call time.
use crate::broadcaster::SpvBroadcaster;
use crate::changeset::{
    ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use crate::error::PlatformWalletError;
use dash_sdk::platform::transition::put_settings::PutSettings;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::prelude::Identifier;
#[cfg(feature = "shielded")]
use {
    super::identity::network::{
        dashpay_contract, restore_journaled_private_payments, JournaledPrivatePayment,
    },
    super::shielded::operations::{
        discharge_read_back_private_payments, private_document_create_from_pool_with_metadata,
    },
    super::shielded::store::{
        PendingRedrive, PrivateContactPayment, PrivatePaymentEntry, PrivatePaymentRecord,
    },
    super::shielded::{
        FileBackedShieldedStore, NetworkShieldedCoordinator, OrchardKeySet, ShieldedStore,
    },
    crate::{
        AutoAcceptProofSource, ContactCryptoProvider, ContactRequest, EstablishedContact,
        PaymentStarted,
    },
    dash_sdk::dapi_grpc::platform::v0::ResponseMetadata,
    dash_sdk::platform::dashpay::PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
    dpp::data_contract::accessors::v0::DataContractV0Getters,
    dpp::platform_value::BinaryData,
    dpp::serialization::PlatformDeserializableTrusted,
    dpp::shielded::builder::OrchardProver,
    dpp::state_transition::private_document_create_from_shielded_pool_transition::accessors::PrivateDocumentCreateFromShieldedPoolTransitionAccessorsV0,
    dpp::state_transition::StateTransition,
    sha2::{Digest, Sha256},
};

/// Unique identifier for a wallet (32-byte hash).
pub type WalletId = [u8; 32];

/// What a private contact request reply is paid with, which
/// [`PlatformWallet::accept_contact_request_replying_in_kind`] asks for only
/// when the reply is private: the shielded coordinator, the seed the spend
/// authority is derived from (dropped once it is), the shielded account and
/// the prover.
#[cfg(feature = "shielded")]
pub struct PrivateReplyPayment<P, Seed> {
    pub coordinator: Arc<NetworkShieldedCoordinator>,
    pub seed: Seed,
    pub account: u32,
    pub prover: P,
}

/// Cached capacity snapshot for shielding a Platform Payment account.
///
/// The figures are computed from the same lexicographic address ordering and
/// fee-reserve rules used by [`PlatformWallet::shielded_shield_from_account`].
/// No DAPI request, signing, proof construction, or broadcast is performed.
#[cfg(feature = "shielded")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShieldedShieldPreflight {
    /// Whether the account can shield at least one credit.
    pub can_shield: bool,
    /// Sum of all funded candidate addresses in the payment account.
    pub account_balance_credits: Credits,
    /// Sum of the lexicographically earliest representable candidate set. It
    /// begins at the first address whose balance is strictly greater than
    /// the fee reserve ([`Self::fee_reserve_credits`]), omits later addresses
    /// below the versioned minimum input amount, and contains at most the
    /// versioned maximum address-input count.
    pub usable_balance_credits: Credits,
    /// Balance retained on input 0 for the transition fee — the versioned
    /// [`shield_fee_reserve_credits`] value the plan was computed with.
    pub fee_reserve_credits: Credits,
    /// Maximum claim accepted by the wallet's deterministic selector:
    /// `usable_balance_credits - fee_reserve_credits`, floored at zero.
    /// This is not a balance-optimized subset of every funded address; it
    /// preserves the established lexicographic selection policy within the
    /// protocol's input-count limit.
    pub max_shieldable_credits: Credits,
    /// Human-readable explanation when [`can_shield`](Self::can_shield) is
    /// false. Capacity exhaustion is a normal result, not a structural error.
    pub reason: Option<String>,
}

#[cfg(feature = "shielded")]
#[derive(Debug, Clone)]
struct ShieldedShieldInputPlan {
    preflight: ShieldedShieldPreflight,
    usable_candidates: Vec<(PlatformAddress, Credits)>,
    min_input_amount: Credits,
}

#[cfg(feature = "shielded")]
impl ShieldedShieldInputPlan {
    fn select_inputs(
        &self,
        amount: Credits,
    ) -> Result<BTreeMap<PlatformAddress, Credits>, PlatformWalletError> {
        if amount == 0 {
            return Err(PlatformWalletError::ShieldedBuildError(
                "amount must be > 0".to_string(),
            ));
        }

        if amount > self.preflight.max_shieldable_credits {
            let available = if self.usable_candidates.is_empty() {
                self.preflight.account_balance_credits
            } else {
                self.preflight.usable_balance_credits
            };
            return Err(PlatformWalletError::PlatformShieldCapacityExceeded {
                available,
                required: amount.saturating_add(self.preflight.fee_reserve_credits),
            });
        }

        let mut chosen = BTreeMap::new();
        let mut accumulated_claim = 0u64;
        for (index, (address, balance)) in self.usable_candidates.iter().enumerate() {
            if accumulated_claim >= amount {
                break;
            }
            let max_claim = if index == 0 {
                balance.saturating_sub(self.preflight.fee_reserve_credits)
            } else {
                *balance
            };
            let remaining = amount - accumulated_claim;
            let mut claim = max_claim.min(remaining);
            // Input 0 receives the shield fee later, so even a tiny base claim
            // clears the protocol minimum after `reserve_shield_fee_on_input_0`.
            // Every later input has no such fee addition. If its final greedy
            // residue is below the versioned minimum, request the minimum
            // instead. Shield inputs are maximum contributions: their sum may
            // exceed `amount`, and drive's reallocation leaves the excess on
            // the source address rather than increasing the shielded output.
            if index > 0 && claim > 0 && claim < self.min_input_amount {
                claim = self.min_input_amount;
            }
            if claim > 0 {
                chosen.insert(*address, claim);
                accumulated_claim = accumulated_claim
                    .checked_add(claim)
                    .ok_or(PlatformWalletError::InputSumOverflow)?;
            }
        }

        // `max_shieldable_credits` is derived from these exact candidates, so
        // this is an invariant guard rather than a second capacity rule.
        if accumulated_claim < amount {
            return Err(PlatformWalletError::PlatformShieldCapacityExceeded {
                available: accumulated_claim,
                required: amount,
            });
        }

        Ok(chosen)
    }
}

#[cfg(feature = "shielded")]
fn checked_credit_sum<'a>(
    mut balances: impl Iterator<Item = &'a Credits>,
) -> Result<Credits, PlatformWalletError> {
    balances.try_fold(0u64, |sum, balance| {
        sum.checked_add(*balance)
            .ok_or(PlatformWalletError::InputSumOverflow)
    })
}

/// Analyze funded Platform addresses once for both preflight and execution.
///
/// The representable set is the lexicographically earliest usable prefix,
/// capped at `max_address_inputs`. Deliberately retaining the wallet's existing
/// ordering policy avoids silently replacing earlier addresses with later,
/// larger balances; consequently preflight Max means the maximum accepted by
/// this deterministic policy, not a globally balance-optimized subset.
///
/// `fee_reserve` is the versioned [`shield_fee_reserve_credits`] value; it is
/// the balance input 0 must retain unclaimed so execution can deduct the
/// actual metered fee from that input's residue (`DeductFromInput(0)`).
#[cfg(feature = "shielded")]
fn plan_shield_inputs(
    mut candidates: Vec<(PlatformAddress, Credits)>,
    fee_reserve: Credits,
    min_input_amount: Credits,
    max_address_inputs: usize,
) -> Result<ShieldedShieldInputPlan, PlatformWalletError> {
    candidates.sort_by_key(|(address, _)| *address);

    let account_balance_credits =
        checked_credit_sum(candidates.iter().map(|(_, balance)| balance))?;
    let viable_input_0 = candidates
        .iter()
        .position(|(_, balance)| *balance > fee_reserve);
    let usable_candidates: Vec<(PlatformAddress, Credits)> = viable_input_0
        .map(|index| {
            // Keep the fee-bearing input 0 regardless of its post-reserve base
            // capacity: the shield fee is added to its requested claim before
            // structure validation. Later addresses get no fee addition, so a
            // full balance below `min_input_amount` can never form a valid
            // input and must not inflate preflight capacity. Finally, truncate
            // the deterministic sequence before deriving capacity so Max can
            // always be represented by a protocol-valid input count.
            std::iter::once(candidates[index])
                .chain(
                    candidates[index + 1..]
                        .iter()
                        .copied()
                        .filter(|(_, balance)| *balance >= min_input_amount),
                )
                .take(max_address_inputs)
                .collect()
        })
        .unwrap_or_default();
    let usable_balance_credits =
        checked_credit_sum(usable_candidates.iter().map(|(_, balance)| balance))?;
    let max_shieldable_credits = usable_balance_credits.saturating_sub(fee_reserve);
    let can_shield = max_shieldable_credits > 0;
    let reason = (!can_shield).then(|| {
        format!(
            "Platform payment account has {account_balance_credits} credits, but no address can retain the {fee_reserve}-credit shield fee reserve"
        )
    });

    Ok(ShieldedShieldInputPlan {
        preflight: ShieldedShieldPreflight {
            can_shield,
            account_balance_credits,
            usable_balance_credits,
            fee_reserve_credits: fee_reserve,
            max_shieldable_credits,
            reason,
        },
        usable_candidates,
        min_input_amount,
    })
}

/// One confirmed spend of a tracked asset lock's input, as the
/// double-spend screen last saw it in live transaction history.
///
/// Session-scoped memory, never persisted and never restored: it exists
/// because `apply_chain_lock` EVICTS a record from history the moment a
/// chainlock buries it (default `keep-finalized-transactions = OFF`), and
/// a retry after that eviction would otherwise find nothing and fall back
/// into the proof wait the screen exists to prevent. The screen writes
/// entries when it observes a confirmed spender, retracts them when live
/// history re-observes that spender unconfirmed (a reorg demotes the
/// record in place), and keeps reporting an entry whose record has LEFT
/// history: promotion-eviction is the only path that removes a record.
///
/// The entry carries no finality: an eviction attests a height-based
/// promotion, not that the spender's block is on the finalized branch, so
/// every verdict the screen builds from this memory is the provisional
/// one. See `wallet::asset_lock::sync::recovery`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObservedInputConflict {
    /// The confirmed transaction the screen saw spending the input.
    pub spender: Txid,
    /// The block height it was seen at.
    pub height: CoreBlockHeight,
}

/// Consolidated mutable state for a platform wallet.
///
/// Lives inside `WalletManager<PlatformWalletInfo>.wallet_infos`. The `Wallet`
/// key material is in `WalletManager.wallets` — NOT inside this struct.
///
/// The per-generation state (lock-free balance + lifecycle gate) is stored as
/// `Arc<WalletGeneration>`; `Arc::ptr_eq` on it is this wallet's generation identity.
pub struct PlatformWalletInfo {
    /// Core wallet metadata, accounts, UTXOs, balances.
    /// Delegates `WalletInfoInterface` methods.
    pub core_wallet: ManagedWalletInfo,
    /// This wallet generation's shared state: the lock-free balance for UI reads
    /// (updated from `ManagedWalletInfo` after each SPV block/mempool processing
    /// and RPC refresh) and the generation's lifecycle gate.
    ///
    /// Deliberately `pub(crate)`, not `pub`: this `Arc` *is* the generation
    /// identity that `Arc::ptr_eq` compares, and `PlatformWalletInfo` is
    /// reachable mutably from outside the crate through
    /// [`PlatformWallet::state_mut`] / [`PlatformWallet::state_mut_blocking`].
    /// A public field would let safe downstream code drop a fresh `Arc` in here
    /// while `PlatformWallet` and `CoreWallet` keep the original, splitting the
    /// identity: `is_current_generation()` would then reject the still-live
    /// wallet, generation-bound reservation cleanup would become a no-op, and
    /// teardown would exclude through a different lifecycle gate than the
    /// payment operations it has to fence. Read it through
    /// [`PlatformWallet::generation`]; it is assigned only at construction.
    pub(crate) generation: Arc<WalletGeneration>,
    pub identity_manager: IdentityManager,
    pub tracked_asset_locks: BTreeMap<OutPoint, TrackedAssetLock>,
    /// Session-scoped double-spend evidence for tracked asset locks — see
    /// [`ObservedInputConflict`]. Interior mutability because the screen
    /// runs under the manager's read lock; a poisoned mutex degrades to
    /// "no memory" rather than failing a resume.
    pub observed_input_conflicts: std::sync::Mutex<BTreeMap<OutPoint, ObservedInputConflict>>,
    /// Outpoints of this wallet's coins that a ChainLocked transaction
    /// spent, kept for the records the ChainLock dropped. With
    /// `keep-finalized-transactions` off (the default) key-wallet drops a
    /// record once a ChainLock covers it and keeps only its txid, so the
    /// spender's inputs are gone with it; this is what is left of them, for
    /// the store reconcile's spend verdicts
    /// ([`classify_outpoints`](crate::manager::accessors::classify_outpoints)).
    /// A ChainLocked spend is final: no fork the engine follows drops it.
    /// In memory only, like key-wallet's own finalized txids: after a
    /// restart neither the spender nor the funding is known, and the
    /// verdict is `Unknown`.
    pub finalized_spends: std::collections::HashSet<OutPoint>,
    /// DPNS name states with sale price (username marketplace), keyed by
    /// domain document id. Session-lifetime working set for the
    /// marketplace sync/orchestration ops; the durable copy is the
    /// host-side persister mirror fed by
    /// [`DpnsNameStateChangeSet`](crate::changeset::DpnsNameStateChangeSet).
    pub dpns_name_states: BTreeMap<Identifier, crate::changeset::DpnsNameStateEntry>,
    /// Durable half of the DashPay coreHeight backfill guard: which receival
    /// contacts the historical rescan already covers, from which height each,
    /// and how far the rescan reached. Restored at load from
    /// [`ClientWalletStartState::dashpay_backfill`](crate::changeset::ClientWalletStartState::dashpay_backfill)
    /// and written by `reconcile_dashpay_rescan` on the round that lowers the
    /// cursor. The in-memory half is `DashPayState::rescan_triggered`; see
    /// [`DashPayBackfillRecord`](crate::changeset::DashPayBackfillRecord)
    /// for how the two compose.
    pub dashpay_backfill: crate::changeset::DashPayBackfillRecord,
    /// Orders this wallet's queued `SyncHeightAdvanced` events against
    /// in-memory rewinds of its scan cursor; see
    /// [`RewindBarrier`](crate::changeset::core_bridge::RewindBarrier). In
    /// memory only.
    pub rewind_barrier: crate::changeset::core_bridge::RewindBarrier,
    /// Outpoints `core_wallet` locked during a transaction check that are not
    /// persisted yet.
    ///
    /// Every ProRegTx the wallet processes locks its collateral, relevant or
    /// not, and no `WalletEvent` carries that lock. The check queues it here
    /// and the wallet-event adapter drains the queue into the next changeset
    /// it stores for this wallet. Interior mutability because the adapter
    /// projects under the manager's read lock; a poisoned mutex keeps its
    /// contents.
    pub(crate) pending_outpoint_locks: std::sync::Mutex<BTreeSet<OutPoint>>,
}

/// A platform wallet that combines core UTXO functionality with identity management.
///
/// This is SPV-free. It needs only key material and an `Sdk`.
/// For SPV support, use [`PlatformWalletManager`](crate::manager::PlatformWalletManager).
///
/// # Cloning
///
/// `PlatformWallet` is cheaply cloneable (a few atomic increments). A clone is a
/// **shared handle** to the same mutable state — not an independent copy. All
/// clones see the same UTXOs, balances, and identities through the shared
/// `WalletManager` lock.
pub struct PlatformWallet {
    wallet_id: WalletId,
    pub(crate) sdk: Arc<dash_sdk::Sdk>,
    pub(crate) wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    // Sub-wallets that hold a broadcaster are monomorphized with
    // `SpvBroadcaster` — the only production broadcaster in use.
    // Swapping this out to another broadcaster is a three-line flip
    // right here plus the `new()` signature below; the sub-wallet
    // definitions themselves stay untouched.
    pub(crate) core: CoreWallet<SpvBroadcaster>,
    pub(crate) identity: IdentityWallet<SpvBroadcaster>,
    pub(crate) platform: PlatformAddressWallet,
    /// Shared asset lock manager.
    pub(crate) asset_locks: Arc<AssetLockManager<SpvBroadcaster>>,
    /// Per-wallet persistence handle.
    persister: WalletPersister,
    /// This generation's shared state, cloned from `PlatformWalletInfo.generation`.
    pub(crate) generation: Arc<WalletGeneration>,
    /// Per-account Orchard keysets, populated by [`bind_shielded`].
    /// `None` until bind has run; remains `None` for `WatchOnly`
    /// / `ExternalSignable` wallets that have never had a
    /// resolver-driven bind. The `RwLock` lets read paths (the
    /// shielded sync coordinator, balance/address accessors)
    /// observe the bound state without serializing against
    /// unrelated wallet writes.
    ///
    /// Sync / spend operations source the shared
    /// commitment-tree store from
    /// [`NetworkShieldedCoordinator`] (one SQLite handle per
    /// network) rather than per-wallet, so all this slot holds
    /// is the per-account viewing-grade material (FVK / IVK /
    /// OVK / default address), mirrored on the coordinator's
    /// account registry. No `SpendAuthorizingKey` is resident:
    /// spend operations re-derive the full `OrchardKeySet` from
    /// the caller-supplied wallet seed for the duration of the
    /// spend call only, then drop it.
    ///
    /// [`bind_shielded`]: Self::bind_shielded
    /// [`NetworkShieldedCoordinator`]: crate::wallet::shielded::NetworkShieldedCoordinator
    #[cfg(feature = "shielded")]
    pub(crate) shielded_keys:
        Arc<RwLock<Option<std::collections::BTreeMap<u32, super::shielded::AccountViewingKeys>>>>,
    /// Per-wallet single-flight guard for shield-class operations
    /// (Type 15, and the identity-side Types 21 and 22). Two concurrent
    /// `shield` calls on one wallet would each fetch the same address
    /// nonce and build with `nonce + 1`, so the second to reach
    /// drive-abci is rejected as a replay after a ~30 s proof. Holding
    /// this across fetch → build → broadcast serializes the double-tap /
    /// retry-while-proving case. The identity-side operations also apply
    /// the proof-attested absolute identity balance, so holding it across
    /// their waits keeps those writes in execution order. `Arc` so cloned
    /// wallet handles share the one lock.
    #[cfg(feature = "shielded")]
    pub(crate) shield_guard: Arc<tokio::sync::Mutex<()>>,
    /// Set once this wallet has been removed from the manager, to stop
    /// a handle that outlives the removal from binding shielded state
    /// back onto the coordinator. Callers resolve an
    /// `Arc<PlatformWallet>` and then hold it across a bind that can
    /// take arbitrarily long (it may resolve a mnemonic through the
    /// host), so a removal can complete in the middle; without this
    /// flag the bind would re-register the wallet the removal just
    /// detached, and the next sync pass would re-fetch and re-persist
    /// shielded history the host believes it deleted. `Arc` so cloned
    /// wallet handles observe the one flag.
    #[cfg(feature = "shielded")]
    pub(crate) shielded_detached: Arc<std::sync::atomic::AtomicBool>,
    /// This generation's restorations of its private contact request
    /// payments from a shielded store's journal: the store one last succeeded
    /// from, and the store a deferred one is queued for
    /// ([`Self::restore_private_payments_or_defer`]). `Arc` so cloned wallet
    /// handles share them.
    #[cfg(feature = "shielded")]
    private_payment_restores: Arc<std::sync::Mutex<PrivatePaymentRestores>>,
}

impl PlatformWallet {
    /// Access the core wallet (balance, UTXOs, addresses).
    pub fn core(&self) -> &CoreWallet<SpvBroadcaster> {
        &self.core
    }

    /// Access the identity wallet.
    ///
    /// Covers both identity-lifecycle and DashPay-contract operations in
    /// one facade, since the underlying `ManagedIdentity` state is shared
    /// between them. Keeps the single `SpvBroadcaster`
    /// specialization the rest of this wallet uses.
    pub fn identity(&self) -> &IdentityWallet<SpvBroadcaster> {
        &self.identity
    }

    /// Access the platform address wallet.
    pub fn platform(&self) -> &PlatformAddressWallet {
        &self.platform
    }

    /// Access the shared asset lock manager.
    pub fn asset_locks(&self) -> &Arc<AssetLockManager<SpvBroadcaster>> {
        &self.asset_locks
    }

    /// Get the wallet ID.
    pub fn wallet_id(&self) -> WalletId {
        self.wallet_id
    }

    /// The Dash network this wallet operates on. Delegates to the
    /// asset-lock manager, which is the single source of truth.
    pub fn network(&self) -> dashcore::Network {
        self.asset_locks.network()
    }

    /// Get a reference to the SDK.
    pub fn sdk(&self) -> &dash_sdk::Sdk {
        &self.sdk
    }

    /// Clone the underlying `Arc<dash_sdk::Sdk>` so callers (e.g. FFI
    /// async blocks moved onto a worker runtime) can hold an
    /// independently-owned SDK handle without keeping the
    /// `PlatformWallet` borrow alive.
    pub fn sdk_arc(&self) -> Arc<dash_sdk::Sdk> {
        Arc::clone(&self.sdk)
    }

    /// Get a reference to the shared wallet manager lock.
    pub fn wallet_manager(&self) -> &Arc<RwLock<WalletManager<PlatformWalletInfo>>> {
        &self.wallet_manager
    }

    /// Get the lock-free balance for UI reads.
    pub fn balance(&self) -> &WalletBalance {
        self.generation.balance()
    }

    /// This wallet's [`WalletGeneration`] `Arc` — its generation identity and
    /// lifecycle gate. See [`CoreWallet::is_same_generation`].
    pub fn generation(&self) -> &Arc<WalletGeneration> {
        &self.generation
    }

    /// Get a reference to the per-wallet persistence handle.
    ///
    /// Callers that hold a `&PlatformWallet` and need to invoke mutation
    /// methods on [`ManagedIdentity`] (e.g. `set_dashpay_profile`,
    /// `record_dashpay_payment`, `add_identity`) must pass this persister
    /// so those methods can persist the resulting changeset immediately.
    pub fn persister(&self) -> &WalletPersister {
        &self.persister
    }

    /// Read-lock the wallet manager and return a guard that derefs to this
    /// wallet's `PlatformWalletInfo`.
    pub async fn state(&self) -> WalletStateReadGuard<'_> {
        WalletStateReadGuard {
            guard: self.wallet_manager.read().await,
            wallet_id: self.wallet_id,
        }
    }

    /// Write-lock the wallet manager and return a guard that derefs to this
    /// wallet's `PlatformWalletInfo` (with `DerefMut`).
    pub async fn state_mut(&self) -> WalletStateWriteGuard<'_> {
        WalletStateWriteGuard {
            guard: self.wallet_manager.write().await,
            wallet_id: self.wallet_id,
        }
    }

    /// Blocking read-lock.
    pub fn state_blocking(&self) -> WalletStateReadGuard<'_> {
        WalletStateReadGuard {
            guard: self.wallet_manager.blocking_read(),
            wallet_id: self.wallet_id,
        }
    }

    /// Blocking write-lock.
    ///
    /// Uses `tokio::sync::RwLock::blocking_write` — must NOT be
    /// called from within a tokio async context. Exists so sync
    /// callers (e.g. SPV-driven transaction processing) can reach
    /// mutation methods that require the wallet-manager write lock.
    pub fn state_mut_blocking(&self) -> WalletStateWriteGuard<'_> {
        WalletStateWriteGuard {
            guard: self.wallet_manager.blocking_write(),
            wallet_id: self.wallet_id,
        }
    }

    // -----------------------------------------------------------------
    // Address-funded identity flows
    //
    // Composite orchestration: each flow spends or credits platform
    // addresses through the identity sub-wallet, then routes the
    // proof-attested `AddressInfos` through the platform-address
    // sub-wallet's shared reconciliation seam
    // (`PlatformAddressWallet::reconcile_address_infos`) so displayed
    // balances and the next input selection reflect on-chain reality
    // without waiting for the next BLAST sync round. Only this struct
    // owns both sub-wallets, which is why the composition lives here.
    // -----------------------------------------------------------------

    /// Top up an existing identity's credit balance by spending platform
    /// address balances, then reconcile the spent addresses' local
    /// balances and nonces from the proof-attested post-spend
    /// `AddressInfos`.
    ///
    /// See [`IdentityWallet::top_up_from_addresses`] for the identity-side
    /// semantics. Reconciliation failures are logged inside the seam
    /// rather than propagated — Platform already accepted the top-up, and
    /// a later sync reconciles.
    ///
    /// Returns the identity's new credit balance.
    pub async fn top_up_from_addresses<S: Signer<PlatformAddress> + Send + Sync>(
        &self,
        identity_id: &Identifier,
        inputs: BTreeMap<PlatformAddress, Credits>,
        address_signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Credits, PlatformWalletError> {
        let (address_infos, new_balance, proof_height) = self
            .identity
            .top_up_from_addresses(identity_id, inputs, address_signer, settings)
            .await?;
        // The reconciled absolutes are pinned at the proof's block height
        // (`AddressFunds::as_of_height`), so the sync's delta replay can
        // never re-apply this transition's on-chain ops on top of them.
        self.platform
            .reconcile_address_infos(&address_infos, proof_height, "identity top-up")
            .await;
        Ok(new_balance)
    }

    /// Register a new identity funded by platform-address balances, then
    /// reconcile the spent funding addresses' local balances and nonces
    /// from the proof-attested post-spend `AddressInfos`.
    ///
    /// See [`IdentityWallet::register_from_addresses`] for the
    /// identity-side semantics (placeholder construction, signer
    /// responsibilities, local-manager registration). Reconciliation
    /// failures are logged inside the seam rather than propagated —
    /// Platform already accepted the registration, and a later sync
    /// reconciles.
    ///
    /// Returns the registered identity.
    #[allow(clippy::too_many_arguments)]
    pub async fn register_from_addresses<IS, AS>(
        &self,
        identity: &Identity,
        inputs: BTreeMap<PlatformAddress, Credits>,
        output: Option<(PlatformAddress, Credits)>,
        identity_index: u32,
        identity_signer: &IS,
        input_address_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<Identity, PlatformWalletError>
    where
        IS: Signer<IdentityPublicKey>,
        AS: Signer<PlatformAddress> + Send + Sync,
    {
        // The optional refund-style `output` is credited on-chain via an
        // `AddBalanceToAddress` DELTA at the proof's block height. The
        // reconciled absolutes carry that height as their pin
        // (`AddressFunds::as_of_height`), so the sync's delta replay drops
        // the credit instead of re-applying it on top (ADDR-09).
        let (registered_identity, address_infos, proof_height) = self
            .identity
            .register_from_addresses(
                identity,
                inputs,
                output,
                identity_index,
                identity_signer,
                input_address_signer,
                settings,
            )
            .await?;
        self.platform
            .reconcile_address_infos(&address_infos, proof_height, "identity registration")
            .await;
        Ok(registered_identity)
    }

    /// Transfer credits from an identity to platform addresses, then
    /// reconcile any wallet-owned recipient addresses' local balances
    /// from the proof-attested `AddressInfos` (recipients belonging to
    /// third parties are skipped by the seam).
    ///
    /// See [`IdentityWallet::transfer_credits_to_addresses_with_external_signer`]
    /// for the identity-side semantics. Reconciliation failures are
    /// logged inside the seam rather than propagated — Platform already
    /// accepted the transfer, and a later sync reconciles.
    ///
    /// Returns the sender identity's new credit balance.
    pub async fn transfer_credits_to_addresses_with_external_signer<S>(
        &self,
        identity_id: &Identifier,
        recipient_addresses: BTreeMap<PlatformAddress, Credits>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Credits, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        // Every recipient is credited on-chain via an `AddBalanceToAddress`
        // DELTA at the proof's block height — and the primary use of this
        // flow is consolidating identity credits into the wallet's OWN
        // platform addresses. The reconciled absolutes carry that height
        // as their pin (`AddressFunds::as_of_height`), so the sync's delta
        // replay drops the credit instead of re-applying it on top
        // (ADDR-09).
        let (address_infos, new_balance, proof_height) = self
            .identity
            .transfer_credits_to_addresses_with_external_signer(
                identity_id,
                recipient_addresses,
                signer,
                settings,
            )
            .await?;
        self.platform
            .reconcile_address_infos(&address_infos, proof_height, "credit transfer to addresses")
            .await;
        Ok(new_balance)
    }

    /// Non-blocking read-lock. Returns `None` if the lock is currently
    /// held by a writer, or cannot be acquired without parking the
    /// thread. Safe to call from any context — never panics, never
    /// blocks. Intended for sync callers that run on a tokio runtime
    /// thread (e.g. egui UI render callbacks) where blocking variants
    /// would panic and async variants cannot be awaited.
    pub fn try_state(&self) -> Option<WalletStateReadGuard<'_>> {
        self.wallet_manager
            .try_read()
            .ok()
            .map(|guard| WalletStateReadGuard {
                guard,
                wallet_id: self.wallet_id,
            })
    }

    /// Non-blocking write-lock. Returns `None` if the lock is currently
    /// held by any reader or writer. Same safety properties as
    /// [`try_state`]: never panics, never blocks.
    pub fn try_state_mut(&self) -> Option<WalletStateWriteGuard<'_>> {
        self.wallet_manager
            .try_write()
            .ok()
            .map(|guard| WalletStateWriteGuard {
                guard,
                wallet_id: self.wallet_id,
            })
    }

    /// Construct a PlatformWallet from a WalletManager that already contains
    /// the wallet. The wallet must have been inserted into the WalletManager
    /// before calling this.
    // Every argument is a shared handle the manager owns; a builder would
    // only move the same nine names behind a struct.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        sdk: Arc<dash_sdk::Sdk>,
        wallet_id: WalletId,
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        generation: Arc<WalletGeneration>,
        lock_notify: Arc<tokio::sync::Notify>,
        persister: Arc<dyn PlatformWalletPersistence>,
        broadcaster: Arc<SpvBroadcaster>,
        sync_fault: Arc<std::sync::atomic::AtomicBool>,
        durable_cursors: crate::changeset::DurableCursors,
    ) -> Self {
        // Build the per-wallet persister handle once and share it with
        // the sub-wallets that need to queue their own changesets
        // (currently just `AssetLockManager` — see Item 8 sub-step 1a).
        let wallet_persister = WalletPersister::new(wallet_id, persister);

        let core = CoreWallet::new(
            Arc::clone(&sdk),
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::clone(&broadcaster),
            Arc::clone(&generation),
        );

        // Asset-lock broadcaster is pinned to `SpvBroadcaster`; the
        // identity wallet's DashPay payment broadcaster uses a clone
        // of the same Arc since production currently runs one
        // broadcaster type across the stack.
        let dashpay_broadcaster = Arc::clone(&broadcaster);

        let mined_height_locator = Arc::new(
            crate::wallet::asset_lock::sync::locate::DapiSpvLocator::new(
                Arc::clone(&sdk),
                Arc::clone(&broadcaster)
                    as Arc<dyn crate::wallet::asset_lock::sync::locate::BlockHeaderSource>,
            ),
        );
        let asset_locks = Arc::new(
            AssetLockManager::new(
                Arc::clone(&sdk),
                Arc::clone(&wallet_manager),
                wallet_id,
                lock_notify,
                broadcaster,
                wallet_persister.clone(),
            )
            .with_mined_height_locator(mined_height_locator),
        );

        let identity: IdentityWallet<SpvBroadcaster> = IdentityWallet {
            sdk: Arc::clone(&sdk),
            wallet_manager: Arc::clone(&wallet_manager),
            wallet_id,
            asset_locks: Arc::clone(&asset_locks),
            persister: wallet_persister.clone(),
            sync_fault,
            durable_cursors,
            generation: Arc::clone(&generation),
            broadcaster: dashpay_broadcaster,
            // DashPay write helper: forwards to the live SDK, erasing its
            // generic write signatures behind concrete by-value methods.
            sdk_writer: Arc::new(
                crate::wallet::identity::network::sdk_writer::SdkWriter::new(Arc::clone(&sdk)),
            ),
            dpns_operation_gate: Arc::new(tokio::sync::Mutex::new(())),
            dpns_sync_progress: Arc::new(std::sync::Mutex::new(BTreeMap::new())),
        };

        let platform = PlatformAddressWallet::new(
            Arc::clone(&sdk),
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::clone(&asset_locks),
            wallet_persister.clone(),
        );

        Self {
            wallet_id,
            sdk,
            wallet_manager,
            core,
            identity,
            platform,
            asset_locks,
            persister: wallet_persister,
            generation,
            #[cfg(feature = "shielded")]
            shielded_keys: Arc::new(RwLock::new(None)),
            #[cfg(feature = "shielded")]
            shield_guard: Arc::new(tokio::sync::Mutex::new(())),
            #[cfg(feature = "shielded")]
            shielded_detached: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            #[cfg(feature = "shielded")]
            private_payment_restores: Arc::default(),
        }
    }

    /// Bind a shielded (Orchard) sub-wallet to this `PlatformWallet`.
    ///
    /// Derives ZIP-32 Orchard keys for every entry of `accounts`
    /// from `seed` (a 32-252 byte BIP-39 seed; see
    /// [`SpendingKey::from_zip32_seed`]) and installs the
    /// viewing-grade half (FVK / IVK / OVK / default address) on
    /// this handle and the coordinator's registry. The caller is
    /// responsible for sourcing the seed (e.g. via the host
    /// `MnemonicResolverHandle`) and for zeroizing it once this
    /// call returns. The seed is not retained, and neither is any
    /// `SpendAuthorizingKey` — spend operations re-derive it from
    /// a caller-supplied seed per call.
    ///
    /// The derived per-account viewing keys are queued to the host
    /// persister (as raw 96-byte FVK encodings) so later launches
    /// can rebind via [`bind_shielded_from_persisted`] without
    /// resolving the mnemonic at all.
    ///
    /// Idempotent: a second call replaces the previously-bound
    /// shielded wallet (e.g. after a network switch).
    ///
    /// `accounts` must be non-empty; pass `&[0]` for the
    /// single-account default.
    ///
    /// [`bind_shielded_from_persisted`]: Self::bind_shielded_from_persisted
    /// [`SpendingKey::from_zip32_seed`]: grovedb_commitment_tree::SpendingKey::from_zip32_seed
    #[cfg(feature = "shielded")]
    pub async fn bind_shielded(
        &self,
        seed: &[u8],
        accounts: &[u32],
        coordinator: &Arc<crate::wallet::shielded::NetworkShieldedCoordinator>,
    ) -> Result<(), PlatformWalletError> {
        use super::shielded::{AccountViewingKeys, OrchardKeySet, SubwalletId};
        let required = crate::changeset::PersistenceCapabilities::SHIELDED_FVK_RESTART;
        let capabilities = self.persister.persistence_capabilities();
        if !capabilities.contains(required) {
            let missing = capabilities.missing(required);
            return Err(PlatformWalletError::Persistence(format!(
                "shielded seedless restart requires persistence capabilities {:?} \
                 (missing mask 0x{:x})",
                missing.names(),
                missing.bits(),
            )));
        }
        if accounts.is_empty() {
            return Err(PlatformWalletError::ShieldedKeyDerivation(
                "shielded wallet requires at least one account".to_string(),
            ));
        }
        // Before anything is read or written: a wallet the manager has
        // dropped must not persist viewing keys the host can no longer
        // delete (see `ensure_shielded_attached`).
        self.ensure_shielded_attached()?;
        // Sampled before the load below so the install can tell that the
        // snapshot predates a Clear.
        let snapshot_generation = coordinator.clear_generation();
        let network = self.sdk.network;
        let mut account_views: std::collections::BTreeMap<u32, AccountViewingKeys> =
            std::collections::BTreeMap::new();
        for &account in accounts {
            // `accounts` may contain duplicates; the BTreeMap
            // dedups by definition. The full keyset (with its
            // `SpendAuthorizingKey`) is dropped at the end of
            // this iteration — only the viewing half survives.
            let ks = OrchardKeySet::from_seed(seed, network, account)?;
            account_views.insert(account, ks.viewing_keys());
        }

        // Refuse to overwrite a persisted viewing key with a different
        // one. The durable notes, activity and sync watermark this
        // wallet already has are keyed by `(wallet_id, account_index)`
        // alone — nothing records which key produced them — so a row
        // written under the old key is indistinguishable from one
        // written under the new. Upserting the new key would leave the
        // old key's notes attributed to it (unspendable, yet counted)
        // and its watermark in force (hiding the new key's own
        // history). No legitimate flow re-keys an account in place, so
        // treat it like the malformed-row case below: surface it rather
        // than silently mixing two keys' state. The recovery is a
        // shielded Clear, which drops both sides at once.
        let start = self.persister.load().map_err(|e| {
            PlatformWalletError::ShieldedBuildError(format!(
                "persister load failed while binding shielded viewing keys: {e}"
            ))
        })?;
        for (account, views) in &account_views {
            let id = SubwalletId::new(self.wallet_id, *account);
            if let Some(persisted) = start.shielded.viewing_keys.get(&id) {
                if persisted.as_slice() != views.to_fvk_bytes().as_slice() {
                    return Err(PlatformWalletError::ShieldedKeyDerivation(format!(
                        "persisted shielded viewing key for account {account} differs from the \
                         one derived from this seed; clear shielded state before binding a \
                         re-keyed account"
                    )));
                }
            }
        }

        // Persist the viewing keys while the seed is legitimately present, so
        // every later launch can rebind seedlessly. Do not install the in-memory
        // keys when persistence rejects the write: that would advertise a
        // working shielded bind that cannot survive restart.
        let mut cs = crate::changeset::ShieldedChangeSet::default();
        for (account, views) in &account_views {
            cs.record_viewing_key(
                SubwalletId::new(self.wallet_id, *account),
                views.to_fvk_bytes(),
            );
        }
        self.persister
            .store(crate::changeset::PlatformWalletChangeSet {
                shielded: Some(cs),
                ..Default::default()
            })
            .map_err(|e| {
                PlatformWalletError::Persistence(format!(
                    "failed to persist shielded viewing keys before bind: {e}"
                ))
            })?;

        // Hand the snapshot loaded above to the install step. It predates
        // the viewing-key upsert just made, which is safe precisely
        // because the loop above proved the derived keys equal the
        // persisted ones: the rows the restore filters on are unchanged
        // by that write.
        self.install_shielded_views(account_views, coordinator, start, snapshot_generation)
            .await
    }

    /// Bind the shielded sub-wallet from viewing keys persisted by a
    /// prior seed-backed [`bind_shielded`](Self::bind_shielded),
    /// without touching the wallet seed.
    ///
    /// Reads the persister's start-state snapshot for this wallet's
    /// per-account FVK rows and installs the reconstructed
    /// viewing-grade material exactly like a seed bind. Returns
    /// `Ok(false)` — with no state change — when the persister has no
    /// viewing key for at least one entry of `accounts` (first bind
    /// after create/import, or legacy persistence predating viewing-key
    /// rows); the caller then falls back to the seed path. A persisted
    /// row that fails to decode is an error, not a fallback — silent
    /// re-resolution would mask persistence corruption.
    #[cfg(feature = "shielded")]
    pub async fn bind_shielded_from_persisted(
        &self,
        accounts: &[u32],
        coordinator: &Arc<crate::wallet::shielded::NetworkShieldedCoordinator>,
    ) -> Result<bool, PlatformWalletError> {
        use super::shielded::{AccountViewingKeys, SubwalletId};
        let required = crate::changeset::PersistenceCapabilities::SHIELDED_FVK_RESTART;
        let capabilities = self.persister.persistence_capabilities();
        if !capabilities.contains(required) {
            let missing = capabilities.missing(required);
            return Err(PlatformWalletError::Persistence(format!(
                "shielded seedless restart requires persistence capabilities {:?} \
                 (missing mask 0x{:x})",
                missing.names(),
                missing.bits(),
            )));
        }
        if accounts.is_empty() {
            return Err(PlatformWalletError::ShieldedKeyDerivation(
                "shielded wallet requires at least one account".to_string(),
            ));
        }
        self.ensure_shielded_attached()?;
        // Sampled before the load so the install can tell that the
        // snapshot predates a Clear.
        let snapshot_generation = coordinator.clear_generation();
        let start = self.persister.load().map_err(|e| {
            PlatformWalletError::ShieldedBuildError(format!(
                "persister load failed while rebinding shielded viewing keys: {e}"
            ))
        })?;
        let mut account_views: std::collections::BTreeMap<u32, AccountViewingKeys> =
            std::collections::BTreeMap::new();
        for &account in accounts {
            let id = SubwalletId::new(self.wallet_id, account);
            let Some(fvk_bytes) = start.shielded.viewing_keys.get(&id) else {
                return Ok(false);
            };
            let fvk_bytes: &[u8; 96] = fvk_bytes.as_slice().try_into().map_err(|_| {
                PlatformWalletError::ShieldedKeyDerivation(format!(
                    "persisted viewing key for account {account} is {} bytes, expected 96",
                    fvk_bytes.len()
                ))
            })?;
            account_views.insert(account, AccountViewingKeys::from_fvk_bytes(fvk_bytes)?);
        }
        // Hand the already-loaded snapshot to the install step so the
        // restore doesn't pay a second full persister load.
        self.install_shielded_views(account_views, coordinator, start, snapshot_generation)
            .await?;
        Ok(true)
    }

    /// Shared tail of the two bind paths: store the viewing-grade
    /// map on this handle, replace this wallet's registration on
    /// the coordinator, and rehydrate persisted notes / watermarks
    /// from `start` — the snapshot both callers have already loaded
    /// (one to reconstruct the viewing keys, the other to check them
    /// against the seed), so the restore never pays a second load.
    ///
    /// Runs as one coordinator install transaction, so a concurrent
    /// bind, wallet removal or Clear either happens entirely before or
    /// entirely after it — never between the key-slot write and the
    /// registration (which would leave sync decrypting under one key
    /// while addresses and spends use another), nor between the
    /// restore's registration check and its store write.
    ///
    /// Once installed, the reservations of the private contact requests the
    /// store holds unconfirmed payments for are restored into this wallet's
    /// identities ([`Self::restore_private_contact_payments`]), with no host
    /// call and no crypto provider.
    #[cfg(feature = "shielded")]
    async fn install_shielded_views(
        &self,
        account_views: std::collections::BTreeMap<u32, super::shielded::AccountViewingKeys>,
        coordinator: &Arc<crate::wallet::shielded::NetworkShieldedCoordinator>,
        start: crate::changeset::ClientStartState,
        snapshot_generation: u64,
    ) -> Result<(), PlatformWalletError> {
        self.install_shielded_views_in_one_transaction(
            account_views,
            coordinator,
            start,
            snapshot_generation,
        )
        .await?;
        // After the install transaction: the restore takes the store and
        // wallet locks on its own.
        self.restore_private_payments_or_defer(coordinator).await;
        Ok(())
    }

    /// Restore this wallet's private contact request payments from
    /// `coordinator`'s store at once, or, while the store is busy, on a task
    /// once it frees. It never waits for the store, which a running scan may
    /// hold for minutes while a host binds on its UI thread.
    ///
    /// A busy store holds the wallet's contact requests until the deferred
    /// restoration succeeds, unless this generation already restored from that
    /// store: then its reservations are in place and nothing is held or
    /// queued. At most one deferred restoration per store is queued: a bind
    /// while one waits queues none. The queue frees however its task ends,
    /// a panic or an abort included (`DeferredFinish`). Returns the task of
    /// the deferred restoration when one was queued.
    #[cfg(feature = "shielded")]
    pub(crate) async fn restore_private_payments_or_defer(
        &self,
        coordinator: &Arc<NetworkShieldedCoordinator>,
    ) -> Option<tokio::task::JoinHandle<()>> {
        let restore = self.private_payment_restore(coordinator);
        match restore.restore_and_release(false).await {
            Ok(Some(_)) => None,
            Ok(None) => {
                if restore.restored_from_this_store() || !restore.queue_deferred() {
                    return None;
                }
                tracing::debug!(
                    wallet_id = %hex::encode(self.wallet_id),
                    "the shielded store is busy; private contact request payments are \
                     restored once it frees"
                );
                // Owned by the task's future from before it is first polled,
                // so the queue frees even for a task dropped unpolled.
                let restore = DeferredFinish(restore);
                Some(tokio::spawn(async move {
                    if let Err(error) = restore.0.restore_and_release(true).await {
                        tracing::warn!(
                            wallet_id = %hex::encode(restore.0.wallet_id),
                            %error,
                            "private contact request payments not restored; the wallet's \
                             contact requests stay held"
                        );
                    }
                }))
            }
            Err(error) => {
                tracing::warn!(
                    wallet_id = %hex::encode(self.wallet_id),
                    %error,
                    "private contact request payments not restored at bind; the wallet's \
                     contact requests stay held until a restoration succeeds"
                );
                None
            }
        }
    }

    /// The body of [`Self::install_shielded_views`]: one coordinator install
    /// transaction.
    #[cfg(feature = "shielded")]
    async fn install_shielded_views_in_one_transaction(
        &self,
        account_views: std::collections::BTreeMap<u32, super::shielded::AccountViewingKeys>,
        coordinator: &Arc<crate::wallet::shielded::NetworkShieldedCoordinator>,
        start: crate::changeset::ClientStartState,
        snapshot_generation: u64,
    ) -> Result<(), PlatformWalletError> {
        // Serialize registration with identity-shield proof construction. Otherwise
        // an account could disappear before that operation arms its durable guard.
        let _shield_guard = self.shield_guard.lock().await;
        let install = coordinator.begin_install(self.wallet_id).await;

        // A wallet the manager has already removed must not be able to
        // re-register itself. Callers reach a bind through an
        // `Arc<PlatformWallet>` they resolved earlier and keep across
        // the (possibly seed-resolving, so arbitrarily long) bind, so
        // the removal can land in between; re-registering here would
        // resurrect the wallet on the coordinator, and the next sync
        // pass would re-fetch and re-persist shielded history the host
        // believes it deleted. Checked inside the transaction, which
        // `remove_wallet`'s unregister also has to take, so the two
        // cannot interleave.
        self.ensure_shielded_attached()?;

        // Refuse to re-key a bound account. Durable per-subwallet state
        // (notes, activity, watermark) is keyed by
        // `(wallet_id, account_index)` alone, so it cannot be attributed
        // to the key that produced it: installing a different key for an
        // account that already has state would leave notes the new
        // spend key cannot spend, and a watermark that makes the scan
        // skip the range where the new key's own notes live. There is no
        // legitimate flow that changes an account's key in place — see
        // `ShieldedChangeSet::viewing_keys` — so this is corruption or a
        // derivation change, and the host has to Clear (which wipes both
        // sides) before binding the new key.
        if let Some(account) = install.conflicting_account(&account_views).await {
            return Err(PlatformWalletError::ShieldedKeyDerivation(format!(
                "account {account} is already bound to a different shielded viewing key; \
                 clear shielded state before binding a re-keyed account"
            )));
        }

        // Compute idempotence BEFORE registering — after
        // register_wallet the registration always matches.
        let identical = install.registration_matches(&account_views).await;

        // (Re-)register on the coordinator. This is non-destructive
        // by construction: the persister handle is replaced, never
        // removed (a sync pass finishing mid-bind always finds one),
        // and per-subwallet store state is purged only for accounts
        // this registration DROPS or re-keys — accounts that remain
        // bound with the same viewing key keep their in-memory notes
        // and watermark. A re-bind racing an in-flight sync pass
        // therefore cannot wipe the pass's results (an
        // unregister-then-register cycle here would purge the whole
        // wallet behind the pass's store lock and then restore a
        // pre-pass snapshot — the "note discovered by sync is
        // unspendable until app restart" / "every pass rescans from
        // 0" failure). Registration also runs BEFORE the restore so
        // the restore path's "is this account registered?" gate sees
        // this wallet's subwallets.
        install
            .register(account_views.clone(), self.persister.clone())
            .await?;
        // Publish wallet keys only after registration accepted the new shape.
        *self.shielded_keys.write().await = Some(account_views);

        // Idempotent re-bind fast path: hosts re-run bind liberally
        // (launch fires it twice — a direct call plus the wallet-set
        // observer — and again on Sync Now / wallet navigation). When
        // the registration is unchanged AND a prior hydration
        // succeeded, the coordinator's in-memory state is strictly
        // fresher than any persister snapshot (the snapshot's rows
        // were produced FROM it), so re-running the restore could
        // only re-apply older data — skip it. The hydration flag is
        // load-bearing: a matching registration alone doesn't prove
        // the store was ever hydrated (the first bind's load/restore
        // may have failed transiently), and
        // skipping on registration match alone would leave notes and
        // the watermark absent until a full rescan or restart.
        if identical && install.is_hydrated().await {
            return Ok(());
        }

        // Rehydrate per-subwallet notes / sync watermarks from
        // the persister's start state if any are present for
        // this wallet. The restore is additive and monotonic
        // (`restore_for_wallet` never rewinds a watermark or
        // overwrites a known note), so applying a snapshot on top
        // of retained live state is safe. Surface restore failures to
        // the host and leave hydration unset: an empty first-launch
        // snapshot succeeds, while an unreadable ledger must not look
        // like a successfully bound zero balance. A subsequent bind
        // retries rather than fast-pathing over an unhydrated store.
        // A Clear that completed after `start` was read wiped both the
        // store and (once it returned) the host's own rows, so this
        // snapshot describes state the user asked to be deleted.
        // Restoring it would put the notes back and re-arm the pre-Clear
        // watermark, which reports caught-up and suppresses the cold
        // rebuild Clear promises. Register (done above) but restore
        // nothing, and leave hydration unset so the next bind hydrates
        // from the host's post-Clear rows.
        if install.snapshot_predates_clear(snapshot_generation) {
            install.mark_hydrated(false).await;
            tracing::info!(
                wallet_id = %hex::encode(self.wallet_id),
                "Skipped shielded snapshot restore: shielded state was cleared while binding"
            );
            return Ok(());
        }

        match install.restore(&start.shielded).await {
            Ok(()) => install.mark_hydrated(true).await,
            Err(e) => {
                install.mark_hydrated(false).await;
                tracing::warn!(
                    wallet_id = %hex::encode(self.wallet_id),
                    error = %e,
                    "Failed to restore shielded snapshot at bind time"
                );
                return Err(e);
            }
        }
        Ok(())
    }

    /// Add another ZIP-32 account to the already-bound shielded
    /// sub-wallet. Returns `ShieldedNotBound` if `bind_shielded`
    /// hasn't run yet.
    ///
    /// **Caveat**: notes belonging to `account` that already
    /// landed on-chain before the bind call only become spendable
    /// after a tree wipe + re-sync. Hosts that need to discover
    /// historical funds for a freshly-added account should drop
    /// the commitment-tree DB and call [`bind_shielded`] again
    /// with the full account list.
    #[cfg(feature = "shielded")]
    pub async fn shielded_add_account(
        &self,
        seed: &[u8],
        account: u32,
    ) -> Result<(), PlatformWalletError> {
        use super::shielded::{OrchardKeySet, SubwalletId};
        let required = crate::changeset::PersistenceCapabilities::SHIELDED_FVK_RESTART;
        let capabilities = self.persister.persistence_capabilities();
        if !capabilities.contains(required) {
            let missing = capabilities.missing(required);
            return Err(PlatformWalletError::Persistence(format!(
                "shielded account persistence requires capabilities {:?} \
                 (missing mask 0x{:x})",
                missing.names(),
                missing.bits(),
            )));
        }
        self.ensure_shielded_attached()?;
        // Everything that calls into the host — the snapshot read below
        // and the viewing-key write further down — stays OUTSIDE the key
        // slot's lock. A host callback invoked while this write guard is
        // held would deadlock against a concurrent bind, which takes the
        // coordinator's lifecycle mutex and then this same slot: the
        // callback re-enters the FFI, waits on the lifecycle mutex, and
        // the bind holding it waits on the slot the callback's caller
        // never released. It also keeps the slot free for the address /
        // balance reads that run constantly while this does host I/O.
        {
            let slot = self.shielded_keys.read().await;
            let keys = slot.as_ref().ok_or(PlatformWalletError::ShieldedNotBound)?;
            if keys.contains_key(&account) {
                return Ok(());
            }
        }
        let views = OrchardKeySet::from_seed(seed, self.sdk.network, account)?.viewing_keys();
        // This is the other writer of a subwallet's viewing-key row, so
        // it owes the same refusal `bind_shielded` makes: an account
        // absent from the in-memory map can still have durable rows from
        // an earlier session, and overwriting its key would leave those
        // notes and their watermark attributed to a key that did not
        // produce them, undetectably (a later seedless bind derives its
        // keys FROM this row, so nothing downstream can see the swap).
        let start = self.persister.load().map_err(|e| {
            PlatformWalletError::ShieldedBuildError(format!(
                "persister load failed while adding shielded account {account}: {e}"
            ))
        })?;
        let id = SubwalletId::new(self.wallet_id, account);
        if let Some(persisted) = start.shielded.viewing_keys.get(&id) {
            if persisted.as_slice() != views.to_fvk_bytes().as_slice() {
                return Err(PlatformWalletError::ShieldedKeyDerivation(format!(
                    "persisted shielded viewing key for account {account} differs from the one \
                     derived from this seed; clear shielded state before binding a re-keyed \
                     account"
                )));
            }
        }
        // Persist the new account's viewing key alongside the
        // in-memory insert, mirroring `bind_shielded`, so the
        // seedless rebind path covers it on the next launch.
        let mut cs = crate::changeset::ShieldedChangeSet::default();
        cs.record_viewing_key(
            SubwalletId::new(self.wallet_id, account),
            views.to_fvk_bytes(),
        );
        self.persister
            .store(crate::changeset::PlatformWalletChangeSet {
                shielded: Some(cs),
                ..Default::default()
            })
            .map_err(|e| {
                PlatformWalletError::Persistence(format!(
                    "failed to persist shielded viewing key for account {account}: {e}"
                ))
            })?;
        // Re-check detachment before mutating the handle: the host I/O
        // above can take arbitrarily long, and a removal completing in
        // that window means this account must not be installed.
        self.ensure_shielded_attached()?;
        let mut slot = self.shielded_keys.write().await;
        let keys = slot.as_mut().ok_or(PlatformWalletError::ShieldedNotBound)?;
        // Idempotent against a bind that added this account while the
        // host I/O above was in flight: same seed and index derive the
        // same key, so re-inserting is a no-op either way.
        keys.insert(account, views);
        // NOTE: this only updates the per-wallet keys slot — the
        // coordinator's `accounts` registry isn't refreshed here.
        // Hosts that add accounts after bind should re-call
        // `bind_shielded` with the full account list so the
        // coordinator's viewing-key registry stays in sync.
        Ok(())
    }

    /// Whether the shielded sub-wallet has been bound via
    /// [`bind_shielded`](Self::bind_shielded).
    #[cfg(feature = "shielded")]
    pub async fn is_shielded_bound(&self) -> bool {
        self.shielded_keys.read().await.is_some()
    }

    /// `Err` once this wallet has been removed from the manager.
    ///
    /// The install transaction performs the authoritative check, but the
    /// bind paths also write to the HOST persister (viewing-key rows)
    /// before they get there, and hosts delete their own wallet data
    /// after `remove_wallet` returns — so a bind that only failed at the
    /// end would still leave a full viewing key on disk for a wallet the
    /// user deleted, with the mnemonic already gone. An FVK discloses
    /// every incoming and outgoing note of its account, so the bind
    /// paths check here first, before writing anything.
    #[cfg(feature = "shielded")]
    fn ensure_shielded_attached(&self) -> Result<(), PlatformWalletError> {
        if self
            .shielded_detached
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(PlatformWalletError::WalletNotFound(format!(
                "{} was removed from the manager; shielded bind refused",
                hex::encode(self.wallet_id)
            )));
        }
        Ok(())
    }

    /// Mark this wallet as removed from the manager, so any handle that
    /// outlives the removal can no longer bind shielded state onto the
    /// coordinator.
    ///
    /// Must be called **before** the coordinator's
    /// `unregister_wallet`: that call takes the same install
    /// transaction a bind holds, so setting the flag first makes every
    /// bind either commit fully before the unregister purges it, or see
    /// the flag and refuse. Setting it afterwards would leave the
    /// window this closes.
    #[cfg(feature = "shielded")]
    pub(crate) fn mark_shielded_detached(&self) {
        self.shielded_detached
            .store(true, std::sync::atomic::Ordering::Release);
    }

    /// Bound ZIP-32 account indices on the shielded sub-wallet,
    /// in ascending order. Empty if not bound.
    #[cfg(feature = "shielded")]
    pub async fn shielded_account_indices(&self) -> Vec<u32> {
        self.shielded_keys
            .read()
            .await
            .as_ref()
            .map(|keys| keys.keys().copied().collect())
            .unwrap_or_default()
    }

    /// The default Orchard payment address for `account` on this
    /// wallet, as the raw 43-byte representation. Returns `None`
    /// if the shielded sub-wallet hasn't been bound or `account`
    /// isn't bound on it. Hosts apply their own bech32m encoding
    /// (HRP + 0x10 type byte) on top.
    #[cfg(feature = "shielded")]
    pub async fn shielded_default_address(&self, account: u32) -> Option<[u8; 43]> {
        let guard = self.shielded_keys.read().await;
        guard
            .as_ref()
            .and_then(|keys| keys.get(&account))
            .map(|ks| ks.default_address.to_raw_address_bytes())
    }

    /// Per-account default Orchard payment addresses (raw 43 bytes).
    #[cfg(feature = "shielded")]
    pub async fn shielded_default_addresses(&self) -> std::collections::BTreeMap<u32, [u8; 43]> {
        let guard = self.shielded_keys.read().await;
        let Some(keys) = guard.as_ref() else {
            return std::collections::BTreeMap::new();
        };
        keys.iter()
            .map(|(account, ks)| (*account, ks.default_address.to_raw_address_bytes()))
            .collect()
    }

    /// Per-account unspent shielded balance.
    ///
    /// Reads against the coordinator's shared store (one SQLite
    /// handle per network); returns an empty map if shielded
    /// support hasn't been configured or this wallet isn't
    /// bound. Folds the network-wide
    /// [`balances_across`](super::shielded::sync::balances_across)
    /// result down to this wallet's per-account slice.
    #[cfg(feature = "shielded")]
    pub async fn shielded_balances(
        &self,
        coordinator: &Arc<crate::wallet::shielded::NetworkShieldedCoordinator>,
    ) -> Result<std::collections::BTreeMap<u32, u64>, PlatformWalletError> {
        use super::shielded::{AccountViewingKeys, SubwalletId};
        let guard = self.shielded_keys.read().await;
        let Some(keys) = guard.as_ref() else {
            return Ok(std::collections::BTreeMap::new());
        };
        let subwallets: Vec<(SubwalletId, AccountViewingKeys)> = keys
            .iter()
            .map(|(account, views)| (SubwalletId::new(self.wallet_id, *account), views.clone()))
            .collect();
        let per_sub =
            super::shielded::sync::balances_across(coordinator.store(), &subwallets).await?;
        Ok(per_sub
            .into_iter()
            .filter(|(id, _)| id.wallet_id == self.wallet_id)
            .map(|(id, v)| (id.account_index, v))
            .collect())
    }

    /// Transiently re-derive `account`'s full `OrchardKeySet` (ASK
    /// included) from the caller-supplied wallet seed, for the
    /// duration of one spend operation. The derived spend authority
    /// is dropped when the returned value goes out of scope — no
    /// `SpendAuthorizingKey` is ever resident on the wallet.
    ///
    /// Guards two invariants before handing the keyset back:
    /// - `account` must be bound (viewing keys installed), so spend
    ///   errors match the pre-split `ShieldedNotBound` /
    ///   "account not bound" contract.
    /// - the seed-derived FVK must equal the bound viewing key —
    ///   a wrong seed (or a persisted-key / seed mismatch) fails
    ///   loudly here instead of burning a ~30 s Halo 2 proof on a
    ///   spend the chain would reject.
    #[cfg(feature = "shielded")]
    async fn derive_spend_keyset(
        &self,
        seed: &[u8],
        account: u32,
    ) -> Result<super::shielded::OrchardKeySet, PlatformWalletError> {
        use super::shielded::OrchardKeySet;
        let bound_fvk = {
            let guard = self.shielded_keys.read().await;
            let keys = guard
                .as_ref()
                .ok_or(PlatformWalletError::ShieldedNotBound)?;
            let views = keys.get(&account).ok_or_else(|| {
                PlatformWalletError::ShieldedKeyDerivation(format!(
                    "shielded account {account} not bound"
                ))
            })?;
            views.full_viewing_key.to_bytes()
        };
        let keyset = OrchardKeySet::from_seed(seed, self.sdk.network, account)?;
        if keyset.full_viewing_key.to_bytes() != bound_fvk {
            return Err(PlatformWalletError::ShieldedKeyDerivation(format!(
                "seed does not derive the bound viewing key for shielded account {account}"
            )));
        }
        Ok(keyset)
    }

    /// Send a private shielded → shielded transfer from `account`'s
    /// notes to `recipient_raw_43` (the recipient's Orchard payment
    /// address as the 43 raw bytes).
    ///
    /// `coordinator` supplies the shared, network-scoped
    /// commitment-tree store; `seed` supplies the spend authority —
    /// the full `OrchardKeySet` (with the `SpendAuthorizingKey`) is
    /// re-derived from it for this call only and dropped on return.
    /// Privilege separation: the ASK never crosses to the
    /// coordinator — the spend free function takes the keyset by
    /// reference at call time.
    ///
    /// The prover is consumed by value rather than borrowed
    /// because `OrchardProver` is impl'd on
    /// `&CachedOrchardProver` (the reference type), not on the
    /// bare struct. Callers pass `&CachedOrchardProver::new()`
    /// and we forward it down to the spend free function's
    /// `&P` parameter.
    #[cfg(feature = "shielded")]
    #[allow(clippy::too_many_arguments)]
    pub async fn shielded_transfer_to<P: dpp::shielded::builder::OrchardProver>(
        &self,
        coordinator: &Arc<crate::wallet::shielded::NetworkShieldedCoordinator>,
        seed: &[u8],
        account: u32,
        recipient_raw_43: &[u8; 43],
        amount: u64,
        memo: [u8; 36],
        prover: P,
    ) -> Result<(), PlatformWalletError> {
        let keyset = self.derive_spend_keyset(seed, account).await?;
        let recipient = Option::<grovedb_commitment_tree::PaymentAddress>::from(
            grovedb_commitment_tree::PaymentAddress::from_raw_address_bytes(recipient_raw_43),
        )
        .ok_or_else(|| {
            PlatformWalletError::ShieldedBuildError(
                "invalid Orchard payment address bytes".to_string(),
            )
        })?;
        super::shielded::operations::transfer(
            &self.sdk,
            coordinator.store(),
            Some(&self.persister),
            self.wallet_id,
            &keyset,
            account,
            &recipient,
            amount,
            memo,
            &prover,
        )
        .await
    }

    /// Unshield from `account`'s notes to a transparent platform
    /// address (`"dash1…"` / `"tdash1…"`). Parsed via
    /// `PlatformAddress::from_bech32m_string`; the recipient's HRP is
    /// verified against the wallet's network HRP class here, since the
    /// network-agnostic decoder does not enforce it. `seed` supplies
    /// the transient spend authority (see
    /// [`shielded_transfer_to`](Self::shielded_transfer_to)).
    #[cfg(feature = "shielded")]
    pub async fn shielded_unshield_to<P: dpp::shielded::builder::OrchardProver>(
        &self,
        coordinator: &Arc<crate::wallet::shielded::NetworkShieldedCoordinator>,
        seed: &[u8],
        account: u32,
        to_platform_addr_bech32m: &str,
        amount: u64,
        prover: P,
    ) -> Result<(), PlatformWalletError> {
        let keyset = self.derive_spend_keyset(seed, account).await?;
        // The decoder is network-agnostic, so guard the recipient's HRP class
        // against the wallet's network before decoding.
        check_recipient_hrp(to_platform_addr_bech32m, self.sdk.network)?;
        let to = dpp::address_funds::PlatformAddress::from_bech32m_string(to_platform_addr_bech32m)
            .map_err(|e| {
                PlatformWalletError::ShieldedBuildError(format!("invalid platform address: {e}"))
            })?;
        super::shielded::operations::unshield(
            &self.sdk,
            coordinator.store(),
            Some(&self.persister),
            self.wallet_id,
            &keyset,
            account,
            &to,
            amount,
            &prover,
        )
        .await
    }

    /// Top up an existing Platform identity's balance from `account`'s
    /// shielded notes (`IdentityTopUpFromShieldedPool`, type 22). The identity
    /// need not belong to this wallet: it only has to exist on Platform. The
    /// identity receives `amount`; the flat pool-paid fee comes out of the
    /// spent notes on top. `seed` supplies the transient spend authority (see
    /// [`shielded_transfer_to`](Self::shielded_transfer_to)).
    #[cfg(feature = "shielded")]
    pub async fn shielded_identity_top_up_from_pool<P: dpp::shielded::builder::OrchardProver>(
        &self,
        coordinator: &Arc<crate::wallet::shielded::NetworkShieldedCoordinator>,
        seed: &[u8],
        account: u32,
        identity_id: &Identifier,
        amount: u64,
        prover: P,
    ) -> Result<Option<Credits>, PlatformWalletError> {
        // A zero amount must not queue behind a long-running shielded operation.
        ensure_positive_amount(amount)?;

        // Single-flight with the other shield-class operations. The proof
        // result carries the identity's absolute post-execution balance, so
        // two top-ups (or a top-up and a shield-from-identity debit) whose
        // waits completed out of execution order would let the older balance
        // overwrite the newer one. Held across build -> broadcast -> wait ->
        // reconcile.
        let _shield_guard = self.shield_guard.lock().await;

        let keyset = self.derive_spend_keyset(seed, account).await?;
        let (mut proven_balance, metadata) =
            super::shielded::operations::identity_top_up_from_pool_with_metadata(
                &self.sdk,
                coordinator.store(),
                Some(&self.persister),
                self.wallet_id,
                &keyset,
                account,
                *identity_id,
                amount,
                &prover,
            )
            .await?;

        // The target may be one of this wallet's identities. Apply the proof-attested
        // balance rather than adding `amount` locally: the fee is carved from the
        // gross amount and a negative-credit identity absorbs part of a top-up, so
        // only the proven value is right. A foreign identity is simply not managed.
        if let Some(balance) = proven_balance {
            let mut wm = self.wallet_manager.write().await;
            let managed = wm
                .get_wallet_info_mut(&self.wallet_id)
                .and_then(|info| info.identity_manager.managed_identity_mut(identity_id));
            if let Some(managed) = managed {
                proven_balance = Some(managed.persist_confirmed_balance(
                    balance,
                    BlockTime::from(metadata),
                    &self.persister,
                ));
            }
        }

        Ok(proven_balance)
    }

    /// Send a DashPay private contact request (protocol version 15) from
    /// `sender_identity_id` to `recipient_identity_id`, paying for it from
    /// `account`'s shielded notes: nothing on chain names either identity. The
    /// request carries `account_label`, `auto_accept_proof` and our
    /// `shielded_address` for the contact, is signed by `signer` as the
    /// sender's transitions are, and is sealed by `crypto`, which keeps the
    /// encryption key. `seed` supplies the transient spend authority (see
    /// [`shielded_transfer_to`](Self::shielded_transfer_to)). See
    /// [`DashPayView::send_private_contact_request`](crate::DashPayView::send_private_contact_request).
    #[cfg(feature = "shielded")]
    #[allow(clippy::too_many_arguments)]
    pub async fn send_private_contact_request<S, C, P>(
        &self,
        coordinator: &Arc<NetworkShieldedCoordinator>,
        seed: &[u8],
        account: u32,
        sender_identity_id: &Identifier,
        recipient_identity_id: &Identifier,
        account_label: Option<String>,
        auto_accept_proof: AutoAcceptProofSource,
        shielded_address: Option<[u8; 43]>,
        signer: &S,
        crypto: &C,
        prover: P,
    ) -> Result<ContactRequest, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
        C: ContactCryptoProvider + Sync,
        P: OrchardProver + Send + Sync,
    {
        let keyset = self.derive_spend_keyset(seed, account).await?;
        self.identity
            .dashpay()
            .send_private_contact_request(
                sender_identity_id,
                recipient_identity_id,
                account_label,
                auto_accept_proof,
                shielded_address,
                signer,
                crypto,
                |entry, started| {
                    self.pay_private_contact_request(
                        coordinator,
                        keyset,
                        account,
                        sender_identity_id,
                        recipient_identity_id,
                        entry,
                        started,
                        prover,
                    )
                },
            )
            .await
    }

    /// Accept a DashPay contact request that came privately, replying
    /// privately and paying for the reply from `account`'s shielded notes, as
    /// [`send_private_contact_request`](Self::send_private_contact_request)
    /// pays and signing it with `signer`. `shielded_address` is the address
    /// the contact may pay us at privately. See
    /// [`DashPayView::accept_private_contact_request`](crate::DashPayView::accept_private_contact_request).
    #[cfg(feature = "shielded")]
    #[allow(clippy::too_many_arguments)]
    pub async fn accept_private_contact_request<S, C, P>(
        &self,
        coordinator: &Arc<NetworkShieldedCoordinator>,
        seed: &[u8],
        account: u32,
        request: &ContactRequest,
        shielded_address: Option<[u8; 43]>,
        signer: &S,
        crypto: &C,
        prover: P,
    ) -> Result<EstablishedContact, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
        C: ContactCryptoProvider + Sync,
        P: OrchardProver + Send + Sync,
    {
        let keyset = self.derive_spend_keyset(seed, account).await?;
        self.identity
            .dashpay()
            .accept_private_contact_request(
                request,
                shielded_address,
                signer,
                crypto,
                |entry, started| {
                    self.pay_private_contact_request(
                        coordinator,
                        keyset,
                        account,
                        &request.recipient_id,
                        &request.sender_id,
                        entry,
                        started,
                        prover,
                    )
                },
            )
            .await
    }

    /// Accept a DashPay contact request replying the way it came: privately,
    /// paid from a shielded account, when it came privately and no reply
    /// exists yet, else with a `contactRequest` document signed by `signer`.
    /// `payment` gives what a private reply is paid with (the shielded
    /// coordinator, the seed for the spend authority, the account and the
    /// prover) and is called only for a private reply, so a public reply or
    /// an adopted one needs neither shielded support nor the seed. See
    /// [`DashPayView::accept_contact_request_replying_in_kind`](crate::DashPayView::accept_contact_request_replying_in_kind).
    #[cfg(feature = "shielded")]
    pub async fn accept_contact_request_replying_in_kind<S, C, P, Seed, R, RFut>(
        &self,
        request: &ContactRequest,
        shielded_address: Option<[u8; 43]>,
        signer: &S,
        crypto: &C,
        payment: R,
    ) -> Result<EstablishedContact, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
        C: ContactCryptoProvider + Sync,
        P: OrchardProver + Send + Sync,
        Seed: AsRef<[u8]> + Send,
        R: FnOnce() -> RFut + Send,
        RFut: std::future::Future<Output = Result<PrivateReplyPayment<P, Seed>, PlatformWalletError>>
            + Send,
    {
        self.identity
            .dashpay()
            .accept_contact_request_replying_in_kind(
                request,
                shielded_address,
                signer,
                crypto,
                |entry, started| async move {
                    // Resolving the payment and deriving the spend key come
                    // first, before `started` is marked: a reply dropped
                    // meanwhile releases its reservation.
                    let PrivateReplyPayment {
                        coordinator,
                        seed,
                        account,
                        prover,
                    } = payment().await?;
                    let keyset = self.derive_spend_keyset(seed.as_ref(), account).await?;
                    drop(seed);
                    self.pay_private_contact_request(
                        &coordinator,
                        keyset,
                        account,
                        &request.recipient_id,
                        &request.sender_id,
                        entry,
                        started,
                        prover,
                    )
                    .await
                },
            )
            .await
    }

    /// Restore, at startup, the pending DashPay private contact requests
    /// whose payment is unconfirmed since before a restart, from the signed
    /// transitions `coordinator`'s store keeps for this wallet's unconfirmed
    /// spends: until a private sync reads each back, no other request to its
    /// recipient is sent, privately or publicly. `crypto` opens the entries.
    /// Returns the (sender, recipient) pairs restored. A private send runs
    /// the same check right before it pays, so without this call a second
    /// private request is still held; this call holds public ones too. A
    /// payment saved with its request needs no call: binding restores it
    /// ([`Self::restore_private_contact_payments`]). This call stays for
    /// records saved before that, and for an identity added after the bind.
    /// See
    /// [`DashPayView::restore_pending_private_contact_requests`](crate::DashPayView::restore_pending_private_contact_requests)
    /// for what it does not cover.
    #[cfg(feature = "shielded")]
    pub async fn restore_pending_private_contact_requests<C>(
        &self,
        coordinator: &Arc<NetworkShieldedCoordinator>,
        crypto: &C,
    ) -> Result<Vec<(Identifier, Identifier)>, PlatformWalletError>
    where
        C: ContactCryptoProvider + Sync,
    {
        let entries = self
            .unconfirmed_private_contact_requests(coordinator)
            .await?;
        self.identity
            .dashpay()
            .restore_pending_private_contact_requests(&entries, crypto)
            .await
    }

    /// Restore the reservations of this wallet's private contact request
    /// payments that may still execute, from the shielded store's private
    /// payment journal (sender, recipient, account reference and entry
    /// commitment of each, saved durably before it could reach the network),
    /// and release the holds a load set: no keys open anything. Run when the
    /// wallet binds to the coordinator, so restored reservations hold every
    /// public and private send from then on. Returns the (sender, recipient)
    /// pairs restored.
    ///
    /// Fails closed, every hold staying and the wallet's raised if it was
    /// not: for a coordinator of another network, a wallet removed or
    /// re-created since this handle was taken, a journal record that does
    /// not read back, a saved spend whose transition does not decode and
    /// that no journal record accounts for, a private contact request
    /// payment saved before the journal kept it with no request to restore
    /// its pair from, and an identity loaded marking a private payment that
    /// may be unresolved when the journal is new
    /// ([`PlatformWalletError::ShieldedRecoveryCorrupted`]). A payment saved
    /// before the journal kept it, with its request, is added to the
    /// journal.
    ///
    /// Run also by [`PlatformWalletManager::configure_shielded`] for the
    /// wallets the manager holds, by the registration or load of a wallet once
    /// the store is configured, and once by a contact request refused as not
    /// restored: no keys are needed. Once it succeeds, the wallet's contact
    /// requests go ahead, which are held
    /// ([`PlatformWalletError::PrivateContactPaymentsNotRestored`]) for a wallet
    /// published with no shielded store configured while its persister keeps
    /// the private scan state, one loaded after binding a shielded account, one
    /// whose last restoration failed or waits for a busy store, and an identity
    /// whose position record, its own or one its persister kept after its
    /// wallet's removal, marks a private payment that may be unresolved. A
    /// payment whose request its sender does not track rewinds the sender's
    /// read of the store to where it was when it paid, so the entry is read
    /// again.
    ///
    /// [`PlatformWalletManager::configure_shielded`]: crate::PlatformWalletManager::configure_shielded
    #[cfg(feature = "shielded")]
    pub async fn restore_private_contact_payments(
        &self,
        coordinator: &Arc<NetworkShieldedCoordinator>,
    ) -> Result<Vec<(Identifier, Identifier)>, PlatformWalletError> {
        self.private_payment_restore(coordinator)
            .restore_and_release(true)
            .await
            .map(Option::unwrap_or_default)
    }

    /// The handles a restoration of this wallet's private contact request
    /// payments runs on ([`Self::restore_private_contact_payments`]).
    ///
    /// Installs on the persister one more restoration from `coordinator`, for
    /// the wallet's views to run, without waiting on the store, before a
    /// contact request is refused as not restored
    /// (`WalletPersister::retry_private_payment_restore`): a failed or
    /// deferred restoration then lifts at the next send.
    #[cfg(feature = "shielded")]
    fn private_payment_restore(
        &self,
        coordinator: &Arc<NetworkShieldedCoordinator>,
    ) -> PrivatePaymentRestore {
        let retry: PrivatePaymentRestoreRetry = {
            let coordinator = Arc::downgrade(coordinator);
            let wallet_manager = Arc::clone(&self.wallet_manager);
            let generation = Arc::clone(&self.generation);
            let restores = Arc::clone(&self.private_payment_restores);
            let (wallet_id, network) = (self.wallet_id, self.sdk.network);
            Arc::new(move |persister: WalletPersister| {
                let restore = coordinator
                    .upgrade()
                    .map(|coordinator| PrivatePaymentRestore {
                        coordinator,
                        wallet_manager: Arc::clone(&wallet_manager),
                        persister,
                        generation: Arc::clone(&generation),
                        restores: Arc::clone(&restores),
                        wallet_id,
                        network,
                    });
                Box::pin(async move {
                    let Some(restore) = restore else {
                        return;
                    };
                    if let Err(error) = restore.restore_and_release(false).await {
                        tracing::warn!(
                            wallet_id = %hex::encode(wallet_id),
                            %error,
                            "private contact request payments not restored before a send; \
                             the wallet's contact requests stay held"
                        );
                    }
                })
            })
        };
        self.persister.set_private_payment_restore_retry(retry);
        PrivatePaymentRestore {
            coordinator: Arc::clone(coordinator),
            wallet_manager: Arc::clone(&self.wallet_manager),
            persister: self.persister.clone(),
            generation: Arc::clone(&self.generation),
            restores: Arc::clone(&self.private_payment_restores),
            wallet_id: self.wallet_id,
            network: self.sdk.network,
        }
    }

    /// Whether this wallet's public contact requests wait for its private
    /// payments from an earlier process to be restored
    /// ([`Self::restore_private_contact_payments`]): it bound a shielded account
    /// before, or one of its identities was loaded marking a private payment
    /// that may be unresolved.
    pub async fn private_payments_held(&self) -> bool {
        self.persister.private_payments_unrestored()
            || self
                .identity
                .dashpay()
                .any_private_payments_unrestored()
                .await
    }

    /// The entries of the private contact requests whose payment from this
    /// wallet's notes `coordinator`'s store holds as unconfirmed.
    #[cfg(feature = "shielded")]
    async fn unconfirmed_private_contact_requests(
        &self,
        coordinator: &Arc<NetworkShieldedCoordinator>,
    ) -> Result<Vec<BinaryData>, PlatformWalletError> {
        let dashpay_id = dashpay_contract()?.id();
        let store = coordinator.store().read().await;
        let redrives = store
            .pending_redrives_for_wallet(self.wallet_id)
            .map_err(PlatformWalletError::from)?;
        Ok(private_contact_request_redrive_entries(
            redrives.iter().map(|(_, redrive)| redrive),
            dashpay_id,
        ))
    }

    /// Append the sealed private contact request `entry`, from
    /// `sender_identity_id` to `recipient_identity_id`, to DashPay's store,
    /// paying from `account`'s notes, and return the metadata of the proof
    /// that it was appended.
    ///
    /// A request between the same pair whose payment is unconfirmed since
    /// before a restart may still execute: then nothing is paid, and the
    /// recipient stays reserved for that request
    /// ([`PlatformWalletError::PrivateContactRequestPending`]).
    ///
    /// Holds the wallet generation's lifecycle gate from the liveness check
    /// until the spend returns, so a removal waits for it. A wallet removed or
    /// re-created since this handle was taken is refused with
    /// [`PlatformWalletError::WalletRemovedBeforeSend`], with nothing
    /// restored, recorded or paid. A caller must not hold the gate.
    ///
    /// `started` is marked right before the transition may reach the
    /// network, after the restore check and the proof: until then a send
    /// dropped releases its reservation.
    #[cfg(feature = "shielded")]
    #[allow(clippy::too_many_arguments)]
    async fn pay_private_contact_request<P>(
        &self,
        coordinator: &Arc<NetworkShieldedCoordinator>,
        keyset: OrchardKeySet,
        account: u32,
        sender_identity_id: &Identifier,
        recipient_identity_id: &Identifier,
        entry: BinaryData,
        started: PaymentStarted,
        prover: P,
    ) -> Result<ResponseMetadata, PlatformWalletError>
    where
        P: OrchardProver,
    {
        // The generation's lifecycle gate, held across the liveness check,
        // the restore and the spend, before the wallet manager lock as the
        // teardown takes them: a removal cannot complete in between, so a
        // removed wallet's spend never records activity, recreating its
        // shielded state, or reaches the network.
        let _lifecycle = self.core().generation_payment_guard().await;
        if !self.core().is_current_generation().await {
            return Err(PlatformWalletError::WalletRemovedBeforeSend(hex::encode(
                self.wallet_id,
            )));
        }
        // An earlier payment of the pair the journal keeps may still execute,
        // whether or not its redrive record reads: then nothing is paid. The
        // journal is read strictly: a record that does not read refuses.
        let journaled: Vec<(u32, [u8; 32])> = {
            let mut store = coordinator.store().write().await;
            discharge_read_back_private_payments(&mut *store, &self.persister, self.wallet_id);
            store
                .private_payment_journal(self.wallet_id)
                .map_err(PlatformWalletError::from)?
                .into_iter()
                .filter(|record| {
                    Identifier::from(record.payment.sender) == *sender_identity_id
                        && Identifier::from(record.payment.recipient) == *recipient_identity_id
                })
                .map(|record| (record.payment.account_reference, record.commitment))
                .collect()
        };
        if self
            .identity
            .dashpay()
            .hold_for_journaled_private_payment(
                sender_identity_id,
                recipient_identity_id,
                &journaled,
            )
            .await?
        {
            return Err(PlatformWalletError::PrivateContactRequestPending(
                *recipient_identity_id,
            ));
        }
        let dashpay_id = dashpay_contract()?.id();
        // Saved with the payment's record, so a restart restores the pair's
        // reservation without opening the entry.
        let private_contact =
            started
                .account_reference()
                .map(|account_reference| PrivateContactPayment {
                    sender: sender_identity_id.to_buffer(),
                    recipient: recipient_identity_id.to_buffer(),
                    account_reference,
                });
        private_document_create_from_pool_with_metadata(
            &self.sdk,
            coordinator.store(),
            Some(&self.persister),
            self.wallet_id,
            &keyset,
            account,
            dashpay_id,
            PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
            vec![entry],
            private_contact,
            &started,
            &prover,
        )
        .await
    }

    /// Withdraw from `account`'s notes to a Core L1 address
    /// (Base58Check string). `core_fee_per_byte` is the L1 fee
    /// rate (duffs/byte). `seed` supplies the transient spend
    /// authority (see
    /// [`shielded_transfer_to`](Self::shielded_transfer_to)).
    #[cfg(feature = "shielded")]
    #[allow(clippy::too_many_arguments)]
    pub async fn shielded_withdraw_to<P: dpp::shielded::builder::OrchardProver>(
        &self,
        coordinator: &Arc<crate::wallet::shielded::NetworkShieldedCoordinator>,
        seed: &[u8],
        account: u32,
        to_core_address: &str,
        amount: u64,
        core_fee_per_byte: u32,
        prover: P,
    ) -> Result<(), PlatformWalletError> {
        let keyset = self.derive_spend_keyset(seed, account).await?;
        let network = self.sdk.network;
        let parsed = to_core_address
            .parse::<dashcore::Address<dashcore::address::NetworkUnchecked>>()
            .map_err(|e| {
                PlatformWalletError::ShieldedBuildError(format!("invalid core address: {e}"))
            })?
            .require_network(network)
            .map_err(|e| {
                PlatformWalletError::ShieldedBuildError(format!(
                    "core address network mismatch: {e}"
                ))
            })?;
        super::shielded::operations::withdraw(
            &self.sdk,
            coordinator.store(),
            Some(&self.persister),
            self.wallet_id,
            &keyset,
            account,
            &parsed,
            amount,
            core_fee_per_byte,
            &prover,
        )
        .await
    }

    /// Create a brand-new Platform identity funded directly from `account`'s shielded notes.
    ///
    /// Spends notes covering a fixed `denomination` (a member of the versioned exit-denomination
    /// set); the whole denomination leaves the pool and the metered fee is taken from it, so the
    /// new identity is created holding `denomination - total_fee`. Any excess re-enters the pool as
    /// a change note to `account`'s default Orchard address.
    ///
    /// `public_keys` is the new identity's key set (each entry pairs the `IdentityPublicKey` with
    /// its `IdentityPublicKeyInCreation` form); `identity_signer` produces each key's
    /// proof-of-possession signature. The Orchard spend authority is re-derived from `seed` for
    /// this call only (the ASK never crosses to the coordinator and is not retained).
    ///
    /// `identity_index` is the DIP-9 identity-registration slot the new identity occupies in the
    /// local `IdentityManager`; on a successful broadcast the proof-verified identity is registered
    /// there (mirroring `register_from_addresses`) so the host persister emits the
    /// `IdentityChangeSet` / `IdentityKeysChangeSet` that creates the app's identity row. A failed
    /// registration after a successful broadcast is logged and swallowed — the identity already
    /// exists on chain, so the next sync heals the local row. Returns the new identity's id.
    #[cfg(feature = "shielded")]
    #[allow(clippy::too_many_arguments)]
    pub async fn shielded_identity_create_from_pool<P, IS>(
        &self,
        coordinator: &Arc<crate::wallet::shielded::NetworkShieldedCoordinator>,
        seed: &[u8],
        account: u32,
        identity_index: u32,
        public_keys: Vec<(
            dpp::identity::IdentityPublicKey,
            dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation,
        )>,
        denomination: u64,
        send_to_address_on_creation_failure: dpp::address_funds::PlatformAddress,
        identity_signer: &IS,
        prover: P,
    ) -> Result<dpp::prelude::Identifier, PlatformWalletError>
    where
        P: dpp::shielded::builder::OrchardProver,
        IS: dpp::identity::signer::Signer<dpp::identity::IdentityPublicKey> + Send + Sync,
    {
        let (identity_id, identity) = {
            // Scope the transient keyset so its spend authority is dropped before we take the
            // wallet-manager write lock below — it's only needed for the spend, not for the
            // registration step.
            let keyset = self.derive_spend_keyset(seed, account).await?;
            super::shielded::operations::identity_create_from_shielded_pool(
                &self.sdk,
                coordinator.store(),
                Some(&self.persister),
                self.wallet_id,
                &keyset,
                account,
                public_keys,
                denomination,
                send_to_address_on_creation_failure,
                identity_signer,
                &prover,
            )
            .await?
        };

        // Register the proof-verified identity in the local manager at its HD slot, exactly like
        // `register_from_addresses`' Step 3 — this drives the host persister's
        // `IdentityChangeSet` / `IdentityKeysChangeSet` emit so the app's identity row is created.
        // The broadcast already succeeded; a registration failure here (e.g. the slot is already
        // occupied locally) is logged and swallowed rather than surfaced as an error, since the
        // identity exists on chain and the next sync heals the local view.
        {
            let mut wm = self.wallet_manager.write().await;
            match wm.get_wallet_info_mut(&self.wallet_id) {
                Some(info) => {
                    if let Err(e) = info.identity_manager.add_identity(
                        identity,
                        identity_index,
                        self.wallet_id,
                        &self.persister,
                    ) {
                        tracing::warn!(
                            identity_index,
                            error = %e,
                            "IdentityCreateFromShieldedPool broadcast succeeded but registering the \
                             identity in the local manager failed; the on-chain identity exists and \
                             the next sync will heal the local row"
                        );
                    }
                }
                None => {
                    tracing::warn!(
                        identity_index,
                        "IdentityCreateFromShieldedPool broadcast succeeded but the wallet info was \
                         not found in the manager; skipping local registration (heals on next sync)"
                    );
                }
            }
        }

        Ok(identity_id)
    }

    #[cfg(feature = "shielded")]
    async fn shielded_shield_plan_for_account(
        &self,
        payment_account: u32,
    ) -> Result<ShieldedShieldInputPlan, PlatformWalletError> {
        let wallet_manager = self.wallet_manager.read().await;
        let wallet_info = wallet_manager
            .get_wallet_info(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let account = wallet_info
            .core_wallet
            .platform_payment_managed_account_at_index(payment_account)
            .ok_or_else(|| {
                PlatformWalletError::AddressOperation(format!(
                    "no platform payment account at index {payment_account}"
                ))
            })?;

        // Candidate discovery must include both the transient derived pool and
        // persisted balances hydrated during wallet load. The latter can be
        // populated before the derived pool after an app relaunch. Sorting
        // happens in `plan_shield_inputs`, after conversion, because the
        // resulting PlatformAddress order is what the BTreeMap and network use
        // to identify input 0.
        let candidate_addresses = merge_platform_payment_candidate_addresses(
            account
                .addresses
                .addresses
                .values()
                .filter_map(|address_info| {
                    PlatformP2PKHAddress::from_address(&address_info.address).ok()
                }),
            account.address_balances.keys().copied(),
        );
        let candidates = candidate_addresses
            .into_iter()
            .filter_map(|p2pkh| {
                let balance = account.address_credit_balance(&p2pkh);
                (balance > 0).then_some((PlatformAddress::P2pkh(p2pkh.to_bytes()), balance))
            })
            .collect();

        let platform_version = self.sdk.version();
        let state_transition_version = &platform_version.dpp.state_transitions;
        plan_shield_inputs(
            candidates,
            shield_fee_reserve_credits(platform_version)?,
            state_transition_version.address_funds.min_input_amount,
            usize::from(state_transition_version.max_address_inputs),
        )
    }

    /// Return a cached capacity snapshot for shielding from one Platform
    /// Payment account.
    ///
    /// This uses the exact planner later executed by
    /// [`shielded_shield_from_account`](Self::shielded_shield_from_account):
    /// Platform addresses are sorted lexicographically, the leading prefix
    /// through the first address able to retain the shared fee reserve is
    /// analyzed once, later addresses below the versioned minimum input amount
    /// are omitted, and the lexicographically earliest usable addresses are
    /// capped at the versioned maximum input count. The reported maximum is
    /// therefore executable under the wallet's deterministic ordering policy;
    /// it is not a balance-optimized subset. It performs no DAPI request,
    /// signing, proof construction, or broadcast. A normal no-capacity state is returned with
    /// `can_shield == false`; only missing wallet/account state or arithmetic
    /// overflow is an error.
    #[cfg(feature = "shielded")]
    pub async fn shielded_shield_preflight(
        &self,
        payment_account: u32,
    ) -> Result<ShieldedShieldPreflight, PlatformWalletError> {
        Ok(self
            .shielded_shield_plan_for_account(payment_account)
            .await?
            .preflight)
    }

    /// Shield credits from a Platform Payment account into the
    /// wallet's shielded pool, with the resulting note assigned
    /// to `shielded_account`'s default Orchard address.
    ///
    /// `payment_account` selects the source Platform Payment
    /// account (different concept from `shielded_account` — this
    /// is the BIP-44-style funding account on the transparent
    /// side, not the ZIP-32 Orchard account). Auto-selects input
    /// addresses from that account in lexicographic Platform-address
    /// order until the cumulative balance covers `amount` plus the
    /// versioned fee reserve ([`shield_fee_reserve_credits`]; the
    /// on-chain fee comes off input 0 via `DeductFromInput(0)`, so
    /// that much balance stays unclaimed on input 0 for the
    /// metered fee).
    ///
    /// The host supplies a `Signer<PlatformAddress>` — typically
    /// `&VTableSigner` from `KeychainSigner.handle` — which signs
    /// each input's pubkey-hash binding to the Orchard bundle.
    ///
    /// Returns `ShieldedNotBound` if no shielded sub-wallet is
    /// bound, `AddressOperation` if the platform-payment account
    /// at `payment_account` doesn't exist, or
    /// `PlatformShieldCapacityExceeded` if the selected Platform-address set
    /// can't cover `amount` plus the fee reserve.
    ///
    /// Succeeds once the shield's verified result reads every input address
    /// at or past the nonce the shield signed. The activity row stays pending
    /// until the shielded scan observes the note on-chain: the result proves
    /// the input addresses, not this shield or its note.
    #[cfg(feature = "shielded")]
    pub async fn shielded_shield_from_account<S, P>(
        &self,
        coordinator: &Arc<crate::wallet::shielded::NetworkShieldedCoordinator>,
        shielded_account: u32,
        payment_account: u32,
        amount: u64,
        signer: &S,
        prover: P,
    ) -> Result<(), PlatformWalletError>
    where
        S: dpp::identity::signer::Signer<dpp::address_funds::PlatformAddress> + Send + Sync,
        P: dpp::shielded::builder::OrchardProver,
    {
        self.shielded_shield_from_account_impl(
            coordinator,
            shielded_account,
            payment_account,
            None,
            amount,
            [0u8; 36], // empty memo
            signer,
            prover,
        )
        .await
    }

    /// Shield credits from a Platform Payment account into a THIRD-PARTY
    /// shielded pool: the resulting note is assigned to
    /// `recipient_raw_43` (a raw 43-byte Orchard payment address — the
    /// same shape [`shielded_transfer_to`](Self::shielded_transfer_to)
    /// takes) instead of the wallet's own default address. Input
    /// selection, fees, and error shapes are identical to
    /// [`shielded_shield_from_account`](Self::shielded_shield_from_account);
    /// the wallet still needs a bound shielded sub-wallet at
    /// `shielded_account` because the send is OVK-encrypted to (and its
    /// activity recorded under) that account — which is how the scan
    /// later recovers it as outgoing history.
    ///
    /// The recipient must actually be a third party: an address this
    /// account's own IVK recognizes (default or any diversified index)
    /// is rejected, because its note would be spendable here and the
    /// live `Sent`/`Out` row would diverge from the self-pay row a
    /// restore's scan derives. Self-shields go through
    /// [`shielded_shield_from_account`](Self::shielded_shield_from_account).
    ///
    /// `memo` is the 36-byte on-chain `DashMemo` encoding attached to
    /// the recipient's note (all-zero = no memo).
    #[cfg(feature = "shielded")]
    #[allow(clippy::too_many_arguments)]
    pub async fn shielded_shield_from_account_to_recipient<S, P>(
        &self,
        coordinator: &Arc<crate::wallet::shielded::NetworkShieldedCoordinator>,
        shielded_account: u32,
        payment_account: u32,
        recipient_raw_43: &[u8; 43],
        amount: u64,
        memo: [u8; 36],
        signer: &S,
        prover: P,
    ) -> Result<(), PlatformWalletError>
    where
        S: dpp::identity::signer::Signer<dpp::address_funds::PlatformAddress> + Send + Sync,
        P: dpp::shielded::builder::OrchardProver,
    {
        let recipient = Option::<grovedb_commitment_tree::PaymentAddress>::from(
            grovedb_commitment_tree::PaymentAddress::from_raw_address_bytes(recipient_raw_43),
        )
        .ok_or_else(|| {
            PlatformWalletError::ShieldedBuildError(
                "invalid Orchard payment address bytes".to_string(),
            )
        })?;
        self.shielded_shield_from_account_impl(
            coordinator,
            shielded_account,
            payment_account,
            Some(recipient),
            amount,
            memo,
            signer,
            prover,
        )
        .await
    }

    /// Shared body of the two shield entry points above; `recipient`
    /// `None` = the wallet's own default Orchard address.
    #[cfg(feature = "shielded")]
    #[allow(clippy::too_many_arguments)]
    async fn shielded_shield_from_account_impl<S, P>(
        &self,
        coordinator: &Arc<crate::wallet::shielded::NetworkShieldedCoordinator>,
        shielded_account: u32,
        payment_account: u32,
        recipient: Option<grovedb_commitment_tree::PaymentAddress>,
        amount: u64,
        memo: [u8; 36],
        signer: &S,
        prover: P,
    ) -> Result<(), PlatformWalletError>
    where
        S: dpp::identity::signer::Signer<dpp::address_funds::PlatformAddress> + Send + Sync,
        P: dpp::shielded::builder::OrchardProver,
    {
        // Preserve the boundary behavior for non-Swift hosts and avoid taking
        // the single-flight/account locks for a request that can never build.
        if amount == 0 {
            return Err(PlatformWalletError::ShieldedBuildError(
                "amount must be > 0".to_string(),
            ));
        }

        // Single-flight: serialize shield-class ops on this wallet so
        // two concurrent calls can't fetch + build with the same
        // address nonce (the second would be rejected as a replay after
        // a ~30 s proof). Held across selection → build → broadcast.
        let _shield_guard = self.shield_guard.lock().await;

        // Planning and amount selection are shared with the cached preflight.
        // The helper drops the wallet-manager read lock before the expensive
        // proof path, while the single-flight guard keeps two shields from
        // planning and broadcasting against the same address nonce.
        let inputs = self
            .shielded_shield_plan_for_account(payment_account)
            .await?
            .select_inputs(amount)?;

        // Clone the account's viewing keys and release the slot before
        // the proof: `shield` runs a Halo 2 proof plus a broadcast, and
        // holding the read guard across it would block the bind path's
        // slot write for that whole window — which, because a bind holds
        // the coordinator's lifecycle mutex while waiting for it, would
        // stall wallet removal and Clear for every wallet on the network
        // (tokio's RwLock is write-preferring, so queued readers pile up
        // behind that write too).
        let keyset = {
            let guard = self.shielded_keys.read().await;
            let keys = guard
                .as_ref()
                .ok_or(PlatformWalletError::ShieldedNotBound)?;
            keys.get(&shielded_account)
                .ok_or_else(|| {
                    PlatformWalletError::ShieldedKeyDerivation(format!(
                        "shielded account {shielded_account} not bound"
                    ))
                })?
                .clone()
        };
        super::shielded::operations::shield_to(
            &self.sdk,
            coordinator.store(),
            Some(&self.persister),
            self.wallet_id,
            &keyset,
            shielded_account,
            recipient.as_ref(),
            inputs,
            amount,
            memo,
            signer,
            &prover,
        )
        .await
    }

    /// Stop retrying one selected identity debit while retaining its unknown
    /// outcome. Requires explicit acknowledgement that it may already have
    /// executed or may execute later; this does not cancel the signed payment.
    #[cfg(feature = "shielded")]
    pub async fn abandon_shielded_identity_debit(
        &self,
        coordinator: &Arc<crate::wallet::shielded::NetworkShieldedCoordinator>,
        account_index: u32,
        activity_id: [u8; 32],
        acknowledge_possible_execution: bool,
    ) -> Result<(), PlatformWalletError> {
        let _shield_guard = self.shield_guard.lock().await;
        coordinator
            .abandon_identity_debit(
                self.wallet_id(),
                account_index,
                activity_id,
                acknowledge_possible_execution,
            )
            .await
    }

    /// Shield credits from one of this wallet's Platform identities straight into
    /// the wallet's shielded pool (`ShieldFromIdentity`, type 21). The note is
    /// assigned to `shielded_account`'s default Orchard address; `identity_id` must
    /// be an identity this wallet manages, and `signer` must hold its TRANSFER key
    /// (typically the same `Signer<IdentityPublicKey>` used for credit transfers).
    ///
    /// The identity is debited `amount` plus the metered fee plus the shielded
    /// compute fee. Returns the proven post-debit balance, which is also applied
    /// to the managed identity. Persistence of this local balance cache is
    /// best-effort after broadcast: a storage failure is logged and does not
    /// turn the payment into a retryable failure. Reloading after such a failure
    /// can show the prior cached balance until it is refreshed from Platform.
    /// A result proof that is not this identity's balance proof reports the
    /// spend as unconfirmed
    /// ([`PlatformWalletError::ShieldedSpendUnconfirmed`]), as does any failure
    /// verdict the proven identity nonce cannot rule out (do not rebuild on it).
    /// The activity row stays pending until the shielded scan observes the note
    /// on-chain; the balance proof alone does not confirm it.
    #[cfg(feature = "shielded")]
    pub async fn shielded_shield_from_identity<S, P>(
        &self,
        coordinator: &Arc<crate::wallet::shielded::NetworkShieldedCoordinator>,
        shielded_account: u32,
        identity_id: &Identifier,
        amount: u64,
        signer: &S,
        prover: P,
    ) -> Result<Credits, PlatformWalletError>
    where
        S: dpp::identity::signer::Signer<IdentityPublicKey> + Send + Sync,
        P: dpp::shielded::builder::OrchardProver,
    {
        self.shielded_shield_from_identity_impl(
            coordinator,
            shielded_account,
            None,
            identity_id,
            amount,
            [0u8; 36],
            signer,
            prover,
        )
        .await
    }

    /// [`shielded_shield_from_identity`](Self::shielded_shield_from_identity) with
    /// the note paid to a THIRD-PARTY Orchard address (`recipient_raw_43`, same shape
    /// as [`shielded_transfer_to`](Self::shielded_transfer_to)) and an optional memo.
    /// An address this account's own IVK recognizes is rejected; self-shields use the
    /// entry point without a recipient.
    #[cfg(feature = "shielded")]
    #[allow(clippy::too_many_arguments)]
    pub async fn shielded_shield_from_identity_to_recipient<S, P>(
        &self,
        coordinator: &Arc<crate::wallet::shielded::NetworkShieldedCoordinator>,
        shielded_account: u32,
        identity_id: &Identifier,
        recipient_raw_43: &[u8; 43],
        amount: u64,
        memo: [u8; 36],
        signer: &S,
        prover: P,
    ) -> Result<Credits, PlatformWalletError>
    where
        S: dpp::identity::signer::Signer<IdentityPublicKey> + Send + Sync,
        P: dpp::shielded::builder::OrchardProver,
    {
        let recipient = Option::<grovedb_commitment_tree::PaymentAddress>::from(
            grovedb_commitment_tree::PaymentAddress::from_raw_address_bytes(recipient_raw_43),
        )
        .ok_or_else(|| {
            PlatformWalletError::ShieldedBuildError(
                "invalid Orchard payment address bytes".to_string(),
            )
        })?;
        self.shielded_shield_from_identity_impl(
            coordinator,
            shielded_account,
            Some(recipient),
            identity_id,
            amount,
            memo,
            signer,
            prover,
        )
        .await
    }

    #[cfg(feature = "shielded")]
    #[allow(clippy::too_many_arguments)]
    async fn shielded_shield_from_identity_impl<S, P>(
        &self,
        coordinator: &Arc<crate::wallet::shielded::NetworkShieldedCoordinator>,
        shielded_account: u32,
        recipient: Option<grovedb_commitment_tree::PaymentAddress>,
        identity_id: &Identifier,
        amount: u64,
        memo: [u8; 36],
        signer: &S,
        prover: P,
    ) -> Result<Credits, PlatformWalletError>
    where
        S: dpp::identity::signer::Signer<IdentityPublicKey> + Send + Sync,
        P: dpp::shielded::builder::OrchardProver,
    {
        if amount == 0 {
            return Err(PlatformWalletError::ShieldedBuildError(
                "amount must be > 0".to_string(),
            ));
        }

        // Single-flight with the address-funded shields: the identity nonce is
        // fetched inside the operation, and two concurrent builds would race it.
        let _shield_guard = self.shield_guard.lock().await;

        let identity = {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            info.identity_manager
                .identity(identity_id)
                .map(|m| m.identity.clone())
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?
        };

        let keyset = {
            let guard = self.shielded_keys.read().await;
            let keys = guard
                .as_ref()
                .ok_or(PlatformWalletError::ShieldedNotBound)?;
            keys.get(&shielded_account)
                .ok_or_else(|| {
                    PlatformWalletError::ShieldedKeyDerivation(format!(
                        "shielded account {shielded_account} not bound"
                    ))
                })?
                .clone()
        };
        let (mut new_balance, metadata) = super::shielded::operations::shield_from_identity_to(
            &self.sdk,
            coordinator.store(),
            Some(&self.persister),
            self.wallet_id,
            &keyset,
            shielded_account,
            recipient.as_ref(),
            &identity,
            amount,
            memo,
            signer,
            &prover,
        )
        .await?;

        // The operation only received a clone of the identity, so the managed
        // identity still carries the pre-debit balance. Apply the proven
        // post-debit balance and persist the snapshot (the pattern
        // `transfer_credits_to_addresses_with_external_signer` follows).
        {
            let mut wm = self.wallet_manager.write().await;
            let managed = wm
                .get_wallet_info_mut(&self.wallet_id)
                .and_then(|info| info.identity_manager.managed_identity_mut(identity_id));
            if let Some(managed) = managed {
                new_balance = managed.persist_confirmed_balance(
                    new_balance,
                    BlockTime::from(metadata),
                    &self.persister,
                );
            }
        }

        Ok(new_balance)
    }
}

impl PlatformWallet {
    // TODO: What these methods for? can we remove? Don't deelete this todo
    /// Queue a changeset for later persistence.
    pub fn queue_persist(&self, changeset: PlatformWalletChangeSet) {
        if let Err(e) = self.persister.store(changeset) {
            tracing::error!(
                error = %e,
                wallet_id = %hex::encode(self.wallet_id),
                "Failed to queue changeset for persistence"
            );
        }
    }

    /// Flush all queued changesets to the storage backend.
    pub fn flush_persist(&self) -> Result<(), PersistenceError> {
        self.persister.flush()
    }

    /// Load persisted state for this wallet.
    ///
    /// Calls the backend inline, without the `spawn_blocking` offload that
    /// [`PlatformWalletManager::load_from_persistor`](crate::manager::PlatformWalletManager::load_from_persistor)
    /// and wallet registration use. A slow backend blocks the calling thread —
    /// an async caller's runtime worker included.
    pub fn load_persisted(&self) -> Result<ClientStartState, PersistenceError> {
        self.persister.load()
    }

    /// Apply a [`PlatformWalletChangeSet`] to this wallet's in-memory
    /// state under the wallet manager write lock.
    ///
    /// Delegates to [`PlatformWalletInfo::apply_changeset`], which is
    /// the canonical restore path. Holds the `WalletManager` write
    /// lock for the duration so the split borrow of `(&mut Wallet,
    /// &mut PlatformWalletInfo)` is safe — `&mut Wallet` is needed so
    /// the core sub-changeset can re-derive HD accounts via
    /// `Wallet::add_account`.
    ///
    /// Returns [`ApplyError::WalletNotFound`](crate::wallet::ApplyError::WalletNotFound)
    /// if the wallet has been removed from the manager between handle
    /// acquisition and this call.
    ///
    /// Consumes the changeset by value — `apply_changeset` drains
    /// every map straight into the wallet maps with no clones.
    pub async fn apply(
        &self,
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), crate::wallet::ApplyError> {
        // The platform-address sync watermark lives on the provider,
        // not on `PlatformWalletInfo`. Pull it out before handing the
        // changeset to `apply_changeset` (which consumes by value), then
        // feed it to the providers once apply completes.
        let pa_sync_state = changeset.platform_addresses.as_ref().map(|pa| {
            (
                pa.sync_height,
                pa.sync_timestamp,
                pa.last_known_recent_block,
            )
        });

        {
            let mut wm = self.wallet_manager.write().await;
            let (wallet, info) = wm
                .get_wallet_mut_and_info_mut(&self.wallet_id)
                .ok_or(crate::wallet::ApplyError::WalletNotFound(self.wallet_id))?;
            info.apply_changeset(wallet, changeset)?;
        }

        if let Some((height, timestamp, recent_block)) = pa_sync_state {
            self.platform
                .apply_sync_state(height, timestamp, recent_block)
                .await;
        }
        Ok(())
    }

    /// Load persisted state from the persister and apply it to the
    /// in-memory wallet. Convenience wrapper for
    /// `apply(load_persisted()?)`.
    ///
    /// **Idempotent** — safe to call multiple times. The apply path
    /// uses monotonic / OR merges on every field it touches
    /// (`highest_used` is MAX-merged, `utxos_instant_locked` is
    /// set-union), so re-applying the same persisted state is a no-op.
    ///
    /// This is the recommended entry point for startup hydration
    /// *after* late-registered accounts (e.g. DashPay contact
    /// accounts that `bootstrap_dashpay_contact_accounts` adds) have
    /// landed. The initial load_and_apply called during
    /// `PlatformWallet` construction only hydrates state for
    /// accounts that exist at that point; a second call after
    /// account bootstrap picks up the rest without regressing
    /// anything.
    ///
    /// Inherits [`load_persisted`](Self::load_persisted)'s inline read with no
    /// offload. A host that wants one must wrap this call itself.
    pub async fn load_and_apply_persisted(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let ClientStartState {
            mut platform_addresses,
            wallets: _,
            #[cfg(feature = "shielded")]
                shielded: _,
            retained_private_contact_scan_states: _,
        } = self.load_persisted()?;

        if let Some(persisted) = platform_addresses.remove(&self.wallet_id) {
            self.platform
                .initialize_from_persisted(persisted)
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        }

        Ok(())
    }
}

impl Clone for PlatformWallet {
    fn clone(&self) -> Self {
        Self {
            wallet_id: self.wallet_id,
            sdk: self.sdk.clone(),
            wallet_manager: self.wallet_manager.clone(),
            core: self.core.clone(),
            identity: self.identity.clone(),
            platform: self.platform.clone(),
            asset_locks: self.asset_locks.clone(),
            persister: self.persister.clone(),
            generation: self.generation.clone(),
            #[cfg(feature = "shielded")]
            shielded_keys: self.shielded_keys.clone(),
            #[cfg(feature = "shielded")]
            shield_guard: self.shield_guard.clone(),
            #[cfg(feature = "shielded")]
            shielded_detached: self.shielded_detached.clone(),
            #[cfg(feature = "shielded")]
            private_payment_restores: self.private_payment_restores.clone(),
        }
    }
}

impl std::fmt::Debug for PlatformWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformWallet")
            .field("wallet_id", &hex::encode(self.wallet_id))
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Wallet state guard types — lock WalletManager, deref to PlatformWalletInfo
// ---------------------------------------------------------------------------

/// Read guard that locks `WalletManager` and derefs to this wallet's
/// `PlatformWalletInfo`. Also provides `.wallet()` for key material access.
pub struct WalletStateReadGuard<'a> {
    guard: RwLockReadGuard<'a, WalletManager<PlatformWalletInfo>>,
    wallet_id: WalletId,
}

impl<'a> WalletStateReadGuard<'a> {
    /// Access the immutable `Wallet` (key material).
    pub fn wallet(&self) -> &Wallet {
        self.guard
            .get_wallet(&self.wallet_id)
            .expect("wallet exists in guard")
    }
}

impl Deref for WalletStateReadGuard<'_> {
    type Target = PlatformWalletInfo;
    fn deref(&self) -> &PlatformWalletInfo {
        self.guard
            .get_wallet_info(&self.wallet_id)
            .expect("wallet exists in guard")
    }
}

/// Write guard that locks `WalletManager` and derefs to this wallet's
/// `PlatformWalletInfo` (with `DerefMut`). Also provides `.wallet()`.
pub struct WalletStateWriteGuard<'a> {
    guard: RwLockWriteGuard<'a, WalletManager<PlatformWalletInfo>>,
    wallet_id: WalletId,
}

impl<'a> WalletStateWriteGuard<'a> {
    /// Access the immutable `Wallet` (key material).
    pub fn wallet(&self) -> &Wallet {
        self.guard
            .get_wallet(&self.wallet_id)
            .expect("wallet exists in guard")
    }
}

impl Deref for WalletStateWriteGuard<'_> {
    type Target = PlatformWalletInfo;
    fn deref(&self) -> &PlatformWalletInfo {
        self.guard
            .get_wallet_info(&self.wallet_id)
            .expect("wallet exists in guard")
    }
}

impl DerefMut for WalletStateWriteGuard<'_> {
    fn deref_mut(&mut self) -> &mut PlatformWalletInfo {
        self.guard
            .get_wallet_info_mut(&self.wallet_id)
            .expect("wallet exists in guard")
    }
}

/// The commitments of the entries a saved private contact request payment
/// appends, read from its signed transition. Refused with
/// [`PlatformWalletError::ShieldedRecoveryCorrupted`] when the transition does
/// not decode, is not a private document create (type 29) appending to
/// DashPay's `privateContactRequest` store (`dashpay_id` being DashPay's
/// contract id), carries no entry, or carries one shorter than its 32-byte
/// commitment: the payment's pair is restored from it, so a payment that does
/// not read back fails the restoration rather than release the holds.
#[cfg(feature = "shielded")]
fn associated_payment_commitments(
    redrive: &PendingRedrive,
    dashpay_id: Identifier,
) -> Result<Vec<PrivatePaymentEntry>, PlatformWalletError> {
    let corrupted = |reason: String| PlatformWalletError::ShieldedRecoveryCorrupted {
        account_index: None,
        reason,
    };
    let transition = match StateTransition::deserialize_from_bytes_trusted(&redrive.st_bytes) {
        Ok(StateTransition::PrivateDocumentCreateFromShieldedPool(transition)) => transition,
        Ok(_) => {
            return Err(corrupted(
                "a saved private contact request payment's transition is not a private \
                 document create"
                    .to_string(),
            ))
        }
        Err(error) => {
            return Err(corrupted(format!(
                "a saved private contact request payment's transition does not decode: {error}"
            )))
        }
    };
    if transition.data_contract_id() != dashpay_id
        || transition.document_type_name() != PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME
    {
        return Err(corrupted(format!(
            "a saved private contact request payment appends to {} {}, not DashPay's \
             {PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME}",
            transition.data_contract_id(),
            transition.document_type_name()
        )));
    }
    if transition.entries().is_empty() {
        return Err(corrupted(
            "a saved private contact request payment carries no entry".to_string(),
        ));
    }
    transition
        .entries()
        .iter()
        .map(|entry| {
            let commitment = entry
                .as_slice()
                .get(..32)
                .and_then(|commitment| <[u8; 32]>::try_from(commitment).ok())
                .ok_or_else(|| {
                    corrupted(
                        "a saved private contact request payment's entry is shorter than its \
                         commitment"
                            .to_string(),
                    )
                })?;
            Ok(PrivatePaymentEntry {
                commitment,
                entry_hash: Sha256::digest(entry.as_slice()).into(),
            })
        })
        .collect()
}

/// The handles a restoration of a wallet's private contact request payments
/// from the shielded store's journal runs on, owned, so one the store is
/// busy for runs on a task ([`PlatformWallet::restore_private_contact_payments`]).
#[cfg(feature = "shielded")]
struct PrivatePaymentRestore {
    coordinator: Arc<NetworkShieldedCoordinator>,
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    persister: WalletPersister,
    generation: Arc<WalletGeneration>,
    restores: Arc<std::sync::Mutex<PrivatePaymentRestores>>,
    wallet_id: WalletId,
    network: dashcore::Network,
}

/// A deferred restoration, freeing its queue
/// ([`PrivatePaymentRestore::finish_deferred`]) however its task ends: run,
/// panicked, or dropped with its runtime or by an abort, polled or not.
/// Otherwise a task that ended early would leave the queue taken, so no later
/// bind would queue another and the wallet would stay held until a configure
/// or a restart.
#[cfg(feature = "shielded")]
struct DeferredFinish(PrivatePaymentRestore);

#[cfg(feature = "shielded")]
impl Drop for DeferredFinish {
    fn drop(&mut self) {
        self.0.finish_deferred();
    }
}

/// A wallet generation's restorations of its private contact request
/// payments ([`PlatformWallet::restore_private_payments_or_defer`]). Weak, so
/// a store dropped since is never taken for a new one at the same address.
#[cfg(feature = "shielded")]
#[derive(Default)]
struct PrivatePaymentRestores {
    /// The store a restoration last succeeded from.
    restored_from: std::sync::Weak<RwLock<FileBackedShieldedStore>>,
    /// The store a deferred restoration is queued for.
    queued_for: std::sync::Weak<RwLock<FileBackedShieldedStore>>,
    /// How many deferred restorations were queued, for tests.
    #[cfg(test)]
    queued: usize,
}

#[cfg(feature = "shielded")]
impl PrivatePaymentRestore {
    /// Restore, and raise or release the wallet's hold as it went: released
    /// once a restoration succeeds, raised when it fails, and when it cannot
    /// run yet unless this generation already restored from this store.
    /// `Ok(None)` when the store is busy and `wait` is false.
    async fn restore_and_release(
        &self,
        wait: bool,
    ) -> Result<Option<Vec<(Identifier, Identifier)>>, PlatformWalletError> {
        let restored = self.restore(wait).await;
        match &restored {
            Ok(Some(_)) => {
                self.restores_lock().restored_from = Arc::downgrade(self.coordinator.store());
                self.persister.note_private_payment_restore(None);
                self.persister.private_payments_restored();
            }
            Ok(None) if self.restored_from_this_store() => {}
            Ok(None) => self.persister.hold_until_private_payments_restored(),
            Err(error) => {
                self.persister
                    .note_private_payment_restore(Some(error.to_string()));
                self.persister.hold_until_private_payments_restored();
            }
        }
        restored
    }

    /// Whether the requests the wallet tracks are durable now: at once for a
    /// backend that commits inline, as each was stored before it was
    /// tracked; after a flush that succeeds otherwise.
    fn settled_requests_durable(&self) -> bool {
        if self.persister.store_commits_inline() {
            return true;
        }
        match self.persister.flush() {
            Ok(()) => true,
            Err(error) => {
                tracing::warn!(
                    wallet_id = %hex::encode(self.wallet_id),
                    %error,
                    "the requests settling private payments are not flushed; their journal \
                     records stay for a later restoration"
                );
                false
            }
        }
    }

    fn restores_lock(&self) -> std::sync::MutexGuard<'_, PrivatePaymentRestores> {
        self.restores
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Whether this generation already restored from this coordinator's store.
    fn restored_from_this_store(&self) -> bool {
        self.restores_lock()
            .restored_from
            .upgrade()
            .is_some_and(|store| Arc::ptr_eq(&store, self.coordinator.store()))
    }

    /// Queue a deferred restoration from this store, unless one is queued
    /// already. Returns whether it was queued.
    fn queue_deferred(&self) -> bool {
        let mut restores = self.restores_lock();
        if restores
            .queued_for
            .upgrade()
            .is_some_and(|store| Arc::ptr_eq(&store, self.coordinator.store()))
        {
            return false;
        }
        restores.queued_for = Arc::downgrade(self.coordinator.store());
        #[cfg(test)]
        {
            restores.queued += 1;
        }
        true
    }

    /// The deferred restoration from this store ran: another may be queued.
    fn finish_deferred(&self) {
        let mut restores = self.restores_lock();
        if restores
            .queued_for
            .upgrade()
            .is_some_and(|store| Arc::ptr_eq(&store, self.coordinator.store()))
        {
            restores.queued_for = std::sync::Weak::new();
        }
    }

    /// The restoration [`PlatformWallet::restore_private_contact_payments`]
    /// documents, fenced against the wallet's replacement.
    ///
    /// It holds no lifecycle gate, so a removal never waits for its store
    /// waits. The fence is the step that reserves the pairs
    /// (`reserve_unresolved_private_payments`), which checks the wallet's
    /// generation under the wallet manager's write lock, the one a removal
    /// takes: a restoration of a wallet removed, or re-created under its id,
    /// reserves nothing on the replacement and fails. What it does before
    /// that holds whatever the generation: the journal's read-back
    /// discharges and migrations are facts about the shared store, and the
    /// holds it raises or releases are this generation's own.
    async fn restore(
        &self,
        wait: bool,
    ) -> Result<Option<Vec<(Identifier, Identifier)>>, PlatformWalletError> {
        if self.coordinator.network() != self.network {
            return Err(PlatformWalletError::InvalidParameter(format!(
                "the shielded coordinator is for {:?}, this wallet for {:?}",
                self.coordinator.network(),
                self.network
            )));
        }
        let current = {
            let wm = self.wallet_manager.read().await;
            wm.get_wallet_info(&self.wallet_id)
                .is_some_and(|info| Arc::ptr_eq(&info.generation, &self.generation))
        };
        if !current {
            return Err(PlatformWalletError::WalletNotFound(hex::encode(
                self.wallet_id,
            )));
        }
        let dashpay_id = dashpay_contract()?.id();
        // Read-backs whose flush failed transiently are noted once a flush
        // succeeds now, so the journal read below discharges their records.
        self.persister
            .note_read_back_once_durable(std::iter::empty());
        let (payments, journal_authoritative) = {
            let mut store = if wait {
                self.coordinator.store().write().await
            } else {
                match self.coordinator.store().try_write() {
                    Ok(store) => store,
                    Err(_) => return Ok(None),
                }
            };
            read_private_payment_journal(&mut *store, &self.persister, self.wallet_id, dashpay_id)?
        };
        let (restored, settled) = restore_journaled_private_payments(
            &self.wallet_manager,
            &self.generation,
            &self.persister,
            self.wallet_id,
            &payments,
            journal_authoritative,
        )
        .await?;
        // A record whose request the tracked one records or supersedes was
        // read back by an earlier process, which ended before it discharged
        // it: discharged now, so its store is not read for it on every launch.
        // A busy store leaves it for the next restoration.
        //
        // Only once what records that request is durable, as a restart would
        // otherwise find neither it nor the record. A backend that commits
        // inline stored it before the wallet tracked it. Any other is flushed
        // first, and a flush that fails leaves the records for a later
        // restoration; so does a flush that failed for good earlier in this
        // process, after which the tracked request may be in memory only. A
        // record whose read-back still waits for its flush is left to that
        // path, which discharges it once a flush succeeds.
        let settled: Vec<[u8; 32]> = settled
            .into_iter()
            .filter(|commitment| !self.persister.read_back_awaiting_flush(commitment))
            .collect();
        if !settled.is_empty()
            && !self.persister.buffered_writes_lost()
            && self.settled_requests_durable()
        {
            let store = if wait {
                Some(self.coordinator.store().write().await)
            } else {
                self.coordinator.store().try_write().ok()
            };
            if let Some(mut store) = store {
                let discharged: Vec<[u8; 32]> = settled
                    .into_iter()
                    .filter(|commitment| {
                        store
                            .discharge_private_payment(self.wallet_id, commitment)
                            .inspect_err(|error| {
                                tracing::warn!(
                                    wallet_id = %hex::encode(self.wallet_id),
                                    %error,
                                    "a settled private payment's journal record was not \
                                     discharged; the next restoration discharges it"
                                )
                            })
                            .is_ok()
                    })
                    .collect();
                drop(store);
                self.persister.note_discharged_private_entries(discharged);
            }
        }
        // Opened only once a restoration succeeded: a restoration that failed
        // closed over a deleted or replaced store fails the same way until a
        // known-good store is back. One left closed here is opened by the
        // next restoration, which no unresolved marker holds back now.
        if !journal_authoritative {
            let opened = if wait {
                Some(self.coordinator.store().write().await)
            } else {
                self.coordinator.store().try_write().ok()
            }
            .map(|mut store| store.open_private_payment_journal(self.wallet_id));
            if let Some(Err(error)) = opened {
                tracing::warn!(
                    wallet_id = %hex::encode(self.wallet_id),
                    %error,
                    "the private payment journal was not opened; the next restoration opens it"
                );
            }
        }
        Ok(Some(restored))
    }
}

/// Read `wallet_id`'s private payment journal for a restoration, as
/// (sender, recipient, account reference, entry commitment), with whether it
/// is authoritative (open: a restoration from it succeeded before). The
/// read-back notices are discharged first. While the journal is new, a spend
/// saved before it kept the wallet's payments is accounted for: one paying
/// for a private contact request with its request saved is added to the
/// journal, one with no request, or whose transition does not decode and
/// that no record accounts for, fails the read
/// ([`PlatformWalletError::ShieldedRecoveryCorrupted`]).
#[cfg(feature = "shielded")]
fn read_private_payment_journal<S: ShieldedStore>(
    store: &mut S,
    persister: &WalletPersister,
    wallet_id: WalletId,
    dashpay_id: Identifier,
) -> Result<(Vec<JournaledPrivatePayment>, bool), PlatformWalletError>
where
    PlatformWalletError: From<S::Error>,
{
    discharge_read_back_private_payments(store, persister, wallet_id);
    let journal_authoritative = store.private_payment_journal_open(wallet_id)?;
    let mut records = store.private_payment_journal(wallet_id)?;
    let journaled: std::collections::BTreeSet<[u8; 32]> =
        records.iter().map(|record| record.commitment).collect();
    let accounted: std::collections::BTreeSet<[u8; 32]> =
        records.iter().map(|record| record.activity_id).collect();
    let corrupted =
        |account_index: u32, reason: &str| PlatformWalletError::ShieldedRecoveryCorrupted {
            account_index: Some(account_index),
            reason: reason.to_string(),
        };
    // The private contact request payments saved before the journal kept
    // them: type 29 spends to DashPay's store no record accounts for.
    // While the journal is new only: once a restoration from it succeeded,
    // every payment is journaled with its redrive record in one write, so a
    // record no journal record accounts for paid for nothing, or was
    // discharged.
    let mut unjournaled = Vec::new();
    let saved_before_the_journal = if journal_authoritative {
        Vec::new()
    } else {
        store.pending_redrives_for_wallet(wallet_id)?
    };
    for (id, redrive) in saved_before_the_journal {
        // Identity debits are never private document payments.
        if redrive.nullifiers.is_empty() || accounted.contains(&redrive.activity_id) {
            continue;
        }
        let transition = match StateTransition::deserialize_from_bytes_trusted(&redrive.st_bytes) {
            Ok(StateTransition::PrivateDocumentCreateFromShieldedPool(transition)) => transition,
            Ok(_) => continue,
            Err(_) => {
                return Err(corrupted(
                    id.account_index,
                    "a saved spend's transition does not decode, and no private payment journal \
                     record shows it pays for no private contact request",
                ))
            }
        };
        if transition.data_contract_id() == dashpay_id
            && transition.document_type_name() == PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME
        {
            unjournaled.push((id, redrive));
        }
    }
    // Read only when needed, so a damaged association of a payment the
    // journal keeps holds nothing up: for an unjournaled payment, and for a
    // new journal, which accounts for nothing saved before it, damaged
    // records retained without being loaded included.
    let associations: BTreeMap<[u8; 32], PrivateContactPayment> = if journal_authoritative {
        BTreeMap::new()
    } else {
        store
            .private_contact_redrives_for_wallet(wallet_id)?
            .into_iter()
            .map(|(redrive, payment)| (redrive.activity_id, payment))
            .collect()
    };
    let mut migrated = Vec::new();
    for (id, redrive) in unjournaled {
        let Some(payment) = associations.get(&redrive.activity_id) else {
            return Err(corrupted(
                id.account_index,
                "a private contact request payment saved before the journal kept them carries \
                 no request, so its pair cannot be restored",
            ));
        };
        for entry in associated_payment_commitments(&redrive, dashpay_id)? {
            if !journaled.contains(&entry.commitment) {
                migrated.push(PrivatePaymentRecord {
                    account_index: id.account_index,
                    activity_id: redrive.activity_id,
                    anchor: redrive.anchor,
                    // As the record keeps it: unknown for one saved before
                    // heights were kept, whose expiry is then never proven,
                    // so its pair stays held.
                    anchor_height: redrive.anchor_height,
                    commitment: entry.commitment,
                    payment: *payment,
                    nullifiers: redrive.nullifiers.clone(),
                    entry_hash: Some(entry.entry_hash),
                    // Not kept with a record saved before the journal: its
                    // expiry reads the store from its start.
                    store_position: None,
                });
            }
        }
    }
    if !migrated.is_empty() {
        store.journal_private_payments(wallet_id, &migrated)?;
        records.extend(migrated);
    }
    Ok((
        records
            .into_iter()
            .map(|record| {
                (
                    Identifier::from(record.payment.sender),
                    Identifier::from(record.payment.recipient),
                    record.payment.account_reference,
                    record.commitment,
                    record.store_position,
                )
            })
            .collect(),
        journal_authoritative,
    ))
}

/// The entries of every private document create (type 29) among `redrives`
/// that appends to DashPay's `privateContactRequest` store, `dashpay_id`
/// being DashPay's contract id. A record whose transition does not decode
/// is skipped.
#[cfg(feature = "shielded")]
fn private_contact_request_redrive_entries<'a>(
    redrives: impl IntoIterator<Item = &'a PendingRedrive>,
    dashpay_id: Identifier,
) -> Vec<BinaryData> {
    redrives
        .into_iter()
        .filter_map(|redrive| {
            match StateTransition::deserialize_from_bytes_trusted(&redrive.st_bytes) {
                Ok(StateTransition::PrivateDocumentCreateFromShieldedPool(transition))
                    if transition.data_contract_id() == dashpay_id
                        && transition.document_type_name()
                            == PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME =>
                {
                    Some(transition.entries().to_vec())
                }
                _ => None,
            }
        })
        .flatten()
        .collect()
}

/// Verify a bech32m recipient's network class matches `network` before decoding.
///
/// The address decoder is network-agnostic (`tdash` is shared by
/// Testnet/Devnet/Regtest), so the wrong-network guard lives here. Network
/// classification (mainnet vs non-mainnet, plus malformed/non-platform input
/// rejection) is delegated to [`PlatformAddress::is_mainnet_bech32m`]. A
/// mainnet wallet requires a mainnet (`dash`) address; any non-mainnet wallet
/// requires a non-mainnet (`tdash`) address.
#[cfg(feature = "shielded")]
fn check_recipient_hrp(
    recipient: &str,
    network: dashcore::Network,
) -> Result<(), PlatformWalletError> {
    use dpp::address_funds::PlatformAddress;

    let addr_is_mainnet = PlatformAddress::is_mainnet_bech32m(recipient).map_err(|e| {
        PlatformWalletError::ShieldedBuildError(format!("invalid platform address: {e}"))
    })?;
    if addr_is_mainnet != (network == dashcore::Network::Mainnet) {
        let addr_class = if addr_is_mainnet {
            "mainnet"
        } else {
            "non-mainnet"
        };
        return Err(PlatformWalletError::ShieldedBuildError(format!(
            "platform address network mismatch: {addr_class} address, wallet {network:?}"
        )));
    }
    Ok(())
}

#[cfg(all(test, feature = "shielded"))]
mod private_contact_request_redrive_tests {
    use super::*;
    use dpp::serialization::PlatformSerializable;
    use dpp::shielded::SerializedAction;
    use dpp::state_transition::private_document_create_from_shielded_pool_transition::methods::PrivateDocumentCreateFromShieldedPoolTransitionMethodsV0;
    use dpp::state_transition::private_document_create_from_shielded_pool_transition::PrivateDocumentCreateFromShieldedPoolTransition;
    use dpp::version::PlatformVersion;

    /// A type 29 transition appending `entries` to the store of
    /// `document_type_name` of `data_contract_id`, with placeholder actions.
    fn private_document_create(
        data_contract_id: Identifier,
        document_type_name: &str,
        entries: Vec<BinaryData>,
    ) -> StateTransition {
        let action = SerializedAction {
            nullifier: [1; 32],
            rk: [2; 32],
            cmx: [3; 32],
            encrypted_note: vec![4; 216],
            cv_net: [5; 32],
            spend_auth_sig: [6; 64],
        };
        PrivateDocumentCreateFromShieldedPoolTransition::try_from_bundle(
            data_contract_id,
            document_type_name.to_string(),
            entries,
            vec![action.clone(), action],
            0,
            [7; 32],
            vec![8; 64],
            [9; 64],
            PlatformVersion::latest(),
        )
        .expect("a type 29 transition")
    }

    /// The unconfirmed spend that broadcast `st_bytes`.
    fn redrive(st_bytes: Vec<u8>) -> PendingRedrive {
        PendingRedrive {
            activity_id: [1; 32],
            anchor: [7; 32],
            nullifiers: vec![[1; 32]],
            st_bytes,
            attempts: 0,
            identity_nonce_finalized: false,
            identity_user_abandoned: false,
            anchor_height: None,
        }
    }

    /// Only the entries of type 29 transitions to DashPay's private contact
    /// request store are read: not another contract's, not another document
    /// type's, and not a record that does not decode.
    #[test]
    fn should_read_dashpays_private_contact_requests_from_unconfirmed_spends() {
        let dashpay_id = dashpay_contract().expect("bundled DashPay").id();
        let entry = |byte: u8| BinaryData::new(vec![byte; 589]);
        let serialized = |transition: StateTransition| {
            transition
                .serialize_to_bytes()
                .expect("the transition serializes")
        };
        let redrives = [
            redrive(serialized(private_document_create(
                dashpay_id,
                PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
                vec![entry(1), entry(2)],
            ))),
            redrive(serialized(private_document_create(
                Identifier::new([4; 32]),
                PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
                vec![entry(3)],
            ))),
            redrive(serialized(private_document_create(
                dashpay_id,
                "privateNote",
                vec![entry(4)],
            ))),
            redrive(vec![0xFF; 16]),
            redrive(serialized(private_document_create(
                dashpay_id,
                PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
                vec![entry(5)],
            ))),
        ];
        assert_eq!(
            private_contact_request_redrive_entries(&redrives, dashpay_id),
            vec![entry(1), entry(2), entry(5)]
        );
        assert_eq!(
            private_contact_request_redrive_entries(&[] as &[PendingRedrive], dashpay_id),
            Vec::<BinaryData>::new()
        );
    }
}

#[cfg(all(test, feature = "shielded"))]
mod private_contact_request_payment_tests {
    use super::*;
    use crate::events::{EventHandler, PlatformEventHandler};
    use crate::wallet::persister::NoPlatformPersistence;
    use crate::wallet::shielded::FileBackedShieldedStore;
    use crate::PlatformWalletManager;
    use grovedb_commitment_tree::ProvingKey;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use std::time::Duration;

    const SEED: [u8; 64] = [0x42; 64];

    struct NoopEventHandler;
    impl EventHandler for NoopEventHandler {}
    impl PlatformEventHandler for NoopEventHandler {}

    /// A prover the refused or unfunded payments here never reach.
    struct UnreachableProver;
    impl OrchardProver for UnreachableProver {
        fn proving_key(&self) -> &ProvingKey {
            panic!("nothing is proved")
        }
    }

    fn coordinator_at(dir: &std::path::Path) -> Arc<NetworkShieldedCoordinator> {
        coordinator_on(dir, dashcore::Network::Testnet)
    }

    fn coordinator_on(
        dir: &std::path::Path,
        network: dashcore::Network,
    ) -> Arc<NetworkShieldedCoordinator> {
        let db_path = dir.join("tree.sqlite");
        let store = FileBackedShieldedStore::open_path(&db_path, 100).expect("open file store");
        Arc::new(NetworkShieldedCoordinator::new(
            Arc::new(dash_sdk::Sdk::new_mock()),
            network,
            db_path,
            store,
        ))
    }

    async fn manager_with_wallet() -> (
        Arc<PlatformWalletManager<NoPlatformPersistence>>,
        Arc<PlatformWallet>,
    ) {
        let manager = Arc::new(PlatformWalletManager::new(
            Arc::new(dash_sdk::Sdk::new_mock()),
            Arc::new(NoPlatformPersistence),
            Arc::new(NoopEventHandler),
        ));
        let wallet = add_wallet(&manager).await;
        (manager, wallet)
    }

    async fn add_wallet(
        manager: &PlatformWalletManager<NoPlatformPersistence>,
    ) -> Arc<PlatformWallet> {
        manager
            .create_wallet_from_seed_bytes(
                dashcore::Network::Testnet,
                &SEED,
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet")
    }

    /// Pay for a private contact request from `wallet`'s account 0.
    async fn pay(
        wallet: &PlatformWallet,
        coordinator: &Arc<NetworkShieldedCoordinator>,
    ) -> Result<ResponseMetadata, PlatformWalletError> {
        let keyset =
            OrchardKeySet::from_seed(&SEED, dashcore::Network::Testnet, 0).expect("keyset");
        wallet
            .pay_private_contact_request(
                coordinator,
                keyset,
                0,
                &Identifier::from([0xA1; 32]),
                &Identifier::from([0xB1; 32]),
                BinaryData::new(vec![1; 589]),
                PaymentStarted::default(),
                UnreachableProver,
            )
            .await
    }

    /// A handle to a wallet removed, or re-created under the same id, since
    /// it was taken pays nothing for a private contact request: it is
    /// refused at the liveness check, before the spend.
    #[tokio::test]
    async fn should_refuse_a_private_payment_from_a_wallet_no_longer_current() {
        let dir = tempfile::tempdir().expect("temporary shielded store");
        let coordinator = coordinator_at(dir.path());
        let (manager, removed) = manager_with_wallet().await;
        let wallet_id = removed.wallet_id();
        manager.remove_wallet(&wallet_id).await.expect("removed");
        let refused = pay(&removed, &coordinator).await;
        assert!(
            matches!(&refused, Err(PlatformWalletError::WalletRemovedBeforeSend(id))
                if *id == hex::encode(wallet_id)),
            "{refused:?}"
        );

        let recreated = add_wallet(&manager).await;
        assert_eq!(recreated.wallet_id(), wallet_id);
        let refused = pay(&removed, &coordinator).await;
        assert!(
            matches!(
                refused,
                Err(PlatformWalletError::WalletRemovedBeforeSend(_))
            ),
            "{refused:?}"
        );
        let current = pay(&recreated, &coordinator).await;
        assert!(
            !matches!(
                current,
                Err(PlatformWalletError::WalletRemovedBeforeSend(_))
            ),
            "the current wallet's payment goes on to the spend: {current:?}"
        );
    }

    /// A removal waits for a private payment that entered its fenced section,
    /// here held at the shielded store, and completes once it returns.
    #[tokio::test]
    async fn should_hold_a_wallets_removal_until_its_private_payment_returns() {
        let dir = tempfile::tempdir().expect("temporary shielded store");
        let coordinator = coordinator_at(dir.path());
        let (manager, wallet) = manager_with_wallet().await;
        let wallet_id = wallet.wallet_id();

        let store = coordinator.store().write().await;
        let payment = {
            let wallet = Arc::clone(&wallet);
            let coordinator = Arc::clone(&coordinator);
            tokio::spawn(async move { pay(&wallet, &coordinator).await })
        };
        tokio::time::sleep(Duration::from_millis(100)).await;
        let removal = {
            let manager = Arc::clone(&manager);
            tokio::spawn(async move { manager.remove_wallet(&wallet_id).await.map(|_| ()) })
        };
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(!payment.is_finished(), "the payment waits at the store");
        assert!(
            !removal.is_finished(),
            "the removal waits for the payment in flight"
        );
        assert!(wallet.core().is_current_generation().await);

        drop(store);
        let paid = payment.await.expect("payment task");
        assert!(
            !matches!(paid, Err(PlatformWalletError::WalletRemovedBeforeSend(_))),
            "the wallet was current when it was checked: {paid:?}"
        );
        removal
            .await
            .expect("removal task")
            .expect("the removal completes");
        assert!(!wallet.core().is_current_generation().await);
    }

    /// A restoration waiting for a busy shielded store holds no lifecycle
    /// gate: a removal is not held behind its store wait. The step that
    /// reserves its pairs fences it to the wallet's generation instead.
    #[tokio::test]
    async fn should_not_hold_a_removal_behind_a_restoration_s_store_wait() {
        let dir = tempfile::tempdir().expect("temporary shielded store");
        let (_manager, wallet) = manager_with_wallet().await;
        let coordinator = coordinator_on(dir.path(), wallet.sdk.network);

        let store = coordinator.store().write().await;
        let restore = {
            let wallet = Arc::clone(&wallet);
            let coordinator = Arc::clone(&coordinator);
            tokio::spawn(async move { wallet.restore_private_contact_payments(&coordinator).await })
        };
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(!restore.is_finished(), "the restoration waits at the store");
        let teardown = tokio::time::timeout(
            Duration::from_millis(200),
            wallet.generation.teardown_guard(),
        )
        .await;
        assert!(
            teardown.is_ok(),
            "a removal takes the lifecycle gate while the restoration waits"
        );
        drop(teardown);
        drop(store);
        restore
            .await
            .expect("restore task")
            .expect("the restoration completes");
    }

    /// A deferred restoration whose task ends early, here aborted while it
    /// waits for the store, still frees the queue: the next bind queues one.
    #[tokio::test]
    async fn should_free_the_deferred_restore_queue_when_its_task_is_dropped() {
        let dir = tempfile::tempdir().expect("temporary shielded store");
        let (_manager, wallet) = manager_with_wallet().await;
        let coordinator = coordinator_on(dir.path(), wallet.sdk.network);
        let queued = || {
            wallet
                .private_payment_restores
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .queued
        };

        let store = coordinator.store().write().await;
        let task = wallet
            .restore_private_payments_or_defer(&coordinator)
            .await
            .expect("queued");
        assert!(wallet
            .restore_private_payments_or_defer(&coordinator)
            .await
            .is_none());
        task.abort();
        assert!(task.await.is_err_and(|error| error.is_cancelled()));
        assert!(
            wallet
                .restore_private_payments_or_defer(&coordinator)
                .await
                .is_some(),
            "the aborted task freed the queue"
        );
        assert_eq!(queued(), 2);
        drop(store);
    }

    /// A bind while the shielded store is busy defers the restoration of
    /// the wallet's private payments to one task per store: a second bind
    /// while it waits queues none. Once a restoration from that store
    /// succeeded, a busy store neither holds the wallet's contact requests
    /// again nor queues another.
    #[tokio::test]
    async fn should_coalesce_deferred_restorations_and_keep_a_restored_wallet_available() {
        let dir = tempfile::tempdir().expect("temporary shielded store");
        let (_manager, wallet) = manager_with_wallet().await;
        let coordinator = coordinator_on(dir.path(), wallet.sdk.network);
        let queued = || {
            wallet
                .private_payment_restores
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .queued
        };

        let store = coordinator.store().write().await;
        assert!(wallet
            .restore_private_payments_or_defer(&coordinator)
            .await
            .is_some());
        assert!(wallet
            .restore_private_payments_or_defer(&coordinator)
            .await
            .is_none());
        assert_eq!(queued(), 1, "one deferred restoration per store");
        assert!(wallet.private_payments_held().await, "held until it runs");
        drop(store);
        for _ in 0..100 {
            if !wallet.private_payments_held().await {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(!wallet.private_payments_held().await, "restored");

        let store = coordinator.store().write().await;
        assert!(wallet
            .restore_private_payments_or_defer(&coordinator)
            .await
            .is_none());
        assert!(
            !wallet.private_payments_held().await,
            "a busy store holds a restored wallet no more"
        );
        assert_eq!(queued(), 1, "and queues nothing");
        drop(store);
    }
}

#[cfg(all(test, feature = "shielded"))]
mod check_recipient_hrp_tests {
    use super::*;
    use dpp::address_funds::PlatformAddress;

    fn recipient(network: dashcore::Network) -> String {
        PlatformAddress::P2pkh([0x11; 20]).to_bech32m_string(network)
    }

    #[test]
    fn devnet_address_into_devnet_wallet_is_accepted() {
        // A devnet `tdash1…` recipient must be
        // accepted by a devnet wallet (not mis-rejected as
        // Testnet).
        let addr = recipient(dashcore::Network::Devnet);
        assert!(addr.starts_with("tdash1"));
        assert!(check_recipient_hrp(&addr, dashcore::Network::Devnet).is_ok());
    }

    #[test]
    fn testnet_address_into_testnet_wallet_is_accepted() {
        let addr = recipient(dashcore::Network::Testnet);
        assert!(check_recipient_hrp(&addr, dashcore::Network::Testnet).is_ok());
    }

    #[test]
    fn tdash_address_crosses_the_tdash_shared_networks() {
        // `tdash` is shared, so a testnet-encoded address is accepted by a
        // devnet/regtest wallet and vice versa.
        let testnet_addr = recipient(dashcore::Network::Testnet);
        assert!(check_recipient_hrp(&testnet_addr, dashcore::Network::Devnet).is_ok());
        assert!(check_recipient_hrp(&testnet_addr, dashcore::Network::Regtest).is_ok());
        let devnet_addr = recipient(dashcore::Network::Devnet);
        assert!(check_recipient_hrp(&devnet_addr, dashcore::Network::Testnet).is_ok());
    }

    #[test]
    fn mainnet_address_into_testnet_wallet_is_rejected() {
        let addr = recipient(dashcore::Network::Mainnet);
        assert!(addr.starts_with("dash1"));
        let err = check_recipient_hrp(&addr, dashcore::Network::Testnet).unwrap_err();
        assert!(
            matches!(&err, PlatformWalletError::ShieldedBuildError(m) if m.contains("network mismatch")),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn mainnet_address_into_devnet_wallet_is_rejected() {
        let addr = recipient(dashcore::Network::Mainnet);
        let err = check_recipient_hrp(&addr, dashcore::Network::Devnet).unwrap_err();
        assert!(
            matches!(&err, PlatformWalletError::ShieldedBuildError(m) if m.contains("network mismatch")),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn uppercase_recipient_is_accepted() {
        let addr = recipient(dashcore::Network::Testnet).to_uppercase();
        assert!(check_recipient_hrp(&addr, dashcore::Network::Testnet).is_ok());
    }

    #[test]
    fn non_platform_hrp_reports_not_a_platform_address() {
        // A valid Bitcoin bech32 SegWit address has HRP "bc", which decodes fine
        // but is not a platform HRP — so classification rejects it cleanly.
        let err = check_recipient_hrp(
            "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4",
            dashcore::Network::Testnet,
        )
        .unwrap_err();
        assert!(
            matches!(&err, PlatformWalletError::ShieldedBuildError(m) if m.contains("not a platform address")),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn missing_separator_errors_without_panic() {
        let err = check_recipient_hrp("nodelimiterhere", dashcore::Network::Testnet).unwrap_err();
        // bech32::decode emits "parsing failed" for strings without the separator
        assert!(
            matches!(&err, PlatformWalletError::ShieldedBuildError(m)
                if m.contains("invalid platform address")),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn empty_recipient_errors_without_panic() {
        let err = check_recipient_hrp("", dashcore::Network::Testnet).unwrap_err();
        assert!(
            matches!(&err, PlatformWalletError::ShieldedBuildError(m)
                if m.contains("invalid platform address")),
            "unexpected error: {err:?}"
        );
    }
}

#[cfg(all(test, feature = "shielded"))]
mod shield_input_selection_tests {
    use super::*;
    use dpp::address_funds::PlatformAddress;
    use dpp::version::LATEST_PLATFORM_VERSION;

    fn reserve() -> Credits {
        shield_fee_reserve_credits(LATEST_PLATFORM_VERSION)
            .expect("latest shield fee reserve must be computable")
    }

    fn addr(b: u8) -> PlatformAddress {
        PlatformAddress::P2pkh([b; 20])
    }

    fn indexed_addr(index: usize) -> PlatformAddress {
        let encoded = index.to_be_bytes();
        let mut hash = [0u8; 20];
        hash[20 - encoded.len()..].copy_from_slice(&encoded);
        PlatformAddress::P2pkh(hash)
    }

    fn min_input_amount() -> Credits {
        LATEST_PLATFORM_VERSION
            .dpp
            .state_transitions
            .address_funds
            .min_input_amount
    }

    fn max_address_inputs() -> usize {
        usize::from(
            LATEST_PLATFORM_VERSION
                .dpp
                .state_transitions
                .max_address_inputs,
        )
    }

    fn plan(
        candidates: Vec<(PlatformAddress, Credits)>,
    ) -> Result<ShieldedShieldInputPlan, PlatformWalletError> {
        plan_shield_inputs(
            candidates,
            reserve(),
            min_input_amount(),
            max_address_inputs(),
        )
    }

    #[test]
    fn skips_leading_dust_address_below_reserve() {
        // addr(1) sorts first but is dust (== reserve, not > reserve);
        // addr(2) must become input 0.
        let candidates = vec![(addr(1), reserve()), (addr(2), 5 * reserve())];
        let plan = plan(candidates).unwrap();
        let chosen = plan.select_inputs(2 * reserve()).unwrap();
        assert!(
            !chosen.contains_key(&addr(1)),
            "dust leading address must be skipped"
        );
        assert_eq!(chosen.get(&addr(2)), Some(&(2 * reserve())));
    }

    #[test]
    fn balance_exactly_at_reserve_is_not_viable_input_0() {
        // Strict `> reserve`: a sole address holding exactly the reserve
        // cannot be input 0.
        let candidates = vec![(addr(1), reserve())];
        let plan = plan(candidates).unwrap();
        assert_eq!(
            plan.preflight,
            ShieldedShieldPreflight {
                can_shield: false,
                account_balance_credits: reserve(),
                usable_balance_credits: 0,
                fee_reserve_credits: reserve(),
                max_shieldable_credits: 0,
                reason: plan.preflight.reason.clone(),
            }
        );
        assert!(plan.preflight.reason.is_some());
        let err = plan.select_inputs(1).unwrap_err();
        assert!(matches!(
            err,
            PlatformWalletError::PlatformShieldCapacityExceeded { available, required }
                if available == reserve() && required == 1 + reserve()
        ));
    }

    #[test]
    fn amount_equal_to_total_minus_reserve_claims_exactly_amount() {
        // Single address holding exactly amount + reserve: claim ==
        // amount, leaving the full reserve for DeductFromInput(0).
        let amount = 3 * reserve();
        let candidates = vec![(addr(1), amount + reserve())];
        let plan = plan(candidates).unwrap();
        assert_eq!(plan.preflight.max_shieldable_credits, amount);
        let chosen = plan.select_inputs(amount).unwrap();
        assert_eq!(chosen.len(), 1);
        assert_eq!(chosen.get(&addr(1)), Some(&amount));
    }

    #[test]
    fn accumulates_across_inputs_reserving_only_on_input_0() {
        let amount = 5 * reserve();
        // input 0 (addr 1) holds 2*reserve → contributes reserve after
        // its headroom; addr 2 covers the rest.
        let candidates = vec![(addr(1), 2 * reserve()), (addr(2), 5 * reserve())];
        let plan = plan(candidates).unwrap();
        let chosen = plan.select_inputs(amount).unwrap();
        assert_eq!(chosen.get(&addr(1)), Some(&reserve()));
        assert_eq!(chosen.get(&addr(2)), Some(&(4 * reserve())));
        assert_eq!(chosen.values().sum::<u64>(), amount);
    }

    #[test]
    fn insufficient_usable_balance_errors() {
        // Needs amount + reserve = 5*reserve, only 2*reserve available.
        let candidates = vec![(addr(1), 2 * reserve())];
        let plan = plan(candidates).unwrap();
        let err = plan.select_inputs(4 * reserve()).unwrap_err();
        assert!(matches!(
            err,
            PlatformWalletError::PlatformShieldCapacityExceeded { .. }
        ));
    }

    #[test]
    fn regression_reports_max_from_usable_suffix_not_total_account_balance() {
        // Account snapshot whose leading address cannot pay the fee, so
        // capacity must come from the usable suffix, not the account total.
        // The leading balance is derived from the reserve — one credit below
        // the strict `> reserve` viability threshold, the largest balance that
        // must still be rejected as input 0 — so the shape holds whatever the
        // versioned fee schedule does next.
        let dust = reserve() - 1;
        let usable = 3_623_849_220;
        let candidates = vec![
            (addr(1), dust),
            (addr(2), 2_000_000_000),
            (addr(3), 1_623_849_220),
        ];
        let plan = plan(candidates).unwrap();
        let expected_max = usable - reserve();

        assert_eq!(plan.preflight.account_balance_credits, dust + usable);
        assert_eq!(plan.preflight.usable_balance_credits, usable);
        assert_eq!(plan.preflight.fee_reserve_credits, reserve());
        assert_eq!(plan.preflight.max_shieldable_credits, expected_max);
        assert!(plan.preflight.can_shield);
        assert_eq!(plan.preflight.reason, None);

        let chosen = plan.select_inputs(expected_max).unwrap();
        assert!(!chosen.contains_key(&addr(1)));
        assert_eq!(chosen.values().sum::<u64>(), expected_max);

        // `available` reports the usable suffix, never the account total —
        // the whole point of the regression.
        let err = plan.select_inputs(expected_max + 1).unwrap_err();
        assert!(matches!(
            err,
            PlatformWalletError::PlatformShieldCapacityExceeded { available, required }
                if available == usable && required == usable + 1
        ));
    }

    #[test]
    fn no_viable_address_is_a_normal_zero_capacity_preflight() {
        // Both addresses are funded but neither strictly exceeds the reserve,
        // so no address can serve as the fee-paying input 0.
        let below_reserve = reserve() / 2;
        let plan = plan(vec![(addr(2), below_reserve), (addr(1), reserve())]).unwrap();

        assert!(!plan.preflight.can_shield);
        assert_eq!(
            plan.preflight.account_balance_credits,
            reserve() + below_reserve
        );
        assert_eq!(plan.preflight.usable_balance_credits, 0);
        assert_eq!(plan.preflight.max_shieldable_credits, 0);
        assert!(plan
            .preflight
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("Platform payment account")));
    }

    #[test]
    fn planner_sorts_lexicographically_and_reserves_only_on_input_zero() {
        let below_reserve = reserve() / 2;
        let plan = plan(vec![
            (addr(3), 2 * reserve()),
            (addr(1), below_reserve),
            (addr(2), 2 * reserve()),
        ])
        .unwrap();

        assert_eq!(
            plan.preflight.account_balance_credits,
            4 * reserve() + below_reserve
        );
        assert_eq!(plan.preflight.usable_balance_credits, 4 * reserve());
        assert_eq!(plan.preflight.max_shieldable_credits, 3 * reserve());
        let chosen = plan.select_inputs(2 * reserve()).unwrap();
        assert_eq!(chosen.get(&addr(2)), Some(&reserve()));
        assert_eq!(chosen.get(&addr(3)), Some(&reserve()));
        assert!(!chosen.contains_key(&addr(1)));
    }

    #[test]
    fn planner_rejects_credit_sum_overflow() {
        let err = plan(vec![(addr(1), u64::MAX), (addr(2), 1)]).unwrap_err();
        assert!(matches!(err, PlatformWalletError::InputSumOverflow));
    }

    #[test]
    fn versioned_input_cap_excludes_max_plus_one_candidate_from_capacity_and_selection() {
        let max_inputs = max_address_inputs();
        assert!(max_inputs > 0, "latest protocol must permit shield inputs");

        let candidates = (1..=max_inputs + 1)
            .map(|index| (indexed_addr(index), 2 * reserve()))
            .collect();
        let plan = plan(candidates).unwrap();
        let expected_account_balance = (max_inputs as u64 + 1) * 2 * reserve();
        let expected_usable_balance = max_inputs as u64 * 2 * reserve();
        let expected_max = expected_usable_balance - reserve();

        assert_eq!(
            plan.preflight.account_balance_credits,
            expected_account_balance
        );
        assert_eq!(
            plan.preflight.usable_balance_credits,
            expected_usable_balance
        );
        assert_eq!(plan.preflight.max_shieldable_credits, expected_max);
        assert_eq!(plan.usable_candidates.len(), max_inputs);

        let selected = plan.select_inputs(expected_max).unwrap();
        assert_eq!(selected.len(), max_inputs);
        assert!(!selected.contains_key(&indexed_addr(max_inputs + 1)));

        let err = plan.select_inputs(expected_max + 1).unwrap_err();
        assert!(matches!(
            err,
            PlatformWalletError::PlatformShieldCapacityExceeded { available, required }
                if available == expected_usable_balance
                    && required == expected_usable_balance + 1
        ));
    }

    #[test]
    fn excludes_later_address_below_versioned_minimum_from_max() {
        let dust = min_input_amount() - 1;
        let plan = plan(vec![(addr(1), 2 * reserve()), (addr(2), dust)]).unwrap();

        assert_eq!(plan.preflight.account_balance_credits, 2 * reserve() + dust);
        assert_eq!(plan.preflight.usable_balance_credits, 2 * reserve());
        assert_eq!(plan.preflight.max_shieldable_credits, reserve());
        let chosen = plan.select_inputs(reserve()).unwrap();
        assert_eq!(chosen.len(), 1);
        assert_eq!(chosen.get(&addr(1)), Some(&reserve()));
        assert!(!chosen.contains_key(&addr(2)));

        let err = plan.select_inputs(reserve() + 1).unwrap_err();
        assert!(matches!(
            err,
            PlatformWalletError::PlatformShieldCapacityExceeded { available, required }
                if available == 2 * reserve() && required == 2 * reserve() + 1
        ));
    }

    #[test]
    fn lifts_non_first_greedy_tail_to_versioned_minimum() {
        let minimum = min_input_amount();
        let plan = plan(vec![
            (addr(1), 2 * reserve()),
            (addr(2), minimum.saturating_mul(2)),
        ])
        .unwrap();

        let amount = reserve() + 1;
        let chosen = plan.select_inputs(amount).unwrap();
        assert_eq!(chosen.get(&addr(1)), Some(&reserve()));
        assert_eq!(chosen.get(&addr(2)), Some(&minimum));
        assert_eq!(chosen.values().sum::<u64>(), amount + minimum - 1);
        assert!(chosen
            .iter()
            .skip(1)
            .all(|(_, requested)| *requested >= minimum));
    }
}
