import Foundation
import DashSDKFFI

/// Swift wrapper for a wallet account
public class Account {
    private let handle: OpaquePointer
    private weak var wallet: Wallet?

    internal init(handle: OpaquePointer, wallet: Wallet) {
        self.handle = handle
        self.wallet = wallet
    }

    deinit {
        account_free(handle)
    }

    // The account-specific functionality would be implemented here
    // For now, this is a placeholder that manages the FFI handle lifecycle

    // MARK: - Derivation (account-based)

    /// Derive a private key (WIF) at `index` under this account.
    ///
    /// The account's own derivation path is applied by the FFI:
    /// `account_derive_private_key_as_wif_at` → `derive_xpriv_from_master_xpriv`,
    /// which resolves `Account::derivation_path()` — the FULL path from the
    /// wallet master (e.g. `m/9'/5'/3'/1'` for provider voting keys). So this
    /// hands it the wallet **master** and takes no path argument.
    ///
    /// It previously accepted a `masterPath` and pre-derived it, applying the
    /// account path twice: a caller passing the account root got the key at
    /// `m/9'/5'/3'/1'/9'/5'/3'/1'/index` instead of `m/9'/5'/3'/1'/index`.
    /// Nothing failed locally — the key was well-formed and deterministic, just
    /// not the account's key at that index. That is only detectable against
    /// something holding the real one: a masternode vote signed with it is
    /// rejected as having no voter identity, since the voter identity is
    /// derived from the signing key's own hash160.
    ///
    /// - Parameters:
    ///   - wallet: The parent wallet, used for its master extended private key
    ///   - index: The child index to derive (e.g., 0 for the first key)
    /// - Returns: The private key encoded as WIF
    public func derivePrivateKeyWIF(wallet: Wallet, index: UInt32) throws -> String {
        var error = FFIError()
        // "m" parses to the empty derivation path — the master key itself.
        let masterPtr = "m".withCString { pathCStr in
            wallet_derive_extended_private_key(wallet.ffiHandle, pathCStr, &error)
        }

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
            if let m = masterPtr {
                extended_private_key_free(m)
            }
        }

        guard let master = masterPtr else {
            throw KeyWalletError(ffiError: error)
        }

        // Derive child private key as WIF at the given index
        let wifPtr = account_derive_private_key_as_wif_at(self.handle, master, index, &error)

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        guard let ptr = wifPtr else {
            throw KeyWalletError(ffiError: error)
        }
        let wif = String(cString: ptr)
        string_free(ptr)
        return wif
    }
}
