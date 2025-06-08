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