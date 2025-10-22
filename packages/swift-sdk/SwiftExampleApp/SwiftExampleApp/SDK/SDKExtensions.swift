import Foundation
import SwiftDashSDK

// MARK: - Network Helper  
// C enums are imported as structs with RawValue in Swift
// We'll use the raw values directly

extension SDK {
    var network: SwiftDashSDK.Network {
        // In a real implementation, we would track the network during initialization
        // For now, return testnet as default
        return DashSDKNetwork_SDKTestnet // Testnet
    }
}

// MARK: - Signer Protocol
protocol Signer {
    func sign(identityPublicKey: Data, data: Data) -> Data?
    func canSign(identityPublicKey: Data) -> Bool
}

// MARK: - SDK Extensions for the example app
// No global signer storage is kept; signers are created and used at call sites.
