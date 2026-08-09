import Foundation
import DashSDKFFI

/// Per-pass summary returned by
/// `PlatformWalletManager.dpnsSyncNow()`. Mirrors the FFI out-params of
/// `platform_wallet_manager_dpns_sync_sync_now`.
///
/// `success == 0 && errors == 0 && syncUnixSeconds == 0` is the "no pass
/// ran" sentinel — a pass was already in flight (e.g. fired by the
/// background loop) and the manager skipped. Check `isDpnsSyncing()` to
/// distinguish "skipped" from "swept zero wallets".
///
/// This is the cross-wallet coordinator's summary. For one wallet's
/// marketplace delta (names tracked / added / departed / re-priced) use
/// `ManagedPlatformWallet.syncDpnsMarketplace()`.
public struct DpnsSyncSummary: Sendable, Equatable {
    /// Wallets whose marketplace sync succeeded in this pass.
    public let success: Int
    /// Wallets whose marketplace sync failed (logged Rust-side,
    /// non-fatal to the rest of the pass).
    public let errors: Int
    /// Unix seconds the pass completed, or 0 if no pass ran.
    public let syncUnixSeconds: UInt64

    public init(success: Int, errors: Int, syncUnixSeconds: UInt64) {
        self.success = success
        self.errors = errors
        self.syncUnixSeconds = syncUnixSeconds
    }
}

/// One registered wallet's result in a completed manager-wide DPNS
/// marketplace sync pass.
public struct DpnsWalletSyncResult: Sendable, Equatable {
    public let walletId: Data
    public let success: Bool
    public let namesTracked: UInt32
    public let namesAdded: UInt32
    public let namesDeparted: UInt32
    public let pricesChanged: UInt32
    public let errorMessage: String?

    public init(
        walletId: Data,
        success: Bool,
        namesTracked: UInt32,
        namesAdded: UInt32,
        namesDeparted: UInt32,
        pricesChanged: UInt32,
        errorMessage: String?
    ) {
        self.walletId = walletId
        self.success = success
        self.namesTracked = namesTracked
        self.namesAdded = namesAdded
        self.namesDeparted = namesDeparted
        self.pricesChanged = pricesChanged
        self.errorMessage = errorMessage
    }

    /// Copy callback-duration native storage into an owned Swift value.
    init(ffi: DpnsSyncWalletResultFFI) {
        var walletIdTuple = ffi.wallet_id
        self.init(
            walletId: Swift.withUnsafeBytes(of: &walletIdTuple) { Data($0) },
            success: ffi.success,
            namesTracked: ffi.names_tracked,
            namesAdded: ffi.names_added,
            namesDeparted: ffi.names_departed,
            pricesChanged: ffi.prices_changed,
            errorMessage: ffi.error_message.map { String(cString: $0) }
        )
    }
}

/// One completed manager-wide DPNS marketplace sync pass.
public struct DpnsSyncEvent: Sendable, Equatable {
    public let syncUnixSeconds: UInt64
    public let walletResults: [DpnsWalletSyncResult]

    public init(syncUnixSeconds: UInt64, walletResults: [DpnsWalletSyncResult]) {
        self.syncUnixSeconds = syncUnixSeconds
        self.walletResults = walletResults
    }

    public func result(for walletId: Data) -> DpnsWalletSyncResult? {
        walletResults.first { $0.walletId == walletId }
    }
}

/// C trampoline for
/// `EventHandlerCallbacksExtension.on_dpns_marketplace_sync_completed_fn`.
/// Rust owns every pointer only for this call, so the complete result array
/// and every error string are copied before hopping to the main actor.
func dpnsMarketplaceSyncCompletedCallback(
    context: UnsafeMutableRawPointer?,
    resultsPtr: UnsafePointer<DpnsSyncWalletResultFFI>?,
    count: UInt,
    syncUnixSeconds: UInt64
) {
    guard let context else { return }

    let handler = Unmanaged<PlatformWalletEventHandler>
        .fromOpaque(context)
        .takeUnretainedValue()

    var results: [DpnsWalletSyncResult] = []
    if let resultsPtr, count > 0 {
        results.reserveCapacity(Int(count))
        for index in 0..<Int(count) {
            results.append(DpnsWalletSyncResult(ffi: resultsPtr[index]))
        }
    }

    let event = DpnsSyncEvent(
        syncUnixSeconds: syncUnixSeconds,
        walletResults: results
    )
    Task { @MainActor [weak manager = handler.manager] in
        manager?.handleDpnsSyncCompleted(event)
    }
}

