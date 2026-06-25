import Foundation
import SwiftData
import Combine
import DashSDKFFI

/// Lock-guarded monotonic generation counter, safe to read and bump from
/// any thread. Used to drop shielded sync completion events that belong
/// to a generation already superseded by a `stop`/`clear`, even when a
/// restart happens in the same `@MainActor` turn (a plain boolean gate
/// can't, because the restart re-opens the gate before the stale,
/// previously-enqueued completion task runs).
final class ShieldedSyncGenerationCounter: @unchecked Sendable {
    private let lock = NSLock()
    private var value: UInt64 = 0
    func current() -> UInt64 { lock.withLock { value } }
    @discardableResult func bump() -> UInt64 { lock.withLock { value &+= 1; return value } }
}

/// The one thing SwiftUI needs for all wallet operations.
///
/// Owns the Rust-side `PlatformWalletManager` handle which drives:
/// - Wallet creation from mnemonic/seed
/// - SPV sync (core chain: headers, filters, masternodes)
/// - BLAST address balance sync
/// - Identity, DashPay, asset lock, token-balance tracking
/// - Persistence via SwiftData callbacks
///
/// Use as a root `@StateObject` and pass via `.environmentObject(_:)`.
/// Views observe `@Published` properties directly — no coordinator
/// class in the middle.
@MainActor
public class PlatformWalletManager: ObservableObject {
    // MARK: - Published observables

    /// Whether [`configure`] has been called successfully.
    @Published public private(set) var isConfigured: Bool = false

    /// The current SPV sync progress. Updated by the polling task
    /// started in [`configure`].
    @Published public private(set) var spvProgress: PlatformSpvSyncProgress = .empty

    @Published public private(set) var spvIsRunning: Bool = false

    /// Block time of the SPV header storage's current tip (if any).
    /// `nil` while SPV isn't running or hasn't stored a header yet.
    /// Useful as a "is core producing blocks?" indicator — when this
    /// stamp stops advancing, the chain is stalled even though the
    /// local SPV client is healthy.
    @Published public private(set) var spvTipBlockTime: Date?

    /// Whether the Rust-owned platform-address sync manager is currently in flight.
    @Published public private(set) var platformAddressSyncIsSyncing: Bool = false

    /// Last completed platform-address sync event emitted by Rust.
    @Published public internal(set) var lastPlatformAddressSyncEvent: PlatformAddressSyncEvent?

    /// Whether the Rust-owned shielded sync coordinator currently has
    /// a pass in flight.
    @Published public private(set) var shieldedSyncIsSyncing: Bool = false

    /// Last completed shielded sync event emitted by Rust.
    @Published public internal(set) var lastShieldedSyncEvent: ShieldedSyncEvent?

    /// Cumulative number of encrypted notes scanned in the **current**
    /// in-flight shielded sync pass, published once per chunk (~every
    /// 2048 notes) via the Rust-side progress callback. Nil between
    /// passes. Lets UI render a live counter / `ProgressView` during
    /// the cold sync of a large pool (1M notes can take 20+ min in a
    /// single SDK call; without this there's no signal between start
    /// and end).
    ///
    /// Paired with `currentShieldedSyncBlockHeight` — emitted in the
    /// same callback. They update together; the chain-tip number lets
    /// the UI estimate "still N blocks behind".
    @Published public internal(set) var currentShieldedSyncScanned: UInt64?
    @Published public internal(set) var currentShieldedSyncBlockHeight: UInt64?

    /// Cumulative count of note commitments appended to the local
    /// Orchard commitment tree in the **current** in-flight shielded
    /// sync pass — the "checked / committed-to-tree" signal, distinct
    /// from `currentShieldedSyncScanned` (which counts *downloaded*
    /// notes). Published once per committed batch via the Rust-side
    /// tree-progress callback. Nil between passes.
    ///
    /// Paired with `currentShieldedTreeTotal` — emitted in the same
    /// callback. `currentShieldedTreeTotal == 0` (or nil) means the
    /// total is indeterminate; the UI should render a spinner rather
    /// than a determinate bar.
    @Published public internal(set) var currentShieldedTreeCommitted: UInt64?
    @Published public internal(set) var currentShieldedTreeTotal: UInt64?

