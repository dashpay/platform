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
        var txLen: UInt = 0

        // Build C string array
        let cStrings = recipients.map { ($0.address as NSString).utf8String }
        let amounts = recipients.map { $0.amountDuffs }

        // Resolver-backed signer owns mnemonic access for the lifetime
        // of this call. Each Core ECDSA signature happens atomically
        // inside the resolver vtable (mnemonic fetched, key derived,
        // digest signed, buffers zeroed) — no priv key leaves Swift.
        let resolver = MnemonicResolver()

        try cStrings.withUnsafeBufferPointer { addrBuf in
            try amounts.withUnsafeBufferPointer { amountBuf in
                try withExtendedLifetime(resolver) {
                    try core_wallet_send_to_addresses(
                        handle,
                        accountType.rawValue,
                        accountIndex,
                        addrBuf.baseAddress,
                        amountBuf.baseAddress,
                        UInt(recipients.count),
                        resolver.handle,
                        &txBytesPtr,
                        &txLen
                    ).check()
                }
            }
        }

        guard let ptr = txBytesPtr, txLen > 0 else {
            throw PlatformWalletError.unknown("FFI returned success but tx buffer was empty")
        }
        defer { core_wallet_free_tx_bytes(ptr, txLen) }

        return Data(bytes: ptr, count: Int(txLen))
    }

    /// Broadcast a raw signed transaction.
    ///
    /// Returns the transaction ID as a hex string.
    public func broadcastTransaction(_ txData: Data) throws -> String {
        var txidPtr: UnsafeMutablePointer<CChar>? = nil
        try txData.withUnsafeBytes { txBuf in
            try core_wallet_broadcast_transaction(
                handle,
                txBuf.baseAddress?.assumingMemoryBound(to: UInt8.self),
                UInt(txData.count),
                &txidPtr
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
}
