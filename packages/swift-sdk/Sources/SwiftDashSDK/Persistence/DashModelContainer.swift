import CoreData
import Foundation
import SwiftData

/// Why `DashModelContainer.open` refused to open a store. Typed so a host can
/// tell a store it must not touch from a store it cannot read, and say the
/// right thing to the user instead of surfacing SwiftData's opaque
/// `loadIssueModelContainer`.
public enum DashModelContainerError: LocalizedError, Equatable {
    /// The configuration was built from a `Schema` whose entity set differs
    /// from the SDK's. `unexpected` names entities the SDK schema lacks;
    /// `missing` names SDK entities the configuration lacks.
    case schemaMismatch(unexpected: [String], missing: [String])
    /// The store was written by a build with a newer schema than this SDK
    /// registers (`reason` says how that was detected). Opening it with
    /// inferred migration would silently drop what the newer build wrote,
    /// so `open` refuses; the only safe ways forward are a newer build or a
    /// wallet reset.
    case storeFromNewerBuild(reason: String)

    public var errorDescription: String? {
        switch self {
        case .schemaMismatch(let unexpected, let missing):
            return "The store configuration's schema does not match the SDK schema"
                + " (unexpected: \(unexpected.joined(separator: ", ")); missing: \(missing.joined(separator: ", ")))."
        case .storeFromNewerBuild:
            return "The wallet database on this device was written by a newer version of the app."
                + " This version cannot open it without losing data; update the app, or reset the wallet."
        }
    }
}

/// Factory for creating SwiftData model containers for Dash Platform persistence
public enum DashModelContainer {
    private struct StoreFileSizes {
        let main: UInt64
        let wal: UInt64
        let shm: UInt64

        /// Shares the exporter's saturating rule so a corrupt size can never
        /// trap and so both totals move together if that rule ever changes.
        var total: UInt64 {
            diagnosticSaturatingSum([main, wal, shm])
        }
    }

    /// Which of the two open attempts produced the result being reported.
    private enum StoreMigrationPath: String {
        /// `DashMigrationPlan` accepted the store.
        case staged
        /// The staged plan rejected the store and SwiftData's inferred
        /// lightweight migration was used instead — see `create`.
        case inferredFallback = "inferred_fallback"
    }

    /// What the store at `storeURL` is, relative to the schemas
    /// `DashMigrationPlan` registers.
    ///
    /// This is the question staged migration answers with Cocoa 134504 when
    /// it cannot place a store. It has to be asked here directly, because the
    /// error SwiftData surfaces for it is `SwiftDataError.loadIssueModelContainer`
    /// with no explanation and no underlying `NSError` — the same value a
    /// corrupt file produces — so nothing in the thrown error distinguishes
    /// the one failure the fallback may answer from every failure it must not.
    enum StoreSchemaVerdict: Equatable {
        /// The metadata could not be read: not a version question.
        case unreadable
        /// A registered schema is compatible with it. The staged plan can
        /// open it, so a failure to do so is something else entirely.
        case matchesRegisteredVersion
        /// Written by a registered version whose live models have since
        /// drifted, with exactly the drifted shapes `knownDriftedEntityHashes`
        /// lists (every v4.2.0-dev.1 store, until the remaining V1/V2 shapes
        /// are frozen). Inferred migration may open it.
        case driftedRegisteredVersion
        /// Positively written by a NEWER build: the store carries an entity
        /// this schema does not have, which nothing older could have created.
        /// Inferred migration would open it and silently drop that entity's
        /// table, so it must not run; the pre-fallback crash was the safe
        /// outcome here. This is the only verdict the host may turn into
        /// "update the app", because it is the only one whose evidence has a
        /// direction — see `unplaceable` for why a hash disagreement does not.
        case newerThanRegistered(reason: String)
        /// The metadata reads but does not place the store against any
        /// registered version: no declared version identifier, or nothing to
        /// compare it by. Inferred migration must not answer this either — an
        /// unplaced store opened by inference is trimmed to the current schema
        /// exactly like a downgrade — but it is NOT evidence of a newer build,
        /// so the host must not be told to update or reset. SwiftData's own
        /// error is passed through instead.
        case unplaceable(reason: String)