    /// Monotonic generation for shielded sync passes. Each `stop`/`clear`
    /// bumps it; the FFI completion callback snapshots the generation at
    /// enqueue time and `handleShieldedSyncCompleted` drops any event whose
    /// snapshot no longer matches the current generation.
    ///
    /// `stop` cancels the in-flight pass before its completion fires;
    /// `clear` goes further with a full Rust-side quiesce. Either way, an
    /// already-dispatched completion event can still land on this
    /// `@MainActor` after stop/clear returns. A plain boolean gate is
    /// bypassable: a caller can stop (set the flag) and restart (clear it)
    /// in the same actor turn, re-opening the gate before the stale
    /// completion task runs — so the old event leaks into the new run.
    /// Tying suppression to a generation closes that race: the stale task
    /// carries the pre-stop generation, the restart does not reset the
    /// counter, so the snapshot mismatches and the event is dropped even
    /// on a same-turn restart.
    ///
    /// `nonisolated` + lock-guarded so the FFI callback thread can snapshot
    /// it without hopping onto the main actor first.
    nonisolated let shieldedSyncGeneration = ShieldedSyncGenerationCounter()

    /// All wallets currently held by the Rust-side
    /// `PlatformWalletManager`, keyed by the 32-byte wallet id.
    ///
    /// The Rust manager holds N wallets concurrently — BLAST sync
    /// iterates every wallet in this map. Views should look up the
    /// wallet they care about via [`wallet(for:)`] rather than
    /// assuming a single "active" wallet.
    @Published public private(set) var wallets: [Data: ManagedPlatformWallet] = [:]

    /// Last error from a wallet operation, if any. Cleared on successful op.
    @Published public private(set) var lastError: Error?

    // MARK: - Internals

    /// FFI handle; `NULL_HANDLE` until [`configure`] is called.
    internal private(set) var handle: Handle = NULL_HANDLE

    /// Retained for the lifetime of the FFI handle so the callback
    /// context pointer remains valid.
    private var persistenceHandler: PlatformWalletPersistenceHandler?

    /// Retained for the lifetime of the FFI handle so the event-handler
    /// context pointer remains valid.
    private var eventHandler: PlatformWalletEventHandler?

    /// Background task that polls SPV progress.
    private var progressPollTask: Task<Void, Never>?

    // MARK: - Init

    /// Empty init for `@StateObject` usage. Call [`configure`] before
    /// any wallet operations.
    public init() {}

    /// Convenience: create and configure in one call.
    public convenience init(sdk: SDK, modelContainer: ModelContainer? = nil) throws {
        self.init()
        try self.configure(sdk: sdk, modelContainer: modelContainer)
    }

    deinit {
        progressPollTask?.cancel()
        guard handle != NULL_HANDLE else { return }

        // Tear down the Rust manager: signal the two host-driven sync
        // loops (platform-address + shielded) to cancel and `discard()`
        // them — both are cancel-only on the Rust side and never report
        // an incomplete drain. Identity-sync and the event adapter are
        // joined inside `destroy`, which is the single host-visible
        // join point.
        platform_wallet_manager_platform_address_sync_stop(handle).discard()
        platform_wallet_manager_shielded_sync_stop(handle).discard()

        // Capture the CODE (not just free the message) for the one call
        // that CAN report `.errorShutdownIncomplete`: `destroy`. Rust
        // returns that code when a background coordinator did not drain
        // within the join deadline, or when a prior-generation shielded
        // thread is still parked alive as an orphan (a tight
        // `stop()`→`start()` reap that had to detach it past the wedge
        // backstop). In either case a lingering `!Send` coordinator
        // thread may still hold the `passUnretained` context pointers
        // Rust was handed for our `persistenceHandler` / `eventHandler`
        // and fire ONE final callback through them. The contract: on
        // that code the host must NOT free the callback context
        // immediately.
        let destroyCode =
            platform_wallet_manager_destroy(handle).discardReturningCode()

        // Both handlers are passed to Rust via `Unmanaged.passUnretained`
        // (see `PlatformWalletPersistenceHandler`/`PlatformWalletEventHandler`
        // `makeCallbacks()`), so Rust holds non-owning pointers and these
        // objects are kept alive ONLY by the stored properties below. The
        // instant this deinit returns, ARC releases them — which would be a
        // use-after-free if a lingering coordinator then fires its final
        // callback. So, ONLY on an incomplete shutdown, deliberately leak one
        // extra strong reference to each (an unbalanced `passRetained` that is
        // never released) so they outlive any lingering thread. A clean
        // shutdown (the common case) takes neither branch and releases the
        // handlers normally — we never leak unconditionally. The leak is
        // bounded by how often a shutdown wedges (rare) and trades two small
        // objects for guaranteed callback safety, since an incomplete drain
        // gives no later signal that the lingering thread has finally exited.
        if destroyCode == .errorShutdownIncomplete {
            if let persistenceHandler { _ = Unmanaged.passRetained(persistenceHandler) }
            if let eventHandler { _ = Unmanaged.passRetained(eventHandler) }
        }
    }

