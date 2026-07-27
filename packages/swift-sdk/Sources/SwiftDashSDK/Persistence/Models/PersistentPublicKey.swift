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
    /// JSON-encoded `[base64(contractId)]` — legacy storage shape
    /// that only retains the contract id, never the document-type
    /// name. New code paths still write here for the id portion;
    /// `contractBoundsDocumentTypeName` carries the doc-type so
    /// the `SingleContractDocumentType` variant round-trips
    /// faithfully. Keeping the field shape lets old SwiftData
    /// stores that predate the doc-type column continue to load
    /// without migration (the doc-type column is just `nil`).
    public var contractBoundsData: Data?

    /// When set, the key's bounds are
    /// `.singleContractDocumentType(id: contractBoundsData[0],
    /// documentTypeName: contractBoundsDocumentTypeName)`. When
    /// `nil`, the key is either unbounded (when `contractBoundsData`
    /// is also nil) or bounded to a whole contract via
    /// `.singleContract(id:)`. Optional so old stores load cleanly.
    public var contractBoundsDocumentTypeName: String?

    // MARK: - Private Key Reference (optional)
    public var privateKeyKeychainIdentifier: String?

    // MARK: - Derivation breadcrumb (derive-sign-destroy)
    /// 32-byte wallet id that owns this identity key, denormalized from the
    /// discovery breadcrumb. Paired with `identityDerivationPath`, it lets the
    /// signer derive this key on demand from the Keychain-held seed instead of
    /// reading a stored scalar. `nil` for rows persisted before this column
    /// existed and for keys with no wallet association; such rows fall back to
    /// the stored scalar until the backfill populates them. Additive optional
    /// column => SwiftData lightweight migration.
    public var walletId: Data?

    /// Full DIP-9 identity-authentication path
    /// `m/9'/coin'/5'/0'/ECDSA'/identityIndex'/keyIndex'` the signer feeds to
    /// the mnemonic resolver to derive this key's private scalar at sign time.
    /// The authoritative breadcrumb; `nil` until written on persist or
    /// backfilled from the key's Keychain metadata.
    public var identityDerivationPath: String?

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
        contractBoundsDocumentTypeName: String? = nil,
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
        self.contractBoundsDocumentTypeName = contractBoundsDocumentTypeName
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
            // Always clear the doc-type column when the contract-
            // bounds ids change through this setter. The
            // `documentTypeName` is paired with a SPECIFIC id, so
            // mutating ids without explicitly carrying the doc-
            // type would leave the columns inconsistent and make
            // `toIdentityPublicKey()` reconstruct a stale variant.
            // Callers that want the full `.singleContractDocumentType`
            // round-trip should write `contractBoundsDocumentTypeName`
            // explicitly after this setter, or go through
            // `PersistentPublicKey.from(IdentityPublicKey, identityId:)`
            // which sets both columns atomically.
            contractBoundsDocumentTypeName = nil
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
    /// Convert to IdentityPublicKey.
    ///
    /// Reconstructs the `ContractBounds` variant from the two
    /// persisted columns: when `contractBoundsDocumentTypeName` is
    /// set, we hand back `.singleContractDocumentType` so the
    /// document-type qualifier survives a SwiftData round-trip
    /// (without this, the DashPay encryption/decryption keys'
    /// `contactRequest` scope would silently weaken to a whole-
    /// contract `.singleContract` bound). When only the legacy
    /// `contractBoundsData` is set, fall back to `.singleContract`.
    public func toIdentityPublicKey() -> IdentityPublicKey? {
        guard let purpose = purposeEnum,
              let securityLevel = securityLevelEnum,
              let keyType = keyTypeEnum else {
            return nil
        }

        // Validate the persisted id is the canonical 32-byte
        // contract identifier before constructing a
        // `ContractBounds` variant. Downstream FFI marshalling
        // (`ManagedPlatformWallet.pinContractBounds`) hard-asserts
        // a 32-byte payload, so a corrupt / short / over-long
        // row would crash on the NEXT call instead of being
        // rejected here. Drop the bounds projection on length
        // mismatch — the rest of the key is still recoverable.
        let bounds: ContractBounds?
        if let id = contractBounds?.first, id.count == 32 {
            if let docTypeName = contractBoundsDocumentTypeName, !docTypeName.isEmpty {
                bounds = .singleContractDocumentType(id: id, documentTypeName: docTypeName)
            } else {
                bounds = .singleContract(id: id)
            }
        } else {
            bounds = nil
        }

        return IdentityPublicKey(
            id: KeyID(keyId),
            purpose: purpose,
            securityLevel: securityLevel,
            contractBounds: bounds,
            keyType: keyType,
            readOnly: readOnly,
            data: publicKeyData,
            disabledAt: disabledAt.map { TimestampMillis($0) }
        )
    }

    /// Create from IdentityPublicKey, preserving both
    /// `ContractBounds` variants (id + optional document-type name).
    public static func from(_ publicKey: IdentityPublicKey, identityId: String) -> PersistentPublicKey? {
        let boundsIds: [Data]?
        let docTypeName: String?
        switch publicKey.contractBounds {
        case .singleContract(let id):
            boundsIds = [id]
            docTypeName = nil
        case .singleContractDocumentType(let id, let name):
            boundsIds = [id]
            docTypeName = name
        case .none:
            boundsIds = nil
            docTypeName = nil
        }
        return PersistentPublicKey(
            keyId: Int32(publicKey.id),
            purpose: publicKey.purpose,
            securityLevel: publicKey.securityLevel,
            keyType: publicKey.keyType,
            publicKeyData: publicKey.data,
            readOnly: publicKey.readOnly,
            disabledAt: publicKey.disabledAt.map { Int64($0) },
            contractBounds: boundsIds,
            contractBoundsDocumentTypeName: docTypeName,
            identityId: identityId
        )
    }
}
