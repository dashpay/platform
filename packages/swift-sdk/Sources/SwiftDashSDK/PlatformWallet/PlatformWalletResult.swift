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
    /// Shielded-broadcast failure, typically refused before anything was
    /// sent (no node address, or the SDK's own refusal). Whether it may run
    /// again is the result's payment class (`PlatformPaymentRecovery`), not
    /// this code. A node's refusal or consensus verdict is unconfirmed
    /// instead (18, or 17 for identity-create).
    case errorShieldedBroadcastFailed = 16
    /// An identity create from the pool may have reached a node (accepted, a
    /// send attempt that may have delivered it, a node's refusal or verdict),
    /// its result is not proved and a direct fetch of the derived id came
    /// back empty. The wallet keeps the notes reserved and the saved
    /// transition. The identity may already exist on chain: do not treat it
    /// as unregistered or re-submit. The FFI fills the derived id into
    /// `outIdentityId` on this code, so the caller can hold the slot.
    case errorShieldedBroadcastUnconfirmed = 17
    /// A shielded operation (shield from Platform addresses, unshield /
    /// transfer / withdrawal) may have reached a node and its result is not
    /// proved: a failed result wait, a node's refusal or a consensus verdict
    /// (a stale address nonce included). It may have executed. Spends keep
    /// their notes reserved and the saved transition for the wallet to
    /// resolve; a shield's activity row stays pending until the scan sees its
    /// note. Do NOT auto-retry: a retry signs a second transition.
    case errorShieldedSpendUnconfirmed = 18
    /// A shielded spend could not be built against a Platform-recorded anchor:
    /// the wallet's commitment tree isn't synced to a checkpoint Platform has
    /// recorded (an in-progress / interrupted sync leaves it mid-block). Nothing
    /// was broadcast and the notes were released. This is retryable — let the
    /// shielded sync reach a confirmed state and try again. Distinct from
    /// `errorShieldedSpendUnconfirmed`, which must NOT be retried.
    case errorShieldedNoRecordedAnchor = 19
    /// A Core transaction or Platform state transition may have executed
    /// but its outcome could not be confirmed. Reconcile the original operation
    /// before another submission. Core input protections do not apply to
    /// Platform state transitions. Do NOT auto-retry.
    case errorTransactionBroadcastUnconfirmed = 20
    /// A masternode (evonode) identity credit withdrawal may have reached a
    /// node and its remaining balance is not proved (a failed result wait, a
    /// node's refusal or a consensus verdict included). It may have executed;
    /// a retry signs a second withdrawal with the next identity nonce. Do NOT
    /// retry; re-read the claimable balance first. Only a withdrawal that
    /// failed before anything was sent keeps an ordinary, retryable code.
    case errorMasternodeWithdrawalUnconfirmed = 42
    /// The deterministic masternode list isn't available yet (SPV not
    /// running or masternode sync incomplete), so a list-backed query such
    /// as `locateMasternode` has nothing to search. Transient: retry once
    /// `spvProgress.masternodes` reports the list synced.
    /// (46 — 43/44/45 are held by the shielded-invite error trio, #4313.)
    case errorMasternodeListUnavailable = 46
    /// Address-nonce race: Platform rejected an address-funds transition (a
    /// transfer or withdrawal from Platform addresses, or an identity
    /// registration from them) because the submitted address nonce
    /// raced Platform's expected value (a lagging DAPI replica read). Not a
    /// retry signal: the verdict may follow an earlier dispatch that landed,
    /// so Rust classes it `.maybeSent` and not retryable. A shield does not
    /// return it (see `errorShieldedSpendUnconfirmed`), nor does an identity
    /// top-up from Platform addresses (its nonce verdict is
    /// `errorTransactionBroadcastUnconfirmed`, 20). The submitted/expected
    /// nonce values travel in the message string, not as structured fields.
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
    /// Also returned by SPV start, stop and storage clear while an SPV
    /// teardown is still running: stop SPV again.
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
    /// message string: `PlatformWalletFFIResult` carries no per-error value
    /// fields, so there are no out-params for them.
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
    /// touch the network, but an earlier broadcast of the token may have sent
    /// the payment, so it is `.maybeSent`: let sync settle it before building
    /// the payment again. NOT retryable. (Release is idempotent and never
    /// surfaces this.)
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
    /// read and by the consensus `DocumentNotForSaleError` (40108). A verdict
    /// may follow an earlier dispatch, so whether anything executed is the
    /// result's payment class, not this code.
    case errorDocumentNotForSale = 37
    /// The listing no longer matches the price the user confirmed — either
    /// the pre-flight read disagreed, or consensus rejected the broadcast
    /// (40109) because the listing changed in between. Whether anything was
    /// purchased is the result's payment class, not this code. The message is a stable JSON
    /// detail object carrying both prices.
    case errorDocumentPriceChanged = 38
    /// The identity's credit balance cannot cover the operation (purchase
    /// price + fee reserve, or Platform's own arithmetic). Whether anything
    /// executed is the result's payment class. The message is a stable JSON detail
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
    /// still attempts recovery. With a ready transport, the sighting bounds
    /// the proof wait and this is what that wait expired with. In the
    /// `Broadcast` arm, after a readiness miss and pre-dispatch rejection, a
    /// still-standing conflict returns immediately after refreshing local
    /// finality, and the readiness-deferred retry owns the next proof wait. A
    /// `Broadcast`-status lock may also represent an earlier attempt that sent
    /// the transaction. This is the ONLY double-spend code the SDK emits,
    /// and it is PROVISIONAL: no discard licence, keep the lock tracked and
    /// retry later. A later chainlock does not upgrade it to 47 today; what a
    /// retry can resolve is a reorg dropping the sibling. Repetition licenses
    /// nothing either — a conflict that persists across sessions still does
    /// not prove finalized ancestry. Its absence is not proof of liveness —
    /// the Rust-side scan cannot see conflicts whose spender was already
    /// pruned.
    case errorAssetLockInputContested = 48
    /// Reading persisted wallet state failed on a store that reported the
    /// failure as retryable (`SQLITE_BUSY` and friends). Nothing was
    /// mutated — a load is a read. Retry later.
    case errorPersisterLoadTransient = 49
    /// Reading persisted wallet state failed permanently — a corrupt or
    /// unreadable store, or a decode that will fail identically next time.
    /// Do NOT retry; inspect the message. Constraint-class failures fold in
    /// here too: a read cannot violate one, and neither is retryable.
    case errorPersisterLoadFatal = 50
    /// Writing wallet state failed on a busy or momentarily unavailable
    /// store. **Nothing was committed** — the SDK only reports this when the
    /// persister rolls a failed changeset round back whole, so re-issuing the
    /// operation cannot double-apply part of it. Retry later.
    case errorPersisterStoreTransient = 51
    /// Writing wallet state failed permanently — a full disk, a corrupt
    /// schema, an I/O error outside the retryable class. Do NOT retry;
    /// inspect the message. The wallet rolled its in-memory state back, so
    /// the operation may be re-attempted once the fault is fixed.
    case errorPersisterStoreFatal = 52
    /// A write violated a constraint / foreign key / integrity rule.
    /// Deliberately distinct from `errorPersisterStoreFatal`: this is "the
    /// data is wrong" (a caller or schema-mapping bug) rather than "the
    /// storage engine is unhappy" (an operator problem), and the two route
    /// to different people. Do NOT retry unchanged; fix the data.
    case errorPersisterStoreConstraint = 53
    /// Rehydrating persisted platform-address state into a freshly
    /// registered wallet failed. One code rather than three: it wraps a
    /// wallet error, not a store error, so it carries no retry
    /// classification. The wrapped error's rendering is in the message.
    case errorPersisterRestore = 54
    /// An earlier identity-funded shield is unresolved. This request was not
    /// built or broadcast; wait for shielded sync before starting another.
    case errorShieldedIdentityDebitPending = 55
    /// A durable recovery record is malformed or invalid; retain it for diagnosis.
    case errorShieldedRecoveryCorrupted = 56
    /// Recovery needs its account and compatible keys; damaged ciphertext can look the same.
    case errorShieldedRecoveryKeysRequired = 57
    /// Platform returned no balance. Retrying the read is safe; ownership is unchanged.
    case errorIdentityBalanceUnavailable = 58
    /// SPV startup or teardown panicked. Not retryable: restart the app.
    case errorSpvProcessRestartRequired = 59
    /// An asset lock (a drain, or a fixed amount in many small coins) exceeds
    /// the input cap. A fresh build submitted nothing; a resumed lock over the
    /// cap may have been sent by an earlier attempt, so read the error's
    /// `payment` rather than this code.
    case errorAssetLockTooManyInputs = 60
    /// A shielded fund-from-asset-lock resume named a different Orchard
    /// recipient than the one its tracked lock recorded. This call submitted
    /// nothing new, but the lock is held (`.lockHeld`): resume it with the
    /// recorded recipient.
    case errorAssetLockRecipientMismatch = 61
    /// A key of a DashPay invitation claim already belongs to another
    /// identity; the message is a JSON detail naming it.
    case errorInvitationKeysAlreadyRegistered = 63
    /// The DashPay invitation's identity is one this wallet already records;
    /// the message is a JSON detail naming it. Nothing was recorded.
    case errorInvitationClaimedByThisWallet = 64
    /// The invitation claim could not verify Platform's answers yet; nothing
    /// was broadcast. Retry once synced.
    case errorInvitationNotReady = 65
    /// The asset lock is not usable yet (not propagated, or not chain-locked
    /// for a ChainLock proof). Read the error's `payment`: `.notSent` means
    /// retry later; `.maybeSent` means the claim may have been sent, so
    /// check the invitation's identity before claiming again.
    case errorAssetLockNotChainLocked = 66
    /// The contact requests with that identity came or went privately, so a
    /// public request to it, in reply or as a rotation, would publish the
    /// relationship. Nothing was sent; accept or send it privately.
    case errorPrivateContactRequestNeedsPrivateReply = 68
    /// The operation needs the shielded pool, and the native library was
    /// built without shielded support. Nothing was sent; not retryable on
    /// this build.
    case errorShieldedBuildRequired = 69
    /// The network is proven at a protocol version below the one that
    /// serves the request (for example the private contact request store
    /// before protocol version 15), so the SDK did not send it. Expected on
    /// such a network; it lifts when the network upgrades. An SDK pinned
    /// below the version is `errorSdkPinnedBelowProtocolVersion` (75).
    case errorProtocolVersionNotActive = 70
    /// A private contact request to that identity is pending: in flight, or
    /// paid for and not yet read back from the store, or a private payment
    /// the shielded store's journal keeps unresolved holds the pair. Nothing
    /// new was paid. Do not submit it again: the entry read back by a private
    /// sync, or the payment proven never to execute, frees it. An unfinished
    /// private store read is `errorPrivateContactStoreNotRead` (74), not this.
    case errorPrivateContactRequestPending = 71
    /// The wallet, not one contact, is held: its private payments are not
    /// restored from the shielded store's journal, so no contact request,
    /// public or private, is sent or accepted. The wallet was published
    /// with no shielded store configured (configure it when the manager is
    /// activated, before loading or creating wallets), it bound a shielded
    /// account in an earlier process, its last restoration failed or waits
    /// for a busy store, or the identity was loaded marking a private payment
    /// that may be unresolved. Configuring the shielded store or binding the
    /// wallet's shielded account restores them; a private sync does not.
    case errorPrivateContactPaymentsNotRestored = 72
    /// The persister cannot keep a private payment's recovery state across
    /// a restart (it does not declare the private contact request scan
    /// state with atomic changesets), so a private send or reply is refused
    /// before anything is sealed or paid. A configuration error, not
    /// retryable as it is.
    case errorPrivateContactRecoveryUnsupported = 73
    /// DashPay's private contact request store could not be read to its end
    /// for the identity before a contact request or acceptance, public or
    /// private, was decided. Nothing was reserved, sent or paid; calling
    /// again continues the read. Not a pending request.
    case errorPrivateContactStoreNotRead = 74
    /// The SDK is pinned below the protocol version the request needs, so it
    /// did not send it, whatever the network runs. A network upgrade does not
    /// lift it; only raising or removing the pin does.
    case errorSdkPinnedBelowProtocolVersion = 75
    /// A public contact request to that identity was broadcast and may have
    /// been delivered (its own send or accept reported
    /// `errorTransactionBroadcastUnconfirmed`, 20), and no proof settled its
    /// outcome yet. Nothing new was signed. Unconfirmed, not failed: do not
    /// submit it again; a public contact request sync settles it.
    case errorPublicContactRequestUnconfirmed = 76
    /// A shielded spend run against a plan (`shieldedPlanSpend`) selected
    /// different notes or a different fee than the plan, as the wallet's
    /// notes changed since, or is priced at another protocol version than
    /// the plan. Nothing is built or broadcast and no note stays reserved.
    case errorShieldedPlanChanged = 77
    /// The wallet's notes cover a shielded spend, or its plan, only with more
    /// notes than one shielded transition spends. Nothing was reserved, built
    /// or broadcast: a smaller amount fits, or the notes can be merged first.
    case errorShieldedSpendTooManyNotes = 78
    /// A shielded withdrawal, or its plan, reserves an amount for Core below
    /// the protocol minimum for its Core fee rate or above the maximum.
    /// Nothing was reserved, built or broadcast: another amount fits.
    case errorShieldedWithdrawalAmountOutOfRange = 79
    /// The network runs a protocol version this SDK has no tables for, so a
    /// shielded transfer, unshield, withdrawal or identity top-up from the
    /// pool, or its plan, cannot be priced. Nothing is built or broadcast and
    /// no note stays reserved. A retry does not help: ask the user to update
    /// the app.
    case errorNetworkProtocolVersionUnsupported = 81
    /// Platform could not be asked for an identity the operation names (the
    /// lookup failed, which is not Platform's answer that it does not exist,
    /// 7). Nothing was signed, reserved or broadcast: retry as it is.
    case errorIdentityLookupFailed = 80
    /// The balance history could not read a page of Platform's address
    /// balance changes at a height consistent with the pages it had already
    /// read, within Rust's per-page retry bound: every attempt got a
    /// successfully fetched, proof-verified reply below that height. No
    /// history was returned; the read can be made again later. SDK errors
    /// (stale-node rejection, transport or proof failures) are not this code.
    case errorBalanceHistoryInconsistentHeights = 83
    /// A shielded Clear was refused because a shielded operation (a bind, an
    /// account being added, a token pool transfer or unshield, a DashPay
    /// profile write, a tip address preparation or a shielded tip being
    /// sent, or another Clear) is in flight. Clear does not wait for it:
    /// nothing was changed and sync keeps
    /// running, so the host keeps its rows. Retry once it has finished.
    case errorShieldedOperationInProgress = 84
    /// A shielded tip was refused before anything was selected, built or
    /// sent: another tip from this wallet is being sent, or an earlier
    /// shielded payment to the same recipient may still execute. This call
    /// sent nothing. A shielded sync resolves the earlier payment as executed
    /// or as unable to execute, so the refusal ending is not a reason to send
    /// again: reconcile the earlier payment first, and send again only once
    /// it is known to have failed. Not `errorShieldedSpendUnconfirmed` (18).
    case errorShieldedTipPending = 85
    /// A shielded tip started from a saved contact was refused because the
    /// username now belongs to another identity than the contact's. Nothing
    /// was sent; the user picks the recipient again.
    case errorShieldedTipRecipientMismatch = 86
    /// A local shielded balance read did not finish within its wait budget
    /// (the wallet's state or its shielded store was held, by a scan for
    /// instance), or a shielded Clear was running. Nothing failed and
    /// nothing changed: keep the last state shown and read again later. The
    /// only code of that read that means retry.
    case errorShieldedBalanceBusy = 87
    /// A shielded spend selected a note no checkpoint of the wallet's local
    /// commitment tree covers yet. Refused while the spend was assembled,
    /// before any proof or broadcast, and the notes were released: nothing was
    /// sent. Retryable after the next shielded sync, which checkpoints the
    /// tree. A store that cannot produce the path is not this case: it stays
    /// `errorWalletOperation`. Distinct from
    /// `errorShieldedSpendUnconfirmed`, which must NOT be retried.
    case errorShieldedMerkleWitnessUnavailable = 88
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
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_PERSISTER_LOAD_TRANSIENT:
            self = .errorPersisterLoadTransient
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_PERSISTER_LOAD_FATAL:
            self = .errorPersisterLoadFatal
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_PERSISTER_STORE_TRANSIENT:
            self = .errorPersisterStoreTransient
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_PERSISTER_STORE_FATAL:
            self = .errorPersisterStoreFatal
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_PERSISTER_STORE_CONSTRAINT:
            self = .errorPersisterStoreConstraint
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_PERSISTER_RESTORE:
            self = .errorPersisterRestore
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHIELDED_IDENTITY_DEBIT_PENDING:
            self = .errorShieldedIdentityDebitPending
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHIELDED_RECOVERY_CORRUPTED:
            self = .errorShieldedRecoveryCorrupted
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHIELDED_RECOVERY_KEYS_REQUIRED:
            self = .errorShieldedRecoveryKeysRequired
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_IDENTITY_BALANCE_UNAVAILABLE:
            self = .errorIdentityBalanceUnavailable
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SPV_PROCESS_RESTART_REQUIRED:
            self = .errorSpvProcessRestartRequired
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_ASSET_LOCK_TOO_MANY_INPUTS:
            self = .errorAssetLockTooManyInputs
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_ASSET_LOCK_RECIPIENT_MISMATCH:
            self = .errorAssetLockRecipientMismatch
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_INVITATION_KEYS_ALREADY_REGISTERED:
            self = .errorInvitationKeysAlreadyRegistered
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_INVITATION_CLAIMED_BY_THIS_WALLET:
            self = .errorInvitationClaimedByThisWallet
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_INVITATION_NOT_READY:
            self = .errorInvitationNotReady
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_ASSET_LOCK_NOT_CHAIN_LOCKED:
            self = .errorAssetLockNotChainLocked
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_PRIVATE_CONTACT_REQUEST_NEEDS_PRIVATE_REPLY:
            self = .errorPrivateContactRequestNeedsPrivateReply
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHIELDED_BUILD_REQUIRED:
            self = .errorShieldedBuildRequired
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_PROTOCOL_VERSION_NOT_ACTIVE:
            self = .errorProtocolVersionNotActive
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_PRIVATE_CONTACT_REQUEST_PENDING:
            self = .errorPrivateContactRequestPending
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_PRIVATE_CONTACT_PAYMENTS_NOT_RESTORED:
            self = .errorPrivateContactPaymentsNotRestored
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_PRIVATE_CONTACT_RECOVERY_UNSUPPORTED:
            self = .errorPrivateContactRecoveryUnsupported
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_PRIVATE_CONTACT_STORE_NOT_READ:
            self = .errorPrivateContactStoreNotRead
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SDK_PINNED_BELOW_PROTOCOL_VERSION:
            self = .errorSdkPinnedBelowProtocolVersion
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_PUBLIC_CONTACT_REQUEST_UNCONFIRMED:
            self = .errorPublicContactRequestUnconfirmed
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHIELDED_PLAN_CHANGED:
            self = .errorShieldedPlanChanged
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHIELDED_SPEND_TOO_MANY_NOTES:
            self = .errorShieldedSpendTooManyNotes
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHIELDED_WITHDRAWAL_AMOUNT_OUT_OF_RANGE:
            self = .errorShieldedWithdrawalAmountOutOfRange
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_NETWORK_PROTOCOL_VERSION_UNSUPPORTED:
            self = .errorNetworkProtocolVersionUnsupported
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_IDENTITY_LOOKUP_FAILED:
            self = .errorIdentityLookupFailed
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_BALANCE_HISTORY_INCONSISTENT_HEIGHTS:
            self = .errorBalanceHistoryInconsistentHeights
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHIELDED_OPERATION_IN_PROGRESS:
            self = .errorShieldedOperationInProgress
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHIELDED_TIP_PENDING:
            self = .errorShieldedTipPending
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHIELDED_TIP_RECIPIENT_MISMATCH:
            self = .errorShieldedTipRecipientMismatch
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHIELDED_BALANCE_BUSY:
            self = .errorShieldedBalanceBusy
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHIELDED_MERKLE_WITNESS_UNAVAILABLE:
            self = .errorShieldedMerkleWitnessUnavailable
        case PLATFORM_WALLET_FFI_RESULT_CODE_NOT_FOUND:
            self = .notFound
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_UNKNOWN:
            self = .errorUnknown
        default:
            self = .errorUnknown
        }
    }
}