        var logLabel: String {
            switch self {
            case .unreadable: return "unreadable"
            case .matchesRegisteredVersion: return "matches_registered_version"
            case .driftedRegisteredVersion: return "drifted_registered_version"
            case .newerThanRegistered(let reason): return "newer_than_registered:\(reason)"
            case .unplaceable(let reason): return "unplaceable:\(reason)"
            }
        }
    }

    /// The exact per-entity version hashes a v4.2.0-dev.1 store carries for
    /// the two shapes changed in place since V1 — the reason such a store no
    /// longer matches V1's checksum although V1 wrote it.
    ///
    /// A store is "drifted" only if every entity whose hash disagrees with its
    /// declared version's model carries EXACTLY the hash listed here. The hash
    /// is a function of the shape, so any other shape of these two entities —
    /// a newer build's, with an added attribute — has a different hash and is
    /// refused, and so is a disagreement on any other entity. That is what
    /// makes the fallback answer precisely the store the fixture proves and
    /// nothing else: there is no "same name, unknown shape" residual left.
    ///
    /// `Dev1StoreUpgradeTests` pins these to the fixture. Extend only with a
    /// hash read from a real store of a supported prerelease; shrink as the
    /// shapes get frozen in `DashSchemaFrozenModels.swift`, after which the
    /// fallback has no case left to answer and can go.
    static let knownDriftedEntityHashes: [String: Data] = [
        "PersistentDocumentType": Data(base64Encoded: "w2iUSIQfuddeVRUyE/lgUE7oObvG1pWzO1Ah2IB2xBs=")!,
        "PersistentIndex": Data(base64Encoded: "iJRVIyu7GslKt5zO+2oa194YXGtujJqnzcAe1FPWWa8=")!,
    ]

    /// One registered version as the verdict sees it: its identifier and
    /// the per-entity hashes of the model built from its live types.
    struct RegisteredVersionHashes: Equatable {
        let identifier: String
        let entityHashes: [String: Data]
    }

    /// Classifies the store from its metadata alone; never opens it.
    static func classifyStore(at storeURL: URL) -> StoreSchemaVerdict {
        guard let metadata = try? NSPersistentStoreCoordinator.metadataForPersistentStore(
            ofType: NSSQLiteStoreType,
            at: storeURL,
            options: nil
        ) else { return .unreadable }

        let models = DashMigrationPlan.schemas.compactMap { schema -> (String, NSManagedObjectModel)? in
            NSManagedObjectModel.makeManagedObjectModel(for: schema.models)
                .map { (schema.versionIdentifier.description, $0) }
        }
        let matches = models.contains { _, model in
            model.isConfiguration(withName: nil, compatibleWithStoreMetadata: metadata)
        }
        return storeSchemaVerdict(
            matchesRegisteredVersion: matches,
            storeEntityHashes: (metadata[NSStoreModelVersionHashesKey] as? [String: Data]) ?? [:],
            storeVersionIdentifiers: (metadata[NSStoreModelVersionIdentifiersKey] as? [String]) ?? [],
            registered: models.map {
                RegisteredVersionHashes(identifier: $0.0, entityHashes: $0.1.entityVersionHashesByName)
            },
            currentEntities: Set(schema.entities.map(\.name))
        )
    }

