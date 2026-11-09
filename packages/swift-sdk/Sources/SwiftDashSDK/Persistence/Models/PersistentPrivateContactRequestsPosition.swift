import Foundation
import SwiftData

/// Where an identity reads DashPay's private contact request store from
/// next, so a relaunch resumes the private contact request sync instead of
/// reading the store from its start. Written in the same changeset round as
/// the contact requests read before it.
@Model
public final class PersistentPrivateContactRequestsPosition {
    #Unique<PersistentPrivateContactRequestsPosition>([\.networkRaw, \.walletId, \.identityId])
    public var networkRaw: UInt32
    public var walletId: Data
    public var identityId: Data
    // Bit pattern preserves the full unsigned FFI domain in a SQLite integer.
    public var position: Int64
    /// The fingerprint of the scanning keys `position` holds for; opaque
    /// here, handed back to the native wallet as it came.
    public var keyFingerprint: Data

    public init(networkRaw: UInt32, walletId: Data, identityId: Data,
                position: UInt64, keyFingerprint: Data) {
        self.networkRaw = networkRaw
        self.walletId = walletId
        self.identityId = identityId
        self.position = Int64(bitPattern: position)
        self.keyFingerprint = keyFingerprint
    }
}
