import Foundation
import SwiftData

/// SwiftData model for persisting public key data
@Model
final class PersistentPublicKey {
    // MARK: - Core Properties
    var keyId: Int32
    var purpose: String
    var securityLevel: String
    var keyType: String
    var readOnly: Bool
    var disabledAt: Int64?
    
    // MARK: - Key Data
    var publicKeyData: Data
    
    // MARK: - Contract Bounds
    var contractBoundsData: Data?
    
    // MARK: - Metadata
    var identityId: String
    var createdAt: Date
    
    // MARK: - Initialization
    init(
        keyId: Int32,
        purpose: KeyPurpose,
        securityLevel: SecurityLevel,
        keyType: KeyType,
        publicKeyData: Data,
        readOnly: Bool = false,
        disabledAt: Int64? = nil,
        contractBounds: [Data]? = nil,
        identityId: String
    ) {
        self.keyId = keyId
        self.purpose = purpose.rawValue
        self.securityLevel = securityLevel.rawValue
        self.keyType = keyType.rawValue
        self.publicKeyData = publicKeyData
        self.readOnly = readOnly
        self.disabledAt = disabledAt
        self.contractBoundsData = contractBounds.map { try? JSONSerialization.data(withJSONObject: $0.map { $0.base64EncodedString() }) }
        self.identityId = identityId
        self.createdAt = Date()
    }
    
    // MARK: - Computed Properties
    var contractBounds: [Data]? {
        get {
            guard let data = contractBoundsData,
                  let json = try? JSONSerialization.jsonObject(with: data),
                  let strings = json as? [String] else {
                return nil
            }
            return strings.compactMap { Data(base64Encoded: $0) }
        }
        set {
            contractBoundsData = newValue.map { 
                try? JSONSerialization.data(withJSONObject: $0.map { $0.base64EncodedString() }) 
            }
        }
    }
    
    var purposeEnum: KeyPurpose? {
        KeyPurpose(rawValue: purpose)
    }
    
    var securityLevelEnum: SecurityLevel? {
        SecurityLevel(rawValue: securityLevel)
    }
    
    var keyTypeEnum: KeyType? {
        KeyType(rawValue: keyType)
    }
    
    var isDisabled: Bool {
        disabledAt != nil
    }
}

// MARK: - Conversion Extensions

extension PersistentPublicKey {
    /// Convert to IdentityPublicKey
    func toIdentityPublicKey() -> IdentityPublicKey? {
        guard let purpose = purposeEnum,
              let securityLevel = securityLevelEnum,
              let keyType = keyTypeEnum else {
            return nil
        }
        
        return IdentityPublicKey(
            id: keyId,
            purpose: purpose,
            securityLevel: securityLevel,
            keyType: keyType,
            data: publicKeyData,
            readOnly: readOnly,
            disabledAt: disabledAt.map { BlockHeight($0) },
            contractBounds: contractBounds?.map { ContractBounds(contractId: $0) }
        )
    }
    
    /// Create from IdentityPublicKey
    static func from(_ publicKey: IdentityPublicKey, identityId: String) -> PersistentPublicKey? {
        return PersistentPublicKey(
            keyId: publicKey.id,
            purpose: publicKey.purpose,
            securityLevel: publicKey.securityLevel,
            keyType: publicKey.keyType,
            publicKeyData: publicKey.data,
            readOnly: publicKey.readOnly,
            disabledAt: publicKey.disabledAt.map { Int64($0) },
            contractBounds: publicKey.contractBounds?.map { $0.contractId },
            identityId: identityId
        )
    }
}