import Foundation
import CSwiftDashSDK

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
    
    var sdkNetwork: SwiftDashSwiftDashNetwork? {
        switch self {
        case .mainnet:
            return SwiftDashSwiftDashNetwork(rawValue: 0) // mainnet
        case .testnet:
            return SwiftDashSwiftDashNetwork(rawValue: 1) // testnet
        case .devnet:
            return SwiftDashSwiftDashNetwork(rawValue: 2) // devnet/regtest
        }
    }
    
    static var defaultNetwork: Network {
        return .testnet
    }
}