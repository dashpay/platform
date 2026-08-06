import Foundation
import DashSDKFFI

public struct PlatformAddressWalletSyncResult: Sendable {
    public let walletId: Data
    public let success: Bool
    public let foundCount: Int
    public let absentCount: Int
    public let checkpointHeight: UInt64
    public let newSyncHeight: UInt64
    public let newSyncTimestamp: UInt64
    public let lastKnownRecentBlock: UInt64
    public let metrics: AddressSyncMetrics
    public let errorMessage: String?

    init(ffi: PlatformAddressSyncWalletResultFFI) {
        var walletId = ffi.wallet_id
        self.walletId = withUnsafeBytes(of: &walletId) { Data($0) }
        self.success = ffi.success
        self.foundCount = Int(ffi.found_count)
        self.absentCount = Int(ffi.absent_count)
        self.checkpointHeight = ffi.checkpoint_height
        self.newSyncHeight = ffi.new_sync_height
        self.newSyncTimestamp = ffi.new_sync_timestamp
        self.lastKnownRecentBlock = ffi.last_known_recent_block
        self.metrics = AddressSyncMetrics(platformFFI: ffi.metrics)
        self.errorMessage = ffi.error_message.map { String(cString: $0) }
    }
}

public struct PlatformAddressSyncEvent: Sendable {
    public let syncUnixSeconds: UInt64
    public let walletResults: [PlatformAddressWalletSyncResult]

    public func result(for walletId: Data) -> PlatformAddressWalletSyncResult? {
        walletResults.first { $0.walletId == walletId }
    }
}

/// `@unchecked Sendable`, matching `PlatformWalletPersistenceHandler`: this
/// object *is* a cross-thread callback context by construction — Rust owns
/// a retained reference to it and invokes the callbacks below from its own
/// background threads. `manager` is written once at `init` and only read
/// afterwards (weak loads are atomic), and every touch of the main-actor
/// manager already hops through `Task { @MainActor }`.
final class PlatformWalletEventHandler: @unchecked Sendable {
    weak var manager: PlatformWalletManager?

    init(manager: PlatformWalletManager) {
        self.manager = manager
    }

    /// Build `EventHandlerCallbacks` that point to this handler.
    ///
    /// **Transfers ownership of a strong reference to Rust**: the context
    /// is `passRetained`, and `release_fn` balances that retain exactly
    /// once — when the Rust manager and every worker that can still
    /// dispatch an event have dropped their references (possibly on a
    /// Rust thread, possibly after `destroy` returns if a worker
    /// straggles). ARC therefore cannot free this handler while any Rust
    /// worker can still call back into it.
    ///
    /// If manager creation fails, Rust never took the reference — the
    /// caller must balance the retain itself (see `configure`).
    func makeCallbacks() -> EventHandlerCallbacks {
        var callbacks = EventHandlerCallbacks()
        callbacks.context = Unmanaged.passRetained(self).toOpaque()
        callbacks.release_fn = { context in
            guard let context else { return }
            Unmanaged<PlatformWalletEventHandler>.fromOpaque(context).release()
        }
        callbacks.on_platform_address_sync_completed_fn = platformAddressSyncCompletedCallback
        callbacks.on_shielded_sync_completed_fn = shieldedSyncCompletedCallback
        callbacks.on_shielded_sync_progress_fn = shieldedSyncProgressCallback
        callbacks.on_shielded_tree_progress_fn = shieldedTreeProgressCallback
        return callbacks
    }
}

private func platformAddressSyncCompletedCallback(
    context: UnsafeMutableRawPointer?,
    resultsPtr: UnsafePointer<PlatformAddressSyncWalletResultFFI>?,
    count: UInt,
    syncUnixSeconds: UInt64
) {
    guard let context else { return }

    let handler = Unmanaged<PlatformWalletEventHandler>
        .fromOpaque(context)
        .takeUnretainedValue()

    var results: [PlatformAddressWalletSyncResult] = []
    if let resultsPtr, count > 0 {
        results.reserveCapacity(Int(count))
        for i in 0..<Int(count) {
            results.append(PlatformAddressWalletSyncResult(ffi: resultsPtr[i]))
        }
    }

    let event = PlatformAddressSyncEvent(
        syncUnixSeconds: syncUnixSeconds,
        walletResults: results
    )

    // Snapshot the generation now, on the FFI callback thread, BEFORE the
    // main-actor hop, so a stop/reset that bumps the counter after this
    // point invalidates the trailing event (mirrors the shielded path).
    let generation = handler.manager?.platformAddressSyncGeneration.current() ?? 0

    Task { @MainActor [weak manager = handler.manager] in
        manager?.handlePlatformAddressSyncCompleted(event, generation: generation)
    }
}

private extension AddressSyncMetrics {
    init(platformFFI ffi: PlatformAddressSyncMetricsFFI) {
        self.trunkQueries = ffi.trunk_queries
        self.branchQueries = ffi.branch_queries
        self.totalElementsSeen = ffi.total_elements_seen
        self.totalProofBytes = ffi.total_proof_bytes
        self.iterations = ffi.iterations
        self.compactedQueries = ffi.compacted_queries
        self.recentQueries = ffi.recent_queries
        self.recentEntriesReturned = ffi.recent_entries_returned
        self.compactedEntriesReturned = ffi.compacted_entries_returned
    }
}

