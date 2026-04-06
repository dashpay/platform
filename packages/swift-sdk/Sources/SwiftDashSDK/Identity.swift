import Foundation
import DashSDKFFI

/// Swift wrapper for Dash Platform Identity
public class Identity {
    /// Create an Identity from a C handle
    public init?(handle: UnsafeMutablePointer<IdentityHandle>) {}
}