extension PlatformWalletManager {
    // MARK: - DPNS marketplace sync lifecycle
    //
    // Mirrors the DashPay coordinator: NOT auto-started. The host
    // lifecycle calls `startDpnsSync()` once the wallets are registered
    // and the SDK is connected; the on-demand `dpnsSyncNow()` entry point
    // backs pull-to-refresh. The sweep is wallet-driven — every
    // registered wallet is swept on every pass — so there is no
    // per-identity registry surface here. It runs on a slower default
    // cadence (60s) than DashPay's 15s because marketplace state changes
    // are rare.

    func handleDpnsSyncCompleted(_ event: DpnsSyncEvent) {
        lastDpnsSyncEvent = event
    }

    /// Start the recurring DPNS username-marketplace sync background
    /// loop. Idempotent — calling while already running is a no-op.
    public func startDpnsSync(intervalSeconds: UInt64? = nil) throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        if let intervalSeconds {
            try setDpnsSyncInterval(seconds: intervalSeconds)
        }
        try platform_wallet_manager_dpns_sync_start(handle).check()
    }

    /// Stop the recurring DPNS marketplace sync loop if it is running.
    /// Cancel-only: a pass already in flight keeps running to completion.
    public func stopDpnsSync() throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        try platform_wallet_manager_dpns_sync_stop(handle).check()
    }

    /// Whether the DPNS marketplace sync background loop is running.
    public func isDpnsSyncRunning() throws -> Bool {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        var running = false
        try platform_wallet_manager_dpns_sync_is_running(handle, &running).check()
        return running
    }

    /// Whether a DPNS marketplace sync pass is currently in flight.
    public func isDpnsSyncing() throws -> Bool {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        var syncing = false
        try platform_wallet_manager_dpns_sync_is_syncing(handle, &syncing).check()
        return syncing
    }

    /// Unix seconds of the last completed DPNS marketplace sync pass, or
    /// 0 if no pass has ever completed. The watermark is global (one
    /// last-sync per manager, not per-wallet) — the sweep is
    /// wallet-driven.
    public func dpnsLastSyncUnixSeconds() throws -> UInt64 {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        var lastSync: UInt64 = 0
        try platform_wallet_manager_dpns_sync_last_sync_unix_seconds(handle, &lastSync).check()
        return lastSync
    }

    /// Set the background DPNS marketplace sync interval (clamped to >= 1
    /// second on the Rust side). The running loop picks the new interval
    /// up on its next sleep.
    public func setDpnsSyncInterval(seconds: UInt64) throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        try platform_wallet_manager_dpns_sync_set_interval(handle, seconds).check()
    }

    /// Run one DPNS marketplace sync pass across every registered wallet.
    /// Synchronous from the FFI side — runs on a detached worker `Task`.
    /// If a pass is already in flight, the Rust manager skips and the
    /// returned summary is the all-zero "no pass ran" sentinel (see
    /// ``DpnsSyncSummary``).
    @discardableResult
    public func dpnsSyncNow() async throws -> DpnsSyncSummary {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) { () -> DpnsSyncSummary in
            var successCount: UInt = 0
            var errorCount: UInt = 0
            var syncUnixSeconds: UInt64 = 0
            try platform_wallet_manager_dpns_sync_sync_now(
                handle,
                &successCount,
                &errorCount,
                &syncUnixSeconds
            ).check()
            return DpnsSyncSummary(
                success: Int(successCount),
                errors: Int(errorCount),
                syncUnixSeconds: syncUnixSeconds
            )
        }.value
    }
}