extension PlatformWalletManager {
    func handlePlatformAddressSyncCompleted(_ event: PlatformAddressSyncEvent, generation: UInt64) {
        // Drop a trailing event the Rust drain already dispatched but the
        // main actor only delivers after a stop/reset bumped the counter —
        // its snapshot predates the bump. Without this, a completion from a
        // pass drained by `resetPlatformAddressSyncState` (Clear) repaints
        // chain-tip height, last-sync time, and metrics over the freshly
        // cleared UI. Mirrors the shielded guard.
        guard generation == platformAddressSyncGeneration.current() else { return }
        lastPlatformAddressSyncEvent = event
    }

    public func startPlatformAddressSync(
        intervalSeconds: UInt64? = nil,
        config: AddressSyncConfig? = nil
    ) throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }

        if let intervalSeconds {
            try setPlatformAddressSyncInterval(seconds: intervalSeconds)
        }
        if let config {
            try setPlatformAddressSyncConfig(config)
        }

        try platform_wallet_manager_platform_address_sync_start(handle).check()
    }

    public func stopPlatformAddressSync() throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }

        try platform_wallet_manager_platform_address_sync_stop(handle).check()
        // The Rust drain returned; bump the generation so any trailing
        // completion the main actor delivers after this point is dropped
        // (its snapshot predates this bump). Mirrors the shielded stop.
        platformAddressSyncGeneration.bump()
    }

    public func isPlatformAddressSyncRunning() throws -> Bool {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }

        var running = false
        try platform_wallet_manager_platform_address_sync_is_running(handle, &running).check()
        return running
    }

    /// Whether the native manager has frozen its durable sync watermark this
    /// session (dashpay/platform#4069). `true` means the wallet-event adapter
    /// dropped record-bearing events, or a persistence `store()` was rejected,
    /// so the persisted `syncedHeight` is deliberately held behind the chain
    /// tip and a rescan is pending on the next launch. Poll this to surface a
    /// hard "verification failed / rescan pending" state instead of leaving
    /// the fault visible only in the error logs.
    ///
    /// The flag latches for this native manager's lifetime: once `true` it stays
    /// `true` until the manager is destroyed.
    public func syncFaultDetected() throws -> Bool {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }

        var detected = false
        try platform_wallet_manager_sync_fault_detected(handle, &detected).check()
        return detected
    }

    public func isPlatformAddressSyncing() throws -> Bool {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }

        var syncing = false
        try platform_wallet_manager_platform_address_sync_is_syncing(handle, &syncing).check()
        return syncing
    }

    public func lastPlatformAddressSyncUnixSeconds() throws -> UInt64 {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }

        var lastSyncUnixSeconds: UInt64 = 0
        try platform_wallet_manager_platform_address_sync_last_sync_unix_seconds(
            handle,
            &lastSyncUnixSeconds
        ).check()
        return lastSyncUnixSeconds
    }

    public func setPlatformAddressSyncInterval(seconds: UInt64) throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }

        try platform_wallet_manager_platform_address_sync_set_interval(handle, seconds).check()
    }

    public func setPlatformAddressSyncConfig(_ config: AddressSyncConfig?) throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }

        if let config {
            var ffiConfig = AddressSyncConfigFFI(
                min_privacy_count: config.minPrivacyCount,
                max_concurrent_requests: config.maxConcurrentRequests,
                max_iterations: config.maxIterations,
                full_rescan_after_time_s: config.fullRescanAfterTimeSeconds
            )
            try withUnsafePointer(to: &ffiConfig) { configPtr in
                try platform_wallet_manager_platform_address_sync_set_config(
                    handle, configPtr
                ).check()
            }
        } else {
            try platform_wallet_manager_platform_address_sync_set_config(handle, nil).check()
        }
    }

    public func syncPlatformAddressNow() async throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }

        let handle = self.handle
        try await Task.detached(priority: .userInitiated) {
            try platform_wallet_manager_platform_address_sync_sync_now(handle).check()
        }.value
    }

    /// Reset the platform-address (BLAST/DIP-17) incremental-sync
    /// watermark and drop every cached balance across all registered
    /// wallets, forcing a full rescan on the next sync. Backs the
    /// SwiftExampleApp Platform Sync "Clear" button.
    ///
    /// Quiesces the background sync loop before resetting (so no
    /// in-flight pass re-writes the watermark) and leaves it stopped —
    /// callers re-arm via `startPlatformAddressSync` or one-shot
    /// `syncPlatformAddressNow`. Runs off the main actor because the
    /// quiesce drains any in-flight pass.
    public func resetPlatformAddressSyncState() async throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }

        let handle = self.handle
        try await Task.detached(priority: .userInitiated) {
            try platform_wallet_manager_platform_address_sync_reset(handle).check()
        }.value
        // The Rust reset quiesced + drained the in-flight pass; bump the
        // generation so a trailing completion captured before this point
        // (and delivered onto the main actor after Clear) is dropped
        // instead of repainting the just-cleared sync-status UI.
        platformAddressSyncGeneration.bump()
        // Drop the retained published mirror too: the generation guard only
        // blocks future stale callbacks, but a later `configure()` would
        // replay the current `@Published` value to its fresh subscriber and
        // repaint the cleared UI.
        resetPlatformAddressPublishedMirror()
    }
}
