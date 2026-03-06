import Foundation
import DashSDKFFI

/// Established Contact representing a bidirectional friendship in DashPay
public class EstablishedContact {
    internal let handle: Handle

    internal init(handle: Handle) {
        self.handle = handle
    }

    deinit {
        established_contact_destroy(handle)
    }

    /// Get the contact's identity ID
    public func getContactIdentityId() throws -> Identifier {
        var ffiId = IdentifierBytes(bytes: (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0))
        var error = PlatformWalletFFIError()

        let result = established_contact_get_contact_identity_id(handle, &ffiId, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return identifierFromFFI( ffiId)
    }

    /// Get the contact's alias
    public func getAlias() throws -> String? {
        var aliasPtr: UnsafeMutablePointer<CChar>? = nil
        var error = PlatformWalletFFIError()

        let result = established_contact_get_alias(handle, &aliasPtr, &error)

        if result == ErrorContactNotFound {
            return nil
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

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
    public func setAlias(_ alias: String) throws {
        var error = PlatformWalletFFIError()
        let aliasCStr = (alias as NSString).utf8String

        let result = established_contact_set_alias(handle, aliasCStr, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    /// Clear the contact's alias
    public func clearAlias() throws {
        var error = PlatformWalletFFIError()

        let result = established_contact_clear_alias(handle, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    /// Get the contact's note
    public func getNote() throws -> String? {
        var notePtr: UnsafeMutablePointer<CChar>? = nil
        var error = PlatformWalletFFIError()

        let result = established_contact_get_note(handle, &notePtr, &error)

        if result == ErrorContactNotFound {
            return nil
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

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
    public func setNote(_ note: String) throws {
        var error = PlatformWalletFFIError()
        let noteCStr = (note as NSString).utf8String

        let result = established_contact_set_note(handle, noteCStr, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    /// Clear the contact's note
    public func clearNote() throws {
        var error = PlatformWalletFFIError()

        let result = established_contact_clear_note(handle, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    /// Check if the contact is hidden
    public func isHidden() throws -> Bool {
        var hidden: Bool = false
        var error = PlatformWalletFFIError()

        let result = established_contact_is_hidden(handle, &hidden, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return hidden
    }

    /// Hide the contact
    public func hide() throws {
        var error = PlatformWalletFFIError()

        let result = established_contact_hide(handle, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    /// Unhide the contact
    public func unhide() throws {
        var error = PlatformWalletFFIError()

        let result = established_contact_unhide(handle, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }
}
