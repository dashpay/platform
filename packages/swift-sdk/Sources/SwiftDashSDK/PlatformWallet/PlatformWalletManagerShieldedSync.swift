import Foundation
import DashSDKFFI

/// Thrown by `shieldedIdentityCreateFromPool` when the Type-20
/// transition may have reached a node (a broadcast a node accepted, a
/// send attempt that may have delivered it, a node's refusal or verdict),
/// its result is not proved, and a direct fetch of the derived id also
/// came back empty (the Rust side already retried).
///
/// This is NOT a registration failure: the identity may already exist
/// on chain or may still be created (its result is not proved, e.g. after
/// a transient DAPI/proof error or a node's refusal). The caller MUST hold
/// the slot against re-submission and surface the pending identity
/// rather than treating it as unregistered — re-registering the same
/// keys while the identity is live would fail the registered-key-hash
/// stateful check and burn the funds. The note reservations were
/// intentionally left in place wallet-side; the next nullifier sync
/// reconciles them.
///
/// `identityId` is the 32-byte derived id the FFI filled into
/// `outIdentityId` on this specific result code (valid, deterministic
/// in the spent notes). `message` is the Rust-supplied diagnostic.
public struct ShieldedIdentityCreateUnconfirmedError: LocalizedError {
    public let identityId: Data
    public let message: String
    public var errorDescription: String? { message }
}

/// Per-wallet outcome from a completed shielded sync pass.
///
/// Mirrors the Rust-side
/// [`ShieldedSyncWalletResultFFI`](https://github.com/dashpay/platform/blob/v4.0-dev/packages/rs-platform-wallet-ffi/src/shielded_types.rs)
/// with three states:
///
/// - `success == true`: sync succeeded; the numeric counters are
///   meaningful and `errorMessage` is `nil`.
/// - `skipped == true`: the wallet has no bound shielded sub-wallet
///   so the pass passed it over; both `success` and `errorMessage`
///   are vacuous.
/// - both flags `false` and `errorMessage != nil`: the sync itself
///   failed.
public struct ShieldedWalletSyncResult: Sendable {
    public let walletId: Data
    public let success: Bool
    public let skipped: Bool
    /// `true` when `success` is true but the Rust pass was
    /// short-circuited by the caught-up cooldown — no SDK fetch
    /// / trial-decrypt / nullifier scan / balance read ran.
    /// When this flag is set, every numeric field on this
    /// struct (`newNotes`, `totalScanned`, `newlySpent`,
    /// `balance`) is zero — hosts should preserve their cached
    /// balance and counters rather than apply the payload.
    /// `false` for any pass that actually walked Platform.
    public let cooldownSkip: Bool
    public let newNotes: UInt32
    public let totalScanned: UInt64
    public let newlySpent: UInt32
    public let balance: UInt64
    public let errorMessage: String?

    init(ffi: ShieldedSyncWalletResultFFI) {
        var walletId = ffi.wallet_id
        self.walletId = withUnsafeBytes(of: &walletId) { Data($0) }
        self.success = ffi.success
        self.skipped = ffi.skipped
        self.cooldownSkip = ffi.cooldown_skip
        self.newNotes = ffi.new_notes
        self.totalScanned = ffi.total_scanned
        self.newlySpent = ffi.newly_spent
        self.balance = ffi.balance
        self.errorMessage = ffi.error_message.map { String(cString: $0) }
    }
}

/// One shielded sync pass dispatched from the Rust coordinator.
public struct ShieldedSyncEvent: Sendable {
    public let syncUnixSeconds: UInt64
    public let walletResults: [ShieldedWalletSyncResult]

    public func result(for walletId: Data) -> ShieldedWalletSyncResult? {
        walletResults.first { $0.walletId == walletId }
    }
}

/// The native shield-from-identity-to-recipient call; injectable through
/// `PlatformWalletManager.nativeShieldFromIdentityToRecipientCall`.
typealias ShieldFromIdentityToRecipientCall = @Sendable (
    Handle, UnsafePointer<UInt8>, UInt32, UnsafePointer<UInt8>, UnsafePointer<UInt8>,
    UInt64, UnsafePointer<CChar>?, OpaquePointer, UnsafeMutablePointer<UInt64>
) -> PlatformWalletFFIResult

/// Which token pool spend a `shieldedTokenSpendPreview` asks about. Mirrors
/// the Rust `TokenPoolSpendKind`; the raw value is the FFI `kind` byte.
public enum ShieldedTokenPoolSpendKind: UInt8, Sendable {
    /// `shieldedTokenTransfer` (type 26): the tokens move within the pool.
    case shieldedTransfer = 0
    /// `shieldedTokenUnshield` (type 27): the tokens leave the pool into an
    /// identity's token balance.
    case unshield = 1
}

/// What a token pool spend would take from an account's notes, as Rust
/// chooses them (`shieldedTokenSpendPreview`). Mirrors the Rust
/// `ShieldedTokenSpendPreview`; every value is Rust's.
public struct ShieldedTokenSpendPreview: Sendable, Hashable {
    /// The token notes the spend would spend.
    public let tokenNotes: UInt32
    /// The credit pool notes that would pay its fee.
    public let feeNotes: UInt32
    /// The fee, in credits, paid from the credit pool.
    public let fee: UInt64

    public init(tokenNotes: UInt32, feeNotes: UInt32, fee: UInt64) {
        self.tokenNotes = tokenNotes
        self.feeNotes = feeNotes
        self.fee = fee
    }
}

/// One account's tokens in one token shielded pool a wallet watches, as the
/// local scan knows them. Mirrors the Rust `ShieldedTokenPoolBalance`
/// (`ShieldedTokenPoolBalanceFFI` across the FFI); every value is Rust's,
/// copied as is. `shieldedTokenPoolBalances(walletId:)` lists them, one per
/// watched pool and bound account.
public struct ShieldedTokenPoolBalance: Sendable, Hashable {
    /// The watched token's 32-byte id.
    public let tokenId: Data
    /// The 32-byte id of the contract that defines the token.
    public let dataContractId: Data
    /// The token's position in its contract.
    public let tokenContractPosition: UInt16
    /// The Orchard account the notes belong to.
    public let accountIndex: UInt32
    /// The value of the account's unspent notes in the pool that no spend in
    /// flight holds, in the token's smallest unit: what a token pool spend
    /// can use now. Covers exactly the `scannedNoteCount` notes checked.
    public let balance: UInt64
    /// How many notes make up `balance`.
    public let unspentNoteCount: UInt32
    /// The value of the account's unspent notes a spend in flight holds (a
    /// submitted `shieldedTokenTransfer`), in the token's smallest unit. They
    /// leave when a shielded sync sees them spent, or return to `balance` if
    /// the spend can no longer execute. Not part of `balance`.
    public let reservedBalance: UInt64
    /// How many of the pool's notes, from the first without a gap, were
    /// checked for this account; `nil` before the first scan reached it.
    public let scannedNoteCount: UInt64?
    /// The highest note count a page of the pool's scans has proved; `nil`
    /// before any. A scan answered by a lagging node never lowers it, and it
    /// does not mean the scan covered it: `scannedNoteCount` is what was
    /// checked.
    public let poolNoteCount: UInt64?
    /// The highest height a page of the pool's scans was proven at; `nil`
    /// before any. It does not by itself certify that a scan reached the
    /// pool's end.
    public let scannedHeight: UInt64?
    /// Why the last scan of the pool stopped early, if it did. The next
    /// chunk read clears it.
    public let lastError: String?

    public init(
        tokenId: Data,
        dataContractId: Data,
        tokenContractPosition: UInt16,
        accountIndex: UInt32,
        balance: UInt64,
        unspentNoteCount: UInt32,
        reservedBalance: UInt64 = 0,
        scannedNoteCount: UInt64?,
        poolNoteCount: UInt64?,
        scannedHeight: UInt64?,
        lastError: String?
    ) {
        self.tokenId = tokenId
        self.dataContractId = dataContractId
        self.tokenContractPosition = tokenContractPosition
        self.accountIndex = accountIndex
        self.balance = balance
        self.unspentNoteCount = unspentNoteCount
        self.reservedBalance = reservedBalance
        self.scannedNoteCount = scannedNoteCount
        self.poolNoteCount = poolNoteCount
        self.scannedHeight = scannedHeight
        self.lastError = lastError
    }

    /// Copy a Rust-owned row. Each `has_*` flag gates its value: a `false`
    /// flag becomes `nil`, never the zero the C struct holds. Must run while
    /// the Rust allocation is alive: the error string is copied, not kept.
    init(ffi: ShieldedTokenPoolBalanceFFI) {
        var tokenId = ffi.token_id
        var contractId = ffi.data_contract_id
        self.init(
            tokenId: withUnsafeBytes(of: &tokenId) { Data($0) },
            dataContractId: withUnsafeBytes(of: &contractId) { Data($0) },
            tokenContractPosition: ffi.token_contract_position,
            accountIndex: ffi.account_index,
            balance: ffi.balance,
            unspentNoteCount: ffi.unspent_note_count,
            reservedBalance: ffi.reserved_balance,
            scannedNoteCount: ffi.has_scanned_note_count ? ffi.scanned_note_count : nil,
            poolNoteCount: ffi.has_pool_note_count ? ffi.pool_note_count : nil,
            scannedHeight: ffi.has_scanned_height ? ffi.scanned_height : nil,
            lastError: ffi.last_error.map { String(cString: $0) }
        )
    }
}

extension PlatformWalletManager {
    /// Shutdown leaves the handle live for admitted operations after its
    /// early shielded stop. Do not restart or mutate that lifecycle while
    /// the main actor is reentrant during the drain.
    private func ensureShieldedLifecycleAllowed(_ name: String) throws {
        try ensureConfigured()
        guard !shutdownRequested else {
            throw PlatformWalletError.invalidHandle(
                "manager shutdown is in progress; \(name) rejected")
        }
    }

    /// Throws `walletOperation` while an async [`stopShieldedSync()`] is in
    /// flight. The main actor is free during that stop, so a shielded call
    /// issued meanwhile would otherwise restart the loop the stop is
    /// cancelling, start a pass that outlives it, or block the main thread on
    /// the same drain.
    func ensureNoShieldedStopInFlight(before operation: String) throws {
        guard shieldedStopsInFlight == 0 else {
            throw PlatformWalletError.walletOperation(
                "Shielded sync stop in progress; await stopShieldedSync() before \(operation)")
        }
    }

    func handleShieldedSyncCompleted(_ event: ShieldedSyncEvent, generation: UInt64) {
        // Drop a trailing event that the Rust drain already dispatched but
        // the main actor only delivers after stop/clear returned. The FFI
        // callback snapshots `shieldedSyncGeneration` at enqueue time; a
        // stop/clear bumps the counter, so a stale event's snapshot no
        // longer matches and is dropped — even if a restart happened in the
        // same actor turn (the restart does not reset the counter). Shutdown
        // suppresses callbacks immediately, while leaving the generation
        // valid until admitted local reads finish their MainActor delivery.
        guard !shutdownRequested, generation == shieldedSyncGeneration.current() else { return }
        // The main actor is free while an async stop drains, so a completion
        // of the pass being stopped arrives with a current generation. Hold
        // it for the stop to settle (`shieldedCompletionHeldByStop`): dropped
        // if the stop drains that pass, published if the stop times out and
        // the pass keeps running.
        if shieldedStopsInFlight > 0 {
            shieldedCompletionHeldByStop = (event, generation)
            return
        }
        lastShieldedSyncEvent = event
        // A completed pass means the per-chunk progress counter for
        // this pass is no longer meaningful — clear so the next pass
        // starts from nil. Also matches the false→true edge UI gating
        // in ShieldedService's currentSyncElapsed timer.
        resetCurrentShieldedProgress()
    }

    /// Clear the four per-pass live-progress mirrors so the next pass
    /// starts from nil. Routed through by every path that ends a pass:
    /// the normal completion (`handleShieldedSyncCompleted`) plus the
    /// `stopShieldedSync()` / `clearShielded()` paths that suppress the
    /// trailing completion event — without this, a pass stopped/cleared
    /// mid-flight would leave the last published `currentShielded*`
    /// values visible (stale UI) between passes. Main-actor-isolated
    /// like every other member of this `@MainActor` class.
    private func resetCurrentShieldedProgress() {
        currentShieldedSyncScanned = nil
        currentShieldedSyncBlockHeight = nil
        currentShieldedTreeCommitted = nil
        currentShieldedTreeTotal = nil
    }

