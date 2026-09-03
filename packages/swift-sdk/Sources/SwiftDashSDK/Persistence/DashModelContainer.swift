import Foundation
import SwiftData

/// Factory for creating SwiftData model containers for Dash Platform persistence
public enum DashModelContainer {
    /// Every registered schema version's model list, parameterised on the
    /// one model whose shape differs between versions.
    ///
    /// Ordering is load-bearing only in the sense that it must not need to
    /// change: keeping `assetLock` in the slot the live `PersistentAssetLock`
    /// occupied means a frozen version's list is positionally identical to
    /// what that version shipped.
    private static func allModelTypes(
        assetLock: any PersistentModel.Type
    ) -> [any PersistentModel.Type] {
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
            assetLock,
            PersistentInvitation.self,
            PersistentMasternode.self
        ]
    }


    /// The V1/V2/V3 model set: frozen copies for every model in the
    /// relationship component (see `DashSchemaFrozenModels.swift`), live
    /// types for the eleven models outside it, and `assetLock` for the one
    /// model whose shape differs between V2 and V3.
    ///
    /// Positionally identical to `allModelTypes` — a released version's
    /// list must describe exactly the entities that version shipped.
    private static func componentFrozenModelTypes(
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
            PersistentPlatformAddressesSyncState.self,
            DashSchemaV1.PersistentWallet.self,
            DashSchemaV1.PersistentAccount.self,
            DashSchemaV1.PersistentCoreAddress.self,
            DashSchemaV1.PersistentTransaction.self,
            DashSchemaV1.PersistentTxo.self,
            DashSchemaV1.PersistentPendingInput.self,
            PersistentWalletManagerMetadata.self,
            PersistentShieldedNote.self,
            PersistentShieldedOutgoingNote.self,
            PersistentShieldedSyncState.self,
            PersistentShieldedActivity.self,
            PersistentShieldedViewingKey.self,
            assetLock,
            PersistentInvitation.self,
            PersistentMasternode.self
        ]
    }

    /// The exact model set registered as schema V1. Keep frozen: staged
    /// migration identifies an existing store by this schema's checksum, so
    /// this list may only reference models whose shape is frozen (see
    /// `DashSchemaFrozenModels.swift`).
    fileprivate static var v1ModelTypes: [any PersistentModel.Type] {
        componentFrozenModelTypes(assetLock: DashSchemaV1.PersistentAssetLock.self)
    }

    /// The exact model set registered as schema V2 — V1 plus
    /// `PersistentTrackedMasternode`. Frozen for the same reason as
    /// `v1ModelTypes`.
    fileprivate static var v2ModelTypes: [any PersistentModel.Type] {
        v1ModelTypes + [PersistentTrackedMasternode.self]
    }

    /// The exact model set registered as schema V3 — V2's frozen component
    /// with the LIVE `PersistentAssetLock`, which is the only model V3
    /// changed. Frozen for the same reason as `v1ModelTypes`.
    fileprivate static var v3ModelTypes: [any PersistentModel.Type] {
        componentFrozenModelTypes(assetLock: PersistentAssetLock.self)
            + [PersistentTrackedMasternode.self]
    }

    /// All persistent model types in the current Dash SDK schema (V4).
    /// Unlike the lists above this one tracks the LIVE models, so it moves
    /// whenever a model gains a property — which is exactly why the
    /// released versions must not.
    public static var modelTypes: [any PersistentModel.Type] {
        allModelTypes(assetLock: PersistentAssetLock.self) + [PersistentTrackedMasternode.self]
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

        // Always wire the migration plan so stores created by an older SDK
        // advance through the registered versioned schemas.
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
///   - `PersistentTxo` gained the optional `supersededByTxid`, and
///     `PersistentPendingInput` gained `isSweptTombstone` (defaulted
///     `false`). Together they let a sweep's claim on an input whose
///     funding TXO hasn't arrived yet survive the loser transaction's
///     deletion — previously that claim lived only on the doomed row's
///     `PersistentPendingInput`, which cascades away with it. Both
///     additive with defaults ⇒ lightweight migration; existing rows
///     migrate as ordinary (non-tombstone, non-superseded) entries.
///   - `PersistentPendingInput` gained the optional `winnerMinedHeight`
///     (a block-context sweep tombstone's finality stamp — the winner's
///     own mined height) and `PersistentWallet` gained the optional
///     `lastAppliedChainLockHeight` (the numeric chainlock watermark
///     delivered by `on_persist_wallet_changeset_chain_lock_height_fn`,
///     stored monotonic-max). Together they drive the bounded tombstone
///     lifetime: a tombstone is collected exactly when
///     `min(chainlockHeight, syncedHeight)` reaches its stamp. Both
///     optional ⇒ lightweight migration; pre-existing rows read as
///     unstamped (held forever) over a wallet with no boundary yet.
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

/// Version 4 adds the sweep columns: `isGloballySwept` on
/// `PersistentTransaction`, `supersededByTxid` on `PersistentTxo`,
/// `isSweptTombstone` / `winnerMinedHeight` on `PersistentPendingInput`,
/// and `lastAppliedChainLockHeight` on `PersistentWallet`. Every one is
/// additive with a default or optional, so a lightweight migration
/// preserves each existing row: transactions read as not swept, TXOs as
/// unsuperseded, pending inputs as ordinary unstamped claims, and a wallet
/// as having no chainlock boundary yet.
///
/// Registering it required freezing the whole relationship component those
/// four models sit in — see `DashSchemaFrozenModels.swift`.
public enum DashSchemaV4: VersionedSchema {
    public static var versionIdentifier: Schema.Version {
        Schema.Version(4, 0, 0)
    }

    public static var models: [any PersistentModel.Type] {
        DashModelContainer.modelTypes
    }
}
