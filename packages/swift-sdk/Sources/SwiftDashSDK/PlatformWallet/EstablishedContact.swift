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

    /// Set the contact's alias
    @available(*, deprecated, message: "Mutates a detached FFI clone — the change never reaches wallet state, is never persisted, and is silently lost when the handle is freed. Use ManagedPlatformWallet.setDashPayContactInfo(identityId:contactId:alias:note:hidden:signer:), which writes real state and publishes the encrypted contactInfo document.")
    public func setAlias(_ alias: String) throws {
        let aliasCStr = (alias as NSString).utf8String
        try established_contact_set_alias(handle, aliasCStr).check()
    }

    /// Clear the contact's alias
    @available(*, deprecated, message: "Mutates a detached FFI clone — the change never reaches wallet state, is never persisted, and is silently lost when the handle is freed. Use ManagedPlatformWallet.setDashPayContactInfo(identityId:contactId:alias:note:hidden:signer:), which writes real state and publishes the encrypted contactInfo document.")
    public func clearAlias() throws {
        try established_contact_clear_alias(handle).check()
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

    /// Set the contact's note
    @available(*, deprecated, message: "Mutates a detached FFI clone — the change never reaches wallet state, is never persisted, and is silently lost when the handle is freed. Use ManagedPlatformWallet.setDashPayContactInfo(identityId:contactId:alias:note:hidden:signer:), which writes real state and publishes the encrypted contactInfo document.")
    public func setNote(_ note: String) throws {
        let noteCStr = (note as NSString).utf8String
        try established_contact_set_note(handle, noteCStr).check()
    }

    /// Clear the contact's note
    @available(*, deprecated, message: "Mutates a detached FFI clone — the change never reaches wallet state, is never persisted, and is silently lost when the handle is freed. Use ManagedPlatformWallet.setDashPayContactInfo(identityId:contactId:alias:note:hidden:signer:), which writes real state and publishes the encrypted contactInfo document.")
    public func clearNote() throws {
        try established_contact_clear_note(handle).check()
    }

    /// Check if the contact is hidden
    public func isHidden() throws -> Bool {
        var hidden: Bool = false
        try established_contact_is_hidden(handle, &hidden).check()
        return hidden
    }

    /// Hide the contact
    @available(*, deprecated, message: "Mutates a detached FFI clone — the change never reaches wallet state, is never persisted, and is silently lost when the handle is freed. Use ManagedPlatformWallet.setDashPayContactInfo(identityId:contactId:alias:note:hidden:signer:), which writes real state and publishes the encrypted contactInfo document.")
    public func hide() throws {
        try established_contact_hide(handle).check()
    }

    /// Unhide the contact
    @available(*, deprecated, message: "Mutates a detached FFI clone — the change never reaches wallet state, is never persisted, and is silently lost when the handle is freed. Use ManagedPlatformWallet.setDashPayContactInfo(identityId:contactId:alias:note:hidden:signer:), which writes real state and publishes the encrypted contactInfo document.")
    public func unhide() throws {
        try established_contact_unhide(handle).check()
    }
}
