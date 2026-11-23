import Foundation
import DashSDKFFI

// MARK: - Value types

/// The withdrawals contract status of a withdrawal, mirrored from the code
/// `platform_wallet_identity_withdrawals` reports. A code this build does not
/// know is refused when decoding, never mapped to a nearby case.
public enum WithdrawalStatus: UInt8, Sendable, Equatable, CaseIterable {
    /// Waiting in Platform state to be pooled.
    case queued = 0
    /// Pooled into an asset unlock transaction, waiting for a quorum to sign.
    case pooled = 1
    /// Platform built the asset unlock transaction and queued it for a quorum
    /// to sign and for broadcast to Core. This status does not show whether
    /// either has happened: it stays `broadcasted` after Core receives the
    /// signed transaction.
    case broadcasted = 2
    /// Platform saw the transaction in a chain-locked Core block.
    case complete = 3
    /// Core never mined the transaction in time; Platform signs it again.
    case expired = 4
    /// The signed transaction can never be mined (its payout is below Core's
    /// dust threshold).
    case failed = 5
}

/// One withdrawal of an identity's credits to Core, as the withdrawals system
/// contract stores it. Rust decides `status`, `statusLabel` and `isFinal`;
/// this value only carries them.
public struct IdentityWithdrawal: Sendable, Equatable, Identifiable {
    /// The id of the withdrawal document.
    public let id: Data
    /// The identity the credits were withdrawn from.
    public let ownerId: Data
    /// The credits withdrawn, before the Core transaction fee.
    public let amountCredits: UInt64
    /// The Core fee rate the withdrawal offered, in duffs per byte.
    public let coreFeePerByte: UInt32
    /// Where the withdrawal stands.
    public let status: WithdrawalStatus
    /// The status as Rust names it ("Queued", "Pooled", ...).
    public let statusLabel: String
    /// Whether the withdrawal reached a status it never leaves (complete or
    /// failed), as Rust decided it.
    public let isFinal: Bool
    /// The Core address the withdrawal pays on the wallet's network; `nil`
    /// when the output script is not an address.
    public let destination: String?
    /// The Core output script the withdrawal pays.
    public let outputScript: Data
    /// The index of the asset unlock transaction that carries the withdrawal
    /// to Core, once Platform pooled it.
    public let transactionIndex: UInt64?
    /// The Core chain-locked height Platform built the asset unlock
    /// transaction at (its request height), once it built it. Not proof that
    /// a quorum signed the transaction or that Core accepted it.
    public let transactionSignHeight: UInt32?
    /// When the withdrawal was submitted, block time in Unix milliseconds.
    public let createdAtMs: UInt64
    /// When its status last changed, block time in Unix milliseconds.
    public let updatedAtMs: UInt64

    public init(
        id: Data,
        ownerId: Data,
        amountCredits: UInt64,
        coreFeePerByte: UInt32,
        status: WithdrawalStatus,
        statusLabel: String,
        isFinal: Bool,
        destination: String?,
        outputScript: Data,
        transactionIndex: UInt64?,
        transactionSignHeight: UInt32?,
        createdAtMs: UInt64,
        updatedAtMs: UInt64
    ) {
        self.id = id
        self.ownerId = ownerId
        self.amountCredits = amountCredits
        self.coreFeePerByte = coreFeePerByte
        self.status = status
        self.statusLabel = statusLabel
        self.isFinal = isFinal
        self.destination = destination
        self.outputScript = outputScript
        self.transactionIndex = transactionIndex
        self.transactionSignHeight = transactionSignHeight
        self.createdAtMs = createdAtMs
        self.updatedAtMs = updatedAtMs
    }
}

// MARK: - FFI decoding

