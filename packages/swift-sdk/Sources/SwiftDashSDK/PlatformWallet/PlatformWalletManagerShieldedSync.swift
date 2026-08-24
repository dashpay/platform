import Foundation
import DashSDKFFI

/// Thrown by `shieldedIdentityCreateFromPool` when the Type-20
/// transition was broadcast and ACCEPTED by the relay, but the SDK
/// could not confirm its execution result and a direct fetch of the
/// derived id also came back empty (the Rust side already retried).
///
/// This is NOT a registration failure: the identity may already exist
/// on chain (the broadcast landed; only the result-proof confirmation
/// failed — e.g. a transient DAPI/proof error). The caller MUST hold
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

extension PlatformWalletManager {
    func handleShieldedSyncCompleted(_ event: ShieldedSyncEvent, generation: UInt64) {
        // Drop a trailing event that the Rust drain already dispatched but
        // the main actor only delivers after stop/clear returned. The FFI
        // callback snapshots `shieldedSyncGeneration` at enqueue time; a
        // stop/clear bumps the counter, so a stale event's snapshot no
        // longer matches and is dropped — even if a restart happened in the
        // same actor turn (the restart does not reset the counter).
        guard generation == shieldedSyncGeneration.current() else { return }
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
    func handleShieldedSyncProgress(
        cumulativeScanned: UInt64,
        blockHeight: UInt64,
        generation: UInt64
    ) {
        guard generation == shieldedSyncGeneration.current() else { return }
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
    /// just reset.
    func handleShieldedTreeProgress(
        committed: UInt64,
        total: UInt64,
        generation: UInt64
    ) {
        guard generation == shieldedSyncGeneration.current() else { return }
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
    public func bindShielded(
        walletId: Data,
        resolver: MnemonicResolver,
        accounts: [UInt32] = [0]
    ) throws {
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
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
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
    public func startShieldedSync(intervalSeconds: UInt64? = nil) throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }

        if let intervalSeconds {
            try setShieldedSyncInterval(seconds: intervalSeconds)
        }
        // No generation reset needed: events emitted by this new run
        // snapshot the current generation, so they pass the guard. A
        // trailing event from a prior, stopped run still carries the older
        // generation and is dropped.
        try platform_wallet_manager_shielded_sync_start(handle).check()
    }

    public func stopShieldedSync() throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
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
    public func clearShielded() throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        try platform_wallet_manager_shielded_clear(handle).check()
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
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        try platform_wallet_manager_shielded_sync_set_interval(handle, seconds).check()
    }

    public func syncShieldedNow() async throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
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
    /// Throws `PlatformWalletError.shieldedSpendUnconfirmed` when the
    /// broadcast was accepted but its execution result couldn't be
    /// confirmed — the spend may already be on chain, so the caller
    /// must NOT retry (the spent notes stay reserved Rust-side; the
    /// next shielded sync reconciles them).
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
        guard let resolverHandle = resolver.handle else {
            throw PlatformWalletError.invalidParameter(
                "MnemonicResolver has no handle"
            )
        }

        let handle = self.handle
        try await Task.detached(priority: .userInitiated) {
            // MnemonicResolver is passed to Rust via `passUnretained`,
            // so the Rust ctx pointer dangles unless the Swift owner
            // stays alive across the whole FFI call. A bare
            // `_ = resolver` is folklore the optimizer may elide in -O
            // builds; `withExtendedLifetime` is the guaranteed
            // keepalive (same as the identity-create sibling).
            try withExtendedLifetime(resolver) {
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
                            try platform_wallet_manager_shielded_transfer(
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
        }.value
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
    /// Throws `PlatformWalletError.shieldedSpendUnconfirmed` when the
    /// broadcast was accepted but its execution result couldn't be
    /// confirmed — the shield may already be on chain, so the caller
    /// must NOT retry (a retry would rebuild the bundle and could
    /// double-shield; the next sync reconciles the outcome). A shield
    /// spends no notes, so nothing is reserved wallet-side.
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

        let handle = self.handle
        let signerHandle = addressSigner.handle

        try await Task.detached(priority: .userInitiated) {
            // Keepalive — same rationale as `topUpFromAddresses`.
            // The trampoline ctx pointer inside the signer
            // dangles unless the Swift owner outlives this
            // detached work.
            _ = addressSigner

            try walletId.withUnsafeBytes { widRaw in
                guard let widPtr = widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                else {
                    throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
                }
                try platform_wallet_manager_shielded_shield(
                    handle, widPtr, shieldedAccount, paymentAccount, amount, signerHandle
                ).check()
            }
        }.value
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
    /// Throws `PlatformWalletError.shieldedSpendUnconfirmed` when the
    /// broadcast was accepted but its execution result couldn't be
    /// confirmed — the spend may already be on chain, so the caller
    /// must NOT retry (the spent notes stay reserved Rust-side; the
    /// next shielded sync reconciles them).
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
        guard let resolverHandle = resolver.handle else {
            throw PlatformWalletError.invalidParameter(
                "MnemonicResolver has no handle"
            )
        }

        let handle = self.handle
        try await Task.detached(priority: .userInitiated) {
            // Guaranteed resolver keepalive across the FFI call —
            // same rationale as `shieldedTransfer`.
            try withExtendedLifetime(resolver) {
                try walletId.withUnsafeBytes { widRaw in
                    guard let widPtr = widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                    else {
                        throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
                    }
                    try toPlatformAddress.withCString { addrCStr in
                        try platform_wallet_manager_shielded_unshield(
                            handle, widPtr, resolverHandle, account, addrCStr, amount
                        ).check()
                    }
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
    /// Throws `PlatformWalletError.shieldedSpendUnconfirmed` when the
    /// broadcast was accepted but its execution result couldn't be
    /// confirmed — the spend may already be on chain, so the caller
    /// must NOT retry (the spent notes stay reserved Rust-side; the
    /// next shielded sync reconciles them).
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
        guard let resolverHandle = resolver.handle else {
            throw PlatformWalletError.invalidParameter(
                "MnemonicResolver has no handle"
            )
        }

        let handle = self.handle
        try await Task.detached(priority: .userInitiated) {
            // Guaranteed resolver keepalive across the FFI call —
            // same rationale as `shieldedTransfer`.
            try withExtendedLifetime(resolver) {
                try walletId.withUnsafeBytes { widRaw in
                    guard let widPtr = widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                    else {
                        throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
                    }
                    try toCoreAddress.withCString { addrCStr in
                        try platform_wallet_manager_shielded_withdraw(
                            handle, widPtr, resolverHandle, account, addrCStr, amount,
                            coreFeePerByte
                        ).check()
                    }
                }
            }
        }.value
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
        guard let resolverHandle = resolver.handle else {
            throw PlatformWalletError.invalidParameter(
                "MnemonicResolver has no handle"
            )
        }

        let handle = self.handle
        let identitySignerHandle = identitySigner.handle
        let fallbackAddressBytes = sendToAddressOnCreationFailure

        return try await Task.detached(priority: .userInitiated) { () -> Data in
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
            // signer-lifetime guidance).
            let result = try withExtendedLifetime((identitySigner, resolver)) {
                try walletId.withUnsafeBytes { widRaw -> PlatformWalletFFIResult in
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
                            platform_wallet_manager_shielded_identity_create_from_pool(
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
            //   - unconfirmed: the broadcast landed but its result couldn't be
            //     confirmed; Rust filled `outIdentityId` with the derived id on
            //     THIS code (and only this code). Throw the typed
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
        }.value
    }

    public func syncShieldedWalletNow(walletId: Data) async throws {
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
