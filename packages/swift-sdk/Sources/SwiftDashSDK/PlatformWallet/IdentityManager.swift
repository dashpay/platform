import Foundation
import DashSDKFFI

/// Identity Manager for managing Platform identities
public class IdentityManager {
    internal let handle: Handle

    internal init(handle: Handle) {
        self.handle = handle
    }

    deinit {
        identity_manager_destroy(handle)
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
        var ffiId = identifierToFFI(identityId)

        let result = identity_manager_remove_identity(handle, ffiId, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    /// Get an identity by ID
    public func getIdentity(_ identityId: Identifier) throws -> ManagedIdentity {
        var identityHandle: Handle = NULL_HANDLE
        var error = PlatformWalletFFIError()
        var ffiId = identifierToFFI(identityId)

        let result = identity_manager_get_identity(handle, ffiId, &identityHandle, &error)
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
            platform_wallet_identifier_array_free(array)
        }

        guard let items = array.items else {
            return []
        }

        var identifiers: [Identifier] = []
        for i in 0..<array.count {
            let ffiId = items[Int(i)]
            identifiers.append(identifierFromFFI( ffiId))
        }

        return identifiers
    }

    /// Get the primary identity ID
    public func getPrimaryIdentityId() throws -> Identifier? {
        var ffiId = IdentifierBytes(bytes: (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0))
        var error = PlatformWalletFFIError()

        let result = identity_manager_get_primary_identity_id(handle, &ffiId, &error)

        if result == ErrorIdentityNotFound {
            return nil
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return identifierFromFFI( ffiId)
    }

    /// Set the primary identity
    public func setPrimaryIdentity(_ identityId: Identifier) throws {
        var error = PlatformWalletFFIError()
        var ffiId = identifierToFFI(identityId)

        let result = identity_manager_set_primary_identity(handle, ffiId, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

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