    // MARK: - Configuration

    /// Configure the manager with an SDK and an optional SwiftData
    /// container. Must be called before any wallet operations.
    ///
    /// Spawns a background task that polls SPV sync progress every
    /// second and publishes it to [`spvProgress`].
    public func configure(sdk: SDK, modelContainer: ModelContainer? = nil) throws {
        precondition(!isConfigured, "PlatformWalletManager already configured")
        guard let sdkHandle = sdk.handle else {
            throw PlatformWalletError.invalidParameter("SDK has no handle")
        }
        guard let innerSdkPtr = dash_sdk_get_inner_sdk_ptr(sdkHandle) else {
            throw PlatformWalletError.invalidParameter(
                "dash_sdk_get_inner_sdk_ptr returned NULL for the supplied SDK"
            )
        }
        // The Rust manager is network-locked at construction
        // (`WalletManager::new(sdk.network)`); thread that same
        // network through to the persistence handler so its
        // `loadWalletList` only restores wallets bound to this
        // network, matching the per-network manager design.
        try configure(
            sdkPointer: UnsafeRawPointer(innerSdkPtr),
            modelContainer: modelContainer,
            network: sdk.network
        )
    }

    /// Configure with a raw Sdk pointer (advanced usage).
    public func configure(
        sdkPointer: UnsafeRawPointer,
        modelContainer: ModelContainer? = nil,
        network: Network? = nil
    ) throws {
        var handle: Handle = NULL_HANDLE

        let handler: PlatformWalletPersistenceHandler?
        var persistence: PersistenceCallbacks
        if let container = modelContainer {
            let h = PlatformWalletPersistenceHandler(
                modelContainer: container,
                network: network
            )
            persistence = h.makeCallbacks()
            handler = h
        } else {
            persistence = PersistenceCallbacks()
            handler = nil
        }

        let eventHandler = PlatformWalletEventHandler(manager: self)
        var eventHandlerCallbacks = eventHandler.makeCallbacks()

        try platform_wallet_manager_create(
            sdkPointer,
            &persistence,
            &eventHandlerCallbacks,
            &handle
        ).check()

        self.handle = handle
        self.persistenceHandler = handler
        self.eventHandler = eventHandler
        self.isConfigured = true

        startProgressPolling()
    }

    /// Access the persistence handler for loading cached data.
    public var persistence: PlatformWalletPersistenceHandler? {
        persistenceHandler
    }

    // MARK: - Wallet creation

    /// Create a wallet from a BIP39 mnemonic phrase (English).
    ///
    /// Stores the returned wallet as the active [`wallet`] published
    /// property. If `name` is provided, writes it onto the persisted
    /// [`PersistentWallet`] row so the wallet detail view has a
    /// user-facing label.
    @discardableResult
    public func createWallet(
        mnemonic: String,
        network: Network,
        name: String? = nil,
        createDefaultAccounts: Bool = true
    ) throws -> ManagedPlatformWallet {
        try ensureConfigured()
        var walletHandle: Handle = NULL_HANDLE
        var walletId: FFIByteTuple32 =
            (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)

        let accountOptions: UInt32 = createDefaultAccounts ? 1 : 0

        try mnemonic.withCString { mnemonicPtr in
            try platform_wallet_manager_create_wallet_from_mnemonic(
                handle,
                mnemonicPtr,
                network.ffiValue,
                accountOptions,
                &walletHandle,
                &walletId
            ).check()
        }

        let idData = withUnsafeBytes(of: &walletId) { Data($0) }
        if let name = name, !name.isEmpty {
            persistenceHandler?.setWalletName(walletId: idData, name: name)
        }
        let w = ManagedPlatformWallet(handle: walletHandle, walletId: idData)
        self.wallets[idData] = w
        return w
    }

