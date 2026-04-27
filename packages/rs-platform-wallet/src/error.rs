//! Unified error type returned across the public API.

use dpp::identifier::Identifier;
use key_wallet::Network;

/// Errors that can occur in platform wallet operations.
///
/// This is the unified error type returned from public APIs across the
/// crate (manager, wallet, identity, asset lock, broadcaster, SPV). Most
/// variants wrap an opaque `String` describing the underlying failure;
/// structured fields (`Identifier`, `Network`, `account_index`) are used
/// only when callers are expected to branch on the data.
#[derive(Debug, thiserror::Error)]
pub enum PlatformWalletError {
    /// Wallet creation failed (e.g. seed import, account derivation).
    #[error("Wallet creation failed: {0}")]
    WalletCreation(String),

    /// No wallet with the given identifier is registered with the manager.
    #[error("Wallet not found: {0}")]
    WalletNotFound(String),

    /// A wallet with the same `WalletId` (derived from the seed) is already
    /// registered. Importing the same seed twice would otherwise fork state.
    #[error("Wallet already exists: {0}")]
    WalletAlreadyExists(String),

    /// An identity with the given identifier is already managed by this
    /// wallet (registration replay or duplicate import).
    #[error("Identity already exists: {0}")]
    IdentityAlreadyExists(Identifier),

    /// No managed identity matches the given identifier.
    #[error("Identity not found: {0}")]
    IdentityNotFound(Identifier),

    /// An operation that needs a primary (default) identity was attempted
    /// before one was set.
    #[error("No primary identity set")]
    NoPrimaryIdentity,

    /// The supplied identity payload failed validation (missing keys,
    /// malformed structure, etc.).
    #[error("Invalid identity data: {0}")]
    InvalidIdentityData(String),

    /// No contact request found with the given identifier on the wallet's
    /// active identity.
    #[error("Contact request not found: {0}")]
    ContactRequestNotFound(Identifier),

    /// The HD `identity_index` for the given identity has not been set —
    /// register the identity (or discover it) before signing.
    #[error("Identity index not set for identity {0} — register or discover the identity first")]
    IdentityIndexNotSet(Identifier),

