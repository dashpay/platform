import Foundation

// MARK: - Document Models based on DPP

/// Main Document structure representing a Dash Platform document
public struct DPPDocument: Identifiable, Codable, Equatable, Sendable {
    public let id: Identifier
    public let ownerId: Identifier
    public let properties: [String: PlatformValue]
    public let revision: Revision?
    public let createdAt: TimestampMillis?
    public let updatedAt: TimestampMillis?
    public let transferredAt: TimestampMillis?
    public let createdAtBlockHeight: BlockHeight?
    public let updatedAtBlockHeight: BlockHeight?
    public let transferredAtBlockHeight: BlockHeight?
    public let createdAtCoreBlockHeight: CoreBlockHeight?
    public let updatedAtCoreBlockHeight: CoreBlockHeight?
    public let transferredAtCoreBlockHeight: CoreBlockHeight?

    /// Get the document ID as a base58 string
    public var idString: String {
        id.toBase58String()
    }

    /// Get the owner ID as a base58 string
    public var ownerIdString: String {
        ownerId.toBase58String()
    }

    public init(
        id: Identifier,
        ownerId: Identifier,
        properties: [String: PlatformValue],
        revision: Revision? = nil,
        createdAt: TimestampMillis? = nil,
        updatedAt: TimestampMillis? = nil,
        transferredAt: TimestampMillis? = nil,
        createdAtBlockHeight: BlockHeight? = nil,
        updatedAtBlockHeight: BlockHeight? = nil,
        transferredAtBlockHeight: BlockHeight? = nil,
        createdAtCoreBlockHeight: CoreBlockHeight? = nil,
        updatedAtCoreBlockHeight: CoreBlockHeight? = nil,
        transferredAtCoreBlockHeight: CoreBlockHeight? = nil
    ) {
        self.id = id
        self.ownerId = ownerId
        self.properties = properties
        self.revision = revision
        self.createdAt = createdAt
        self.updatedAt = updatedAt
        self.transferredAt = transferredAt
        self.createdAtBlockHeight = createdAtBlockHeight
        self.updatedAtBlockHeight = updatedAtBlockHeight
        self.transferredAtBlockHeight = transferredAtBlockHeight
        self.createdAtCoreBlockHeight = createdAtCoreBlockHeight
        self.updatedAtCoreBlockHeight = updatedAtCoreBlockHeight
        self.transferredAtCoreBlockHeight = transferredAtCoreBlockHeight
    }

    /// Get created date
    public var createdDate: Date? {
        guard let createdAt = createdAt else { return nil }
        return Date(timeIntervalSince1970: Double(createdAt) / 1000)
    }

    /// Get updated date
    public var updatedDate: Date? {
        guard let updatedAt = updatedAt else { return nil }
        return Date(timeIntervalSince1970: Double(updatedAt) / 1000)
    }

    /// Get transferred date
    public var transferredDate: Date? {
        guard let transferredAt = transferredAt else { return nil }
        return Date(timeIntervalSince1970: Double(transferredAt) / 1000)
    }
}

// MARK: - Extended Document

/// Extended document that includes data contract and metadata
public struct ExtendedDocument: Identifiable, Codable, Equatable, Sendable {
    public let documentTypeName: String
    public let dataContractId: Identifier
    public let document: DPPDocument
    public let dataContract: DPPDataContract
    public let metadata: DocumentMetadata?
    public let entropy: Bytes32
    public let tokenPaymentInfo: TokenPaymentInfo?

    /// Convenience accessor for document ID
    public var id: Identifier {
        document.id
    }

    /// Get the data contract ID as a base58 string
    public var dataContractIdString: String {
        dataContractId.toBase58String()
    }

    public init(
        documentTypeName: String,
        dataContractId: Identifier,
        document: DPPDocument,
        dataContract: DPPDataContract,
        metadata: DocumentMetadata?,
        entropy: Bytes32,
        tokenPaymentInfo: TokenPaymentInfo?
    ) {
        self.documentTypeName = documentTypeName
        self.dataContractId = dataContractId
        self.document = document
        self.dataContract = dataContract
        self.metadata = metadata
        self.entropy = entropy
        self.tokenPaymentInfo = tokenPaymentInfo
    }
}

