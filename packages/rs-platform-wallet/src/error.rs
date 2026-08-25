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
        "Identity discovery incomplete: {failed_probes} of {probed} index probe(s) from {start_index} \
         could not reach Platform and no identity was found; last error: {source}"
    )]
    /// A gap-limit scan ended empty with at least one index left unanswered.
    /// Distinct from an empty success: it means "we do not know", so the
    /// caller must retry rather than record that the seed owns no identity.
    /// Both outcomes used to arrive as `Ok(vec![])`, which is how a transient
    /// DAPI failure right after restore-from-seed became a whole session
    /// without an identity.
    ///
    /// "Retry" is the contract, not a promise that the cause is transient — a
    /// probe can also fail on configuration, protocol or proof errors. The
    /// underlying failure is kept typed in `source` so a Rust caller can
    /// classify it instead of parsing the rendered message.
    IdentityDiscoveryIncomplete {
        /// First index the scan probed.
        start_index: u32,
        /// How many indices were probed before the gap limit stopped the scan.
        probed: u32,
        /// How many of those probes failed to reach Platform.
        failed_probes: u32,
        /// The last probe failure. Boxed to keep this variant from widening
        /// the enum past the existing `Sdk` variant.
        #[source]
        source: Box<dash_sdk::Error>,
    },

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

    /// A finalized transaction handle
    /// (`core_wallet_tx_builder_finalize` → `broadcast_finalized_transaction`)
    /// was held long enough that its funding reservation may already have been
    /// swept and re-selected by key-wallet's TTL: the wallet's
    /// `last_processed_height` advanced at least
    /// `RESERVATION_MAX_AGE_BLOCKS`
    /// blocks past the height the reservation was stamped at
    /// ([`SignedCoreTransaction::reservation_height`](crate::SignedCoreTransaction::reservation_height)).
    /// Broadcasting it could spend against a newer, unrelated reservation, so it
    /// is refused **before** touching the network — NOT retryable in place, the
    /// caller must rebuild the payment. The refusal reconciles the reservation
    /// on the way out: a funded finalize always stamps an owner token, so the
    /// release is owner-guarded (`release_reservation_if_owner`, safe at any
    /// age — it no-ops once ownership transferred) and the still-owned inputs
    /// are freed for the instructed rebuild. Abandoning/freeing the handle
    /// likewise releases owner-guarded at any age; only a token-less build
    /// skips its unguarded by-outpoint release past the bound and leaves the
    /// aged outpoint for key-wallet's TTL to reclaim.
    ///
    /// This is the handle-path sibling of the deferred registry-token
    /// [`SignedPaymentError::StaleReservationToken`](crate::SignedPaymentError::StaleReservationToken);
    /// both share the same age bound and the FFI `ErrorStaleReservationToken`
    /// code. Carries no token — the handle path is keyed by an opaque handle,
    /// not a numeric reservation token.
    #[error("finalized transaction reservation has outlived its lifetime; rebuild the payment")]
    StaleReservation,

    #[error("Transaction building failed: {0}")]
    TransactionBuild(String),

    /// Coin selection picked an outpoint that a broadcast dispatch is still
    /// holding — the transaction spending it is in flight, or has reached the
    /// network and has not yet been observed spent by this wallet
    /// ([`WalletGeneration::in_broadcast_conflict`](crate::wallet::core::WalletGeneration::in_broadcast_conflict)).
    /// Completing the build would race that transaction on the wire, so it is
    /// refused and its own fresh reservation released. NOTHING was built,
    /// signed or broadcast.
    ///
    /// A TRANSIENT, EXPECTED condition, and the reason it is a variant of its
    /// own rather than a [`Self::TransactionBuild`] /
    /// [`Self::AssetLockTransaction`] string: it is the one build failure a
    /// caller may safely retry UNCHANGED once the in-flight dispatch settles,
    /// and telling it apart from a genuine build failure previously meant
    /// substring-matching prose (`message.contains("mid-broadcast")`, which the
    /// tests did too). All three selection choke points — the
    /// finalized-transaction build, the contact-payment build and the
    /// asset-lock build — now return this one variant.
    ///
    /// `outpoint` is the first conflicting input, carried structurally so
    /// callers and diagnostics need not parse it back out of a message.
    ///
    /// Reaching a caller at all is the uncommon path: a fenced input is
    /// normally still reserved and never offered to selection. This fires only
    /// in the window after key-wallet's reservation TTL swept that dispatch's
    /// reservation, which is exactly what the fence exists to cover
    /// (`dashpay/platform#4309`).
    #[error(
        "selected input {outpoint} is mid-broadcast by an in-flight dispatch; \
         retry after it completes"
    )]
    InputMidBroadcast { outpoint: dashcore::OutPoint },

    /// The address handed to [`CoreWallet::sign_message`] cannot be a signing
    /// target at all: unparseable, encoded for a different network than the
    /// wallet's, or not P2PKH. A caller-input error — the classic Dash
    /// signed-message format recovers a public key and compares its
    /// `PubkeyHash` payload, so P2SH / SegWit payloads have no defined
    /// verification and are refused rather than signed into something no
    /// verifier accepts. `reason` names which of the three it was.
    ///
    /// [`CoreWallet::sign_message`]: crate::wallet::core::CoreWallet::sign_message
    #[error("message-signing address {address:?} is unusable: {reason}")]
    MessageSigningAddressInvalid { address: String, reason: String },

    /// The bytes handed to `core_wallet_sign_message` as the message are not
    /// valid UTF-8, so there is no string to sign. Caller input, exactly like
    /// [`Self::MessageSigningAddressInvalid`] — and given its own variant for
    /// the same reason the address case has one: these errors exist to name
    /// *which argument* the caller must fix, and reusing the address variant
    /// for a message problem would render "address … is unusable" over a
    /// perfectly good address.
    ///
    /// Only reachable across the FFI, where the message arrives as raw bytes; a
    /// Rust or Kotlin caller cannot construct an ill-formed `&str`/`String`.
    /// `address` is the (already validated as UTF-8) signing target, carried
    /// for log correlation like every sibling — the *message* is what failed.
    #[error("the message to sign for address {address} is not valid UTF-8: {reason}")]
    MessageSigningMessageInvalid { address: String, reason: String },

    /// [`CoreWallet::sign_message`] holds no usable signing key for a
    /// well-formed P2PKH address on the right network. Two producers:
    ///
    /// * **Address resolution** — the address belongs to no *signable* funds
    ///   account (BIP44 / BIP32 / CoinJoin / DashPay-receiving), or it belongs
    ///   to a watch-only DashPay **external** account (a contact's receiving
    ///   address, whose keys we never had). No signer is invoked.
    /// * **The signer itself** — the backend reported its key missing, stamped
    ///   as the reserved [`SIGNER_KEY_UNAVAILABLE_PREFIX`] at position 0 of its
    ///   error rendering (`MnemonicResolverCoreSigner::NotFound` in
    ///   production: the keychain holds no mnemonic for the wallet).
    ///   `sign_message` checks that marker BEFORE prepending any context, so
    ///   the condition stays typed across the FFI.
    ///
    /// Either way the conclusion is the same — no key can sign for this
    /// address as things stand — so hosts route both to key repair / address
    /// correction (FFI code 31). Carries no retry value as-is.
    ///
    /// [`CoreWallet::sign_message`]: crate::wallet::core::CoreWallet::sign_message
    #[error(
        "no signing key for message-signing address {address}: it belongs to no \
         signable funds account of this wallet"
    )]
    MessageSigningKeyUnavailable { address: String },

    /// [`CoreWallet::sign_message`] resolved a derivation path for the address
    /// but could not produce a signature over it. Four causes, all carried in
    /// `reason`: the signer backend does not advertise
    /// [`SignerMethod::Digest`], so it cannot sign a host-computed digest at
    /// all and is refused before it is ever invoked; the [`Signer`] itself
    /// failed (Keystore/Keychain round-trip); the public key it returned does
    /// not hash to the target address (a path-resolution bug — the guard exists
    /// so a wrong-key signature can never be handed out as if it were the
    /// address owner's); or no recovery id in `0..=3` recovers that public key.
    ///
    /// The capability refusal shares this variant rather than taking a
    /// dedicated one because this is the crate's "a path resolved but no
    /// signature came back" bucket, and because key-wallet folds the very same
    /// refusal into its ordinary `BuilderError::SigningFailed`. It is
    /// unreachable with any signer that ships today — the production mnemonic
    /// resolver advertises `Digest` — so it does not warrant a new FFI code and
    /// the host mirror-enum churn that follows one.
    ///
    /// Deliberately NOT given a dedicated FFI code: [`Signer::Error`] is
    /// generic and bounded only by `Display`, so what lands here cannot be
    /// classified structurally and falls through to `ErrorUnknown`. The one
    /// signer failure with a typed meaning — a key-unavailable rendering with
    /// [`SIGNER_KEY_UNAVAILABLE_PREFIX`] at position 0 — never reaches this
    /// variant: `sign_message` promotes it to
    /// [`MessageSigningKeyUnavailable`] (FFI code 31) before any context
    /// string is composed. See the `MessageSigningFailed` arm's NOTE in
    /// `platform-wallet-ffi`'s error conversion.
    ///
    /// [`MessageSigningKeyUnavailable`]: Self::MessageSigningKeyUnavailable
    /// [`Signer::Error`]: key_wallet::signer::Signer::Error
    ///
    /// [`CoreWallet::sign_message`]: crate::wallet::core::CoreWallet::sign_message
    /// [`Signer`]: key_wallet::signer::Signer
    /// [`SignerMethod::Digest`]: key_wallet::signer::SignerMethod::Digest
    #[error("message signing failed for address {address}: {reason}")]
    MessageSigningFailed { address: String, reason: String },

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

    /// Atomic Core finalization could not select enough unreserved funds for a
    /// POOLED build (more than one funding source offered).
    ///
    /// Separate from [`CoreInsufficientFunds`] because `available`/`required`
    /// describe the UNION of every offered source: attributing them to one
    /// account would misreport the figures and could name a source that
    /// contributed nothing — or that the wallet does not even have. FFI maps
    /// both variants to the same host-facing insufficient-funds code, so hosts
    /// classify a shortfall identically either way.
    ///
    /// [`CoreInsufficientFunds`]: Self::CoreInsufficientFunds
    #[error(
        "insufficient unreserved Core funds across the pooled funding sources \
         {sources:?}: available {available:?}, required {required:?}"
    )]
    CorePooledInsufficientFunds {
        sources: Vec<AccountTypePreference>,
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

    /// A one-shot asset lock outpoint cannot be reused. This can come from a
    /// local `Consumed` tombstone or an unauthenticated Platform consumption
    /// report; callers must not infer completion of the requested operation
    /// from this signal alone.
    #[error("Asset lock {0} cannot be reused; Platform completion is unconfirmed")]
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

    /// No DPNS `domain` document exists for the requested name (exact
    /// normalized-label lookup came back empty). Distinct from
    /// [`Self::InvalidParameter`]: the input was well-formed, the name
    /// just isn't registered (or is hidden inside an unresolved contest —
    /// see [`Self::ContestedNameNotTradable`] for the pre-checked case).
    #[error("DPNS name not found: {name:?}")]
    DpnsNameNotFound { name: String },

    /// The DPNS domain document carries no `$price` — it is not listed
    /// for sale. Raised by the wallet's pre-flight check and by the
    /// consensus downcast of `DocumentNotForSaleError` (DPP code 40108).
    #[error("document {document_id} is not for sale")]
    DocumentNotForSale { document_id: Identifier },

    /// The listed price no longer equals the price the user confirmed.
    /// Raised pre-flight (fresh read ≠ confirmed price) and by the
    /// consensus downcast of `DocumentIncorrectPurchasePriceError` (DPP
    /// code 40109) when the listing changed between the pre-flight read
    /// and broadcast — the purchase did NOT execute in either case.
    #[error(
        "document {document_id} price changed: purchase was confirmed at \
         {expected} credits but the listing is now {actual} credits"
    )]
    DocumentPriceChanged {
        document_id: Identifier,
        expected: Credits,
        actual: Credits,
    },

    /// The identity's credit balance cannot cover the operation
    /// (principal + fee margin for pre-flight checks; Platform's own
    /// arithmetic for the consensus downcast of
    /// `IdentityInsufficientBalanceError`).
    #[error(
        "identity {identity_id} has insufficient credits: {required} required, \
         {available} available"
    )]
    InsufficientIdentityCredits {
        identity_id: Identifier,
        required: Credits,
        available: Credits,
    },

    /// The name is inside an active contested-name vote, so its domain
    /// document is not yet in the documents tree and cannot be listed,
    /// transferred, or purchased. Without this guard the network returns
    /// a bare `DocumentNotFoundError` (40101), which reads as "no such
    /// name" — this typed error says what is actually going on.
    /// `ends_at_ms == 0` means the vote's end time was unavailable.
    #[error(
        "DPNS name {label:?} is in an active contested-name vote \
         (ends at {ends_at_ms} ms) and cannot be traded until the contest resolves"
    )]
    ContestedNameNotTradable { label: String, ends_at_ms: u64 },

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

    /// A caller passed an argument this API cannot act on — as opposed to a
    /// lookup that found nothing. Kept distinct from [`WalletNotFound`] so a
    /// host is told to fix its input rather than that the wallet is missing;
    /// FFI maps it to the existing invalid-parameter code, which is what the
    /// FFI boundary already returns for the same class of rejection.
    ///
    /// [`WalletNotFound`]: Self::WalletNotFound
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

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

    /// A Platform Payment-account shield cannot be represented from the
    /// wallet's deterministic address-input set at the requested amount.
    /// Distinct from [`ShieldedInsufficientBalance`](Self::ShieldedInsufficientBalance),
    /// which refers exclusively to private note selection.
    #[error("Platform shield capacity exceeded: available {available}, required {required}")]
    PlatformShieldCapacityExceeded { available: u64, required: u64 },

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

    /// A masternode (evonode) identity credit withdrawal was **broadcast and
    /// accepted**, but its execution result could not be confirmed (the
    /// result-proof fetch/verify failed — transient DAPI/proof error or
    /// timeout, not a Platform rejection). The claim may already have
    /// executed, and the SDK's identity-nonce cache was bumped for it, so
    /// re-submitting could execute a SECOND withdrawal with the next nonce.
    /// Callers must NOT retry until they have re-read the identity's
    /// claimable balance (and the payout) and reconciled the outcome.
    /// `reason` carries the underlying SDK error for diagnostics.
    ///
    /// Shielded sibling: [`Self::ShieldedSpendUnconfirmed`]; core sibling:
    /// [`Self::TransactionBroadcastUnconfirmed`].
    #[error(
        "Masternode withdrawal of {amount_credits} credits from identity {identity_id} was \
         broadcast but its result could not be confirmed; it may already have executed — do \
         not re-submit until the claimable balance has been re-read: {reason}"
    )]
    MasternodeWithdrawalUnconfirmed {
        identity_id: Identifier,
        amount_credits: u64,
        reason: String,
    },

    #[error("Shielded sync failed: {0}")]
    ShieldedSyncFailed(String),

    /// The foreign-key (one-time-invitation) note scan consumed its
    /// per-attempt work budget before covering the requested value
    /// (dashpay/platform#4306). RETRYABLE, and the retry is CHEAP: progress
    /// was checkpointed at tree position `scanned_through`, so the next
    /// attempt resumes there instead of restarting — attempts compound until
    /// the note is found or the tree is genuinely exhausted.
    ///
    /// Hosts MUST render this as "still searching — retry", never as an
    /// invalid, already-claimed, or unfunded invitation: the scan has simply
    /// not looked far enough yet, and treating it as terminal would strand a
    /// genuinely funded claim whose note sits deep in the tree.
    #[error(
        "shielded foreign-key scan paused at tree position {scanned_through} after \
         exhausting its per-attempt budget; progress is checkpointed — retry to continue"
    )]
    ShieldedForeignScanBudgetExhausted { scanned_through: u64 },

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

    /// A one-time-key (shielded invitation) claim could not be completed: the invitation note's
    /// nullifier is already spent on chain, and the wallet could **not** produce positive evidence
    /// that *this* claim's Type-20 transition created an identity.
    ///
    /// This is a **terminal** outcome for the invitation — the note is consumed, so no retry can
    /// spend it again — and it is deliberately distinct from
    /// [`Self::ShieldedBroadcastUnconfirmed`] (retryable: executed, not yet resolvable) and from
    /// success. It is returned instead of a success whenever the recovered identity fails either
    /// ownership binding checked by `recovered_identity_matches_claim`, which covers two real
    /// on-chain outcomes that a naive "nullifier spent + a key matches" test reports as success:
    ///
    /// 1. **Chargeable `UnshieldAction` fallback.** When a submitted unique public-key hash is
    ///    already registered, Type-20 finalizes the shielded spend as an `UnshieldTransitionAction`
    ///    with `chargeable_failure: true` and creates **no** identity, crediting the invitation
    ///    value to `send_to_address_on_creation_failure` minus a penalty. The nullifier is spent and
    ///    the *pre-existing* colliding identity is findable under the submitted MASTER auth key
    ///    hash, so key-hash existence alone would report a successful claim that never happened.
    /// 2. **A competing holder of the same bearer key.** The identity id is derived from published
    ///    nullifiers only, never from identity keys, so when two or more real notes are spent (no
    ///    randomized padding action) another holder of the same one-time key produces the *same*
    ///    derived id under *their* keys. Returning that identity would register a foreign identity
    ///    at this wallet's identity index.
    ///
    /// `reason` carries which binding failed, for diagnostics.
    #[error(
        "Shielded invitation already claimed: its note is spent on chain but this wallet cannot \
         prove that this claim created an identity ({reason}); the invitation cannot be claimed \
         again"
    )]
    ShieldedInviteAlreadyClaimed { reason: String },

    /// A one-time-key (shielded invitation) claim was retried with arguments that do **not** match
    /// the transition the earlier attempt actually submitted, so the retry was refused before
    /// touching the network.
    ///
    /// The durable pending-claim record is keyed by wallet id and the invitation's full viewing
    /// key alone — nothing in that key distinguishes *which* identity the original attempt was
    /// creating. The record does, however, carry the byte-exact serialized transition, and that
    /// transition is the authoritative statement of what was submitted: its `public_keys` are the
    /// keys the binding signature committed to, and its `denomination` is the value that left the
    /// pool. Resuming means re-broadcasting those exact bytes, so the identity that results belongs
    /// to *those* keys — never to whatever keys the retry happened to pass in.
    ///
    /// A retry whose keys or denomination differ is therefore not a resume of the same claim; it is
    /// a request to create a different identity from an invitation that is already committed
    /// elsewhere. Honouring it would let the caller
    ///
    /// * classify the original identity as belonging to another holder and clear the record (making
    ///   a padded single-note claim permanently unrecoverable — its declared id embeds a random
    ///   dummy nullifier and exists nowhere else),
    /// * backfill an empty proof result with keys that were never in the stored transition, or
    /// * register the original identity at the retry's local HD slot.
    ///
    /// So the claim fails closed here instead: nothing is re-broadcast, no proof is burned, and the
    /// record is left intact for a retry that presents the original arguments.
    #[error(
        "Shielded invitation claim retry does not match the transition the earlier attempt \
         submitted ({mismatch}); refusing to resume — retry with the original arguments, which \
         the pending claim record has preserved"
    )]
    ShieldedClaimBindingMismatch { mismatch: String },

    /// A shielded lifecycle operation could not obtain admission at the store, so it was refused
    /// rather than allowed to run concurrently with the operation that holds it.
    ///
    /// Two directions, both retryable:
    ///
    /// * A **one-time-key claim** refused because `clear` / `unregister_wallet` / `remove_wallet`
    ///   holds destructive admission over its wallet. Nothing was scanned, built or broadcast.
    /// * A **destructive operation** refused because in-flight claims still hold admission and did
    ///   not drain within the wait. Nothing was purged — deleting a pending-claim record while its
    ///   transition is on the wire strands the created identity, so the purge fails closed and the
    ///   caller retries.
    ///
    /// Admission is taken at the store rather than on the coordinator because that is the only
    /// state two coordinators — or two processes on the same SQLite file — actually share
    /// (`dashpay/platform#4313`).
    #[error("Shielded lifecycle operation refused: {reason}")]
    ShieldedLifecycleBusy { reason: String },

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

