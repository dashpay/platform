import Foundation
import DashSDKFFI

/// Whether the wallet is automatically retrying an identity-funded shield.
/// This status describes recovery, separately from the activity's outcome.
public enum ShieldedIdentityDebitRecoveryStatus: UInt8, Sendable {
    case retrying = 0
    case parked = 1
    /// Automatic retry was explicitly abandoned; execution is still possible.
    case unknown = 2
}

/// A durable identity-funded shield record. Identify it with the wallet id used
/// to list it plus `accountIndex` and `activityId`. Unreadable records retain
/// their scope; identity, nonce and amount are nil when unavailable.
public struct ShieldedIdentityDebitRecoveryRecord: Sendable, Equatable {
    public let accountIndex: UInt32
    public let activityId: Data
    public let identityId: Data?
    public let nonce: UInt64?
    public let amount: UInt64?
    public let status: ShieldedIdentityDebitRecoveryStatus

    init(ffi: ShieldedIdentityDebitRecoveryRecordFFI) throws {
        guard let status = ShieldedIdentityDebitRecoveryStatus(rawValue: ffi.status) else {
            throw PlatformWalletError.deserialization("Unknown identity debit recovery status: \(ffi.status)")
        }
        accountIndex = ffi.account_index
        var activity = ffi.activity_id
        activityId = withUnsafeBytes(of: &activity) { Data($0) }
        var identity = ffi.identity_id
        identityId = ffi.has_identity_id ? withUnsafeBytes(of: &identity) { Data($0) } : nil
        nonce = ffi.has_nonce ? ffi.nonce : nil
        amount = ffi.has_amount ? ffi.amount : nil
        self.status = status
    }
}

extension PlatformWalletManager {
    /// List active and archived durable identity-funded shield records for this
    /// manager's wallet. Does not expose private keys or signed transaction bytes.
    public func shieldedIdentityDebitRecoveryRecords(
        walletId: Data
    ) async throws -> [ShieldedIdentityDebitRecoveryRecord] {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter("walletId must be exactly 32 bytes")
        }
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) {
            try walletId.withUnsafeBytes { raw in
                var records: UnsafeMutablePointer<ShieldedIdentityDebitRecoveryRecordFFI>?
                var count: UInt = 0
                let result = platform_wallet_manager_shielded_identity_debit_recovery_records(
                    handle, raw.baseAddress!.assumingMemoryBound(to: UInt8.self), &records, &count
                )
                defer { platform_wallet_shielded_identity_debit_recovery_records_free(records, count) }
                try result.check()
                guard let records else { return [ShieldedIdentityDebitRecoveryRecord]() }
                return try UnsafeBufferPointer(start: records, count: Int(count)).map {
                    try ShieldedIdentityDebitRecoveryRecord(ffi: $0)
                }
            }
        }.value
    }

    /// Stop automatic retry of exactly `(walletId, accountIndex, activityId)`.
    /// Archives the complete signed record for later scan confirmation and audit,
    /// preserves an Unknown outcome, and permits a new identity debit after saving.
    ///
    /// This does not cancel an already relayed or in-flight payment. A new payment
    /// may be an additional debit, even if the original nonce is still usable.
    /// Obtain the user's informed acknowledgement before passing true. The
    /// acknowledgement is required, has no default, and false is rejected.
    public func abandonShieldedIdentityDebit(
        walletId: Data,
        accountIndex: UInt32,
        activityId: Data,
        acknowledgePossibleExecution: Bool
    ) async throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        guard walletId.count == 32, activityId.count == 32 else {
            throw PlatformWalletError.invalidParameter("walletId and activityId must be exactly 32 bytes")
        }
        let handle = self.handle
        try await Task.detached(priority: .userInitiated) {
            try walletId.withUnsafeBytes { wallet in
                try activityId.withUnsafeBytes { activity in
                    try platform_wallet_manager_abandon_shielded_identity_debit(
                        handle,
                        wallet.baseAddress!.assumingMemoryBound(to: UInt8.self),
                        accountIndex,
                        activity.baseAddress!.assumingMemoryBound(to: UInt8.self),
                        acknowledgePossibleExecution
                    ).check()
                }
            }
        }.value
    }
}
