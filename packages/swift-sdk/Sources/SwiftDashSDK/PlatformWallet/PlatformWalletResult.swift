import Foundation
import DashSDKFFI

// MARK: - Typed result code

/// Swift mirror of the Rust `PlatformWalletFFIResultCode` enum.
public enum PlatformWalletResultCode: Int32, Sendable {
    case success = 0
    case errorInvalidHandle = 1
    case errorInvalidParameter = 2
    case errorNullPointer = 3
    case errorSerialization = 4
    case errorDeserialization = 5
    case errorWalletOperation = 6
    case errorIdentityNotFound = 7
    case errorContactNotFound = 8
    case errorInvalidNetwork = 9
    case errorInvalidIdentifier = 10
    case errorMemoryAllocation = 11
    case errorUtf8Conversion = 12
    case errorArithmeticOverflow = 13
    case errorNoSelectableInputs = 14
    case errorWalletAlreadyExists = 15
    /// Definitive shielded-broadcast failure: the shielded transition
    /// (identity-create, unshield, transfer, or withdrawal) was not executed
    /// (relay/CheckTx rejection or a platform-reported execution error), the
    /// spent notes were released, and the caller may retry.
    case errorShieldedBroadcastFailed = 16
    /// Shielded broadcast accepted but its execution result could not be
    /// confirmed; the identity may already exist on chain. The FFI also fills
    /// the derived id into `outIdentityId` on this code, so the caller can
    /// hold the slot rather than treat the registration as failed.
    case errorShieldedBroadcastUnconfirmed = 17
    /// A shielded operation (shield / unshield / transfer / withdrawal) was
    /// accepted by the relay but its execution result could not be
    /// confirmed. It may have executed on chain; for the spend-based
    /// operations the wallet keeps the notes reserved (a shield reserves
    /// nothing) until the next sync (or app restart) reconciles the
    /// outcome. Do NOT auto-retry — a retry would rebuild the bundle and
    /// could double-execute if the original landed.
    case errorShieldedSpendUnconfirmed = 18
    /// A shielded spend could not be built against a Platform-recorded anchor:
    /// the wallet's commitment tree isn't synced to a checkpoint Platform has
    /// recorded (an in-progress / interrupted sync leaves it mid-block). Nothing
    /// was broadcast and the notes were released. This is retryable — let the
    /// shielded sync reach a confirmed state and try again. Distinct from
    /// `errorShieldedSpendUnconfirmed`, which must NOT be retried.
    case errorShieldedNoRecordedAnchor = 19
    /// A core transaction broadcast (send, DashPay payment, or asset-lock
    /// funding) failed with an ambiguous outcome — the transaction may
    /// already be on the network. The wallet keeps the spent inputs' UTXO
    /// reservation, so an immediate retry fails at input selection instead
    /// of double-spending; the reservation TTL or a sync reconciles the
    /// outcome. Do NOT auto-retry.
    case errorTransactionBroadcastUnconfirmed = 20
    /// A masternode (evonode) identity credit withdrawal was broadcast and
    /// accepted, but its execution result could not be confirmed — it may
    /// already have executed, and the identity nonce was consumed for it, so
    /// a blind retry could submit a SECOND withdrawal. Do NOT retry; re-read
    /// the claimable balance and reconcile first. Definitive rejections keep
    /// their ordinary codes and stay retryable.
    case errorMasternodeWithdrawalUnconfirmed = 42
    /// The deterministic masternode list isn't available yet (SPV not
    /// running or masternode sync incomplete), so a list-backed query such
    /// as `locateMasternode` has nothing to search. Transient: retry once
    /// `spvProgress.masternodes` reports the list synced.
    /// (46 — 43/44/45 are held by the shielded-invite error trio, #4313.)
    case errorMasternodeListUnavailable = 46
    /// Definitively-failed address-nonce race: Platform rejected an
    /// address-funds transition (shield, or identity top-up-from-addresses)
    /// because the submitted address nonce raced Platform's expected value
    /// (a lagging DAPI replica read). The transition did NOT execute and any
    /// notes were released (a shield reserves none) — safe to retry; the retry
    /// re-fetches the nonce and self-heals. The submitted/expected nonce values
    /// travel in the message string, not as structured fields.
    case errorAddressNonceMismatch = 21
    /// Atomic Core selection found no or insufficient unreserved UTXOs.
    case errorCoreInsufficientFunds = 22
    case errorAssetLockNotTracked = 23
    case errorAssetLockAlreadyConsumed = 24
    case errorAssetLockFundingMismatch = 25
    /// Core definitively rejected the transaction. Its reserved inputs were
    /// released and a corrected transaction may be submitted again.
    case errorTransactionBroadcastRejected = 26
    /// A quiesce/drain barrier did not complete within its budget: an
    /// in-flight sync pass was still running when a Clear / reset /
    /// sync-stop needed it provably drained. The operation failed closed —
    /// no state was wiped — and the caller should retry once sync is idle.
    /// (Not returned by `destroy`: Rust owns the callback contexts, so a
    /// straggling worker is memory-safe and merely logged there.)
    case errorShutdownIncomplete = 27
    /// Asset-lock coin selection came up short over the *permitted* funding
    /// set (dashpay/platform#4073). Nothing was built or broadcast and no
    /// funding output was consumed, so the caller may refresh its preflight
    /// and retry.
    ///
    /// **Recovery depends on the funding form** — both reach this one code,
    /// and only one of them can be retried at a smaller amount:
    /// - *exact-amount* funding carries a caller-chosen amount, so the fix is
    ///   to re-run preflight and confirm a smaller one;
    /// - a whole-account *drain* accepts no amount argument at all, so there
    ///   is nothing to lower. Its shortfall means the account's drainable
    ///   balance is under the required minimum lock floor: add funds to that
    ///   account, or lower the floor.
    ///
    /// Do not present "try a smaller amount" to the user on the drain path.
    ///
    /// The structured `available` / `required` duff amounts travel in the
    /// message string — `PlatformWalletFFIResult` is ABI-frozen at code +
    /// message, so there are no out-params for them.
    ///
    /// Distinct from `errorCoreInsufficientFunds` (22), which is the atomic
    /// Core-send selector rather than the asset-lock builder. What the figures
    /// cover depends on the funding form: an exact-amount build pools the
    /// default source list (BIP44 + BIP32 + every DashPay contact-receiving
    /// account) and its shortfall describes that whole permitted union, while
    /// a whole-account *drain* build names exactly one account. CoinJoin funds
    /// only through the drain form.
    case errorAssetLockInsufficientFunds = 29
    /// A state transition could not be signed because the signer has no
    /// usable private key for the requested public key — restored from the
    /// structured signer completion code (dashpay/platform#4060 finding 7).
    /// Route to key repair; not retryable as-is.
    ///
    /// Also produced with no signer round-trip by
    /// `ManagedCoreWallet.signMessage`, for a message-signing address this
    /// wallet does not own — or owns only watch-only, a DashPay *external*
    /// account holding a contact's addresses.
    case errorSigningKeyUnavailable = 31
    // Codes 27-33 are claimed outside this PR — except 29, mirrored above —
    // and must not be reused here: 27 errorShutdownIncomplete
    // (dashpay/platform#4268, merged), 29 errorAssetLockInsufficientFunds
    // (claimed by #4184, which was closed unmerged along with its successor
    // #4316; this PR salvages the code at its reserved number, so the mirror
    // above matches the Rust discriminant rather than trailing it), 31
    // errorSigningKeyUnavailable (#4183/#4259), 32 errorTransactionBuild
    // (#4247/#4256), 33 errorTransactionSigning (#4256); 28 and 30 are free.
    // The deferred-token trio therefore occupies the contiguous block 34-36.
    // These raw values MUST match `PlatformWalletFFIResultCode` in
    // packages/rs-platform-wallet-ffi/src/error.rs — there is no compile-time
    // check across the ABI. See ERROR_CODE_REGISTRY.md (#4261).
    /// A deferred (BIP70/BIP270) reservation token has outlived its funding
    /// reservation's lifetime: key-wallet's TTL may already have swept and
    /// re-selected the inputs, so acting on it could touch a newer, unrelated
    /// reservation. The call did NOT touch the network. NOT retryable in place —
    /// rebuild the payment.
    case errorStaleReservationToken = 34
    /// A deferred reservation token is unknown, already broadcast, or already
    /// released — the guard that turns a double-broadcast (or a broadcast after
    /// release) into a typed error instead of a second send. The call did NOT
    /// touch the network. NOT retryable: rebuild the payment. (Release is
    /// idempotent and never surfaces this.)
    case errorReservationTokenConsumed = 35
    /// A deferred reservation token was minted against a different wallet
    /// *generation* than the one broadcasting it (e.g. a wallet re-created under
    /// the same id); its reservation lives in that other generation's reservation
    /// set. The call did NOT touch the network and did NOT consume the rightful
    /// owner's token. NOT retryable through this handle: rebuild the payment.
    case errorReservationWalletMismatch = 36
    // DPNS username-marketplace trade rejections (37-40) — a fresh
    // contiguous block above every prior claim, for the same reason the
    // 34-36 trio moved there.
    /// The document carries no `$price`, so it cannot be purchased (and a
    /// DPNS delist has nothing to clear). Raised by the wallet's pre-flight
    /// read and by the consensus `DocumentNotForSaleError` (40108). The
    /// transition did NOT execute.
    case errorDocumentNotForSale = 37
    /// The listing no longer matches the price the user confirmed — either
    /// the pre-flight read disagreed, or consensus rejected the broadcast
    /// (40109) because the listing changed in between. The purchase did NOT
    /// execute; re-confirm at the new price. The message is a stable JSON
    /// detail object carrying both prices.
    case errorDocumentPriceChanged = 38
    /// The identity's credit balance cannot cover the operation (purchase
    /// price + fee reserve, or Platform's own arithmetic). Nothing executed;
    /// top the identity up and retry. The message is a stable JSON detail
    /// object carrying the required and available amounts.
    case errorInsufficientIdentityCredits = 39
    /// The DPNS name is inside an active contested-name vote, so its domain
    /// document is not in the documents tree and no trade transition can
    /// reference it. Retry after the contest resolves. The message is a
    /// stable JSON detail object carrying the label and the vote end time.
    case errorContestedNameNotTradable = 40
    /// A Platform-to-shielded operation can no longer cover the requested
    /// amount plus input 0's retained fee reserve. Refresh the shield
    /// preflight and ask the user to confirm the new capacity.
    case errorShieldedInsufficientBalance = 41
    /// RESERVED — the Rust side has no code path that produces this today, so
    /// it does not currently cross the boundary. It is the TERMINAL form of
    /// the double-spend verdict: the tracked asset-lock transaction spends an
    /// outpoint a different, already-confirmed transaction of the same wallet
    /// spent first, AND that spender's block is proven to be on the finalized
    /// chain. The proof is what is missing — chainlock contexts and the
    /// wallet's applied chainlock height are height-based promotion artifacts,
    /// not evidence of finalized ancestry — so every detection reports
    /// `errorAssetLockInputContested` (48) instead, chainlocked-looking
    /// spenders included. Kept pinned so the slot stays stable for hosts and
    /// for the future emitter, which would carry the same meaning: the one
    /// code that lets a host discard the asset lock and rebuild from
    /// currently-unspent inputs. Read nothing into its absence.
    case errorAssetLockInputConflict = 47
    /// A confirmed transaction of this wallet already spent one of the tracked
    /// lock's inputs — typically a restored wallet whose rescan resurrected a
    /// UTXO one of its own earlier asset locks had already consumed. Peers drop
    /// such a double spend without replying, so the lock cannot confirm while
    /// that spender stands and an unbounded proof wait would hang. The resume
    /// still runs — the sighting bounds that wait rather than replacing it, so
    /// the lock was (re-)broadcast and waited on (a `Broadcast`-status lock
    /// was also sent on an earlier call) — and this is what the bounded wait
    /// expired with. This is the ONLY double-spend code the SDK emits, and it
    /// is PROVISIONAL: no discard licence, keep the lock tracked and retry
    /// later. A later chainlock does not upgrade it to 47 today; what a retry
    /// can resolve is a reorg dropping the sibling. Repetition licenses
    /// nothing either — a conflict that persists across sessions still does
    /// not prove finalized ancestry. Its absence is not proof of liveness —
    /// the Rust-side scan cannot see conflicts whose spender was already
    /// pruned.
    case errorAssetLockInputContested = 48
    // Persister failure classes (49-50) — claimed from the error-code
    // registry's allocation frontier; these raw values are hand-mirrored ABI.
    // errorPersisterTransient was originally 48; merged #4356 took 48 for
    // errorAssetLockInputContested on 2026-08-31, so it moved to 50 — see
    // the registry's row 50. errorPersisterFatal was never contested and
    // keeps 49.
    /// A persister operation failed transiently; callers may retry.
    case errorPersisterTransient = 50
    /// A persister operation failed permanently; callers must not retry.
    case errorPersisterFatal = 49
    /// The named thing does not exist. Besides the handle/lookup failures this
    /// has always covered, BOTH deferred-send paths report the
    /// wallet-was-REMOVED case here.
    ///
    /// Deferred (BIP70/BIP270) *token* path: a signed-payment broadcast refuses
    /// a token whose wallet is no longer registered in the manager, and a
    /// signed-payment finalize refuses to register a payment whose wallet was
    /// removed while it was being signed.
    ///
    /// Finalized-transaction *handle* path: `finalizeAtomic` publishes no
    /// handle when the wallet was removed or re-created during signing, and
    /// `broadcastTransactionWithOutcome(_: FinalizedCoreTransaction)` refuses a
    /// handle whose generation is gone.
    ///
    /// Every one of these reconciles the build's UTXO reservation before
    /// returning. Distinct from `errorReservationWalletMismatch` (36), where a
    /// *different* live generation answers to the same id. The call did NOT touch
    /// the network and is NOT retryable — the wallet is gone.
    case notFound = 98
    case errorUnknown = 99

