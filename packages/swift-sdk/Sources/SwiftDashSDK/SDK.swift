import Foundation
import DashSDKFFI

// MARK: - Data Extensions
extension Data {
    /// Convert Data to Base58 string
    func toBase58() -> String {
        let alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
        var bytes = Array(self)
        var encoded = ""
        var zeroCount = 0
        
        // Count leading zeros
        for byte in bytes {
            if byte == 0 {
                zeroCount += 1
            } else {
                break
            }
        }
        
        // Remove leading zeros for processing
        bytes = Array(bytes.dropFirst(zeroCount))
        
        // Convert bytes to base58
        while !bytes.isEmpty {
            var remainder: UInt = 0
            var newBytes: [UInt8] = []
            
            for byte in bytes {
                let temp = UInt(byte) + remainder * 256
                remainder = temp % 58
                let quotient = temp / 58
                if !newBytes.isEmpty || quotient > 0 {
                    newBytes.append(UInt8(quotient))
                }
            }
            
            bytes = newBytes
            encoded = String(alphabet[alphabet.index(alphabet.startIndex, offsetBy: Int(remainder))]) + encoded
        }
        
        // Add '1' for each leading zero byte
        encoded = String(repeating: "1", count: zeroCount) + encoded
        
        return encoded
    }
    
    /// Convert to hex string
    func toHexString() -> String {
        return self.map { String(format: "%02x", $0) }.joined()
    }
}

/// Swift wrapper for the Dash Platform SDK
public class SDK {
    public private(set) var handle: OpaquePointer?
    
    /// Identities operations
    public lazy var identities = Identities(sdk: self)
    
    /// Contracts operations  
    public lazy var contracts = Contracts(sdk: self)
    
    /// Initialize the SDK library (call once at app startup)
    public static func initialize() {
        dash_sdk_init()
    }
    
    /// Testnet DAPI addresses provided by the user
    private static let testnetDAPIAddresses = [
        "https://54.186.161.118:1443",
        "https://52.43.70.6:1443",
        "https://18.237.42.109:1443",
        "https://52.42.192.140:1443",
        "https://35.166.242.82:1443",
        "https://35.93.135.201:1443",
        "https://35.91.145.176:1443",
        "https://52.10.229.11:1443",
        "https://54.200.102.141:1443",
        "https://52.33.28.47:1443",
        "https://54.189.18.97:1443",
        "https://44.236.189.81:1443",
        "https://52.88.31.190:1443",
        "https://52.10.216.154:1443",
        "https://35.85.157.172:1443",
        "https://44.228.242.181:1443",
        "https://54.69.121.35:1443",
        "https://52.89.154.228:1443",
        "https://35.163.144.230:1443",
        "https://52.32.4.156:1443"
    ].joined(separator: ",")
    
    /// Create a new SDK instance
    public init(network: Network, useTrustedSetup: Bool = true) throws {
        var config = DashSDKConfig()
        
        // Map network - in C enums, Swift imports them as raw values
        config.network = network
        
        // Set DAPI addresses based on network
        switch network {
        case DashSDKNetwork(rawValue: 0): // Mainnet
            config.dapi_addresses = nil // Use default mainnet addresses
        case DashSDKNetwork(rawValue: 1): // Testnet
            // Use the testnet addresses provided by the user
            config.dapi_addresses = nil // Will be set below
        case DashSDKNetwork(rawValue: 2): // Devnet
            config.dapi_addresses = nil // Use default devnet addresses
        case DashSDKNetwork(rawValue: 3): // Local
            config.dapi_addresses = nil // Use default local addresses
        default:
            config.dapi_addresses = nil
        }
        
        config.skip_asset_lock_proof_verification = false
        config.request_retry_count = 3
        config.request_timeout_ms = 30000 // 30 seconds
        
        // Create SDK with new FFI
        let result: DashSDKResult
        if network == DashSDKNetwork(rawValue: 1) { // Testnet
            result = Self.testnetDAPIAddresses.withCString { addressesCStr -> DashSDKResult in
                var mutableConfig = config
                mutableConfig.dapi_addresses = addressesCStr
                if useTrustedSetup {
                    return dash_sdk_create_trusted(&mutableConfig)
                } else {
                    return dash_sdk_create(&mutableConfig)
                }
            }
        } else {
            if useTrustedSetup {
                result = dash_sdk_create_trusted(&config)
            } else {
                result = dash_sdk_create(&config)
            }
        }
        
        // Check for errors
        if result.error != nil {
            let error = result.error!.pointee
            let errorMessage = error.message != nil ? String(cString: error.message!) : "Unknown error"
            defer {
                dash_sdk_error_free(result.error)
            }
            
            throw SDKError.internalError("Failed to create SDK: \(errorMessage)")
        }
        
        guard result.data != nil else {
            throw SDKError.internalError("No SDK handle returned")
        }
        
        // Store the handle
        handle = OpaquePointer(result.data)
    }
    
