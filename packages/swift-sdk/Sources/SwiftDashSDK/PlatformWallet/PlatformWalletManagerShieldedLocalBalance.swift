import Foundation
import DashSDKFFI

/// Availability of a local shielded ledger, without contacting the network.
public enum ShieldedLocalBalanceState: Sendable, Equatable {
    /// No shielded coordinator or bound accounts exist for this wallet.
    case unbound
    /// Bound accounts exist, but local restoration has not completed.
    case restoreIncomplete
    case ready(ShieldedLocalBalanceSnapshot)
}

/// A local read completed, but its result cannot be delivered for the current
/// Swift binding lifecycle. Callers own whether and when to request another read.
public enum ShieldedLocalBalanceReadError: LocalizedError, Sendable, Equatable {
    /// A successful bind overlapped delivery, including an idempotent bind.
    /// No transaction was attempted; a fresh snapshot read is safe.
    case bindingChanged

    public var errorDescription: String? {
        "Shielded binding changed during the local balance snapshot; request a fresh read"
    }
}

public struct ShieldedLocalBalanceSnapshot: Sendable, Equatable {
    /// Every bound account is present, including accounts with zero credits.
    public let accounts: [UInt32: ShieldedLocalAccountBalance]

    public init(accounts: [UInt32: ShieldedLocalAccountBalance]) {
        self.accounts = accounts
    }
}

public struct ShieldedLocalAccountBalance: Sendable, Equatable {
    /// Locally selectable credits, excluding spent notes and pending spend
    /// reservations. A new network scan may change this amount.
    public let spendableCredits: UInt64
    /// The account's local scan coverage. A persisted index of zero is
    /// distinct from absent scan history (`nil`).
    public let lastScannedIndex: UInt64?
    public let source: ShieldedBalanceSource

    public init(
        spendableCredits: UInt64,
        lastScannedIndex: UInt64?,
        source: ShieldedBalanceSource
    ) {
        self.spendableCredits = spendableCredits
        self.lastScannedIndex = lastScannedIndex
        self.source = source
    }
}

public enum ShieldedBalanceSource: Sendable, Equatable {
    /// Local initialization succeeded, but no persisted ledger or completed
    /// scan is known. Zero does not establish that the wallet has no funds.
    case noHistory
    /// Notes and/or a scan-state row were restored from local persistence.
    /// Legacy notes can be restored without a scan index.
    case restored
    /// A complete successful scan occurred in this manager session.
    case scannedThisSession
}

/// Raw native values never cross the continuation: the call, decoding, and
/// release all run on the same queue. Tests inject matching Swift allocations
/// and release closures so Rust never frees memory allocated by Swift.
struct PlatformWalletNativeShieldedLocalBalanceCalls: Sendable {
    typealias Read = @Sendable (Handle, Data)
        -> (result: PlatformWalletFFIResult, snapshot: ShieldedLocalBalanceSnapshotFFI)
    typealias Free = @Sendable (UnsafeMutablePointer<ShieldedLocalBalanceSnapshotFFI>) -> Void

    let read: Read
    let free: Free

    static let live = PlatformWalletNativeShieldedLocalBalanceCalls(
        read: { handle, walletId in
            var snapshot = ShieldedLocalBalanceSnapshotFFI()
            let result = walletId.withUnsafeBytes { bytes in
                platform_wallet_manager_local_shielded_balance_snapshot(
                    handle, bytes.bindMemory(to: UInt8.self).baseAddress, &snapshot)
            }
            return (result, snapshot)
        },
        free: platform_wallet_manager_local_shielded_balance_snapshot_free
    )
}

extension PlatformWalletManager {
    /// Clear can purge notes before a later storage error is returned.
    /// Invalidate before entering native code, including its failure path.
    func withShieldedLocalBalanceMutation<T>(_ body: () throws -> T) rethrows -> T {
        shieldedLocalBalanceGeneration.bump()
        return try body()
    }

    /// Native bind does not report whether it preserved or changed the
    /// registration. Mark queued delivery obsolete after either successful
    /// outcome; discard on failure because it cannot certify the old ledger.
    /// The synchronous MainActor call and epoch update cannot interleave
    /// with a snapshot's MainActor delivery.
    func withShieldedLocalBalanceBind<T>(_ body: () throws -> T) rethrows -> T {
        do {
            let result = try body()
            shieldedLocalBalanceBindGeneration.bump()
            return result
        } catch {
            shieldedLocalBalanceGeneration.bump()
            throw error
        }
    }

