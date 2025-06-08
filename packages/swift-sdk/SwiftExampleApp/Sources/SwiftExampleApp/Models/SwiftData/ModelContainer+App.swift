import Foundation
import SwiftData

/// App-specific SwiftData model container configuration
extension ModelContainer {
    /// Create the app's model container with all persistent models
    static func appContainer() throws -> ModelContainer {
        let schema = Schema([
            PersistentIdentity.self,
            PersistentDocument.self,
            PersistentContract.self,
            PersistentPublicKey.self,
            PersistentTokenBalance.self
        ])
        
        let modelConfiguration = ModelConfiguration(
            schema: schema,
            isStoredInMemoryOnly: false,
            allowsSave: true,
            groupContainer: .automatic,
            cloudKitDatabase: .none  // Disable CloudKit sync for now
        )
        
        return try ModelContainer(
            for: schema,
            configurations: [modelConfiguration]
        )
    }
    
    /// Create an in-memory container for testing
    static func inMemoryContainer() throws -> ModelContainer {
        let schema = Schema([
            PersistentIdentity.self,
            PersistentDocument.self,
            PersistentContract.self,
            PersistentPublicKey.self,
            PersistentTokenBalance.self
        ])
        
        let modelConfiguration = ModelConfiguration(
            schema: schema,
            isStoredInMemoryOnly: true
        )
        
        return try ModelContainer(
            for: schema,
            configurations: [modelConfiguration]
        )
    }
}

/// SwiftData migration plan for model updates
enum AppMigrationPlan: SchemaMigrationPlan {
    static var schemas: [any VersionedSchema.Type] {
        [AppSchemaV1.self]
    }
    
    static var stages: [MigrationStage] {
        []  // No migrations yet - this is V1
    }
}

/// Version 1 of the app schema
enum AppSchemaV1: VersionedSchema {
    static var versionIdentifier: Schema.Version {
        Schema.Version(1, 0, 0)
    }
    
    static var models: [any PersistentModel.Type] {
        [
            PersistentIdentity.self,
            PersistentDocument.self,
            PersistentContract.self,
            PersistentPublicKey.self,
            PersistentTokenBalance.self
        ]
    }
}