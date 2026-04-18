//! Persistence traits for wallet storage backends.
//!
//! Implementors choose their own storage engine (SQLite, file, memory, remote).
//! The traits guarantee that deltas are persisted atomically.

use key_wallet::account::AccountType;
use key_wallet::bip32::ExtendedPubKey;
use key_wallet::managed_account::address_pool::AddressPoolType;
use key_wallet::AddressInfo;
use key_wallet::Network;

use crate::changeset::changeset::PlatformWalletChangeSet;
use crate::changeset::client_start_state::ClientStartState;
use crate::wallet::platform_wallet::WalletId;

/// Storage backend for [`PlatformWalletChangeSet`] deltas.
///
/// The persister persists what the changeset carries — nothing more,
/// nothing less. See the **Scope** section below for the exact
/// boundary.
///
/// Changesets flow through a two-phase pipeline:
///
/// 1. **`store`** — merge a delta into per-wallet state. Implementations
///    MAY defer I/O until [`flush`](Self::flush) or MAY flush inline;
///    callers must not assume `store` is free of I/O. See the
///    **Nuance** section below.
/// 2. **`flush`** — ensure all buffered deltas for the given `wallet_id`
///    are durable, then clear that wallet's buffer.
///
/// The trait uses `&self` with a `wallet_id` parameter so a single persister
/// instance can be shared across all wallets in a [`PlatformWalletManager`].
/// Implementations are responsible for internal synchronization (e.g.
/// `Mutex` / `RwLock` around staged changeset buffers).
///
/// # Scope
///
/// ## Inside scope — must be persisted via `store` / `flush`
///
/// - **Wallet-level core state**: chain height, per-account address-pool
///   watermarks (`highest_used`), UTXO set (inserts, spends, IS-lock flags),
///   per-account transaction records, and account pool state.
/// - **Asset locks**: all tracked asset-lock transactions with their funding
///   type, proof, and chain-lock state (`AssetLockChangeSet`).
/// - **Identity-level state** (`IdentityEntry` inside `IdentityChangeSet`):
///   wallet_id, wallet_index, DPNS usernames, top-up history, lifecycle
///   status, key storage, DashPay profile, and DashPay payment history.
/// - **Contact state** (`ContactChangeSet`): sent contact requests, incoming
///   (received) contact requests, and established (mutually-accepted) contacts.
///
/// ## Outside scope — persisted through other mechanisms
///
/// - **Raw `identity.data` BLOB**: the bincoded `QualifiedIdentity` record
///   (an evo-tool wrapper type around `dpp::Identity`) is currently written
///   directly by `Database::insert_local_qualified_identity` and
///   `Database::update_local_qualified_identity`, called from backend tasks.
///   Moving this blob into the persister is planned as a future commit
///   (evo-tool task #130 / Phase 9c). Until then, the persister does not
///   write or read the `identity.data` column.
/// - **Platform addresses** and **token balances**: these are dropped on
///   flush; backend tasks own their persistence.
///
/// ## Nuance on `store` and I/O
///
/// The `store` method is documented as "buffer for later writing (cheap, no
/// I/O)", but implementations are free to flush inline. In particular,
/// `SqliteWalletPersister` (the canonical evo-tool implementation) flushes
/// on every `store` call — there is no deferred batch window. Callers must
/// not assume that `store` is free of I/O; treat any `store` → `flush`
/// sequence as potentially performing I/O at either point. If a caller needs
/// to guarantee a batch flush, it should call `flush` explicitly after all
/// `store` calls and treat `store` as a best-effort buffer hint.
pub trait PlatformWalletPersistence: Send + Sync {
    /// Buffer a changeset for later persistence.
    ///
    /// Implementations should merge into an internal per-wallet accumulator so
    /// that a single [`flush`](Self::flush) writes the combined delta.
    ///
    /// Returns an error if the internal accumulator cannot be accessed
    /// (e.g. mutex poisoning). Callers that use fire-and-forget
    /// semantics should log the error rather than propagating.
    fn store(
        &self,
        wallet_id: WalletId,
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Write all buffered changesets atomically for the given wallet, then
    /// clear that wallet's buffer.
    fn flush(&self, wallet_id: WalletId) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Load the full client state from storage.
    ///
    /// Returns a [`ClientStartState`] — a ready-to-boot snapshot covering
    /// every wallet the persister knows about. It mirrors
    /// [`PlatformWalletChangeSet`] for most sub-areas, but platform-address
    /// state comes back as
    /// [`PlatformAddressSyncStartState`](crate::changeset::PlatformAddressSyncStartState)
    /// so the caller can hand it straight to
    /// `PlatformPaymentAddressProvider::from_persisted` instead of
    /// replaying an accumulated changeset.
    ///
    /// Called once at startup — this is a whole-client operation, not a
    /// per-wallet one, because `ClientStartState::platform_addresses` is
    /// already keyed by wallet id and the sub-changesets carry their own
    /// wallet attribution where needed.
    fn load(&self) -> Result<ClientStartState, Box<dyn std::error::Error + Send + Sync>>;

    /// Persist an account entry for `wallet_id`. Called on account
    /// insertion (initial registration or later `add_account` calls).
    ///
    /// This captures enough material to reconstruct every account of a
    /// wallet as watch-only via `Account::from_xpub` at load time —
    /// hardened derivation at the account level means the per-account
    /// xpub is the only way to get it back without the mnemonic. The
    /// `wallet_id` travels through `store_wallet_metadata`; paired with
    /// this call, `Wallet::new_watch_only(network, wallet_id, accounts)`
    /// has everything it needs.
    ///
    /// Default implementation is a no-op.
    fn store_account(
        &self,
        wallet_id: WalletId,
        account_type: &AccountType,
        account_xpub: &ExtendedPubKey,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let _ = (wallet_id, account_type, account_xpub);
        Ok(())
    }

    /// Persist per-wallet metadata that isn't derivable from the xpub
    /// alone: the network this wallet is bound to and the birth height
    /// (best estimate of the chain tip at creation time; 0 means
    /// "scan from genesis / unknown").
    ///
    /// Called once at registration alongside
    /// [`store_wallet_root_xpub`](Self::store_wallet_root_xpub).
    ///
    /// Default implementation is a no-op.
    fn store_wallet_metadata(
        &self,
        wallet_id: WalletId,
        network: Network,
        birth_height: u32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let _ = (wallet_id, network, birth_height);
        Ok(())
    }

    /// Persist every [`AddressInfo`] from one of an account's address
    /// pools. Called per pool — callers emit external then internal
    /// (or the single pool for simpler account types) so the entries
    /// slice is homogeneous with respect to `pool_type`.
    ///
    /// Called at wallet create (initial gap-limit population) and on
    /// any pool extension / "used" flip that happens during sync.
    ///
    /// Default implementation is a no-op.
    fn store_account_addresses(
        &self,
        wallet_id: WalletId,
        account_type: &AccountType,
        pool_type: AddressPoolType,
        addresses: &[AddressInfo],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let _ = (wallet_id, account_type, pool_type, addresses);
        Ok(())
    }
}
