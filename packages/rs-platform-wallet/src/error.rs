use dpp::address_funds::PlatformAddress;
use dpp::consensus::state::address_funds::AddressInvalidNonceError;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::prelude::AddressNonce;
use key_wallet::account::StandardAccountType;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
use key_wallet::wallet::managed_wallet_info::transaction_building::AccountTypePreference;
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

    /// A general Core L1 payment build (`CoreWallet::build_signed_payment`)
    /// could not cover the requested outputs plus fee from the union of the
    /// wallet's *signable* funds accounts (BIP44 + BIP32 + CoinJoin + DashPay
    /// receiving; watch-only DashPay external accounts are excluded). `available`
    /// is the total selectable value across those accounts, `required` the
    /// outputs-plus-fee target — carried as exact duff amounts (instead of being
    /// flattened into a string) so callers can render a precise shortfall.
    #[error(
        "payment coin selection is short: available {available} duffs, \
         required {required} duffs"
    )]
    PaymentInsufficientFunds { available: u64, required: u64 },

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

    /// Atomic Core finalization could not select enough unreserved funds.
    #[error(
        "insufficient unreserved Core funds on {account_type:?} account {account_index}: \
         available {available:?}, required {required:?}"
    )]
    CoreInsufficientFunds {
        account_type: AccountTypePreference,
        account_index: u32,
        available: Option<u64>,
        required: Option<u64>,
    },

    #[error("no spendable inputs available on {account_type} account {account_index}: {context}")]
    NoSpendableInputs {
        account_type: StandardAccountType,
        account_index: u32,
        context: String,
    },

    #[error("Asset lock proof waiting failed: {0}")]
    AssetLockProofWait(String),

    /// The caller supplied an outpoint that this wallet does not own/track.
    /// Kept distinct from proof-wait failures so FFI hosts can classify a
    /// stale or foreign recovery request without parsing text.
    #[error("Asset lock {0} is not tracked by this wallet")]
    AssetLockNotTracked(dashcore::OutPoint),

    /// A one-shot asset lock has already funded a successful Platform
    /// transition and cannot be resumed again.
    #[error("Asset lock {0} has already been consumed")]
    AssetLockAlreadyConsumed(dashcore::OutPoint),

    /// A tracked outpoint belongs to another funding family or identity
    /// index. Resuming it for the requested destination would spend the
    /// one-shot output on the wrong operation.
    #[error(
        "Asset lock {out_point} is ineligible for {expected_funding_type:?} index \
         {expected_identity_index}: tracked as {actual_funding_type:?} index \
         {actual_identity_index}"
    )]
    AssetLockFundingMismatch {
        out_point: dashcore::OutPoint,
        expected_funding_type: AssetLockFundingType,
        expected_identity_index: u32,
        actual_funding_type: AssetLockFundingType,
        actual_identity_index: u32,
    },

    #[error("SDK error: {0}")]
    Sdk(#[from] dash_sdk::Error),

    /// Platform rejected an address-funds transition because a spent address's
    /// provided nonce did not equal its expected next value (DPP consensus code
    /// 40603, `AddressInvalidNonceError`) — an optimistic `fetched + 1` nonce
    /// racing a lagging replica read. Carries Platform's `expected_nonce`
    /// verbatim so the caller can rebuild and retry without re-fetching.
    #[error(
        "Address nonce mismatch for {address}: submitted nonce {provided_nonce}, \
         Platform expected {expected_nonce}; retry the operation with the \
         expected nonce"
    )]
    AddressNonceMismatch {
        address: PlatformAddress,
        provided_nonce: AddressNonce,
        expected_nonce: AddressNonce,
    },

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

    /// A background sync pass did not drain within its quiesce budget, so
    /// the operation that required a "no more persister stores" barrier
    /// (manager shutdown, `clear_shielded`, a sync-state reset) aborted
    /// fail-closed. The wedged pass may still fire persistence / event
    /// callbacks; the host must keep its callback context alive and must
    /// not commit any wipe it was about to pair with this call.
    /// FFI mirror: `PlatformWalletFFIResultCode::ErrorShutdownIncomplete`.
    #[error("Background sync did not quiesce: {0}")]
    ShutdownIncomplete(String),

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

