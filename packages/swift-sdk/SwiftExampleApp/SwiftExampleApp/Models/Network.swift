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
            return dash_sdk_DashSDKNetwork(rawValue: 0)
        case .testnet:
            return dash_sdk_DashSDKNetwork(rawValue: 1)
        case .devnet:
            return dash_sdk_DashSDKNetwork(rawValue: 2)
        }
    }
    
    static var defaultNetwork: Network {
        return .testnet
    }
}