import Foundation
import Security
import SwiftDashSDK

// MARK: - Testnet Node Models

/// Container for testnet node configurations
struct TestnetNodes: Codable, Sendable {
    let masternodes: [String: MasternodeInfo]
    let hpMasternodes: [String: HPMasternodeInfo]

    init(masternodes: [String: MasternodeInfo], hpMasternodes: [String: HPMasternodeInfo]) {
        self.masternodes = masternodes
        self.hpMasternodes = hpMasternodes
    }

    enum CodingKeys: String, CodingKey {
        case masternodes
        case hpMasternodes = "hp_masternodes"
    }
}

/// Standard masternode information
struct MasternodeInfo: Codable, Sendable {
    let proTxHash: String
    let owner: KeyInfo
    let voter: KeyInfo

    init(proTxHash: String, owner: KeyInfo, voter: KeyInfo) {
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
struct HPMasternodeInfo: Codable, Sendable {
    let protxTxHash: String
    let owner: KeyInfo
    let voter: KeyInfo
    let payout: KeyInfo

    init(protxTxHash: String, owner: KeyInfo, voter: KeyInfo, payout: KeyInfo) {
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
struct KeyInfo: Codable, Sendable {
    let privateKey: String

    init(privateKey: String) {
        self.privateKey = privateKey
    }

    enum CodingKeys: String, CodingKey {
        case privateKey = "private_key"
    }
}

// MARK: - Testnet Nodes Loader

/// Loads testnet node configurations from YAML or provides sample data
class TestnetNodesLoader {

    /// Load testnet nodes from a YAML file
    /// - Parameter fileName: The name of the YAML file (default: ".testnet_nodes.yml")
    /// - Returns: TestnetNodes configuration, or sample data if file not found
    static func loadFromYAML(fileName: String = ".testnet_nodes.yml") -> TestnetNodes? {
        // In a real implementation, this would load from the app bundle or documents directory
        // For now, return sample data for demonstration
        return createSampleTestnetNodes()
    }

    /// Create fresh sample testnet nodes for testing.
    /// - Returns: Random sample data, or empty node lists if generation fails.
    static func createSampleTestnetNodes() -> TestnetNodes {
        guard let masternodeHash = randomBytes(),
              let hpmnHash = randomBytes(),
              let masternodeOwner = randomKey(),
              let masternodeVoter = randomKey(),
              let hpmnOwner = randomKey(),
              let hpmnVoter = randomKey(),
              let hpmnPayout = randomKey() else {
            return TestnetNodes(masternodes: [:], hpMasternodes: [:])
        }

        let sampleMasternode = MasternodeInfo(
            proTxHash: masternodeHash.toHexString(),
            owner: masternodeOwner,
            voter: masternodeVoter
        )

        let sampleHPMasternode = HPMasternodeInfo(
            protxTxHash: hpmnHash.toHexString(),
            owner: hpmnOwner,
            voter: hpmnVoter,
            payout: hpmnPayout
        )

        return TestnetNodes(
            masternodes: ["test-masternode-1": sampleMasternode],
            hpMasternodes: ["test-hpmn-1": sampleHPMasternode]
        )
    }

    private static func randomBytes() -> Data? {
        var bytes = Data(count: 32)
        let status = bytes.withUnsafeMutableBytes { (buffer: UnsafeMutableRawBufferPointer) in
            guard let base = buffer.baseAddress else { return errSecParam }
            return SecRandomCopyBytes(kSecRandomDefault, buffer.count, base)
        }
        return status == errSecSuccess ? bytes : nil
    }

    private static func randomKey() -> KeyInfo? {
        guard let bytes = randomBytes(),
              let wif = WIFParser.encodeToWIF(bytes, network: .testnet) else { return nil }
        return KeyInfo(privateKey: wif)
    }
}
