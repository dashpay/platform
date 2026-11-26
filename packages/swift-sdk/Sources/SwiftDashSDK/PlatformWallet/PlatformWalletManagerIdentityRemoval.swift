import Foundation
import DashSDKFFI

extension PlatformWalletManager {
    /// Remove `identityId` from every running wallet that manages it, so no
    /// later sync stores it again. Nothing is broadcast and the identity
    /// stays on Platform.
    ///
    /// Call it before deleting the host's own copy of the identity (its
    /// SwiftData rows, then its Keychain items). Rust decides whether the
    /// identity can go: while its DashPay contact request channel is held it
    /// throws `PlatformWalletError.privateContactRequestPending`,
    /// `.publicContactRequestUnconfirmed` or
    /// `.privateContactPaymentsNotRestored`, nothing was removed, and the
    /// host should delete nothing.
    ///
    /// Accepted, each wallet that held the identity emits the removal through
    /// the persistence handler, which deletes the identity's rows. The
    /// Keychain items are the host's to delete, so read what identifies them
    /// before this call.
    ///
    /// - Returns: `true` when at least one wallet managed the identity and
    ///   removed it, `false` when none did (an identity the host loaded on
    ///   its own), in which case its rows are still there.
    @discardableResult
    public func removeIdentityFromLiveWallets(identityId: Identifier) throws -> Bool {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        guard identityId.count == 32 else {
            throw PlatformWalletError.invalidIdentifier(
                "identityId must be 32 bytes, got \(identityId.count)"
            )
        }
        var wasManaged = false
        try identityId.withUnsafeBytes { raw in
            guard let base = raw.baseAddress?.assumingMemoryBound(to: FFIByteTuple32.self) else {
                throw PlatformWalletError.nullPointer(
                    "identity_id buffer base address was nil"
                )
            }
            try platform_wallet_manager_remove_identity_from_device(
                handle, base, &wasManaged
            ).check()
        }
        return wasManaged
    }
}