    /// The decision behind `classifyStore`, on plain values so every branch
    /// can be tested without building a store for it. Four checks, in order:
    /// a registered version is compatible; the declared version identifier
    /// is one this plan registered; every entity is one the current schema
    /// has; and every entity disagreeing with the declared version's model
    /// carries the one hash `knownDriftedEntityHashes` lists for it. Only
    /// the last yields `driftedRegisteredVersion`; everything else that is
    /// not a match is refused.
    static func storeSchemaVerdict(
        matchesRegisteredVersion: Bool,
        storeEntityHashes: [String: Data],
        storeVersionIdentifiers: [String],
        registered: [RegisteredVersionHashes],
        currentEntities: Set<String>
    ) -> StoreSchemaVerdict {
        if matchesRegisteredVersion { return .matchesRegisteredVersion }

        // Nothing to place the store against — every schema failed to build a
        // model. Not a fact about the store at all.
        guard !registered.isEmpty else {
            return .unplaceable(reason: "no_registered_models")
        }

        // SwiftData writes each `VersionedSchema.versionIdentifier` into the
        // store. One this plan does not register places it nowhere, but says
        // nothing about which side is older: a pre-V1 store and a version
        // since dropped from `DashMigrationPlan.schemas` look exactly like a
        // future one from here.
        let registeredIdentifiers = Set(registered.map(\.identifier))
        if let unknown = storeVersionIdentifiers.first(where: { !registeredIdentifiers.contains($0) }) {
            return .unplaceable(reason: "unregistered_version_identifier=\(unknown)")
        }
        // No identifier at all places the store nowhere. Older stores and
        // stores with truncated metadata land here too, so this is not a
        // newer build and must not be reported to the user as one.
        guard !storeVersionIdentifiers.isEmpty else {
            return .unplaceable(reason: "no_version_identifier")
        }
        // Drift is a statement about hashes that disagree. With no hashes to
        // read there is nothing to disagree, and every check below would pass
        // vacuously — `disagreeing` empty, so `unknownShapes` empty, so
        // `driftedRegisteredVersion` from a comparison that never happened,
        // authorizing inferred migration over a store nothing is known about.
        guard !storeEntityHashes.isEmpty else {
            return .unplaceable(reason: "no_entity_hashes")
        }

        // An entity the current schema does not have can only have been
        // written by a newer build — this is the one asymmetric fact
        // available here, so it is the one verdict allowed to say "newer".
        // Inferred migration would drop its table.
        let unknownEntities = Set(storeEntityHashes.keys).subtracting(currentEntities).sorted()
        if !unknownEntities.isEmpty {
            return .newerThanRegistered(
                reason: "unknown_entities=\(unknownEntities.joined(separator: "|"))"
            )
        }

        // Same entity names, so which ones disagree with the version the
        // store declares — and is each disagreeing shape the one known drift?
        // A newer build that added an attribute keeps the name and the
        // identifier but not the hash, whether it touched one of the two
        // drifted entities or any other. Only the declared version's model
        // is a fair comparison: later versions legitimately differ.
        var unexpectedDrift: Set<String> = []
        for version in registered where storeVersionIdentifiers.contains(version.identifier) {
            let disagreeing = storeEntityHashes.filter { name, hash in
                version.entityHashes[name] != hash
            }
            let unknownShapes = Set(disagreeing.compactMap { name, hash in
                knownDriftedEntityHashes[name] == hash ? nil : name
            })
            // `!disagreeing.isEmpty` is the load-bearing half: drift is what
            // the fallback answers, and a store that agrees on every hash it
            // carries and still is not compatible differs by something these
            // hashes do not describe — an entity the store lacks entirely,
            // say. Whatever that is, it is not the drift the pinned hashes
            // authorize, so it does not get inferred migration.
            if !disagreeing.isEmpty, unknownShapes.isEmpty {
                return .driftedRegisteredVersion
            }
            unexpectedDrift.formUnion(unknownShapes)
        }
        guard !unexpectedDrift.isEmpty else {
            return .unplaceable(reason: "no_entity_disagreement")
        }
        // Deliberately NOT `newerThanRegistered`. A hash disagreement is
        // symmetric: it says the store's shape of that entity is not the live
        // model's, not which of the two came first. V1/V2/V3 are still
        // unfrozen (only `PersistentAssetLock` is frozen), so adding one
        // attribute to any live model makes every EXISTING store disagree on
        // that entity — an older store, reported as a newer one, with a reset
        // offered as the remedy. Direction needs evidence this comparison does
        // not have; until the models are frozen, the honest verdict is that
        // the store cannot be placed.
        return .unplaceable(
            reason: "unexpected_entity_drift=\(unexpectedDrift.sorted().joined(separator: "|"))"
        )
    }