// MARK: - Consensus rejection

/// Platform's own verdict on a state transition it refused.
///
/// Drive answers a rejected transition with an rs-dpp consensus error: its
/// numeric `code` names the exact rule that refused it (40722 is a second
/// once-per-identity token claim), and its `kind` says which family of rule
/// that was. Rust reads both off the consensus error and puts them on the FFI
/// result (`PlatformWalletFFIResult` from the wallet FFI, `DashSDKError` from
/// rs-sdk-ffi), so a host branches on the code rather than recognising the
/// rejection in its rendered text.
public struct PlatformConsensusError: Equatable, Sendable {
    /// The family a consensus rejection belongs to.
    ///
    /// rs-dpp groups its codes by family (basic 1xxxx, signature 2xxxx, fee
    /// 3xxxx, state 4xxxx) and hands the grouping over as a value of its own,
    /// so this side never derives it from the digits of `code`. Mirror of
    /// `PlatformWalletFFIConsensusErrorKind` and rs-sdk-ffi's
    /// `DashSDKConsensusErrorKind`, which name the same families.
    public enum Kind: Equatable, Sendable {
        case basic
        case signature
        case fee
        case state

        /// `nil` for the FFI's `None` value, which pairs with
        /// `consensus_code == 0`, and for any family a newer Rust side adds
        /// before this mirror learns it.
        init?(ffi: PlatformWalletFFIConsensusErrorKind) {
            switch ffi {
            case PLATFORM_WALLET_FFI_CONSENSUS_ERROR_KIND_BASIC:     self = .basic
            case PLATFORM_WALLET_FFI_CONSENSUS_ERROR_KIND_SIGNATURE: self = .signature
            case PLATFORM_WALLET_FFI_CONSENSUS_ERROR_KIND_FEE:       self = .fee
            case PLATFORM_WALLET_FFI_CONSENSUS_ERROR_KIND_STATE:     self = .state
            default: return nil
            }
        }