    /// Create a wallet from raw 64-byte seed bytes.
    @discardableResult
    public func createWallet(
        seed: Data,
        network: Network,
        name: String? = nil,
        createDefaultAccounts: Bool = true
    ) throws -> ManagedPlatformWallet {
        try ensureConfigured()
        guard seed.count == 64 else {
            throw PlatformWalletError.invalidParameter(
                "seed must be 64 bytes, got \(seed.count)"
            )
        }

        var walletHandle: Handle = NULL_HANDLE
        var walletId: FFIByteTuple32 =
            (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)

        let accountOptions: UInt32 = createDefaultAccounts ? 1 : 0

        try seed.withUnsafeBytes { seedPtr in
            try platform_wallet_manager_create_wallet_from_seed(
                handle,
                network.ffiValue,
                seedPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                UInt(seed.count),
                accountOptions,
                &walletHandle,
                &walletId
            ).check()
        }

        let idData = withUnsafeBytes(of: &walletId) { Data($0) }
        if let name = name, !name.isEmpty {
            persistenceHandler?.setWalletName(walletId: idData, name: name)
        }
        let w = ManagedPlatformWallet(handle: walletHandle, walletId: idData)
        self.wallets[idData] = w
        return w
    }

    // MARK: - Watch-only restore from persister

    /// Rehydrate wallets from SwiftData on app launch.
    ///
    /// Calls `platform_wallet_manager_load_from_persistor` which fires
    /// the Swift-side `on_load_wallet_list_fn` callback. For each
    /// persisted wallet, Rust reconstructs a **watch-only** `Wallet`
    /// plus the wallet's persisted platform-address sync snapshot.
    /// After the FFI returns, we call `platform_wallet_manager_get_wallet`
    /// for each restored id so Swift gets a `ManagedPlatformWallet`
    /// handle.
    ///
    /// Signing operations will fail until a future unlock flow
    /// upgrades a watch-only wallet to a signing wallet via the
    /// mnemonic stored in Keychain.
    ///
    /// Idempotent: if there's no persisted state, does nothing and
    /// leaves `self.wallets` untouched. Safe to call before any
    /// `createWallet` flow.
    @discardableResult
    public func loadFromPersistor() throws -> [ManagedPlatformWallet] {
        try ensureConfigured()

        try platform_wallet_manager_load_from_persistor(handle).check()

        // Ask SwiftData for the list of wallet ids we just told Rust
        // to load. We reuse the same container rather than shipping a
        // separate FFI "list ids" entry, because SwiftData already is
        // the source of truth.
        guard let persistenceHandler = persistenceHandler else {
            return []
        }
        let walletIds = persistenceHandler.restorableWalletIds()
        var restored: [ManagedPlatformWallet] = []
        restored.reserveCapacity(walletIds.count)

        for walletId in walletIds {
            guard walletId.count == 32 else { continue }
            var walletHandle: Handle = NULL_HANDLE
            do {
                try walletId.withUnsafeBytes { idPtr in
                    // C signature is `const uint8_t (*wallet_id)[32]`, which Swift
                    // imports as `UnsafePointer<FFIByteTuple32>?`. Rebind the raw
                    // 32-byte buffer to that 32-tuple shape so the call type-checks.
                    guard let base = idPtr.baseAddress?.assumingMemoryBound(to: FFIByteTuple32.self) else {
                        throw PlatformWalletError.nullPointer(
                            "wallet_id buffer base address was nil"
                        )
                    }
                    try platform_wallet_manager_get_wallet(
                        handle,
                        base,
                        &walletHandle
                    ).check()
                }
                let managedWallet = ManagedPlatformWallet(handle: walletHandle, walletId: walletId)
                restored.append(managedWallet)
                self.wallets[walletId] = managedWallet
            } catch {
                // Log and skip — one wallet failing doesn't fail the
                // whole restore. Usually means wallet_id / xpub
                // disagreement (SwiftData drift vs. Rust recompute).
                self.lastError = error
            }
        }

        // Kick off a background catch-up pass for every persisted
        // asset lock at `statusRaw < 2`. Closes the SPV-restart gap:
        // the wallet's in-memory transactions map was just
        // selectively repopulated by the load path (Rust-side
        // `restore_unresolved_asset_lock_tx_records`), so the next
        // chain-lock event picked up by SPV will cascade through
        // `apply_chain_lock` and promote each funding tx; the
        // catch-up `Task` parks on `wait_for_proof` until that
        // happens and on success the Rust changeset writes
        // `statusRaw = 3 + proofBytes` back to SwiftData. UI updates
        // reactively via `@Query`.
        catchUpStuckAssetLocks(wallets: restored)

        return restored
    }

