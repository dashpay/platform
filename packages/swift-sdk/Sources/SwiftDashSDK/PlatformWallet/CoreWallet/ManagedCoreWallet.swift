import Foundation

/// Core wallet for UTXO management, address derivation, and transaction broadcasting.
///
/// Obtained via `ManagedPlatformWallet.coreWallet()`.
public class ManagedCoreWallet {
    let handle: Handle

    init(handle: Handle) {
        self.handle = handle
    }

    deinit {
        var error = PlatformWalletFFIError()
        _ = core_wallet_destroy(handle, &error)
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
        var error = PlatformWalletFFIError()

        let result = core_wallet_get_balance(
            handle, &confirmed, &unconfirmed, &immature, &locked, &error
        )
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return CoreBalance(
            confirmed: confirmed,
            unconfirmed: unconfirmed,
            immature: immature,
            locked: locked
        )
    }

    /// Get the network this wallet operates on.
    public func network() throws -> PlatformNetwork {
        var networkValue: UInt32 = 0
        var error = PlatformWalletFFIError()

        let result = core_wallet_get_network(handle, &networkValue, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return PlatformNetwork(rawValue: networkValue) ?? .testnet
    }

    // MARK: - Addresses

    /// Get the next unused receive address for a specific BIP-44 account.
    public func nextReceiveAddress(accountIndex: UInt32 = 0) throws -> String {
        var addressPtr: UnsafeMutablePointer<CChar>? = nil
        var error = PlatformWalletFFIError()

        let result = core_wallet_next_receive_address(handle, accountIndex, &addressPtr, &error)
        guard result == Success, let ptr = addressPtr else {
            throw PlatformWalletError(result: result, error: error)
        }
        defer { core_wallet_free_address(ptr) }

        return String(cString: ptr)
    }

    /// Get the next unused change address for a specific BIP-44 account.
    public func nextChangeAddress(accountIndex: UInt32 = 0) throws -> String {
        var addressPtr: UnsafeMutablePointer<CChar>? = nil
        var error = PlatformWalletFFIError()

        let result = core_wallet_next_change_address(handle, accountIndex, &addressPtr, &error)
        guard result == Success, let ptr = addressPtr else {
            throw PlatformWalletError(result: result, error: error)
        }
        defer { core_wallet_free_address(ptr) }

        return String(cString: ptr)
    }

    // MARK: - Transactions

    /// Account type for transaction building.
    public enum AccountType: UInt32 {
        case bip44 = 0
        case bip32 = 1
    }

    /// Build, sign, and broadcast a payment to the given addresses.
    ///
    /// Returns the serialized signed transaction.
    public func sendToAddresses(
        accountType: AccountType = .bip44,
        accountIndex: UInt32 = 0,
        recipients: [(address: String, amountDuffs: UInt64)]
    ) throws -> Data {
        var txBytesPtr: UnsafeMutablePointer<UInt8>? = nil
        var txLen: Int = 0
        var error = PlatformWalletFFIError()

        // Build C string array
        let cStrings = recipients.map { ($0.address as NSString).utf8String }
        let amounts = recipients.map { $0.amountDuffs }

        let result = cStrings.withUnsafeBufferPointer { addrBuf in
            amounts.withUnsafeBufferPointer { amountBuf in
                core_wallet_send_to_addresses(
                    handle,
                    accountType.rawValue,
                    accountIndex,
                    addrBuf.baseAddress,
                    amountBuf.baseAddress,
                    recipients.count,
                    &txBytesPtr,
                    &txLen,
                    &error
                )
            }
        }

        guard result == Success, let ptr = txBytesPtr, txLen > 0 else {
            throw PlatformWalletError(result: result, error: error)
        }
        defer { core_wallet_free_tx_bytes(ptr, txLen) }

        return Data(bytes: ptr, count: txLen)
    }

    /// Broadcast a raw signed transaction.
    ///
    /// Returns the transaction ID as a hex string.
    public func broadcastTransaction(_ txData: Data) throws -> String {
        var txidPtr: UnsafeMutablePointer<CChar>? = nil
        var error = PlatformWalletFFIError()

        let result = txData.withUnsafeBytes { txBuf in
            core_wallet_broadcast_transaction(
                handle,
                txBuf.baseAddress?.assumingMemoryBound(to: UInt8.self),
                txData.count,
                &txidPtr,
                &error
            )
        }

        guard result == Success, let ptr = txidPtr else {
            throw PlatformWalletError(result: result, error: error)
        }
        defer { core_wallet_free_address(ptr) } // same free for C strings

        return String(cString: ptr)
    }
}
