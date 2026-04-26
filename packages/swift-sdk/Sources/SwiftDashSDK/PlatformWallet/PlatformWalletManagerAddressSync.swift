import Foundation

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
        self.foundCount = ffi.found_count
        self.absentCount = ffi.absent_count
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

final class PlatformWalletEventHandler {
    weak var manager: PlatformWalletManager?

    init(manager: PlatformWalletManager) {
        self.manager = manager
    }

    func makeCallbacks() -> EventHandlerCallbacks {
        var callbacks = EventHandlerCallbacks()
        callbacks.context = Unmanaged.passUnretained(self).toOpaque()
        callbacks.on_platform_address_sync_completed_fn = platformAddressSyncCompletedCallback
        return callbacks
    }
}

private func platformAddressSyncCompletedCallback(
    context: UnsafeMutableRawPointer?,
    resultsRaw: UnsafeRawPointer?,
    count: Int,
    syncUnixSeconds: UInt64
) {
    guard let context else { return }

    let handler = Unmanaged<PlatformWalletEventHandler>
        .fromOpaque(context)
        .takeUnretainedValue()

    var results: [PlatformAddressWalletSyncResult] = []
    if let resultsRaw, count > 0 {
        let resultsPtr = resultsRaw.assumingMemoryBound(to: PlatformAddressSyncWalletResultFFI.self)
        results.reserveCapacity(count)
        for i in 0..<count {
            results.append(PlatformAddressWalletSyncResult(ffi: resultsPtr[i]))
        }
    }

    let event = PlatformAddressSyncEvent(
        syncUnixSeconds: syncUnixSeconds,
        walletResults: results
    )

    Task { @MainActor [weak manager = handler.manager] in
        manager?.handlePlatformAddressSyncCompleted(event)
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
    func handlePlatformAddressSyncCompleted(_ event: PlatformAddressSyncEvent) {
        lastPlatformAddressSyncEvent = event
    }

    public func startPlatformAddressSync(
        intervalSeconds: UInt64? = nil,
        config: AddressSyncConfig? = nil
    ) throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle
        }

        if let intervalSeconds {
            try setPlatformAddressSyncInterval(seconds: intervalSeconds)
        }
        if let config {
            try setPlatformAddressSyncConfig(config)
        }

        var error = PlatformWalletFFIError()
        let result = platform_wallet_manager_platform_address_sync_start(handle, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    public func stopPlatformAddressSync() throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle
        }

        var error = PlatformWalletFFIError()
        let result = platform_wallet_manager_platform_address_sync_stop(handle, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    public func isPlatformAddressSyncRunning() throws -> Bool {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle
        }

        var running = false
        var error = PlatformWalletFFIError()
        let result = platform_wallet_manager_platform_address_sync_is_running(
            handle,
            &running,
            &error
        )
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
        return running
    }

    public func isPlatformAddressSyncing() throws -> Bool {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle
        }

        var syncing = false
        var error = PlatformWalletFFIError()
        let result = platform_wallet_manager_platform_address_sync_is_syncing(
            handle,
            &syncing,
            &error
        )
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
        return syncing
    }

    public func lastPlatformAddressSyncUnixSeconds() throws -> UInt64 {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle
        }

        var lastSyncUnixSeconds: UInt64 = 0
        var error = PlatformWalletFFIError()
        let result = platform_wallet_manager_platform_address_sync_last_sync_unix_seconds(
            handle,
            &lastSyncUnixSeconds,
            &error
        )
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
        return lastSyncUnixSeconds
    }

    public func setPlatformAddressSyncInterval(seconds: UInt64) throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle
        }

        var error = PlatformWalletFFIError()
        let result = platform_wallet_manager_platform_address_sync_set_interval(
            handle,
            seconds,
            &error
        )
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    public func setPlatformAddressSyncConfig(_ config: AddressSyncConfig?) throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle
        }

        var error = PlatformWalletFFIError()
        let result: PlatformWalletFFIResult
        if let config {
            var ffiConfig = AddressSyncConfigFFI(
                min_privacy_count: config.minPrivacyCount,
                max_concurrent_requests: config.maxConcurrentRequests,
                max_iterations: config.maxIterations,
                full_rescan_after_time_s: config.fullRescanAfterTimeSeconds
            )
            result = withUnsafePointer(to: &ffiConfig) { configPtr in
                platform_wallet_manager_platform_address_sync_set_config(handle, configPtr, &error)
            }
        } else {
            result = platform_wallet_manager_platform_address_sync_set_config(handle, nil, &error)
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    public func syncPlatformAddressNow() async throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle
        }

        let handle = self.handle
        try await Task.detached(priority: .userInitiated) {
            var error = PlatformWalletFFIError()
            let result = platform_wallet_manager_platform_address_sync_sync_now(handle, &error)
            guard result == Success else {
                throw PlatformWalletError(result: result, error: error)
            }
        }.value
    }
}
