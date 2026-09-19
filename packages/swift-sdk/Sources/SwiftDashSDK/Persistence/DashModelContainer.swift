import Foundation
import SwiftData

/// Factory for creating SwiftData model containers for Dash Platform persistence
public enum DashModelContainer {
    /// Accepted V1 graph. Every registered type stays frozen: relationship
    /// destinations can otherwise rebind by entity name to a changed live type.
    private static func frozenModelGraph(
        assetLock: any PersistentModel.Type
    ) -> [any PersistentModel.Type] {
        [
            DashSchemaV1.PersistentIdentity.self,
            DashSchemaV1.PersistentDPNSName.self,
            DashSchemaV1.PersistentDashpayProfile.self,
            DashSchemaV1.PersistentDashpayContactProfile.self,
            DashSchemaV1.PersistentDashpayContactRequest.self,
            DashSchemaV1.PersistentDashpayPayment.self,
            DashSchemaV1.PersistentDashpayIgnoredSender.self,
            DashSchemaV1.PersistentDocument.self,
            DashSchemaV1.PersistentDataContract.self,
            DashSchemaV1.PersistentPublicKey.self,
            DashSchemaV1.PersistentTokenBalance.self,
            DashSchemaV1.PersistentKeyword.self,
            DashSchemaV1.PersistentToken.self,
            DashSchemaV1.PersistentDocumentType.self,
            DashSchemaV1.PersistentIndex.self,
            DashSchemaV1.PersistentProperty.self,
            DashSchemaV1.PersistentTokenHistoryEvent.self,
            DashSchemaV1.PersistentPlatformAddress.self,
            DashSchemaV1.PersistentPlatformAddressesSyncState.self,
            DashSchemaV1.PersistentWallet.self,
            DashSchemaV1.PersistentAccount.self,
            DashSchemaV1.PersistentCoreAddress.self,
            DashSchemaV1.PersistentTransaction.self,
            DashSchemaV1.PersistentTxo.self,
            DashSchemaV1.PersistentPendingInput.self,
            DashSchemaV1.PersistentWalletManagerMetadata.self,
            DashSchemaV1.PersistentShieldedNote.self,
            DashSchemaV1.PersistentShieldedOutgoingNote.self,
            DashSchemaV1.PersistentShieldedSyncState.self,
            DashSchemaV1.PersistentShieldedActivity.self,
            DashSchemaV1.PersistentShieldedViewingKey.self,
            assetLock,
            DashSchemaV1.PersistentInvitation.self,
            DashSchemaV1.PersistentMasternode.self
        ]
    }

    /// The exact model set registered as schema V1. Keep frozen: staged
    /// migration identifies an existing store by this schema's checksum.
    fileprivate static var v1ModelTypes: [any PersistentModel.Type] {
        frozenModelGraph(assetLock: DashSchemaV1.PersistentAssetLock.self)
    }

    /// Live models for the next App Store schema. A release snapshot does not
    /// replace these types until a subsequent shape change introduces a new live
    /// version; callers must continue fetching the top-level model types.
    public static var modelTypes: [any PersistentModel.Type] {
        [
            PersistentIdentity.self,
            PersistentDPNSName.self,
            PersistentDashpayProfile.self,
            PersistentDashpayContactProfile.self,
            PersistentDashpayContactRequest.self,
            PersistentDashpayPayment.self,
            PersistentDashpayIgnoredSender.self,
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
            PersistentTxo.self,
            PersistentPendingInput.self,
            PersistentWalletManagerMetadata.self,
            PersistentShieldedNote.self,
            PersistentShieldedOutgoingNote.self,
            PersistentShieldedSyncState.self,
            PersistentShieldedActivity.self,
            PersistentShieldedViewingKey.self,
            PersistentAssetLock.self,
            PersistentInvitation.self,
            PersistentMasternode.self,
            PersistentTrackedMasternode.self
        ]
    }

    /// Create the schema for all Dash Platform models
    public static var schema: Schema {
        Schema(versionedSchema: DashSchemaV2.self)
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
        return try makeContainer(configuration: modelConfiguration)
    }

    /// Open (or create) the store at an explicit file URL through the same
    /// schema and migration plan as `create(cloudKit:groupContainer:)`. The
    /// migration tests use it to open stores written by older builds exactly
    /// the way the app would.
    static func create(url: URL) throws -> ModelContainer {
        let modelConfiguration = ModelConfiguration(
            schema: schema,
            url: url,
            allowsSave: true,
            cloudKitDatabase: .none
        )
        return try makeContainer(configuration: modelConfiguration)
    }

    /// The one place a persistent container is built: the live schema is
    /// constructed first (`schema`), and the container then runs the
    /// migration plan over it. That order is what the frozen versions are
    /// tested against, because it is the order under which a mixed
    /// live/frozen graph would rebind a released version's entities.
    private static func makeContainer(
        configuration: ModelConfiguration
    ) throws -> ModelContainer {
        // Always wire the migration plan so stores created by an older SDK
        // advance through the registered versioned schemas.
        try ModelContainer(
            for: schema,
            migrationPlan: DashMigrationPlan.self,
            configurations: [configuration]
        )
    }

    /// Create an in-memory model container for testing
    /// - Returns: A configured in-memory ModelContainer
    public static func createInMemory() throws -> ModelContainer {
        let modelConfiguration = ModelConfiguration(
            schema: schema,
            isStoredInMemoryOnly: true
        )
        return try makeContainer(configuration: modelConfiguration)
    }
}

/// SwiftData migration plan for Dash Platform model updates
public enum DashMigrationPlan: SchemaMigrationPlan {
    public static var schemas: [any VersionedSchema.Type] {
        [DashSchemaV1.self, DashSchemaV2.self]
    }

    public static var stages: [MigrationStage] {
        [
            .lightweight(fromVersion: DashSchemaV1.self, toVersion: DashSchemaV2.self)
        ]
    }
}

/// The accepted historical baseline, pinned by the unchanged generated V1
/// models and `Fixtures/SchemaStores/dash-v1.store`. Preserve those definitions
/// and fixture bytes when introducing later schema versions.
///
/// Migration tests establish compatibility from this accepted baseline into
/// the current live schema. They do not reconstruct or verify the database
/// written by the original App Store binary. Other historical development
/// layouts are unsupported: opening an unrecognized store throws an error;
/// the container does not silently erase or recreate it.
public enum DashSchemaV1: VersionedSchema {
    public static var versionIdentifier: Schema.Version {
        Schema.Version(1, 0, 0)
    }

    public static var models: [any PersistentModel.Type] {
        DashModelContainer.v1ModelTypes
    }
}

/// Unreleased V2 combines the tracked-masternode, asset-lock recipient, sweep,
/// and public-key usage-limit additions. V1 is the accepted historical baseline. Intermediate beta
/// layouts are not supported release schemas.
///
/// After App Store publication a separate DashSchemaSnapshotV2 preserves the
/// complete graph. Keep this version on the live models while they match that
/// snapshot. The next shape change must move this version onto the snapshot,
/// introduce a new live version and explicitly test its migration.
public enum DashSchemaV2: VersionedSchema {
    public static var versionIdentifier: Schema.Version {
        Schema.Version(2, 0, 0)
    }

    public static var models: [any PersistentModel.Type] {
        DashModelContainer.modelTypes
    }
}