    /// A DashPay receiving (incoming-funds) account already exists for the
    /// `(identity, contact)` pair on the given network at `account_index`.
    #[error(
        "DashPay receiving account already exists for identity {identity} with contact {contact} on network {network:?} (account index {account_index})"
    )]
    DashpayReceivingAccountAlreadyExists {
        /// The identity that owns the receiving account.
        identity: Identifier,
        /// The contact whose payments would land in this account.
        contact: Identifier,
        /// Network the account was derived for.
        network: Network,
        /// BIP44/DIP-15 account index of the existing account.
        account_index: u32,
    },

    /// A DashPay external (outgoing-funds) account already exists for the
    /// `(identity, contact)` pair on the given network at `account_index`.
    #[error(
        "DashPay external account already exists for identity {identity} with contact {contact} on network {network:?} (account index {account_index})"
    )]
    DashpayExternalAccountAlreadyExists {
        /// The identity that owns the external account.
        identity: Identifier,
        /// The contact this account sends payments to.
        contact: Identifier,
        /// Network the account was derived for.
        network: Network,
        /// BIP44/DIP-15 account index of the existing account.
        account_index: u32,
    },

    /// Building or signing the asset-lock transaction failed.
    #[error("Asset lock transaction failed: {0}")]
    AssetLockTransaction(String),

    /// The broadcaster reported a failure pushing the transaction onto the
    /// network (DAPI / SPV / RPC, depending on the implementation).
    #[error("Transaction broadcast failed: {0}")]
    TransactionBroadcast(String),

    /// Transaction construction failed before broadcast (insufficient
    /// funds, fee estimation, signing, etc.).
    #[error("Transaction building failed: {0}")]
    TransactionBuild(String),

    /// Waiting for an InstantSend / ChainLock proof on an asset lock
    /// timed out or failed before a usable proof became available.
    #[error("Asset lock proof waiting failed: {0}")]
    AssetLockProofWait(String),

    /// Generic SDK failure surfaced from `dash-sdk` (Drive, DAPI, proof
    /// verification, etc.).
    #[error("SDK error: {0}")]
    Sdk(#[from] dash_sdk::Error),

    /// Platform-address sync (the periodic balance refresh) failed.
    #[error("Address sync failed: {0}")]
    AddressSync(String),

    /// A platform-address operation (transfer, withdrawal, derivation)
    /// failed for a reason that doesn't fit a more specific variant.
    #[error("Address operation failed: {0}")]
    AddressOperation(String),

    /// The given platform address is not part of this wallet's managed set.
    #[error("Platform address not found in wallet: {0}")]
    AddressNotFound(String),

    /// HD key derivation failed (path out of range, locked seed, malformed
    /// derivation context).
    #[error("Key derivation failed: {0}")]
    KeyDerivation(String),

    /// The wallet is locked and the requested operation needs the seed.
    /// Unlock the wallet (e.g. via `Wallet::unlock`) before retrying.
    #[error("Wallet is locked — unlock it before performing this operation")]
    WalletLocked,

    /// `start_spv` was called while the SPV runtime was already up.
    #[error("SPV is already running — stop it before starting again")]
    SpvAlreadyRunning,

    /// SPV cannot start because no wallets have been registered with the
    /// manager yet.
    #[error("No wallets configured — add a wallet before starting SPV")]
    NoWalletsConfigured,

    /// An operation that requires the SPV runtime was attempted while it
    /// was stopped.
    #[error("SPV client is not running")]
    SpvNotRunning,

    /// Generic SPV-layer failure (peer disconnect, header validation,
    /// filter mismatch, etc.).
    #[error("SPV error: {0}")]
    SpvError(String),

    /// Token operation failed (mint, transfer, burn, configuration).
    #[error("Token operation failed: {0}")]
    TokenError(String),

    /// Timed out waiting for an InstantSend or ChainLock finality proof
    /// for the given transaction.
    #[error("Timed out waiting for finality proof for transaction {0}")]
    FinalityTimeout(dashcore::Txid),

    /// The InstantSend proof is too old to use and no ChainLock proof has
    /// been produced yet — the asset lock cannot be redeemed until a
    /// ChainLock arrives or the lock is rebuilt.
    #[error("Asset lock proof expired (IS proof too old, CL not yet available): {0}")]
    AssetLockExpired(String),

    /// The asset-lock transaction has not been ChainLocked, so we can't
    /// fall back to a CL proof when the IS proof is rejected.
    #[error("Asset lock transaction not chain-locked, cannot fall back to CL proof: {0}")]
    AssetLockNotChainLocked(String),

    // --- Shielded pool errors (feature-gated) ---
    /// No unspent shielded notes are available to fund the requested
    /// shielded operation.
    #[error("No unspent shielded notes available")]
    ShieldedNoUnspentNotes,

    /// The selected shielded notes don't add up to the required amount.
    #[error("Insufficient shielded balance: available {available}, required {required}")]
    ShieldedInsufficientBalance {
        /// Total spendable shielded balance, in duffs.
        available: u64,
        /// Amount the operation needed, in duffs.
        required: u64,
    },

    /// Building the shielded transaction (proof construction, balance
    /// commitment) failed.
    #[error("Shielded build error: {0}")]
    ShieldedBuildError(String),

    /// Broadcasting a shielded transaction failed.
    #[error("Shielded broadcast failed: {0}")]
    ShieldedBroadcastFailed(String),

    /// Syncing the shielded note set against the chain failed.
    #[error("Shielded sync failed: {0}")]
    ShieldedSyncFailed(String),

    /// Updating the shielded commitment tree failed.
    #[error("Shielded commitment tree update failed: {0}")]
    ShieldedTreeUpdateFailed(String),

    /// The shielded note store reported a failure (read/write/integrity).
    #[error("Shielded store error: {0}")]
    ShieldedStoreError(String),

    /// Syncing the nullifier set (to detect spent notes) failed.
    #[error("Shielded nullifier sync failed: {0}")]
    ShieldedNullifierSyncFailed(String),

    /// The Merkle witness needed to spend a shielded note is missing or
    /// stale (the commitment tree advanced past the note's anchor).
    #[error("Shielded Merkle witness unavailable: {0}")]
    ShieldedMerkleWitnessUnavailable(String),

    /// Deriving the shielded spending / viewing keys failed.
    #[error("Shielded key derivation failed: {0}")]
    ShieldedKeyDerivation(String),
}

/// Check whether an SDK error indicates that an InstantSend lock proof was
/// rejected by Platform (e.g. the IS lock has expired).
///
/// This matches the `InvalidInstantAssetLockProofSignatureError` consensus
/// error returned by Drive when the instant lock signature cannot be verified
/// (typically because the quorum that signed it has rotated out).
pub fn is_instant_lock_proof_invalid(error: &dash_sdk::Error) -> bool {
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::ConsensusError;

    let consensus_error = match error {
        dash_sdk::Error::StateTransitionBroadcastError(broadcast_err) => {
            broadcast_err.cause.as_ref()
        }
        dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(ce)) => Some(ce.as_ref()),
        _ => None,
    };
    matches!(
        consensus_error,
        Some(ConsensusError::BasicError(
            BasicError::InvalidInstantAssetLockProofSignatureError(_),
        ))
    )
}
