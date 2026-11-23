import Foundation
import SwiftData

/// Raw payment addresses for an owned or cached contact profile. Keeping this
/// metadata independent preserves the released profile/identity relationship schema.
@Model
public final class PersistentDashpayPaymentAddresses {
    #Unique<PersistentDashpayPaymentAddresses>([\.networkRaw, \.ownerIdentityId, \.profileIdentityId])

    public var networkRaw: UInt32
    public var ownerIdentityId: Data
    public var profileIdentityId: Data
    public var corePaymentAddress: Data?
    public var platformPaymentAddress: Data?
    public var shieldedAddress: Data?

    public init(networkRaw: UInt32, ownerIdentityId: Data, profileIdentityId: Data,
                corePaymentAddress: Data? = nil, platformPaymentAddress: Data? = nil,
                shieldedAddress: Data? = nil) {
        self.networkRaw = networkRaw
        self.ownerIdentityId = ownerIdentityId
        self.profileIdentityId = profileIdentityId
        self.corePaymentAddress = corePaymentAddress
        self.platformPaymentAddress = platformPaymentAddress
        self.shieldedAddress = shieldedAddress
    }

    static func fetch(in context: ModelContext, networkRaw: UInt32,
                      ownerIdentityId: Data, profileIdentityId: Data, fetcher: any ModelFetching = LiveModelFetcher()) throws -> PersistentDashpayPaymentAddresses? {
        let query = FetchDescriptor<PersistentDashpayPaymentAddresses>(predicate: #Predicate {
            $0.networkRaw == networkRaw && $0.ownerIdentityId == ownerIdentityId && $0.profileIdentityId == profileIdentityId
        })
        return try fetcher.fetch(query, in: context).first
    }

    /// The row for a profile's read-only address accessors, which cannot throw.
    /// A failed read is logged before it is reported as `nil`, so the log tells
    /// "the store could not be read" from "no address is stored".
    static func fetchForDisplay(in context: ModelContext, networkRaw: UInt32,
                                ownerIdentityId: Data, profileIdentityId: Data) -> PersistentDashpayPaymentAddresses? {
        do {
            return try fetch(in: context, networkRaw: networkRaw,
                             ownerIdentityId: ownerIdentityId, profileIdentityId: profileIdentityId)
        } catch {
            SDKLogger.event("persistence_profile_addresses_read_failed", category: .persistence,
                severity: .error, error: error)
            return nil
        }
    }

    /// Called within the persister's transaction; empty metadata removes the row.
    static func replace(in context: ModelContext, networkRaw: UInt32,
                        ownerIdentityId: Data, profileIdentityId: Data,
                        core: Data?, platform: Data?, shielded: Data?,
                        fetcher: any ModelFetching = LiveModelFetcher()) throws {
        let existing = try fetch(in: context, networkRaw: networkRaw,
                             ownerIdentityId: ownerIdentityId, profileIdentityId: profileIdentityId, fetcher: fetcher)
        if core == nil && platform == nil && shielded == nil {
            if let existing { context.delete(existing) }
            return
        }
        let row = existing ?? PersistentDashpayPaymentAddresses(networkRaw: networkRaw,
            ownerIdentityId: ownerIdentityId, profileIdentityId: profileIdentityId)
        row.corePaymentAddress = core
        row.platformPaymentAddress = platform
        row.shieldedAddress = shielded
        if existing == nil { context.insert(row) }
    }

    /// Delete every address row an identity owns (its own profile's and its
    /// cached contacts'). Rows are keyed by id, not by relationship, so any
    /// path that deletes an identity must call this.
    ///
    /// The rows are fetched and marked for deletion one by one, so the deletes
    /// land with the caller's next `save()` and `rollback()` undoes them. A
    /// batch `context.delete(model:where:)` would write to the store at once
    /// and survive a rollback, leaving the identity without its addresses
    /// when the save that should delete the identity fails.
    public static func removeOwned(in context: ModelContext, networkRaw: UInt32, ownerIdentityId: Data) throws {
        let owned = try context.fetch(FetchDescriptor<PersistentDashpayPaymentAddresses>(predicate: #Predicate {
            $0.networkRaw == networkRaw && $0.ownerIdentityId == ownerIdentityId
        }))
        for row in owned {
            context.delete(row)
        }
    }
}
