import Foundation
import SwiftData

/// Factory for creating SwiftData model containers for Dash Platform persistence
public enum DashModelContainer {
    /// All persistent model types for the Dash SDK
    public static var modelTypes: [any PersistentModel.Type] {
        [
            PersistentIdentity.self,
            PersistentDocument.self,
            PersistentDataContract.self,
            PersistentPublicKey.self,
            PersistentTokenBalance.self,
            PersistentKeyword.self,
            PersistentToken.self,
            PersistentDocumentType.self,
            PersistentIndex.self,
            PersistentProperty.self,
            PersistentTokenHistoryEvent.self,
            PersistentPlatformAddress.self,
            PersistentPlatformAddressesSyncState.self,
            PersistentWallet.self,
            PersistentAccount.self,
            PersistentCoreAddress.self,
            PersistentTransaction.self,
            PersistentUtxo.self,
            PersistentWalletManagerMetadata.self
        ]
    }

    /// Create the schema for all Dash Platform models
    public static var schema: Schema {
        Schema(modelTypes)
    }

    /// Create a persistent model container for storing data
    /// - Parameters:
    ///   - cloudKit: Whether to enable CloudKit sync (default: disabled)
    ///   - groupContainer: App group container configuration
    /// - Returns: A configured ModelContainer
    public static func create(
        cloudKit: Bool = false,
        groupContainer: ModelConfiguration.GroupContainer = .automatic
    ) throws -> ModelContainer {
        let modelConfiguration = ModelConfiguration(
            schema: schema,
            isStoredInMemoryOnly: false,
            allowsSave: true,
            groupContainer: groupContainer,
            cloudKitDatabase: cloudKit ? .automatic : .none
        )

        // Wire the migration plan even though V1 is the only shipped
        // schema — future schema bumps just have to add a stage to
        // `DashMigrationPlan.stages` without also having to remember
        // to thread the plan into the container construction call.
        return try ModelContainer(
            for: schema,
            migrationPlan: DashMigrationPlan.self,
            configurations: [modelConfiguration]
        )
    }

    /// Create an in-memory model container for testing
    /// - Returns: A configured in-memory ModelContainer
    public static func createInMemory() throws -> ModelContainer {
        let modelConfiguration = ModelConfiguration(
            schema: schema,
            isStoredInMemoryOnly: true
        )

        return try ModelContainer(
            for: schema,
            migrationPlan: DashMigrationPlan.self,
            configurations: [modelConfiguration]
        )
    }
}

/// SwiftData migration plan for Dash Platform model updates
public enum DashMigrationPlan: SchemaMigrationPlan {
    public static var schemas: [any VersionedSchema.Type] {
        [DashSchemaV1.self]
    }

    public static var stages: [MigrationStage] {
        []
    }
}

/// Version 1 of the Dash Platform schema
/// Includes `PersistentCoreAddress` to match the example app's former container schema.
/// The model is additive with optional relationships, so existing narrower stores can
/// use SwiftData's lightweight migration path.
public enum DashSchemaV1: VersionedSchema {
    public static var versionIdentifier: Schema.Version {
        Schema.Version(1, 0, 0)
    }

    public static var models: [any PersistentModel.Type] {
        DashModelContainer.modelTypes
    }
}