    /// For every persisted asset lock at `statusRaw < 2` (Built /
    /// Broadcast), kick off a background `Task` that drives
    /// `asset_lock_manager_catch_up_blocking` to completion or
    /// timeout. Fire-and-forget — the proof reaches the UI via the
    /// `AssetLockChangeSet` that the catch-up call queues internally.
    ///
    /// Called from `loadFromPersistor` after every wallet is
    /// inserted. App-foreground / network-reconnect callers can
    /// invoke this directly to retry whatever was still pending.
    public func catchUpStuckAssetLocks(wallets: [ManagedPlatformWallet]) {
        guard let persistenceHandler = persistenceHandler else { return }
        for wallet in wallets {
            let walletId = wallet.walletId
            let locks = persistenceHandler.loadCachedAssetLocks(walletId: walletId)
            let pending = locks.filter { $0.statusRaw < 2 }
            if pending.isEmpty { continue }

            // Snapshot the asset-lock manager wrapper ON the main
            // actor (where `wallet` lives), then hand the wrapper
            // itself — not just its bare `Handle` — to the detached
            // tasks. Each task's capture is a retain on the wrapper,
            // so `deinit` (which calls `asset_lock_manager_destroy`
            // and invalidates the FFI handle) can't fire until the
            // last in-flight catch-up task drops its retain. That
            // closes the lifetime race where a follow-up
            // `catchUpStuckAssetLocks` call (e.g. on
            // app-foreground / network-reconnect) used to destroy
            // the previous batch's handles mid-FFI-call.
            //
            // `ManagedAssetLockManager` is `@unchecked Sendable`
            // (immutable `let handle`, no shared mutable state, deinit
            // runs exactly once via ARC), so capturing it across
            // task boundaries is safe.
            let assetLockManager: ManagedAssetLockManager
            do {
                assetLockManager = try wallet.assetLockManager()
            } catch {
                self.lastError = error
                continue
            }

            // Cap concurrency to avoid saturating iOS's cooperative
            // thread pool. Each catch-up `block_on` parks a worker
            // thread for up to 300s; N stuck locks at launch (after a
            // multi-identity registration interrupted by an app kill)
            // would otherwise spawn N parallel parked threads,
            // starving every other `Task` in the app (UI updates,
            // SwiftData writes, network calls).
            //
            // `MAX_CONCURRENT_CATCH_UPS = 4` is conservative for a
            // 4-8 worker pool typical on iPhones. The real bottleneck
            // is per-lock SPV chainlock arrival, not the catch-up
            // throughput — running 4 in parallel vs 50 changes nothing
            // about how fast each individual lock resolves.
            let outpoints: [(txid: Data, vout: UInt32)] = pending.compactMap {
                PlatformWalletManager.decodeOutPointForCatchUp($0.outPointHex)
            }
            guard !outpoints.isEmpty else { continue }
            Task.detached(priority: .background) {
                await withTaskGroup(of: Void.self) { group in
                    let maxConcurrent = 4
                    var nextIndex = 0
                    // Seed the group with up to `maxConcurrent` tasks.
                    // Each `group.addTask` closure captures
                    // `assetLockManager` — that retain keeps the
                    // wrapper alive for the duration of the FFI call,
                    // independently of the outer detached task and
                    // independently of any future
                    // `catchUpStuckAssetLocks` invocation.
                    while nextIndex < outpoints.count && nextIndex < maxConcurrent {
                        let (txid, vout) = outpoints[nextIndex]
                        group.addTask {
                            Self.runCatchUp(assetLockManager: assetLockManager, txid: txid, vout: vout)
                        }
                        nextIndex += 1
                    }
                    // As each finishes, queue the next pending entry.
                    while await group.next() != nil {
                        if nextIndex < outpoints.count {
                            let (txid, vout) = outpoints[nextIndex]
                            group.addTask {
                                Self.runCatchUp(assetLockManager: assetLockManager, txid: txid, vout: vout)
                            }
                            nextIndex += 1
                        }
                    }
                }
            }
        }
    }

