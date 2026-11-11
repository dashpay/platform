import Foundation
import SwiftData

/// The shielded address a private contact's tracked request to one of the
/// wallet's identities carries, kept so a relaunch has it without reading
/// the private contact request store again.
@Model
public final class PersistentPrivateShieldedAddress {
    #Unique<PersistentPrivateShieldedAddress>([\.networkRaw, \.walletId, \.ownerId, \.contactId])
    public var networkRaw: UInt32
    public var walletId: Data
    /// The wallet-owned identity the request is to.
    public var ownerId: Data
    /// The contact who sent the request.
    public var contactId: Data
    /// The raw 43-byte Orchard payment address.
    public var address: Data

    public init(networkRaw: UInt32, walletId: Data, ownerId: Data, contactId: Data, address: Data) {
        self.networkRaw = networkRaw
        self.walletId = walletId
        self.ownerId = ownerId
        self.contactId = contactId
        self.address = address
    }
}
