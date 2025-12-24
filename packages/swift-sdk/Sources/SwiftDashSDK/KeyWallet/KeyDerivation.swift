import Foundation
import DashSDKFFI

/// Key derivation utilities
public class KeyDerivation {
    
    /// Create a new master extended private key from seed
    /// - Parameters:
    ///   - seed: The seed bytes
    ///   - network: The network type
    /// - Returns: Extended private key handle
    public static func createMasterKey(seed: Data, network: KeyWalletNetwork = .mainnet) throws -> ExtendedPrivateKey {
        var error = FFIError()
        
        let xprivPtr = seed.withUnsafeBytes { seedBytes in
            let seedPtr = seedBytes.bindMemory(to: UInt8.self).baseAddress
            return derivation_new_master_key(seedPtr, seed.count, network.ffiValue, &error)
        }
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }
        
        guard let handle = xprivPtr else {
            throw KeyWalletError(ffiError: error)
        }
        
        return ExtendedPrivateKey(handle: handle)
    }
    
    /// Get BIP44 account path
    /// - Parameters:
    ///   - network: The network type
    ///   - accountIndex: The account index
    /// - Returns: The derivation path string
    public static func getBIP44AccountPath(network: KeyWalletNetwork = .mainnet,
                                          accountIndex: UInt32) throws -> String {
        var error = FFIError()
        let maxPathLen = 256
        let pathBuffer = UnsafeMutablePointer<CChar>.allocate(capacity: maxPathLen)
        defer {
            pathBuffer.deallocate()
        }
        
        let success = derivation_bip44_account_path(
            network.ffiValue, accountIndex, pathBuffer, maxPathLen, &error)
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }
        
        guard success else {
            throw KeyWalletError(ffiError: error)
        }
        
        return String(cString: pathBuffer)
    }
    
    /// Get BIP44 payment path
    /// - Parameters:
    ///   - network: The network type
    ///   - accountIndex: The account index
    ///   - isChange: Whether this is a change address
    ///   - addressIndex: The address index
    /// - Returns: The derivation path string
    public static func getBIP44PaymentPath(network: KeyWalletNetwork = .mainnet,
                                          accountIndex: UInt32,
                                          isChange: Bool,
                                          addressIndex: UInt32) throws -> String {
        var error = FFIError()
        let maxPathLen = 256
        let pathBuffer = UnsafeMutablePointer<CChar>.allocate(capacity: maxPathLen)
        defer {
            pathBuffer.deallocate()
        }
        
        let success = derivation_bip44_payment_path(
            network.ffiValue, accountIndex, isChange, addressIndex,
            pathBuffer, maxPathLen, &error)
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }
        
        guard success else {
            throw KeyWalletError(ffiError: error)
        }
        
        return String(cString: pathBuffer)
    }
    
    /// Get CoinJoin path
    /// - Parameters:
    ///   - network: The network type
    ///   - accountIndex: The account index
    /// - Returns: The derivation path string
    public static func getCoinJoinPath(network: KeyWalletNetwork = .mainnet,
                                      accountIndex: UInt32) throws -> String {
        var error = FFIError()
        let maxPathLen = 256
        let pathBuffer = UnsafeMutablePointer<CChar>.allocate(capacity: maxPathLen)
        defer {
            pathBuffer.deallocate()
        }
        
        let success = derivation_coinjoin_path(
            network.ffiValue, accountIndex, pathBuffer, maxPathLen, &error)
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }
        
        guard success else {
            throw KeyWalletError(ffiError: error)
        }
        
        return String(cString: pathBuffer)
    }
    
    /// Get identity registration path
    /// - Parameters:
    ///   - network: The network type
    ///   - identityIndex: The identity index
    /// - Returns: The derivation path string
    public static func getIdentityRegistrationPath(network: KeyWalletNetwork = .mainnet,
                                                  identityIndex: UInt32) throws -> String {
        var error = FFIError()
        let maxPathLen = 256
        let pathBuffer = UnsafeMutablePointer<CChar>.allocate(capacity: maxPathLen)
        defer {
            pathBuffer.deallocate()
        }
        
        let success = derivation_identity_registration_path(
            network.ffiValue, identityIndex, pathBuffer, maxPathLen, &error)
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }
        
        guard success else {
            throw KeyWalletError(ffiError: error)
        }
        
        return String(cString: pathBuffer)
    }
    
    /// Get identity top-up path
    /// - Parameters:
    ///   - network: The network type
    ///   - identityIndex: The identity index
    ///   - topupIndex: The top-up index
    /// - Returns: The derivation path string
    public static func getIdentityTopUpPath(network: KeyWalletNetwork = .mainnet,
                                           identityIndex: UInt32,
                                           topupIndex: UInt32) throws -> String {
        var error = FFIError()
        let maxPathLen = 256
        let pathBuffer = UnsafeMutablePointer<CChar>.allocate(capacity: maxPathLen)
        defer {
            pathBuffer.deallocate()
        }
        
        let success = derivation_identity_topup_path(
            network.ffiValue, identityIndex, topupIndex,
            pathBuffer, maxPathLen, &error)
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }
        
        guard success else {
            throw KeyWalletError(ffiError: error)
        }
        
        return String(cString: pathBuffer)
    }
    
    /// Get identity authentication path
    /// - Parameters:
    ///   - network: The network type
    ///   - identityIndex: The identity index
    ///   - keyIndex: The key index
    /// - Returns: The derivation path string
    public static func getIdentityAuthenticationPath(network: KeyWalletNetwork = .mainnet,
                                                    identityIndex: UInt32,
                                                    keyIndex: UInt32) throws -> String {
        var error = FFIError()
        let maxPathLen = 256
        let pathBuffer = UnsafeMutablePointer<CChar>.allocate(capacity: maxPathLen)
        defer {
            pathBuffer.deallocate()
        }
        
        let success = derivation_identity_authentication_path(
            network.ffiValue, identityIndex, keyIndex,
            pathBuffer, maxPathLen, &error)
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }
        
        guard success else {
            throw KeyWalletError(ffiError: error)
        }
        
        return String(cString: pathBuffer)
    }
    
    /// Parse a derivation path string to indices
    /// - Parameter path: The derivation path string
    /// - Returns: Tuple of (indices, hardened flags)
    public static func parsePath(_ path: String) throws -> (indices: [UInt32], hardened: [Bool]) {
        var error = FFIError()
        var indicesPtr: UnsafeMutablePointer<UInt32>?
        var hardenedPtr: UnsafeMutablePointer<Bool>?
        var count: size_t = 0
        
        let success = path.withCString { pathCStr in
            derivation_path_parse(pathCStr, &indicesPtr, &hardenedPtr, &count, &error)
        }
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
            if let indices = indicesPtr, let hardened = hardenedPtr {
                derivation_path_free(indices, hardened, count)
            }
        }
        
        guard success, let indices = indicesPtr, let hardened = hardenedPtr else {
            throw KeyWalletError(ffiError: error)
        }
        
        // Copy the data before freeing
        var indicesArray: [UInt32] = []
        var hardenedArray: [Bool] = []
        
        for i in 0..<count {
            indicesArray.append(indices[i])
            hardenedArray.append(hardened[i])
        }
        
        return (indices: indicesArray, hardened: hardenedArray)
    }
}

