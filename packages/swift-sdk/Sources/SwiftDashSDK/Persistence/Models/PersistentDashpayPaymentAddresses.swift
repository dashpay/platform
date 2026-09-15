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

    static func removeOwned(in context: ModelContext, networkRaw: UInt32, ownerIdentityId: Data) throws {
        try context.delete(model: PersistentDashpayPaymentAddresses.self, where: #Predicate {
            $0.networkRaw == networkRaw && $0.ownerIdentityId == ownerIdentityId
        })
    }
}