extension IdentityWithdrawal {
    /// Copy a Rust-owned row into an owned Swift value. Every `has_*` flag
    /// gates its field: a `false` flag becomes `nil`, never the zero the C
    /// struct holds. Throws for a status code this build does not know, and
    /// for a row without its status label.
    ///
    /// Must be called while the Rust allocation is alive: the strings and the
    /// script are copied here, not retained.
    init(ffi: IdentityWithdrawalFFI) throws {
        guard let status = WithdrawalStatus(rawValue: ffi.status) else {
            throw PlatformWalletError.deserialization(
                "unknown withdrawal status code \(ffi.status)"
            )
        }
        guard let labelPtr = ffi.status_label else {
            throw PlatformWalletError.deserialization("withdrawal row has no status label")
        }
        var idTuple = ffi.id
        var ownerTuple = ffi.owner_id
        let outputScript: Data
        if let scriptPtr = ffi.output_script, ffi.output_script_len > 0 {
            outputScript = Data(bytes: scriptPtr, count: Int(ffi.output_script_len))
        } else {
            outputScript = Data()
        }
        self.init(
            id: Swift.withUnsafeBytes(of: &idTuple) { Data($0) },
            ownerId: Swift.withUnsafeBytes(of: &ownerTuple) { Data($0) },
            amountCredits: ffi.amount,
            coreFeePerByte: ffi.core_fee_per_byte,
            status: status,
            statusLabel: String(cString: labelPtr),
            isFinal: ffi.is_final,
            destination: ffi.destination.map { String(cString: $0) },
            outputScript: outputScript,
            transactionIndex: ffi.has_transaction_index ? ffi.transaction_index : nil,
            transactionSignHeight: ffi.has_transaction_sign_height
                ? ffi.transaction_sign_height
                : nil,
            createdAtMs: ffi.created_at_ms,
            updatedAtMs: ffi.updated_at_ms
        )
    }
}

extension IdentityWithdrawal {
    /// Decode the `count` rows `platform_wallet_identity_withdrawals` wrote
    /// at `rows`, in the order Rust gave them. One row this build cannot
    /// decode fails the whole list. Does not free anything.
    static func decodeRows(
        rows: UnsafePointer<IdentityWithdrawalFFI>?,
        count: Int
    ) throws -> [IdentityWithdrawal] {
        guard let rows, count > 0 else { return [] }
        var withdrawals: [IdentityWithdrawal] = []
        withdrawals.reserveCapacity(count)
        for index in 0..<count {
            withdrawals.append(try IdentityWithdrawal(ffi: rows[index]))
        }
        return withdrawals
    }
}

// MARK: - ManagedPlatformWallet

extension ManagedPlatformWallet {
    /// Every withdrawal of `identityId` to Core, each listed once, most
    /// recently updated first, read with proofs from the withdrawals system
    /// contract (protocol version 14 and later). Rust reads every page at one
    /// block height. When no read of the listing stays at one block height in
    /// a few attempts this throws an error that is safe to retry: call it
    /// again. Any identity can be asked for, not only the wallet's.
    /// Withdrawals from Platform addresses and from the shielded pool are not
    /// found: the contract stores them under its own owner.
    public func identityWithdrawals(identityId: Data) async throws -> [IdentityWithdrawal] {
        let handle = self.handle
        guard identityId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "identityId must contain exactly 32 bytes (got \(identityId.count))"
            )
        }
        let idBytes = Array(identityId)
        return try await Task.detached(priority: .userInitiated) { () -> [IdentityWithdrawal] in
            var outRows: UnsafeMutablePointer<IdentityWithdrawalFFI>? = nil
            var outCount: UInt = 0
            let result = idBytes.withUnsafeBufferPointer { idBp -> PlatformWalletFFIResult in
                platform_wallet_identity_withdrawals(
                    handle,
                    idBp.baseAddress!,
                    &outRows,
                    &outCount
                )
            }
            // The free function accepts null and 0, and Rust writes both
            // before any fallible work, so this is safe on every path.
            defer { platform_wallet_identity_withdrawals_free(outRows, outCount) }
            try result.check()
            return try IdentityWithdrawal.decodeRows(
                rows: UnsafePointer(outRows),
                count: Int(outCount)
            )
        }.value
    }
}
