import Foundation
import SwiftData

/// SwiftData model for persisting documents
@Model
public final class PersistentDocument {
    /// Index `networkRaw` to keep per-network document scans
    /// index-served. The static `predicate(contractId:network:)` helper
    /// and every UI list view filter by the active network.
    #Index<PersistentDocument>([\.networkRaw])

    // Primary key
    @Attribute(.unique) public var documentId: String

    // Core document properties
    public var documentType: String
    public var revision: Int32
    public var data: Data

    // References (stored as strings for queries)
    public var contractId: String
    public var ownerId: String

    // Binary data for efficient operations
    public var contractIdData: Data
    public var ownerIdData: Data

    // Timestamps
    public var createdAt: Date
    public var updatedAt: Date
    public var transferredAt: Date?

    // Block heights
    public var createdAtBlockHeight: Int64?
    public var updatedAtBlockHeight: Int64?
    public var transferredAtBlockHeight: Int64?

    // Core block heights
    public var createdAtCoreBlockHeight: Int64?
    public var updatedAtCoreBlockHeight: Int64?
    public var transferredAtCoreBlockHeight: Int64?

    // Network
    /// Stored as the `Network.rawValue` `UInt32` so SwiftData
    /// `#Predicate` expressions can evaluate it directly. See
    /// `PersistentIdentity.networkRaw` for the full rationale.
    public var networkRaw: UInt32

    /// Type-safe accessor over `networkRaw`. Setter writes through.
    public var network: Network {
        get { Network(rawValue: networkRaw) ?? .testnet }
        set { networkRaw = newValue.rawValue }
    }

    // Deletion flag
    public var isDeleted: Bool = false

    // Local tracking
    public var localCreatedAt: Date
    public var localUpdatedAt: Date

    // Relationships
    public var documentType_relation: PersistentDocumentType?
    public var dataContract: PersistentDataContract?

    // Optional reference to local identity (if owner is local)
    public var ownerIdentity: PersistentIdentity?

    // Computed properties
    public var id: Data {
        Data.identifier(fromBase58: documentId) ?? Data()
    }

    public var idBase58: String {
        documentId
    }

    public var ownerIdBase58: String {
        ownerId
    }

    public var contractIdBase58: String {
        contractId
    }

    public var properties: [String: Any]? {
        try? JSONSerialization.jsonObject(with: data, options: []) as? [String: Any]
    }

    public var displayTitle: String {
        guard let props = properties else { return "Document" }

        if let title = props["title"] as? String { return title }
        if let name = props["name"] as? String { return name }
        if let label = props["label"] as? String { return label }
        if let normalizedLabel = props["normalizedLabel"] as? String { return normalizedLabel }

        return documentType
    }

    public var summary: String {
        var parts: [String] = []

        parts.append("Type: \(documentType)")
        parts.append("Rev: \(revision)")

        // Pin to Gregorian so the `createdAt` year stays CE even
        // when the device is configured for a non-Gregorian
        // calendar (e.g. Thai region → Buddhist era). The SDK
        // doesn't depend on the app's `AppDate` helper, so we
        // configure the formatter inline.
        let formatter = DateFormatter()
        formatter.calendar = Calendar(identifier: .gregorian)
        formatter.dateStyle = .short
        parts.append("Created: \(formatter.string(from: createdAt))")

        return parts.joined(separator: " • ")
    }

    public init(
        documentId: String,
        documentType: String,
        revision: Int32,
        data: Data,
        contractId: String,
        ownerId: String,
        network: Network
    ) {
        self.documentId = documentId
        self.documentType = documentType
        self.revision = revision
        self.data = data
        self.contractId = contractId
        self.ownerId = ownerId
        self.contractIdData = Data.identifier(fromBase58: contractId) ?? Data()
        self.ownerIdData = Data.identifier(fromBase58: ownerId) ?? Data()
        self.networkRaw = network.rawValue
        self.createdAt = Date()
        self.updatedAt = Date()
        self.localCreatedAt = Date()
        self.localUpdatedAt = Date()
    }

    // MARK: - Methods
    public func updateProperties(_ newData: Data) {
        self.data = newData
        self.updatedAt = Date()
    }

    public func updateRevision(_ newRevision: Int64) {
        self.revision = Int32(newRevision)
        self.updatedAt = Date()
    }

    public func markAsDeleted() {
        self.isDeleted = true
        self.updatedAt = Date()
    }

    // MARK: - Static Methods
    public static func predicate(documentId: String) -> Predicate<PersistentDocument> {
        #Predicate<PersistentDocument> { doc in
            doc.documentId == documentId && doc.isDeleted == false
        }
    }

    public static func predicate(contractId: String, network: Network) -> Predicate<PersistentDocument> {
        // See `PersistentIdentity.predicate(network:)` — Foundation's
        // predicate engine can't capture `Network`, so we filter on
        // the UInt32-backed `networkRaw` shadow field.
        let target = network.rawValue
        return #Predicate<PersistentDocument> { doc in
            doc.contractId == contractId && doc.networkRaw == target && doc.isDeleted == false
        }
    }

    public static func predicate(ownerId: Data) -> Predicate<PersistentDocument> {
        let ownerIdString = ownerId.toBase58String()
        return #Predicate<PersistentDocument> { doc in
            doc.ownerId == ownerIdString && doc.isDeleted == false
        }
    }

    // MARK: - Identity Linking

    /// Attach `ownerIdentity` to the persisted identity row matching
    /// `ownerIdData`, if one exists. Any persisted identity qualifies —
    /// the relationship records who owns the document, not whether this
    /// device can sign for them. (An `isLocal == true` clause used to
    /// gate this, but the stored flag was never written `true`, so the
    /// link never fired; the flag has since been removed.)
    public func linkToOwnerIdentityIfNeeded(in modelContext: ModelContext) {
        guard ownerIdentity == nil else { return }

        let ownerIdToMatch = self.ownerIdData
        let identityPredicate = #Predicate<PersistentIdentity> { identity in
            identity.identityId == ownerIdToMatch
        }

        let descriptor = FetchDescriptor<PersistentIdentity>(predicate: identityPredicate)

        do {
            if let ownerRow = try modelContext.fetch(descriptor).first {
                self.ownerIdentity = ownerRow
                self.localUpdatedAt = Date()
            }
        } catch {
            print("Failed to link document to owner identity: \(error)")
        }
    }
}

// The legacy `DocumentModel` value-type bridge has been removed.
// Callers construct `PersistentDocument` directly and read the JSON
// payload via `properties`.