/// Extract the `AddressInvalidNonceError` (DPP consensus code 40603) when
/// Platform rejected an address-funds transition on a stale nonce, exposing
/// `address()`, `provided_nonce()`, and `expected_nonce()` so the caller can
/// retry with the expected value; `None` otherwise.
///
/// Matches the three `dash_sdk::Error` shapes that can carry a consensus
/// verdict — `StateTransitionBroadcastError` (wait-stream), `Protocol(
/// ConsensusError)` (CheckTx), and a `NoAvailableAddressesToRetry` envelope it
/// recurses into — staying in lockstep with `broadcast_definitely_failed`.
pub fn as_address_invalid_nonce(error: &dash_sdk::Error) -> Option<&AddressInvalidNonceError> {
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;

    let consensus_error = match error {
        dash_sdk::Error::StateTransitionBroadcastError(broadcast_err) => {
            broadcast_err.cause.as_ref()
        }
        dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(ce)) => Some(ce.as_ref()),
        // Unwrap the dapi-client's exhausted-retry envelope.
        dash_sdk::Error::NoAvailableAddressesToRetry(inner) => {
            return as_address_invalid_nonce(inner)
        }
        _ => None,
    };
    match consensus_error {
        Some(ConsensusError::StateError(StateError::AddressInvalidNonceError(e))) => Some(e),
        _ => None,
    }
}

/// Promote a nonce-rejection SDK error to the typed
/// [`PlatformWalletError::AddressNonceMismatch`] so callers can recover
/// `expected_nonce` and retry, instead of receiving the rejection flattened
/// to a string.
///
/// Returns `None` for any error [`as_address_invalid_nonce`] does not match,
/// leaving the caller free to keep its existing fallback mapping.
pub fn promote_address_nonce_error(error: &dash_sdk::Error) -> Option<PlatformWalletError> {
    as_address_invalid_nonce(error).map(|e| PlatformWalletError::AddressNonceMismatch {
        address: *e.address(),
        provided_nonce: e.provided_nonce(),
        expected_nonce: e.expected_nonce(),
    })
}

/// Map an address-funded transition's SDK error to a [`PlatformWalletError`],
/// promoting a nonce rejection to the typed
/// [`PlatformWalletError::AddressNonceMismatch`] and otherwise preserving it
/// under [`PlatformWalletError::Sdk`]. Owned-error `.map_err(...)?` analogue of
/// [`promote_address_nonce_error`] for the transfer / withdrawal call sites.
pub fn promote_address_nonce_error_or_sdk(error: dash_sdk::Error) -> PlatformWalletError {
    promote_address_nonce_error(&error).unwrap_or(PlatformWalletError::Sdk(error))
}

/// The reserved machine prefix that a typed `SigningKeyUnavailable` signer
/// completion stamps at the **start** of its `ProtocolError::Generic` payload.
///
/// Canonically owned by the signer-completion boundary as
/// [`rs_sdk_ffi::DASH_SDK_SIGNER_ERR_KEY_UNAVAILABLE_PREFIX`]. It is mirrored
/// here — rather than imported — because this pure-logic crate must recognize
/// the marker *before* an operation wrapper stringifies the underlying SDK
/// error, yet is deliberately kept free of any dependency on the FFI crate.
/// The two definitions are pinned byte-identical by a compile-time assertion in
/// `platform-wallet-ffi` (`src/error.rs`), so any drift is a build failure
/// rather than a silent code-31 regression (dashpay/platform#4183 review).
pub const SIGNER_KEY_UNAVAILABLE_PREFIX: &str = "signer_error:key_unavailable: ";

/// Preserve a structured `SigningKeyUnavailable` signer failure through an
/// operation wrapper that would otherwise flatten it to a string and discard
/// the typed discriminator the FFI boundary restores to code 31
/// (`ErrorSigningKeyUnavailable`).
///
/// Several public signing paths (token transfer, DPNS registration, document
/// replace, …) wrap every SDK failure in an operation-specific string variant
/// (`TokenError`, `InvalidIdentityData`, …). A genuine key-unavailable
/// completion still leaves the reserved prefix inside those strings, but the
/// resulting variant reaches the FFI's `_` arm and flattens to
/// `ErrorUnknown`, losing the host's key-repair routing. This helper keeps the
/// failure verbatim under [`PlatformWalletError::Sdk`] — the one shape
/// `From<PlatformWalletError> for PlatformWalletFFIResult` maps to code 31 —
/// and hands every other error to `wrap`, the caller's stringifying wrapper,
/// unchanged.
///
/// The check is **structural and position-0 only** (the marker must start the
/// nested `ProtocolError::Generic` payload); it is never a substring sniff of
/// the rendered error, so a foreign signer that merely mentions the token is
/// not misrouted into key repair (dashpay/platform#4183 review). This mirrors
/// the guarded restore already performed by the FFI conversion.
pub fn preserve_signer_key_unavailable_or(
    error: dash_sdk::Error,
    wrap: impl FnOnce(dash_sdk::Error) -> PlatformWalletError,
) -> PlatformWalletError {
    if matches!(
        &error,
        dash_sdk::Error::Protocol(dpp::ProtocolError::Generic(s))
            if s.starts_with(SIGNER_KEY_UNAVAILABLE_PREFIX)
    ) {
        PlatformWalletError::Sdk(error)
    } else {
        wrap(error)
    }
}