    /// Builds the common payload for both sides of the container open. The
    /// outcome deliberately describes only what SwiftData tells us: opening an
    /// existing store may have included a migration, but this API does not
    /// expose whether one actually ran.
    private static func storeOpenFields(
        succeeded: Bool,
        existedBefore: Bool,
        migrationPath: StoreMigrationPath,
        startedAt: CFAbsoluteTime,
        sizeBefore: StoreFileSizes,
        sizeAfter: StoreFileSizes
    ) -> [String: SDKLogValue] {
        let elapsed = max(0, (CFAbsoluteTimeGetCurrent() - startedAt) * 1_000)
        let duration: UInt64
        if !elapsed.isFinite {
            duration = 0
        } else if elapsed >= Double(UInt64.max) {
            duration = UInt64.max
        } else {
            duration = UInt64(elapsed)
        }
        let openOutcome: String
        switch (succeeded, existedBefore) {
        case (true, true):
            openOutcome = "existing_store_opened"
        case (true, false):
            openOutcome = "new_store_created"
        case (false, true):
            openOutcome = "existing_store_open_or_migration_failed"
        case (false, false):
            openOutcome = "new_store_creation_failed"
        }

        return [
            "container_result": .publicText(succeeded ? "opened" : "open_failed"),
            "duration_ms": .unsignedInteger(duration),
            "migration_path": .publicText(migrationPath.rawValue),
            "result": .publicText(succeeded ? "success" : "failure"),
            "store_existed_before_open": .boolean(existedBefore),
            "store_main_size_bytes_after": .unsignedInteger(sizeAfter.main),
            "store_main_size_bytes_before": .unsignedInteger(sizeBefore.main),
            "store_open_outcome": .publicText(openOutcome),
            "store_shm_size_bytes_after": .unsignedInteger(sizeAfter.shm),
            "store_shm_size_bytes_before": .unsignedInteger(sizeBefore.shm),
            "store_size_bytes_after": .unsignedInteger(sizeAfter.total),
            "store_size_bytes_before": .unsignedInteger(sizeBefore.total),
            "store_wal_size_bytes_after": .unsignedInteger(sizeAfter.wal),
            "store_wal_size_bytes_before": .unsignedInteger(sizeBefore.wal),
        ]
    }

