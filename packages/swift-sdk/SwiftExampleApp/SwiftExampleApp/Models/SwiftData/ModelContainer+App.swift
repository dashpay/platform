import Foundation
import SwiftData
import SwiftDashSDK

/// App-specific SwiftData model container configuration
extension ModelContainer {
    /// Create the app's model container with all persistent models
    static func appContainer() throws -> ModelContainer {
        return try DashModelContainer.create()
    }

    /// Create an in-memory container for testing
    static func inMemoryContainer() throws -> ModelContainer {
        return try DashModelContainer.createInMemory()
    }
}

/// Re-export SDK migration types for backward compatibility
public typealias AppMigrationPlan = DashMigrationPlan
public typealias AppSchemaV1 = DashSchemaV1
