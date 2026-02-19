import Foundation
import DashSDKFFI

/// Swift wrapper for a wallet account
public class Account {
    private let handle: UnsafeMutablePointer<FFIAccount>
    private weak var wallet: Wallet?
    
    internal init(handle: UnsafeMutablePointer<FFIAccount>, wallet: Wallet) {
        self.handle = handle
        self.wallet = wallet
    }
    
    deinit {
        account_free(handle)
    }
    
    // The account-specific functionality would be implemented here
    // For now, this is a placeholder that manages the FFI handle lifecycle
    
    // MARK: - Derivation (account-based)
    
    /// Derive a private key (WIF) using this account and a master xpriv derived from the given path.
    /// - Parameters:
    ///   - wallet: The parent wallet used to derive the master extended private key
    ///   - masterPath: The account root derivation path (e.g., "m/9'/5'/3'/1'")
    ///   - index: The child index to derive (e.g., 0 for the first key)
    /// - Returns: The private key encoded as WIF
    public func derivePrivateKeyWIF(wallet: Wallet, masterPath: String, index: UInt32) throws -> String {
        var deriveError = FFIError()
        // Derive master extended private key for this account root
        let masterPtr = masterPath.withCString { pathCStr in
            wallet_derive_extended_private_key(wallet.ffiHandle, pathCStr, &deriveError)
        }

        defer {
            if deriveError.message != nil {
                error_message_free(deriveError.message)
            }
            if let m = masterPtr {
                extended_private_key_free(m)
            }
        }

        guard let master = masterPtr else {
            throw KeyWalletError(ffiError: deriveError)
        }

        // Derive child private key as WIF at the given index
        var wifError = FFIError()
        let wifPtr = account_derive_private_key_as_wif_at(self.handle, master, index, &wifError)

        defer {
            if wifError.message != nil {
                error_message_free(wifError.message)
            }
        }

        guard let ptr = wifPtr else {
            throw KeyWalletError(ffiError: wifError)
        }
        let wif = String(cString: ptr)
        string_free(ptr)
        return wif
    }
}
