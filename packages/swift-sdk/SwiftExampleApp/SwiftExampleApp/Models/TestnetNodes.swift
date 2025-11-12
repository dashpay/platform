import Foundation

// MARK: - Testnet Node Models
struct TestnetNodes: Codable {
    let masternodes: [String: MasternodeInfo]
    let hpMasternodes: [String: HPMasternodeInfo]
    
    enum CodingKeys: String, CodingKey {
        case masternodes
        case hpMasternodes = "hp_masternodes"
    }
}

struct MasternodeInfo: Codable {
    let proTxHash: String
    let owner: KeyInfo
    let voter: KeyInfo
    
    enum CodingKeys: String, CodingKey {
        case proTxHash = "pro-tx-hash"
        case owner
        case voter
    }
}

struct HPMasternodeInfo: Codable {
    let protxTxHash: String
    let owner: KeyInfo
    let voter: KeyInfo
    let payout: KeyInfo
    
    enum CodingKeys: String, CodingKey {
        case protxTxHash = "protx-tx-hash"
        case owner
        case voter
        case payout
    }
}

struct KeyInfo: Codable {
    let privateKey: String
    
    enum CodingKeys: String, CodingKey {
        case privateKey = "private_key"
    }
}

// MARK: - Testnet Nodes Loader
class TestnetNodesLoader {
    static func loadFromYAML(fileName: String = ".testnet_nodes.yml") -> TestnetNodes? {
        // In a real app, this would load from the app bundle or documents directory
        // For now, return sample data for demonstration
        return createSampleTestnetNodes()
    }
    
    private static func createSampleTestnetNodes() -> TestnetNodes {
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