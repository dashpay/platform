import Foundation
import CSwiftDashSDK

/// Swift wrapper for the Dash Platform SDK
public class SDK {
    public private(set) var handle: OpaquePointer?
    
    /// Identities operations
    public lazy var identities = Identities(sdk: self)
    
    /// Contracts operations  
    public lazy var contracts = Contracts(sdk: self)
    
    /// Initialize the SDK library (call once at app startup)
    public static func initialize() {
        swift_dash_sdk_init()
    }
    
    /// Create a new SDK instance
    public init(network: SwiftDashSwiftDashNetwork) throws {
        let config: SwiftDashSwiftDashSDKConfig
        
        switch network {
        case SwiftDashSwiftDashNetwork(rawValue: 0): // Mainnet
            config = swift_dash_sdk_config_mainnet()
        case SwiftDashSwiftDashNetwork(rawValue: 1): // Testnet
            config = swift_dash_sdk_config_testnet()
        case SwiftDashSwiftDashNetwork(rawValue: 3): // Local
            config = swift_dash_sdk_config_local()
        default:
            // For devnet or unknown, use testnet config as a fallback
            config = swift_dash_sdk_config_testnet()
        }
        
        handle = swift_dash_sdk_create(config)
        
        if handle == nil {
            throw SDKError.internalError("Failed to create SDK instance")
        }
    }
    
    deinit {
        if let handle = handle {
            swift_dash_sdk_destroy(handle)
        }
    }
    
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
    
    public static func fromSwiftDashError(_ error: SwiftDashError) -> SDKError {
        let message = error.message != nil ? String(cString: error.message!) : "Unknown error"
        
        switch SwiftDashSwiftDashErrorCode(rawValue: error.code) {
        case SwiftDashSwiftDashErrorCode(rawValue: 1): // InvalidParameter
            return .invalidParameter(message)
        case SwiftDashSwiftDashErrorCode(rawValue: 2): // InvalidState
            return .invalidState(message)
        case SwiftDashSwiftDashErrorCode(rawValue: 3): // NetworkError
            return .networkError(message)
        case SwiftDashSwiftDashErrorCode(rawValue: 4): // SerializationError
            return .serializationError(message)
        case SwiftDashSwiftDashErrorCode(rawValue: 5): // ProtocolError
            return .protocolError(message)
        case SwiftDashSwiftDashErrorCode(rawValue: 6): // CryptoError
            return .cryptoError(message)
        case SwiftDashSwiftDashErrorCode(rawValue: 7): // NotFound
            return .notFound(message)
        case SwiftDashSwiftDashErrorCode(rawValue: 8): // Timeout
            return .timeout(message)
        case SwiftDashSwiftDashErrorCode(rawValue: 9): // NotImplemented
            return .notImplemented(message)
        case SwiftDashSwiftDashErrorCode(rawValue: 99): // InternalError
            return .internalError(message)
        default:
            return .unknown(message)
        }
    }
}

/// Swift wrapper for SwiftDashError
public struct SwiftDashError {
    public var code: UInt32 = 0
    public var message: UnsafeMutablePointer<CChar>?
}

/// Identities operations
public class Identities {
    private weak var sdk: SDK?
    
    init(sdk: SDK) {
        self.sdk = sdk
    }
    
    /// Get an identity by ID
    public func get(id: String) throws -> Identity? {
        guard let sdk = sdk, let handle = sdk.handle else {
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
    public func getBalance(id: String) throws -> UInt64 {
        guard let sdk = sdk, let handle = sdk.handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let balance = swift_dash_identity_get_balance(handle, id)
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
        
        // Create array of tuples (32-byte arrays)
        let idTuples: [(UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8)] = 
            idByteArrays.map { bytes in
                (bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                 bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
                 bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23],
                 bytes[24], bytes[25], bytes[26], bytes[27], bytes[28], bytes[29], bytes[30], bytes[31])
            }
        
        guard let resultMapPtr = idTuples.withUnsafeBufferPointer({ buffer -> UnsafeMutablePointer<SwiftDashDashSDKIdentityBalanceMap>? in
            let idsPtr = buffer.baseAddress
            return swift_dash_identities_fetch_balances(handle, idsPtr, idByteArrays.count)
        }) else {
            throw SDKError.networkError("Failed to fetch balances")
        }
        
        defer {
            swift_dash_identity_balance_map_free(resultMapPtr)
        }
        
        let resultMap = resultMapPtr.pointee
        
        // Convert to dictionary
        var balances: [Data: UInt64?] = [:]
        
        if resultMap.count > 0 && resultMap.entries != nil {
            for i in 0..<resultMap.count {
                let entry = resultMap.entries[i]
                let idData = withUnsafeBytes(of: entry.identity_id) { Data($0) }
                
                // Check if balance is u64::MAX (which means not found)
                if entry.balance == UInt64.max {
                    balances[idData] = nil
                } else {
                    balances[idData] = entry.balance
                }
            }
        }
        
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
        guard let sdk = sdk, let handle = sdk.handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // TODO: Call C function to get data contract
        // For now, return nil
        return nil
    }
}

// MARK: - Data Extensions

extension Data {
    /// Convert Data to hex string
    func toHexString() -> String {
        map { String(format: "%02x", $0) }.joined()
    }
}