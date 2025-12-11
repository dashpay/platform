import Foundation

/// App-level network enum (distinct from the SDK's DashSDKNetwork typealias)
public enum AppNetwork: String, CaseIterable, Codable, Sendable {
    case mainnet = "mainnet"
    case testnet = "testnet"
    case devnet = "devnet"

    public var displayName: String {
        switch self {
        case .mainnet:
            return "Mainnet"
        case .testnet:
            return "Testnet"
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
        case .devnet:
            return .devnet
        }
    }
}