/// Extract the consensus verdict from the `dash_sdk::Error` shapes that can
/// carry one — `StateTransitionBroadcastError` (wait-stream),
/// `Protocol(ConsensusError)` (CheckTx), and the dapi-client's
/// exhausted-retry envelope it recurses into. Shared by the typed-promotion
/// matchers below; the same coverage caveat as
/// [`as_asset_lock_proof_cl_height_too_low`] applies (re-audit when
/// `dash_sdk::Error` gains consensus-carrying variants).
fn consensus_error_of(error: &dash_sdk::Error) -> Option<&dpp::consensus::ConsensusError> {
    match error {
        dash_sdk::Error::StateTransitionBroadcastError(broadcast_err) => {
            broadcast_err.cause.as_ref()
        }
        dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(ce)) => Some(ce.as_ref()),
        dash_sdk::Error::NoAvailableAddressesToRetry(inner) => consensus_error_of(inner),
        _ => None,
    }
}

/// Whether Platform rejected a transition because the exact asset-lock
/// outpoint it submitted has already been consumed.
///
/// Matches the structured consensus error carried by both CheckTx
/// (`Protocol(ConsensusError)`) and wait-stream
/// (`StateTransitionBroadcastError`) failures. The outpoint comparison is
/// deliberate: callers may only recognize a report for the tracked lock they
/// actually submitted, never an unrelated outpoint mentioned by a malformed
/// error. This signal alone does not authenticate terminal consumption.
pub fn is_asset_lock_already_consumed(
    error: &dash_sdk::Error,
    out_point: &dashcore::OutPoint,
) -> bool {
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::ConsensusError;

    matches!(
        consensus_error_of(error),
        Some(ConsensusError::BasicError(
            BasicError::IdentityAssetLockTransactionOutPointAlreadyConsumedError(e),
        )) if e.transaction_id() == out_point.txid
            && e.output_index() == out_point.vout as usize
    )
}

