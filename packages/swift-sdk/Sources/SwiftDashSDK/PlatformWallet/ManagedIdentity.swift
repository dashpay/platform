import Foundation
import DashSDKFFI

/// Managed Identity with DashPay metadata
public class ManagedIdentity {
    internal let handle: Handle

    internal init(handle: Handle) {
        self.handle = handle
    }

    deinit {
        _ = managed_identity_destroy(handle)
    }

    /// Create a ManagedIdentity from identity bytes
    public static func fromIdentityBytes(_ bytes: Data) throws -> ManagedIdentity {
        var handle: Handle = NULL_HANDLE
        var error = PlatformWalletFFIError()

        let result = bytes.withUnsafeBytes { bytesPtr in
            managed_identity_create_from_identity_bytes(
                bytesPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                UInt(bytes.count),
                &handle,
                &error
            )
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return ManagedIdentity(handle: handle)
    }

    /// Get the identity ID
    public func getId() throws -> Identifier {
        var ffiId = IdentifierBytes(bytes: (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0))
        var error = PlatformWalletFFIError()

        let result = managed_identity_get_id(handle, &ffiId, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return identifierFromFFI(ffiId)
    }

    /// Get the identity balance
    public func getBalance() throws -> UInt64 {
        var balance: UInt64 = 0
        var error = PlatformWalletFFIError()

        let result = managed_identity_get_balance(handle, &balance, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return balance
    }

    /// Get the identity label
    public func getLabel() throws -> String? {
        var labelPtr: UnsafeMutablePointer<CChar>? = nil
        var error = PlatformWalletFFIError()

        let result = managed_identity_get_label(handle, &labelPtr, &error)

        if result == ErrorIdentityNotFound {
            return nil
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        defer {
            if let ptr = labelPtr {
                platform_wallet_string_free(ptr)
            }
        }

        guard let ptr = labelPtr else {
            return nil
        }

        return String(cString: ptr)
    }

    /// Set the identity label
    public func setLabel(_ label: String) throws {
        var error = PlatformWalletFFIError()
        let labelCStr = (label as NSString).utf8String

        let result = managed_identity_set_label(handle, labelCStr, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    /// Get the last updated balance block time
    public func getLastUpdatedBalanceBlockTime() throws -> PlatformBlockTime? {
        var ffiBlockTime = BlockTime(height: 0, core_height: 0, timestamp: 0)
        var error = PlatformWalletFFIError()

        let result = managed_identity_get_last_updated_balance_block_time(handle, &ffiBlockTime, &error)

        if result == ErrorIdentityNotFound {
            return nil
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return PlatformBlockTime(ffi: ffiBlockTime)
    }

    /// Set the last updated balance block time
    public func setLastUpdatedBalanceBlockTime(_ blockTime: PlatformBlockTime) throws {
        var error = PlatformWalletFFIError()
        let ffiBlockTime = blockTime.ffiValue

        let result = managed_identity_set_last_updated_balance_block_time(handle, ffiBlockTime, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    /// Get the last synced keys block time
    public func getLastSyncedKeysBlockTime() throws -> PlatformBlockTime? {
        var ffiBlockTime = BlockTime(height: 0, core_height: 0, timestamp: 0)
        var error = PlatformWalletFFIError()

        let result = managed_identity_get_last_synced_keys_block_time(handle, &ffiBlockTime, &error)

        if result == ErrorIdentityNotFound {
            return nil
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return PlatformBlockTime(ffi: ffiBlockTime)
    }

    // MARK: - Contact Request Management

    /// Get all sent contact request IDs
    public func getSentContactRequestIds() throws -> [Identifier] {
        var array = IdentifierArray(items: nil, count: 0)
        var error = PlatformWalletFFIError()

        let result = managed_identity_get_sent_contact_request_ids(handle, &array, &error)
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
        for i in 0..<Int(array.count) {
            identifiers.append(identifierFromFFI(items[i]))
        }

        return identifiers
    }

    /// Get all incoming contact request IDs
    public func getIncomingContactRequestIds() throws -> [Identifier] {
        var array = IdentifierArray(items: nil, count: 0)
        var error = PlatformWalletFFIError()

        let result = managed_identity_get_incoming_contact_request_ids(handle, &array, &error)
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
        for i in 0..<Int(array.count) {
            identifiers.append(identifierFromFFI(items[i]))
        }

        return identifiers
    }

    /// Get all established contact IDs
    public func getEstablishedContactIds() throws -> [Identifier] {
        var array = IdentifierArray(items: nil, count: 0)
        var error = PlatformWalletFFIError()

        let result = managed_identity_get_established_contact_ids(handle, &array, &error)
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
        for i in 0..<Int(array.count) {
            identifiers.append(identifierFromFFI(items[i]))
        }

        return identifiers
    }

    /// Get a sent contact request by recipient ID
    public func getSentContactRequest(recipientId: Identifier) throws -> ContactRequest? {
        var requestHandle: Handle = NULL_HANDLE
        var error = PlatformWalletFFIError()
        let ffiId = identifierToFFI(recipientId)

        let result = managed_identity_get_sent_contact_request(handle, ffiId, &requestHandle, &error)

        if result == ErrorContactNotFound {
            return nil
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return ContactRequest(handle: requestHandle)
    }

    /// Get an incoming contact request by sender ID
    public func getIncomingContactRequest(senderId: Identifier) throws -> ContactRequest? {
        var requestHandle: Handle = NULL_HANDLE
        var error = PlatformWalletFFIError()
        let ffiId = identifierToFFI(senderId)

        let result = managed_identity_get_incoming_contact_request(handle, ffiId, &requestHandle, &error)

        if result == ErrorContactNotFound {
            return nil
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return ContactRequest(handle: requestHandle)
    }

    /// Get an established contact by contact ID
    public func getEstablishedContact(contactId: Identifier) throws -> EstablishedContact? {
        var contactHandle: Handle = NULL_HANDLE
        var error = PlatformWalletFFIError()
        let ffiId = identifierToFFI(contactId)

        let result = managed_identity_get_established_contact(handle, ffiId, &contactHandle, &error)

        if result == ErrorContactNotFound {
            return nil
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return EstablishedContact(handle: contactHandle)
    }

    /// Check if a contact is established
    public func isContactEstablished(contactId: Identifier) throws -> Bool {
        var isEstablished: Bool = false
        var error = PlatformWalletFFIError()
        let ffiId = identifierToFFI(contactId)

        let result = managed_identity_is_contact_established(handle, ffiId, &isEstablished, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return isEstablished
    }

    /// Send a contact request by attaching a pre-built request handle.
    /// Build the request via `ContactRequest.create(...)` first.
    public func sendContactRequest(_ request: ContactRequest) throws {
        var error = PlatformWalletFFIError()

        let result = managed_identity_send_contact_request(handle, request.handle, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    /// Accept an incoming contact request.
    /// The request handle typically comes from `getIncomingContactRequest(senderId:)`.
    public func acceptContactRequest(_ request: ContactRequest) throws {
        var error = PlatformWalletFFIError()

        let result = managed_identity_accept_contact_request(handle, request.handle, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    /// Reject a contact request from another identity
    public func rejectContactRequest(senderId: Identifier) throws {
        var error = PlatformWalletFFIError()
        let ffiSenderId = identifierToFFI(senderId)

        let result = managed_identity_reject_contact_request(handle, ffiSenderId, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }
}
