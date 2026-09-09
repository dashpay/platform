import Foundation
import SwiftData

/// Factory for creating SwiftData model containers for Dash Platform persistence
public enum DashModelContainer {
    /// The wallet-and-platform model graph as every released schema version
    /// registered it, built from the frozen copies under `FrozenSchemas/`
    /// and parameterised on the one slot whose frozen shape differs between
    /// versions (`PersistentAssetLock`, which V3 changed).
    ///
    /// Every entry is a nested frozen type, never a live one. A released
    /// version's checksum is the hash of every entity it declares — and a
    /// relationship binds its destination by entity NAME, so a version that
    /// mixed one live model into an otherwise frozen graph would have that
    /// live model's current shape hashed into it (the entity name resolves
    /// to whichever Swift type claimed it first in the process). Freezing
    /// the whole relationship-connected graph per version is what keeps a
    /// released checksum stable no matter what the live models do next, and
    /// `DashModelMigrationTests` proves it against stores an older build
    /// actually wrote.
    ///
    /// Ordering is load-bearing only in the sense that it must not need to
    /// change: keeping each model in the slot its live counterpart occupies
    /// makes a frozen version's list positionally identical to what that
    /// version shipped.
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

    /// The exact model set registered as schema V2 — V1 plus
    /// `PersistentTrackedMasternode`. Frozen for the same reason as
    /// `v1ModelTypes`.
    fileprivate static var v2ModelTypes: [any PersistentModel.Type] {
        v1ModelTypes + [DashSchemaV2.PersistentTrackedMasternode.self]
    }

    /// The exact model set registered as schema V3 — V2 with the asset-lock
    /// shape that gained `recipientIsExternal`. Frozen for the same reason
    /// as `v1ModelTypes`.
    fileprivate static var v3ModelTypes: [any PersistentModel.Type] {
        frozenModelGraph(assetLock: DashSchemaV3.PersistentAssetLock.self)
            + [DashSchemaV2.PersistentTrackedMasternode.self]
    }

    /// All persistent model types in the current Dash SDK schema (V4).
    /// Unlike the released versions above this list tracks the LIVE models,
    /// so it moves whenever a model gains a property — which is exactly why
    /// the released versions must not. When the next property lands: freeze
    /// every model here into the version being retired
    /// (`scripts/freeze_schema_models.py`), add a version, add a stage, and
    /// commit a store written by the build that shipped the retired version
    /// under the test fixtures. `DashModelMigrationTests` proves a freeze
    /// complete (in what the entity hash covers, plus its indexes) only
    /// against such a store, and fails until every retired version has one.
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
        Schema(versionedSchema: DashSchemaV4.self)
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
        [DashSchemaV1.self, DashSchemaV2.self, DashSchemaV3.self, DashSchemaV4.self]
    }

    public static var stages: [MigrationStage] {
        [
            .lightweight(fromVersion: DashSchemaV1.self, toVersion: DashSchemaV2.self),
            .lightweight(fromVersion: DashSchemaV2.self, toVersion: DashSchemaV3.self),
            .lightweight(fromVersion: DashSchemaV3.self, toVersion: DashSchemaV4.self)
        ]
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
///   - `PersistentDashpayContactRequest` gained the additive
///     `paymentChannelBroken` column (defaulted `false`) so the G1c
///     broken-channel flag projected by the persister survives
///     restarts. Additive-with-default ⇒ lightweight migration.
///   - `PersistentDashpayPayment` was added (cascade-owned by
///     `PersistentIdentity` via the new `dashpayPayments`
///     collection). Mirrors the per-identity `dashpay_payments` map
///     read through `managed_identity_get_dashpay_payments`; rows are
///     refreshed by `PlatformWalletManager.refreshDashPayPayments`
///     (the persister doesn't project payment history). Additive
///     model + additive relationship ⇒ lightweight migration.
///   - `PersistentDashpayIgnoredSender` was added (cascade-owned by
///     `PersistentIdentity` via the new `dashpayIgnoredSenders`
///     collection). Persists per-sender ignores (local-only mute, =
///     block, reversible) the persister projects in the `ignored`
///     changeset array so the Rust `ignored_senders` set can be restored
///     at load — without it an ignored sender resurfaces on relaunch.
///     Keyed per-sender (no `accountReference`), so an ignored sender's
///     rotated requests are suppressed too. Additive model + additive
///     relationship ⇒ lightweight migration. (Replaces the earlier
///     per-`(sender, accountReference)` `PersistentDashpayRejectedRequest`
///     — the model decision collapsed reject into ignore.)
///   - `PersistentDashpayContactProfile` was added (cascade-owned by
///     `PersistentIdentity` via the new `contactProfiles` collection).
///     Mirrors one entry of the per-identity `contact_profiles` map
///     (cached contacts' public profiles, keyed by the contact's
///     identity id) projected by the persister as
///     `IdentityEntryFFI.contact_profiles` rows, and read back at load to
///     rebuild the Rust cache so contacts don't refetch on every
///     relaunch. Distinct from `PersistentDashpayProfile` (the owner's
///     own profile). Additive model + additive relationship ⇒
///     lightweight migration.
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
///   - `PersistentTokenBalance.balance` remains the original `Int64` SwiftData
///     property and SQLite column. Protocol `u64` values use its raw bits via a
///     computed accessor, so full-domain support does not alter this V1 schema.
///   - `PersistentDPNSName` gained the DPNS username-marketplace
///     columns `documentIdBase58`, `priceCredits`, `saleStatusRaw`,
///     `counterpartyIdBase58`, the three optional document timestamps,
///     and `marketplaceUpdatedAt`, written by
///     the new `on_persist_dpns_name_states_fn` persister callback
///     (`DpnsNameStateFFI`). All optional or defaulted, and the
///     `(networkRaw, normalizedParentDomainName, normalizedLabel)`
///     uniqueness is unchanged ⇒ lightweight migration. Existing rows
///     migrate with a nil `documentIdBase58`, which is the documented
///     "no marketplace state tracked" signal — the next marketplace
///     sync pass fills them in.
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
        DashModelContainer.v1ModelTypes
    }
}