    /// Per-chunk progress callback. Fires once per ~2048 notes
    /// processed during a cold sync; bridged here from the C
    /// trampoline `shieldedSyncProgressCallback`. Cheap publish; UI
    /// gets it through ShieldedService.
    ///
    /// Generation-guarded like `handleShieldedSyncCompleted`: a stale
    /// progress hop delivered after a stop/clear bumped the generation
    /// must be dropped so it can't re-publish phantom progress over the
    /// `resetCurrentShieldedProgress()` mirrors the stop/clear just reset.
    /// Dropped outright while an async stop is in flight, for the same
    /// reason.
    func handleShieldedSyncProgress(
        cumulativeScanned: UInt64,
        blockHeight: UInt64,
        generation: UInt64
    ) {
        guard !shutdownRequested, shieldedStopsInFlight == 0,
              generation == shieldedSyncGeneration.current() else { return }
        currentShieldedSyncScanned = cumulativeScanned
        currentShieldedSyncBlockHeight = blockHeight
    }

    /// Per-batch tree-progress callback — the "checked /
    /// committed-to-tree" signal. Fires once per committed batch as
    /// commitments are appended to the local Orchard tree; bridged
    /// here from the C trampoline `shieldedTreeProgressCallback`.
    /// `total == 0` means the on-chain total is indeterminate. Cheap
    /// publish; UI gets it through ShieldedService.
    ///
    /// Generation-guarded like `handleShieldedSyncCompleted`: a stale
    /// tree-progress hop delivered after a stop/clear bumped the
    /// generation must be dropped so it can't re-publish phantom progress
    /// over the `resetCurrentShieldedProgress()` mirrors the stop/clear
    /// just reset. Dropped outright while an async stop is in flight, for
    /// the same reason.
    func handleShieldedTreeProgress(
        committed: UInt64,
        total: UInt64,
        generation: UInt64
    ) {
        guard !shutdownRequested, shieldedStopsInFlight == 0,
              generation == shieldedSyncGeneration.current() else { return }
        currentShieldedTreeCommitted = committed
        currentShieldedTreeTotal = total
    }

