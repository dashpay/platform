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
    /// Whether a private contact request the identity paid for may still be
    /// unresolved; handed back as it came. Loaded set, Rust holds the
    /// identity's public contact requests until its private payments are
    /// restored from the shielded store.
    public var privatePaymentsUnresolved: Bool = false

    public init(networkRaw: UInt32, walletId: Data, identityId: Data,
                position: UInt64, keyFingerprint: Data, privatePaymentsUnresolved: Bool = false) {
        self.networkRaw = networkRaw
        self.walletId = walletId
        self.identityId = identityId
        self.position = Int64(bitPattern: position)
        self.keyFingerprint = keyFingerprint
        self.privatePaymentsUnresolved = privatePaymentsUnresolved
    }
}
