import Foundation
import SwiftData

/// SwiftData model for persisting public key data
@Model
public final class PersistentPublicKey {
    // MARK: - Core Properties
    public var keyId: Int32
    public var purpose: String
    public var securityLevel: String
    public var keyType: String
    public var readOnly: Bool
    public var disabledAt: Int64?

    // MARK: - Key Data
    public var publicKeyData: Data

    // MARK: - Contract Bounds
    public var contractBoundsData: Data?

    // MARK: - Private Key Reference (optional)
    public var privateKeyKeychainIdentifier: String?

    // MARK: - Metadata
    public var identityId: String
    public var createdAt: Date
    public var lastAccessed: Date?

    // MARK: - Relationships
    @Relationship(inverse: \PersistentIdentity.publicKeys)
    public var identity: PersistentIdentity?

    // MARK: - Initialization
    public init(
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
    public var contractBounds: [Data]? {
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

    public var purposeEnum: KeyPurpose? {
        guard let purposeInt = UInt8(purpose) else { return nil }
        return KeyPurpose(rawValue: purposeInt)
    }

    public var securityLevelEnum: SecurityLevel? {
        guard let levelInt = UInt8(securityLevel) else { return nil }
        return SecurityLevel(rawValue: levelInt)
    }

    public var keyTypeEnum: KeyType? {
        guard let typeInt = UInt8(keyType) else { return nil }
        return KeyType(rawValue: typeInt)
    }

    public var isDisabled: Bool {
        disabledAt != nil
    }

    /// Check if this public key has an associated private key identifier
    public var hasPrivateKeyIdentifier: Bool {
        privateKeyKeychainIdentifier != nil
    }
}

// MARK: - Conversion Extensions

extension PersistentPublicKey {
    /// Convert to IdentityPublicKey
    public func toIdentityPublicKey() -> IdentityPublicKey? {
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
    public static func from(_ publicKey: IdentityPublicKey, identityId: String) -> PersistentPublicKey? {
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
