use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use key_wallet::account::StandardAccountType;
use key_wallet::Network;

/// Errors that can occur in platform wallet operations
#[derive(Debug, thiserror::Error)]
pub enum PlatformWalletError {
    #[error("Wallet creation failed: {0}")]
    WalletCreation(String),

    #[error("Wallet not found: {0}")]
    WalletNotFound(String),

    #[error("Wallet already exists: {0}")]
    WalletAlreadyExists(String),

    #[error("Identity already exists: {0}")]
    IdentityAlreadyExists(Identifier),

    #[error("Identity not found: {0}")]
    IdentityNotFound(Identifier),

    #[error("No primary identity set")]
    NoPrimaryIdentity,

    #[error("Invalid identity data: {0}")]
    InvalidIdentityData(String),

    #[error("Failed to persist state: {0}")]
    /// A persister `store(...)` round failed. Returned (not swallowed) by
    /// user-initiated writes whose loss leaves a silent, non-self-healing
    /// broken state — e.g. a reject tombstone that, if not persisted, lets
    /// the rejected contact resurrect on the next launch. The in-memory
    /// mutation has already happened for this session; the error tells the
    /// caller (FFI → UI) to surface the failure and retry rather than
    /// reporting a success that didn't reach disk.
    Persistence(String),

    #[error("Contact request not found: {0}")]
    ContactRequestNotFound(Identifier),

    #[error("Identity index not set for identity {0} — register or discover the identity first")]
    IdentityIndexNotSet(Identifier),

