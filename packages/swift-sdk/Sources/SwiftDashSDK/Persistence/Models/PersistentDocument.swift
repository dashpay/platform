import Foundation
import SwiftData

/// SwiftData model for persisting documents
@Model
public final class PersistentDocument {
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
    public var network: String

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

        let formatter = DateFormatter()
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
        network: String = "testnet"
    ) {
        self.documentId = documentId
        self.documentType = documentType
        self.revision = revision
        self.data = data
        self.contractId = contractId
        self.ownerId = ownerId
        self.contractIdData = Data.identifier(fromBase58: contractId) ?? Data()
        self.ownerIdData = Data.identifier(fromBase58: ownerId) ?? Data()
        self.network = network
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

    public static func predicate(contractId: String, network: String) -> Predicate<PersistentDocument> {
        #Predicate<PersistentDocument> { doc in
            doc.contractId == contractId && doc.network == network && doc.isDeleted == false
        }
    }

    public static func predicate(ownerId: Data) -> Predicate<PersistentDocument> {
        let ownerIdString = ownerId.toBase58String()
        return #Predicate<PersistentDocument> { doc in
            doc.ownerId == ownerIdString && doc.isDeleted == false
        }
    }

    // MARK: - Identity Linking
    public func linkToLocalIdentityIfNeeded(in modelContext: ModelContext) {
        guard ownerIdentity == nil else { return }

        let ownerIdToMatch = self.ownerIdData
        let identityPredicate = #Predicate<PersistentIdentity> { identity in
            identity.identityId == ownerIdToMatch && identity.isLocal == true
        }

        let descriptor = FetchDescriptor<PersistentIdentity>(predicate: identityPredicate)

        do {
            if let localIdentity = try modelContext.fetch(descriptor).first {
                self.ownerIdentity = localIdentity
                self.localUpdatedAt = Date()
            }
        } catch {
            print("Failed to link document to local identity: \(error)")
        }
    }
}

// MARK: - Conversion Methods

extension PersistentDocument {
    /// Create a PersistentDocument from a DocumentModel
    public static func from(_ document: DocumentModel) -> PersistentDocument {
        let dataToStore = (try? JSONSerialization.data(withJSONObject: document.data, options: [])) ?? Data()

        let persistent = PersistentDocument(
            documentId: document.id,
            documentType: document.documentType,
            revision: Int32(document.revision),
            data: dataToStore,
            contractId: document.contractId,
            ownerId: document.ownerId.toBase58String()
        )

        if let createdAt = document.createdAt {
            persistent.createdAt = createdAt
        }
        if let updatedAt = document.updatedAt {
            persistent.updatedAt = updatedAt
        }

        return persistent
    }

    /// Convert to a DocumentModel
    public func toDocumentModel() -> DocumentModel {
        let dataDict: [String: Any] = properties ?? [:]

        return DocumentModel(
            id: documentId,
            contractId: contractId,
            documentType: documentType,
            ownerId: ownerIdData,
            data: dataDict,
            createdAt: createdAt,
            updatedAt: updatedAt,
            dppDocument: nil,
            revision: Revision(revision)
        )
    }
}
