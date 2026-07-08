// ManagedPlatformAccount.swift
// SwiftDashSDK
//
// Swift wrapper for a managed platform payment account (DIP-17).
// Platform payment accounts are identified by (account_index, key_class)
// and hold credit/duff balances with their own address pool.

import Foundation
import DashSDKFFI

// MARK: - Platform Payment Account Key

/// Identifies a platform payment account by its account index and key class.
public struct PlatformPaymentAccountKey: Sendable, Equatable {
    /// Account index (hardened).
    public let account: UInt32
    /// Key class (hardened).
    public let keyClass: UInt32

    public init(account: UInt32, keyClass: UInt32) {
        self.account = account
        self.keyClass = keyClass
    }
}

// MARK: - Managed Platform Account

/// Swift wrapper for an FFI-managed platform payment account.
///
/// Provides access to credit/duff balances, address counts, and the
/// account's address pool. The underlying FFI handle is freed on deinit.
public class ManagedPlatformAccount {
    private let handle: OpaquePointer

    internal init(handle: OpaquePointer) {
        self.handle = handle
    }

    deinit {
        managed_platform_account_free(handle)
    }

    // MARK: - Properties

    /// The network this account belongs to.
    public var network: Network {
        let ffiNetwork = managed_platform_account_get_network(handle)
        return Network(ffiNetwork: ffiNetwork)
    }

    /// The account index (hardened).
    public var accountIndex: UInt32 {
        managed_platform_account_get_account_index(handle)
    }

    /// The key class (hardened).
    public var keyClass: UInt32 {
        managed_platform_account_get_key_class(handle)
    }

    /// Total balance in credits (1000 credits = 1 duff).
    public var creditBalance: UInt64 {
        managed_platform_account_get_credit_balance(handle)
    }

    /// Total balance in duffs (credit_balance / 1000).
    public var duffBalance: UInt64 {
        managed_platform_account_get_duff_balance(handle)
    }

    /// Number of addresses that have been funded (have non-zero balance).
    public var fundedAddressCount: UInt32 {
        managed_platform_account_get_funded_address_count(handle)
    }

    /// Total number of derived addresses in this account.
    public var totalAddressCount: UInt32 {
        managed_platform_account_get_total_address_count(handle)
    }

    /// Whether this is a watch-only account.
    public var isWatchOnly: Bool {
        managed_platform_account_get_is_watch_only(handle)
    }

    // MARK: - Address Pool

    /// Get the address pool for this platform account.
    ///
    /// Platform accounts have a single address pool (no internal/external split).
    /// - Returns: The address pool, or nil if unavailable.
    public func getAddressPool() -> AddressPool? {
        guard let poolHandle = managed_platform_account_get_address_pool(handle) else {
            return nil
        }
        return AddressPool(handle: poolHandle)
    }
}
