import CoreData
import Foundation
import Dispatch
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
            PersistentTrackedMasternode.self,
            PersistentIdentityBalanceMetadata.self
        ]
    }

    /// Create the schema for all Dash Platform models
    public static var schema: Schema {
        Schema(versionedSchema: DashSchemaV3.self)
    }

    /// Create a persistent model container for storing data.
    /// This synchronous call can copy and migrate a full database and block for
    /// seconds. Do not call it on a UI actor. Apps with an explicit local URL
    /// can use `createAsync(url:)` to open on the SDK's dedicated queue.
    /// The legacy compatibility bridge is local-only; CloudKit uses the normal
    /// migration plan because copying SQLite cannot preserve its sync state.
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
        return try makeContainer(configuration: modelConfiguration, bridgeLegacyStore: !cloudKit)
    }

    /// Open (or create) the store at an explicit file URL through the same
    /// schema and migration plan as `create(cloudKit:groupContainer:)`. The
    /// migration tests use it to open stores written by older builds exactly
    /// the way the app would. This synchronous call can block for seconds;
    /// use `createAsync(url:)` on startup or from a UI actor.
    public static func create(url: URL) throws -> ModelContainer {
        let modelConfiguration = ModelConfiguration(
            schema: schema,
            url: url,
            allowsSave: true,
            cloudKitDatabase: .none
        )
        return try makeContainer(configuration: modelConfiguration)
    }

    private static let storeOpenQueue = DispatchQueue(
        label: "org.dash.swift-sdk.store-open", qos: .userInitiated)

    /// Open and migrate a local store without blocking the caller's actor.
    /// Only the Sendable container crosses the queue; create/use contexts on
    /// their owning actor after this returns. Callers sharing a URL should
    /// coalesce in-flight opens and retain one container for that store.
    public static func createAsync(url: URL) async throws -> ModelContainer {
        try await withCheckedThrowingContinuation { continuation in
            storeOpenQueue.async {
                do {
                    let container = try autoreleasepool { try create(url: url) }
                    continuation.resume(returning: container)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// The one place a persistent container is built: the live schema is
    /// constructed first (`schema`), and the container then runs the
    /// migration plan over it. That order is what the frozen versions are
    /// tested against, because it is the order under which a mixed
    /// live/frozen graph would rebind a released version's entities.
    private static func makeContainer(
        configuration: ModelConfiguration,
        bridgeLegacyStore: Bool = true
    ) throws -> ModelContainer {
        SDKLogger.event("store_open_started", category: .persistence,
                        fields: ["target_version": .publicText("3.0.0")])
        do {
            let container: ModelContainer
            if bridgeLegacyStore {
                container = try DashLegacySchemaBridge.open(
                    configuration: configuration, schema: schema, plan: DashMigrationPlan.self)
            } else {
                container = try ModelContainer(
                    for: schema,
                    migrationPlan: try migrationPlan(at: configuration.url, defaultPlan: DashMigrationPlan.self),
                    configurations: [configuration])
            }
            SDKLogger.event("store_open_succeeded", category: .persistence,
                            fields: ["target_version": .publicText("3.0.0")])
            return container
        } catch {
            logMigrationFailure(error)
            throw error
        }
    }

    /// Error descriptions/userInfo can include rows, URLs and other private
    /// values. Record only a known system domain (or the error's type) and code.
    private static func logMigrationFailure(_ error: Error) {
        let nsError = error as NSError
        let systemDomains: Set<String> = [NSCocoaErrorDomain, NSPOSIXErrorDomain, NSOSStatusErrorDomain, "NSSQLiteErrorDomain"]
        let domain = systemDomains.contains(nsError.domain) ? nsError.domain : String(reflecting: type(of: error))
        SDKLogger.event("store_open_failed", category: .persistence, severity: .error,
                        fields: ["error_domain": .publicText(domain), "error_code": .integer(Int64(nsError.code)),
                                 "target_version": .publicText("3.0.0")])
    }

    /// Select by the complete stored model identity, after journal recovery.
    /// Accepted V1 contains fields absent from the real historical V2; sending
    /// it through V2 would delete those values before V3 adds the fields again.
    /// `identity` resolves a schema's stored identity; tests inject a failing
    /// probe to pin which routes may depend on it. Only labels whose route is
    /// undecidable without it (1.0.0, 2.0.0) consult the probe at all.
    static func migrationPlan(
        at url: URL, defaultPlan: any SchemaMigrationPlan.Type,
        identity: (any VersionedSchema.Type) throws -> DashLegacySchemaBridge.Identity
            = DashLegacySchemaBridge.identity(for:)
    ) throws -> any SchemaMigrationPlan.Type {
        guard ObjectIdentifier(defaultPlan) == ObjectIdentifier(DashMigrationPlan.self) else { return defaultPlan }
        guard FileManager.default.fileExists(atPath: url.path) else {
            SDKLogger.event("store_migration_route", category: .persistence, fields: [
                "route": .publicText("new-store"), "target_version": .publicText("3.0.0")])
            return defaultPlan
        }
        let metadata = try NSPersistentStoreCoordinator.metadataForPersistentStore(type: .sqlite, at: url)
        let versions = metadata[NSStoreModelVersionIdentifiersKey] as? [String]
        let hashes = metadata[NSStoreModelVersionHashesKey] as? [String: Data]
        let checksum = metadata["NSStoreModelVersionChecksumKey"] as? String
        // Only validated schema metadata is public diagnostic material. Never
        // log arbitrary strings from malformed store metadata or any store path.
        let safeVersions = versions.flatMap { values -> String? in
            guard !values.isEmpty, values.count <= 4,
                  values.allSatisfy({ $0.count <= 32 && $0.range(of: #"^\d+\.\d+\.\d+$"#, options: .regularExpression) != nil }) else { return nil }
            return values.joined(separator: ",")
        } ?? "unavailable"
        let safeChecksum = checksum.flatMap { Data(base64Encoded: $0)?.count == 32 ? $0 : nil } ?? "unavailable"
        func logRoute(_ route: String) {
            SDKLogger.event("store_migration_route", category: .persistence, fields: [
                "source_version": .publicText(safeVersions), "source_checksum": .publicText(safeChecksum),
                "route": .publicText(route), "target_version": .publicText("3.0.0")])
        }
        func matches(_ type: any VersionedSchema.Type) throws -> Bool {
            let expected = try identity(type)
            return hashes == expected.hashes && (checksum == nil || checksum == expected.checksum)
        }
        // A 1.0.0 label alone cannot choose between the accepted-V1 route and
        // the legacy bridge, and the default plan has no V1 stage, so a failed
        // probe stays fatal here: the open fails with the probe's own error
        // instead of an inapplicable plan, and the store is untouched.
        if versions == ["1.0.0"], try matches(DashSchemaV1.self) {
            logRoute("accepted-v1-to-v3")
            return DashAcceptedV1MigrationPlan.self
        }
        if versions == ["2.0.0"] {
            // The immediately preceding candidate used the current graph with
            // a V2 label. Accept its exact shape, never arbitrary beta V2 data.
            // A probe failure is equally undecidable and refuses the same way.
            let historical = try matches(DashSchemaV2.self)
            let previousCurrent = try !historical && matches(DashSchemaV3.self)
            guard historical || previousCurrent else {
                logRoute("unsupported-v2")
                throw DashLegacyStoreSQLite.Failure.unsupported(
                    "The database identifies itself as schema 2.0.0 but its model does not match the supported historical or current schema. The original database has not been replaced. Contact support; do not delete the app.")
            }
            logRoute(historical ? "historical-v2-to-v3" : "previous-live-v2-current-shape")
        } else if versions == ["3.0.0"] {
            // Same plan either way. Opening a current store must neither wait
            // on nor fail with a schema probe that could only refine this line;
            // `source_checksum` above already identifies the exact graph.
            logRoute("labelled-current-v3")
        } else {
            logRoute("ordinary-current-plan")
        }
        return defaultPlan
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
        [DashSchemaV2.self, DashSchemaV3.self]
    }

    public static var stages: [MigrationStage] {
        [
            .lightweight(fromVersion: DashSchemaV2.self, toVersion: DashSchemaV3.self)
        ]
    }
}

/// Separate compatibility route: V1 has fields missing from historical V2.
/// Never insert V2 between this baseline and the current schema.
enum DashAcceptedV1MigrationPlan: SchemaMigrationPlan {
    static var schemas: [any VersionedSchema.Type] { [DashSchemaV1.self, DashSchemaV3.self] }
    static var stages: [MigrationStage] {
        [.lightweight(fromVersion: DashSchemaV1.self, toVersion: DashSchemaV3.self)]
    }
}

/// The accepted historical baseline, pinned by the unchanged generated V1
/// models and `Fixtures/SchemaStores/dash-v1.store`. Preserve those definitions
/// and fixture bytes when introducing later schema versions.
///
/// Migration tests establish compatibility from this accepted baseline into
/// the current live schema. They do not reconstruct or verify the database
/// written by the original App Store binary. Other historical development
/// layouts are accepted only by the local legacy bridge when every existing
/// value and relationship survives migration to fixed V3. Other layouts fail;
/// the container never erases or recreates a user's database.
public enum DashSchemaV1: VersionedSchema {
    public static var versionIdentifier: Schema.Version {
        Schema.Version(1, 0, 0)
    }

    public static var models: [any PersistentModel.Type] {
        DashModelContainer.v1ModelTypes
    }
}

/// Historical V2 reconstructed from commit 52e8d4ec68. Its complete frozen
/// graph matches the database observed on the released application. Other
/// development layouts that reused the V2 number are not historical V2.
public enum DashSchemaV2: VersionedSchema {
    public static var versionIdentifier: Schema.Version { Schema.Version(2, 0, 0) }
    public static var models: [any PersistentModel.Type] {
        [
            DashSchemaV2.PersistentIdentity.self,
            DashSchemaV2.PersistentDPNSName.self,
            DashSchemaV2.PersistentDashpayProfile.self,
            DashSchemaV2.PersistentDashpayContactProfile.self,
            DashSchemaV2.PersistentDashpayContactRequest.self,
            DashSchemaV2.PersistentDashpayPayment.self,
            DashSchemaV2.PersistentDashpayIgnoredSender.self,
            DashSchemaV2.PersistentDocument.self,
            DashSchemaV2.PersistentDataContract.self,
            DashSchemaV2.PersistentPublicKey.self,
            DashSchemaV2.PersistentTokenBalance.self,
            DashSchemaV2.PersistentKeyword.self,
            DashSchemaV2.PersistentToken.self,
            DashSchemaV2.PersistentDocumentType.self,
            DashSchemaV2.PersistentIndex.self,
            DashSchemaV2.PersistentProperty.self,
            DashSchemaV2.PersistentTokenHistoryEvent.self,
            DashSchemaV2.PersistentPlatformAddress.self,
            DashSchemaV2.PersistentPlatformAddressesSyncState.self,
            DashSchemaV2.PersistentWallet.self,
            DashSchemaV2.PersistentAccount.self,
            DashSchemaV2.PersistentCoreAddress.self,
            DashSchemaV2.PersistentTransaction.self,
            DashSchemaV2.PersistentTxo.self,
            DashSchemaV2.PersistentPendingInput.self,
            DashSchemaV2.PersistentWalletManagerMetadata.self,
            DashSchemaV2.PersistentShieldedNote.self,
            DashSchemaV2.PersistentShieldedOutgoingNote.self,
            DashSchemaV2.PersistentShieldedSyncState.self,
            DashSchemaV2.PersistentShieldedActivity.self,
            DashSchemaV2.PersistentShieldedViewingKey.self,
            DashSchemaV2.PersistentAssetLock.self,
            DashSchemaV2.PersistentInvitation.self,
            DashSchemaV2.PersistentMasternode.self,
            DashSchemaV2.PersistentTrackedMasternode.self
        ]
    }
}

/// Current working schema. A later shape change must preserve this graph as
/// the fixed legacy-bridge target before introducing another live version.
public enum DashSchemaV3: VersionedSchema {
    public static var versionIdentifier: Schema.Version { Schema.Version(3, 0, 0) }
    public static var models: [any PersistentModel.Type] { DashModelContainer.modelTypes }
}