    #[error(
        "DashPay receiving account already exists for identity {identity} with contact {contact} on network {network:?} (account index {account_index})"
    )]
    DashpayReceivingAccountAlreadyExists {
        identity: Identifier,
        contact: Identifier,
        network: Network,
        account_index: u32,
    },

    #[error(
        "DashPay external account already exists for identity {identity} with contact {contact} on network {network:?} (account index {account_index})"
    )]
    DashpayExternalAccountAlreadyExists {
        identity: Identifier,
        contact: Identifier,
        network: Network,
        account_index: u32,
    },

    #[error("Asset lock transaction failed: {0}")]
    AssetLockTransaction(String),

    #[error("Transaction broadcast failed: {0}")]
    TransactionBroadcast(String),

    /// A core transaction broadcast failed with an **ambiguous** outcome — the
    /// transaction may already have reached the network (transport timeout
    /// after delivery, partial peer send, or an internal multi-node retry
    /// whose earlier attempt may have succeeded). The spent inputs'
    /// reservation is intentionally kept, so an immediate retry fails at
    /// input selection instead of double-spending; the reservation-TTL
    /// backstop (or a sync observing the transaction) reconciles the outcome.
    ///
    /// The shielded sibling is [`Self::ShieldedSpendUnconfirmed`].
    #[error(
        "Transaction broadcast outcome unknown — it may already be on the \
         network; its inputs stay reserved until a sync or the reservation \
         TTL reconciles the outcome: {0}"
    )]
    TransactionBroadcastUnconfirmed(String),

    #[error("Transaction building failed: {0}")]
    TransactionBuild(String),

    #[error("no spendable inputs available on {account_type} account {account_index}: {context}")]
    NoSpendableInputs {
        account_type: StandardAccountType,
        account_index: u32,
        context: String,
    },

    #[error("Asset lock proof waiting failed: {0}")]
    AssetLockProofWait(String),

    #[error("SDK error: {0}")]
    Sdk(#[from] dash_sdk::Error),

    #[error("Address sync failed: {0}")]
    AddressSync(String),

    #[error("Address operation failed: {0}")]
    AddressOperation(String),

    #[error(
        "no selectable inputs: only funded addresses appear as destinations \
         (funded_outputs={funded_outputs:?}, sub_min_count={sub_min_count}, \
         sub_min_aggregate={sub_min_aggregate}, min_input_amount={min_input_amount}); \
         rotate to a fresh receive address, consolidate funds, or use \
         InputSelection::Explicit"
    )]
    OnlyOutputAddressesFunded {
        /// Funded addresses dropped by the input-equals-output filter.
        funded_outputs: Vec<PlatformAddress>,
        /// Number of additional addresses with a positive balance below
        /// `min_input_amount`. Preserved even though the output-collision
        /// signal is the typically-actionable fix, so a UI rotating to a
        /// fresh receive address has the dust breadcrumb on the next try.
        sub_min_count: usize,
        /// Aggregate of the sub-minimum balances counted in `sub_min_count`.
        sub_min_aggregate: Credits,
        /// Per-input minimum from the active platform version.
        min_input_amount: Credits,
    },

    // The `Display` text is surfaced verbatim to the user by the withdrawal
    // preflight (the FFI carries `e.to_string()` as the can't-fund reason), so
    // it is kept user-presentable: it explains the situation and the action
    // ("consolidate funds onto fewer addresses") without naming an internal
    // selection API. The numeric fields stay in the message as an actionable
    // breadcrumb.
    #[error(
        "Every funded address holds less than the per-input minimum of \
         {min_input_amount} credits ({sub_min_count} addresses totaling \
         {sub_min_aggregate} credits), so none can fund this operation on \
         its own. Consolidate funds onto fewer addresses, then try again."
    )]
    OnlyDustInputs {
        /// Number of addresses with a positive balance below `min_input_amount`.
        sub_min_count: usize,
        /// Aggregate of those sub-minimum balances.
        sub_min_aggregate: Credits,
        /// Per-input minimum from the active platform version.
        min_input_amount: Credits,
    },

    #[error(
        "change output amount {change_amount} is below the protocol per-output \
         minimum {min_output_amount}; raise the input sum or drop the change \
         address so the residual would exceed the minimum"
    )]
    ChangeBelowMinimumOutput {
        /// `Σ inputs − Σ user_outputs` — the residual that would have been
        /// routed to the change output.
        change_amount: Credits,
        /// Per-output minimum from the active platform version.
        min_output_amount: Credits,
    },

    #[error("input sum overflow: caller-supplied input balances exceed u64::MAX")]
    InputSumOverflow,

    #[error("Platform address not found in wallet: {0}")]
    AddressNotFound(String),

    #[error("Key derivation failed: {0}")]
    KeyDerivation(String),

    #[error("Wallet is locked — unlock it before performing this operation")]
    WalletLocked,

    #[error(
        "Signer does not bind to wallet {wallet_id}: it derives a different \
         BIP44 account-0 xpub (refusing to sign with the wrong seed)"
    )]
    /// The host signer derives a BIP44 account-0 extended public key that does
    /// not equal this wallet's persisted account xpub — the signer resolves a
    /// different seed than the one that owns the wallet (e.g. a mis-mapped
    /// Keychain slot). The operation is refused so a wrong seed can never sign
    /// for this wallet. Surfaced by [`crate::PlatformWallet::verify_seed_binds`].
    SeedMismatch {
        /// Hex of the wallet id whose binding check failed.
        wallet_id: String,
    },

    #[error("SPV is already running — stop it before starting again")]
    SpvAlreadyRunning,

    #[error("No wallets configured — add a wallet before starting SPV")]
    NoWalletsConfigured,

    #[error("SPV error: {0}")]
    SpvError(String),

    #[error("Token operation failed: {0}")]
    TokenError(String),

    #[error("Timed out waiting for finality proof for outpoint {0}")]
    /// IS-lock did not propagate within `wait_for_proof`'s deadline.
    /// Carries the outpoint (not just the txid) so the caller can
    /// drive the IS→CL upgrade flow without re-walking the
    /// tracked-asset-lock map by `(funding_type, identity_index)` —
    /// which is BTreeMap-order, non-deterministic when multiple
    /// unproven locks share that key.
    FinalityTimeout(dashcore::OutPoint),

    #[error("Asset lock proof expired (IS proof too old, CL not yet available): {0}")]
    AssetLockExpired(String),

    #[error("Asset lock transaction not chain-locked, cannot fall back to CL proof: {0}")]
    AssetLockNotChainLocked(String),

    // --- Shielded pool errors (feature-gated) ---
    #[error("No unspent shielded notes available")]
    ShieldedNoUnspentNotes,

    #[error("Insufficient shielded balance: available {available}, required {required}")]
    ShieldedInsufficientBalance { available: u64, required: u64 },

    #[error("Shielded build error: {0}")]
    ShieldedBuildError(String),

    #[error("Shielded broadcast failed: {0}")]
    ShieldedBroadcastFailed(String),

    /// The shielded identity-create transition was **broadcast and accepted by the relay**, but the
    /// SDK could not confirm its execution result (the result-proof fetch/verify failed — e.g. a
    /// transient DAPI/proof error, not a platform rejection). The identity with `identity_id` may
    /// already exist on chain, so the caller must NOT treat it as unregistered: the slot stays held
    /// against re-submission and the spent notes' reservations are left in place (the next nullifier
    /// sync reconciles them). `reason` carries the underlying SDK error for diagnostics.
    #[error(
        "Shielded broadcast succeeded but its execution result could not be confirmed; \
         identity {identity_id} may already exist on chain — do not re-submit \
         (it will appear after the next sync): {reason}"
    )]
    ShieldedBroadcastUnconfirmed {
        identity_id: Identifier,
        reason: String,
    },

    /// A shielded transition (`operation` is `"shield"`, `"unshield"`, `"transfer"` or
    /// `"withdraw"`) was **broadcast and accepted by the relay**, but the SDK could not confirm
    /// its execution result (the result-proof fetch/verify failed — e.g. a transient DAPI/proof
    /// error or timeout, not a platform rejection). The operation may already be executed on
    /// chain, so re-submitting risks a double-execution. For the spend-based operations the spent
    /// notes' reservations are intentionally left in place rather than released — releasing them
    /// would invite re-selecting notes whose nullifiers may already be consumed; a shield spends
    /// no notes, so it has nothing reserved. The next sync (or an app restart, since reservations
    /// are in-memory only) reconciles the outcome either way. `reason` carries the underlying SDK
    /// error for diagnostics.
    ///
    /// The identity-create sibling is [`Self::ShieldedBroadcastUnconfirmed`], which additionally
    /// carries the derived identity id so the caller can hold the registration slot.
    #[error(
        "Shielded {operation} broadcast succeeded but its execution result could not be \
         confirmed; it may already be executed on chain — do not re-submit \
         (the next sync reconciles the outcome): {reason}"
    )]
    ShieldedSpendUnconfirmed {
        operation: &'static str,
        reason: String,
    },

    #[error("Shielded sync failed: {0}")]
    ShieldedSyncFailed(String),

    #[error("Shielded commitment tree update failed: {0}")]
    ShieldedTreeUpdateFailed(String),

    #[error("Shielded store error: {0}")]
    ShieldedStoreError(String),

    #[error("Shielded Merkle witness unavailable: {0}")]
    ShieldedMerkleWitnessUnavailable(String),

    /// No Platform-recorded anchor covers the notes selected for a shielded
    /// spend, so the wallet cannot build a proof Platform will accept.
    ///
    /// Platform records one commitment-tree anchor per block, but an
    /// index-chunk sync routinely leaves the wallet's tree mid-block, so the
    /// current (depth-0) root is frequently a value Platform never recorded.
    /// This variant is **retryable**: it is returned *before* any broadcast,
    /// the note reservations are released by the caller's generic error path,
    /// and the next shielded sync advances the tree onto a recorded boundary.
    /// `0` carries a human-readable reason.
    #[error("Shielded spend cannot use a Platform-recorded anchor: {0}")]
    ShieldedNoRecordedAnchor(String),

    #[error("Shielded key derivation failed: {0}")]
    ShieldedKeyDerivation(String),

    #[error("Shielded sub-wallet not bound: call bind_shielded first")]
    ShieldedNotBound,
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