    /// Bind `walletId`'s multi-account shielded sub-wallet to the
    /// `PlatformWallet` — from viewing keys the persister already
    /// holds when possible, deriving from the mnemonic only when it
    /// doesn't.
    ///
    /// `accounts` is the list of ZIP-32 account indices to bind.
    /// Pass `[0]` for the single-account default; pass
    /// `[0, 1, …]` to bind multiple accounts up front. Each entry
    /// produces an independent FVK / IVK / OVK / default address;
    /// notes are scoped per-`(walletId, accountIndex)` inside the
    /// store. Must be non-empty and at most 64 entries.
    ///
    /// The resolver does NOT fire on the common path: when every
    /// requested account has a persisted viewing key (written by the
    /// first seed-backed bind into `PersistentShieldedViewingKey`
    /// rows), Rust rebinds from those rows and the mnemonic is never
    /// read. Only when a row is missing (first bind after create /
    /// import) does the resolver fire — exactly once, with the
    /// mnemonic and derived seed in zeroized Rust-side buffers,
    /// scrubbed before this call returns.
    ///
    /// Idempotent: calling again replaces the previously-bound
    /// shielded wallet.
    ///
    /// **Prerequisite**: [`configureShielded(dbPath:)`] must have
    /// been called on this manager first — the per-network
    /// SQLite handle is opened there. `bindShielded` reuses it
    /// across every wallet.
    ///
    /// Throws `walletOperation` while an async [`stopShieldedSync()`] is in
    /// flight: a first registration or a changed account set takes the
    /// shielded store lock, and would wait on the calling thread for the
    /// pass that stop is draining.
    public func bindShielded(
        walletId: Data,
        resolver: MnemonicResolver,
        accounts: [UInt32] = [0]
    ) throws {
        try ensureShieldedLifecycleAllowed("bindShielded")
        try ensureNoShieldedStopInFlight(before: "binding a shielded wallet")
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be exactly 32 bytes"
            )
        }
        guard !accounts.isEmpty else {
            throw PlatformWalletError.invalidParameter(
                "accounts must be non-empty"
            )
        }
        guard accounts.count <= 64 else {
            throw PlatformWalletError.invalidParameter(
                "accounts must contain at most 64 entries"
            )
        }
        guard let resolverHandle = resolver.handle else {
            throw PlatformWalletError.invalidParameter(
                "MnemonicResolver has no handle"
            )
        }

        try withShieldedLocalBalanceBind {
            try walletId.withUnsafeBytes { walletIdRaw in
                guard let walletIdPtr = walletIdRaw.baseAddress?
                    .assumingMemoryBound(to: UInt8.self)
                else {
                    throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
                }
                try accounts.withUnsafeBufferPointer { accountsBuf in
                    guard let accountsPtr = accountsBuf.baseAddress else {
                        throw PlatformWalletError.invalidParameter(
                            "accounts baseAddress is nil"
                        )
                    }
                    try platform_wallet_manager_bind_shielded(
                        handle,
                        walletIdPtr,
                        resolverHandle,
                        accountsPtr,
                        UInt(accountsBuf.count)
                    ).check()
                }
            }
        }
    }

    /// Prepare the dedicated shielded account of the wallet-owned identity
    /// `identityId` on `walletId` and return the account's default 43-byte
    /// raw Orchard address. Publishes nothing and moves no funds.
    ///
    /// Rust proves the identity's slot with the seed `resolver` supplies,
    /// binds the identity's account next to the wallet's bound accounts,
    /// persists its viewing key and records the preparation for this
    /// session. A contact request sent afterwards carries the sender's
    /// per-contact shielded address only once this preparation stands.
    /// Repeated calls are idempotent.
    ///
    /// **Prerequisite**: [`bindShielded(walletId:resolver:accounts:)`] on
    /// this manager. The resolver fires on every call; the mnemonic and
    /// derived seed live in zeroized Rust-side buffers and are scrubbed
    /// before the native call returns.
    ///
    /// Throws `walletOperation` while an async [`stopShieldedSync()`] is in
    /// flight, as `bindShielded` does: the preparation is a bind. From the
    /// native call it throws `identityNotFound` when the wallet holds no
    /// identity `identityId`, `shieldedRecoveryCorrupted` or
    /// `shieldedRecoveryKeysRequired` when the bind meets unusable persisted
    /// shielded state, and `walletOperation` for every other failure (an
    /// unbound wallet, a seed that does not derive the identity's key), with
    /// the wallet's reason in the message.
    public func prepareShieldedIdentityAddress(
        walletId: Data,
        identityId: Data,
        resolver: MnemonicResolver
    ) async throws -> Data {
        try ensureShieldedLifecycleAllowed("prepareShieldedIdentityAddress")
        try ensureNoShieldedStopInFlight(before: "preparing an identity shielded account")
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be exactly 32 bytes"
            )
        }
        guard identityId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "identityId must be exactly 32 bytes"
            )
        }
        guard let resolverHandle = resolver.handle else {
            throw PlatformWalletError.invalidParameter(
                "MnemonicResolver has no handle"
            )
        }
        // Counted as the other native manager operations are, so a teardown
        // waits for this call to return before it frees the manager.
        try admitNativeOp("prepareShieldedIdentityAddress")
        defer { finishNativeOp() }
        let handle = self.handle
        do {
            let address = try await Task.detached(priority: .userInitiated) { () -> Data in
                // Guaranteed keepalive of the resolver across the FFI call,
                // whose vtable callback fires during it: same rationale as
                // `shieldedTransfer`.
                try withExtendedLifetime(resolver) {
                    var bytes = [UInt8](repeating: 0, count: 43)
                    try walletId.withUnsafeBytes { widRaw in
                        guard let widPtr = widRaw.baseAddress?
                            .assumingMemoryBound(to: UInt8.self)
                        else {
                            throw PlatformWalletError.invalidParameter(
                                "walletId baseAddress is nil"
                            )
                        }
                        try identityId.withUnsafeBytes { idRaw in
                            guard let idPtr = idRaw.baseAddress?
                                .assumingMemoryBound(to: UInt8.self)
                            else {
                                throw PlatformWalletError.invalidParameter(
                                    "identityId baseAddress is nil"
                                )
                            }
                            try bytes.withUnsafeMutableBufferPointer { outBuf in
                                guard let outPtr = outBuf.baseAddress else {
                                    throw PlatformWalletError.invalidParameter(
                                        "prepareShieldedIdentityAddress out buffer baseAddress is nil"
                                    )
                                }
                                try platform_wallet_manager_prepare_shielded_identity_address(
                                    handle,
                                    widPtr,
                                    resolverHandle,
                                    idPtr,
                                    outPtr
                                ).check()
                            }
                        }
                    }
                    return Data(bytes)
                }
            }.value
            // The preparation can add an account to the bound set, so a local
            // balance snapshot read across it is obsolete, as after a bind.
            shieldedLocalBalanceBindGeneration.bump()
            return address
        } catch {
            shieldedLocalBalanceGeneration.bump()
            throw error
        }
    }

    /// Configure the network-scoped shielded coordinator. Opens
    /// the per-network commitment-tree SQLite file at `dbPath`
    /// and installs a single shared handle every subsequent
    /// [`bindShielded`] call reuses.
    ///
    /// Must be called once before any `bindShielded` on this
    /// manager. Idempotent: a second call with the same `dbPath`
    /// is a no-op; a second call with a different `dbPath`
    /// throws — the SQLite handle is opened once per manager and
    /// can't be repointed mid-flight.
    public func configureShielded(dbPath: String) throws {
        try ensureShieldedLifecycleAllowed("configureShielded")
        try dbPath.withCString { dbPathPtr in
            try platform_wallet_manager_configure_shielded(handle, dbPathPtr).check()
        }
    }

    /// Start the shielded sync coordinator's background loop.
    ///
    /// Wallets that have not yet been bound via [`bindShielded`] are
    /// emitted as `skipped` results on every pass — the host can
    /// call `bindShielded` later and the loop will pick the binding
    /// up on its next tick.
    ///
    /// Throws `walletOperation` while an async [`stopShieldedSync()`] is in
    /// flight, instead of restarting the loop that stop is cancelling.
    public func startShieldedSync(intervalSeconds: UInt64? = nil) throws {
        try ensureShieldedLifecycleAllowed("startShieldedSync")
        try ensureNoShieldedStopInFlight(before: "starting shielded sync")

        if let intervalSeconds {
            try setShieldedSyncInterval(seconds: intervalSeconds)
        }
        // No generation reset needed: events emitted by this new run
        // snapshot the current generation, so they pass the guard. A
        // trailing event from a prior, stopped run still carries the older
        // generation and is dropped.
        try platform_wallet_manager_shielded_sync_start(handle).check()
    }

    /// Stop the shielded sync loop, waiting on the calling thread.
    ///
    /// The native stop cancels the loop and the pass in flight, then waits
    /// up to 10 s (the Rust coordinator's drain budget) for that pass to
    /// stop at its next checkpoint; a scan cut short publishes nothing and
    /// the next pass re-walks from the last completed watermark. On the
    /// main actor the wait freezes the UI for as long; the async overload
    /// runs the same stop off the main thread. If the pass does not drain
    /// in time (it is wedged in a host persister, or in a broadcast the
    /// SDK is still retrying) this throws `shutdownIncomplete`, before
    /// bumping the generation.
    ///
    /// Throws `walletOperation` while an async [`stopShieldedSync()`] is in
    /// flight: this stop would wait on the same drain on the calling thread.
    public func stopShieldedSync() throws {
        try ensureShieldedLifecycleAllowed("stopShieldedSync")
        try ensureNoShieldedStopInFlight(before: "a blocking stopShieldedSync()")
        try platform_wallet_manager_shielded_sync_stop(handle).check()
        // The Rust drain returned; bump the generation so any trailing
        // completion event the main actor delivers after this point is
        // dropped (its snapshot predates this bump).
        shieldedSyncGeneration.bump()
        // The dropped completion would normally clear the per-pass
        // progress mirrors; do it here so a pass stopped mid-flight
        // doesn't leave stale `currentShielded*` values on the UI.
        resetCurrentShieldedProgress()
    }

    /// Off-main variant of [`stopShieldedSync()`]: the same native stop, run
    /// on [`shieldedStopQueue`] instead of the calling thread. In an `async`
    /// context overload resolution prefers this variant; sync contexts keep
    /// the blocking one.
    ///
    /// The native stop cancels the loop at once, so no new pass starts, and
    /// cancels the pass in flight, which stops at its next checkpoint and
    /// publishes nothing; the stop waits up to 10 s for it to do so. If it
    /// does not drain in time (wedged in a host persister, or in a broadcast
    /// the SDK is still retrying) this throws `shutdownIncomplete`, and the
    /// pass keeps running: it can still persist what it finds and deliver
    /// its events after this returns.
    ///
    /// Events: while the stop is in flight, progress events that reach the
    /// main actor are dropped, and a completion is held
    /// ([`shieldedCompletionHeldByStop`]). A stop that drains the pass bumps
    /// the generation and clears the progress mirrors, which drops the held
    /// completion and the events that pass dispatched but the main actor only
    /// delivers afterwards. A stop that times out leaves the generation and
    /// the mirrors alone, like the blocking overload, and publishes the held
    /// completion: the pass keeps running, and its events are real data. A
    /// stop that finishes while [`shutdown()`] is draining admitted
    /// operations also leaves the generation alone: the bump would cancel the
    /// local balance reads that drain waits for, and shutdown bumps the
    /// generation itself afterwards.
    ///
    /// Cancelling the calling task does not abort the stop. The native call
    /// cannot be interrupted, so the caller keeps waiting for its result, and
    /// the steps after it still run.
    ///
    /// Admitted like the other async native entry points: `shutdown()` waits
    /// for an in-flight stop before destroying the handle, and a stop
    /// requested once a shutdown has begun throws `invalidHandle` before any
    /// native work. While the stop is in flight:
    /// - [`startShieldedSync(intervalSeconds:)`], [`syncShieldedNow()`],
    ///   [`syncShieldedWalletNow(walletId:)`], the blocking stop,
    ///   [`clearShielded()`] and [`bindShielded(walletId:resolver:accounts:)`]
    ///   throw `walletOperation`;
    /// - the synchronous native operations that refuse to run beside an
    ///   admitted async operation (wallet create, load and delete) throw
    ///   `invalidHandle`;
    /// - the native call holds the FFI handle registry's read guard for the
    ///   whole drain, as the native SPV stop does.
    public func stopShieldedSync() async throws {
        try ensureShieldedLifecycleAllowed("stopShieldedSync")
        // Admission and the in-flight count change on the main actor with no
        // suspension in between, so neither `shutdown()`'s drain nor the
        // event handlers can miss this stop.
        try admitNativeOp("stopShieldedSync")
        defer { finishNativeOp() }
        shieldedStopsInFlight += 1
        defer { shieldedStopsInFlight -= 1 }

        let h = handle
        let stop = nativeTeardownCalls.shieldedSyncStop
        // Map the result on the queue: the raw result's Rust-owned message
        // never crosses the continuation.
        let (failure, milliseconds): (PlatformWalletError?, Int) = await withCheckedContinuation { continuation in
            shieldedStopQueue.async {
                let started = CFAbsoluteTimeGetCurrent()
                let result = PlatformWalletResult(stop(h))
                let milliseconds = Int((CFAbsoluteTimeGetCurrent() - started) * 1000)
                continuation.resume(
                    returning: (result.isSuccess ? nil : PlatformWalletError(result: result), milliseconds))
            }
        }
        SDKLogger.event(
            "shielded_sync_stop_completed",
            category: .lifecycle,
            severity: failure == nil ? .info : .warning,
            fields: ["duration_ms": .integer(Int64(milliseconds))],
            error: failure
        )
        // Same main-actor turn as the deferred in-flight release, so no
        // event of the drained pass can land between the two.
        if failure == nil, !shutdownRequested {
            shieldedSyncGeneration.bump()
            resetCurrentShieldedProgress()
        }
        // The last stop in flight settles the held completion: stale after
        // the bump above, still current after a timeout.
        if shieldedStopsInFlight == 1, let held = shieldedCompletionHeldByStop {
            shieldedCompletionHeldByStop = nil
            if !shutdownRequested, held.generation == shieldedSyncGeneration.current() {
                lastShieldedSyncEvent = held.event
                resetCurrentShieldedProgress()
            }
        }
        if let failure {
            throw failure
        }
    }

    /// Reset the Rust-side shielded state on this manager:
    /// stops the background sync loop, drops every wallet
    /// registration on the network-scoped coordinator, and
    /// resets the caught-up cooldown stamp.
    ///
    /// Use this from the host's "Clear" flow before wiping
    /// host-side persistence (e.g. SwiftData rows). The single
    /// per-network SQLite commitment-tree file stays open —
    /// Clear semantics are "wipe my host persistence and
    /// re-sync from index 0 on the shared tree", not "blow
    /// away the chain-wide cache". After Clear, the next
    /// [`bindShielded`] call repopulates the coordinator's
    /// registries and the next sync pass re-saves notes via
    /// the changeset path.
    ///
    /// Throws `walletOperation` while an async [`stopShieldedSync()`] is in
    /// flight: Clear quiesces the same loop and would wait on the calling
    /// thread for the pass that stop is draining.
    ///
    /// Throws `PlatformWalletError.shieldedOperationInProgress` at once,
    /// without waiting, while a shielded operation is in flight (a bind, an
    /// account being added, a token pool transfer or unshield, a DashPay
    /// profile write, a tip address preparation or a shielded tip being
    /// sent, or another Clear). Rust
    /// decides that and changed nothing: the sync loop keeps running, and
    /// this manager keeps its sync generation, progress mirrors and any
    /// local balance read in flight. The host must not wipe its own rows;
    /// call again once the operation has finished.
    ///
    /// Throws the same error, before Rust is called, while an async native
    /// operation is admitted on this manager (for example a shielded spend
    /// or a wallet drain): Clear would wipe the recovery state that
    /// operation is about to write or has armed. Nothing changed here
    /// either.
    public func clearShielded() throws {
        try ensureShieldedLifecycleAllowed("clearShielded")
        try ensureNoShieldedStopInFlight(before: "clearing shielded state")
        guard !hasAdmittedNativeOp else {
            throw PlatformWalletError.shieldedOperationInProgress(
                "an async native operation is in flight; clearShielded rejected")
        }
        try ensureSyncNativeOpAllowed("clearShielded")
        // Every throw, the refusal included, leaves before the bump and the
        // progress reset below.
        try withShieldedLocalBalanceMutation {
            try platform_wallet_manager_shielded_clear(handle).check()
        }
        // The Rust drain returned; bump the generation so any trailing
        // completion event the main actor delivers after Clear is dropped
        // (it would otherwise briefly repopulate the mirror the host is
        // about to wipe).
        shieldedSyncGeneration.bump()
        // The dropped completion would normally clear the per-pass
        // progress mirrors; do it here so a pass cleared mid-flight
        // doesn't leave stale `currentShielded*` values on the UI.
        resetCurrentShieldedProgress()
    }

    public func isShieldedSyncRunning() throws -> Bool {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        var running = false
        try platform_wallet_manager_shielded_sync_is_running(handle, &running).check()
        return running
    }

    public func isShieldedSyncing() throws -> Bool {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        return try Self.readIsShieldedSyncing(handle)
    }

    /// The native read behind [`isShieldedSyncing()`] (an atomic load,
    /// never parks); also what the progress poller runs.
    nonisolated static func readIsShieldedSyncing(_ handle: Handle) throws -> Bool {
        var syncing = false
        try platform_wallet_manager_shielded_sync_is_syncing(handle, &syncing).check()
        return syncing
    }

    public func lastShieldedSyncUnixSeconds() throws -> UInt64 {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        var lastSync: UInt64 = 0
        try platform_wallet_manager_shielded_sync_last_sync_unix_seconds(handle, &lastSync).check()
        return lastSync
    }

    public func setShieldedSyncInterval(seconds: UInt64) throws {
        try ensureShieldedLifecycleAllowed("setShieldedSyncInterval")
        try platform_wallet_manager_shielded_sync_set_interval(handle, seconds).check()
    }

    /// Throws `walletOperation` while an async [`stopShieldedSync()`] is in
    /// flight: once the stop's drain succeeds the native gate reopens, so a
    /// pass requested meanwhile could start and outlive that stop.
    public func syncShieldedNow() async throws {
        try ensureShieldedLifecycleAllowed("syncShieldedNow")
        try ensureNoShieldedStopInFlight(before: "a manual shielded sync")
        // No generation reset needed: this run's completion event snapshots
        // the current generation and passes the guard, while a trailing
        // event from a prior stopped run still carries the older generation.
        let handle = self.handle
        try await Task.detached(priority: .userInitiated) {
            try platform_wallet_manager_shielded_sync_sync_now(handle).check()
        }.value
    }

    /// Read the default Orchard payment address for `account` on
    /// `walletId` as the 43 raw bytes. Returns `nil` when the
    /// wallet exists on the manager but has no bound shielded
    /// sub-wallet, or `account` isn't bound on it. Throws when
    /// the wallet id isn't known to the manager.
    ///
    /// The host is responsible for bech32m-encoding the result
    /// for display (HRP `dash` / `tdash` + `0x10` type byte).
    public func shieldedDefaultAddress(
        walletId: Data,
        account: UInt32 = 0
    ) throws -> Data? {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be exactly 32 bytes"
            )
        }

        var bytes = [UInt8](repeating: 0, count: 43)
        var present = false
        try walletId.withUnsafeBytes { raw in
            guard let ptr = raw.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
            }
            try bytes.withUnsafeMutableBufferPointer { outBuf in
                guard let outPtr = outBuf.baseAddress else {
                    throw PlatformWalletError.invalidParameter(
                        "shieldedDefaultAddress out buffer baseAddress is nil"
                    )
                }
                try platform_wallet_manager_shielded_default_address(
                    handle,
                    ptr,
                    account,
                    outPtr,
                    &present
                ).check()
            }
        }
        return present ? Data(bytes) : nil
    }

    /// Draw a fresh Orchard payment address for `account` on `walletId`,
    /// as the 43 raw bytes. Every call draws a new, uniformly random
    /// 88-bit diversifier index from the OS CSPRNG; index 0 (the
    /// default shielded address) is never returned. Nothing is stored:
    /// the incoming viewing key finds notes sent to any address this
    /// returns, now and after a restore.
    ///
    /// Returns `nil` when the wallet is known but has no bound shielded
    /// sub-wallet, or `account` isn't bound on it. Throws when the
    /// manager is not configured, `walletId` is not 32 bytes, the wallet
    /// id isn't known to the manager, or the CSPRNG fails.
    ///
    /// The host bech32m-encodes the result for display, as for
    /// `shieldedDefaultAddress`. Synchronous and blocking on the main
    /// actor, like every member of this class: the FFI call drives the Rust
    /// runtime with `block_on` and returns once the address is derived.
    public func shieldedFreshAddress(
        walletId: Data,
        account: UInt32 = 0
    ) throws -> Data? {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be exactly 32 bytes"
            )
        }

        var bytes = [UInt8](repeating: 0, count: 43)
        var present = false
        try walletId.withUnsafeBytes { raw in
            guard let ptr = raw.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
            }
            try bytes.withUnsafeMutableBufferPointer { outBuf in
                guard let outPtr = outBuf.baseAddress else {
                    throw PlatformWalletError.invalidParameter(
                        "shieldedFreshAddress out buffer baseAddress is nil"
                    )
                }
                try platform_wallet_manager_shielded_fresh_address(
                    handle,
                    ptr,
                    account,
                    outPtr,
                    &present
                ).check()
            }
        }
        return present ? Data(bytes) : nil
    }

    /// Whether the 43-byte raw Orchard address `raw43` is one of the
    /// payment addresses of `account` on `walletId` — the default one
    /// or any diversified (fresh) one. Tests the address against the
    /// account's incoming viewing key (`diversifier_index`), the same
    /// test the shielded scan uses for self-change; nothing is looked up
    /// in storage.
    ///
    /// Returns `nil` when the wallet is known but has no bound shielded
    /// sub-wallet, or `account` isn't bound on it — ownership is unknown,
    /// not `false`. Returns `false` for a `raw43` that is not 43 bytes or
    /// is not a valid Orchard address, so a corrupt recipient never masks
    /// a real send. Throws when the manager is not configured, `walletId`
    /// is not 32 bytes, or the wallet id isn't known to the manager.
    /// Synchronous and blocking on the main actor, like every member of this
    /// class: the FFI call drives the Rust runtime with `block_on`.
    public func isOwnShieldedAddress(
        walletId: Data,
        account: UInt32 = 0,
        raw43: Data
    ) throws -> Bool? {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be exactly 32 bytes"
            )
        }

        // An empty `Data` has no base address; the FFI needs a non-null
        // pointer even for a zero length, so back it with one byte.
        let address = raw43.isEmpty ? [UInt8(0)] : [UInt8](raw43)
        var isOwn = false
        var present = false
        try walletId.withUnsafeBytes { raw in
            guard let ptr = raw.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
            }
            try address.withUnsafeBufferPointer { addressBuf in
                guard let addressPtr = addressBuf.baseAddress else {
                    throw PlatformWalletError.invalidParameter(
                        "isOwnShieldedAddress address baseAddress is nil"
                    )
                }
                try platform_wallet_manager_is_own_shielded_address(
                    handle,
                    ptr,
                    account,
                    addressPtr,
                    UInt(raw43.count),
                    &isOwn,
                    &present
                ).check()
            }
        }
        return present ? isOwn : nil
    }

    /// Build the Halo 2 proving key on a background thread so the
    /// first shielded send doesn't pay the ~30 s build cost
    /// inline. Idempotent and safe to call from any thread; later
    /// calls return immediately. Independent of any wallet — the
    /// cache is process-global on the Rust side.
    public static func warmUpShieldedProver() async {
        await Task.detached(priority: .background) {
            platform_wallet_shielded_warm_up_prover()
        }.value
    }

    /// Whether the Halo 2 proving key has been built yet. Useful
    /// for a "preparing prover…" UI affordance — `false` doesn't
    /// mean shielded sends will fail, just that the next one
    /// pays the build cost.
    public static var isShieldedProverReady: Bool {
        platform_wallet_shielded_prover_is_ready()
    }

    /// Which consensus fee formula a pool-paid shielded transition is
    /// charged under. Mirrors the `kind` byte the Rust FFI
    /// `platform_wallet_shielded_estimate_fee` dispatches on.
    public enum ShieldedFeeKind: UInt8 {
        /// ShieldedTransfer / Shield base (`compute_minimum_shielded_fee`).
        case transfer = 0
        /// Unshield (`compute_shielded_unshield_fee`).
        case unshield = 1
        /// ShieldedWithdrawal (`compute_shielded_withdrawal_fee`).
        case withdrawal = 2
        /// ShieldFromIdentity (`compute_shielded_identity_balance_write_fee`):
        /// the conservative complete-fee floor (compute + note storage
        /// allowance + identity write allowance) consensus requires the
        /// identity to hold on top of the amount; the exact fee is metered
        /// at execution.
        case shieldFromIdentity = 3
        /// IdentityTopUpFromShieldedPool
        /// (`compute_shielded_identity_top_up_fee`): the base flat fee plus
        /// the flat identity-balance write cost, carved from the value
        /// balance so the pool pays it.
        case identityTopUpFromPool = 4
        /// TokenPurchaseFromShieldedPool
        /// (`compute_token_purchase_from_pool_fee`): the fee paid on top of
        /// the price, for the token bundle's two actions and `numActions`
        /// actions in the credit pool fee bundle.
        case tokenPurchaseFromPool = 5
    }

    /// Consensus-pinned flat shielded fee (in credits) for a pool-paid
    /// shielded transition with `numActions` Orchard actions, computed at
    /// this manager's network-tracked platform version (`sdk.version()`) —
    /// the same version the shielded builders carve fees with — so the
    /// estimate can't drift from the carved fee even when the connected
    /// network hasn't activated the client's latest protocol version yet.
    /// No network round-trip; just the handle → version lookup and a pure
    /// computation. A single-note spend with change is `numActions: 2`.
    public func estimateShieldedFee(
        kind: ShieldedFeeKind,
        numActions: Int = 2
    ) throws -> UInt64 {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        // `num_actions` is `usize` on the Rust side → imported as `UInt`,
        // whose checked initializer traps on a negative Int.
        guard numActions >= 0 else {
            throw PlatformWalletError.invalidParameter(
                "numActions must be non-negative, got \(numActions)"
            )
        }
        var fee: UInt64 = 0
        try platform_wallet_shielded_estimate_fee(
            handle,
            kind.rawValue,
            UInt(numActions),
            &fee
        ).check()
        return fee
    }

    /// Shielded → Shielded transfer. Spends notes from `account`
    /// on `walletId` and creates a new note for `recipientRaw43`
    /// (the recipient's raw 43-byte Orchard payment address).
    /// Amount is in credits (1 DASH = 1e11). Heavy CPU work runs
    /// on a detached task so the caller's actor isn't blocked
    /// through the proof build.
    ///
    /// `memo` is an optional UTF-8 text note attached to the
    /// recipient's note. `nil` (or an empty string) means no memo;
    /// a non-empty memo's UTF-8 byte length must be at most 32 or
    /// Rust rejects it. The 36-byte on-chain encoding is done on the
    /// Rust side.
    ///
    /// `resolver` supplies the Orchard spend authority for this one
    /// operation: Rust resolves the mnemonic, derives the spend key,
    /// signs, and scrubs everything before returning — no spend key
    /// stays resident between spends (the launch-time bind is
    /// viewing-grade only).
    ///
    /// Throws `PlatformWalletError.shieldedSpendUnconfirmed` once the
    /// spend may have reached a node and its result is not proved (a
    /// failed result wait, a node's refusal or consensus verdict
    /// included): it may already be on chain or may still execute, so the
    /// caller must NOT retry (the spent notes stay reserved Rust-side
    /// until a later shielded sync resolves them).
    ///
    /// Admitted as an async native operation until its result is delivered:
    /// `shutdown()` waits for it, `deleteWallet` throws `invalidHandle` while
    /// it runs, and a call made once a shutdown has begun throws
    /// `invalidHandle` before any native work.
    public func shieldedTransfer(
        walletId: Data,
        resolver: MnemonicResolver,
        account: UInt32 = 0,
        recipientRaw43: Data,
        amount: UInt64,
        memo: String? = nil
    ) async throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be exactly 32 bytes"
            )
        }
        guard recipientRaw43.count == 43 else {
            throw PlatformWalletError.invalidParameter(
                "recipient must be exactly 43 raw Orchard bytes"
            )
        }
        guard resolver.handle != nil else {
            throw PlatformWalletError.invalidParameter(
                "MnemonicResolver has no handle"
            )
        }

        // Admitted through the detached call and the result delivery, so
        // `shutdown()` cannot destroy the handle under the spend. Cancelling
        // the caller does not release it early: the helper waits for the task.
        let calls = nativeShieldedSpendCalls
        try await withAdmittedNativeOperation("shieldedTransfer") { handle -> Void in
            // MnemonicResolver is passed to Rust via `passUnretained`,
            // so the Rust ctx pointer dangles unless the Swift owner
            // stays alive across the whole FFI call. A bare
            // `_ = resolver` is folklore the optimizer may elide in -O
            // builds; `withExtendedLifetime` is the guaranteed
            // keepalive (same as the identity-create sibling). The
            // handle is read here, inside the admitted body, because the
            // raw pointer is not `Sendable` (the resolver is).
            try withExtendedLifetime(resolver) {
                guard let resolverHandle = resolver.handle else {
                    throw PlatformWalletError.invalidParameter(
                        "MnemonicResolver has no handle"
                    )
                }
                try walletId.withUnsafeBytes { widRaw in
                    guard let widPtr = widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                    else {
                        throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
                    }
                    try recipientRaw43.withUnsafeBytes { recipientRaw in
                        guard let recipientPtr = recipientRaw.baseAddress?
                            .assumingMemoryBound(to: UInt8.self)
                        else {
                            throw PlatformWalletError.invalidParameter(
                                "recipient baseAddress is nil"
                            )
                        }
                        // `nil` / empty → null pointer (no memo); otherwise
                        // pass the text as a C string. Rust validates the
                        // 32-byte limit and does the 36-byte encoding.
                        let send: (UnsafePointer<CChar>?) throws -> Void = { memoCStr in
                            try calls.transfer(
                                handle, widPtr, resolverHandle, account, recipientPtr, amount,
                                memoCStr
                            ).check()
                        }
                        if let memo, !memo.isEmpty {
                            try memo.withCString { try send($0) }
                        } else {
                            try send(nil)
                        }
                    }
                }
            }
        }
    }

    /// Cached Platform-to-shielded capacity for one payment account.
    ///
    /// All values come from the same Rust planner used by `shieldedShield`.
    /// `reason` is non-nil only for a normal zero-capacity result; bad handles,
    /// wallet IDs, and missing payment accounts throw instead.
    public struct ShieldedShieldPreflight: Sendable {
        public let canShield: Bool
        public let accountBalanceCredits: UInt64
        public let usableBalanceCredits: UInt64
        public let feeReserveCredits: UInt64
        public let maxShieldableCredits: UInt64
        public let reason: String?
    }

    /// Return the cached amount a Platform Payment account can currently
    /// shield without signing, proving, broadcasting, or querying DAPI.
    ///
    /// Rust sorts funded addresses lexicographically, excludes the leading
    /// prefix before the first address that can retain the fee reserve, omits
    /// later addresses below the protocol version's minimum input amount, and
    /// truncates the lexicographically earliest usable set to the versioned
    /// maximum input count. The result is executable under that deterministic
    /// wallet policy rather than globally optimized over later balances. A
    /// fragmented/no-capacity account returns `canShield == false` with
    /// meaningful numeric fields and a reason; it is not thrown as an error.
    public func shieldedShieldPreflight(
        walletId: Data,
        paymentAccount: UInt32 = 0
    ) async throws -> ShieldedShieldPreflight {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be exactly 32 bytes"
            )
        }

        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) {
            () -> ShieldedShieldPreflight in
            var out = ShieldedShieldPreflightFFI(
                can_shield: false,
                account_balance_credits: 0,
                usable_balance_credits: 0,
                fee_reserve_credits: 0,
                max_shieldable_credits: 0
            )
            let result = try walletId.withUnsafeBytes { walletIdRaw in
                guard let walletIdPointer = walletIdRaw.baseAddress?
                    .assumingMemoryBound(to: UInt8.self)
                else {
                    throw PlatformWalletError.invalidParameter(
                        "walletId baseAddress is nil"
                    )
                }
                return PlatformWalletResult(
                    platform_wallet_manager_shielded_shield_preflight(
                        handle,
                        walletIdPointer,
                        paymentAccount,
                        &out
                    )
                )
            }
            try result.throwIfError()
            let reason = out.can_shield ? nil : result.message
            return ShieldedShieldPreflight(
                canShield: out.can_shield,
                accountBalanceCredits: out.account_balance_credits,
                usableBalanceCredits: out.usable_balance_credits,
                feeReserveCredits: out.fee_reserve_credits,
                maxShieldableCredits: out.max_shieldable_credits,
                reason: reason
            )
        }.value
    }

    /// Platform → Shielded. Spends credits from a Platform Payment
    /// account on `walletId` into the bound shielded sub-wallet's
    /// pool. Inputs are auto-selected from the account's addresses
    /// in lexicographic Platform-address order until they cover `amount` plus
    /// a conservative on-chain fee buffer; the actual fee is
    /// deducted from input 0 by the network via the shield
    /// transition's fee strategy.
    ///
    /// `addressSigner` is the host-side `KeychainSigner` whose
    /// `.handle` produces ECDSA signatures over each input's
    /// pubkey-hash binding to the Orchard bundle. Borrowed for the
    /// duration of the call.
    ///
    /// Heavy CPU work (Halo 2 proof + per-input signing) runs on a
    /// detached task so the caller's actor isn't blocked.
    ///
    /// Throws `PlatformWalletError.shieldedSpendUnconfirmed` once the
    /// shield may have reached a node and its result is not proved (a
    /// node's refusal or consensus verdict, stale nonce included): it may
    /// already be on chain, so the caller must NOT retry (a retry signs a
    /// second transition). Its activity row stays pending until the
    /// shielded scan sees its note; nothing is reserved wallet-side.
    ///
    /// Admitted until its result is delivered, as [`shieldedTransfer`] is.
    /// It queues behind a Core asset-lock funding in flight on the same
    /// wallet, which can wait for a ChainLock with no deadline, so
    /// `shutdown()` then waits as long and `deleteWallet` stays refused; do
    /// not stop SPV while a funding is in flight (#6003).
    public func shieldedShield(
        walletId: Data,
        shieldedAccount: UInt32 = 0,
        paymentAccount: UInt32 = 0,
        amount: UInt64,
        addressSigner: KeychainSigner
    ) async throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be exactly 32 bytes"
            )
        }

        // Admitted through the call and the result delivery, as in
        // `shieldedTransfer`.
        let calls = nativeShieldedSpendCalls
        try await withAdmittedNativeOperation("shieldedShield") { handle -> Void in
            // KeychainSigner is passed to Rust via `passUnretained`, so
            // the Rust ctx pointer dangles unless the Swift owner stays
            // alive across the whole FFI call — Rust re-materializes it
            // inside the proof worker and signs through it. A bare
            // `_ = addressSigner` is folklore the optimizer may elide in
            // -O builds; `withExtendedLifetime` is the guaranteed
            // keepalive (same as `shieldedTransfer`). The signer handle is
            // read inside the admitted body because the raw pointer is not
            // `Sendable` (the signer is).
            try withExtendedLifetime(addressSigner) {
                let signerHandle = addressSigner.handle
                try walletId.withUnsafeBytes { widRaw in
                    guard let widPtr = widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                    else {
                        throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
                    }
                    try calls.shield(
                        handle, widPtr, shieldedAccount, paymentAccount, amount, signerHandle
                    ).check()
                }
            }
        }
    }

    /// Identity → Shielded. The Type 21 `ShieldFromIdentity` transition:
    /// credits leave `identityId`'s Platform identity balance and enter the
    /// bound shielded sub-wallet's pool directly: no transparent Platform
    /// address and no payment account in between. The new note lands on
    /// `shieldedAccount`'s default Orchard address.
    ///
    /// `identityId` is a 32-byte identity id managed by `walletId`'s wallet.
    /// `identitySigner` is the host-side `KeychainSigner` whose `.handle`
    /// signs the transition with the identity's TRANSFER key: the same
    /// signer the identity credit-transfer path uses. Borrowed for the
    /// duration of the call.
    ///
    /// The identity is debited `amount` plus the metered fee plus the
    /// shielded compute fee (`estimateShieldedFee(kind: .shieldFromIdentity)`).
    /// Returns the identity's proven post-debit balance; the wallet only
    /// confirms on the identity's own balance proof, and any other result
    /// throws `.shieldedSpendUnconfirmed` (do not resubmit).
    ///
    /// Heavy CPU work (Halo 2 proof + the identity signature) runs on a
    /// detached task so the caller's actor isn't blocked.
    ///
    /// Throws `PlatformWalletError.shieldedSpendUnconfirmed` once the
    /// shield may have reached a node and its result is not proved (a
    /// node's refusal or consensus verdict included): it may already be on
    /// chain or may still execute, so the caller must
    /// NOT retry (a retry signs a second transition;
    /// the next sync reconciles the outcome). Like every shield it spends
    /// no notes. An unresolved debit blocks a fresh shield from that identity
    /// with `.shieldedIdentityDebitPending`. Inspect durable records with
    /// `shieldedIdentityDebitRecoveryRecords(walletId:)`; explicit abandonment
    /// requires acknowledging that another payment may be an additional debit.
    ///
    /// Admitted until its balance is returned, as [`shieldedTransfer`] is.
    /// It queues behind an asset-lock funding on the same wallet, with the
    /// shutdown caveat of [`shieldedShield`] (#6003).
    @discardableResult
    public func shieldedShieldFromIdentity(
        walletId: Data,
        shieldedAccount: UInt32 = 0,
        identityId: Data,
        amount: UInt64,
        identitySigner: KeychainSigner
    ) async throws -> UInt64 {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be exactly 32 bytes"
            )
        }
        guard identityId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "identityId must be exactly 32 bytes"
            )
        }

        // Admitted through the call and the delivery of the balance, as in
        // `shieldedTransfer`.
        let calls = nativeShieldedSpendCalls
        return try await withAdmittedNativeOperation("shieldedShieldFromIdentity") {
            handle -> UInt64 in
            // Rust writes the proven post-debit identity balance here on
            // success; it leaves the slot untouched on every error path, so
            // the initializer is the value a failed call keeps (and a call
            // that throws never returns it).
            var newBalance: UInt64 = 0
            // Guaranteed signer keepalive across the whole FFI call :
            // same rationale as `shieldedShield`, which also explains why
            // the signer handle is read in here.
            try withExtendedLifetime(identitySigner) {
                let signerHandle = identitySigner.handle
                try walletId.withUnsafeBytes { widRaw in
                    guard let widPtr = widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                    else {
                        throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
                    }
                    try identityId.withUnsafeBytes { idRaw in
                        guard let idPtr = idRaw.baseAddress?
                            .assumingMemoryBound(to: UInt8.self)
                        else {
                            throw PlatformWalletError.invalidParameter(
                                "identityId baseAddress is nil"
                            )
                        }
                        try calls.shieldFromIdentity(
                            handle, widPtr, shieldedAccount, idPtr, amount,
                            signerHandle, &newBalance
                        ).check()
                    }
                }
            }
            return newBalance
        }
    }

    /// Identity → EXTERNAL Shielded. The Type 21 `ShieldFromIdentity` transition
    /// with the note assigned to `recipientRaw43` (a third-party raw 43-byte
    /// Orchard payment address) instead of the wallet's own default address.
    /// Same signer, confirmation and no-retry rules as
    /// [`shieldedShieldFromIdentity`].
    ///
    /// The identity is debited `amount` plus the metered fee plus the shielded
    /// compute fee (`estimateShieldedFee(kind: .shieldFromIdentity)`).
    /// Returns the identity's proven post-debit balance; any unconfirmed
    /// outcome throws `.shieldedSpendUnconfirmed` (do not resubmit).
    ///
    /// `memo` follows [`shieldedShieldToRecipient`]'s rules: `nil` / empty
    /// means no memo; a non-empty memo's UTF-8 byte length must be at most 32
    /// or the call throws `invalidParameter` (Rust owns the limit).
    ///
    /// Admitted as an async native operation until the result is delivered:
    /// `shutdown()` waits for it, a call made once a shutdown has begun throws
    /// `invalidHandle` before any native work, and the synchronous wallet
    /// create, load and delete throw `invalidHandle` while it is in flight.
    @discardableResult
    public func shieldedShieldFromIdentityToRecipient(
        walletId: Data,
        shieldedAccount: UInt32 = 0,
        identityId: Data,
        recipientRaw43: Data,
        amount: UInt64,
        memo: String? = nil,
        identitySigner: KeychainSigner
    ) async throws -> UInt64 {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be exactly 32 bytes"
            )
        }
        guard identityId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "identityId must be exactly 32 bytes"
            )
        }
        guard recipientRaw43.count == 43 else {
            throw PlatformWalletError.invalidParameter(
                "recipientRaw43 must be exactly 43 bytes"
            )
        }
        // The memo crosses the FFI as a C string, which ends at the first
        // NUL: a memo holding one would be sent cut short.
        guard memo?.utf8.contains(0) != true else {
            throw PlatformWalletError.invalidParameter(
                "memo must not contain NUL characters"
            )
        }

        // Admitted through the detached call and the result delivery, like
        // `withWalletDrainNativeOperation`: `shutdown()` waits for the spend
        // before destroying the handle, and `deleteWallet` is rejected while
        // it runs. Cancelling the caller does not release it early.
        try admitNativeOp("shieldedShieldFromIdentityToRecipient")
        defer { finishNativeOp() }
        let handle = self.handle
        let signerHandle = identitySigner.handle
        let shield = nativeShieldFromIdentityToRecipientCall

        return try await Task.detached(priority: .userInitiated) { () -> UInt64 in
            var newBalance: UInt64 = 0
            try withExtendedLifetime(identitySigner) {
                try walletId.withUnsafeBytes { widRaw in
                    guard let widPtr = widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                    else {
                        throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
                    }
                    try identityId.withUnsafeBytes { idRaw in
                        guard let idPtr = idRaw.baseAddress?
                            .assumingMemoryBound(to: UInt8.self)
                        else {
                            throw PlatformWalletError.invalidParameter(
                                "identityId baseAddress is nil"
                            )
                        }
                        try recipientRaw43.withUnsafeBytes { rcpRaw in
                            guard let rcpPtr = rcpRaw.baseAddress?
                                .assumingMemoryBound(to: UInt8.self)
                            else {
                                throw PlatformWalletError.invalidParameter(
                                    "recipientRaw43 baseAddress is nil"
                                )
                            }
                            // `nil` / empty → null pointer (no memo); otherwise
                            // pass the text as a C string — Rust validates the
                            // 32-byte limit and does the 36-byte encoding.
                            let send: (UnsafePointer<CChar>?) throws -> Void = { memoCStr in
                                try shield(
                                    handle, widPtr, shieldedAccount, idPtr, rcpPtr, amount,
                                    memoCStr, signerHandle, &newBalance
                                ).check()
                            }
                            if let memo, !memo.isEmpty {
                                try memo.withCString { try send($0) }
                            } else {
                                try send(nil)
                            }
                        }
                    }
                }
            }
            return newBalance
        }.value
    }

    /// Platform → EXTERNAL Shielded. The Type 15 shield with the note
    /// assigned to `recipientRaw43` (a third-party raw 43-byte Orchard
    /// payment address — same shape [`shieldedTransfer`] takes) instead
    /// of the wallet's own default address. Input selection, fees, and
    /// error shapes are identical to [`shieldedShield`]; the wallet
    /// still needs a bound shielded sub-wallet at `shieldedAccount`
    /// because the send is OVK-encrypted to (and its activity recorded
    /// under) that account — that is how the wallet's own scan later
    /// shows it as sent history.
    ///
    /// The recipient must actually be a third party: an address the
    /// account's own keys recognize (default or diversified) is
    /// rejected by Rust — self-shields go through [`shieldedShield`].
    ///
    /// `memo` follows [`shieldedTransfer`]'s rules: `nil` / empty means
    /// no memo; a non-empty memo's UTF-8 byte length must be at most 32
    /// or Rust rejects it. The 36-byte on-chain encoding is done on the
    /// Rust side.
    ///
    /// Throws `PlatformWalletError.shieldedSpendUnconfirmed` once the
    /// shield may have reached a node and its result is not proved (a
    /// node's refusal or consensus verdict, stale nonce included): it may
    /// already be on chain, so the caller must NOT retry (a retry signs a
    /// second transition). Its activity row stays pending until the
    /// shielded scan sees its note; nothing is reserved wallet-side.
    ///
    /// Admitted until its result is delivered, as [`shieldedTransfer`] is.
    /// It queues behind an asset-lock funding on the same wallet, with the
    /// shutdown caveat of [`shieldedShield`] (#6003).
    public func shieldedShieldToRecipient(
        walletId: Data,
        shieldedAccount: UInt32 = 0,
        paymentAccount: UInt32 = 0,
        recipientRaw43: Data,
        amount: UInt64,
        memo: String? = nil,
        addressSigner: KeychainSigner
    ) async throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be exactly 32 bytes"
            )
        }
        guard recipientRaw43.count == 43 else {
            throw PlatformWalletError.invalidParameter(
                "recipient must be exactly 43 raw Orchard bytes"
            )
        }

        // Admitted through the call and the result delivery, as in
        // `shieldedTransfer`.
        let calls = nativeShieldedSpendCalls
        try await withAdmittedNativeOperation("shieldedShieldToRecipient") { handle -> Void in
            // Guaranteed signer keepalive across the whole FFI call :
            // same rationale as `shieldedShield`, which also explains why
            // the signer handle is read in here.
            try withExtendedLifetime(addressSigner) {
                let signerHandle = addressSigner.handle
                try walletId.withUnsafeBytes { widRaw in
                    guard let widPtr = widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                    else {
                        throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
                    }
                    try recipientRaw43.withUnsafeBytes { recipientRaw in
                        guard let recipientPtr = recipientRaw.baseAddress?
                            .assumingMemoryBound(to: UInt8.self)
                        else {
                            throw PlatformWalletError.invalidParameter(
                                "recipient baseAddress is nil"
                            )
                        }
                        // `nil` / empty → null pointer (no memo); otherwise
                        // pass the text as a C string — Rust validates the
                        // 32-byte limit and does the 36-byte encoding.
                        let send: (UnsafePointer<CChar>?) throws -> Void = { memoCStr in
                            try calls.shieldToRecipient(
                                handle, widPtr, shieldedAccount, paymentAccount,
                                recipientPtr, amount, memoCStr, signerHandle
                            ).check()
                        }
                        if let memo, !memo.isEmpty {
                            try memo.withCString { try send($0) }
                        } else {
                            try send(nil)
                        }
                    }
                }
            }
        }
    }

    /// Shielded → Platform unshield. Spends notes from `walletId`'s
    /// shielded balance and credits `toPlatformAddress`, a bech32m
    /// string (`"dash1…"` on mainnet, `"tdash1…"` on testnet). Rust
    /// parses and network-checks the address; hosts don't have to
    /// hand-roll the bincode storage variant tag.
    ///
    /// `resolver` supplies the per-operation Orchard spend authority
    /// (see [`shieldedTransfer`]).
    ///
    /// Throws `PlatformWalletError.shieldedSpendUnconfirmed` once the
    /// spend may have reached a node and its result is not proved (a
    /// failed result wait, a node's refusal or consensus verdict
    /// included): it may already be on chain or may still execute, so the
    /// caller must NOT retry (the spent notes stay reserved Rust-side
    /// until a later shielded sync resolves them).
    ///
    /// Admitted until its result is delivered, as [`shieldedTransfer`] is.
    public func shieldedUnshield(
        walletId: Data,
        resolver: MnemonicResolver,
        account: UInt32 = 0,
        toPlatformAddress: String,
        amount: UInt64
    ) async throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be exactly 32 bytes"
            )
        }
        guard !toPlatformAddress.isEmpty else {
            throw PlatformWalletError.invalidParameter(
                "toPlatformAddress is empty"
            )
        }
        guard resolver.handle != nil else {
            throw PlatformWalletError.invalidParameter(
                "MnemonicResolver has no handle"
            )
        }

        // Admitted through the call and the result delivery, as in
        // `shieldedTransfer`.
        let calls = nativeShieldedSpendCalls
        try await withAdmittedNativeOperation("shieldedUnshield") { handle -> Void in
            // Guaranteed resolver keepalive across the FFI call :
            // same rationale as `shieldedTransfer`.
            try withExtendedLifetime(resolver) {
                guard let resolverHandle = resolver.handle else {
                    throw PlatformWalletError.invalidParameter(
                        "MnemonicResolver has no handle"
                    )
                }
                try walletId.withUnsafeBytes { widRaw in
                    guard let widPtr = widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                    else {
                        throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
                    }
                    try toPlatformAddress.withCString { addrCStr in
                        try calls.unshield(
                            handle, widPtr, resolverHandle, account, addrCStr, amount
                        ).check()
                    }
                }
            }
        }
    }

    /// Shielded → an EXISTING identity's balance. The Type 22
    /// `IdentityTopUpFromShieldedPool` transition: notes from
    /// `walletId`'s shielded balance on `account` are spent so that
    /// `identityId` receives `amount` credits. The flat pool-paid fee
    /// (`estimateShieldedFee(kind: .identityTopUpFromPool)`) is spent
    /// from the notes on top of `amount`.
    ///
    /// The identity only has to exist on Platform; it does NOT have to
    /// be managed by this wallet, and no identity-side signer is
    /// involved: the only spend authority is the Orchard one `resolver`
    /// supplies per operation (see [`shieldedTransfer`]), exactly as for
    /// [`shieldedUnshield`].
    ///
    /// Throws `PlatformWalletError.shieldedSpendUnconfirmed` once the
    /// spend may have reached a node and its result is not proved (a
    /// failed result wait, a node's refusal or consensus verdict
    /// included): it may already be on chain or may still execute, so the
    /// caller must NOT retry (the spent notes stay reserved Rust-side
    /// until a later shielded sync resolves them).
    ///
    /// Admitted until its result is delivered, as [`shieldedTransfer`] is.
    /// It queues behind an asset-lock funding on the same wallet, with the
    /// shutdown caveat of [`shieldedShield`] (#6003).
    public func shieldedIdentityTopUpFromPool(
        walletId: Data,
        resolver: MnemonicResolver,
        account: UInt32 = 0,
        identityId: Data,
        amount: UInt64
    ) async throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be exactly 32 bytes"
            )
        }
        guard identityId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "identityId must be exactly 32 bytes"
            )
        }
        guard resolver.handle != nil else {
            throw PlatformWalletError.invalidParameter(
                "MnemonicResolver has no handle"
            )
        }

        // Admitted through the call and the result delivery, as in
        // `shieldedTransfer`.
        let calls = nativeShieldedSpendCalls
        try await withAdmittedNativeOperation("shieldedIdentityTopUpFromPool") { handle -> Void in
            // Guaranteed resolver keepalive across the FFI call :
            // same rationale as `shieldedTransfer`.
            try withExtendedLifetime(resolver) {
                guard let resolverHandle = resolver.handle else {
                    throw PlatformWalletError.invalidParameter(
                        "MnemonicResolver has no handle"
                    )
                }
                try walletId.withUnsafeBytes { widRaw in
                    guard let widPtr = widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                    else {
                        throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
                    }
                    try identityId.withUnsafeBytes { idRaw in
                        guard let idPtr = idRaw.baseAddress?
                            .assumingMemoryBound(to: UInt8.self)
                        else {
                            throw PlatformWalletError.invalidParameter(
                                "identityId baseAddress is nil"
                            )
                        }
                        try calls.identityTopUpFromPool(
                            handle, widPtr, resolverHandle, account, idPtr, amount
                        ).check()
                    }
                }
            }
        }
    }

    /// Shielded → a token's shielded pool. The Type 28
    /// `TokenPurchaseFromShieldedPool` transition: notes from `walletId`'s
    /// shielded balance on `account` pay `totalAgreedPrice` credits plus the
    /// pool-paid fee (`estimateShieldedFee(kind: .tokenPurchaseFromPool)`),
    /// and `tokenCount` tokens of the token at `tokenPosition` of
    /// `contractId` are minted as a note to the wallet's own address in the
    /// token's shielded pool. Rust checks the token, the network and the
    /// notes, builds, broadcasts, records the activity and requests a
    /// shielded sync to look for the purchase.
    ///
    /// Returning means submitted, not confirmed: Platform accepted the
    /// purchase, but its answer does not show that it executed, so its
    /// activity row stays pending (and its notes reserved) until a shielded
    /// sync confirms it once it lands. Do not retry it.
    ///
    /// `totalAgreedPrice` is the price the buyer agreed to; Drive refuses
    /// the purchase unless it covers the token's configured price for
    /// `tokenCount`. Rust watches the token's pool for the wallet, so the
    /// shielded sync that confirms the purchase also finds the bought tokens
    /// (`shieldedTokenPoolBalances(walletId:)`).
    ///
    /// `resolver` supplies the per-operation Orchard spend authority (see
    /// [`shieldedTransfer`]). No identity is involved.
    ///
    /// Throws `PlatformWalletError.shieldedSpendUnconfirmed` once the
    /// spend may have reached a node and its result is not proved (a
    /// failed result wait, a node's refusal or consensus verdict
    /// included): it may already be on chain or may still execute, so the
    /// caller must NOT retry (the spent notes stay reserved Rust-side
    /// until a shielded sync sees the purchase land).
    ///
    /// Admitted until its result is delivered, as [`shieldedTransfer`] is.
    public func shieldedTokenPurchaseFromPool(
        walletId: Data,
        resolver: MnemonicResolver,
        account: UInt32 = 0,
        contractId: Data,
        tokenPosition: UInt16,
        tokenCount: UInt64,
        totalAgreedPrice: UInt64
    ) async throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be exactly 32 bytes"
            )
        }
        guard contractId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "contractId must be exactly 32 bytes"
            )
        }
        guard resolver.handle != nil else {
            throw PlatformWalletError.invalidParameter(
                "MnemonicResolver has no handle"
            )
        }

        // Admitted through the call and the result delivery, as in
        // `shieldedTransfer`.
        let calls = nativeShieldedSpendCalls
        try await withAdmittedNativeOperation("shieldedTokenPurchaseFromPool") { handle -> Void in
            // Guaranteed resolver keepalive across the FFI call:
            // same rationale as `shieldedTransfer`.
            try withExtendedLifetime(resolver) {
                guard let resolverHandle = resolver.handle else {
                    throw PlatformWalletError.invalidParameter(
                        "MnemonicResolver has no handle"
                    )
                }
                try walletId.withUnsafeBytes { widRaw in
                    guard let widPtr = widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                    else {
                        throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
                    }
                    try contractId.withUnsafeBytes { cidRaw in
                        guard let cidPtr = cidRaw.baseAddress?
                            .assumingMemoryBound(to: UInt8.self)
                        else {
                            throw PlatformWalletError.invalidParameter(
                                "contractId baseAddress is nil"
                            )
                        }
                        try calls.tokenPurchaseFromPool(
                            handle,
                            widPtr,
                            resolverHandle,
                            account,
                            cidPtr,
                            tokenPosition,
                            tokenCount,
                            totalAgreedPrice
                        ).check()
                    }
                }
            }
        }
    }

    /// A token's shielded pool → an Orchard address in the same pool. The
    /// Type 26 `TokenShieldedTransferWithShieldedFee` transition: `amount`
    /// of the token at `tokenContractPosition` of `dataContractId`, in the
    /// token's smallest unit, leaves `walletId`'s notes on `account` in the
    /// token's shielded pool for `recipientRaw43` (the same raw 43-byte
    /// Orchard address `shieldedTransfer` takes), the change goes back to
    /// the account, and the fee is paid from the account's notes in the
    /// credit pool. Rust checks the token, the network and both note sets,
    /// builds, broadcasts, records the activity (kind tag 11) and requests a
    /// shielded sync when the transfer may be on the network.
    ///
    /// Returning means submitted, not confirmed: Platform accepted the
    /// transfer, but its answer does not show that it executed, so its
    /// activity row stays pending (and its notes in both pools reserved,
    /// `ShieldedTokenPoolBalance.reservedBalance`) until a shielded sync
    /// confirms it once it lands. Do not retry it.
    ///
    /// `resolver` supplies the per-operation Orchard spend authority (see
    /// [`shieldedTransfer`]). No identity is involved.
    ///
    /// Throws `PlatformWalletError.shieldedSpendUnconfirmed` once the
    /// spend may have reached a node and its result is not proved: it may
    /// already be on chain or may still execute, so the caller must NOT
    /// retry. Other refusals (not enough tokens or fee notes, too many
    /// notes, no recorded anchor yet, a token without a shielded pool)
    /// carry Rust's reason in the thrown error.
    public func shieldedTokenTransfer(
        walletId: Data,
        resolver: MnemonicResolver,
        account: UInt32 = 0,
        dataContractId: Data,
        tokenContractPosition: UInt16,
        recipientRaw43: Data,
        amount: UInt64
    ) async throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be exactly 32 bytes"
            )
        }
        guard dataContractId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "dataContractId must be exactly 32 bytes"
            )
        }
        guard recipientRaw43.count == 43 else {
            throw PlatformWalletError.invalidParameter(
                "recipient must be exactly 43 raw Orchard bytes"
            )
        }
        guard resolver.handle != nil else {
            throw PlatformWalletError.invalidParameter(
                "MnemonicResolver has no handle"
            )
        }

        // Admitted from before the FFI call until its result is converted, so
        // `shutdown()` waits for a transfer already past this point instead
        // of destroying the handle under it.
        try await withAdmittedNativeOperation("shieldedTokenTransfer") { handle -> Void in
            // Guaranteed resolver keepalive across the FFI call:
            // same rationale as `shieldedTransfer`. The handle is read here,
            // inside the admitted body, because the raw pointer is not
            // `Sendable` (the resolver is).
            try withExtendedLifetime(resolver) {
                guard let resolverHandle = resolver.handle else {
                    throw PlatformWalletError.invalidParameter(
                        "MnemonicResolver has no handle"
                    )
                }
                try walletId.withUnsafeBytes { widRaw in
                    guard let widPtr = widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                    else {
                        throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
                    }
                    try dataContractId.withUnsafeBytes { cidRaw in
                        guard let cidPtr = cidRaw.baseAddress?
                            .assumingMemoryBound(to: UInt8.self)
                        else {
                            throw PlatformWalletError.invalidParameter(
                                "dataContractId baseAddress is nil"
                            )
                        }
                        try recipientRaw43.withUnsafeBytes { recipientRaw in
                            guard let recipientPtr = recipientRaw.baseAddress?
                                .assumingMemoryBound(to: UInt8.self)
                            else {
                                throw PlatformWalletError.invalidParameter(
                                    "recipient baseAddress is nil"
                                )
                            }
                            try platform_wallet_manager_shielded_token_transfer(
                                handle,
                                widPtr,
                                resolverHandle,
                                account,
                                cidPtr,
                                tokenContractPosition,
                                recipientPtr,
                                amount
                            ).check()
                        }
                    }
                }
            }
        }
    }

    /// A token's shielded pool → an existing identity's token balance. The
    /// Type 27 `TokenUnshieldWithShieldedFee` transition: `amount` of the
    /// token at `tokenContractPosition` of `dataContractId`, in the token's
    /// smallest unit, leaves `walletId`'s notes on `account` in the token's
    /// shielded pool for the token balance of `recipientIdentityId` (32
    /// bytes), the change goes back to the account, and the fee is paid from
    /// the account's notes in the credit pool. Rust checks, selects, builds,
    /// broadcasts, records the activity (kind tag 12) and requests a shielded
    /// sync when the unshield may be on the network.
    ///
    /// Returning means submitted, not confirmed, as for
    /// `shieldedTokenTransfer`: the activity row stays pending (and the notes
    /// in both pools reserved) until a shielded sync confirms it. Do not
    /// retry it. `resolver` supplies the per-operation Orchard spend
    /// authority (see [`shieldedTransfer`]).
    ///
    /// Throws `PlatformWalletError.shieldedSpendUnconfirmed` once the spend
    /// may have reached a node and its result is not proved: the caller must
    /// NOT retry. Other refusals carry Rust's reason in the thrown error.
    public func shieldedTokenUnshield(
        walletId: Data,
        resolver: MnemonicResolver,
        account: UInt32 = 0,
        dataContractId: Data,
        tokenContractPosition: UInt16,
        recipientIdentityId: Data,
        amount: UInt64
    ) async throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be exactly 32 bytes"
            )
        }
        guard dataContractId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "dataContractId must be exactly 32 bytes"
            )
        }
        guard recipientIdentityId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "recipientIdentityId must be exactly 32 bytes"
            )
        }
        guard resolver.handle != nil else {
            throw PlatformWalletError.invalidParameter(
                "MnemonicResolver has no handle"
            )
        }

        // Admitted from before the FFI call until its result is converted, so
        // `shutdown()` waits for an unshield already past this point instead
        // of destroying the handle under it.
        try await withAdmittedNativeOperation("shieldedTokenUnshield") { handle -> Void in
            // Guaranteed resolver keepalive across the FFI call:
            // same rationale as `shieldedTransfer`. The handle is read here,
            // inside the admitted body, because the raw pointer is not
            // `Sendable` (the resolver is).
            try withExtendedLifetime(resolver) {
                guard let resolverHandle = resolver.handle else {
                    throw PlatformWalletError.invalidParameter(
                        "MnemonicResolver has no handle"
                    )
                }
                try walletId.withUnsafeBytes { widRaw in
                    guard let widPtr = widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                    else {
                        throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
                    }
                    try dataContractId.withUnsafeBytes { cidRaw in
                        guard let cidPtr = cidRaw.baseAddress?
                            .assumingMemoryBound(to: UInt8.self)
                        else {
                            throw PlatformWalletError.invalidParameter(
                                "dataContractId baseAddress is nil"
                            )
                        }
                        try recipientIdentityId.withUnsafeBytes { ridRaw in
                            guard let ridPtr = ridRaw.baseAddress?
                                .assumingMemoryBound(to: UInt8.self)
                            else {
                                throw PlatformWalletError.invalidParameter(
                                    "recipientIdentityId baseAddress is nil"
                                )
                            }
                            try platform_wallet_manager_shielded_token_unshield(
                                handle,
                                widPtr,
                                resolverHandle,
                                account,
                                cidPtr,
                                tokenContractPosition,
                                ridPtr,
                                amount
                            ).check()
                        }
                    }
                }
            }
        }
    }

    /// What a token pool spend of `kind` moving `amount` (the token's
    /// smallest unit) of the token at `tokenContractPosition` of
    /// `dataContractId` would take from `walletId`'s notes on `account`, as
    /// Rust chooses them: its fee in credits and the token and credit pool
    /// note counts. Local state only: no network read, nothing reserved, no
    /// mnemonic. Throws Rust's refusal (a zero amount, too few tokens, too few
    /// credits for the fee, too many notes) exactly as the spend would refuse
    /// on these notes. Async because it waits for a shielded sync pass that
    /// holds the store.
    public func shieldedTokenSpendPreview(
        walletId: Data,
        account: UInt32 = 0,
        dataContractId: Data,
        tokenContractPosition: UInt16,
        kind: ShieldedTokenPoolSpendKind,
        amount: UInt64
    ) async throws -> ShieldedTokenSpendPreview {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be exactly 32 bytes"
            )
        }
        guard dataContractId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "dataContractId must be exactly 32 bytes"
            )
        }

        let kindByte = kind.rawValue
        // Admitted through the FFI call and the result conversion, as the
        // spends are, so `shutdown()` waits for a preview in flight.
        return try await withAdmittedNativeOperation("shieldedTokenSpendPreview") {
            handle -> ShieldedTokenSpendPreview in
            var fee: UInt64 = 0
            var tokenNotes: UInt32 = 0
            var feeNotes: UInt32 = 0
            try walletId.withUnsafeBytes { widRaw in
                guard let widPtr = widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                else {
                    throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
                }
                try dataContractId.withUnsafeBytes { cidRaw in
                    guard let cidPtr = cidRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                    else {
                        throw PlatformWalletError.invalidParameter(
                            "dataContractId baseAddress is nil"
                        )
                    }
                    try platform_wallet_manager_shielded_token_spend_preview(
                        handle,
                        widPtr,
                        account,
                        cidPtr,
                        tokenContractPosition,
                        kindByte,
                        amount,
                        &fee,
                        &tokenNotes,
                        &feeNotes
                    ).check()
                }
            }
            return ShieldedTokenSpendPreview(tokenNotes: tokenNotes, feeNotes: feeNotes, fee: fee)
        }
    }

    /// Whether a token's direct purchase can be paid from the shielded pool,
    /// as Rust reads the token's verified configuration
    /// (`platform_wallet_manager_shielded_token_purchase_availability`).
    public enum ShieldedTokenPurchaseAvailability: UInt8 {
        /// The token has a shielded pool to mint the bought tokens into.
        case available = 0
        /// The token's configuration has no shielded pool.
        case noShieldedPool = 1
        /// The token's configuration could not be read; asking again may
        /// succeed.
        case configurationUnreadable = 2
    }

    /// Whether the token at `tokenPosition` of `contractId` can be bought
    /// with `shieldedTokenPurchaseFromPool`, from the same verified read of
    /// its configuration that purchase refuses on.
    public func shieldedTokenPurchaseAvailability(
        contractId: Data,
        tokenPosition: UInt16
    ) async throws -> ShieldedTokenPurchaseAvailability {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        guard contractId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "contractId must be exactly 32 bytes"
            )
        }

        let handle = self.handle
        let code = try await Task.detached(priority: .userInitiated) { () -> UInt8 in
            var out: UInt8 = 0
            try contractId.withUnsafeBytes { cidRaw in
                guard let cidPtr = cidRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                else {
                    throw PlatformWalletError.invalidParameter("contractId baseAddress is nil")
                }
                try platform_wallet_manager_shielded_token_purchase_availability(
                    handle,
                    cidPtr,
                    tokenPosition,
                    &out
                ).check()
            }
            return out
        }.value
        guard let availability = ShieldedTokenPurchaseAvailability(rawValue: code) else {
            throw PlatformWalletError.invalidParameter(
                "unknown token purchase availability \(code)"
            )
        }
        return availability
    }

    /// Watch the shielded pool of the token at `tokenContractPosition` of
    /// `dataContractId` for `walletId`, so every later shielded sync scans it
    /// for the wallet's notes. Rust reads the token's configuration with
    /// proofs first and refuses a token without a shielded pool, or one whose
    /// configuration cannot be read (asking again may succeed); the thrown
    /// error carries Rust's reason. Watching a watched pool again changes
    /// nothing. Returns the token's 32-byte id.
    ///
    /// The new pool is scanned by the next shielded sync
    /// (`syncShieldedNow()` or the background loop).
    public func shieldedTokenPoolWatch(
        walletId: Data,
        dataContractId: Data,
        tokenContractPosition: UInt16
    ) async throws -> Data {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be exactly 32 bytes"
            )
        }
        guard dataContractId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "dataContractId must be exactly 32 bytes"
            )
        }

        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) { () -> Data in
            var outTokenId = [UInt8](repeating: 0, count: 32)
            try walletId.withUnsafeBytes { widRaw in
                guard let widPtr = widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                else {
                    throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
                }
                try dataContractId.withUnsafeBytes { cidRaw in
                    guard let cidPtr = cidRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                    else {
                        throw PlatformWalletError.invalidParameter(
                            "dataContractId baseAddress is nil"
                        )
                    }
                    try outTokenId.withUnsafeMutableBufferPointer { outBp in
                        try platform_wallet_manager_shielded_token_pool_watch(
                            handle,
                            widPtr,
                            cidPtr,
                            tokenContractPosition,
                            outBp.baseAddress!
                        ).check()
                    }
                }
            }
            return Data(outTokenId)
        }.value
    }

    /// Stop watching `tokenId`'s shielded pool for `walletId`; Rust forgets
    /// the wallet's notes and scan state there. Unwatching a pool not watched
    /// changes nothing. Async because it waits for a shielded sync pass that
    /// holds the store.
    public func shieldedTokenPoolUnwatch(walletId: Data, tokenId: Data) async throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        guard walletId.count == 32, tokenId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId and tokenId must be exactly 32 bytes"
            )
        }

        let handle = self.handle
        try await Task.detached(priority: .userInitiated) {
            try walletId.withUnsafeBytes { widRaw in
                guard let widPtr = widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                else {
                    throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
                }
                try tokenId.withUnsafeBytes { tidRaw in
                    guard let tidPtr = tidRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                    else {
                        throw PlatformWalletError.invalidParameter("tokenId baseAddress is nil")
                    }
                    try platform_wallet_manager_shielded_token_pool_unwatch(
                        handle,
                        widPtr,
                        tidPtr
                    ).check()
                }
            }
        }.value
    }

    /// `walletId`'s tokens in every shielded pool it watches, one entry per
    /// watched pool and bound account, in Rust's order, from the local scan
    /// alone (no network read). Refresh them with a shielded sync
    /// (`syncShieldedNow()`), which scans the watched pools. Async because it
    /// waits for a shielded sync pass that holds the store.
    public func shieldedTokenPoolBalances(
        walletId: Data
    ) async throws -> [ShieldedTokenPoolBalance] {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be exactly 32 bytes"
            )
        }

        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) {
            try walletId.withUnsafeBytes { raw in
                guard let widPtr = raw.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                    throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
                }
                var rows: UnsafeMutablePointer<ShieldedTokenPoolBalanceFFI>?
                var count: UInt = 0
                let result = platform_wallet_manager_shielded_token_pool_balances(
                    handle,
                    widPtr,
                    &rows,
                    &count
                )
                // The free function accepts null and 0, and Rust resets both
                // before any fallible work, so this is safe on every path.
                defer { platform_wallet_manager_shielded_token_pool_balances_free(rows, count) }
                try result.check()
                guard let rows else { return [ShieldedTokenPoolBalance]() }
                return UnsafeBufferPointer(start: rows, count: Int(count)).map {
                    ShieldedTokenPoolBalance(ffi: $0)
                }
            }
        }.value
    }

    /// Shielded → Core L1 withdraw. Spends notes from `walletId`'s
    /// shielded balance and creates an L1 withdrawal to
    /// `toCoreAddress` (Base58Check string). `coreFeePerByte` is
    /// the L1 fee rate in duffs/byte (`1` is the dashmate default).
    ///
    /// `resolver` supplies the per-operation Orchard spend authority
    /// (see [`shieldedTransfer`]).
    ///
    /// Throws `PlatformWalletError.shieldedSpendUnconfirmed` once the
    /// spend may have reached a node and its result is not proved (a
    /// failed result wait, a node's refusal or consensus verdict
    /// included): it may already be on chain or may still execute, so the
    /// caller must NOT retry (the spent notes stay reserved Rust-side
    /// until a later shielded sync resolves them).
    ///
    /// Admitted until its result is delivered, as [`shieldedTransfer`] is.
    public func shieldedWithdraw(
        walletId: Data,
        resolver: MnemonicResolver,
        account: UInt32 = 0,
        toCoreAddress: String,
        amount: UInt64,
        coreFeePerByte: UInt32 = 1
    ) async throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be exactly 32 bytes"
            )
        }
        guard resolver.handle != nil else {
            throw PlatformWalletError.invalidParameter(
                "MnemonicResolver has no handle"
            )
        }

        // Admitted through the call and the result delivery, as in
        // `shieldedTransfer`.
        let calls = nativeShieldedSpendCalls
        try await withAdmittedNativeOperation("shieldedWithdraw") { handle -> Void in
            // Guaranteed resolver keepalive across the FFI call :
            // same rationale as `shieldedTransfer`.
            try withExtendedLifetime(resolver) {
                guard let resolverHandle = resolver.handle else {
                    throw PlatformWalletError.invalidParameter(
                        "MnemonicResolver has no handle"
                    )
                }
                try walletId.withUnsafeBytes { widRaw in
                    guard let widPtr = widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                    else {
                        throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
                    }
                    try toCoreAddress.withCString { addrCStr in
                        try calls.withdraw(
                            handle, widPtr, resolverHandle, account, addrCStr, amount,
                            coreFeePerByte
                        ).check()
                    }
                }
            }
        }
    }

    /// Shielded → new identity (Type 20). Spends notes from
    /// `walletId`'s shielded balance to fund a brand-new Platform
    /// identity. The whole `denomination` (a member of the versioned
    /// exit-denomination set, in credits) leaves the pool and the
    /// metered fee is taken from it, so the new identity is created
    /// holding `denomination - totalFee`; any excess re-enters the
    /// pool as a change note.
    ///
    /// `identityPubkeys` is the new identity's key set (the first row
    /// should be the MASTER key). `identitySigner` is the host-side
    /// `KeychainSigner` whose `.handle` produces each key's
    /// proof-of-possession signature; the Orchard spend authority is
    /// re-derived per operation via `resolver` (see
    /// [`shieldedTransfer`]). Returns the 32-byte new identity id
    /// (`double_sha256(sorted nullifiers)`).
    ///
    /// `identityIndex` is the DIP-9 identity-registration slot the new
    /// identity occupies. On a successful broadcast the Rust wallet
    /// registers the proof-verified identity at this slot in its local
    /// `IdentityManager` (mirroring address-funded registration), which
    /// drives the persister callbacks that create the app's identity
    /// row. This wrapper only marshals it across the FFI.
    ///
    /// `sendToAddressOnCreationFailure` is the REQUIRED fallback
    /// platform address as raw `PlatformAddress` storage bytes (21
    /// bytes: 1-byte variant tag + 20-byte hash, the encoding
    /// `PlatformAddress.toBytes()` produces). If creation fails a
    /// stateful check (a public-key hash already registered to another
    /// identity) the spend is still finalized and the value is credited
    /// to this address minus a penalty. It is bound into the transition
    /// sighash, so it cannot be redirected after signing.
    ///
    /// Heavy CPU work (Halo 2 proof + per-key signing) runs on a
    /// detached task so the caller's actor isn't blocked.
    ///
    /// Admitted until the identity id or its error is delivered, as
    /// [`shieldedTransfer`] is.
    public func shieldedIdentityCreateFromPool(
        walletId: Data,
        resolver: MnemonicResolver,
        account: UInt32 = 0,
        identityIndex: UInt32,
        identityPubkeys: [ManagedPlatformWallet.IdentityPubkey],
        denomination: UInt64,
        sendToAddressOnCreationFailure: Data,
        identitySigner: KeychainSigner
    ) async throws -> Data {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be exactly 32 bytes"
            )
        }
        guard !identityPubkeys.isEmpty else {
            throw PlatformWalletError.invalidParameter(
                "identityPubkeys is empty"
            )
        }
        guard sendToAddressOnCreationFailure.count == 21 else {
            throw PlatformWalletError.invalidParameter(
                "sendToAddressOnCreationFailure must be exactly 21 PlatformAddress bytes"
            )
        }
        guard resolver.handle != nil else {
            throw PlatformWalletError.invalidParameter(
                "MnemonicResolver has no handle"
            )
        }

        // Admitted through the call and the delivery of the identity id or
        // its typed error, as in `shieldedTransfer`.
        let fallbackAddressBytes = sendToAddressOnCreationFailure
        let calls = nativeShieldedSpendCalls

        return try await withAdmittedNativeOperation("shieldedIdentityCreateFromPool") {
            handle -> Data in
            var outIdentityId: (
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
            ) = (
                0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0
            )

            // Pin every pubkey buffer simultaneously (and the
            // wallet-id bytes), then hand the pinned
            // `[IdentityPubkeyFFI]` rows + signer handle to the FFI.
            // Reuses the same marshalling helper the address-funded
            // registration path uses so the two can't drift.
            let pubkeyBuffers: [Data] = identityPubkeys.map { $0.pubkeyBytes }
            // KeychainSigner and MnemonicResolver are passed to Rust via `passUnretained`, so the
            // Rust ctx pointers dangle unless the Swift owners are kept alive across the FFI call.
            // `_ = identitySigner` is folklore that the optimizer may elide in -O builds;
            // `withExtendedLifetime` is the guaranteed keepalive (matches this module's
            // signer-lifetime guidance). Both raw handles are read in here
            // because the pointers are not `Sendable` (their owners are).
            let result = try withExtendedLifetime((identitySigner, resolver)) {
                guard let resolverHandle = resolver.handle else {
                    throw PlatformWalletError.invalidParameter(
                        "MnemonicResolver has no handle"
                    )
                }
                let identitySignerHandle = identitySigner.handle
                return try walletId.withUnsafeBytes { widRaw -> PlatformWalletFFIResult in
                    guard let widPtr = widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                    else {
                        throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
                    }
                    // Pin the 21-byte fallback `PlatformAddress` bytes for the whole FFI call so the
                    // pointer handed to Rust stays valid (validated `== 21` above).
                    return try fallbackAddressBytes.withUnsafeBytes {
                        fallbackRaw -> PlatformWalletFFIResult in
                        guard let fallbackPtr = fallbackRaw.baseAddress?.assumingMemoryBound(
                            to: UInt8.self
                        ) else {
                            throw PlatformWalletError.invalidParameter(
                                "sendToAddressOnCreationFailure baseAddress is nil"
                            )
                        }
                        return ManagedPlatformWallet.withPubkeyFFIArray(
                            identityPubkeys,
                            buffers: pubkeyBuffers
                        ) { ffiRowsPtr, ffiRowsCount in
                            calls.identityCreateFromPool(
                                handle,
                                widPtr,
                                resolverHandle,
                                account,
                                identityIndex,
                                ffiRowsPtr,
                                UInt(ffiRowsCount),
                                denomination,
                                fallbackPtr,
                                identitySignerHandle,
                                &outIdentityId
                            )
                        }
                    }
                }
            }

            // Wrap the FFI result EXACTLY ONCE so its Rust-owned message is
            // freed once in `deinit` (don't also call `result.check()`, which
            // would construct a second wrapper over the same struct and
            // double-free). Inspect the typed code directly:
            //   - success: return the derived id.
            //   - unconfirmed: the transition may have reached a node and its
            //     result is not proved; Rust filled `outIdentityId` with the
            //     derived id on THIS code (and only this code). Throw the typed
            //     `ShieldedIdentityCreateUnconfirmedError` so the caller holds
            //     the slot instead of treating it as failed.
            //   - any other non-success: throw the regular typed error.
            let wrapped = PlatformWalletResult(result)
            switch wrapped.code {
            case .success:
                return withUnsafeBytes(of: outIdentityId) { Data($0) }
            case .errorShieldedBroadcastUnconfirmed:
                let identityId = withUnsafeBytes(of: outIdentityId) { Data($0) }
                throw ShieldedIdentityCreateUnconfirmedError(
                    identityId: identityId,
                    message: wrapped.message ?? "shielded identity-create broadcast unconfirmed"
                )
            default:
                throw PlatformWalletError(result: wrapped)
            }
        }
    }

    /// Throws `walletOperation` while an async [`stopShieldedSync()`] is in
    /// flight, like [`syncShieldedNow()`].
    public func syncShieldedWalletNow(walletId: Data) async throws {
        try ensureShieldedLifecycleAllowed("syncShieldedWalletNow")
        try ensureNoShieldedStopInFlight(before: "a manual shielded sync")
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be exactly 32 bytes"
            )
        }

        let handle = self.handle
        let walletIdCopy = walletId
        try await Task.detached(priority: .userInitiated) {
            try walletIdCopy.withUnsafeBytes { raw in
                guard let ptr = raw.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                    throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
                }
                try platform_wallet_manager_shielded_sync_wallet(handle, ptr).check()
            }
        }.value
    }
}

