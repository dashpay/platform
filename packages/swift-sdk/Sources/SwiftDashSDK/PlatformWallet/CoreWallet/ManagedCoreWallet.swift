import Foundation
import DashSDKFFI

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
    /// Returns the transaction ID as a hex string.
    @available(*, deprecated, message: "Use the atomic FinalizedCoreTransaction send path")
    public func broadcastTransaction(_ tx: CoreTransaction) throws -> String {
        var txidPtr: UnsafeMutablePointer<CChar>? = nil
        try withUnsafePointer(to: tx.ffi) { txPtr in
            try core_wallet_broadcast_transaction(
                handle, txPtr, tx.accountType.ffi, tx.accountIndex, &txidPtr
            ).check()
        }

        guard let ptr = txidPtr else {
            throw PlatformWalletError.nullPointer(
                "core_wallet_broadcast_transaction returned a NULL txid pointer"
            )
        }
        defer { core_wallet_free_address(ptr) } // same free for C strings

        return String(cString: ptr)
    }

    /// Consume and broadcast an atomically finalized transaction.
    public func broadcastTransaction(_ tx: FinalizedCoreTransaction) throws -> String {
        let transactionHandle = try tx.takeForBroadcast()
        var txidPtr: UnsafeMutablePointer<CChar>? = nil
        try core_wallet_broadcast_signed_transaction_v2(
            handle,
            transactionHandle,
            &txidPtr
        ).check()
        guard let ptr = txidPtr else {
            throw PlatformWalletError.nullPointer(
                "core_wallet_broadcast_signed_transaction_v2 returned NULL"
            )
        }
        defer { core_wallet_free_address(ptr) }
        return String(cString: ptr)
    }

    /// Consume without sending and release its reservation immediately.
    public func abandonTransaction(_ tx: FinalizedCoreTransaction) throws {
        try core_wallet_abandon_signed_transaction_v2(
            handle,
            tx.takeForAbandon()
        ).check()
    }
}