    init(ffi: PlatformWalletFFIResultCode) {
        switch ffi {
        case PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS:
            self = .success
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_INVALID_HANDLE:
            self = .errorInvalidHandle
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_INVALID_PARAMETER:
            self = .errorInvalidParameter
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_NULL_POINTER:
            self = .errorNullPointer
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SERIALIZATION:
            self = .errorSerialization
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_DESERIALIZATION:
            self = .errorDeserialization
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_WALLET_OPERATION:
            self = .errorWalletOperation
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_IDENTITY_NOT_FOUND:
            self = .errorIdentityNotFound
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_CONTACT_NOT_FOUND:
            self = .errorContactNotFound
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_INVALID_NETWORK:
            self = .errorInvalidNetwork
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_INVALID_IDENTIFIER:
            self = .errorInvalidIdentifier
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_MEMORY_ALLOCATION:
            self = .errorMemoryAllocation
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_UTF8_CONVERSION:
            self = .errorUtf8Conversion
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_ARITHMETIC_OVERFLOW:
            self = .errorArithmeticOverflow
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_NO_SELECTABLE_INPUTS:
            self = .errorNoSelectableInputs
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_WALLET_ALREADY_EXISTS:
            self = .errorWalletAlreadyExists
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHIELDED_BROADCAST_FAILED:
            self = .errorShieldedBroadcastFailed
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHIELDED_BROADCAST_UNCONFIRMED:
            self = .errorShieldedBroadcastUnconfirmed
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHIELDED_SPEND_UNCONFIRMED:
            self = .errorShieldedSpendUnconfirmed
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHIELDED_NO_RECORDED_ANCHOR:
            self = .errorShieldedNoRecordedAnchor
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_TRANSACTION_BROADCAST_UNCONFIRMED:
            self = .errorTransactionBroadcastUnconfirmed
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_MASTERNODE_WITHDRAWAL_UNCONFIRMED:
            self = .errorMasternodeWithdrawalUnconfirmed
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_MASTERNODE_LIST_UNAVAILABLE:
            self = .errorMasternodeListUnavailable
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_ADDRESS_NONCE_MISMATCH:
            self = .errorAddressNonceMismatch
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_CORE_INSUFFICIENT_FUNDS:
            self = .errorCoreInsufficientFunds
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_ASSET_LOCK_NOT_TRACKED:
            self = .errorAssetLockNotTracked
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_ASSET_LOCK_ALREADY_CONSUMED:
            self = .errorAssetLockAlreadyConsumed
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_ASSET_LOCK_FUNDING_MISMATCH:
            self = .errorAssetLockFundingMismatch
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_ASSET_LOCK_INSUFFICIENT_FUNDS:
            self = .errorAssetLockInsufficientFunds
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_PERSISTER_TRANSIENT:
            self = .errorPersisterTransient
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_PERSISTER_FATAL:
            self = .errorPersisterFatal
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_TRANSACTION_BROADCAST_REJECTED:
            self = .errorTransactionBroadcastRejected
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHUTDOWN_INCOMPLETE:
            self = .errorShutdownIncomplete
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SIGNING_KEY_UNAVAILABLE:
            self = .errorSigningKeyUnavailable
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_STALE_RESERVATION_TOKEN:
            self = .errorStaleReservationToken
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_RESERVATION_TOKEN_CONSUMED:
            self = .errorReservationTokenConsumed
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_RESERVATION_WALLET_MISMATCH:
            self = .errorReservationWalletMismatch
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_DOCUMENT_NOT_FOR_SALE:
            self = .errorDocumentNotForSale
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_DOCUMENT_PRICE_CHANGED:
            self = .errorDocumentPriceChanged
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_INSUFFICIENT_IDENTITY_CREDITS:
            self = .errorInsufficientIdentityCredits
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_CONTESTED_NAME_NOT_TRADABLE:
            self = .errorContestedNameNotTradable
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHIELDED_INSUFFICIENT_BALANCE:
            self = .errorShieldedInsufficientBalance
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_ASSET_LOCK_INPUT_CONFLICT:
            self = .errorAssetLockInputConflict
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_ASSET_LOCK_INPUT_CONTESTED:
            self = .errorAssetLockInputContested
        case PLATFORM_WALLET_FFI_RESULT_CODE_NOT_FOUND:
            self = .notFound
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_UNKNOWN:
            self = .errorUnknown
        default:
            self = .errorUnknown
        }
    }
}