    /// Single catch-up invocation body. Extracted from the inline
    /// `Task.detached` so the `withTaskGroup` coordinator can call it
    /// directly. The `assetLockManager` parameter is captured by each
    /// task closure — the task's retain on the wrapper guarantees the
    /// FFI handle (read via `assetLockManager.handle`) stays valid
    /// for the entire `asset_lock_manager_catch_up_blocking` call,
    /// even if a follow-up `catchUpStuckAssetLocks` invocation
    /// replaces the manager-wide reference midway through.
    ///
    /// `nonisolated` because `PlatformWalletManager` is
    /// `@MainActor`-isolated by default and the detached task body
    /// runs off the main actor — the FFI call is synchronous and
    /// reads no `PlatformWalletManager` state.
    nonisolated private static func runCatchUp(assetLockManager: ManagedAssetLockManager, txid: Data, vout: UInt32) {
        // Build the txid tuple inline so the Task body captures only
        // Sendable values.
        var txidTuple: FFIByteTuple32 =
            (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
             0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
        txid.withUnsafeBytes { buf in
            withUnsafeMutableBytes(of: &txidTuple) { dst in
                dst.copyBytes(from: buf.prefix(32))
            }
        }
        // Five-minute ceiling matches the `wait_for_proof` deadline
        // the production resume path uses.
        let result = asset_lock_manager_catch_up_blocking(
            assetLockManager.handle, &txidTuple, vout, 300
        )
        // Timeouts and proof-wait failures (catch-up
        // `errorWalletOperation`) are expected during normal
        // operation (SPV not yet caught up to the funding block,
        // chain-lock hasn't fired yet) — discard.
        // `errorInvalidHandle` is NOT expected: each task retains its
        // own `assetLockManager` wrapper, so the handle is guaranteed
        // valid for the duration of this call. If it surfaces, log it
        // loudly via NSLog so an operator running without `tracing`
        // capture still sees the programmer error.
        let code = PlatformWalletResultCode(ffi: result.code)
        if code == .errorInvalidHandle {
            NSLog(
                "[catch-up] asset_lock_manager_catch_up_blocking returned errorInvalidHandle for outpoint %@:%u — handle invalid despite task-owned wrapper retain",
                txid.map { String(format: "%02x", $0) }.joined(),
                vout
            )
        }
    }

    /// Parse `<txid_hex (display order)>:<vout>` into the wire-order
    /// 32-byte `txid` + `vout` pair the FFI expects. Internal to
    /// `catchUpStuckAssetLocks`; mirrors the Swift-side
    /// `decodeOutPointHex` helper without taking a dependency on the
    /// private one in the persistence handler.
    private static func decodeOutPointForCatchUp(
        _ hex: String
    ) -> (txid: Data, vout: UInt32)? {
        let parts = hex.split(separator: ":", maxSplits: 1, omittingEmptySubsequences: false)
        guard parts.count == 2 else { return nil }
        let txidHex = String(parts[0])
        guard let vout = UInt32(parts[1]) else { return nil }
        guard txidHex.count == 64 else { return nil }
        var txid = Data(capacity: 32)
        var idx = txidHex.startIndex
        for _ in 0..<32 {
            let end = txidHex.index(idx, offsetBy: 2)
            guard let byte = UInt8(txidHex[idx..<end], radix: 16) else { return nil }
            txid.append(byte)
            idx = end
        }
        // outPointHex is display-order; the FFI expects wire-order
        // (the same orientation `PersistentTransaction.txid` is
        // stored in).
        return (txid: Data(txid.reversed()), vout: vout)
    }

    // MARK: - Wallet deletion

    /// Fully wipe a wallet's Rust, SwiftData, and Keychain footprint.
    ///
    /// Requires the manager to have been `configure`d with a
    /// `ModelContainer` — the per-identity Keychain sweep needs the
    /// wallet's identity ids, which only the persistence handler can
    /// resolve. The no-persistence configuration mode is rejected
    /// here rather than silently leaving identity key material behind.
    ///
    /// Deleting an already-removed wallet succeeds unless an
    /// operation fails.
    public func deleteWallet(walletId: Data) throws {
        try ensureConfigured()
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be 32 bytes, got \(walletId.count)"
            )
        }
        guard let persistenceHandler = persistenceHandler else {
            throw PlatformWalletError.invalidHandle(
                "deleteWallet requires a persistence handler — configure the manager with a ModelContainer"
            )
        }

        let identityIds = try persistenceHandler.identityIdsForWallet(walletId: walletId)

