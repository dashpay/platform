import Foundation
import SwiftDashSDK

// MARK: - Network Helper  
// C enums are imported as structs with RawValue in Swift
// We'll use the raw values directly

extension SDK {
    var network: SwiftDashSDK.Network {
        // In a real implementation, we would track the network during initialization
        // For now, return testnet as default
        return DashSDKNetwork(rawValue: 1) // Testnet
    }
}

// MARK: - Signer Protocol
protocol Signer {
    func sign(identityPublicKey: Data, data: Data) -> Data?
    func canSign(identityPublicKey: Data) -> Bool
}

// Global signer storage for C callbacks
private var globalSignerStorage: Signer?

// MARK: - SDK Extensions for the example app
extension SDK {
    /// Initialize SDK with a custom signer for the example app
    convenience init(network: SwiftDashSDK.Network, signer: Signer) throws {
        // Store the signer globally for C callbacks
        globalSignerStorage = signer
        
        // Initialize the SDK normally
        try self.init(network: network)
    }
}