#[cfg(test)]
mod signer_key_unavailable_tests {
    use super::*;

    /// A structured key-unavailable signer completion (the reserved marker at
    /// the start of a `ProtocolError::Generic` payload) is preserved verbatim
    /// under `Sdk` so the FFI boundary can restore code 31 — the operation
    /// wrapper is NOT applied.
    #[test]
    fn preserves_structured_key_unavailable_error() {
        let error = dash_sdk::Error::Protocol(dpp::ProtocolError::Generic(format!(
            "{SIGNER_KEY_UNAVAILABLE_PREFIX}no private key stored for 02abcd"
        )));
        let mapped = preserve_signer_key_unavailable_or(error, |e| {
            PlatformWalletError::TokenError(format!("Token transfer failed: {e}"))
        });
        match mapped {
            PlatformWalletError::Sdk(dash_sdk::Error::Protocol(dpp::ProtocolError::Generic(s))) => {
                assert!(s.starts_with(SIGNER_KEY_UNAVAILABLE_PREFIX));
            }
            other => panic!("expected preserved Sdk(Protocol(Generic)), got {other:?}"),
        }
    }

    /// An unrelated SDK error is handed to the caller's wrapper unchanged.
    #[test]
    fn wraps_unrelated_error() {
        let error = dash_sdk::Error::Generic("boom".to_string());
        let mapped = preserve_signer_key_unavailable_or(error, |e| {
            PlatformWalletError::TokenError(format!("Token transfer failed: {e}"))
        });
        match mapped {
            PlatformWalletError::TokenError(msg) => {
                assert!(msg.contains("Token transfer failed"));
                assert!(msg.contains("boom"));
            }
            other => panic!("expected wrapped TokenError, got {other:?}"),
        }
    }

    /// The marker only counts at position 0: a generic error that merely
    /// mentions it mid-message is wrapped, never preserved as the typed
    /// key-unavailable shape (dashpay/platform#4183 review).
    #[test]
    fn substring_marker_is_not_preserved() {
        let error = dash_sdk::Error::Protocol(dpp::ProtocolError::Generic(format!(
            "remote signer reported: {SIGNER_KEY_UNAVAILABLE_PREFIX}oops"
        )));
        let mapped = preserve_signer_key_unavailable_or(error, |e| {
            PlatformWalletError::InvalidIdentityData(format!("Failed to replace document: {e}"))
        });
        assert!(
            matches!(mapped, PlatformWalletError::InvalidIdentityData(_)),
            "a mid-message marker must be wrapped, not preserved"
        );
    }
}

#[cfg(test)]
mod address_nonce_tests {
    use super::*;
    use dash_sdk::error::StateTransitionBroadcastError;

    const ADDR_BYTES: [u8; 20] = [7u8; 20];

    /// An `AddressInvalidNonceError` wrapped as a `ConsensusError`, plus the
    /// address it names, for asserting round-trip field fidelity.
    fn nonce_consensus_error(
        provided: AddressNonce,
        expected: AddressNonce,
    ) -> (PlatformAddress, dpp::consensus::ConsensusError) {
        let address = PlatformAddress::P2pkh(ADDR_BYTES);
        let err = AddressInvalidNonceError::new(address, provided, expected);
        (address, err.into())
    }