        /// The same decoding for rs-sdk-ffi's kind: `nil` for
        /// `ConsensusErrorKindNone` and for any family this mirror does not
        /// know yet.
        init?(ffi: DashSDKConsensusErrorKind) {
            switch ffi {
            case ConsensusErrorKindBasic:     self = .basic
            case ConsensusErrorKindSignature: self = .signature
            case ConsensusErrorKindFee:       self = .fee
            case ConsensusErrorKindState:     self = .state
            default: return nil
            }
        }
    }

    /// The rs-dpp consensus error code
    /// (`packages/rs-dpp/src/errors/consensus/codes.rs`). Always 10000 or
    /// above: 0 is the FFI's "no rejection" sentinel and never reaches here.
    public let code: UInt32

    /// Which family of consensus rule refused the transition.
    public let kind: Kind

    public init(code: UInt32, kind: Kind) {
        self.code = code
        self.kind = kind
    }

    /// The rejection an FFI result carries, or `nil` when it carries none.
    init?(ffi: PlatformWalletFFIResult) {
        guard ffi.consensus_code != 0, let kind = Kind(ffi: ffi.consensus_kind) else {
            return nil
        }
        self.init(code: ffi.consensus_code, kind: kind)
    }

    /// The rejection an rs-sdk-ffi error carries, or `nil` when it carries
    /// none.
    init?(ffi: DashSDKError) {
        guard ffi.consensus_code != 0, let kind = Kind(ffi: ffi.consensus_kind) else {
            return nil
        }
        self.init(code: ffi.consensus_code, kind: kind)
    }
}

// MARK: - Payment delivery

/// Whether a failed payment may have left the wallet, as the Rust wallet
/// classified it.
///
/// Decide "retry", "wait" or "resume" from this, never from lists of error
/// cases (`PlatformPaymentRecovery` does it): `.notSent` offers a retry only
/// where `isRetryable` allows it, `.maybeSent` waits for sync and never
/// offers a second payment, `.lockHeld` resumes the named asset lock and never
/// pays again. Mirror of `PlatformWalletFFIPaymentDelivery`, with the held
/// lock's outpoint from the result's `held_asset_lock`.
public enum PlatformPaymentDelivery: Equatable, Sendable {
    /// The payment never left the wallet and can never execute.
    case notSent
    /// The payment may have left the wallet, and may execute or already has.
    case maybeSent
    /// The payment holds the tracked asset lock at this outpoint: it may be on
    /// the network and its funds left the spendable balance. Resume it (the
    /// outpoint is the one the resume calls take); never fund a new one.
    case lockHeld(AssetLockBuiltOutPoint)

    /// `nil` for the FFI's `None`, which only a success carries, and for its
    /// `Unknown`, a failure no wallet error classified. A value this mirror
    /// does not know yet reads as `.maybeSent`, the class that never invites a
    /// second payment. `heldAssetLock` is read only under `LockHeld`.
    init?(ffi: PlatformWalletFFIPaymentDelivery, heldAssetLock: OutPointFFI) {
        switch ffi {
        case PLATFORM_WALLET_FFI_PAYMENT_DELIVERY_NONE,
             PLATFORM_WALLET_FFI_PAYMENT_DELIVERY_UNKNOWN:  return nil
        case PLATFORM_WALLET_FFI_PAYMENT_DELIVERY_NOT_SENT: self = .notSent
        case PLATFORM_WALLET_FFI_PAYMENT_DELIVERY_LOCK_HELD:
            var raw = withUnsafeBytes(of: heldAssetLock.txid) { Data($0) }
            withUnsafeBytes(of: heldAssetLock.vout.littleEndian) { raw.append(contentsOf: $0) }
            self = .lockHeld(AssetLockBuiltOutPoint(reportedBytes: raw))
        default:                                            self = .maybeSent
        }
    }
}

/// What Rust decided about a failed payment, carried by every
/// `PlatformWalletError` case built from a result. Read from the result,
/// never derived from its code.
public struct PlatformPaymentClass: Equatable, Sendable {
    /// Whether the failed payment may have left the wallet.
    public let delivery: PlatformPaymentDelivery
    /// Whether the wallet allows running the failed operation again: never
    /// where the payment may have left the wallet, and never under
    /// `.lockHeld`, whose payment goes on only by resuming that lock (running
    /// the operation again would fund a second payment). Rust decides it
    /// (the result's `retryable`); Swift only reads it.
    public let isRetryable: Bool

    public init(delivery: PlatformPaymentDelivery, isRetryable: Bool) {
        self.delivery = delivery
        self.isRetryable = isRetryable
    }

    /// The class a Swift-side guard passes when it throws without a result
    /// after the payment may have left, so no second one is offered: only
    /// after Rust returned success but handed back no output (a NULL
    /// pointer or handle), when the operation's effect already happened.
    /// It grants nothing (no retry, no resume). A refusal Swift raises
    /// before Rust is asked carries no class at all (`payment: nil`): the
    /// class and retry decision are Rust's.
    public static let maybeSent = PlatformPaymentClass(delivery: .maybeSent, isRetryable: false)
}

/// What a host may offer a person after a failed payment, read from Rust's
/// class alone (`delivery`, `isRetryable` and the held lock), never from the
/// error's case, code or text. Build it with `init(_:)` and show
/// `suggestion`; offer a retry only under `.retry`, and a resume only under
/// `.resume` with its outpoint.
public enum PlatformPaymentRecovery: Equatable, Sendable {
    /// Nothing left the wallet and Rust allows running the operation again.
    case retry
    /// Nothing left the wallet, and Rust does not allow running it again as
    /// it is. The reason is the error's own (its message); offer no retry.
    case wait
    /// The payment holds the tracked asset lock at this outpoint: resume it,
    /// never fund a new one.
    case resume(AssetLockBuiltOutPoint)
    /// The payment may have left the wallet: let sync settle it and never
    /// offer a second one.
    case reconcile
    /// No wallet error classified the failure: unconfirmed, never retried.
    case unconfirmed

    public init(_ payment: PlatformPaymentClass?) {
        guard let payment else {
            self = .unconfirmed
            return
        }
        switch payment.delivery {
        case .notSent: self = payment.isRetryable ? .retry : .wait
        case .maybeSent: self = .reconcile
        case .lockHeld(let outPoint): self = .resume(outPoint)
        }
    }

    /// Text fit to show a person: what happened to the payment and the one
    /// action the class allows.
    public var suggestion: String {
        switch self {
        case .retry:
            return "Nothing was sent. You can try again."
        case .wait:
            return "Nothing was sent. This request cannot go through as it is."
        case .resume:
            return "The funds are held in an asset lock. Resume that lock to finish; do not start a new payment."
        case .reconcile:
            return "The payment may have gone through. Wait for the wallet to sync before doing anything else; "
                + "do not send it again."
        case .unconfirmed:
            return "Submission result is unconfirmed. Check history and reconcile the original operation "
                + "before submitting again."
        }
    }

