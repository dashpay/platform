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
            PersistentMasternode.self
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
        DashModelContainer.modelTypes
    }
}