        // Wipe Keychain BEFORE the SwiftData identity deletion runs.
        // Order matters for retry-safety: if `deleteWalletData`
        // commits identity rows and then throws partway, a retry
        // would see `identityIdsForWallet == []` and the
        // `deleteAllKeychainItems(forIdentityId:)` sweep below
        // could no longer find the keys to purge. Doing the
        // keychain side first leaves at worst stale SwiftData
        // rows on a retry — repeating the wipe is harmless, and
        // every keychain call here is idempotent (no-op on "not
        // found"). Mnemonic / metadata stay in `WalletStorage`
        // for now so a retry can still derive any missed key.
        for identityId in identityIds {
            try KeychainManager.shared.deleteAllKeychainItems(forIdentityId: identityId)
        }
        try KeychainManager.shared.deleteAllIdentityPrivateKeys(forWalletId: walletId)

        try walletId.withUnsafeBytes { raw in
            guard let base = raw.baseAddress?.assumingMemoryBound(to: FFIByteTuple32.self) else {
                throw PlatformWalletError.nullPointer(
                    "wallet_id buffer base address was nil"
                )
            }
            try platform_wallet_manager_remove_wallet(handle, base).check()
        }

        wallets.removeValue(forKey: walletId)

        try persistenceHandler.deleteWalletData(walletId: walletId)

