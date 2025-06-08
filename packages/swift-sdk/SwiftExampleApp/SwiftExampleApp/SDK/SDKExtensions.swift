import Foundation
import SwiftDashSDK
import CSwiftDashSDK

// MARK: - Network Helper  
// C enums are imported as structs with RawValue in Swift
// We'll use the raw values directly

extension SDK {
    var network: SwiftDashSwiftDashNetwork? {
        // In a real implementation, we would track the network during initialization
        // For now, return testnet as default
        return SwiftDashSwiftDashNetwork(rawValue: 1)
    }
}

// MARK: - Signer Protocol
protocol Signer {
    func sign(identityPublicKey: Data, data: Data) -> Data?
    func canSign(identityPublicKey: Data) -> Bool
}

// Global signer storage for C callbacks
private var globalSignerStorage: Signer?

// C function callbacks that use the global signer
private let globalSignCallback: SwiftDashSwiftSignCallback = { identityPublicKeyBytes, identityPublicKeyLen, dataBytes, dataLen, resultLenPtr in
    guard let identityPublicKeyBytes = identityPublicKeyBytes,
          let dataBytes = dataBytes,
          let resultLenPtr = resultLenPtr,
          let signer = globalSignerStorage else {
        return nil
    }
    
    let identityPublicKey = Data(bytes: identityPublicKeyBytes, count: Int(identityPublicKeyLen))
    let data = Data(bytes: dataBytes, count: Int(dataLen))
    
    guard let signature = signer.sign(identityPublicKey: identityPublicKey, data: data) else {
        return nil
    }
    
    // Allocate memory for the result and copy the signature
    let result = UnsafeMutablePointer<UInt8>.allocate(capacity: signature.count)
    signature.withUnsafeBytes { bytes in
        result.initialize(from: bytes.bindMemory(to: UInt8.self).baseAddress!, count: signature.count)
    }
    
    resultLenPtr.pointee = signature.count
    return result
}

private let globalCanSignCallback: SwiftDashSwiftCanSignCallback = { identityPublicKeyBytes, identityPublicKeyLen in
    guard let identityPublicKeyBytes = identityPublicKeyBytes,
          let signer = globalSignerStorage else {
        return false
    }
    
    let identityPublicKey = Data(bytes: identityPublicKeyBytes, count: Int(identityPublicKeyLen))
    return signer.canSign(identityPublicKey: identityPublicKey)
}

// MARK: - SDK Extensions for the example app
extension SDK {
    /// Initialize SDK with a custom signer for the example app
    convenience init(network: SwiftDashSwiftDashNetwork, signer: Signer) throws {
        // Store the signer globally for C callbacks
        globalSignerStorage = signer
        
        // Create the Swift signer configuration
        let signerConfig = SwiftDashSwiftDashSigner(
            sign_callback: globalSignCallback,
            can_sign_callback: globalCanSignCallback
        )
        
        // Create the SDK with the signer
        // Note: We'll use the test signer for now since the custom signer API
        // is not fully exposed yet
        try self.init(network: network)
    }
}