/// Version 2 adds wallet-independent tracked masternodes. The new model has
/// no relationship or required-data dependency on V1 rows, so a lightweight
/// migration preserves every existing row and creates its table.
public enum DashSchemaV2: VersionedSchema {
    public static var versionIdentifier: Schema.Version {
        Schema.Version(2, 0, 0)
    }

    public static var models: [any PersistentModel.Type] {
        DashModelContainer.v2ModelTypes
    }
}

/// Version 3 adds `recipientIsExternal` to `PersistentAssetLock` — an
/// optional column on an existing entity, so a lightweight migration
/// preserves every existing row and backfills `NULL`.
///
/// This is the first version to be registered alongside a genuinely frozen
/// copy of the model it changes (`DashSchemaV1.PersistentAssetLock`). Without
/// that copy, adding the property would have mutated V1's and V2's checksums
/// in place and a store written by the V2 binary would have matched no
/// registered schema, failing to open with Cocoa error 134504 rather than
/// migrating. Follow the same pattern for the next property added to any
/// model: freeze the old shape, add a version, add a stage.
public enum DashSchemaV3: VersionedSchema {
    public static var versionIdentifier: Schema.Version {
        Schema.Version(3, 0, 0)
    }

    public static var models: [any PersistentModel.Type] {
        DashModelContainer.v3ModelTypes
    }
}

/// Version 4 adds the sweep columns, on the same entity set as V3:
///   - `PersistentTxo.supersededByTxid` (optional) and
///     `PersistentPendingInput.isSweptTombstone` (defaulted `false`).
///     Together they let a sweep's claim on an input whose funding TXO
///     hasn't arrived yet survive the loser transaction's deletion —
///     previously that claim lived only on the doomed row's
///     `PersistentPendingInput`, which cascades away with it. Existing
///     rows migrate as ordinary (non-tombstone, non-superseded) entries.
///   - `PersistentPendingInput.winnerMinedHeight` (optional — a
///     block-context sweep tombstone's finality stamp, the winner's own
///     mined height) and `PersistentWallet.lastAppliedChainLockHeight`
///     (optional — the numeric chainlock watermark delivered by
///     `on_persist_wallet_changeset_chain_lock_height_fn`, stored
///     monotonic-max). Together they drive the bounded tombstone lifetime:
///     a tombstone is collected exactly when
///     `min(chainlockHeight, syncedHeight)` reaches its stamp.
///     Pre-existing rows read as unstamped (held forever) over a wallet
///     with no boundary yet.
///   - The `(walletId, isSweptTombstone)` index on
///     `PersistentPendingInput`, serving the collector's tombstone-only
///     scan.
/// Every column is additive with a default or optional and the index is
/// additive, so a lightweight migration preserves each existing row.
///
/// Registering it required freezing every model V1–V3 register — the
/// generated copies under `FrozenSchemas/`, see
/// `scripts/freeze_schema_models.py`.
public enum DashSchemaV4: VersionedSchema {
    public static var versionIdentifier: Schema.Version {
        Schema.Version(4, 0, 0)
    }

    public static var models: [any PersistentModel.Type] {
        DashModelContainer.modelTypes
    }
}
