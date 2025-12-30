import Foundation

/// App-level network enum (distinct from the SDK's DashSDKNetwork typealias)
public enum AppNetwork: String, CaseIterable, Codable, Sendable {
    case mainnet = "mainnet"
    case testnet = "testnet"
    case regtest = "regtest"
    case devnet = "devnet"
    
    init(network: KeyWalletNetwork) {
        switch network {
        case .mainnet: self = .mainnet  // Dash = 0
        case .testnet: self = .testnet  // Testnet = 1
        case .regtest: self = .regtest  // Regtest = 2
        case .devnet: self = .devnet   // Devnet = 3
        default: self = .mainnet
        }
    }

    public var displayName: String {
        switch self {
        case .mainnet:
            return "Mainnet"
        case .testnet:
            return "Testnet"
        case .regtest:
            return "Regtest"
        case .devnet:
            return "Devnet"
        }
    }

    public var sdkNetwork: DashSDKNetwork {
        switch self {
        case .mainnet:
            return DashSDKNetwork(rawValue: 0)
        case .testnet:
            return DashSDKNetwork(rawValue: 1)
        case .regtest:
            return DashSDKNetwork(rawValue: 2)
        case .devnet:
            return DashSDKNetwork(rawValue: 3)
        }
    }

    public static var defaultNetwork: AppNetwork {
        return .testnet
    }

    // Convert to KeyWalletNetwork for wallet operations
    public func toKeyWalletNetwork() -> KeyWalletNetwork {
        switch self {
        case .mainnet:
            return .mainnet
        case .testnet:
            return .testnet
        case .regtest:
            return .regtest
        case .devnet:
            return .devnet
        }
    }
}
