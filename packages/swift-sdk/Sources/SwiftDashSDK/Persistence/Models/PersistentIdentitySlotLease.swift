import Foundation
import SwiftData

/// The identity index an invitation claim holds before its identity lands,
/// keyed by the invitation's asset-lock outpoint. Written by the native
/// wallet before any key is derived at that index, so a claim retried after
/// a relaunch returns to the same slot and a registration never takes it.
/// Every field is opaque here and handed back as it came.
@Model
public final class PersistentIdentitySlotLease {
    #Unique<PersistentIdentitySlotLease>([\.networkRaw, \.walletId, \.invitationOutPoint])
    public var networkRaw: UInt32
    public var walletId: Data
    /// 36 bytes: raw txid, then the little-endian vout.
    public var invitationOutPoint: Data
    public var identityIndex: UInt32
    /// The native lease phase code (`IDENTITY_SLOT_LEASE_PHASE_*`).
    public var phase: UInt8

    public init(networkRaw: UInt32, walletId: Data, invitationOutPoint: Data,
                identityIndex: UInt32, phase: UInt8) {
        self.networkRaw = networkRaw
        self.walletId = walletId
        self.invitationOutPoint = invitationOutPoint
        self.identityIndex = identityIndex
        self.phase = phase
    }
}
