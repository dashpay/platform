import Foundation
import SwiftData

@Model
final class PersistentDataContract {
    @Attribute(.unique) var id: Data
    var name: String
    var serializedContract: Data
    var createdAt: Date
    var lastAccessedAt: Date
    
    // Computed properties
    var idBase58: String {
        id.toBase58String()
    }
    
    init(id: Data, name: String, serializedContract: Data) {
        self.id = id
        self.name = name
        self.serializedContract = serializedContract
        self.createdAt = Date()
        self.lastAccessedAt = Date()
    }
    
    func updateLastAccessed() {
        self.lastAccessedAt = Date()
    }
}