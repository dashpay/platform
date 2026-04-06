import Foundation
import DashSDKFFI

/// Swift wrapper for a Dash wallet with HD key derivation
public class Wallet {
    internal let handle: UnsafeMutablePointer<FFIWallet>
    private let ownsHandle: Bool

    // MARK: - Wallet Properties

    /// Check if wallet is watch-only
    public var isWatchOnly: Bool {
        var error = FFIError()
        let result = wallet_is_watch_only(handle, &error)

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        return result
    }

    // MARK: - Account Management

    /// Get an account by type and index
    /// - Parameters:
    ///   - type: The account type
    ///   - index: The account index
    /// - Returns: An account handle
    public func getAccount(type: AccountType, index: UInt32 = 0) throws -> Account {
        let result = wallet_get_account(handle, index, type.ffiValue)

        defer {
            if result.error_message != nil {
                var mutableResult = result
                account_result_free_error(&mutableResult)
            }
        }

        guard let accountHandle = result.account else {
            var error = FFIError()
            error.code = FFIErrorCode(rawValue: UInt32(result.error_code))
            if let msg = result.error_message {
                error.message = msg
            }
            throw KeyWalletError(ffiError: error)
        }

        return Account(handle: accountHandle, wallet: self)
    }

    /// Add an account to the wallet
    /// - Parameters:
    ///   - type: The account type
    ///   - index: The account index
    ///   - xpub: Optional extended public key for watch-only accounts
    /// - Returns: The newly added account
    public func addAccount(type: AccountType, index: UInt32, xpub: String? = nil) throws -> Account {
        let result: FFIAccountResult

        if let xpub = xpub {
            result = xpub.withCString { xpubCStr in
                wallet_add_account_with_string_xpub(
                    handle, type.ffiValue, index, xpubCStr)
            }
        } else {
            result = wallet_add_account(
                handle, type.ffiValue, index)
        }

        defer {
            if result.error_message != nil {
                var mutableResult = result
                account_result_free_error(&mutableResult)
            }
        }

        guard let accountHandle = result.account else {
            var error = FFIError()
            error.code = FFIErrorCode(rawValue: UInt32(result.error_code))
            if let msg = result.error_message {
                error.message = msg
            }
            throw KeyWalletError(ffiError: error)
        }

        return Account(handle: accountHandle, wallet: self)
    }

    /// Add a Platform Payment account (DIP-17) to the wallet.
    ///
    /// Platform Payment accounts use derivation path:
    /// `m/9'/coin_type'/17'/account'/key_class'/index`
    ///
    /// - Parameters:
    ///   - accountIndex: The account index (hardened).
    ///   - keyClass: The key class (hardened). 0 = receive.
    public func addPlatformPaymentAccount(accountIndex: UInt32 = 0, keyClass: UInt32 = 0) throws {
        let result = wallet_add_platform_payment_account(handle, accountIndex, keyClass)

        defer {
            if result.error_message != nil {
                var mutableResult = result
                account_result_free_error(&mutableResult)
            }
        }

        guard result.account != nil else {
            var error = FFIError()
            error.code = FFIErrorCode(rawValue: UInt32(result.error_code))
            if let msg = result.error_message {
                error.message = msg
            }
            throw KeyWalletError(ffiError: error)
        }
    }

    // MARK: - Internal

    /// Get the raw FFI handle (for internal use)

    internal var ffiHandle: UnsafeMutablePointer<FFIWallet> { handle }

    // Non-owning initializer for wallets obtained from WalletManager
    public init(nonOwningHandle handle: UnsafeRawPointer) {
        self.handle = UnsafeMutablePointer<FFIWallet>(mutating: handle.bindMemory(to: FFIWallet.self, capacity: 1))
        self.ownsHandle = false
    }


    deinit {
        if ownsHandle {
            wallet_free(handle)
        }
    }
}