/// C trampoline matching `EventHandlerCallbacks.on_shielded_sync_completed_fn`.
func shieldedSyncCompletedCallback(
    context: UnsafeMutableRawPointer?,
    resultsPtr: UnsafePointer<ShieldedSyncWalletResultFFI>?,
    count: UInt,
    syncUnixSeconds: UInt64
) {
    guard let context else { return }

    let handler = Unmanaged<PlatformWalletEventHandler>
        .fromOpaque(context)
        .takeUnretainedValue()

    var results: [ShieldedWalletSyncResult] = []
    if let resultsPtr, count > 0 {
        results.reserveCapacity(Int(count))
        for i in 0..<Int(count) {
            results.append(ShieldedWalletSyncResult(ffi: resultsPtr[i]))
        }
    }

    let event = ShieldedSyncEvent(
        syncUnixSeconds: syncUnixSeconds,
        walletResults: results
    )

    // Snapshot the generation now, on the FFI callback thread, BEFORE the
    // event is enqueued onto the main actor. A subsequent stop/clear bumps
    // the counter, so this trailing event is dropped when it finally runs.
    let generation = handler.manager?.shieldedSyncGeneration.current() ?? 0

    Task { @MainActor [weak manager = handler.manager] in
        manager?.handleShieldedSyncCompleted(event, generation: generation)
    }
}

