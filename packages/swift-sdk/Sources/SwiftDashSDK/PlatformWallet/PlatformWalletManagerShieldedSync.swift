import Foundation
import DashSDKFFI

/// Per-wallet outcome from a completed shielded sync pass.
///
/// Mirrors the Rust-side
/// [`ShieldedSyncWalletResultFFI`](https://github.com/dashpay/platform/blob/v3.1-dev/packages/rs-platform-wallet-ffi/src/shielded_types.rs)
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
    func handleShieldedSyncCompleted(_ event: ShieldedSyncEvent) {
        lastShieldedSyncEvent = event
    }

    /// Derive Orchard keys for `walletId` from the host-side mnemonic
    /// resolver, open or create the per-network commitment tree at
    /// `dbPath`, and bind the resulting shielded sub-wallet to the
    /// `PlatformWallet`.
    ///
    /// The resolver is fired exactly once. The mnemonic and the
    /// derived seed live in zeroized buffers on the Rust side and
    /// are scrubbed before this call returns; only the FVK / IVK /
    /// OVK / default address survive on the wallet handle.
    ///
    /// Idempotent: calling again with a different account or
    /// `dbPath` replaces the previously-bound shielded wallet.
    public func bindShielded(
        walletId: Data,
        resolver: MnemonicResolver,
        account: UInt32 = 0,
        dbPath: String
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
            try dbPath.withCString { dbPathPtr in
                try platform_wallet_manager_bind_shielded(
                    handle,
                    walletIdPtr,
                    resolverHandle,
                    account,
                    dbPathPtr
                ).check()
            }
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
        try platform_wallet_manager_shielded_sync_start(handle).check()
    }

    public func stopShieldedSync() throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        try platform_wallet_manager_shielded_sync_stop(handle).check()
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
        let handle = self.handle
        try await Task.detached(priority: .userInitiated) {
            try platform_wallet_manager_shielded_sync_sync_now(handle).check()
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

    Task { @MainActor [weak manager = handler.manager] in
        manager?.handleShieldedSyncCompleted(event)
    }
}
