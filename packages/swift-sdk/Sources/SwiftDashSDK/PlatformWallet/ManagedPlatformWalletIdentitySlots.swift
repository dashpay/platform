import Foundation
import DashSDKFFI

// MARK: - Identity slots
//
// The identity index a registration or an invitation claim derives its keys
// at is chosen by the wallet (`IdentityWallet::acquire_identity_slot`) and
// leased to the operation before any key is derived. These wrappers marshal
// the lease in and out; they choose nothing. The host derives the keys at the
// returned index and passes the slot back with them.

extension ManagedPlatformWallet {
    /// A slot leased to a new identity registration.
    public struct RegistrationSlot: Sendable, Equatable {
        /// The index to derive the identity keys at.
        public let identityIndex: UInt32
        /// The lease's token: pass the slot to `registerIdentityWithFunding`,
        /// `registerIdentityFromAddresses` or
        /// `PlatformWalletManager.shieldedIdentityCreateFromPool` (or the
        /// token to `resumeIdentityWithAssetLock` when reclaiming an
        /// invitation voucher as a new identity), or to
        /// `releaseRegistrationSlot` when the registration is abandoned.
        public let token: UInt64
    }

    /// A slot leased to the claim of one invitation.
    public struct InvitationClaimSlot: Sendable, Equatable {
        /// The index to derive the identity keys at; pass it to
        /// `claimInvitation`.
        public let identityIndex: UInt32
        /// The invitation's funding outpoint (32-byte raw txid, then the
        /// 4-byte little-endian vout) that keys the lease.
        public let invitationOutPoint: Data
        /// An earlier attempt of this claim may have reached Platform.
        public let submitted: Bool
    }

    /// How many times in a row a host acquires a new slot after
    /// `PlatformWalletError.identitySlotTakenOnChain` before giving up.
    public static let identitySlotTakenRetryLimit = Int(IDENTITY_SLOT_TAKEN_RETRY_LIMIT)

    /// Lease an identity slot for a new registration. The lease lives in
    /// memory only; Rust frees it when the registration lands, or when the
    /// registration finds the slot taken on chain
    /// (`.identitySlotTakenOnChain`). A shielded-pool create left
    /// unconfirmed is held by the wallet itself, across a restart too.
    /// Release it with `releaseRegistrationSlot` when the registration is
    /// abandoned.
    public func acquireRegistrationSlot() async throws -> RegistrationSlot {
        // `self` stays alive for the whole call: a deinit mid-call would
        // destroy the handle the FFI is still using.
        try await Task.detached(priority: .userInitiated) { [self] () -> RegistrationSlot in
            try withExtendedLifetime(self) {
                var identityIndex: UInt32 = 0
                var token: UInt64 = 0
                try platform_wallet_identity_slot_acquire_registration(
                    handle,
                    &identityIndex,
                    &token
                ).check()
                return RegistrationSlot(identityIndex: identityIndex, token: token)
            }
        }.value
    }

    /// Lease the identity slot for a claim of the invitation link `uri`. A
    /// claim retried gets the slot its earlier attempt leased. Costs one
    /// funding-tx fetch; the lease is stored durably, and a persister that
    /// cannot keep it refuses the claim.
    public func acquireInvitationClaimSlot(uri: String) async throws -> InvitationClaimSlot {
        try await Task.detached(priority: .userInitiated) { [self] () -> InvitationClaimSlot in
            try withExtendedLifetime(self) {
                var identityIndex: UInt32 = 0
                var outPoint = Self.emptyOutPoint
                var submitted = false
                let result = uri.withCString { uriPtr in
                    platform_wallet_identity_slot_acquire_invitation_claim(
                        handle,
                        uriPtr,
                        &identityIndex,
                        &outPoint,
                        &submitted
                    )
                }
                try result.check()
                return InvitationClaimSlot(
                    identityIndex: identityIndex,
                    invitationOutPoint: Swift.withUnsafeBytes(of: outPoint) { Data($0) },
                    submitted: submitted
                )
            }
        }.value
    }

    /// Give up the slot of an abandoned registration. A slot no longer
    /// held (the registration landed, or found the slot taken) is not an
    /// error.
    public func releaseRegistrationSlot(_ slot: RegistrationSlot) async throws {
        try await Task.detached(priority: .userInitiated) { [self] in
            try withExtendedLifetime(self) {
                try platform_wallet_identity_slot_release_registration(handle, slot.token).check()
            }
        }.value
    }

    /// Give up the slot of an invitation claim abandoned before it was
    /// sent. Refused with `PlatformWalletError.identitySlotClaimSubmitted`
    /// when the claim may have reached Platform: its outcome frees the slot
    /// then, so a caller keeps the slot on that refusal.
    public func releaseInvitationClaimSlot(_ slot: InvitationClaimSlot) async throws {
        guard slot.invitationOutPoint.count == 36 else {
            throw PlatformWalletError.invalidParameter(
                "invitationOutPoint must be exactly 36 bytes (was \(slot.invitationOutPoint.count))"
            )
        }
        try await Task.detached(priority: .userInitiated) { [self] in
            try withExtendedLifetime(self) {
                var outPoint = Self.emptyOutPoint
                Swift.withUnsafeMutableBytes(of: &outPoint) { dst in
                    slot.invitationOutPoint.withUnsafeBytes { dst.copyMemory(from: $0) }
                }
                try platform_wallet_identity_slot_release_invitation_claim(handle, &outPoint).check()
            }
        }.value
    }

    private static var emptyOutPoint: (
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    ) {
        (
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
        )
    }
}