/// C trampoline matching `EventHandlerCallbacks.on_shielded_sync_progress_fn`.
/// Fires once per ~2048 notes processed during a cold sync. Cheap —
/// just hops to the main actor and publishes the snapshot.
func shieldedSyncProgressCallback(
    context: UnsafeMutableRawPointer?,
    cumulativeScanned: UInt64,
    blockHeight: UInt64
) {
    guard let context else { return }

    let handler = Unmanaged<PlatformWalletEventHandler>
        .fromOpaque(context)
        .takeUnretainedValue()

    // Snapshot the generation now, on the FFI callback thread, BEFORE the
    // event is enqueued onto the main actor. A subsequent stop/clear bumps
    // the counter, so this trailing event is dropped when it finally runs.
    let generation = handler.manager?.shieldedSyncGeneration.current() ?? 0

    Task { @MainActor [weak manager = handler.manager] in
        manager?.handleShieldedSyncProgress(
            cumulativeScanned: cumulativeScanned,
            blockHeight: blockHeight,
            generation: generation
        )
    }
}

/// C trampoline matching `EventHandlerCallbacks.on_shielded_tree_progress_fn`.
/// The "checked / committed-to-tree" signal — fires once per committed
/// batch as commitments are appended to the local Orchard tree.
/// `totalTarget == 0` means the on-chain total is indeterminate. Cheap —
/// just hops to the main actor and publishes the snapshot.
func shieldedTreeProgressCallback(
    context: UnsafeMutableRawPointer?,
    leavesCommitted: UInt64,
    totalTarget: UInt64
) {
    guard let context else { return }

    let handler = Unmanaged<PlatformWalletEventHandler>
        .fromOpaque(context)
        .takeUnretainedValue()

    // Snapshot the generation now, on the FFI callback thread, BEFORE the
    // event is enqueued onto the main actor. A subsequent stop/clear bumps
    // the counter, so this trailing event is dropped when it finally runs.
    let generation = handler.manager?.shieldedSyncGeneration.current() ?? 0

    Task { @MainActor [weak manager = handler.manager] in
        manager?.handleShieldedTreeProgress(
            committed: leavesCommitted,
            total: totalTarget,
            generation: generation
        )
    }
}