        // The mnemonic + metadata blobs in the Keychain are keyed by
        // `walletId`. With network-scoped wallet ids the same mnemonic
        // maps to a DIFFERENT id per network, so a given id is owned by
        // exactly one network's wallet and carries its own mnemonic
        // copy — purging it can't orphan a sibling network (those live
        // under their own distinct ids). The `walletRowCountAcrossNetworks
        // == 0` check is therefore expected to be true right after
        // `deleteWalletData` removes this id's lone row; it is retained
        // as a defensive guard (and to stay correct should the id model
        // ever change) so we never delete the phrase while any row for
        // this exact id still exists.
        let remaining = try persistenceHandler.walletRowCountAcrossNetworks(walletId: walletId)
        if remaining == 0 {
            let storage = WalletStorage()
            // Delete metadata first so the mnemonic remains available for retry.
            try storage.deleteMetadata(for: walletId)
            try storage.deleteMnemonic(for: walletId)
        }
    }

    // MARK: - Per-wallet lookup

    /// Return the managed wallet with the given 32-byte id, or `nil`
    /// if it is not loaded.
    ///
    /// The Rust manager can hold multiple wallets at once (BLAST
    /// sync operates on all of them); UI surfaces that act against a
    /// specific wallet should route through this lookup rather than
    /// assuming a single active wallet exists.
    public func wallet(for walletId: Data) -> ManagedPlatformWallet? {
        wallets[walletId]
    }

    /// Convenience for bootstrap / single-wallet UI surfaces: an
    /// arbitrary-but-deterministic wallet from the managed set
    /// (sorted by walletId). Returns `nil` when no wallets are
    /// loaded.
    public var firstWallet: ManagedPlatformWallet? {
        guard !wallets.isEmpty else { return nil }
        let key = wallets.keys.min(by: { $0.lexicographicallyPrecedes($1) })
        return key.flatMap { wallets[$0] }
    }

    // MARK: - Xpub rendering

    /// Render a bincode-encoded per-account `ExtendedPubKey` (as
    /// stored on `PersistentAccount.accountExtendedPubKeyBytes`) as a
    /// BIP32 base58check string. The encoded key carries its own
    /// network, so `xpub…`/`tpub…` is produced automatically.
    ///
    /// Returns `nil` if the bytes are empty or the decode fails.
    public static func accountExtendedPubKeyString(bytes: Data) -> String? {
        guard !bytes.isEmpty else { return nil }
        var outPtr: UnsafeMutablePointer<CChar>? = nil
        do {
            try bytes.withUnsafeBytes { raw in
                guard let base = raw.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                    throw PlatformWalletError.nullPointer(
                        "xpub bytes buffer base address was nil"
                    )
                }
                try platform_wallet_account_xpub_to_string(
                    base,
                    UInt(bytes.count),
                    &outPtr
                ).check()
            }
        } catch {
            return nil
        }
        guard let cStr = outPtr else {
            return nil
        }
        let str = String(cString: cStr)
        platform_wallet_free_string(cStr)
        return str
    }

    // MARK: - Per-account balances

    /// Per-account balance snapshot read from Rust's in-memory state.
    ///
    /// `keysUsed` / `keysTotal` are the number of derived addresses
    /// across every pool on the account, with `keysUsed` further
    /// filtered by `AddressInfo.used`. The fields are populated for
    /// both funds and keys variants — the explorer surfaces them as
    /// the headline number on keys-only rows where balance is zero by
    /// construction.
    public struct AccountBalance {
        public let typeTag: UInt8
        public let standardTag: UInt8
        public let index: UInt32
        public let registrationIndex: UInt32
        public let keyClass: UInt32
        public let userIdentityId: Data
        public let friendIdentityId: Data
        public let confirmed: UInt64
        public let unconfirmed: UInt64
        public let immature: UInt64
        public let locked: UInt64
        public let keysUsed: UInt32
        public let keysTotal: UInt32
    }

    /// Query per-account balances directly from the Rust-side
    /// `WalletManager`'s in-memory state. No disk I/O — reads the
    /// live `ManagedCoreAccount.balance` values maintained during SPV
    /// processing.
    public func accountBalances(for walletId: Data) -> [AccountBalance] {
        guard isConfigured, handle != NULL_HANDLE, walletId.count == 32 else {
            return []
        }

        var outEntries: UnsafePointer<AccountBalanceEntryFFI>?
        var outCount: UInt = 0

        let ffiResult = walletId.withUnsafeBytes { (raw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
            let base = raw.baseAddress?.assumingMemoryBound(to: UInt8.self)
            return platform_wallet_manager_get_account_balances(
                handle,
                base,
                &outEntries,
                &outCount
            )
        }

        let result = PlatformWalletResult(ffiResult)


        guard result.isSuccess else {
            self.lastError = PlatformWalletError(result: result)
            return []
        }

        guard let entries = outEntries, outCount > 0 else {
            return []
        }

        defer {
            platform_wallet_manager_free_account_balances(
                UnsafeMutablePointer(mutating: entries),
                outCount
            )
        }

        return (0..<Int(outCount)).map { i in
            var entry = entries[i]
            let uid = withUnsafeBytes(of: &entry.user_identity_id) { Data($0) }
            let fid = withUnsafeBytes(of: &entry.friend_identity_id) { Data($0) }
            return AccountBalance(
                typeTag: entry.type_tag,
                standardTag: entry.standard_tag,
                index: entry.index,
                registrationIndex: entry.registration_index,
                keyClass: entry.key_class,
                userIdentityId: uid,
                friendIdentityId: fid,
                confirmed: entry.confirmed,
                unconfirmed: entry.unconfirmed,
                immature: entry.immature,
                locked: entry.locked,
                keysUsed: entry.keys_used,
                keysTotal: entry.keys_total
            )
        }
    }

    // MARK: - Internals

    private func ensureConfigured() throws {
        if !isConfigured || handle == NULL_HANDLE {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
    }

    /// Starts the SPV progress polling loop. Cancelled on deinit.
    ///
    /// `@Published` assignments are gated on inequality so that identical
    /// snapshots don't trigger SwiftUI re-evaluation. A naive 1 Hz reassignment
    /// of a non-Equatable struct caused every observer (sync screens, memory
    /// explorer, global indicator) to re-evaluate every second, accreting
    /// SwiftUI attribute-graph state and burning CPU long after sync settled.
    private func startProgressPolling() {
        progressPollTask?.cancel()
        progressPollTask = Task { [weak self] in
            while !Task.isCancelled {
                guard let self = self else { return }
                if let progress = try? self.syncProgress(), progress != self.spvProgress {
                    self.spvProgress = progress
                }
                if let running = try? self.isSpvRunning(), running != self.spvIsRunning {
                    self.spvIsRunning = running
                }
                if let isSyncing = try? self.isPlatformAddressSyncing(),
                   isSyncing != self.platformAddressSyncIsSyncing {
                    self.platformAddressSyncIsSyncing = isSyncing
                }
                if let isSyncing = try? self.isShieldedSyncing(),
                   isSyncing != self.shieldedSyncIsSyncing {
                    self.shieldedSyncIsSyncing = isSyncing
                }
                let tip = (try? self.currentSpvTipBlockTime()) ?? nil
                if tip != self.spvTipBlockTime {
                    self.spvTipBlockTime = tip
                }
                try? await Task.sleep(for: .seconds(1))
            }
        }
    }
}