/// Promote a document-trade consensus rejection to its typed
/// [`PlatformWalletError`] so callers get structured data instead of a
/// stringified verdict:
///
/// - `DocumentNotForSaleError` (40108) → [`PlatformWalletError::DocumentNotForSale`]
/// - `DocumentIncorrectPurchasePriceError` (40109) →
///   [`PlatformWalletError::DocumentPriceChanged`] (carries both prices —
///   the race-lost purchase case; the transition did NOT execute)
/// - `IdentityInsufficientBalanceError` →
///   [`PlatformWalletError::InsufficientIdentityCredits`]
///
/// Returns `None` for anything else, leaving the caller's fallback mapping
/// in charge.
pub fn promote_document_trade_error(error: &dash_sdk::Error) -> Option<PlatformWalletError> {
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;

    match consensus_error_of(error)? {
        ConsensusError::StateError(StateError::DocumentNotForSaleError(e)) => {
            Some(PlatformWalletError::DocumentNotForSale {
                document_id: *e.document_id(),
            })
        }
        ConsensusError::StateError(StateError::DocumentIncorrectPurchasePriceError(e)) => {
            Some(PlatformWalletError::DocumentPriceChanged {
                document_id: *e.document_id(),
                expected: e.trying_to_purchase_at_price(),
                actual: e.actual_price(),
            })
        }
        ConsensusError::StateError(StateError::IdentityInsufficientBalanceError(e)) => {
            Some(PlatformWalletError::InsufficientIdentityCredits {
                identity_id: *e.identity_id(),
                required: e.required_balance(),
                available: e.balance(),
            })
        }
        _ => None,
    }
}

