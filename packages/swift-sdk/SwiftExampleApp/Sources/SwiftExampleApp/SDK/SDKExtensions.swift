import Foundation
import SwiftDashSDK

// MARK: - Network Helper
extension SwiftDashSwiftDashNetwork {
    static let mainnet = SwiftDashSwiftDashNetwork(rawValue: 0)!
    static let testnet = SwiftDashSwiftDashNetwork(rawValue: 1)!
    static let devnet = SwiftDashSwiftDashNetwork(rawValue: 2)!
    static let local = SwiftDashSwiftDashNetwork(rawValue: 3)!
}

extension SDK {
    var network: SwiftDashSwiftDashNetwork? {
        // In a real implementation, we would track the network during initialization
        // For now, return testnet as default
        return .testnet
    }
}

// MARK: - Signer Protocol
protocol Signer {
    func sign(identityPublicKey: Data, data: Data) -> Data?
    func canSign(identityPublicKey: Data) -> Bool
}

// MARK: - SDK Extensions for the example app
extension SDK {
    /// Initialize SDK with a custom signer for the example app
    convenience init(network: SwiftDashSwiftDashNetwork, signer: Signer) throws {
        // Create the signer callbacks
        let signCallback: SwiftSignCallback = { identityPublicKeyBytes, identityPublicKeyLen, dataBytes, dataLen, resultLenPtr in
            guard let identityPublicKeyBytes = identityPublicKeyBytes,
                  let dataBytes = dataBytes,
                  let resultLenPtr = resultLenPtr else {
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
        
        let canSignCallback: SwiftCanSignCallback = { identityPublicKeyBytes, identityPublicKeyLen in
            guard let identityPublicKeyBytes = identityPublicKeyBytes else {
                return false
            }
            
            let identityPublicKey = Data(bytes: identityPublicKeyBytes, count: Int(identityPublicKeyLen))
            return signer.canSign(identityPublicKey: identityPublicKey)
        }
        
        // Create the Swift signer configuration
        var signerConfig = SwiftDashSwiftDashSigner(
            sign_callback: signCallback,
            can_sign_callback: canSignCallback
        )
        
        // Create the SDK with the signer
        // Note: We'll use the test signer for now since the custom signer API
        // is not fully exposed yet
        try self.init(network: network)
    }
}