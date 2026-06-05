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

        // Own the C-string storage explicitly. The naive shape
        // `recipients.map { ($0.address as NSString).utf8String }`
        // yields pointers into a bridged `NSString` temporary whose
        // lifetime ends with the `.map` closure — Rust would then
        // `CStr::from_ptr` against freed (or autorelease-pool-
        // recycled) memory. `strdup` malloc's a fresh NUL-terminated
        // copy we own through the FFI call; `defer` frees them after
        // Rust returns.
        let cStringStorage: [UnsafeMutablePointer<CChar>?] = recipients.map {
            strdup($0.address)
        }
        defer {
            for ptr in cStringStorage {
                if let p = ptr { free(p) }
            }
        }
        // Promote the owned buffers to immutable `UnsafePointer<CChar>?`
        // for the FFI's `*const *const c_char` signature. No
        // `assumingMemoryBound` re-interpretation needed — the bytes
        // are already correctly typed.
        let cStringPointers: [UnsafePointer<CChar>?] = cStringStorage.map { ptr in
            ptr.map { UnsafePointer($0) }
        }
        let amounts = recipients.map { $0.amountDuffs }

        // Resolver-backed signer owns mnemonic access for the lifetime
        // of this call. Each Core ECDSA signature happens atomically
        // inside the resolver vtable (mnemonic fetched, key derived,
        // digest signed, buffers zeroed) — no priv key leaves Swift.
        let resolver = MnemonicResolver()

        try cStringPointers.withUnsafeBufferPointer { addrBuf in
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

    /// Sweep the entire spendable balance of the wallet's CoinJoin account to
    /// `destination` across one or more transactions, leaving no change so the
    /// account is fully emptied.
    ///
    /// CoinJoin "mixed coins" live on a dedicated account (BIP44 purpose 4')
    /// that ``sendToAddresses(accountType:accountIndex:recipients:)`` cannot
    /// reach. Used by the DashSync → SwiftDashSDK migration to move a user's
    /// mixed coins (no longer supported) into their spendable balance.
    ///
    /// A heavy mixer's UTXO set can exceed a single transaction's relay-size
    /// limit, so the sweep is split into balanced chunks. Returns the
    /// **wire-order** txids of the broadcast transactions (already broadcast by
    /// the FFI), in chunk order — the caller records them to group the
    /// resulting withdrawals.
    public func sweepCoinJoinAccount(
        accountIndex: UInt32 = 0,
        destination: String
    ) throws -> [Data] {
        var txidsPtr: UnsafeMutablePointer<UInt8>? = nil
        var count: UInt = 0

        // Single owned recipient string; the resolver-backed signer owns
        // mnemonic access for the lifetime of the call (same model as
        // `sendToAddresses`).
        let resolver = MnemonicResolver()

        try destination.withCString { destPtr in
            try withExtendedLifetime(resolver) {
                try core_wallet_sweep_coinjoin(
                    handle,
                    accountIndex,
                    destPtr,
                    resolver.handle,
                    &txidsPtr,
                    &count
                ).check()
            }
        }

        guard let ptr = txidsPtr, count > 0 else {
            throw PlatformWalletError.unknown("FFI returned success but txid buffer was empty")
        }
        // One contiguous `count * 32` byte buffer; free it as a single block.
        let byteCount = Int(count) * 32
        defer { core_wallet_free_tx_bytes(ptr, UInt(byteCount)) }

        // Split into individual 32-byte wire-order txids (chunk order).
        let allBytes = Data(bytes: ptr, count: byteCount)
        return (0..<Int(count)).map { i in
            allBytes.subdata(in: (i * 32)..<((i + 1) * 32))
        }
    }

    /// Widen the wallet's CoinJoin account address gap limit to `gapLimit` and
    /// generate the addresses so SPV watches the wider window.
    ///
    /// CoinJoin "mixed coins" (BIP44 purpose 4') are scattered with holes wider
    /// than the default gap (30), so a fresh post-migration scan would miss
    /// deep coins. The DashSync → SwiftDashSDK migration calls this — only for
    /// wallets that used CoinJoin — before starting SPV so the one-time
    /// recovery scan finds every mixed coin. The widened limit is in-memory
    /// only (not persisted), so a later launch reverts to the default gap.
    ///
    /// Idempotent. Returns the pool's highest generated address index.
    @discardableResult
    public func setCoinJoinGapLimit(
        accountIndex: UInt32 = 0,
        gapLimit: UInt32
    ) throws -> UInt32 {
        var highestIndex: UInt32 = 0
        try core_wallet_set_coinjoin_gap_limit(
            handle,
            accountIndex,
            gapLimit,
            &highestIndex
        ).check()
        return highestIndex
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