/// Map a document-trade transition's SDK error to a [`PlatformWalletError`]:
/// typed trade rejections first ([`promote_document_trade_error`]), then the
/// structured signer-key-unavailable preservation, then the caller's `wrap`
/// fallback. Owned-error `.map_err(...)?` analogue for the set-price /
/// purchase / transfer call sites.
pub fn promote_document_trade_error_or(
    error: dash_sdk::Error,
    wrap: impl FnOnce(dash_sdk::Error) -> PlatformWalletError,
) -> PlatformWalletError {
    if let Some(promoted) = promote_document_trade_error(&error) {
        return promoted;
    }
    preserve_signer_key_unavailable_or(error, wrap)
}

/// The reserved machine prefix that a typed `SigningKeyUnavailable` signer
/// completion stamps at the **start** of its `ProtocolError::Generic` payload.
/// Also stamped at position 0 of `MnemonicResolverCoreSigner::NotFound`'s
/// `Display`, which is how a missing key stays recognizable across key-wallet's
/// `Signer` surface (whose error type is only `Display`) — `sign_message`
/// checks this prefix on the signer's rendering before adding any context and
/// promotes the failure to the typed
/// [`PlatformWalletError::MessageSigningKeyUnavailable`].
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

#[cfg(test)]
mod asset_lock_already_consumed_tests {
    use super::*;
    use dash_sdk::error::StateTransitionBroadcastError;
    use dashcore::hashes::Hash;
    use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutPointAlreadyConsumedError;
    use dpp::consensus::basic::UnsupportedProtocolVersionError;

