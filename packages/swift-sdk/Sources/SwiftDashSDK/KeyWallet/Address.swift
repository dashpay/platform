import Foundation
import DashSDKFFI

/// Address utilities
public class Address {
    /// Validate an address
    /// - Parameters:
    ///   - address: The address to validate
    ///   - network: The network type
    /// - Returns: True if the address is valid
    public static func validate(_ address: String, network: KeyWalletNetwork = .mainnet) -> Bool {
        var error = FFIError()

        let isValid = address.withCString { addressCStr in
            address_validate(addressCStr, network.ffiValue, &error)
        }

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        return isValid
    }
}