/// Extended private key handle
public class ExtendedPrivateKey {
    private let handle: OpaquePointer
    
    internal init(handle: OpaquePointer) {
        self.handle = handle
    }
    
    deinit {
        derivation_xpriv_free(handle)
    }
    
    /// Convert to extended public key
    public func toPublicKey() throws -> ExtendedPublicKey {
        var error = FFIError()
        guard let xpubHandle = derivation_xpriv_to_xpub(handle, &error) else {
            defer {
                if error.message != nil {
                    error_message_free(error.message)
                }
            }
            throw KeyWalletError(ffiError: error)
        }
        
        return ExtendedPublicKey(handle: xpubHandle)
    }
    
    /// Get string representation
    public func toString() throws -> String {
        var error = FFIError()
        guard let strPtr = derivation_xpriv_to_string(handle, &error) else {
            defer {
                if error.message != nil {
                    error_message_free(error.message)
                }
            }
            throw KeyWalletError(ffiError: error)
        }
        
        let str = String(cString: strPtr)
        derivation_string_free(strPtr)
        return str
    }
}

/// Extended public key handle
public class ExtendedPublicKey {
    private let handle: OpaquePointer
    
    internal init(handle: OpaquePointer) {
        self.handle = handle
    }
    
    deinit {
        derivation_xpub_free(handle)
    }
    
    /// Get string representation
    public func toString() throws -> String {
        var error = FFIError()
        guard let strPtr = derivation_xpub_to_string(handle, &error) else {
            defer {
                if error.message != nil {
                    error_message_free(error.message)
                }
            }
            throw KeyWalletError(ffiError: error)
        }
        
        let str = String(cString: strPtr)
        derivation_string_free(strPtr)
        return str
    }
    
    /// Get fingerprint (4 bytes)
    public func getFingerprint() throws -> Data {
        var error = FFIError()
        var fingerprint = Data(count: 4)
        
        let success = fingerprint.withUnsafeMutableBytes { bytes in
            let ptr = bytes.bindMemory(to: UInt8.self).baseAddress
            return derivation_xpub_fingerprint(handle, ptr, &error)
        }
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }
        
        guard success else {
            throw KeyWalletError(ffiError: error)
        }
        
        return fingerprint
    }
}