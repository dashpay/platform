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
        self.purpose = String(purpose.rawValue)
        self.securityLevel = String(securityLevel.rawValue)
        self.keyType = String(keyType.rawValue)
        self.publicKeyData = publicKeyData
        self.readOnly = readOnly
        self.disabledAt = disabledAt
        if let contractBounds = contractBounds {
            self.contractBoundsData = try? JSONSerialization.data(withJSONObject: contractBounds.map { $0.base64EncodedString() })
        } else {
            self.contractBoundsData = nil
        }
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
            if let newValue = newValue {
                contractBoundsData = try? JSONSerialization.data(withJSONObject: newValue.map { $0.base64EncodedString() })
            } else {
                contractBoundsData = nil
            }
        }
    }
    
    var purposeEnum: KeyPurpose? {
        guard let purposeInt = UInt8(purpose) else { return nil }
        return KeyPurpose(rawValue: purposeInt)
    }
    
    var securityLevelEnum: SecurityLevel? {
        guard let levelInt = UInt8(securityLevel) else { return nil }
        return SecurityLevel(rawValue: levelInt)
    }
    
    var keyTypeEnum: KeyType? {
        guard let typeInt = UInt8(keyType) else { return nil }
        return KeyType(rawValue: typeInt)
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
            id: KeyID(keyId),
            purpose: purpose,
            securityLevel: securityLevel,
            contractBounds: contractBounds?.first.map { .singleContract(id: $0) },
            keyType: keyType,
            readOnly: readOnly,
            data: publicKeyData,
            disabledAt: disabledAt.map { TimestampMillis($0) }
        )
    }
    
    /// Create from IdentityPublicKey
    static func from(_ publicKey: IdentityPublicKey, identityId: String) -> PersistentPublicKey? {
        return PersistentPublicKey(
            keyId: Int32(publicKey.id),
            purpose: publicKey.purpose,
            securityLevel: publicKey.securityLevel,
            keyType: publicKey.keyType,
            publicKeyData: publicKey.data,
            readOnly: publicKey.readOnly,
            disabledAt: publicKey.disabledAt.map { Int64($0) },
            contractBounds: publicKey.contractBounds != nil ? [publicKey.contractBounds!.contractId] : nil,
            identityId: identityId
        )
    }
}