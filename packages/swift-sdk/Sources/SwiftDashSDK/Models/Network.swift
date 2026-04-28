import Foundation

/// App-level network enum (distinct from the SDK's DashSDKNetwork typealias).
///
/// Raw values intentionally match `KeyWalletNetwork.rawValue` so conversions
/// between the two are a direct raw-value passthrough.
public enum AppNetwork: Int, CaseIterable, Codable, Sendable {
    case mainnet = 0
    case testnet = 1
    case regtest = 2
    case devnet = 3

    init(network: KeyWalletNetwork) {
        // Raw values are aligned with `KeyWalletNetwork` by construction,
        // so every case maps; the force-unwrap can never fire.
        self = AppNetwork(rawValue: Int(network.rawValue))!
    }

    public var displayName: String {
        switch self {
        case .mainnet:
            return "Mainnet"
        case .testnet:
            return "Testnet"
        case .regtest:
            return "Local"
        case .devnet:
            return "Devnet"
        }
    }

    /// Lowercase canonical name ("mainnet", "testnet", "regtest",
    /// "devnet"). Used as the persisted string key for sibling
    /// `PersistentX` rows (identities, documents, contracts, sync
    /// state) that still store network as `String`.
    public var networkName: String {
        switch self {
        case .mainnet:
            return "mainnet"
        case .testnet:
            return "testnet"
        case .regtest:
            return "regtest"
        case .devnet:
            return "devnet"
        }
    }

    public var sdkNetwork: DashSDKNetwork {
        DashSDKNetwork(rawValue: UInt32(rawValue))
    }

    public static var defaultNetwork: AppNetwork {
        return .testnet
    }

    // Convert to KeyWalletNetwork for wallet operations
    public func toKeyWalletNetwork() -> KeyWalletNetwork {
        // Raw values are aligned by construction; unwrap is safe.
        return KeyWalletNetwork(rawValue: UInt32(rawValue))!
    }
}