    deinit {
        if let handle = handle {
            // The handle is already the correct type for the C function
            dash_sdk_destroy(handle)
        }
    }
    
    // TODO: Re-enable when CDashSDKFFI module is working
    // /// Test the new FFI connection
    // public func testNewFFI() -> Bool {
    //     guard let newHandle = newFFIHandle else {
    //         print("No new FFI handle available")
    //         return false
    //     }
    //     
    //     // Try to get the network from the new FFI
    //     let sdkHandle = UnsafePointer<dash_sdk_SDKHandle>(OpaquePointer(newHandle))
    //     let network = dash_sdk_get_network(sdkHandle)
    //     
    //     print("New FFI network: \(network)")
    //     return true
    // }
    
    /// Get an identity by ID
    public func getIdentity(id: String) async throws -> Identity? {
        // This would call the C function to get identity
        // For now, return nil as placeholder
        return nil
    }
    
    /// Get a data contract by ID
    public func getDataContract(id: String) async throws -> DataContract? {
        // This would call the C function to get data contract
        // For now, return nil as placeholder
        return nil
    }
}

/// SDK Error handling
public enum SDKError: Error {
    case invalidParameter(String)
    case invalidState(String)
    case networkError(String)
    case serializationError(String)
    case protocolError(String)
    case cryptoError(String)
    case notFound(String)
    case timeout(String)
    case notImplemented(String)
    case internalError(String)
    case unknown(String)
    
    public static func fromDashSDKError(_ error: DashSDKError) -> SDKError {
        let message = error.message != nil ? String(cString: error.message!) : "Unknown error"
        
        switch error.code {
        case DashSDKErrorCode(rawValue: 1): // Invalid parameter
            return .invalidParameter(message)
        case DashSDKErrorCode(rawValue: 2): // Invalid state
            return .invalidState(message)
        case DashSDKErrorCode(rawValue: 3): // Network error
            return .networkError(message)
        case DashSDKErrorCode(rawValue: 4): // Serialization error
            return .serializationError(message)
        case DashSDKErrorCode(rawValue: 5): // Protocol error
            return .protocolError(message)
        case DashSDKErrorCode(rawValue: 6): // Crypto error
            return .cryptoError(message)
        case DashSDKErrorCode(rawValue: 7): // Not found
            return .notFound(message)
        case DashSDKErrorCode(rawValue: 8): // Timeout
            return .timeout(message)
        case DashSDKErrorCode(rawValue: 9): // Not implemented
            return .notImplemented(message)
        case DashSDKErrorCode(rawValue: 99): // Internal error
            return .internalError(message)
        default:
            return .unknown(message)
        }
    }
}


/// Identities operations
public class Identities {
    private weak var sdk: SDK?
    
    init(sdk: SDK) {
        self.sdk = sdk
    }
    
    /// Get an identity by ID
    public func get(id: String) throws -> Identity? {
        guard let sdk = sdk, let _ = sdk.handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // TODO: Call C function to get identity
        // For now, return nil
        return nil
    }
    
    /// Get an identity by ID using Data
    public func get(id: Data) throws -> Identity? {
        guard id.count == 32 else {
            throw SDKError.invalidParameter("Identity ID must be exactly 32 bytes")
        }
        
        // Convert Data to hex string for now
        return try get(id: id.toHexString())
    }
    
    /// Get a single identity balance
    public func getBalance(id: Data) throws -> UInt64 {
        guard let sdk = sdk, let handle = sdk.handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        guard id.count == 32 else {
            throw SDKError.invalidParameter("Identity ID must be exactly 32 bytes")
        }
        
        // Convert Data to Base58 string (the FFI expects string IDs)
        let idString = id.toBase58()
        
        let result = idString.withCString { cString in
            // Handle is OpaquePointer which Swift should convert automatically
            return dash_sdk_identity_fetch_balance(handle, cString)
        }
        
        // Check for errors
        if result.error != nil {
            let error = result.error!.pointee
            defer {
                dash_sdk_error_free(result.error)
            }
            throw SDKError.fromDashSDKError(error)
        }
        
        guard result.data != nil else {
            throw SDKError.internalError("No balance data returned")
        }
        
        // Parse the balance from result
        let balancePtr = result.data.assumingMemoryBound(to: UInt64.self)
        let balance = balancePtr.pointee
        
        // Free the result data
        dash_sdk_bytes_free(result.data)
        
        return balance
    }
    