    /// SQLite's durable state can be mostly in the WAL immediately after an
    /// app kill, so the main file alone is not a useful corruption signal.
    /// Read only sizes and never include any component of the device path.
    private static func storeFileSizes(at storeURL: URL) -> StoreFileSizes {
        func fileSize(at url: URL) -> UInt64 {
            // A fresh URL each time: `resourceValues` bridges to `NSURL`,
            // which caches a value per instance, and the same `storeURL` is
            // read before and after the open. A cached size would report a
            // migration that grew the store as no growth at all.
            let fresh = URL(fileURLWithPath: url.path)
            guard let size = try? fresh.resourceValues(forKeys: [.fileSizeKey]).fileSize,
                  size >= 0
            else { return 0 }
            return UInt64(size)
        }

        return StoreFileSizes(
            main: fileSize(at: storeURL),
            wal: fileSize(at: URL(fileURLWithPath: storeURL.path + "-wal")),
            shm: fileSize(at: URL(fileURLWithPath: storeURL.path + "-shm"))
        )
    }

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
        return try open(
            ModelConfiguration(
                schema: schema,
                isStoredInMemoryOnly: false,
                allowsSave: true,
                groupContainer: groupContainer,
                cloudKitDatabase: cloudKit ? .automatic : .none
            )
        )
    }

    /// The instrumented store-opening path, parameterised on the configuration.
    ///
    /// Public so a host that builds its own `ModelConfiguration` — DashWallet
    /// does, with a per-network URL — gets the same `core_store_open_result`
    /// telemetry and the same narrowly-scoped migration fallback as `create`,
    /// instead of a bare `ModelContainer(for:configurations:)` that reports
    /// nothing. It is also what lets `Dev1StoreUpgradeTests` drive exactly the
    /// path that ships against a fixture store.
    ///
    /// The configuration contributes the store URL and options only. The
    /// container is always built for the SDK's own `schema`, because that is
    /// what `DashMigrationPlan` migrates toward; a configuration built from a
    /// different `Schema` is refused with ``DashModelContainerError`` rather
    /// than silently opened under the wrong one.
    public static func open(_ modelConfiguration: ModelConfiguration) throws -> ModelContainer {
        if let provided = modelConfiguration.schema {
            let providedNames = Set(provided.entities.map(\.name))
            let sdkNames = Set(schema.entities.map(\.name))
            guard providedNames == sdkNames else {
                throw DashModelContainerError.schemaMismatch(
                    unexpected: providedNames.subtracting(sdkNames).sorted(),
                    missing: sdkNames.subtracting(providedNames).sorted()
                )
            }
        }
        // Always wire the migration plan so stores created by an older SDK
        // advance through the registered versioned schemas. Record only
        // metadata about the store — never its device path.
        let storeURL = modelConfiguration.url
        let existedBefore = FileManager.default.fileExists(atPath: storeURL.path)
        let sizeBefore = storeFileSizes(at: storeURL)
        let started = CFAbsoluteTimeGetCurrent()

        func report(
            succeeded: Bool,
            migrationPath: StoreMigrationPath,
            error: Error? = nil,
            storeVerdict: StoreSchemaVerdict? = nil
        ) {
            var fields = storeOpenFields(
                succeeded: succeeded,
                existedBefore: existedBefore,
                migrationPath: migrationPath,
                startedAt: started,
                sizeBefore: sizeBefore,
                sizeAfter: storeFileSizes(at: storeURL)
            )
            if let storeVerdict {
                fields["store_verdict"] = .publicText(storeVerdict.logLabel)
            }
            SDKLogger.event(
                "core_store_open_result",
                category: .persistence,
                severity: succeeded ? .info : .error,
                fields: fields,
                error: error,
                redacting: [storeURL.path]
            )
        }

        do {
            let container = try ModelContainer(
                for: schema,
                migrationPlan: DashMigrationPlan.self,
                configurations: [modelConfiguration]
            )
            report(succeeded: true, migrationPath: .staged)
            return container
        } catch {
            // Staged migration matches a store by the CHECKSUM of each
            // registered `VersionedSchema`, and only `PersistentAssetLock` is
            // frozen so far (see `DashSchemaFrozenModels.swift`). Every other
            // V1/V2 model is still referenced live, so a shape that has drifted
            // since — `PersistentDocumentType` and `PersistentIndex` for the
            // v4.2.0-dev.1 stores `Dev1StoreUpgradeTests` pins — leaves the
            // real store matching no registered version, and the staged open
            // fails with Cocoa 134504 rather than migrating.
            //
            // Hosts turn that throw into `fatalError` at launch, so for THAT
            // failure retry the way they already open the store themselves:
            // the current schema with SwiftData's inferred lightweight
            // migration and no plan. The match is deliberately exact, and it
            // is made on the store rather than the error (see
            // `classifyStore`). Every stage in
            // `DashMigrationPlan` is `.lightweight` today, but the day a
            // custom stage lands, a failure inside it must surface — a store
            // that matches a registered version and still failed to open is
            // exactly that case, and falling back would reopen it without the
            // stage and stamp the current checksum on it, so the stage could
            // never run later. And a store written by a NEWER build — a
            // downgrade — would be opened by inference and silently trimmed
            // to this schema, which is worse than the crash it replaces.
            // Everything except "existing store, drifted registered version"
            // is therefore rethrown untouched, with the verdict in the log.
            let verdict: StoreSchemaVerdict = existedBefore
                ? Self.classifyStore(at: storeURL)
                : .unreadable
            guard case .driftedRegisteredVersion = verdict else {
                report(succeeded: false, migrationPath: .staged, error: error, storeVerdict: verdict)
                // A newer build's store is the one refusal the host can act
                // on, so it gets a typed error — and only it, because that
                // error's text tells the user to update the app or reset the
                // wallet, and resetting is destructive on a store that is
                // merely unplaceable. `.unplaceable` and `.unreadable` are
                // SwiftData's own failure, passed through untouched with the
                // verdict in the log.
                if case .newerThanRegistered(let reason) = verdict {
                    throw DashModelContainerError.storeFromNewerBuild(reason: reason)
                }
                throw error
            }
            SDKLogger.event(
                "core_store_staged_migration_failed",
                category: .persistence,
                severity: .warning,
                fields: [
                    "store_existed_before_open": .boolean(existedBefore),
                    "store_verdict": .publicText(verdict.logLabel),
                ],
                error: error,
                redacting: [storeURL.path]
            )
            do {
                let container = try ModelContainer(
                    for: schema,
                    configurations: [modelConfiguration]
                )
                report(succeeded: true, migrationPath: .inferredFallback)
                return container
            } catch let fallbackError {
                report(
                    succeeded: false,
                    migrationPath: .inferredFallback,
                    error: fallbackError
                )
                throw fallbackError
            }
        }
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
/// Registering it required freezing the whole relationship component those
/// three models sit in — see `DashSchemaFrozenModels.swift`.
public enum DashSchemaV4: VersionedSchema {
    public static var versionIdentifier: Schema.Version {
        Schema.Version(4, 0, 0)
    }

    public static var models: [any PersistentModel.Type] {
        DashModelContainer.modelTypes
    }
}
