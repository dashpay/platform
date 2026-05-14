import Foundation
import SwiftData
import Combine
import DashSDKFFI

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

    /// `ManagedAssetLockManager` instances retained while their
    /// detached catch-up tasks run. Each wrapper's `deinit`
    /// invalidates the underlying FFI handle, which would crash the
    /// in-flight `asset_lock_manager_catch_up_blocking` call — so we
    /// hold the wrappers here for the duration of a batch. Replaced
    /// (and the previous batch released) on each
    /// `catchUpStuckAssetLocks` invocation, by which time the prior
    /// batch's tasks have either resolved or hit their five-minute
    /// timeout.
    private var retainedAssetLockManagers: [ManagedAssetLockManager] = []

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
        if handle != NULL_HANDLE {
            platform_wallet_manager_platform_address_sync_stop(handle).discard()
            platform_wallet_manager_shielded_sync_stop(handle).discard()
            platform_wallet_manager_destroy(handle).discard()
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
        // Release the previous batch's manager wrappers now that we
        // know their tasks have either completed or timed out (any
        // task still running past the 300s timeout is misbehaving
        // and the bound is on the Rust side anyway). Without this
        // the array would grow unboundedly across foregroundings.
        retainedAssetLockManagers.removeAll(keepingCapacity: true)
        for wallet in wallets {
            let walletId = wallet.walletId
            let locks = persistenceHandler.loadCachedAssetLocks(walletId: walletId)
            let pending = locks.filter { $0.statusRaw < 2 }
            if pending.isEmpty { continue }

            // Snapshot the asset-lock manager handle ON the main
            // actor (where `wallet` lives). The `ManagedAssetLockManager`
            // class isn't `Sendable` (its `deinit` calls
            // `asset_lock_manager_destroy`), so the detached Task
            // captures the bare `Handle` value (an `Int64`) and
            // calls the FFI directly. Lifetime: stash the manager
            // wrapper on `retainedAssetLockManagers` so its `deinit`
            // (which would invalidate the handle) waits for the
            // tasks to complete; the wrapper is dropped on the next
            // call to `catchUpStuckAssetLocks` or on manager
            // shutdown, whichever comes first.
            let assetLockManager: ManagedAssetLockManager
            do {
                assetLockManager = try wallet.assetLockManager()
            } catch {
                self.lastError = error
                continue
            }
            // The previous batch's manager wrappers (if any) are
            // dropped here — their tasks have either completed
            // (success path persisted via the changeset) or hit the
            // 300s timeout long ago. The replacement keeps the
            // current batch's handles alive for the duration of the
            // new tasks.
            retainedAssetLockManagers.append(assetLockManager)
            let handle = assetLockManager.handle

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
                    while nextIndex < outpoints.count && nextIndex < maxConcurrent {
                        let (txid, vout) = outpoints[nextIndex]
                        group.addTask {
                            Self.runCatchUp(handle: handle, txid: txid, vout: vout)
                        }
                        nextIndex += 1
                    }
                    // As each finishes, queue the next pending entry.
                    while await group.next() != nil {
                        if nextIndex < outpoints.count {
                            let (txid, vout) = outpoints[nextIndex]
                            group.addTask {
                                Self.runCatchUp(handle: handle, txid: txid, vout: vout)
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
    /// directly. Sendable inputs only: `Handle` is `Int64`, `Data`
    /// captures the txid bytes, `UInt32` is trivially Sendable.
    ///
    /// `nonisolated` because `PlatformWalletManager` is
    /// `@MainActor`-isolated by default and the detached task body
    /// runs off the main actor — the FFI call is synchronous and
    /// reads no `PlatformWalletManager` state.
    nonisolated private static func runCatchUp(handle: Handle, txid: Data, vout: UInt32) {
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
            handle, &txidTuple, vout, 300
        )
        // Timeouts and proof-wait failures (catch-up
        // `errorWalletOperation`) are expected during normal
        // operation (SPV not yet caught up to the funding block,
        // chain-lock hasn't fired yet) — discard.
        // `errorInvalidHandle` is NOT expected and indicates the
        // manager wrapper was released mid-task (lifecycle race /
        // programmer error); log it loudly via NSLog so an operator
        // running without `tracing` capture still sees it.
        let code = PlatformWalletResultCode(ffi: result.code)
        if code == .errorInvalidHandle {
            NSLog(
                "[catch-up] asset_lock_manager_catch_up_blocking returned errorInvalidHandle for outpoint %@:%u — wrapper released before task finished",
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
