import Foundation
import DashSDKFFI

/// Per-pass summary returned by
/// `PlatformWalletManager.dashPaySyncNow()`. Mirrors the FFI
/// out-params of `platform_wallet_manager_dashpay_sync_sync_now`.
///
/// `success == 0 && errors == 0 && syncUnixSeconds == 0` is the
/// "no pass ran" sentinel — a pass was already in flight (e.g. fired
/// by the background loop) and the manager skipped. Check
/// `isDashPaySyncing()` to distinguish "skipped" from "swept zero
/// wallets".
public struct DashPaySyncSummary: Sendable, Equatable {
    /// Wallets whose `dashpay_sync()` succeeded in this pass.
    public let success: Int
    /// Wallets whose `dashpay_sync()` failed (logged Rust-side,
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

extension PlatformWalletManager {
    // MARK: - DashPay sync lifecycle
    //
    // Mirrors the identity-token / shielded coordinators: NOT
    // auto-started. The host lifecycle calls `startDashPaySync()`
    // once the wallets are registered and the SDK is connected
    // (same place it starts the address / shielded loops); the
    // on-demand `dashPaySyncNow()` entry point backs pull-to-refresh.
    // Unlike the identity-token coordinator the DashPay sweep is
    // wallet-driven — every registered wallet is swept on every pass,
    // so there is no per-identity registry surface here.

    /// Start the recurring DashPay (contact-request + profile) sync
    /// background loop. Idempotent — calling while already running is
    /// a no-op.
    public func startDashPaySync(intervalSeconds: UInt64? = nil) throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        if let intervalSeconds {
            try setDashPaySyncInterval(seconds: intervalSeconds)
        }
        try platform_wallet_manager_dashpay_sync_start(handle).check()
    }

    /// Stop the recurring DashPay sync loop if it is running.
    /// Cancel-only: a pass already in flight keeps running to
    /// completion.
    public func stopDashPaySync() throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        try platform_wallet_manager_dashpay_sync_stop(handle).check()
    }

    /// Whether the DashPay sync background loop is running.
    public func isDashPaySyncRunning() throws -> Bool {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        var running = false
        try platform_wallet_manager_dashpay_sync_is_running(handle, &running).check()
        return running
    }

    /// Whether a DashPay sync pass is currently in flight.
    ///
    /// Prefer observing the published mirror
    /// [`PlatformWalletManager.dashPaySyncIsSyncing`] from SwiftUI;
    /// this is the direct FFI read backing it.
    public func isDashPaySyncing() throws -> Bool {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        var syncing = false
        try platform_wallet_manager_dashpay_sync_is_syncing(handle, &syncing).check()
        return syncing
    }

    /// Unix seconds of the last completed DashPay sync pass, or 0 if
    /// no pass has ever completed. The watermark is global (one
    /// last-sync per manager, not per-identity) — the sweep is
    /// wallet-driven.
    public func dashPayLastSyncUnixSeconds() throws -> UInt64 {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        var lastSync: UInt64 = 0
        try platform_wallet_manager_dashpay_sync_last_sync_unix_seconds(handle, &lastSync).check()
        return lastSync
    }

    /// Set the background DashPay sync interval (clamped to >= 1
    /// second on the Rust side). The running loop picks the new
    /// interval up on its next sleep.
    public func setDashPaySyncInterval(seconds: UInt64) throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        try platform_wallet_manager_dashpay_sync_set_interval(handle, seconds).check()
    }

    /// Run one DashPay sync pass across every registered wallet.
    /// Synchronous from the FFI side — runs on a detached worker
    /// `Task`. If a pass is already in flight, the Rust manager skips
    /// and the returned summary is the all-zero "no pass ran"
    /// sentinel (see [`DashPaySyncSummary`]).
    ///
    /// This is the pull-to-refresh entry point: a refresh during
    /// an in-flight sync attaches to it (skip + sentinel) instead of
    /// double-firing.
    @discardableResult
    public func dashPaySyncNow() async throws -> DashPaySyncSummary {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) { () -> DashPaySyncSummary in
            var successCount: UInt = 0
            var errorCount: UInt = 0
            var syncUnixSeconds: UInt64 = 0
            try platform_wallet_manager_dashpay_sync_sync_now(
                handle,
                &successCount,
                &errorCount,
                &syncUnixSeconds
            ).check()
            return DashPaySyncSummary(
                success: Int(successCount),
                errors: Int(errorCount),
                syncUnixSeconds: syncUnixSeconds
            )
        }.value
    }

    // MARK: - DashPay payment history refresh

    /// Refresh the persisted DashPay payment history for one
    /// identity: one FFI read
    /// (`managed_identity_get_dashpay_payments`) + one persistence
    /// pass upserting `PersistentDashpayPayment` rows, so the UI can
    /// `@Query` them.
    ///
    /// Requires the manager to have been configured with a
    /// `ModelContainer` (no-persistence mode has nowhere to land the
    /// rows). Throws `.identityNotFound` when no loaded wallet knows
    /// this identity.
    @discardableResult
    public func refreshDashPayPayments(
        walletId: Data,
        identityId: Identifier
    ) throws -> [DashPayPayment] {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        guard let wallet = wallets[walletId] else {
            throw PlatformWalletError.invalidParameter(
                "no loaded wallet with id \(walletId.toHexString())"
            )
        }
        guard let persistenceHandler = persistence else {
            throw PlatformWalletError.invalidHandle(
                "refreshDashPayPayments requires a persistence handler — configure the manager with a ModelContainer"
            )
        }
        let payments = try wallet.getDashPayPayments(identityId: identityId)
        persistenceHandler.persistDashpayPayments(
            ownerIdentityId: identityId,
            payments: payments
        )
        return payments
    }
}