// MARK: - Class wrapper

/// Reference-counted wrapper around a `PlatformWalletFFIResult`
/// returned by Rust. Owns the heap-allocated `message` C string and
/// frees it through `platform_wallet_ffi_result_free` in `deinit`,
/// regardless of whether the caller threw, returned early, or simply
/// dropped the wrapper.
///
/// Use the `try ffiResult.check()` extension below for the common
/// "throw on non-success" shape; reach for an explicit instance when
/// you want to inspect the `message` without throwing (e.g. logging
/// a warning on a soft-failure path).
final class PlatformWalletResult {
    private var inner: PlatformWalletFFIResult

    init(_ ffi: PlatformWalletFFIResult) {
        self.inner = ffi
    }

    deinit {
        platform_wallet_ffi_result_free(&inner)
    }

    /// Typed result code; unknown raw values fall back to `.errorUnknown`.
    var code: PlatformWalletResultCode {
        PlatformWalletResultCode(ffi: inner.code)
    }

    var message: String? {
        inner.message.map { String(cString: $0) }
    }

    var isSuccess: Bool {
        code == .success
    }

    func throwIfError() throws {
        guard !isSuccess else { return }
        throw PlatformWalletError(result: self)
    }
}

// MARK: - PlatformWalletError

/// Platform Wallet error type.
///
/// Built directly from a `PlatformWalletResult` via
/// `PlatformWalletError(result:)`. The wrapper owns both the typed
/// code and the Rust-supplied message, so taking it as a single
/// input avoids mismatched (code, message) pairs at the call site.
/// Most call sites just write `try ffi(...).check()` and let the
/// FFI extension do the construction.
public enum PlatformWalletError: LocalizedError {
    case nullPointer(String)
    case invalidHandle(String)
    case invalidParameter(String)
    case invalidIdentifier(String)
    case invalidNetwork(String)
    case walletOperation(String)
    case identityNotFound(String)
    case contactNotFound(String)
    case utf8Conversion(String)
    case serialization(String)
    case deserialization(String)
    case memoryAllocation(String)
    case arithmeticOverflow(String)
    case noSelectableInputs(String)
    case coreInsufficientFunds(String)
    case assetLockNotTracked(String)
    /// The one-shot output cannot be reused. This may come from a retained
    /// local tombstone or an unauthenticated Platform report, so it does not
    /// prove that the requested operation completed.
    case assetLockAlreadyConsumed(String)
    case assetLockFundingMismatch(String)
    /// Asset-lock coin selection could not cover the requested funding over
    /// the permitted source set. Nothing was built or broadcast and no
    /// funding output was consumed — refresh the preflight, then recover by
    /// the funding form: an exact-amount build can confirm a smaller amount,
    /// while a whole-account drain takes no amount to lower and instead needs
    /// funds added to the drained account (or a lower minimum lock floor).
    /// The `available` / `required` duff figures are in the message. Kotlin
    /// parity: `DashSdkError.PlatformWallet.AssetLockInsufficientFunds`.
    case assetLockInsufficientFunds(String)
    /// A persister operation failed transiently; callers may retry.
    case persisterTransient(String)
    /// A persister operation failed permanently; callers must not retry.
    case persisterFatal(String)
    case walletAlreadyExists(String)
    /// Definitive shielded-broadcast failure: the shielded transition
    /// (identity-create or a spend — unshield / transfer / withdrawal) was
    /// not executed and the spent notes were released; safe to retry.
    case shieldedBroadcastFailed(String)
    /// Shielded broadcast accepted but its execution result could not be
    /// confirmed; the identity may already exist on chain. Callers that need
    /// the derived id should special-case the
    /// `.errorShieldedBroadcastUnconfirmed` result code (which carries
    /// `outIdentityId`) before falling back to this error — see
    /// `ShieldedIdentityCreateUnconfirmedError`.
    case shieldedBroadcastUnconfirmed(String)
    /// A shielded operation (shield / unshield / transfer / withdrawal)
    /// was accepted by the relay but its execution result could not be
    /// confirmed. It may have executed; spend-based operations keep their
    /// notes reserved wallet-side (a shield reserves nothing) until the
    /// next sync reconciles the outcome. Do NOT auto-retry.
    case shieldedSpendUnconfirmed(String)
    /// A shielded spend could not be built against a Platform-recorded anchor —
    /// the wallet's commitment tree isn't synced to a checkpoint Platform has
    /// recorded (an in-progress / interrupted sync leaves it mid-block). Nothing
    /// was broadcast and the notes were released; retryable once the shielded
    /// sync reaches a confirmed state. Distinct from `shieldedSpendUnconfirmed`,
    /// which must NOT be retried.
    case shieldedNoRecordedAnchor(String)
    /// The cached Platform Payment-account input set cannot cover a shield's
    /// requested amount plus the fee reserve retained on input 0. Despite the
    /// retained public Swift/FFI name, this is not a shielded-pool balance
    /// failure.
    case shieldedInsufficientBalance(String)
    /// A core transaction broadcast was submitted but its outcome is
    /// unknown — the transaction may already be on the network. The wallet
    /// keeps the spent inputs reserved so a retry cannot double-spend; the
    /// reservation TTL or a later sync reconciles the outcome. Do NOT
    /// auto-retry. Core sibling of `shieldedSpendUnconfirmed`.
    case transactionBroadcastUnconfirmed(String)
    /// A masternode (evonode) credit withdrawal was broadcast and accepted
    /// but its result could not be confirmed. It may already have executed
    /// and the identity nonce was consumed — do NOT retry; re-read the
    /// claimable balance first (`.errorMasternodeWithdrawalUnconfirmed`).
    case masternodeWithdrawalUnconfirmed(String)
    /// The masternode list hasn't synced yet, so there is nothing to look a
    /// masternode up in (`.errorMasternodeListUnavailable`). Retry after
    /// masternode sync completes.
    case masternodeListUnavailable(String)
    /// Core definitively rejected the transaction and its input reservation
    /// was released. Unlike `transactionBroadcastUnconfirmed`, retry is safe.
    case transactionBroadcastRejected(String)
    /// Definitively-failed address-nonce race (shield, or identity
    /// top-up-from-addresses): Platform rejected the transition because the
    /// submitted address nonce raced its expected value. The transition did
    /// NOT execute and any notes were released (a shield reserves none) — safe
    /// to retry, and the retry re-fetches the address nonce so the mismatch
    /// self-heals. The submitted/expected nonce values are in the message.
    case addressNonceMismatch(String)
    /// A quiesce/drain barrier (Clear / reset / sync-stop) timed out with a
    /// sync pass still in flight. The operation failed closed — retry once
    /// sync is idle.
    case shutdownIncomplete(String)
    /// The signer has no usable private key for the requested public key
    /// (missing / stranded scalar) — the operation itself did not fail.
    /// Restored from the structured signer completion code
    /// (dashpay/platform#4060 finding 7); route to key repair. Kotlin
    /// parity: `DashSdkError.PlatformWallet.SigningKeyUnavailable`.
    ///
    /// `signMessage` also raises it for an address the wallet does not own.
    /// Distinct from `invalidParameter`, which means the address itself is
    /// unusable (unparseable, wrong network, or not P2PKH).
    case signingKeyUnavailable(String)
    /// A deferred (BIP70/BIP270) reservation token has outlived its funding
    /// reservation's lifetime — key-wallet's TTL may already have swept and
    /// re-selected the inputs. Nothing was broadcast. NOT retryable in place;
    /// rebuild the payment. Sibling of `reservationTokenConsumed` and
    /// `reservationWalletMismatch`, which this code used to conflate.
    case staleReservationToken(String)
    /// A deferred reservation token is unknown, already broadcast, or already
    /// released — the double-broadcast guard. Nothing was broadcast. NOT
    /// retryable; rebuild the payment.
    case reservationTokenConsumed(String)
    /// A deferred reservation token was minted against a different wallet
    /// generation than the one broadcasting it (e.g. a wallet re-created under
    /// the same id). Nothing was broadcast and the rightful owner's token was
    /// not consumed. NOT retryable through this handle; rebuild the payment.
    case reservationWalletMismatch(String)
    /// The DPNS name / document is not listed for sale, so it cannot be
    /// purchased and has no price to clear. Raised by the wallet's
    /// pre-flight read and by the consensus not-for-sale rejection; the
    /// transition did NOT execute.
    case notForSale(String)
    /// The listing no longer matches the price the user confirmed. Rebuilt
    /// from the FFI's JSON detail, so the UI can show both numbers and
    /// re-prompt at `actual`. Nothing was purchased — a purchase is only
    /// ever broadcast at the confirmed price, and consensus rejects it if
    /// the listing moved.
    case priceChanged(documentId: String, expected: UInt64, actual: UInt64)
    /// The identity's credit balance cannot cover the operation. Both
    /// amounts are in credits (1 duff = 1000 credits) and include the fee
    /// reserve the wallet requires on top of a purchase price. Nothing
    /// executed; top the identity up and retry.
    case insufficientIdentityCredits(identityId: String, required: UInt64, available: UInt64)
    /// The DPNS name is inside an active contested-name vote and cannot be
    /// listed, transferred, or purchased until the contest resolves.
    /// `endsAtMs == 0` means the vote's end time was unavailable — show it
    /// as unknown rather than as "ends at the epoch".
    case contestedNameNotTradable(label: String, endsAtMs: UInt64)
    /// RESERVED, and never produced today: the TERMINAL double-spend verdict,
    /// which would additionally attest that the confirmed spender's block is
    /// on the finalized chain. The wallet cannot prove that (chainlock
    /// contexts and the applied chainlock height are height-based promotion
    /// artifacts, not ancestry proofs), so every detection arrives as
    /// `assetLockInputContested`. The case is kept so the FFI code stays
    /// mapped and hosts that already branch on it keep compiling; if it ever
    /// ships it means what it always meant — unlike
    /// `transactionBroadcastUnconfirmed`, where the transaction may well be
    /// alive and discarding it would strand real funds, this is the one
    /// asset-lock error that lets a host discard the lock and rebuild it from
    /// currently-unspent inputs. The message names the lock's outpoint, the
    /// conflicting input, the confirmed spender, and that spender's finality.
    case assetLockInputConflict(String)
    /// The tracked asset lock spends an outpoint a different,
    /// already-confirmed transaction of this wallet spent first, so no peer
    /// will relay it while that spender stands. The resume still ran — it
    /// re-broadcast and waited for a proof under a bounded timeout, and this
    /// is what the wait expired with. A `Broadcast`-status lock was also
    /// already sent on an earlier call, so this is not a claim that nothing
    /// ever reached the network.
    ///
    /// The only double-spend verdict the SDK emits, and PROVISIONAL: the
    /// tracked lock must NOT be discarded on this error. A conflict that
    /// persists across sessions still does not prove finalized ancestry —
    /// the sighting can even be a block record restored from a previous
    /// session whose block was reorganized out while the wallet was offline.
    /// Keep the tracked lock and continue treating this result as retryable;
    /// only `assetLockInputConflict`, or an independent finalized-ancestry
    /// proof, may authorize discarding it. No funds move either way: the
    /// confirmed spender is this wallet's own transaction, so the value
    /// behind the contested input lives on in it.
    case assetLockInputContested(String)
    /// The named thing does not exist. For the deferred payment calls this is
    /// the wallet-was-REMOVED case: the token's wallet (or the wallet a payment
    /// was just signed against) is no longer registered in the manager, so there
    /// is no live generation to act through. Nothing was broadcast; the
    /// finalize path reconciles the build's reservation before returning. NOT
    /// retryable — unlike `reservationWalletMismatch`, no other generation holds
    /// this payment either.
    case notFound(String)
    case unknown(String)

