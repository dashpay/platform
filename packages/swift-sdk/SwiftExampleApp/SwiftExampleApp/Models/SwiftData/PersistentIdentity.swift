import Foundation
import SwiftData
import SwiftDashSDK

// Re-export SDK types for backward compatibility
public typealias PersistentIdentity = SwiftDashSDK.PersistentIdentity

// App-specific extensions that depend on app types
extension SwiftDashSDK.PersistentIdentity {
    /// Convert to app's IdentityModel
    @MainActor
    func toIdentityModel() -> IdentityModel {
        let publicKeyModels = publicKeys.compactMap { $0.toIdentityPublicKey() }

        // Convert public keys with private keys to Data array by retrieving from keychain
        let privateKeyData = publicKeys
            .filter { $0.hasPrivateKeyIdentifier }
            .sorted(by: { $0.keyId < $1.keyId })
            .compactMap { persistentKey -> Data? in
                guard let identityData = Data.identifier(fromBase58: persistentKey.identityId) else { return nil }
                return KeychainManager.shared.retrievePrivateKey(identityId: identityData, keyIndex: persistentKey.keyId)
            }

        // Retrieve special keys from keychain
        let votingKey = votingPrivateKeyIdentifier != nil ?
            KeychainManager.shared.retrieveSpecialKey(identityId: identityId, keyType: .voting) : nil
        let ownerKey = ownerPrivateKeyIdentifier != nil ?
            KeychainManager.shared.retrieveSpecialKey(identityId: identityId, keyType: .owner) : nil
        let payoutKey = payoutPrivateKeyIdentifier != nil ?
            KeychainManager.shared.retrieveSpecialKey(identityId: identityId, keyType: .payout) : nil

        return IdentityModel(
            id: identityId,
            balance: UInt64(balance),
            isLocal: isLocal,
            alias: alias,
            type: identityTypeEnum,
            privateKeys: privateKeyData,
            votingPrivateKey: votingKey,
            ownerPrivateKey: ownerKey,
            payoutPrivateKey: payoutKey,
            dpnsName: dpnsName,
            mainDpnsName: mainDpnsName,
            publicKeys: publicKeyModels
        )
    }

    /// Create from IdentityModel
    @MainActor
    static func from(_ model: IdentityModel, network: String = "testnet") -> SwiftDashSDK.PersistentIdentity {
        // Store special keys in keychain first
        var votingKeyId: String? = nil
        var ownerKeyId: String? = nil
        var payoutKeyId: String? = nil

        if let votingKey = model.votingPrivateKey {
            votingKeyId = KeychainManager.shared.storeSpecialKey(votingKey, identityId: model.id, keyType: .voting)
        }
        if let ownerKey = model.ownerPrivateKey {
            ownerKeyId = KeychainManager.shared.storeSpecialKey(ownerKey, identityId: model.id, keyType: .owner)
        }
        if let payoutKey = model.payoutPrivateKey {
            payoutKeyId = KeychainManager.shared.storeSpecialKey(payoutKey, identityId: model.id, keyType: .payout)
        }

        let persistent = SwiftDashSDK.PersistentIdentity(
            identityId: model.id,
            balance: Int64(model.balance),
            revision: 0,
            isLocal: model.isLocal,
            alias: model.alias,
            dpnsName: model.dpnsName,
            mainDpnsName: model.mainDpnsName,
            identityType: model.type,
            votingPrivateKeyIdentifier: votingKeyId,
            ownerPrivateKeyIdentifier: ownerKeyId,
            payoutPrivateKeyIdentifier: payoutKeyId,
            network: network
        )

        // Add public keys
        for publicKey in model.publicKeys {
            if let persistentKey = SwiftDashSDK.PersistentPublicKey.from(publicKey, identityId: model.idString) {
                persistent.addPublicKey(persistentKey)
            }
        }

        // Handle private keys - match them to their corresponding public keys using cryptographic validation
        for privateKeyData in model.privateKeys {
            if let matchingPublicKey = KeyValidation.matchPrivateKeyToPublicKeys(
                privateKeyData: privateKeyData,
                publicKeys: model.publicKeys,
                isTestnet: network == "testnet"
            ) {
                if let persistentKey = persistent.publicKeys.first(where: { $0.keyId == matchingPublicKey.id }) {
                    if let keychainId = KeychainManager.shared.storePrivateKey(privateKeyData, identityId: model.id, keyIndex: persistentKey.keyId) {
                        persistentKey.privateKeyKeychainIdentifier = keychainId
                    }
                }
            }
        }

        return persistent
    }

    /// Create from DPPIdentity
    static func from(_ dppIdentity: DPPIdentity, alias: String? = nil, type: SwiftDashSDK.IdentityType = .user, network: String = "testnet") -> SwiftDashSDK.PersistentIdentity {
        let persistent = SwiftDashSDK.PersistentIdentity(
            identityId: dppIdentity.id,
            balance: Int64(dppIdentity.balance),
            revision: Int64(dppIdentity.revision),
            isLocal: false,
            alias: alias,
            identityType: type,
            network: network
        )

        // Add public keys
        for (_, publicKey) in dppIdentity.publicKeys {
            if let persistentKey = SwiftDashSDK.PersistentPublicKey.from(publicKey, identityId: dppIdentity.idString) {
                persistent.addPublicKey(persistentKey)
            }
        }

        return persistent
    }
}