    fn out_point() -> dashcore::OutPoint {
        dashcore::OutPoint::new(dashcore::Txid::all_zeros(), 7)
    }

    fn consensus_error() -> dpp::consensus::ConsensusError {
        let out_point = out_point();
        IdentityAssetLockTransactionOutPointAlreadyConsumedError::new(
            out_point.txid,
            out_point.vout as usize,
        )
        .into()
    }

    fn unrelated_consensus_error() -> dpp::consensus::ConsensusError {
        UnsupportedProtocolVersionError::new(2, 1).into()
    }

    #[test]
    fn recognizes_protocol_consensus_error_for_exact_outpoint() {
        let error = dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(Box::new(
            consensus_error(),
        )));

        assert!(is_asset_lock_already_consumed(&error, &out_point()));
    }

    #[test]
    fn recognizes_broadcast_consensus_error_for_exact_outpoint() {
        let error = dash_sdk::Error::StateTransitionBroadcastError(StateTransitionBroadcastError {
            code: 10504,
            message: "asset lock already consumed".to_string(),
            cause: Some(consensus_error()),
        });

        assert!(is_asset_lock_already_consumed(&error, &out_point()));
    }

    #[test]
    fn ignores_unrelated_errors_and_different_outpoints() {
        let error = dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(Box::new(
            consensus_error(),
        )));
        let different_out_point = dashcore::OutPoint::new(dashcore::Txid::all_zeros(), 8);

        assert!(!is_asset_lock_already_consumed(
            &error,
            &different_out_point
        ));
        assert!(!is_asset_lock_already_consumed(
            &dash_sdk::Error::Generic("boom".to_string()),
            &out_point()
        ));

        let unrelated_protocol = dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(
            Box::new(unrelated_consensus_error()),
        ));
        assert!(!is_asset_lock_already_consumed(
            &unrelated_protocol,
            &out_point()
        ));

        let unrelated_broadcast =
            dash_sdk::Error::StateTransitionBroadcastError(StateTransitionBroadcastError {
                code: 10504,
                // Deliberately resembles the target message: matching must
                // depend on the structured cause, never this display text.
                message: "asset lock output already completely used".to_string(),
                cause: Some(unrelated_consensus_error()),
            });
        assert!(!is_asset_lock_already_consumed(
            &unrelated_broadcast,
            &out_point()
        ));
    }
}
