import Foundation
import SwiftData

@Model
final class PersistentKeyword {
    @Attribute(.unique) var id: String // contractId + keyword
    var keyword: String
    var contractId: String
    
    // Relationship
    var contract: PersistentContract?
    
    init(keyword: String, contractId: String) {
        self.id = "\(contractId)_\(keyword)"
        self.keyword = keyword
        self.contractId = contractId
    }
}

// MARK: - Queries
extension PersistentKeyword {
    static func predicate(keyword: String) -> Predicate<PersistentKeyword> {
        #Predicate<PersistentKeyword> { item in
            item.keyword.localizedStandardContains(keyword)
        }
    }
    
    static func predicate(contractId: String) -> Predicate<PersistentKeyword> {
        #Predicate<PersistentKeyword> { item in
            item.contractId == contractId
        }
    }
}