    /// `Protocol(ConsensusError)` — the CheckTx-rejection shape.
    fn protocol_shape(provided: AddressNonce, expected: AddressNonce) -> dash_sdk::Error {
        let (_, cause) = nonce_consensus_error(provided, expected);
        dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(Box::new(cause)))
    }

    /// `StateTransitionBroadcastError` — the wait-stream-rejection shape.
    fn broadcast_shape(provided: AddressNonce, expected: AddressNonce) -> dash_sdk::Error {
        let (_, cause) = nonce_consensus_error(provided, expected);
        dash_sdk::Error::StateTransitionBroadcastError(StateTransitionBroadcastError {
            code: 40603,
            message: "invalid address nonce".to_string(),
            cause: Some(cause),
        })
    }

    #[test]
    fn extracts_nonce_error_from_protocol_shape() {
        let err = protocol_shape(1, 2);
        let got = as_address_invalid_nonce(&err).expect("protocol shape must match");
        assert_eq!(*got.address(), PlatformAddress::P2pkh(ADDR_BYTES));
        assert_eq!(got.provided_nonce(), 1);
        assert_eq!(got.expected_nonce(), 2);
    }

    #[test]
    fn extracts_nonce_error_from_broadcast_shape() {
        let err = broadcast_shape(5, 6);
        let got = as_address_invalid_nonce(&err).expect("broadcast shape must match");
        assert_eq!(*got.address(), PlatformAddress::P2pkh(ADDR_BYTES));
        assert_eq!(got.provided_nonce(), 5);
        assert_eq!(got.expected_nonce(), 6);
    }

    #[test]
    fn ignores_unrelated_and_causeless_errors() {
        // A plainly unrelated SDK error.
        assert!(as_address_invalid_nonce(&dash_sdk::Error::Generic("boom".to_string())).is_none());
        // The DAPI wait-timeout shape: a broadcast error with no consensus
        // cause must NOT be misread as a nonce rejection.
        let causeless =
            dash_sdk::Error::StateTransitionBroadcastError(StateTransitionBroadcastError {
                code: 0,
                message: "timeout".to_string(),
                cause: None,
            });
        assert!(as_address_invalid_nonce(&causeless).is_none());
    }

    #[test]
    fn promotes_both_shapes_to_typed_variant() {
        for err in [protocol_shape(1, 2), broadcast_shape(1, 2)] {
            match promote_address_nonce_error(&err) {
                Some(PlatformWalletError::AddressNonceMismatch {
                    address,
                    provided_nonce,
                    expected_nonce,
                }) => {
                    assert_eq!(address, PlatformAddress::P2pkh(ADDR_BYTES));
                    assert_eq!(provided_nonce, 1);
                    assert_eq!(expected_nonce, 2);
                }
                other => panic!("expected AddressNonceMismatch, got {other:?}"),
            }
        }
    }

    #[test]
    fn promotion_leaves_unrelated_errors_for_the_fallback() {
        assert!(
            promote_address_nonce_error(&dash_sdk::Error::Generic("boom".to_string())).is_none()
        );
    }

    #[test]
    fn promote_or_sdk_promotes_a_matching_nonce_error() {
        // The transfer / withdrawal call sites route their SDK error through
        // this helper; a nonce rejection must surface as the typed variant.
        for err in [protocol_shape(3, 4), broadcast_shape(3, 4)] {
            match promote_address_nonce_error_or_sdk(err) {
                PlatformWalletError::AddressNonceMismatch {
                    address,
                    provided_nonce,
                    expected_nonce,
                } => {
                    assert_eq!(address, PlatformAddress::P2pkh(ADDR_BYTES));
                    assert_eq!(provided_nonce, 3);
                    assert_eq!(expected_nonce, 4);
                }
                other => panic!("expected AddressNonceMismatch, got {other:?}"),
            }
        }
    }

    #[test]
    fn promote_or_sdk_falls_back_to_sdk_for_unrelated_errors() {
        // A non-nonce error must be preserved verbatim under `Sdk`, not
        // flattened — this is the fallback the transfer / withdrawal sites keep.
        match promote_address_nonce_error_or_sdk(dash_sdk::Error::Generic("boom".to_string())) {
            PlatformWalletError::Sdk(dash_sdk::Error::Generic(msg)) => assert_eq!(msg, "boom"),
            other => panic!("expected Sdk(Generic), got {other:?}"),
        }
    }

    #[test]
    fn promote_or_sdk_promotes_through_the_retry_envelope() {
        // A nonce rejection wrapped in the dapi-client's retry envelope must
        // still promote (the helper recurses via `as_address_invalid_nonce`).
        let wrapped =
            dash_sdk::Error::NoAvailableAddressesToRetry(Box::new(protocol_shape(11, 12)));
        match promote_address_nonce_error_or_sdk(wrapped) {
            PlatformWalletError::AddressNonceMismatch {
                provided_nonce,
                expected_nonce,
                ..
            } => {
                assert_eq!(provided_nonce, 11);
                assert_eq!(expected_nonce, 12);
            }
            other => panic!("expected AddressNonceMismatch, got {other:?}"),
        }
    }

    #[test]
    fn extracts_nonce_error_wrapped_in_no_available_addresses_to_retry() {
        // The dapi-client wraps the last rejection in `NoAvailableAddressesToRetry`
        // when every address is exhausted mid-retry; the extractor must recurse
        // into it (lockstep with `broadcast_definitely_failed`).
        let inner = Box::new(protocol_shape(9, 10));
        let wrapped = dash_sdk::Error::NoAvailableAddressesToRetry(inner);
        let got = as_address_invalid_nonce(&wrapped).expect("must unwrap the retry envelope");
        assert_eq!(got.provided_nonce(), 9);
        assert_eq!(got.expected_nonce(), 10);
    }
}
