import Foundation

public struct ContractModel: Identifiable, Hashable {
    /// Get the owner ID as a hex string
    public var ownerIdString: String {
        ownerId.toHexString()
    }

    public static func == (lhs: ContractModel, rhs: ContractModel) -> Bool {
        lhs.id == rhs.id
    }

    public func hash(into hasher: inout Hasher) {
        hasher.combine(id)
    }
    public let id: String
    public let name: String
    public let version: Int
    public let ownerId: Data
    public let documentTypes: [String]
    public let schema: [String: Any]

    // DPP-related properties
    public let dppDataContract: DPPDataContract?
    public let tokens: [DPPTokenConfiguration]
    public let keywords: [String]
    public let description: String?

    public init(id: String, name: String, version: Int, ownerId: Data, documentTypes: [String], schema: [String: Any], dppDataContract: DPPDataContract? = nil, tokens: [DPPTokenConfiguration] = [], keywords: [String] = [], description: String? = nil) {
        self.id = id
        self.name = name
        self.version = version
        self.ownerId = ownerId
        self.documentTypes = documentTypes
        self.schema = schema
        self.dppDataContract = dppDataContract
        self.tokens = tokens
        self.keywords = keywords
        self.description = description
    }
    
    /// Create from DPP Data Contract
    public init(from dppContract: DPPDataContract, name: String) {
        self.id = dppContract.idString
        self.name = name
        self.version = Int(dppContract.version)
        self.ownerId = dppContract.ownerId
        self.documentTypes = Array(dppContract.documentTypes.keys)
        
        // Convert document types to simple schema representation
        var simpleSchema: [String: Any] = [:]
        for (docType, documentType) in dppContract.documentTypes {
            var docSchema: [String: Any] = [:]
            docSchema["type"] = "object"
            docSchema["properties"] = documentType.properties.mapValues { prop in
                return ["type": prop.type.rawValue]
            }
            simpleSchema[docType] = docSchema
        }
        self.schema = simpleSchema
        
        self.dppDataContract = dppContract
        self.tokens = Array(dppContract.tokens.values)
        self.keywords = dppContract.keywords
        self.description = dppContract.description
    }
    
    public var formattedSchema: String {
        guard let jsonData = try? JSONSerialization.data(withJSONObject: schema, options: .prettyPrinted),
              let jsonString = String(data: jsonData, encoding: .utf8) else {
            return "Invalid schema"
        }
        return jsonString
    }
}