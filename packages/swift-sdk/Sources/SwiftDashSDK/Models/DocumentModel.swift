import Foundation

public struct DocumentModel: Identifiable {
    /// Get the owner ID as a hex string
    public var ownerIdString: String {
        ownerId.toHexString()
    }

    public let id: String
    public let contractId: String
    public let documentType: String
    public let ownerId: Data
    public let data: [String: Any]
    public let createdAt: Date?
    public let updatedAt: Date?

    // DPP-related properties
    public let dppDocument: DPPDocument?
    public let revision: Revision

    public init(id: String, contractId: String, documentType: String, ownerId: Data, data: [String: Any], createdAt: Date? = nil, updatedAt: Date? = nil, dppDocument: DPPDocument? = nil, revision: Revision = 0) {
        self.id = id
        self.contractId = contractId
        self.documentType = documentType
        self.ownerId = ownerId
        self.data = data
        self.createdAt = createdAt
        self.updatedAt = updatedAt
        self.dppDocument = dppDocument
        self.revision = revision
    }

    /// Create from DPP Document
    public init(from dppDocument: DPPDocument, contractId: String, documentType: String) {
        self.id = dppDocument.idString
        self.contractId = contractId
        self.documentType = documentType
        self.ownerId = dppDocument.ownerId

        // Convert PlatformValue properties to simple dictionary
        var simpleData: [String: Any] = [:]
        for (key, value) in dppDocument.properties {
            switch value {
            case .string(let str):
                simpleData[key] = str
            case .integer(let int):
                simpleData[key] = int
            case .bool(let bool):
                simpleData[key] = bool
            case .float(let double):
                simpleData[key] = double
            case .bytes(let data):
                simpleData[key] = data
            default:
                // Handle complex types as needed
                break
            }
        }
        self.data = simpleData

        self.createdAt = dppDocument.createdDate
        self.updatedAt = dppDocument.updatedDate
        self.dppDocument = dppDocument
        self.revision = dppDocument.revision ?? 0
    }

    public var formattedData: String {
        guard let jsonData = try? JSONSerialization.data(withJSONObject: data, options: .prettyPrinted),
              let jsonString = String(data: jsonData, encoding: .utf8) else {
            return "Invalid data"
        }
        return jsonString
    }
}
