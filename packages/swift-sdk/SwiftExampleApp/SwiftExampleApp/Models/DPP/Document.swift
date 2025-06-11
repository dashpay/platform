import Foundation

// MARK: - Document Models based on DPP

/// Main Document structure
struct DPPDocument: Identifiable, Codable, Equatable {
    let id: Identifier
    let ownerId: Identifier
    let properties: [String: PlatformValue]
    let revision: Revision?
    let createdAt: TimestampMillis?
    let updatedAt: TimestampMillis?
    let transferredAt: TimestampMillis?
    let createdAtBlockHeight: BlockHeight?
    let updatedAtBlockHeight: BlockHeight?
    let transferredAtBlockHeight: BlockHeight?
    let createdAtCoreBlockHeight: CoreBlockHeight?
    let updatedAtCoreBlockHeight: CoreBlockHeight?
    let transferredAtCoreBlockHeight: CoreBlockHeight?
    
    /// Get the document ID as a string
    var idString: String {
        id.toBase58String()
    }
    
    /// Get the owner ID as a string
    var ownerIdString: String {
        ownerId.toBase58String()
    }
    
    /// Get created date
    var createdDate: Date? {
        guard let createdAt = createdAt else { return nil }
        return Date(timeIntervalSince1970: Double(createdAt) / 1000)
    }
    
    /// Get updated date
    var updatedDate: Date? {
        guard let updatedAt = updatedAt else { return nil }
        return Date(timeIntervalSince1970: Double(updatedAt) / 1000)
    }
    
    /// Get transferred date
    var transferredDate: Date? {
        guard let transferredAt = transferredAt else { return nil }
        return Date(timeIntervalSince1970: Double(transferredAt) / 1000)
    }
}

// MARK: - Extended Document

/// Extended document that includes data contract and metadata
struct ExtendedDocument: Identifiable, Codable, Equatable {
    let documentTypeName: String
    let dataContractId: Identifier
    let document: DPPDocument
    let dataContract: DPPDataContract
    let metadata: DocumentMetadata?
    let entropy: Bytes32
    let tokenPaymentInfo: TokenPaymentInfo?
    
    /// Convenience accessor for document ID
    var id: Identifier {
        document.id
    }
    
    /// Get the data contract ID as a string
    var dataContractIdString: String {
        dataContractId.toBase58String()
    }
}

// MARK: - Document Metadata

struct DocumentMetadata: Codable, Equatable {
    let blockHeight: BlockHeight
    let coreBlockHeight: CoreBlockHeight
    let timeMs: TimestampMillis
    let protocolVersion: UInt32
}

// MARK: - Token Payment Info

struct TokenPaymentInfo: Codable, Equatable {
    let tokenId: Identifier
    let amount: UInt64
    
    var tokenIdString: String {
        tokenId.toBase58String()
    }
}

// MARK: - Document Patch

/// Represents a partial document update
struct DocumentPatch: Codable, Equatable {
    let id: Identifier
    let properties: [String: PlatformValue]
    let revision: Revision?
    let updatedAt: TimestampMillis?
    
    /// Get the document ID as a string
    var idString: String {
        id.toBase58String()
    }
}

// MARK: - Document Property Names

struct DocumentPropertyNames {
    static let featureVersion = "$version"
    static let id = "$id"
    static let dataContractId = "$dataContractId"
    static let revision = "$revision"
    static let ownerId = "$ownerId"
    static let price = "$price"
    static let createdAt = "$createdAt"
    static let updatedAt = "$updatedAt"
    static let transferredAt = "$transferredAt"
    static let createdAtBlockHeight = "$createdAtBlockHeight"
    static let updatedAtBlockHeight = "$updatedAtBlockHeight"
    static let transferredAtBlockHeight = "$transferredAtBlockHeight"
    static let createdAtCoreBlockHeight = "$createdAtCoreBlockHeight"
    static let updatedAtCoreBlockHeight = "$updatedAtCoreBlockHeight"
    static let transferredAtCoreBlockHeight = "$transferredAtCoreBlockHeight"
    
    static let identifierFields = [id, ownerId, dataContractId]
    static let timestampFields = [createdAt, updatedAt, transferredAt]
    static let blockHeightFields = [
        createdAtBlockHeight, updatedAtBlockHeight, transferredAtBlockHeight,
        createdAtCoreBlockHeight, updatedAtCoreBlockHeight, transferredAtCoreBlockHeight
    ]
}

// MARK: - Document Factory

extension DPPDocument {
    /// Create a new document
    static func create(
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
    
    /// Create from our simplified DocumentModel
    init(from model: DocumentModel) {
        // model.id is a string, convert it to Data
        self.id = Data.identifier(fromHex: model.id) ?? Data(repeating: 0, count: 32)
        // model.ownerId is already Data
        self.ownerId = model.ownerId
        
        // Convert properties - in a real implementation, this would properly convert types
        var platformProperties: [String: PlatformValue] = [:]
        for (key, value) in model.data {
            if let stringValue = value as? String {
                platformProperties[key] = .string(stringValue)
            } else if let intValue = value as? Int {
                platformProperties[key] = .integer(Int64(intValue))
            } else if let boolValue = value as? Bool {
                platformProperties[key] = .bool(boolValue)
            }
            // Add more type conversions as needed
        }
        self.properties = platformProperties
        
        self.revision = 0
        self.createdAt = model.createdAt.map { TimestampMillis($0.timeIntervalSince1970 * 1000) }
        self.updatedAt = model.updatedAt.map { TimestampMillis($0.timeIntervalSince1970 * 1000) }
        self.transferredAt = nil
        self.createdAtBlockHeight = nil
        self.updatedAtBlockHeight = nil
        self.transferredAtBlockHeight = nil
        self.createdAtCoreBlockHeight = nil
        self.updatedAtCoreBlockHeight = nil
        self.transferredAtCoreBlockHeight = nil
    }
}

// MARK: - Helper Extensions

extension Data {
    /// Pad or truncate data to specified length
    func paddedToLength(_ length: Int) -> Data {
        if self.count >= length {
            return self.prefix(length)
        } else {
            var padded = self
            padded.append(Data(repeating: 0, count: length - self.count))
            return padded
        }
    }
}