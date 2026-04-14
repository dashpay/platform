import Foundation

/// A managed wallet created by `PlatformWalletManager`.
///
/// Provides access to sub-wallets (platform addresses, core, identity, etc.)
/// and lock-free balance reads.
public class ManagedPlatformWallet {
    let handle: Handle

    /// The 32-byte wallet identifier.
    public let walletId: Data

    init(handle: Handle, walletId: Data) {
        self.handle = handle
        self.walletId = walletId
    }

    deinit {
        var error = PlatformWalletFFIError()
        _ = platform_wallet_destroy(handle, &error)
    }

    // MARK: - Balance (lock-free)

    /// Wallet balance breakdown. These are atomic reads — no lock contention.
    public struct WalletBalance {
        public let spendable: UInt64
        public let unconfirmed: UInt64
        public let immature: UInt64
        public let locked: UInt64

        public var total: UInt64 {
            spendable + unconfirmed + immature + locked
        }
    }

    /// Read the current balance (lock-free atomic reads).
    public func balance() throws -> WalletBalance {
        var spendable: UInt64 = 0
        var unconfirmed: UInt64 = 0
        var immature: UInt64 = 0
        var locked: UInt64 = 0
        var error = PlatformWalletFFIError()

        let result = platform_wallet_get_balance(
            handle,
            &spendable,
            &unconfirmed,
            &immature,
            &locked,
            &error
        )

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return WalletBalance(
            spendable: spendable,
            unconfirmed: unconfirmed,
            immature: immature,
            locked: locked
        )
    }

    // MARK: - Sub-wallet access

    /// Get the platform address wallet for BLAST sync, transfers, and withdrawals.
    ///
    /// Each call returns a new handle (cheap clone — all Arc internals).
    /// The caller is responsible for the returned object's lifetime.
    public func platformAddressWallet() throws -> ManagedPlatformAddressWallet {
        var platformHandle: Handle = NULL_HANDLE
        var error = PlatformWalletFFIError()

        let result = platform_wallet_get_platform(handle, &platformHandle, &error)

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return ManagedPlatformAddressWallet(handle: platformHandle)
    }

    /// Get the core wallet for UTXO management, addresses, and transactions.
    public func coreWallet() throws -> ManagedCoreWallet {
        var coreHandle: Handle = NULL_HANDLE
        var error = PlatformWalletFFIError()

        let result = platform_wallet_get_core(handle, &coreHandle, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return ManagedCoreWallet(handle: coreHandle)
    }

    /// Get the asset lock manager for building and tracking asset locks.
    public func assetLockManager() throws -> ManagedAssetLockManager {
        var assetLockHandle: Handle = NULL_HANDLE
        var error = PlatformWalletFFIError()

        let result = platform_wallet_get_asset_locks(handle, &assetLockHandle, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return ManagedAssetLockManager(handle: assetLockHandle)
    }

    // MARK: - Persistence

    /// Flush all queued changesets to the storage backend.
    public func flushPersist() throws {
        var error = PlatformWalletFFIError()
        let result = platform_wallet_flush_persist(handle, &error)

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    /// Load persisted state and apply it to the in-memory wallet.
    public func loadAndApplyPersisted() throws {
        var error = PlatformWalletFFIError()
        let result = platform_wallet_load_and_apply_persisted(handle, &error)

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }
}
