import Foundation
import DashSDKFFI

/// Swift wrapper for a managed platform account (DIP-17 Platform Payment accounts)
/// This is different from ManagedAccount because ManagedPlatformAccount has a different
/// structure optimized for Platform Payment accounts:
/// - Simple u64 credit balance instead of WalletCoreBalance
/// - Per-address balances tracked directly
/// - No transactions or UTXOs (Platform handles these)
public class ManagedPlatformAccount {
    internal let handle: UnsafeMutablePointer<FFIManagedPlatformAccount>

    internal init(handle: UnsafeMutablePointer<FFIManagedPlatformAccount>) {
        self.handle = handle
    }

    deinit {
        managed_platform_account_free(handle)
    }

    // MARK: - Properties

    /// Get the network this account is on
    public var network: KeyWalletNetwork {
        let ffiNetwork = managed_platform_account_get_network(handle)
        return KeyWalletNetwork(ffiNetwork: ffiNetwork)
    }

    /// Get the account index (hardened)
    public var accountIndex: UInt32 {
        return managed_platform_account_get_account_index(handle)
    }

    /// Get the key class (hardened)
    public var keyClass: UInt32 {
        return managed_platform_account_get_key_class(handle)
    }

    /// Check if this is a watch-only account
    public var isWatchOnly: Bool {
        return managed_platform_account_get_is_watch_only(handle)
    }

    /// Get the total credit balance (1000 credits = 1 duff)
    public var creditBalance: UInt64 {
        return managed_platform_account_get_credit_balance(handle)
    }

    /// Get the total balance in duffs (credit_balance / 1000)
    public var duffBalance: UInt64 {
        return managed_platform_account_get_duff_balance(handle)
    }

    /// Get the number of funded addresses
    public var fundedAddressCount: UInt32 {
        return managed_platform_account_get_funded_address_count(handle)
    }

    /// Get the total number of addresses
    public var totalAddressCount: UInt32 {
        return managed_platform_account_get_total_address_count(handle)
    }

    // MARK: - Address Pool

    /// Get the address pool for this platform account
    /// Platform accounts only have a single address pool
    public func getAddressPool() -> AddressPool? {
        guard let poolHandle = managed_platform_account_get_address_pool(handle) else {
            return nil
        }
        return AddressPool(handle: poolHandle)
    }

    // MARK: - Computed Properties

    /// Get the derivation path for this account
    /// DIP-17 path: m/9'/coinType'/17'/account'/key_class'/index
    public func derivationPath(isTestnet: Bool) -> String {
        let coinType = isTestnet ? "1'" : "5'"
        return "m/9'/\(coinType)/17'/\(accountIndex)'/\(keyClass)'/..."
    }

    /// Get a label for this account
    public var label: String {
        if keyClass == 0 {
            return "Platform Payment \(accountIndex)"
        } else {
            return "Platform Payment \(accountIndex)/\(keyClass)"
        }
    }
}