    /// Reads one coherent local balance snapshot after shielded binding.
    /// This performs no network requests. Call it before starting network
    /// sync during launch. Native lock waits have a short deadline: if an
    /// active scan holds the store across a network request, this throws
    /// instead of blocking native-operation admission until the scan ends.
    ///
    /// Each invocation performs one native read. A successful bind overlapping
    /// delivery throws `ShieldedLocalBalanceReadError.bindingChanged`; the host
    /// decides whether and when to request a fresh read. Clear, stop, failed bind,
    /// and task cancellation discard obsolete delivery with `CancellationError`.
    ///
    /// Native failures throw; unbound and incompletely restored wallets do
    /// not produce numeric balances. Retain the last usable snapshot when a
    /// read fails. The manager remains alive until native read, allocation
    /// release, and delivery finish; shutdown drains admitted reads.
    public func localShieldedBalanceSnapshot(walletId: Data) async throws -> ShieldedLocalBalanceState {
        try Task.checkCancellation()
        try ensureConfigured()
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter("walletId must be exactly 32 bytes")
        }
        try admitNativeOp("localShieldedBalanceSnapshot")
        defer { finishNativeOp() }

        let h = handle
        let generation = shieldedSyncGeneration.current()
        let localGeneration = shieldedLocalBalanceGeneration.current()
        let calls = nativeShieldedLocalBalanceCalls
        let bindGeneration = shieldedLocalBalanceBindGeneration.current()
        let state: ShieldedLocalBalanceState = try await withCheckedThrowingContinuation { continuation in
            // Read-only native access has its own lifecycle/store guards.
            // A different manager's teardown must not delay this read.
            shieldedLocalBalanceQueue.async {
                do {
                    continuation.resume(returning: try Self.readLocalShieldedBalance(
                        handle: h, walletId: walletId, calls: calls))
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
        try Task.checkCancellation()
        guard handle == h, generation == shieldedSyncGeneration.current(),
              localGeneration == shieldedLocalBalanceGeneration.current() else {
            // Clear, stop, or a failed bind makes this request obsolete.
            throw CancellationError()
        }
        guard bindGeneration == shieldedLocalBalanceBindGeneration.current() else {
            throw ShieldedLocalBalanceReadError.bindingChanged
        }
        return state
    }

    nonisolated static func readLocalShieldedBalance(
        handle: Handle,
        walletId: Data,
        calls: PlatformWalletNativeShieldedLocalBalanceCalls
    ) throws -> ShieldedLocalBalanceState {
        let response = calls.read(handle, walletId)
        var snapshot = response.snapshot
        // Release on native errors as well as decoding errors and success.
        // The FFI initializes the out value on every return path.
        defer { calls.free(&snapshot) }
        try response.result.check()
        return try decodeLocalShieldedBalance(snapshot)
    }

    nonisolated static func decodeLocalShieldedBalance(
        _ snapshot: ShieldedLocalBalanceSnapshotFFI
    ) throws -> ShieldedLocalBalanceState {
        switch snapshot.status {
        case SHIELDED_LOCAL_BALANCE_STATUS_FFI_UNBOUND:
            guard snapshot.accounts_count == 0 else {
                throw PlatformWalletError.deserialization("Unbound shielded snapshot contains accounts")
            }
            return .unbound
        case SHIELDED_LOCAL_BALANCE_STATUS_FFI_RESTORE_INCOMPLETE:
            guard snapshot.accounts_count == 0 else {
                throw PlatformWalletError.deserialization("Incomplete shielded snapshot contains accounts")
            }
            return .restoreIncomplete
        case SHIELDED_LOCAL_BALANCE_STATUS_FFI_READY:
            break
        default:
            throw PlatformWalletError.deserialization("Unknown local shielded balance status")
        }

        guard let count = Int(exactly: snapshot.accounts_count), count > 0,
              count <= Int.max / MemoryLayout<ShieldedLocalAccountBalanceFFI>.stride,
              let entries = snapshot.accounts else {
            throw PlatformWalletError.deserialization("Ready shielded snapshot has invalid account storage")
        }
        var accounts: [UInt32: ShieldedLocalAccountBalance] = [:]
        for entry in UnsafeBufferPointer(start: entries, count: count) {
            let source: ShieldedBalanceSource
            switch entry.source {
            case SHIELDED_BALANCE_SOURCE_FFI_NO_HISTORY: source = .noHistory
            case SHIELDED_BALANCE_SOURCE_FFI_RESTORED: source = .restored
            case SHIELDED_BALANCE_SOURCE_FFI_SCANNED_THIS_SESSION: source = .scannedThisSession
            default:
                throw PlatformWalletError.deserialization("Unknown local shielded balance source")
            }
            guard accounts[entry.account_index] == nil else {
                throw PlatformWalletError.deserialization("Duplicate shielded account in local snapshot")
            }
            accounts[entry.account_index] = ShieldedLocalAccountBalance(
                spendableCredits: entry.spendable_credits,
                lastScannedIndex: entry.has_last_scanned_index ? entry.last_scanned_index : nil,
                source: source)
        }
        return .ready(ShieldedLocalBalanceSnapshot(accounts: accounts))
    }
}
