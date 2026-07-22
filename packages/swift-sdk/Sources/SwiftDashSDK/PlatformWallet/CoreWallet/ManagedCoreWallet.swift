import Foundation
import DashSDKFFI

/// Authoritative network outcome for a signed Core transaction.
///
/// `accepted` means Dash Core accepted the transaction into its mempool or
/// already knows it; it does not mean the transaction is mined or
/// InstantSend-locked. `unknown` must not be retried automatically because the
/// Core response may have been lost after acceptance.
public enum CoreTransactionBroadcastOutcome: Equatable, Sendable {
    case accepted(txid: String)
    case rejected(txid: String, reason: String)
    case unknown(txid: String, reason: String)

    public var txid: String {
        switch self {
        case .accepted(let txid), .rejected(let txid, _), .unknown(let txid, _):
            return txid
        }
    }

    init(
        resultCode: PlatformWalletResultCode,
        txid: String,
        reason: String
    ) throws {
        switch resultCode {
        case .success:
            self = .accepted(txid: txid)
        case .errorTransactionBroadcastRejected:
            self = .rejected(txid: txid, reason: reason)
        case .errorTransactionBroadcastUnconfirmed:
            self = .unknown(txid: txid, reason: reason)
        default:
            throw PlatformWalletError.unknown(
                "Cannot create Core broadcast outcome from \(resultCode)"
            )
        }
    }
}

/// Core wallet for UTXO management, address derivation, and transaction broadcasting.
///
/// Obtained via `ManagedPlatformWallet.coreWallet()`.
public class ManagedCoreWallet {
    let handle: Handle

    init(handle: Handle) {
        self.handle = handle
    }

    deinit {
        core_wallet_destroy(handle).discard()
    }

    // MARK: - Balance

    /// Core wallet balance breakdown (lock-free atomic reads).
    public struct CoreBalance {
        /// Confirmed balance (in a block or InstantSend-locked).
        public let confirmed: UInt64
        /// Unconfirmed balance (mempool-only, also spendable).
        public let unconfirmed: UInt64
        /// Immature balance (coinbase UTXOs not yet mature).
        public let immature: UInt64
        /// Locked balance (reserved for specific purposes).
        public let locked: UInt64

        public var total: UInt64 {
            confirmed + unconfirmed + immature + locked
        }
    }

    /// Read the current balance (lock-free atomic reads).
    public func balance() throws -> CoreBalance {
        var confirmed: UInt64 = 0
        var unconfirmed: UInt64 = 0
        var immature: UInt64 = 0
        var locked: UInt64 = 0
        try core_wallet_get_balance(
            handle, &confirmed, &unconfirmed, &immature, &locked
        ).check()

        return CoreBalance(
            confirmed: confirmed,
            unconfirmed: unconfirmed,
            immature: immature,
            locked: locked
        )
    }

    /// Get the network this wallet operates on.
    public func network() throws -> Network {
        var ffiNetwork = FFINetwork(0)
        try core_wallet_get_network(handle, &ffiNetwork).check()
        return Network(ffiNetwork: ffiNetwork)
    }

    // MARK: - Addresses

    /// Get the next unused receive address for a specific BIP-44 account.
    public func nextReceiveAddress(accountIndex: UInt32 = 0) throws -> String {
        var addressPtr: UnsafeMutablePointer<CChar>? = nil
        try core_wallet_next_receive_address(handle, accountIndex, &addressPtr).check()
        guard let ptr = addressPtr else {
            throw PlatformWalletError.nullPointer(
                "core_wallet_next_receive_address returned a NULL address pointer"
            )
        }
        defer { core_wallet_free_address(ptr) }
        return String(cString: ptr)
    }

    /// Get the next unused change address for a specific BIP-44 account.
    public func nextChangeAddress(accountIndex: UInt32 = 0) throws -> String {
        var addressPtr: UnsafeMutablePointer<CChar>? = nil
        try core_wallet_next_change_address(handle, accountIndex, &addressPtr).check()
        guard let ptr = addressPtr else {
            throw PlatformWalletError.nullPointer(
                "core_wallet_next_change_address returned a NULL address pointer"
            )
        }
        defer { core_wallet_free_address(ptr) }
        return String(cString: ptr)
    }

    /// Widen the gap limit for an account, generating the addresses the wider
    /// limit now requires.
    public func setGapLimit(
        accountType: CoreTransactionBuilder.AccountType,
        accountIndex: UInt32,
        gapLimit: UInt32
    ) throws {
        try core_wallet_set_gap_limit(handle, accountType.ffi, accountIndex, gapLimit).check()
    }

    // MARK: - Transactions