    /// Diagnostic detail Rust attached to the originating
    /// `PlatformWalletFFIResult`, or the context string a Swift-side
    /// guard chose when constructing the error inline.
    public var errorDescription: String? {
        switch self {
        case .nullPointer(let m), .invalidHandle(let m), .invalidParameter(let m),
             .invalidIdentifier(let m), .invalidNetwork(let m), .walletOperation(let m),
             .identityNotFound(let m), .contactNotFound(let m), .utf8Conversion(let m),
             .serialization(let m), .deserialization(let m), .memoryAllocation(let m),
             .arithmeticOverflow(let m), .noSelectableInputs(let m),
             .coreInsufficientFunds(let m),
             .assetLockNotTracked(let m), .assetLockAlreadyConsumed(let m),
             .assetLockFundingMismatch(let m), .assetLockInsufficientFunds(let m),
             .persisterTransient(let m), .persisterFatal(let m),
             .walletAlreadyExists(let m), .shieldedBroadcastFailed(let m),
             .shieldedBroadcastUnconfirmed(let m), .shieldedSpendUnconfirmed(let m),
             .shieldedNoRecordedAnchor(let m), .shieldedInsufficientBalance(let m),
             .transactionBroadcastUnconfirmed(let m),
             .masternodeWithdrawalUnconfirmed(let m),
             .masternodeListUnavailable(let m),
             .transactionBroadcastRejected(let m),
             .addressNonceMismatch(let m),
             .shutdownIncomplete(let m),
             .signingKeyUnavailable(let m),
             .staleReservationToken(let m), .reservationTokenConsumed(let m),
             .reservationWalletMismatch(let m),
             .notForSale(let m),
             .assetLockInputConflict(let m),
             .assetLockInputContested(let m),
             .notFound(let m), .unknown(let m):
            return m
        // The three value-carrying marketplace rejections compose their
        // description from the typed values, because their FFI message is
        // the machine-readable JSON detail — showing that raw would be
        // gibberish in a UI alert.
        case .priceChanged(_, let expected, let actual):
            return "The price changed from \(expected) to \(actual) credits before the purchase "
                + "could be confirmed. Nothing was purchased."
        case .insufficientIdentityCredits(let identityId, let required, let available):
            return "Identity \(identityId) has \(available) credits but \(required) are required."
        case .contestedNameNotTradable(let label, let endsAtMs):
            if endsAtMs == 0 {
                return "\"\(label)\" is in an active contested-name vote and cannot be traded "
                    + "until the contest resolves."
            }
            return "\"\(label)\" is in an active contested-name vote (ends at \(endsAtMs) ms) "
                + "and cannot be traded until the contest resolves."
        }
    }

