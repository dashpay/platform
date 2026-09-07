import Foundation
import SwiftData

// MARK: - Frozen model definitions for already-released schema versions
//
// A `VersionedSchema` identifies a store by the CHECKSUM of the entities it
// declares, not by the identity of the Swift type list. So a registered
// schema version may only ever reference model types whose *shape* is frozen
// at the moment that version shipped. Pointing `DashSchemaVN.models` at a
// live `@Model` type means the next property added to that type silently
// mutates version N's checksum in place: a store written by the previously
// released binary then matches no schema in `DashMigrationPlan.schemas`, and
// `ModelContainer(for:migrationPlan:configurations:)` fails to open it with
// Cocoa error 134504 ("Cannot use staged migration with an unknown model
// version") instead of migrating it.
//
// A frozen copy must be a *nested* type, because SwiftData derives the entity
// name from the unqualified type name — `DashSchemaV1.PersistentAssetLock`
// and the top-level `PersistentAssetLock` are two distinct Swift types that
// both describe the entity named "PersistentAssetLock", which is exactly
// what lets a migration stage map one onto the other.
//
// ## Scope of the freeze
//
// Every model a released version registers is frozen — the whole graph, not
// only the models whose shape has changed since. SwiftData resolves a
// relationship's destination by entity name, binding that name to the first
// Swift type that claims it in the process, so a frozen `PersistentWallet`
// referenced from a live `PersistentAccount.wallet` would be hashed with
// the LIVE wallet's shape whenever the live schema was built first (which
// `DashModelContainer.create` always does). Freezing only the changed
// models therefore cannot keep a released checksum stable; freezing the
// graph can, and `DashModelMigrationTests` proves it against stores the
// older builds actually wrote.
//
// The frozen graph lives under `FrozenSchemas/`, one file per model,
// generated from the live models at the commit that shipped the version by
// `scripts/freeze_schema_models.py` (rerun it against the same commit to
// verify a file). V1, V2 and V3 share one copy of every model whose shape
// did not change between them; V2 adds `PersistentTrackedMasternode` and V3
// swaps the asset lock for the shape that gained `recipientIsExternal`.
// The value types a model stores inline are part of the frozen surface
// too: SwiftData expands a stored Codable struct into composite attributes
// of the owning entity (`PersistentToken`'s `ChangeControlRules` columns,
// for one), so `FrozenSchemas/DashSchemaV1+TokenTypes.swift` carries the
// nested copies the frozen models resolve to, and the live
// `Persistence/Types/TokenTypes.swift` may change freely.
//
// The one hand-written copy below is the asset lock as V1 and V2 shipped
// it: that shape predates every commit the generator could be pointed at,
// so it cannot be regenerated. V1's own checksum has already drifted from
// what actually shipped as V1 (see the `DashSchemaV1` doc comment: several
// models were changed in place while V1 was the only registered version,
// and dev stores at V1 are knowingly expected to fail open and be rebuilt).

extension DashSchemaV1 {
    /// `PersistentAssetLock` frozen at the shape it had when schema V2
    /// shipped — i.e. everything the live model has today EXCEPT
    /// `recipientIsExternal`, which is what V3 adds.
    ///
    /// Referenced by both `DashSchemaV1.models` and `DashSchemaV2.models`.
    /// Do not add properties here and do not "fix" its doc comments to
    /// match the live model: every attribute, its optionality, its default
    /// value, the `@Attribute(.unique)` marker and the `#Index` are all
    /// inputs to the V2 checksum, and changing any of them re-breaks the
    /// V2 stores this type exists to keep openable. Doc comments are not
    /// inputs to the checksum, but keeping them minimal here keeps the
    /// live model the single place worth reading.
    ///
    /// See the live ``SwiftDashSDK/PersistentAssetLock`` for what each
    /// column means.
    @Model
    final class PersistentAssetLock {
        #Index<PersistentAssetLock>([\.walletId])

        @Attribute(.unique) var outPointHex: String
        var walletId: Data
        var transactionBytes: Data
        var fundingTypeRaw: Int
        var identityIndexRaw: Int32
        var accountIndexRaw: Int32 = 0
        var amountDuffs: Int64
        var statusRaw: Int
        var proofBytes: Data?
        var recipientPlatformAddressHash: Data?
        var recipientPlatformAddressType: UInt8?
        var createdAt: Date
        var updatedAt: Date

        init(
            outPointHex: String,
            walletId: Data,
            transactionBytes: Data,
            fundingTypeRaw: Int,
            identityIndexRaw: Int32,
            accountIndexRaw: Int32 = 0,
            amountDuffs: Int64,
            statusRaw: Int,
            proofBytes: Data? = nil
        ) {
            self.outPointHex = outPointHex
            self.walletId = walletId
            self.transactionBytes = transactionBytes
            self.fundingTypeRaw = fundingTypeRaw
            self.identityIndexRaw = identityIndexRaw
            self.accountIndexRaw = accountIndexRaw
            self.amountDuffs = amountDuffs
            self.statusRaw = statusRaw
            self.proofBytes = proofBytes
            self.createdAt = Date()
            self.updatedAt = Date()
        }
    }
}
