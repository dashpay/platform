import Foundation
import DashSDKFFI

/// Address utilities
public class Address {
    
    /// Address type enumeration
    public enum AddressType: UInt8 {
        case p2pkh = 0
        case p2sh = 1
        case other = 2
        case unknown = 255
    }
    
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
    
    /// Get the type of an address
    /// - Parameters:
    ///   - address: The address to check
    ///   - network: The network type
    /// - Returns: The address type
    public static func getType(of address: String, network: KeyWalletNetwork = .mainnet) -> AddressType {
        var error = FFIError()
        
        let typeRaw = address.withCString { addressCStr in
            address_get_type(addressCStr, network.ffiValue, &error)
        }
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }
        
        // Map the raw value to our enum
        switch typeRaw {
        case 0:
            return .p2pkh
        case 1:
            return .p2sh
        case 2:
            return .other
        default:
            return .unknown
        }
    }
}