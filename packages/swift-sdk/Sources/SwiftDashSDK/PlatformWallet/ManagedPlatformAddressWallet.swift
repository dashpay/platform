import Foundation

/// Platform address wallet for balance queries, transfers, and withdrawals.
///
/// Obtained via `ManagedPlatformWallet.platformAddressWallet()`.
/// Background BLAST syncing is owned by `PlatformWalletManager`.
///
/// `@unchecked Sendable`: the underlying Rust handle is guarded by
/// `platform-wallet-ffi`'s internal `HandleStorage` (parking_lot RwLock),
/// so dispatching method calls across threads is safe. The Swift side
/// only borrows the opaque pointer.
public final class ManagedPlatformAddressWallet: @unchecked Sendable {
    let handle: Handle

    init(handle: Handle) {
        self.handle = handle
    }

    deinit {
        var error = PlatformWalletFFIError()
        _ = platform_address_wallet_destroy(handle, &error)
    }

    // MARK: - Balance queries

    /// Platform address with its credit balance.
    public struct AddressBalance: Sendable {
        /// Address type (0 = P2PKH).
        public let addressType: UInt8
        /// 20-byte address hash.
        public let hash: Data
        /// Credit balance.
        public let balance: UInt64
    }

    /// Get total platform credits across all addresses.
    public func totalCredits() throws -> UInt64 {
        var credits: UInt64 = 0
        var error = PlatformWalletFFIError()

        let result = platform_address_wallet_total_credits(handle, &credits, &error)

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return credits
    }

    /// Get all platform addresses with their cached balances.
    public func addressesWithBalances() throws -> [AddressBalance] {
        var entriesPtr: UnsafeMutablePointer<AddressBalanceEntryFFI>?
        var count: Int = 0
        var error = PlatformWalletFFIError()

        let result = platform_address_wallet_addresses_with_balances(
            handle, &entriesPtr, &count, &error
        )

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        defer {
            platform_address_wallet_free_address_balances(entriesPtr, count)
        }

        guard let entries = entriesPtr, count > 0 else {
            return []
        }

        return (0..<count).map { i in
            let entry = entries[i]
            let hashData = withUnsafeBytes(of: entry.address.hash) { Data($0) }
            return AddressBalance(
                addressType: entry.address.address_type,
                hash: hashData,
                balance: entry.balance
            )
        }
    }
}
