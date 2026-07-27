import Foundation

// MARK: - Testnet Node Models

/// Container for testnet node configurations
public struct TestnetNodes: Codable, Sendable {
    public let masternodes: [String: MasternodeInfo]
    public let hpMasternodes: [String: HPMasternodeInfo]

    public init(masternodes: [String: MasternodeInfo], hpMasternodes: [String: HPMasternodeInfo]) {
        self.masternodes = masternodes
        self.hpMasternodes = hpMasternodes
    }

    enum CodingKeys: String, CodingKey {
        case masternodes
        case hpMasternodes = "hp_masternodes"
    }
}

/// Standard masternode information
public struct MasternodeInfo: Codable, Sendable {
    public let proTxHash: String
    public let owner: KeyInfo
    public let voter: KeyInfo

    public init(proTxHash: String, owner: KeyInfo, voter: KeyInfo) {
        self.proTxHash = proTxHash
        self.owner = owner
        self.voter = voter
    }

    enum CodingKeys: String, CodingKey {
        case proTxHash = "pro-tx-hash"
        case owner
        case voter
    }
}

/// High-performance masternode information
public struct HPMasternodeInfo: Codable, Sendable {
    public let protxTxHash: String
    public let owner: KeyInfo
    public let voter: KeyInfo
    public let payout: KeyInfo

    public init(protxTxHash: String, owner: KeyInfo, voter: KeyInfo, payout: KeyInfo) {
        self.protxTxHash = protxTxHash
        self.owner = owner
        self.voter = voter
        self.payout = payout
    }

    enum CodingKeys: String, CodingKey {
        case protxTxHash = "protx-tx-hash"
        case owner
        case voter
        case payout
    }
}

/// Masternode key information
public struct KeyInfo: Codable, Sendable {
    public let privateKey: String

    public init(privateKey: String) {
        self.privateKey = privateKey
    }

    enum CodingKeys: String, CodingKey {
        case privateKey = "private_key"
    }
}

// MARK: - Testnet Nodes Loader

/// Loads testnet node configurations from YAML or provides sample data
public class TestnetNodesLoader {

    /// Load testnet nodes from a YAML file
    /// - Parameter fileName: The name of the YAML file (default: ".testnet_nodes.yml")
    /// - Returns: TestnetNodes configuration, or sample data if file not found
    public static func loadFromYAML(fileName: String = ".testnet_nodes.yml") -> TestnetNodes? {
        // In a real implementation, this would load from the app bundle or documents directory
        // For now, return sample data for demonstration
        return createSampleTestnetNodes()
    }

    /// Create sample testnet nodes for testing
    /// - Returns: TestnetNodes with sample data
    public static func createSampleTestnetNodes() -> TestnetNodes {
        let sampleMasternode = MasternodeInfo(
            proTxHash: "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            owner: KeyInfo(privateKey: "cVwySadFkE9GhznGjLHtqGJ2FPvkEbvEE1WnMCCvhUZZMWJmTzrq"),
            voter: KeyInfo(privateKey: "cRtLvGwabTRyJdYfWQ9H2hsg9y5TN9vMEX8PvnYVfcaJdNjNQzNb")
        )

        let sampleHPMasternode = HPMasternodeInfo(
            protxTxHash: "fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321",
            owner: KeyInfo(privateKey: "cN5YgNRq8rbcJwngdp3fRzv833E7Z74TsF8nB6GhzRg8Gd9aGWH1"),
            voter: KeyInfo(privateKey: "cSBnVM4xvxarwGQuAfQFwqDg9k5tErHUHzgWsEfD4zdwUasvqRVY"),
            payout: KeyInfo(privateKey: "cMnkMfwMVmCM3NkF6p6dLKJMcvgN1BQvLRMvdWMjELUTdJM6QpyG")
        )

        return TestnetNodes(
            masternodes: ["test-masternode-1": sampleMasternode],
            hpMasternodes: ["test-hpmn-1": sampleHPMasternode]
        )
    }
}