    init(result: PlatformWalletResult) {
        self.init(code: result.code, message: result.message)
    }

    /// Internal seam for exercising the stable error-code/detail contract
    /// without manufacturing a Rust-owned `PlatformWalletFFIResult` string.
    /// Production callers continue to enter through `init(result:)`.
    init(code: PlatformWalletResultCode, message: String?) {
        let detail = message ?? "<no detail from Rust>"
        switch code {
        case .success:
            // Constructing an error from a success result is a caller bug
            self = .unknown(message
                ?? "PlatformWalletError built from a success result")
        case .errorInvalidHandle:     self = .invalidHandle(detail)
        case .errorInvalidParameter:  self = .invalidParameter(detail)
        case .errorNullPointer:       self = .nullPointer(detail)
        case .errorSerialization:     self = .serialization(detail)
        case .errorDeserialization:   self = .deserialization(detail)
        case .errorWalletOperation:   self = .walletOperation(detail)
        case .errorIdentityNotFound:  self = .identityNotFound(detail)
        case .errorContactNotFound:   self = .contactNotFound(detail)
        case .errorInvalidNetwork:    self = .invalidNetwork(detail)
        case .errorInvalidIdentifier: self = .invalidIdentifier(detail)
        case .errorMemoryAllocation:  self = .memoryAllocation(detail)
        case .errorUtf8Conversion:    self = .utf8Conversion(detail)
        case .errorArithmeticOverflow: self = .arithmeticOverflow(detail)
        case .errorNoSelectableInputs: self = .noSelectableInputs(detail)
        case .errorCoreInsufficientFunds: self = .coreInsufficientFunds(detail)
        case .errorAssetLockNotTracked: self = .assetLockNotTracked(detail)
        case .errorAssetLockAlreadyConsumed: self = .assetLockAlreadyConsumed(detail)
        case .errorAssetLockFundingMismatch: self = .assetLockFundingMismatch(detail)
        case .errorAssetLockInsufficientFunds: self = .assetLockInsufficientFunds(detail)
        case .errorPersisterTransient: self = .persisterTransient(detail)
        case .errorPersisterFatal: self = .persisterFatal(detail)
        case .errorWalletAlreadyExists: self = .walletAlreadyExists(detail)
        case .errorShieldedBroadcastFailed: self = .shieldedBroadcastFailed(detail)
        case .errorShieldedBroadcastUnconfirmed: self = .shieldedBroadcastUnconfirmed(detail)
        case .errorShieldedSpendUnconfirmed: self = .shieldedSpendUnconfirmed(detail)
        case .errorShieldedNoRecordedAnchor: self = .shieldedNoRecordedAnchor(detail)
        case .errorShieldedInsufficientBalance: self = .shieldedInsufficientBalance(detail)
        case .errorTransactionBroadcastUnconfirmed:
            self = .transactionBroadcastUnconfirmed(detail)
        case .errorMasternodeWithdrawalUnconfirmed:
            self = .masternodeWithdrawalUnconfirmed(detail)
        case .errorMasternodeListUnavailable:
            self = .masternodeListUnavailable(detail)
        case .errorTransactionBroadcastRejected:
            self = .transactionBroadcastRejected(detail)
        case .errorAddressNonceMismatch:
            self = .addressNonceMismatch(detail)
        case .errorShutdownIncomplete:
            self = .shutdownIncomplete(detail)
        case .errorSigningKeyUnavailable:
            self = .signingKeyUnavailable(detail)
        case .errorStaleReservationToken:
            self = .staleReservationToken(detail)
        case .errorReservationTokenConsumed:
            self = .reservationTokenConsumed(detail)
        case .errorReservationWalletMismatch:
            self = .reservationWalletMismatch(detail)
        case .errorDocumentNotForSale:
            self = .notForSale(detail)
        // Codes 38/39/40 carry a stable JSON detail object instead of a
        // human message (see `PlatformWalletFFIResultCode` in
        // packages/rs-platform-wallet-ffi/src/error.rs). Decode it into the
        // typed case; if the payload is ever absent or reshaped, degrade to
        // `.unknown(detail)` rather than trapping — a malformed message must
        // not crash the host on an error path.
        case .errorDocumentPriceChanged:
            if let d = TradeErrorDetail.priceChanged(detail) {
                self = .priceChanged(
                    documentId: d.documentId,
                    expected: d.expected,
                    actual: d.actual
                )
            } else {
                self = .unknown(detail)
            }
        case .errorInsufficientIdentityCredits:
            if let d = TradeErrorDetail.insufficientCredits(detail) {
                self = .insufficientIdentityCredits(
                    identityId: d.identityId,
                    required: d.required,
                    available: d.available
                )
            } else {
                self = .unknown(detail)
            }
        case .errorContestedNameNotTradable:
            if let d = TradeErrorDetail.contestedName(detail) {
                self = .contestedNameNotTradable(label: d.label, endsAtMs: d.endsAtMs)
            } else {
                self = .unknown(detail)
            }
        // Both double-spend codes carry the typed `Display` rendering, not a
        // JSON detail object: it already names the asset-lock outpoint, the
        // conflicting input, the confirmed spender's txid and that spender's
        // finality, and reads as a sentence, so they pass through like the
        // other prose-message codes. Which verdict was reached is the CODE's
        // meaning, not the string's — hosts must branch on the case, not on
        // text matching. In practice only 48 arrives; 47 is reserved and has
        // no emitter, and is mapped here so it stays typed if that changes.
        case .errorAssetLockInputConflict:
            self = .assetLockInputConflict(detail)
        case .errorAssetLockInputContested:
            self = .assetLockInputContested(detail)
        case .notFound:               self = .notFound(detail)
        case .errorUnknown:           self = .unknown(detail)
        }
    }
}

