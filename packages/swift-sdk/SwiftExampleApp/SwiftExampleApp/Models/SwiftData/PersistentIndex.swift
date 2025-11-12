import Foundation
import SwiftData

@Model
final class PersistentIndex {
    @Attribute(.unique) var id: Data // Combines contractId + documentType + indexName
    var contractId: Data
    var documentTypeName: String
    var name: String
    
    // Index configuration
    var unique: Bool
    var nullSearchable: Bool
    var contested: Bool
    
    // Properties in the index with sorting
    var propertiesJSON: Data // Array of property objects with sorting
    
    // Contested details (if contested)
    var contestedDetailsJSON: Data? // JSON with field matches and resolution
    
    // Timestamps
    var createdAt: Date
    
    // Relationship to document type
    var documentType: PersistentDocumentType?
    
    init(contractId: Data, documentTypeName: String, name: String, properties: [String]) {
        // Create unique ID by combining contract ID, document type name, and index name
        var idData = contractId
        idData.append(documentTypeName.data(using: .utf8) ?? Data())
        idData.append(name.data(using: .utf8) ?? Data())
        self.id = idData
        
        self.contractId = contractId
        self.documentTypeName = documentTypeName
        self.name = name
        self.unique = false
        self.nullSearchable = false
        self.contested = false
        
        // Store properties as JSON array
        if let jsonData = try? JSONSerialization.data(withJSONObject: properties, options: []) {
            self.propertiesJSON = jsonData
        } else {
            self.propertiesJSON = Data()
        }
        
        self.createdAt = Date()
    }
}

// MARK: - Computed Properties
extension PersistentIndex {
    var properties: [String]? {
        try? JSONSerialization.jsonObject(with: propertiesJSON, options: []) as? [String]
    }
    
    var contestedDetails: [String: Any]? {
        guard let data = contestedDetailsJSON else { return nil }
        return try? JSONSerialization.jsonObject(with: data, options: []) as? [String: Any]
    }
}