/// Check whether a platform-wallet error represents a *Core-side*
/// InstantSend lock timeout (the asset-lock manager waited the full
/// timeout for an IS-lock proof and never observed one).
///
/// Companion to [`is_instant_lock_proof_invalid`] (which detects
/// **Platform-side** rejection of an IS proof after one was obtained).
/// Both surfaces trigger the same fallback path in the registration /
/// top-up flow: upgrade the asset-lock to a ChainLock proof and retry.
///
/// The IS-timeout shape comes from
/// [`AssetLockManager::wait_for_proof`](crate::wallet::asset_lock::manager::AssetLockManager),
/// which emits `PlatformWalletError::FinalityTimeout(Txid)` when the
/// 300-second IS deadline elapses.
pub fn is_instant_lock_timeout(error: &PlatformWalletError) -> bool {
    matches!(error, PlatformWalletError::FinalityTimeout(_))
}

/// Extract the `InvalidAssetLockProofCoreChainHeightError` (DPP
/// consensus code 10506) from an SDK error if Platform rejected a
/// ChainLock asset-lock proof because Platform's
/// `last_committed_core_height` is still behind the proof's
/// `core_chain_locked_height`.
///
/// Returns `Some(&error)` whenever the rejection matches, exposing
/// `proof_core_chain_locked_height()` (what the wallet claimed) and
/// `current_core_chain_locked_height()` (Platform's currently observed
/// tip). The latter is what the wallet can log to attribute the lag
/// to: small means routine race against Platform's
/// `create-empty-blocks-interval` (3m on mainnet); large or stuck
/// means the DAPI node we hit is genuinely behind / misbehaving.
///
/// Returns `None` for everything else. The check is stateless and
/// re-evaluated on every CheckTx, so a resubmit after Platform
/// catches up will pass — but Tenderdash's mempool caches rejected-tx
/// hashes for ~24h on mainnet/testnet (`keep-invalid-txs-in-cache =
/// true` in dashmate's tenderdash template), so the resubmit must
/// carry a *different* signable-bytes hash to bypass the cache. The
/// submission layer handles that by bumping
/// `PutSettings::user_fee_increase` before re-issuing.
///
/// Companion to [`is_instant_lock_proof_invalid`]; both feed the
/// CL-proof retry path in the identity registration / top-up flow.
///
/// **Coverage caveat:** only inspects the two `dash_sdk::Error`
/// variants that today wrap consensus errors —
/// `StateTransitionBroadcastError` (from `broadcast_and_wait`) and
/// `Protocol(ProtocolError::ConsensusError)` (from validation). If a
/// future SDK version surfaces the same `InvalidAssetLockProofCoreChainHeightError`
/// through a different variant (e.g. wrapped in a transport-layer
/// error type), the retry helper silently falls through to the "Sdk"
/// passthrough. Re-audit this matcher whenever `dash_sdk::Error`
/// gains new variants that can carry consensus errors.
pub fn as_asset_lock_proof_cl_height_too_low(
    error: &dash_sdk::Error,
) -> Option<&dpp::consensus::basic::identity::InvalidAssetLockProofCoreChainHeightError> {
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::ConsensusError;

    let consensus_error = match error {
        dash_sdk::Error::StateTransitionBroadcastError(broadcast_err) => {
            broadcast_err.cause.as_ref()
        }
        dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(ce)) => Some(ce.as_ref()),
        _ => None,
    };
    match consensus_error {
        Some(ConsensusError::BasicError(
            BasicError::InvalidAssetLockProofCoreChainHeightError(e),
        )) => Some(e),
        _ => None,
    }
}