// MARK: - Marketplace trade-error detail decoding

/// Decoders for the stable JSON detail objects that FFI result codes
/// 38/39/40 put in the result `message`.
///
/// `PlatformWalletFFIResult` is ABI-frozen at `{ code, message }`, so the
/// wallet layer's typed values (both prices, both credit amounts, the
/// contest end time) can only cross as a documented JSON object. Each
/// decoder returns `nil` on anything that isn't that object, and the
/// caller degrades to `.unknown(message)` — never a trap and never a
/// fabricated zero.
private enum TradeErrorDetail {
    struct PriceChanged: Decodable {
        let documentId: String
        let expected: UInt64
        let actual: UInt64
    }

    struct InsufficientCredits: Decodable {
        let identityId: String
        let required: UInt64
        let available: UInt64
    }

    struct ContestedName: Decodable {
        let label: String
        let endsAtMs: UInt64
    }

    static func priceChanged(_ message: String?) -> PriceChanged? {
        decode(PriceChanged.self, from: message)
    }

    static func insufficientCredits(_ message: String?) -> InsufficientCredits? {
        decode(InsufficientCredits.self, from: message)
    }

    static func contestedName(_ message: String?) -> ContestedName? {
        decode(ContestedName.self, from: message)
    }

    private static func decode<T: Decodable>(_ type: T.Type, from message: String?) -> T? {
        guard let data = message?.data(using: .utf8) else { return nil }
        return try? JSONDecoder().decode(type, from: data)
    }
}

// MARK: - Convenience extensions

extension PlatformWalletFFIResult {
    @inline(__always)
    func check() throws {
        try PlatformWalletResult(self).throwIfError()
    }

    @inline(__always)
    func discard() {
        _ = PlatformWalletResult(self)
    }
}
