import Foundation
import DashSDKFFI

/// Established Contact representing a bidirectional friendship in DashPay.
///
/// `@unchecked Sendable`: immutable `Handle` + Rust-side lock —
/// same pattern as `ContactRequest` / `ManagedPlatformWallet`.
public final class EstablishedContact: @unchecked Sendable {
    internal let handle: Handle

    internal init(handle: Handle) {
        self.handle = handle
    }

    deinit {
        established_contact_destroy(handle).discard()
    }

    /// Get the contact's identity ID
    public func getContactIdentityId() throws -> Identifier {
        var buf = [UInt8](repeating: 0, count: 32)
        try buf.withUnsafeMutableBufferPointer { bp in
            try established_contact_get_contact_identity_id(handle, bp.baseAddress!).check()
        }
        return Data(buf)
    }

    /// Get the contact's alias.
    ///
    /// Returns `nil` when the contact has no alias set; the Rust side
    /// surfaces that as `NotFound` (the inner `Option<String>` is
    /// `None`), which is distinct from a hard error.
    public func getAlias() throws -> String? {
        var aliasPtr: UnsafeMutablePointer<CChar>? = nil
        let result = PlatformWalletResult(
            established_contact_get_alias(handle, &aliasPtr)
        )

        if result.code == .notFound {
            return nil
        }
        try result.throwIfError()

        defer {
            if let ptr = aliasPtr {
                platform_wallet_string_free(ptr)
            }
        }

        guard let ptr = aliasPtr else {
            return nil
        }

        return String(cString: ptr)
    }

    /// Get the contact's note. Returns `nil` when no note is set —
    /// see `getAlias()` for the `NotFound` rationale.
    public func getNote() throws -> String? {
        var notePtr: UnsafeMutablePointer<CChar>? = nil
        let result = PlatformWalletResult(
            established_contact_get_note(handle, &notePtr)
        )

        if result.code == .notFound {
            return nil
        }
        try result.throwIfError()

        defer {
            if let ptr = notePtr {
                platform_wallet_string_free(ptr)
            }
        }

        guard let ptr = notePtr else {
            return nil
        }

        return String(cString: ptr)
    }

    /// Check if the contact is hidden
    public func isHidden() throws -> Bool {
        var hidden: Bool = false
        try established_contact_is_hidden(handle, &hidden).check()
        return hidden
    }
}
