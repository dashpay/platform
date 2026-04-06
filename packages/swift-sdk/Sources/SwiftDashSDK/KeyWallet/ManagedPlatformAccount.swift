// ManagedPlatformAccount.swift
// SwiftDashSDK
//
// Swift wrapper for a managed platform payment account (DIP-17).
// Platform payment accounts are identified by (account_index, key_class)
// and hold credit/duff balances with their own address pool.
//
// FFIManagedPlatformAccount is an opaque C type with a minimal body so Swift
// can use typed pointers (UnsafeMutablePointer<FFIManagedPlatformAccount>).

import Foundation
import DashSDKFFI

// MARK: - Managed Platform Account

/// Swift wrapper for an FFI-managed platform payment account.
///
/// Provides access to credit/duff balances, address counts, and the
/// account's address pool. The underlying FFI handle is freed on deinit.
public class ManagedPlatformAccount {
    private let handle: UnsafeMutablePointer<FFIManagedPlatformAccount>

    internal init(handle: UnsafeMutablePointer<FFIManagedPlatformAccount>) {
        self.handle = handle
    }

    deinit {
        managed_platform_account_free(handle)
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
