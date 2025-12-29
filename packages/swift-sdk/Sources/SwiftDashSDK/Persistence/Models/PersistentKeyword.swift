import Foundation
import SwiftData

/// SwiftData model for persisting contract keywords
@Model
public final class PersistentKeyword {
    @Attribute(.unique) public var id: String
    public var keyword: String
    public var contractId: String

    // Relationship
    public var dataContract: PersistentDataContract?

    public init(keyword: String, contractId: String) {
        self.id = "\(contractId)_\(keyword)"
        self.keyword = keyword
        self.contractId = contractId
    }
}

// MARK: - Queries
extension PersistentKeyword {
    public static func predicate(keyword: String) -> Predicate<PersistentKeyword> {
        #Predicate<PersistentKeyword> { item in
            item.keyword.localizedStandardContains(keyword)
        }
    }

    public static func predicate(contractId: String) -> Predicate<PersistentKeyword> {
        #Predicate<PersistentKeyword> { item in
            item.contractId == contractId
        }
    }
}