    /// Fetch balances for multiple identities using Data (32-byte arrays)
    /// - Parameter ids: Array of identity IDs as Data objects (must be exactly 32 bytes each)
    /// - Returns: Dictionary mapping identity IDs (as Data) to their balances (nil if identity not found)
    public func fetchBalances(ids: [Data]) throws -> [Data: UInt64?] {
        guard let sdk = sdk, let handle = sdk.handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        guard !ids.isEmpty else {
            return [:]
        }
        
        // Validate all IDs are 32 bytes
        for id in ids {
            guard id.count == 32 else {
                throw SDKError.invalidParameter("Identity ID must be exactly 32 bytes, got \(id.count)")
            }
        }
        
        // Convert Data to byte arrays
        let idByteArrays: [[UInt8]] = ids.map { Array($0) }
        
        // Create array of 32-byte arrays for FFI
        let idArrays: [(UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8)] = 
            idByteArrays.map { bytes in
                (bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                 bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
                 bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23],
                 bytes[24], bytes[25], bytes[26], bytes[27], bytes[28], bytes[29], bytes[30], bytes[31])
            }
        
        let result = idArrays.withUnsafeBufferPointer { buffer -> DashSDKResult in
            let idsPtr = buffer.baseAddress
            // The handle is already the correct type for the C function
            return dash_sdk_identities_fetch_balances(handle, idsPtr, UInt(ids.count))
        }
        
        // Check for errors
        if result.error != nil {
            let error = result.error!.pointee
            defer {
                dash_sdk_error_free(result.error)
            }
            throw SDKError.fromDashSDKError(error)
        }
        
        guard result.data != nil else {
            throw SDKError.internalError("No data returned from fetch balances")
        }
        
        // Parse the identity balance map
        let mapPtr = result.data.assumingMemoryBound(to: DashSDKIdentityBalanceMap.self)
        let map = mapPtr.pointee
        
        var balances: [Data: UInt64?] = [:]
        
        if map.count > 0 && map.entries != nil {
            for i in 0..<map.count {
                let entry = map.entries[Int(i)]
                let idData = withUnsafeBytes(of: entry.identity_id) { Data($0) }
                
                // Check if balance is u64::MAX (which means not found)
                if entry.balance == UInt64.max {
                    balances[idData] = nil
                } else {
                    balances[idData] = entry.balance
                }
            }
        }
        
        // Free the result
        dash_sdk_identity_balance_map_free(mapPtr)
        
        // Make sure all requested IDs are in the result
        for id in ids {
            if balances[id] == nil {
                balances[id] = nil
            }
        }
        
        return balances
    }
    
    // Helper function to convert hex string to bytes
    private func hexToBytes(_ hex: String) -> [UInt8]? {
        let hex = hex.trimmingCharacters(in: .whitespacesAndNewlines)
        guard hex.count == 64 else { return nil } // 32 bytes = 64 hex chars
        
        var bytes = [UInt8]()
        var index = hex.startIndex
        
        while index < hex.endIndex {
            let nextIndex = hex.index(index, offsetBy: 2)
            let byteString = hex[index..<nextIndex]
            
            if let byte = UInt8(byteString, radix: 16) {
                bytes.append(byte)
            } else {
                return nil
            }
            
            index = nextIndex
        }
        
        return bytes.count == 32 ? bytes : nil
    }
    
    // Helper function to convert bytes to hex string
    private func bytesToHex(_ bytes: [UInt8]) -> String {
        return bytes.map { String(format: "%02x", $0) }.joined()
    }
}

/// Contracts operations
public class Contracts {
    private weak var sdk: SDK?
    
    init(sdk: SDK) {
        self.sdk = sdk
    }
    
    /// Get a data contract by ID
    public func get(id: String) throws -> DataContract? {
        guard let sdk = sdk, let _ = sdk.handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // TODO: Call C function to get data contract
        // For now, return nil
        return nil
    }
}

