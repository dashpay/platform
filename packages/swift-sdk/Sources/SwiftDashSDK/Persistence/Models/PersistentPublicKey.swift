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

    // MARK: - Usage limits (protocol version 14)
    /// The credits this key may take from the identity over its whole
    /// lifetime, or `nil` for a key registered without a budget. Schema-stable
    /// signed carrier for the protocol's unsigned `Credits`: read it through
    /// `totalBudgetCredits` rather than the column, the same way
    /// `PersistentTokenBalance.balance` carries an unsigned balance.
    /// Additive optional column => SwiftData lightweight migration.
    public var totalBudget: Int64?

    /// The block time in milliseconds from which this key can no longer sign,
    /// or `nil` for a key registered without an expiry. Signed carrier for an
    /// unsigned protocol value, like `totalBudget`; read it through
    /// `expiresAtMillis`. Additive optional column => lightweight migration.
    public var expiresAt: Int64?

    // MARK: - Key Data
    public var publicKeyData: Data

    // MARK: - Contract Bounds
    /// JSON-encoded `[base64(boundId)]`, a legacy storage shape
    /// that only retains the bound id, never the document-type
    /// name. New code paths still write here for the id portion;
    /// `contractBoundsDocumentTypeName` carries the doc-type so
    /// the `SingleContractDocumentType` variant round-trips
    /// faithfully, and `contractBoundsKind` says which variant the
    /// id belongs to (for kind 3 it is a contract GROUP id, not a
    /// contract id). Keeping the field shape lets old SwiftData
    /// stores that predate the doc-type column continue to load
    /// without migration (the doc-type column is just `nil`).
    public var contractBoundsData: Data?

    /// When set, the key's bounds are
    /// `.singleContractDocumentType(id: contractBoundsData[0],
    /// documentTypeName: contractBoundsDocumentTypeName)`. When
    /// `nil`, the key is unbounded, bounded to a whole contract via
    /// `.singleContract(id:)`, or bounded to a contract group via
    /// `.contractGroup(id:)`; `contractBoundsKind` tells those three
    /// apart. Optional so old stores load cleanly.
    public var contractBoundsDocumentTypeName: String?

    /// The FFI `contract_bounds_kind` discriminant this row was
    /// persisted with: 0 none, 1 `SingleContract`,
    /// 2 `SingleContractDocumentType`, 3 `ContractGroup`. Stored
    /// because the two columns above cannot tell a group bound apart
    /// from a whole-contract one (both carry an id and no doc-type
    /// name), so inferring the variant dropped a group bound on every
    /// restart. `nil` on rows written before this column existed;
    /// those fall back to the legacy inference, which never yields 3
    /// because no writer could produce a group bound back then. See
    /// `effectiveContractBoundsKind`. Additive optional column, so
    /// SwiftData's lightweight migration backfills `NULL` when migrating
    /// the accepted V1 baseline to V2.
    public var contractBoundsKind: Int?

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
        totalBudget: Int64? = nil,
        expiresAt: Int64? = nil,
        contractBoundsKind: Int? = nil,
        identityId: String
    ) {
        self.keyId = keyId
        self.purpose = String(purpose.rawValue)
        self.securityLevel = String(securityLevel.rawValue)
        self.keyType = String(keyType.rawValue)
        self.publicKeyData = publicKeyData
        self.readOnly = readOnly
        self.disabledAt = disabledAt
        self.totalBudget = totalBudget
        self.expiresAt = expiresAt
        if let contractBounds = contractBounds {
            self.contractBoundsData = try? JSONSerialization.data(withJSONObject: contractBounds.map { $0.base64EncodedString() })
        } else {
            self.contractBoundsData = nil
        }
        self.contractBoundsDocumentTypeName = contractBoundsDocumentTypeName
        self.contractBoundsKind = contractBoundsKind
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
            // round-trip must write `contractBoundsDocumentTypeName` AND
            // `contractBoundsKind` (2) explicitly after this setter: the
            // stored kind wins over inference on restore, so a doc-type
            // written without kind 2 comes back as `.singleContract`. Or
            // go through
            // `PersistentPublicKey.from(IdentityPublicKey, identityId:)`,
            // which sets every column atomically.
            //
            // The kind column resets with the ids for the same reason: a
            // stale 2 or 3 alongside freshly written ids would restore a
            // variant those ids never had. A bare id is a `.singleContract`
            // bound (1); no id is unbounded (0).
            contractBoundsDocumentTypeName = nil
            contractBoundsKind = newValue == nil ? 0 : 1
            if let newValue = newValue {
                contractBoundsData = try? JSONSerialization.data(withJSONObject: newValue.map { $0.base64EncodedString() })
            } else {
                contractBoundsData = nil
            }
        }
    }

    /// The persisted `contractBoundsKind`, or the pre-column
    /// inference for a legacy row: a doc-type name means
    /// `SingleContractDocumentType` (2), a bare id means
    /// `SingleContract` (1), neither means unbounded (0). Callers
    /// still validate the id length themselves; a kind of 1, 2 or 3
    /// with an unusable id restores as unbounded.
    public var effectiveContractBoundsKind: Int {
        if let contractBoundsKind = contractBoundsKind {
            return contractBoundsKind
        }
        guard contractBoundsData != nil else { return 0 }
        if let name = contractBoundsDocumentTypeName, !name.isEmpty { return 2 }
        return 1
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

    /// The key's lifetime budget in credits, read through the signed
    /// `totalBudget` column's raw bits. Setter writes through.
    public var totalBudgetCredits: UInt64? {
        get { totalBudget.map { UInt64(bitPattern: $0) } }
        set { totalBudget = newValue.map { Int64(bitPattern: $0) } }
    }

    /// The key's expiry as block time in milliseconds, read through the
    /// signed `expiresAt` column's raw bits. Setter writes through.
    public var expiresAtMillis: UInt64? {
        get { expiresAt.map { UInt64(bitPattern: $0) } }
        set { expiresAt = newValue.map { Int64(bitPattern: $0) } }
    }

    /// Whether the key carries either usage limit, which is what makes it
    /// a version 1 key on the wire.
    public var hasLimits: Bool {
        totalBudget != nil || expiresAt != nil
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
    /// Reconstructs the `ContractBounds` variant from the persisted
    /// columns: on kind 2 we hand back `.singleContractDocumentType`
    /// so the document-type qualifier survives a SwiftData round-trip
    /// (without this, the DashPay encryption/decryption keys'
    /// `contactRequest` scope would silently weaken to a whole-
    /// contract `.singleContract` bound), and on kind 1
    /// `.singleContract`. Kind 3 and kind 0 both report no bounds.
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
        // A `ContractGroup` bound (kind 3) has no variant on the
        // DPP-layer `ContractBounds`, and its id is a group id, not a
        // contract id, so projecting it as `.singleContract` would
        // claim a bound the key does not have. Report no bounds
        // instead; the row keeps the group bound for the FFI restore
        // path, which does model it. Same for a kind this build does
        // not know.
        let bounds: ContractBounds?
        let boundsKind = effectiveContractBoundsKind
        if (1...2).contains(boundsKind), let id = contractBounds?.first, id.count == 32 {
            if boundsKind == 2,
                let docTypeName = contractBoundsDocumentTypeName, !docTypeName.isEmpty {
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
            disabledAt: disabledAt.map { TimestampMillis($0) },
            totalBudget: totalBudgetCredits,
            expiresAt: expiresAtMillis
        )
    }

    /// Create from IdentityPublicKey, preserving both
    /// `ContractBounds` variants (id + optional document-type name).
    public static func from(_ publicKey: IdentityPublicKey, identityId: String) -> PersistentPublicKey? {
        let boundsIds: [Data]?
        let docTypeName: String?
        // The DPP-layer enum has no `ContractGroup` variant, so this
        // path only ever writes kinds 0, 1 and 2. Group-bound keys
        // reach the store through the FFI persist path.
        let kind: Int
        switch publicKey.contractBounds {
        case .singleContract(let id):
            boundsIds = [id]
            docTypeName = nil
            kind = 1
        case .singleContractDocumentType(let id, let name):
            boundsIds = [id]
            docTypeName = name
            kind = 2
        case .none:
            boundsIds = nil
            docTypeName = nil
            kind = 0
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
            totalBudget: publicKey.totalBudget.map { Int64(bitPattern: $0) },
            expiresAt: publicKey.expiresAt.map { Int64(bitPattern: $0) },
            contractBoundsKind: kind,
            identityId: identityId
        )
    }
}