// MARK: - Document Metadata

public struct DocumentMetadata: Codable, Equatable, Sendable {
    public let blockHeight: BlockHeight
    public let coreBlockHeight: CoreBlockHeight
    public let timeMs: TimestampMillis
    public let protocolVersion: UInt32

    public init(blockHeight: BlockHeight, coreBlockHeight: CoreBlockHeight, timeMs: TimestampMillis, protocolVersion: UInt32) {
        self.blockHeight = blockHeight
        self.coreBlockHeight = coreBlockHeight
        self.timeMs = timeMs
        self.protocolVersion = protocolVersion
    }
}

// MARK: - Token Payment Info

public struct TokenPaymentInfo: Codable, Equatable, Sendable {
    public let tokenId: Identifier
    public let amount: UInt64

    public var tokenIdString: String {
        tokenId.toBase58String()
    }

    public init(tokenId: Identifier, amount: UInt64) {
        self.tokenId = tokenId
        self.amount = amount
    }
}

// MARK: - Document Patch

/// Represents a partial document update
public struct DocumentPatch: Codable, Equatable, Sendable {
    public let id: Identifier
    public let properties: [String: PlatformValue]
    public let revision: Revision?
    public let updatedAt: TimestampMillis?

    /// Get the document ID as a base58 string
    public var idString: String {
        id.toBase58String()
    }

    public init(id: Identifier, properties: [String: PlatformValue], revision: Revision?, updatedAt: TimestampMillis?) {
        self.id = id
        self.properties = properties
        self.revision = revision
        self.updatedAt = updatedAt
    }
}

// MARK: - Document Property Names

public struct DocumentPropertyNames {
    public static let featureVersion = "$version"
    public static let id = "$id"
    public static let dataContractId = "$dataContractId"
    public static let revision = "$revision"
    public static let ownerId = "$ownerId"
    public static let price = "$price"
    public static let createdAt = "$createdAt"
    public static let updatedAt = "$updatedAt"
    public static let transferredAt = "$transferredAt"
    public static let createdAtBlockHeight = "$createdAtBlockHeight"
    public static let updatedAtBlockHeight = "$updatedAtBlockHeight"
    public static let transferredAtBlockHeight = "$transferredAtBlockHeight"
    public static let createdAtCoreBlockHeight = "$createdAtCoreBlockHeight"
    public static let updatedAtCoreBlockHeight = "$updatedAtCoreBlockHeight"
    public static let transferredAtCoreBlockHeight = "$transferredAtCoreBlockHeight"

    public static let identifierFields = [id, ownerId, dataContractId]
    public static let timestampFields = [createdAt, updatedAt, transferredAt]
    public static let blockHeightFields = [
        createdAtBlockHeight, updatedAtBlockHeight, transferredAtBlockHeight,
        createdAtCoreBlockHeight, updatedAtCoreBlockHeight, transferredAtCoreBlockHeight
    ]
}

// MARK: - Document Factory

extension DPPDocument {
    /// Create a new document with auto-generated ID
    public static func create(
        id: Identifier? = nil,
        ownerId: Identifier,
        properties: [String: PlatformValue] = [:]
    ) -> DPPDocument {
        let documentId = id ?? Data(UUID().uuidString.utf8).prefix(32).paddedToLength(32)

        return DPPDocument(
            id: documentId,
            ownerId: ownerId,
            properties: properties,
            revision: 0,
            createdAt: TimestampMillis(Date().timeIntervalSince1970 * 1000),
            updatedAt: nil,
            transferredAt: nil,
            createdAtBlockHeight: nil,
            updatedAtBlockHeight: nil,
            transferredAtBlockHeight: nil,
            createdAtCoreBlockHeight: nil,
            updatedAtCoreBlockHeight: nil,
            transferredAtCoreBlockHeight: nil
        )
    }
}
