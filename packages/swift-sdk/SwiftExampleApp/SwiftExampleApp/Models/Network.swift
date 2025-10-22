import Foundation
import SwiftDashSDK

enum Network: String, CaseIterable, Codable {
    case mainnet = "mainnet"
    case testnet = "testnet"
    case devnet = "devnet"
    
    var displayName: String {
        switch self {
        case .mainnet:
            return "Mainnet"
        case .testnet:
            return "Testnet"
        case .devnet:
            return "Devnet"
        }
    }
    
    var sdkNetwork: SwiftDashSDK.Network {
        switch self {
        case .mainnet:
            return DashSDKNetwork_SDKMainnet
        case .testnet:
            return DashSDKNetwork_SDKTestnet
        case .devnet:
            return DashSDKNetwork_SDKDevnet
        }
    }
    
    static var defaultNetwork: Network {
        return .testnet
    }
    
    // Convert to KeyWalletNetwork for wallet operations
    func toKeyWalletNetwork() -> KeyWalletNetwork {
        switch self {
        case .mainnet:
            return .mainnet
        case .testnet:
            return .testnet
        case .devnet:
            return .devnet
        }
    }
}