    /// Broadcast a transaction built by `CoreTransactionBuilder.buildSigned`.
    ///
    /// The funding account captured at build time is forwarded so that a
    /// definitive broadcast rejection releases the UTXO reservation
    /// `buildSigned` took, letting an immediate retry reselect those inputs.
    ///
    /// Returns the authoritative accepted/rejected/unknown network outcome.
    /// Throws only for local or FFI failures that prevented an outcome from
    /// being determined.
    public func broadcastTransactionWithOutcome(
        _ tx: CoreTransaction
    ) throws -> CoreTransactionBroadcastOutcome {
        var txidPtr: UnsafeMutablePointer<CChar>? = nil
        let ffiResult = withUnsafePointer(to: tx.ffi) { txPtr in
            core_wallet_broadcast_transaction(
                handle, txPtr, tx.accountType.ffi, tx.accountIndex, &txidPtr
            )
        }
        let result = PlatformWalletResult(ffiResult)

        defer {
            if let txidPtr {
                platform_wallet_string_free(txidPtr)
            }
        }

        switch result.code {
        case .success, .errorTransactionBroadcastRejected,
             .errorTransactionBroadcastUnconfirmed:
            guard let txidPtr else {
                throw PlatformWalletError.nullPointer(
                    "core_wallet_broadcast_transaction returned a NULL txid pointer for \(result.code)"
                )
            }
            let txid = String(cString: txidPtr)
            let reason = result.message ?? "<no detail from Rust>"

            return try CoreTransactionBroadcastOutcome(
                resultCode: result.code,
                txid: txid,
                reason: reason
            )

        default:
            try result.throwIfError()
            throw PlatformWalletError.unknown(
                "core_wallet_broadcast_transaction returned an unexpected success state"
            )
        }
    }

    /// Compatibility wrapper preserving the former throwing API.
    ///
    /// New code should inspect `broadcastTransactionWithOutcome` so an unknown
    /// outcome cannot be mistaken for a definitive rejection.
    @available(*, deprecated, message: "Use broadcastTransactionWithOutcome(_:) and handle accepted/rejected/unknown")
    public func broadcastTransaction(_ tx: CoreTransaction) throws -> String {
        switch try broadcastTransactionWithOutcome(tx) {
        case .accepted(let txid):
            return txid
        case .rejected(_, let reason):
            throw PlatformWalletError.transactionBroadcastRejected(reason)
        case .unknown(_, let reason):
            throw PlatformWalletError.transactionBroadcastUnconfirmed(reason)
        }
    }

    /// Consume and broadcast an atomically finalized transaction, returning
    /// the authoritative accepted/rejected/unknown network outcome.
    public func broadcastTransactionWithOutcome(
        _ tx: FinalizedCoreTransaction
    ) throws -> CoreTransactionBroadcastOutcome {
        let transactionHandle = try tx.takeForBroadcast()
        var txidPtr: UnsafeMutablePointer<CChar>? = nil
        let result = PlatformWalletResult(core_wallet_broadcast_signed_transaction_v2(
            handle,
            transactionHandle,
            &txidPtr
        ))

        defer {
            if let txidPtr {
                core_wallet_free_address(txidPtr)
            }
        }

        switch result.code {
        case .success, .errorTransactionBroadcastRejected,
             .errorTransactionBroadcastUnconfirmed:
            guard let txidPtr else {
                throw PlatformWalletError.nullPointer(
                    "core_wallet_broadcast_signed_transaction_v2 returned a NULL txid pointer for \(result.code)"
                )
            }
            return try CoreTransactionBroadcastOutcome(
                resultCode: result.code,
                txid: String(cString: txidPtr),
                reason: result.message ?? "<no detail from Rust>"
            )

        default:
            try result.throwIfError()
            throw PlatformWalletError.unknown(
                "core_wallet_broadcast_signed_transaction_v2 returned an unexpected success state"
            )
        }
    }

    /// Compatibility wrapper preserving the former throwing API.
    @available(*, deprecated, message: "Use broadcastTransactionWithOutcome(_:) and handle accepted/rejected/unknown")
    public func broadcastTransaction(_ tx: FinalizedCoreTransaction) throws -> String {
        switch try broadcastTransactionWithOutcome(tx) {
        case .accepted(let txid):
            return txid
        case .rejected(_, let reason):
            throw PlatformWalletError.transactionBroadcastRejected(reason)
        case .unknown(_, let reason):
            throw PlatformWalletError.transactionBroadcastUnconfirmed(reason)
        }
    }

    /// Consume without sending and release its reservation immediately.
    public func abandonTransaction(_ tx: FinalizedCoreTransaction) throws {
        try core_wallet_abandon_signed_transaction_v2(
            handle,
            tx.takeForAbandon()
        ).check()
    }
}
