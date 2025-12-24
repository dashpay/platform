import Foundation
import DashSDKFFI

/// Swift wrapper for a BLS account (used for provider keys)
public class BLSAccount {
    internal let handle: UnsafeMutablePointer<FFIBLSAccount>
    private weak var wallet: Wallet?
    
    internal init(handle: UnsafeMutablePointer<FFIBLSAccount>, wallet: Wallet?) {
        self.handle = handle
        self.wallet = wallet
    }
    
    deinit {
        bls_account_free(handle)
    }
    
    // BLS account specific functionality can be added here
    // This class manages the lifecycle of BLS provider key accounts
}
