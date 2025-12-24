import Foundation
import DashSDKFFI

/// Swift wrapper for an address pool from a managed account
public class AddressPool {
    private let handle: UnsafeMutablePointer<FFIAddressPool>
    
    internal init(handle: UnsafeMutablePointer<FFIAddressPool>) {
        self.handle = handle
    }
    
    deinit {
        address_pool_free(handle)
    }
    
    // MARK: - Address Access
    
    /// Get an address at a specific index
    /// - Parameter index: The index of the address to retrieve
    /// - Returns: The address information if it exists
    public func getAddress(at index: UInt32) throws -> AddressInfo {
        var error = FFIError()
        
        guard let infoPtr = address_pool_get_address_at_index(handle, index, &error) else {
            defer {
                if error.message != nil {
                    error_message_free(error.message)
                }
            }
            throw KeyWalletError(ffiError: error)
        }
        
        defer {
            address_info_free(infoPtr)
        }
        
        return AddressInfo(ffiInfo: infoPtr.pointee)
    }
    
    /// Get addresses in a range
    /// - Parameters:
    ///   - startIndex: The starting index (inclusive)
    ///   - endIndex: The ending index (exclusive)
    /// - Returns: Array of address information
    public func getAddresses(from startIndex: UInt32, to endIndex: UInt32) throws -> [AddressInfo] {
        var error = FFIError()
        var count: Int = 0
        
        guard let infosPtr = address_pool_get_addresses_in_range(
            handle, startIndex, endIndex, &count, &error
        ) else {
            defer {
                if error.message != nil {
                    error_message_free(error.message)
                }
            }
            throw KeyWalletError(ffiError: error)
        }
        
        defer {
            address_info_array_free(infosPtr, count)
        }
        
        var addresses: [AddressInfo] = []
        for i in 0..<count {
            if let infoPtr = infosPtr[i] {
                addresses.append(AddressInfo(ffiInfo: infoPtr.pointee))
            }
        }
        
        return addresses
    }
}

/// Address information
public struct AddressInfo {
    public let address: String
    public let scriptPubKey: Data
    public let publicKey: Data?
    public let index: UInt32
    public let path: String
    public let used: Bool
    public let generatedAt: Date
    
    init(ffiInfo: FFIAddressInfo) {
        // Copy address string
        if let addrPtr = ffiInfo.address {
            self.address = String(cString: addrPtr)
        } else {
            self.address = ""
        }
        
        // Copy script pubkey
        if let scriptPtr = ffiInfo.script_pubkey, ffiInfo.script_pubkey_len > 0 {
            self.scriptPubKey = Data(bytes: scriptPtr, count: ffiInfo.script_pubkey_len)
        } else {
            self.scriptPubKey = Data()
        }
        
        // Copy public key if available
        if let pubKeyPtr = ffiInfo.public_key, ffiInfo.public_key_len > 0 {
            self.publicKey = Data(bytes: pubKeyPtr, count: ffiInfo.public_key_len)
        } else {
            self.publicKey = nil
        }
        
        self.index = ffiInfo.index
        
        // Copy derivation path
        if let pathPtr = ffiInfo.path {
            self.path = String(cString: pathPtr)
        } else {
            self.path = ""
        }
        
        self.used = ffiInfo.used
        self.generatedAt = Date(timeIntervalSince1970: TimeInterval(ffiInfo.generated_at))
    }
}
