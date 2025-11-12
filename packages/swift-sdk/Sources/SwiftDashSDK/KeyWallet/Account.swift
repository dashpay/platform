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
        var error = FFIError()
        // Derive master extended private key for this account root
        let masterPtr = masterPath.withCString { pathCStr in
            wallet_derive_extended_private_key(wallet.ffiHandle, wallet.network.ffiValue, pathCStr, &error)
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
