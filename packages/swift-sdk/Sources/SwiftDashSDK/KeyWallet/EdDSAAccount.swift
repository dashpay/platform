import Foundation
import DashSDKFFI

/// Swift wrapper for an EdDSA account (used for platform P2P keys)
public class EdDSAAccount {
    internal let handle: UnsafeMutablePointer<FFIEdDSAAccount>
    private weak var wallet: Wallet?
    
    internal init(handle: UnsafeMutablePointer<FFIEdDSAAccount>, wallet: Wallet?) {
        self.handle = handle
        self.wallet = wallet
    }
    
    deinit {
        eddsa_account_free(handle)
    }
    
    // EdDSA account specific functionality can be added here
    // This class manages the lifecycle of EdDSA platform P2P key accounts
}
