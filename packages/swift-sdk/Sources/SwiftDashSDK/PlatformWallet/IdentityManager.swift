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
        identity_manager_destroy(handle).discard()
    }

    /// Create a new empty Identity Manager
    public static func create() throws -> IdentityManager {
        var handle: Handle = NULL_HANDLE
        try identity_manager_create(&handle).check()
        return IdentityManager(handle: handle)
    }

    /// Add an identity to the manager
    public func addIdentity(_ identity: ManagedIdentity) throws {
        try identity_manager_add_identity(handle, identity.handle).check()
    }

    /// Remove an identity from the manager
    public func removeIdentity(_ identityId: Identifier) throws {
        try identityId.withFFIBytes { idPtr in
            try identity_manager_remove_identity(handle, idPtr).check()
        }
    }

    /// Get an identity by ID
    public func getIdentity(_ identityId: Identifier) throws -> ManagedIdentity {
        var identityHandle: Handle = NULL_HANDLE
        try identityId.withFFIBytes { idPtr in
            try identity_manager_get_identity(handle, idPtr, &identityHandle).check()
        }
        return ManagedIdentity(handle: identityHandle)
    }

    /// Get all identity IDs
    public func getAllIdentityIds() throws -> [Identifier] {
        var array = IdentifierArray(items: nil, count: 0)
        try identity_manager_get_all_identity_ids(handle, &array).check()

        defer {
            platform_wallet_identifier_array_free(&array)
        }

        guard array.items != nil, array.count > 0 else {
            return []
        }

        var identifiers: [Identifier] = []
        identifiers.reserveCapacity(Int(array.count))
        for i in 0..<Int(array.count) {
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
        var count: UInt = 0
        try identity_manager_get_identity_count(handle, &count).check()
        return Int(count)
    }
}
