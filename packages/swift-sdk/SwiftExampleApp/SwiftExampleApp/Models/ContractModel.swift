import Foundation

struct ContractModel: Identifiable, Hashable {
    /// Get the owner ID as a hex string
    var ownerIdString: String {
        ownerId.toHexString()
    }
    
    static func == (lhs: ContractModel, rhs: ContractModel) -> Bool {
        lhs.id == rhs.id
    }
    
    func hash(into hasher: inout Hasher) {
        hasher.combine(id)
    }
    let id: String
    let name: String
    let version: Int
    let ownerId: Data
    let documentTypes: [String]
    let schema: [String: Any]
    
    // DPP-related properties
    let dppDataContract: DPPDataContract?
    let tokens: [TokenConfiguration]
    let keywords: [String]
    let description: String?
    
    init(id: String, name: String, version: Int, ownerId: Data, documentTypes: [String], schema: [String: Any], dppDataContract: DPPDataContract? = nil, tokens: [TokenConfiguration] = [], keywords: [String] = [], description: String? = nil) {
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
    init(from dppContract: DPPDataContract, name: String) {
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
    
    var formattedSchema: String {
        guard let jsonData = try? JSONSerialization.data(withJSONObject: schema, options: .prettyPrinted),
              let jsonString = String(data: jsonData, encoding: .utf8) else {
            return "Invalid schema"
        }
        return jsonString
    }
}