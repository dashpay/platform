import Foundation
import DashSDKFFI

/// Identity Manager for managing Platform identities.
///
/// All FFI calls go through pointer-passing for identifiers — see the
/// `Identifier.withFFIBytes` extension. Out-buffer reads use a local
/// `[UInt8](repeating:count:32)` and copy into a `Data` value at
/// return time.
public class IdentityManager {
    internal let handle: Handle

    internal init(handle: Handle) {
        self.handle = handle
    }

    deinit {
        _ = identity_manager_destroy(handle)
    }

    /// Create a new empty Identity Manager
    public static func create() throws -> IdentityManager {
        var handle: Handle = NULL_HANDLE
        var error = PlatformWalletFFIError()

        let result = identity_manager_create(&handle, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return IdentityManager(handle: handle)
    }

    /// Add an identity to the manager
    public func addIdentity(_ identity: ManagedIdentity) throws {
        var error = PlatformWalletFFIError()

        let result = identity_manager_add_identity(handle, identity.handle, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    /// Remove an identity from the manager
    public func removeIdentity(_ identityId: Identifier) throws {
        var error = PlatformWalletFFIError()
        let result = identityId.withFFIBytes { idPtr in
            identity_manager_remove_identity(handle, idPtr, &error)
        }
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    /// Get an identity by ID
    public func getIdentity(_ identityId: Identifier) throws -> ManagedIdentity {
        var identityHandle: Handle = NULL_HANDLE
        var error = PlatformWalletFFIError()

        let result = identityId.withFFIBytes { idPtr in
            identity_manager_get_identity(handle, idPtr, &identityHandle, &error)
        }
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return ManagedIdentity(handle: identityHandle)
    }

    /// Get all identity IDs
    public func getAllIdentityIds() throws -> [Identifier] {
        var array = IdentifierArray(items: nil, count: 0)
        var error = PlatformWalletFFIError()

        let result = identity_manager_get_all_identity_ids(handle, &array, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        defer {
            platform_wallet_identifier_array_free(&array)
        }

        guard array.items != nil, array.count > 0 else {
            return []
        }

        var identifiers: [Identifier] = []
        identifiers.reserveCapacity(array.count)
        for i in 0..<array.count {
            identifiers.append(identifierFromFFIArray(array, at: i))
        }

        return identifiers
    }

    // Primary-identity selection lives on the Swift UI layer now —
    // the Rust `IdentityManager` no longer carries the field. Callers
    // should track the user's pick in their own state (e.g. an
    // `@AppStorage` or `WalletDataModel.selectedIdentityId`) and look
    // up the matching `ManagedIdentity` via `getIdentity(_:)`.

    /// Get the count of identities
    public func getIdentityCount() throws -> Int {
        var count: Int = 0
        var error = PlatformWalletFFIError()

        let result = identity_manager_get_identity_count(handle, &count, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return count
    }
}
