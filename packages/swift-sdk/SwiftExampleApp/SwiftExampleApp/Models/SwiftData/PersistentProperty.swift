import Foundation
import SwiftData

@Model
final class PersistentProperty {
    @Attribute(.unique) var id: Data // Combines contractId + documentType + propertyName
    var contractId: Data
    var documentTypeName: String
    var name: String
    
    // Property type and constraints
    var type: String
    var format: String?
    var contentEncoding: String?
    var pattern: String?
    var minLength: Int?
    var maxLength: Int?
    var minValue: Int?
    var maxValue: Int?
    var propertyDescription: String?
    
    // Property attributes
    var transient: Bool
    var isRequired: Bool
    
    // Timestamps
    var createdAt: Date
    
    // Relationship to document type
    var documentType: PersistentDocumentType?
    
    init(contractId: Data, documentTypeName: String, name: String, type: String) {
        // Create unique ID by combining contract ID, document type name, and property name
        var idData = contractId
        idData.append(documentTypeName.data(using: .utf8) ?? Data())
        idData.append(name.data(using: .utf8) ?? Data())
        self.id = idData
        
        self.contractId = contractId
        self.documentTypeName = documentTypeName
        self.name = name
        self.type = type
        self.transient = false
        self.isRequired = false
        self.createdAt = Date()
    }
}