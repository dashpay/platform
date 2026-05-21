import Foundation
import SwiftData

/// Factory for creating SwiftData model containers for Dash Platform persistence
public enum DashModelContainer {
    /// All persistent model types for the Dash SDK
    public static var modelTypes: [any PersistentModel.Type] {
        [
            PersistentIdentity.self,
            PersistentDPNSName.self,
            PersistentDashpayProfile.self,
            PersistentDashpayContactRequest.self,
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
            PersistentAssetLock.self
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
///
/// Note: this V1 identifier has accumulated several destructive
/// dev-only changes that cannot be expressed via the lightweight
/// migration path:
///   - `PersistentTransaction.txid` and the renamed
///     `PersistentTxo.outpoint` switched from `String` to raw `Data`
///     (unique-attribute retype).
///   - The `PersistentUtxo` model was renamed to `PersistentTxo`,
///     gained `walletId` + `spendingTransaction`, and the schema
///     topology shifted: `PersistentTransaction` lost both
///     `walletId` and `account` and now hangs on transactions purely
///     through the `outputs` / `inputs` TXO relationships.
///   - `PersistentAccount.outputs` (the cascade-owned
///     `[PersistentTxo]` collection paired with
///     `PersistentTxo.account`) was removed. Per-account TXOs are
///     now derived through `coreAddresses.flatMap(\.txos)` —
///     `PersistentTxo.account` survives as a one-way fallback
///     pointer with no inverse. Removing the inverse changes the
///     relationship topology for the underlying SQLite store, so
///     existing dev stores can't be opened with the new schema.
///   - `PersistentAccount.wallet` was tightened from
///     `PersistentWallet?` to non-optional `PersistentWallet`. Every
///     account currently belongs to a wallet; the type system now
///     reflects that invariant. Switching the optionality of a
///     relationship column rewrites the SQLite schema, so existing
///     dev stores can't be reused.
///   - `PersistentWallet.isWatchOnly` and
///     `PersistentAccount.isWatchOnly` were removed. The runtime
///     watch-only state lives on the native `Wallet` /
///     `ManagedAccount` (FFI-backed); persisting it on the SwiftData
///     side was redundant and the persister never wrote it.
///   - `PersistentDPNSName` was added (cascade-owned by
///     `PersistentIdentity` via the new `dpnsNames` relationship)
///     so DPNS labels are persisted instead of recomputed on every
///     `IdentityDetailView` open. Existing dev stores predate the
///     row collection and rebuild on next sync; the changeset's
///     append-only merge policy populates the new rows from the
///     persister callback.
///   - `PersistentDashpayProfile` was added (cascade-owned by
///     `PersistentIdentity` via the new `dashpayProfile` optional
///     relationship). Mirrors `IdentityEntry::dashpay_profile` from
///     the FFI so DashPay profile fields (display name, public
///     message, avatar URL / hash / fingerprint, bio) are persisted
///     across launches instead of being refetched. Existing dev
///     stores predate the row and rebuild on next profile sync; the
///     persister upserts in place via
///     `PlatformWalletPersistenceHandler.upsertDashpayProfile`.
///   - `PersistentDashpayContactRequest` was added (cascade-owned by
///     `PersistentIdentity` via the new `contactRequests` collection).
///     Mirrors `ContactChangeSet::sent_requests` /
///     `incoming_requests` / `established` projected through the new
///     `on_persist_contacts_fn` FFI callback, with one row per
///     `(network, owner, contact, isOutgoing)` quad. Existing dev
///     stores predate the row collection and rebuild on next
///     DashPay contact sync.
///   - `PersistentAccount` gained `#Unique<…>([\.wallet, \.accountType,
///     \.accountIndex, \.userIdentityId, \.friendIdentityId])` plus
///     `@Attribute(.unique)` on `accountExtendedPubKeyBytes`. The
///     xpub field also flipped from `Data` to `Data?` so multiple
///     unhydrated rows (xpub not yet known) don't collide on the
///     UNIQUE constraint — SQL allows multiple `NULL`s. Together
///     these enforce "one row per account identity, one xpub per
///     account" at the database layer; pre-refactor the persister's
///     `applyAccountChangeset` was string-keyed on the legacy
///     `Debug`-formatted `account_type_name` and could grow
///     duplicate rows for the same logical account.
/// Each of those is a destructive change to a unique-attribute
/// column or to relationship topology, so any pre-existing dev
/// store will fail to open and get rebuilt from scratch on next
/// sync. Bumping the version isn't useful without a real
/// `MigrationStage` (and there's nothing worth preserving in dev
/// databases at this point), so we let the container recreate.
public enum DashSchemaV1: VersionedSchema {
    public static var versionIdentifier: Schema.Version {
        Schema.Version(1, 0, 0)
    }

    public static var models: [any PersistentModel.Type] {
        DashModelContainer.modelTypes
    }
}