    /// `suggestion` for a failed invitation create, whose held lock is the
    /// invitation's voucher: the generic resume refuses a voucher lock, and
    /// its funds come back through the voucher's reclaim. The host knows it
    /// from the operation that failed; the class is the same.
    public var invitationVoucherSuggestion: String {
        switch self {
        case .resume:
            return "The funds are held in the invitation's voucher lock. Reclaim the voucher from Sent "
                + "invitations to get them back; do not create a new invitation."
        case .retry, .wait, .reconcile, .unconfirmed:
            return suggestion
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

    /// Platform's verdict when the failure was a consensus rejection, read
    /// from the result's own fields rather than from `message`.
    var consensusError: PlatformConsensusError? {
        PlatformConsensusError(ffi: inner)
    }

    /// Whether the failed payment may have left the wallet: the class of the
    /// wallet error behind the failure, as precise as Rust has it. `nil` on
    /// success.
    var paymentDelivery: PlatformPaymentDelivery? {
        PlatformPaymentDelivery(ffi: inner.payment_delivery, heldAssetLock: inner.held_asset_lock)
    }

    /// Rust's class of the failure, `nil` on success and for a failure no
    /// wallet error classified.
    var paymentClass: PlatformPaymentClass? {
        paymentDelivery.map { PlatformPaymentClass(delivery: $0, isRetryable: inner.retryable) }
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
///
/// Every case carries a trailing `payment`: the class of the failure as Rust
/// gave it on the result the error was built from (see `payment`), `nil`
/// when no wallet error classified it or Swift threw the case itself without
/// a result.
public enum PlatformWalletError: LocalizedError {
    case nullPointer(String, payment: PlatformPaymentClass? = nil)
    case invalidHandle(String, payment: PlatformPaymentClass? = nil)
    case invalidParameter(String, payment: PlatformPaymentClass? = nil)
    case invalidIdentifier(String, payment: PlatformPaymentClass? = nil)
    case invalidNetwork(String, payment: PlatformPaymentClass? = nil)
    /// The catch-all wallet-operation failure (code 6). It folds failures of
    /// every payment-delivery class, so it carries the one Rust gave this
    /// failure: `.notSent`, `.maybeSent` or `.lockHeld` (a proof wait that
    /// timed out names the lock to resume), `nil` when no wallet error
    /// classified it.
    case walletOperation(String, payment: PlatformPaymentClass? = nil)
    case identityNotFound(String, payment: PlatformPaymentClass? = nil)
    /// A managed identity has no balance in the Platform response; the read may be retried.
    case identityBalanceUnavailable(String, payment: PlatformPaymentClass? = nil)
    case contactNotFound(String, payment: PlatformPaymentClass? = nil)
    case utf8Conversion(String, payment: PlatformPaymentClass? = nil)
    case serialization(String, payment: PlatformPaymentClass? = nil)
    case deserialization(String, payment: PlatformPaymentClass? = nil)
    case memoryAllocation(String, payment: PlatformPaymentClass? = nil)
    case arithmeticOverflow(String, payment: PlatformPaymentClass? = nil)
    case noSelectableInputs(String, payment: PlatformPaymentClass? = nil)
    case coreInsufficientFunds(String, payment: PlatformPaymentClass? = nil)
    case assetLockNotTracked(String, payment: PlatformPaymentClass? = nil)
    /// The one-shot output cannot be reused. This may come from a retained
    /// local tombstone or an unauthenticated Platform report, so it does not
    /// prove that the requested operation completed.
    case assetLockAlreadyConsumed(String, payment: PlatformPaymentClass? = nil)
    case assetLockFundingMismatch(String, payment: PlatformPaymentClass? = nil)
    /// Asset-lock coin selection could not cover the requested funding over
    /// the permitted source set. Nothing was built or broadcast and no
    /// funding output was consumed — refresh the preflight, then recover by
    /// the funding form: an exact-amount build can confirm a smaller amount,
    /// while a whole-account drain takes no amount to lower and instead needs
    /// funds added to the drained account (or a lower minimum lock floor).
    /// The `available` / `required` duff figures are in the message. Kotlin
    /// parity: `DashSdkError.PlatformWallet.AssetLockInsufficientFunds`.
    case assetLockInsufficientFunds(String, payment: PlatformPaymentClass? = nil)
    /// An asset lock needs more inputs than one may carry: a drain of every
    /// eligible input, or a fixed amount held only in many small coins. The
    /// build broadcast nothing, released its reservation and drained nothing;
    /// consolidating the coins lets it fit. Also the outcome of resuming a
    /// tracked lock over the cap: it is not re-sent (an earlier attempt may
    /// have reached the network), it stays tracked, and nothing is released.
    case assetLockTooManyInputs(String, payment: PlatformPaymentClass? = nil)
    /// A shielded fund-from-asset-lock resume (generic outpoint resume or a
    /// repeated-UUID wallet drain) named a different Orchard recipient than
    /// the one the tracked lock recorded when it was built. Nothing was
    /// submitted or paid; resume with the recorded recipient
    /// (`PersistentAssetLock.shieldedRecipientRaw43` /
    /// `TrackedAssetLock.shieldedRecipientRaw43`). Locks tracked before the
    /// recipient was recorded carry none and accept the caller's.
    ///
    /// `payment` is Rust's class: `.lockHeld` with the tracked lock to resume
    /// with the recorded recipient.
    case assetLockRecipientMismatch(String, payment: PlatformPaymentClass? = nil)
    /// The asset lock cannot be used yet: its transaction has not reached the
    /// queried node, or it needs a ChainLock proof and is not chain-locked
    /// yet. Produced today by `claimInvitation`. `payment` says when:
    /// `.notSent` and retryable before the claim was built (retry once the
    /// lock propagates or is chain-locked), `.maybeSent` and not retryable
    /// when the claim submitted with its islock proof was refused (stale
    /// islock, input cap, or a size verdict that may be a node's): it may
    /// have been sent, so check the invitation's identity before claiming
    /// again.
    case assetLockNotChainLocked(String, payment: PlatformPaymentClass? = nil)
    /// A key of the DashPay invitation claim is already registered to
    /// another identity (base58 `identityId`, `nil` when Platform refused the
    /// broadcast without naming it), so the claim cannot be made with these
    /// keys. `consensus` is Platform's refusal (40207, or 10501 before
    /// protocol v11) when it came from the broadcast, `nil` when the claim's
    /// own lookup found the owner and nothing was sent.
    case invitationKeysAlreadyRegistered(
        identityId: String?,
        consensus: PlatformConsensusError?,
        payment: PlatformPaymentClass? = nil
    )
    /// The invitation's identity (base58 `identityId`) is this wallet's: the
    /// wallet already records it in one of its registration slots (no key
    /// the identity holds decides this). Nothing was recorded, and nothing was broadcast
    /// unless a node that lagged behind let the claim through; load the
    /// identity like any of the wallet's own.
    /// `payment` says which: `.notSent` (not retryable) when the claim's
    /// preflight found it, `.maybeSent` when Platform refused the claim.
    case invitationClaimedByThisWallet(identityId: String, payment: PlatformPaymentClass? = nil)
    /// The invitation claim could not verify Platform's answers yet (the
    /// quorum is not verified). Nothing was broadcast; retry once synced.
    case invitationNotReady(String, payment: PlatformPaymentClass? = nil)
    /// The contact requests with that identity came or went privately
    /// (protocol version 15): a public request to it, in reply or as a
    /// rotation, would publish the relationship, so the public accept and
    /// send refuse it and nothing was sent. Accept it with
    /// `PlatformWalletManager.acceptContactRequestReplyingInKind` (which picks
    /// the reply itself) or `acceptPrivateContactRequest`, or send with
    /// `PlatformWalletManager.sendPrivateContactRequest`.
    case privateContactRequestNeedsPrivateReply(String, payment: PlatformPaymentClass? = nil)
    /// The operation needs the shielded pool, and the native library was
    /// built without shielded support: today, accepting a contact request
    /// that came privately with
    /// `PlatformWalletManager.acceptContactRequestReplyingInKind`. Nothing
    /// was sent; a public or adopted reply works on such a build.
    case shieldedBuildRequired(String, payment: PlatformPaymentClass? = nil)
    /// The network does not serve the request yet: it is proven at a protocol
    /// version below the one that does, so the SDK did not send it. Expected
    /// on such a network (for example the private contact request store before
    /// protocol version 15); a host may stay quiet about it. An SDK pinned
    /// below the version is `sdkPinnedBelowProtocolVersion` instead.
    case protocolVersionNotActive(String, payment: PlatformPaymentClass? = nil)
    /// A private contact request to that identity is pending (code 71): in
    /// flight (another private send or reply reserved it), or paid for and
    /// not yet read back from the store, or a private payment the shielded
    /// store's journal keeps unresolved holds the pair. Nothing new was paid.
    /// Pending, not failed: do not submit it again. A
    /// `ManagedPlatformWallet.syncPrivateContactRequests` that reads the
    /// entry back frees it, as does the payment proven never to execute; an
    /// endpoint's refusal of the payment does not.
    /// `ManagedPlatformWallet.privatePairHeld(senderId:recipientId:)` reads
    /// whether the pair is still held.
    case privateContactRequestPending(String, payment: PlatformPaymentClass? = nil)
    /// The wallet's private payments are not restored from the shielded
    /// store's journal, so its contact requests, public or private, sent or
    /// accepted, are held (code 72). Nothing was signed or sent.
    /// `PlatformWalletManager.configureShielded` (called when the manager is
    /// activated, before loading or creating wallets, and again after a
    /// failure) or binding the wallet's shielded account restores them; a
    /// private sync does not.
    /// `ManagedPlatformWallet.privateContactReadiness(identityId:)` reads
    /// whether they are restored and whether the last restoration failed,
    /// and `privatePaymentsRestoreFailure(identityId:)` why.
    case privateContactPaymentsNotRestored(String, payment: PlatformPaymentClass? = nil)
    /// The wallet's storage cannot keep a private payment's recovery state
    /// across a restart (code 73): it does not declare the private contact
    /// request scan state together with atomic changesets. A private send
    /// or reply is refused before anything is sealed or paid. A
    /// configuration error of the store, not something a retry fixes.
    case privateContactRecoveryUnsupported(String, payment: PlatformPaymentClass? = nil)
    /// DashPay's private contact request store could not be read to its end
    /// for the identity before a contact request or acceptance, public or
    /// private, was decided (code 74): a node behind the store already seen,
    /// or keys that kept changing, stop the read. Nothing was reserved, sent
    /// or paid; calling again continues the read. Not a pending request.
    case privateContactStoreNotRead(String, payment: PlatformPaymentClass? = nil)
    /// The SDK is pinned below the protocol version the request needs, so it
    /// did not send it, whatever the network runs (code 75). Only raising or
    /// removing the SDK's protocol version pin lifts it: a developer's
    /// setting, not something the user or a network upgrade changes.
    case sdkPinnedBelowProtocolVersion(String, payment: PlatformPaymentClass? = nil)
    /// A public contact request to that identity was broadcast and may have
    /// been delivered, and no proof settled its outcome yet (code 76): the
    /// wallet holds the pair, keeping the signed transition, and refuses
    /// every contact request or acceptance to it, public or private, with
    /// nothing signed. The attempt that broadcast it reported
    /// `transactionBroadcastUnconfirmed` (20). Unconfirmed, not failed: do not
    /// submit it again. A public contact request sync (the DashPay sync)
    /// settles it: the request proved on Platform is recorded as sent, and one
    /// proved unable to write it frees the pair.
    /// `ManagedPlatformWallet.publicContactRequestUnconfirmed(senderId:recipientId:)`
    /// reads whether the pair is still held so. Also thrown by a public send
    /// or accept made while another public request to that identity is being
    /// sent: one is sent at a time, and nothing is signed for the second.
    case publicContactRequestUnconfirmed(String, payment: PlatformPaymentClass? = nil)
    /// A shielded spend run against a `ShieldedSpendPlan` selected different
    /// notes or a different fee than the plan, as the wallet's notes changed
    /// since, or is priced at another protocol version than the plan (code
    /// 77). Nothing is built or broadcast and no note stays reserved: plan
    /// again and confirm the new fee before spending.
    case shieldedPlanChanged(String, payment: PlatformPaymentClass? = nil)
    /// The wallet's notes cover a shielded spend, or its plan, only with more
    /// notes than one shielded transition spends (code 78). Nothing was
    /// reserved, built or broadcast: a smaller amount fits, or the notes can
    /// be merged first.
    case shieldedSpendTooManyNotes(String, payment: PlatformPaymentClass? = nil)
    /// A shielded withdrawal, or its plan, reserves an amount for Core below
    /// the protocol minimum for its Core fee rate or above the maximum (code
    /// 79). Nothing was reserved, built or broadcast: another amount fits.
    case shieldedWithdrawalAmountOutOfRange(String, payment: PlatformPaymentClass? = nil)
    /// The network runs a protocol version this SDK has no tables for (code
    /// 81), so a shielded transfer, unshield, withdrawal or identity top-up
    /// from the pool, or its plan, cannot be priced. Nothing is built or
    /// broadcast and no note stays reserved; only an app update lifts it.
    case networkProtocolVersionUnsupported(String, payment: PlatformPaymentClass? = nil)
    /// Platform could not be asked for an identity the operation names (code
    /// 80): the lookup itself failed, before anything was signed, reserved or
    /// broadcast. Not sent, so it may be retried as it is; unlike an unmapped
    /// SDK error it is never a possibly delivered operation.
    case identityLookupFailed(String, payment: PlatformPaymentClass? = nil)
    /// `ManagedPlatformAddressWallet.balanceHistory(includeCompacted:)` kept
    /// getting a page of Platform's address balance changes proved below a
    /// height it had already read, within Rust's per-page retry bound, so it
    /// returned no history rather than a partial one. Every attempt got a
    /// successfully fetched, proof-verified reply below that height; errors
    /// from the SDK itself (its own stale-node rejection, transport or proof
    /// failures) keep their existing mapping. Nothing changed; load the
    /// history again later.
    case balanceHistoryInconsistentHeights(String, payment: PlatformPaymentClass? = nil)
    /// `PlatformWalletManager.clearShielded()` was refused at once because a
    /// shielded operation (a bind, an account being added, a token pool
    /// transfer or unshield, a DashPay profile write or a tip address
    /// preparation, or another Clear) is in flight. Clear does not wait for
    /// it. Nothing was changed: sync keeps running and no shielded state was
    /// reset, so the host must not wipe its own rows. Clear again once the
    /// operation has finished.
    case shieldedOperationInProgress(String, payment: PlatformPaymentClass? = nil)
    /// `sendShieldedTip` refused the tip with nothing sent: another tip from
    /// this wallet is being sent, or an earlier shielded payment to the
    /// recipient may still execute. This call sent nothing, unlike
    /// `shieldedSpendUnconfirmed`. The earlier payment may be the same tip
    /// and may pay the recipient: a shielded sync resolves it as executed or
    /// as unable to execute, so do not send again because the refusal
    /// ended, only once that payment is known to have failed.
    case shieldedTipPending(String, payment: PlatformPaymentClass? = nil)
    /// A shielded tip started from a saved contact was refused because the
    /// username now belongs to another identity than the contact's: the name
    /// moved since the contact was saved. Nothing was sent. The user picks
    /// the recipient again; a tip to the name's new owner starts from the
    /// name, not from the contact.
    case shieldedTipRecipientMismatch(String, payment: PlatformPaymentClass? = nil)
    /// `shieldedIdentityBalance` did not finish within its wait budget (the
    /// wallet's state or its shielded store was held, by a scan for
    /// instance), or a shielded Clear was running. Nothing failed and
    /// nothing changed: keep the last state shown and read again later. The
    /// only error of that read that means retry.
    case shieldedBalanceBusy(String, payment: PlatformPaymentClass? = nil)
    case walletAlreadyExists(String, payment: PlatformPaymentClass? = nil)
    /// Definitive shielded-broadcast failure: the shielded transition never
    /// left the wallet (refused before anything was sent), so it cannot
    /// execute; any reserved notes were released. Retry only where
    /// `isRetryable` (Rust's) says so.
    case shieldedBroadcastFailed(String, payment: PlatformPaymentClass? = nil)
    /// An identity create from the pool may have reached a node (a node's
    /// refusal or verdict included), its result is not proved and a direct
    /// fetch of the derived id came back empty. The wallet keeps the notes
    /// reserved and the saved transition; the identity may already exist on
    /// chain, so do not treat it as unregistered or re-submit. Callers that
    /// need the derived id should special-case the
    /// `.errorShieldedBroadcastUnconfirmed` result code (which carries
    /// `outIdentityId`) before falling back to this error: see
    /// `ShieldedIdentityCreateUnconfirmedError`.
    case shieldedBroadcastUnconfirmed(String, payment: PlatformPaymentClass? = nil)
    /// A shielded operation (shield from Platform addresses, unshield /
    /// transfer / withdrawal) may have reached a node and its result is not
    /// proved (a node's refusal or consensus verdict included). It may have
    /// executed; spends keep their notes reserved, a shield stays pending
    /// until the scan sees its note. Do NOT auto-retry.
    case shieldedSpendUnconfirmed(String, payment: PlatformPaymentClass? = nil)
    /// A shielded spend could not be built against a Platform-recorded anchor —
    /// the wallet's commitment tree isn't synced to a checkpoint Platform has
    /// recorded (an in-progress / interrupted sync leaves it mid-block). Nothing
    /// was broadcast and the notes were released; retryable once the shielded
    /// sync reaches a confirmed state. Distinct from `shieldedSpendUnconfirmed`,
    /// which must NOT be retried.
    case shieldedNoRecordedAnchor(String, payment: PlatformPaymentClass? = nil)
    /// A shielded spend selected a note no checkpoint of the local commitment
    /// tree covers yet (code 88). Nothing was proved or broadcast and the notes
    /// were released: Rust classes it `.notSent` and retryable once the next
    /// shielded sync checkpoints the tree. Distinct from
    /// `shieldedSpendUnconfirmed`, which must NOT be retried.
    case shieldedMerkleWitnessUnavailable(String, payment: PlatformPaymentClass? = nil)
    /// The cached Platform Payment-account input set cannot cover a shield's
    /// requested amount plus the fee reserve retained on input 0. Despite the
    /// retained public Swift/FFI name, this is not a shielded-pool balance
    /// failure.
    case shieldedInsufficientBalance(String, payment: PlatformPaymentClass? = nil)
    /// A Core transaction or Platform state transition may have executed,
    /// but its outcome is unknown. Sync and reconcile the original operation
    /// before another submission. Core transactions retain their input
    /// protections; Platform state transitions have no Core UTXO reservation.
    /// Do NOT auto-retry. Sibling of `shieldedSpendUnconfirmed`.
    case transactionBroadcastUnconfirmed(String, payment: PlatformPaymentClass? = nil)
    /// A masternode (evonode) credit withdrawal may have reached a node and
    /// its result is not proved (a node's refusal or verdict included). It may
    /// have executed and a retry signs a second withdrawal: do NOT retry;
    /// re-read the claimable balance first (`.errorMasternodeWithdrawalUnconfirmed`).
    case masternodeWithdrawalUnconfirmed(String, payment: PlatformPaymentClass? = nil)
    /// The masternode list hasn't synced yet, so there is nothing to look a
    /// masternode up in (`.errorMasternodeListUnavailable`). Retry after
    /// masternode sync completes.
    case masternodeListUnavailable(String, payment: PlatformPaymentClass? = nil)
    /// Core definitively rejected the transaction and its input reservation
    /// was released. Retry only where `isRetryable` (Rust's) says so.
    case transactionBroadcastRejected(String, payment: PlatformPaymentClass? = nil)
    /// Address-nonce race (transfer or withdrawal from Platform addresses, or
    /// identity registration from them): Platform rejected the transition
    /// because the submitted address nonce raced its expected value. Not a
    /// retry signal: the verdict may follow an earlier dispatch that landed,
    /// so Rust classes it `.maybeSent` and not retryable; reconcile the
    /// original operation before submitting another transition. Shields and
    /// address-funded identity top-ups do not return it (a top-up returns
    /// `transactionBroadcastUnconfirmed`). The submitted and
    /// expected nonce values are in the message.
    case addressNonceMismatch(String, payment: PlatformPaymentClass? = nil)
    /// A quiesce/drain barrier (Clear / reset / sync-stop) timed out with a
    /// sync pass still in flight. The operation failed closed — retry once
    /// sync is idle. SPV start, stop and storage clear also throw it while an
    /// SPV teardown is still running: call `stopSpv()` again.
    case shutdownIncomplete(String, payment: PlatformPaymentClass? = nil)
    /// SPV startup or teardown panicked. Not retryable: restart the app.
    /// `errorDescription` is fixed user text; the panic detail is on
    /// `failureReason`.
    case spvProcessRestartRequired(String, payment: PlatformPaymentClass? = nil)
    /// The signer has no usable private key for the requested public key
    /// (missing / stranded scalar) — the operation itself did not fail.
    /// Restored from the structured signer completion code
    /// (dashpay/platform#4060 finding 7); route to key repair. Kotlin
    /// parity: `DashSdkError.PlatformWallet.SigningKeyUnavailable`.
    ///
    /// `signMessage` also raises it for an address the wallet does not own.
    /// Distinct from `invalidParameter`, which means the address itself is
    /// unusable (unparseable, wrong network, or not P2PKH).
    case signingKeyUnavailable(String, payment: PlatformPaymentClass? = nil)
    /// A deferred (BIP70/BIP270) reservation token has outlived its funding
    /// reservation's lifetime — key-wallet's TTL may already have swept and
    /// re-selected the inputs. Nothing was broadcast. NOT retryable in place;
    /// rebuild the payment. Sibling of `reservationTokenConsumed` and
    /// `reservationWalletMismatch`, which this code used to conflate.
    case staleReservationToken(String, payment: PlatformPaymentClass? = nil)
    /// A deferred reservation token is unknown, already broadcast, or already
    /// released — the double-broadcast guard. Nothing was broadcast. NOT
    /// retryable; rebuild the payment.
    case reservationTokenConsumed(String, payment: PlatformPaymentClass? = nil)
    /// A deferred reservation token was minted against a different wallet
    /// generation than the one broadcasting it (e.g. a wallet re-created under
    /// the same id). Nothing was broadcast and the rightful owner's token was
    /// not consumed. NOT retryable through this handle; rebuild the payment.
    case reservationWalletMismatch(String, payment: PlatformPaymentClass? = nil)
    /// The DPNS name / document is not listed for sale, so it cannot be
    /// purchased and has no price to clear. Raised by the wallet's
    /// pre-flight read and by the consensus not-for-sale rejection; whether
    /// anything executed is `payment`'s to say.
    case notForSale(String, payment: PlatformPaymentClass? = nil)
    /// The listing no longer matches the price the user confirmed. Rebuilt
    /// from the FFI's JSON detail, so the UI can show both numbers and
    /// re-prompt at `actual`. Whether anything was purchased is `payment`'s
    /// to say: the verdict may follow an earlier dispatch, so Rust classes
    /// it `.maybeSent`.
    case priceChanged(documentId: String, expected: UInt64, actual: UInt64, payment: PlatformPaymentClass? = nil)
    /// The identity's credit balance cannot cover the operation. Both
    /// amounts are in credits (1 duff = 1000 credits) and include the fee
    /// reserve the wallet requires on top of a purchase price. Whether
    /// anything executed is `payment`'s to say.
    case insufficientIdentityCredits(
        identityId: String, required: UInt64, available: UInt64, payment: PlatformPaymentClass? = nil
    )
    /// The DPNS name is inside an active contested-name vote and cannot be
    /// listed, transferred, or purchased until the contest resolves.
    /// `endsAtMs == 0` means the vote's end time was unavailable — show it
    /// as unknown rather than as "ends at the epoch".
    case contestedNameNotTradable(label: String, endsAtMs: UInt64, payment: PlatformPaymentClass? = nil)
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
    case assetLockInputConflict(String, payment: PlatformPaymentClass? = nil)
    /// The tracked asset lock spends an outpoint a different,
    /// already-confirmed transaction of this wallet spent first, so no peer
    /// will relay it while that spender stands. The resume still attempts
    /// recovery. With a ready transport, this is what the bounded proof wait
    /// expired with. In the `Broadcast` arm, after a readiness miss and
    /// pre-dispatch rejection, a still-standing conflict returns immediately
    /// after refreshing local finality and leaves that wait to the
    /// readiness-deferred retry. A `Broadcast`-status lock may also represent
    /// an earlier attempt that sent it, so this is not a claim that nothing
    /// reached the network.
    ///
    /// The only double-spend verdict the SDK emits, and PROVISIONAL: the
    /// tracked lock must NOT be discarded on this error. A conflict that
    /// persists across sessions still does not prove finalized ancestry —
    /// the sighting can even be a block record restored from a previous
    /// session whose block was reorganized out while the wallet was offline.
    /// Keep the tracked lock and resume it later (it is not a retry of the
    /// operation: `isRetryable` is `false` under `.lockHeld`);
    /// only `assetLockInputConflict`, or an independent finalized-ancestry
    /// proof, may authorize discarding it. No funds move either way: the
    /// confirmed spender is this wallet's own transaction, so the value
    /// behind the contested input lives on in it.
    ///
    /// `payment` is Rust's class: `.lockHeld` with the contested lock's
    /// outpoint, the lock a resume takes.
    case assetLockInputContested(String, payment: PlatformPaymentClass? = nil)
    /// Reading persisted wallet state failed on a store that classified the
    /// failure as retryable. Nothing was mutated — retry later. One of the
    /// two retryable persister cases, alongside `persisterStoreTransient`.
    case persisterLoadTransient(String, payment: PlatformPaymentClass? = nil)
    /// Reading persisted wallet state failed permanently. Do NOT retry;
    /// the store needs repair or re-provisioning.
    case persisterLoadFatal(String, payment: PlatformPaymentClass? = nil)
    /// Writing wallet state failed on a busy store, with the whole changeset
    /// round rolled back — nothing was committed, so re-issuing the
    /// operation is safe. Retry later.
    case persisterStoreTransient(String, payment: PlatformPaymentClass? = nil)
    /// Writing wallet state failed permanently. Do NOT retry until the
    /// underlying fault is fixed; the wallet rolled its in-memory state back.
    case persisterStoreFatal(String, payment: PlatformPaymentClass? = nil)
    /// A write violated a constraint / integrity rule — the data is wrong,
    /// as opposed to the storage engine being unhappy. Do NOT retry
    /// unchanged.
    case persisterStoreConstraint(String, payment: PlatformPaymentClass? = nil)
    /// Rehydrating persisted platform-address state into a newly registered
    /// wallet failed. Carries no retry classification: it wraps a wallet
    /// error rather than a store error.
    case persisterRestore(String, payment: PlatformPaymentClass? = nil)
    /// An earlier identity-funded shield is unresolved. This request was not
    /// built or broadcast; wait for shielded sync before starting another.
    case shieldedIdentityDebitPending(String, payment: PlatformPaymentClass? = nil)
    /// A durable recovery record is malformed or invalid; retain it for diagnosis.
    case shieldedRecoveryCorrupted(String, payment: PlatformPaymentClass? = nil)
    /// Recovery needs its account and compatible keys; damaged ciphertext can look the same.
    case shieldedRecoveryKeysRequired(String, payment: PlatformPaymentClass? = nil)
    /// The named thing does not exist. For the deferred payment calls this is
    /// the wallet-was-REMOVED case: the token's wallet (or the wallet a payment
    /// was just signed against) is no longer registered in the manager, so there
    /// is no live generation to act through. Nothing was broadcast; the
    /// finalize path reconciles the build's reservation before returning. NOT
    /// retryable — unlike `reservationWalletMismatch`, no other generation holds
    /// this payment either.
    case notFound(String, payment: PlatformPaymentClass? = nil)
    /// Platform refused the state transition, and the wallet layer has no
    /// dedicated case for that particular refusal. Carries the rs-dpp
    /// consensus code and family, so a host can branch on the exact rule that
    /// refused it (`error.consensusError?.code == 40722` is a second
    /// once-per-identity token claim), plus the rendered message `.unknown`
    /// would have carried.
    ///
    /// Only the catch-all FFI code reaches here. A rejection the wallet layer
    /// promotes to a code of its own (a changed listing price, an
    /// address-nonce race) keeps its typed case, which already carries the
    /// values that case exists for.
    case consensusRejection(PlatformConsensusError, String, payment: PlatformPaymentClass? = nil)
    /// The catch-all failure (code 99) with no consensus verdict. Like
    /// `walletOperation`, it carries the payment-delivery class Rust gave
    /// this failure, `nil` when no wallet error classified it or it was
    /// built from a success result.
    case unknown(String, payment: PlatformPaymentClass? = nil)

    /// Platform's verdict when this error is a consensus rejection the wallet
    /// layer left untyped, so a caller can write
    /// `error.consensusError?.code == 40722` without pattern matching, and
    /// the refusal behind `.invitationKeysAlreadyRegistered` when Platform
    /// refused the broadcast. `nil` for every other case, including the
    /// other typed promotions of specific rejections.
    public var consensusError: PlatformConsensusError? {
        switch self {
        case .consensusRejection(let consensus, _, _),
             .invitationKeysAlreadyRegistered(_, let consensus?, _):
            return consensus
        default:
            return nil
        }
    }

    /// Rust's class of the failure, read from the result it was built from.
    /// Carried by every case, so a host reads the same class Rust gave the
    /// result whatever its code: a `.notSent` under a shared code may offer
    /// the retry Rust allowed. `nil` when no wallet error classified the
    /// failure, or Swift threw the case itself without a result. `nil` means
    /// unclassified: not proven safe to retry, so never retry automatically
    /// on it.
    public var payment: PlatformPaymentClass? {
        switch self {
        case .nullPointer(_, let payment),
             .invalidHandle(_, let payment),
             .invalidParameter(_, let payment),
             .invalidIdentifier(_, let payment),
             .invalidNetwork(_, let payment),
             .walletOperation(_, let payment),
             .identityNotFound(_, let payment),
             .identityBalanceUnavailable(_, let payment),
             .contactNotFound(_, let payment),
             .utf8Conversion(_, let payment),
             .serialization(_, let payment),
             .deserialization(_, let payment),
             .memoryAllocation(_, let payment),
             .arithmeticOverflow(_, let payment),
             .noSelectableInputs(_, let payment),
             .coreInsufficientFunds(_, let payment),
             .assetLockNotTracked(_, let payment),
             .assetLockAlreadyConsumed(_, let payment),
             .assetLockFundingMismatch(_, let payment),
             .assetLockInsufficientFunds(_, let payment),
             .assetLockTooManyInputs(_, let payment),
             .assetLockRecipientMismatch(_, let payment),
             .assetLockNotChainLocked(_, let payment),
             .invitationKeysAlreadyRegistered(_, _, let payment),
             .invitationClaimedByThisWallet(_, let payment),
             .invitationNotReady(_, let payment),
             .privateContactRequestNeedsPrivateReply(_, let payment),
             .shieldedBuildRequired(_, let payment),
             .protocolVersionNotActive(_, let payment),
             .privateContactRequestPending(_, let payment),
             .privateContactPaymentsNotRestored(_, let payment),
             .privateContactRecoveryUnsupported(_, let payment),
             .privateContactStoreNotRead(_, let payment),
             .sdkPinnedBelowProtocolVersion(_, let payment),
             .publicContactRequestUnconfirmed(_, let payment),
             .shieldedPlanChanged(_, let payment),
             .shieldedSpendTooManyNotes(_, let payment),
             .shieldedWithdrawalAmountOutOfRange(_, let payment),
             .networkProtocolVersionUnsupported(_, let payment),
             .identityLookupFailed(_, let payment),
             .balanceHistoryInconsistentHeights(_, let payment),
             .shieldedOperationInProgress(_, let payment),
             .shieldedTipPending(_, let payment),
             .shieldedTipRecipientMismatch(_, let payment),
             .shieldedBalanceBusy(_, let payment),
             .walletAlreadyExists(_, let payment),
             .shieldedBroadcastFailed(_, let payment),
             .shieldedBroadcastUnconfirmed(_, let payment),
             .shieldedSpendUnconfirmed(_, let payment),
             .shieldedNoRecordedAnchor(_, let payment), .shieldedMerkleWitnessUnavailable(_, let payment),
             .shieldedInsufficientBalance(_, let payment),
             .transactionBroadcastUnconfirmed(_, let payment),
             .masternodeWithdrawalUnconfirmed(_, let payment),
             .masternodeListUnavailable(_, let payment),
             .transactionBroadcastRejected(_, let payment),
             .addressNonceMismatch(_, let payment),
             .shutdownIncomplete(_, let payment),
             .spvProcessRestartRequired(_, let payment),
             .signingKeyUnavailable(_, let payment),
             .staleReservationToken(_, let payment),
             .reservationTokenConsumed(_, let payment),
             .reservationWalletMismatch(_, let payment),
             .notForSale(_, let payment),
             .priceChanged(_, _, _, let payment),
             .insufficientIdentityCredits(_, _, _, let payment),
             .contestedNameNotTradable(_, _, let payment),
             .assetLockInputConflict(_, let payment),
             .assetLockInputContested(_, let payment),
             .persisterLoadTransient(_, let payment),
             .persisterLoadFatal(_, let payment),
             .persisterStoreTransient(_, let payment),
             .persisterStoreFatal(_, let payment),
             .persisterStoreConstraint(_, let payment),
             .persisterRestore(_, let payment),
             .shieldedIdentityDebitPending(_, let payment),
             .shieldedRecoveryCorrupted(_, let payment),
             .shieldedRecoveryKeysRequired(_, let payment),
             .notFound(_, let payment),
             .consensusRejection(_, _, let payment),
             .unknown(_, let payment):
            return payment
        }
    }

    /// Whether the failed payment may have left the wallet, as Rust
    /// classified it (`.lockHeld` with its outpoint included); see
    /// `payment`. `nil` means unclassified: not proven safe to retry.
    public var delivery: PlatformPaymentDelivery? {
        payment?.delivery
    }

    /// Whether the wallet allows running the failed operation again, as Rust
    /// decided it; see `payment`. `false` for an unclassified error.
    public var isRetryable: Bool {
        payment?.isRetryable ?? false
    }

    /// What to show a person. For most cases this is still the diagnostic
    /// detail Rust attached to the originating `PlatformWalletFFIResult` (or
    /// the context string a Swift-side guard chose when constructing the
    /// error inline); the persister cases and the value-carrying marketplace
    /// rejections compose their own text instead, because theirs is an error
    /// chain or a JSON payload that reads as gibberish in an alert. The
    /// persister chain stays available on `failureReason`.
    public var errorDescription: String? {
        switch self {
        case .nullPointer(let m, _), .invalidHandle(let m, _), .invalidParameter(let m, _),
             .invalidIdentifier(let m, _), .invalidNetwork(let m, _), .walletOperation(let m, _),
             .identityNotFound(let m, _), .identityBalanceUnavailable(let m, _), .contactNotFound(let m, _), .utf8Conversion(let m, _),
             .serialization(let m, _), .deserialization(let m, _), .memoryAllocation(let m, _),
             .arithmeticOverflow(let m, _), .noSelectableInputs(let m, _),
             .coreInsufficientFunds(let m, _),
             .assetLockNotTracked(let m, _), .assetLockAlreadyConsumed(let m, _),
             .assetLockFundingMismatch(let m, _), .assetLockInsufficientFunds(let m, _),
             .assetLockTooManyInputs(let m, _), .assetLockRecipientMismatch(let m, _),
             .assetLockNotChainLocked(let m, _), .invitationNotReady(let m, _),
             .privateContactRequestNeedsPrivateReply(let m, _),
             .shieldedBuildRequired(let m, _), .protocolVersionNotActive(let m, _),
             .privateContactRequestPending(let m, _), .privateContactPaymentsNotRestored(let m, _),
             .privateContactRecoveryUnsupported(let m, _),
             .privateContactStoreNotRead(let m, _), .sdkPinnedBelowProtocolVersion(let m, _),
             .publicContactRequestUnconfirmed(let m, _), .shieldedPlanChanged(let m, _),
             .shieldedSpendTooManyNotes(let m, _), .shieldedWithdrawalAmountOutOfRange(let m, _),
             .networkProtocolVersionUnsupported(let m, _),
             .identityLookupFailed(let m, _),
             .balanceHistoryInconsistentHeights(let m, _),
             .shieldedOperationInProgress(let m, _),
             .shieldedTipPending(let m, _), .shieldedTipRecipientMismatch(let m, _),
             .shieldedBalanceBusy(let m, _),
             .walletAlreadyExists(let m, _), .shieldedBroadcastFailed(let m, _),
             .shieldedBroadcastUnconfirmed(let m, _), .shieldedSpendUnconfirmed(let m, _),
             .shieldedNoRecordedAnchor(let m, _), .shieldedMerkleWitnessUnavailable(let m, _),
             .shieldedInsufficientBalance(let m, _),
             .shieldedIdentityDebitPending(let m, _),
             .shieldedRecoveryCorrupted(let m, _), .shieldedRecoveryKeysRequired(let m, _),
             .transactionBroadcastUnconfirmed(let m, _),
             .masternodeWithdrawalUnconfirmed(let m, _),
             .masternodeListUnavailable(let m, _),
             .transactionBroadcastRejected(let m, _),
             .addressNonceMismatch(let m, _),
             .shutdownIncomplete(let m, _),
             .signingKeyUnavailable(let m, _),
             .staleReservationToken(let m, _), .reservationTokenConsumed(let m, _),
             .reservationWalletMismatch(let m, _),
             .notForSale(let m, _),
             .assetLockInputConflict(let m, _),
             .assetLockInputContested(let m, _),
             .consensusRejection(_, let m, _),
             .notFound(let m, _), .unknown(let m, _):
            return m
        // The persister messages are a nested Rust error chain naming the
        // operation, the backend classification and the store's own phrasing
        // ("… changeset: persistence backend error (Transient): database is
        // locked"). That is log material, not alert material, so these six
        // state what the person can do and leave the chain on
        // `failureReason`. Which text applies is the CASE's meaning: a
        // transient is worth retrying, a read failure and a write failure
        // must not be described to a user as each other.
        // A busy store is worth retrying only where Rust allowed it.
        case .persisterLoadTransient(_, let payment), .persisterStoreTransient(_, let payment):
            return payment?.isRetryable == true
                ? "The wallet database is busy. Try again in a moment."
                : "The wallet database is busy."
        case .persisterLoadFatal, .persisterRestore:
            return "The wallet data could not be read and may need to be restored."
        case .persisterStoreFatal, .persisterStoreConstraint:
            return "The wallet data could not be saved and may need to be restored."
        case .spvProcessRestartRequired:
            return "Sync could not be stopped cleanly. Restart the app to use it again."
        // The three value-carrying marketplace rejections compose their
        // description from the typed values, because their FFI message is
        // the machine-readable JSON detail — showing that raw would be
        // gibberish in a UI alert.
        // Whether anything was purchased is the payment class's to say
        // (`PlatformPaymentRecovery`), not this text's.
        case .priceChanged(_, let expected, let actual, _):
            return "The price changed from \(expected) to \(actual) credits before the purchase "
                + "could be confirmed."
        case .insufficientIdentityCredits(let identityId, let required, let available, _):
            return "Identity \(identityId) has \(available) credits but \(required) are required."
        case .invitationKeysAlreadyRegistered(let identityId, _, _):
            guard let identityId else {
                return "A key of this invitation claim is already registered to another identity."
            }
            return "A key of this invitation claim is already registered to identity \(identityId)."
        case .invitationClaimedByThisWallet(let identityId, _):
            return "This invitation was already claimed by this wallet as identity \(identityId)."
        case .contestedNameNotTradable(let label, let endsAtMs, _):
            if endsAtMs == 0 {
                return "\"\(label)\" is in an active contested-name vote and cannot be traded "
                    + "until the contest resolves."
            }
            return "\"\(label)\" is in an active contested-name vote (ends at \(endsAtMs) ms) "
                + "and cannot be traded until the contest resolves."
        }
    }

    /// The raw diagnostic chain behind a case whose `errorDescription` is
    /// user-facing text — log it, do not display it. `nil` for every case
    /// that already passes its detail through as the description.
    public var failureReason: String? {
        switch self {
        case .persisterLoadTransient(let m, _), .persisterLoadFatal(let m, _),
             .persisterStoreTransient(let m, _), .persisterStoreFatal(let m, _),
             .persisterStoreConstraint(let m, _), .persisterRestore(let m, _),
             .spvProcessRestartRequired(let m, _):
            return m
        default:
            return nil
        }
    }

    init(result: PlatformWalletResult) {
        self.init(
            code: result.code,
            message: result.message,
            consensus: result.consensusError,
            payment: result.paymentClass
        )
    }

    /// Internal seam for exercising the stable error-code/detail contract
    /// without manufacturing a Rust-owned `PlatformWalletFFIResult` string.
    /// Production callers continue to enter through `init(result:)`.
    ///
    /// `consensus` is Platform's verdict when the result carried one, and is
    /// consulted on the catch-all code, where it picks the case, and on code
    /// 63, which keeps it beside its own classification. Every other code is
    /// a classification the wallet layer chose deliberately and must keep
    /// reaching its own case.
    ///
    /// `payment` is Rust's class of the failure, kept by the cases that carry
    /// one; `nil` when no wallet error classified it. It is never derived
    /// from `code`.
    init(
        code: PlatformWalletResultCode,
        message: String?,
        consensus: PlatformConsensusError? = nil,
        payment: PlatformPaymentClass? = nil
    ) {
        let detail = message ?? "<no detail from Rust>"
        switch code {
        case .success:
            // Constructing an error from a success result is a caller bug
            self = .unknown(
                message ?? "PlatformWalletError built from a success result",
                payment: nil
            )
        case .errorInvalidHandle:     self = .invalidHandle(detail, payment: payment)
        case .errorInvalidParameter:  self = .invalidParameter(detail, payment: payment)
        case .errorNullPointer:       self = .nullPointer(detail, payment: payment)
        case .errorSerialization:     self = .serialization(detail, payment: payment)
        case .errorDeserialization:   self = .deserialization(detail, payment: payment)
        case .errorWalletOperation:   self = .walletOperation(detail, payment: payment)
        case .errorIdentityNotFound:  self = .identityNotFound(detail, payment: payment)
        case .errorIdentityBalanceUnavailable: self = .identityBalanceUnavailable(detail, payment: payment)
        case .errorContactNotFound:   self = .contactNotFound(detail, payment: payment)
        case .errorInvalidNetwork:    self = .invalidNetwork(detail, payment: payment)
        case .errorInvalidIdentifier: self = .invalidIdentifier(detail, payment: payment)
        case .errorMemoryAllocation:  self = .memoryAllocation(detail, payment: payment)
        case .errorUtf8Conversion:    self = .utf8Conversion(detail, payment: payment)
        case .errorArithmeticOverflow: self = .arithmeticOverflow(detail, payment: payment)
        case .errorNoSelectableInputs: self = .noSelectableInputs(detail, payment: payment)
        case .errorCoreInsufficientFunds: self = .coreInsufficientFunds(detail, payment: payment)
        case .errorAssetLockNotTracked: self = .assetLockNotTracked(detail, payment: payment)
        case .errorAssetLockAlreadyConsumed: self = .assetLockAlreadyConsumed(detail, payment: payment)
        case .errorAssetLockFundingMismatch: self = .assetLockFundingMismatch(detail, payment: payment)
        case .errorAssetLockInsufficientFunds: self = .assetLockInsufficientFunds(detail, payment: payment)
        case .errorAssetLockTooManyInputs: self = .assetLockTooManyInputs(detail, payment: payment)
        case .errorAssetLockRecipientMismatch:
            self = .assetLockRecipientMismatch(detail, payment: payment)
        case .errorAssetLockNotChainLocked:
            self = .assetLockNotChainLocked(detail, payment: payment)
        case .errorInvitationNotReady: self = .invitationNotReady(detail, payment: payment)
        // Codes 63/64 carry a stable JSON detail naming the identity; a
        // payload that does not decode degrades to `.unknown(detail)`.
        case .errorInvitationKeysAlreadyRegistered:
            if let d = InvitationErrorDetail.decode(detail) {
                self = .invitationKeysAlreadyRegistered(
                    identityId: d.identityId, consensus: consensus, payment: payment
                )
            } else {
                self = .unknown(detail, payment: payment)
            }
        case .errorInvitationClaimedByThisWallet:
            if let identityId = InvitationErrorDetail.decode(detail)?.identityId {
                self = .invitationClaimedByThisWallet(identityId: identityId, payment: payment)
            } else {
                self = .unknown(detail, payment: payment)
            }
        case .errorPrivateContactRequestNeedsPrivateReply:
            self = .privateContactRequestNeedsPrivateReply(detail, payment: payment)
        case .errorShieldedBuildRequired:
            self = .shieldedBuildRequired(detail, payment: payment)
        case .errorProtocolVersionNotActive:
            self = .protocolVersionNotActive(detail, payment: payment)
        case .errorPrivateContactRequestPending:
            self = .privateContactRequestPending(detail, payment: payment)
        case .errorPrivateContactPaymentsNotRestored:
            self = .privateContactPaymentsNotRestored(detail, payment: payment)
        case .errorPrivateContactRecoveryUnsupported:
            self = .privateContactRecoveryUnsupported(detail, payment: payment)
        case .errorPrivateContactStoreNotRead:
            self = .privateContactStoreNotRead(detail, payment: payment)
        case .errorSdkPinnedBelowProtocolVersion:
            self = .sdkPinnedBelowProtocolVersion(detail, payment: payment)
        case .errorPublicContactRequestUnconfirmed:
            self = .publicContactRequestUnconfirmed(detail, payment: payment)
        case .errorShieldedPlanChanged:
            self = .shieldedPlanChanged(detail, payment: payment)
        case .errorShieldedSpendTooManyNotes:
            self = .shieldedSpendTooManyNotes(detail, payment: payment)
        case .errorShieldedWithdrawalAmountOutOfRange:
            self = .shieldedWithdrawalAmountOutOfRange(detail, payment: payment)
        case .errorNetworkProtocolVersionUnsupported:
            self = .networkProtocolVersionUnsupported(detail, payment: payment)
        case .errorIdentityLookupFailed:
            self = .identityLookupFailed(detail, payment: payment)
        case .errorBalanceHistoryInconsistentHeights:
            self = .balanceHistoryInconsistentHeights(detail, payment: payment)
        case .errorShieldedOperationInProgress:
            self = .shieldedOperationInProgress(detail, payment: payment)
        case .errorShieldedTipPending:
            self = .shieldedTipPending(detail, payment: payment)
        case .errorShieldedTipRecipientMismatch:
            self = .shieldedTipRecipientMismatch(detail, payment: payment)
        case .errorShieldedBalanceBusy:
            self = .shieldedBalanceBusy(detail, payment: payment)
        case .errorWalletAlreadyExists: self = .walletAlreadyExists(detail, payment: payment)
        case .errorShieldedBroadcastFailed: self = .shieldedBroadcastFailed(detail, payment: payment)
        case .errorShieldedBroadcastUnconfirmed: self = .shieldedBroadcastUnconfirmed(detail, payment: payment)
        case .errorShieldedSpendUnconfirmed: self = .shieldedSpendUnconfirmed(detail, payment: payment)
        case .errorShieldedNoRecordedAnchor: self = .shieldedNoRecordedAnchor(detail, payment: payment)
        case .errorShieldedMerkleWitnessUnavailable:
            self = .shieldedMerkleWitnessUnavailable(detail, payment: payment)
        case .errorShieldedInsufficientBalance: self = .shieldedInsufficientBalance(detail, payment: payment)
        case .errorTransactionBroadcastUnconfirmed:
            self = .transactionBroadcastUnconfirmed(detail, payment: payment)
        case .errorMasternodeWithdrawalUnconfirmed:
            self = .masternodeWithdrawalUnconfirmed(detail, payment: payment)
        case .errorMasternodeListUnavailable:
            self = .masternodeListUnavailable(detail, payment: payment)
        case .errorTransactionBroadcastRejected:
            self = .transactionBroadcastRejected(detail, payment: payment)
        case .errorAddressNonceMismatch:
            self = .addressNonceMismatch(detail, payment: payment)
        case .errorShutdownIncomplete:
            self = .shutdownIncomplete(detail, payment: payment)
        case .errorSpvProcessRestartRequired:
            self = .spvProcessRestartRequired(detail, payment: payment)
        case .errorSigningKeyUnavailable:
            self = .signingKeyUnavailable(detail, payment: payment)
        case .errorStaleReservationToken:
            self = .staleReservationToken(detail, payment: payment)
        case .errorReservationTokenConsumed:
            self = .reservationTokenConsumed(detail, payment: payment)
        case .errorReservationWalletMismatch:
            self = .reservationWalletMismatch(detail, payment: payment)
        case .errorDocumentNotForSale:
            self = .notForSale(detail, payment: payment)
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
                    actual: d.actual,
                    payment: payment
                )
            } else {
                self = .unknown(detail, payment: payment)
            }
        case .errorInsufficientIdentityCredits:
            if let d = TradeErrorDetail.insufficientCredits(detail) {
                self = .insufficientIdentityCredits(
                    identityId: d.identityId,
                    required: d.required,
                    available: d.available,
                    payment: payment
                )
            } else {
                self = .unknown(detail, payment: payment)
            }
        case .errorContestedNameNotTradable:
            if let d = TradeErrorDetail.contestedName(detail) {
                self = .contestedNameNotTradable(label: d.label, endsAtMs: d.endsAtMs, payment: payment)
            } else {
                self = .unknown(detail, payment: payment)
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
            self = .assetLockInputConflict(detail, payment: payment)
        case .errorAssetLockInputContested:
            self = .assetLockInputContested(detail, payment: payment)
        // The persister codes carry the wallet's typed `Display` as the
        // message. Which operation failed and whether a retry can help is
        // the CODE's meaning, not the string's — branch on the case, never
        // on the text, and log the string rather than displaying it
        // (`errorDescription` holds the user-facing wording).
        case .errorPersisterLoadTransient:
            self = .persisterLoadTransient(detail, payment: payment)
        case .errorPersisterLoadFatal:
            self = .persisterLoadFatal(detail, payment: payment)
        case .errorPersisterStoreTransient:
            self = .persisterStoreTransient(detail, payment: payment)
        case .errorPersisterStoreFatal:
            self = .persisterStoreFatal(detail, payment: payment)
        case .errorPersisterStoreConstraint:
            self = .persisterStoreConstraint(detail, payment: payment)
        case .errorPersisterRestore:
            self = .persisterRestore(detail, payment: payment)
        case .errorShieldedIdentityDebitPending:
            self = .shieldedIdentityDebitPending(detail, payment: payment)
        case .errorShieldedRecoveryCorrupted:
            self = .shieldedRecoveryCorrupted(detail, payment: payment)
        case .errorShieldedRecoveryKeysRequired:
            self = .shieldedRecoveryKeysRequired(detail, payment: payment)
        case .notFound:               self = .notFound(detail, payment: payment)
        // The catch-all, which is where a consensus rejection with no
        // dedicated code lands. When Rust stamped its verdict on the result,
        // keep it: a host that has to tell one rejection from another gets
        // the code instead of the rendered sentence.
        case .errorUnknown:
            if let consensus {
                self = .consensusRejection(consensus, detail, payment: payment)
            } else {
                self = .unknown(detail, payment: payment)
            }
        }
    }
}

// MARK: - Marketplace trade-error detail decoding

/// Decoders for the stable JSON detail objects that FFI result codes
/// 38/39/40 put in the result `message`.
///
/// `PlatformWalletFFIResult` carries no per-error value fields, so the
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

// MARK: - Invitation-claim error detail decoding

/// Decoder for the JSON detail FFI result codes 63/64 put in the result
/// `message`: `{"identityId":"<base58>"}`, with `null` when code 63 could not
/// name the identity.
private enum InvitationErrorDetail {
    struct Detail: Decodable {
        let identityId: String?
    }

    static func decode(_ message: String?) -> Detail? {
        guard let data = message?.data(using: .utf8) else { return nil }
        return try? JSONDecoder().decode(Detail.self, from: data)
    }
}

// MARK: - Convenience extensions

extension PlatformWalletFFIResult {
    /// A result built from the `{ code, message }` pair alone, carrying no
    /// consensus verdict, no payment-delivery class (`None` on success,
    /// `Unknown` otherwise), no held asset lock and no retry: the same
    /// defaults Rust's own `ok()` / `err()` constructors fill in.
    ///
    /// Swift-side call sites that fabricate a result (the manager's
    /// null-pointer guard, the native call doubles in the tests) describe a
    /// failure this side invented, which by definition is not a Platform
    /// rejection, so they say only what they mean.
    init(code: PlatformWalletFFIResultCode, message: UnsafeMutablePointer<CChar>?) {
        self.init(
            code: code,
            message: message,
            consensus_code: 0,
            consensus_kind: PLATFORM_WALLET_FFI_CONSENSUS_ERROR_KIND_NONE,
            payment_delivery: code == PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS
                ? PLATFORM_WALLET_FFI_PAYMENT_DELIVERY_NONE
                : PLATFORM_WALLET_FFI_PAYMENT_DELIVERY_UNKNOWN,
            held_asset_lock: OutPointFFI(),
            retryable: false
        )
    }

    @inline(__always)
    func check() throws {
        try PlatformWalletResult(self).throwIfError()
    }

    @inline(__always)
    func discard() {
        _ = PlatformWalletResult(self)
    }
}
