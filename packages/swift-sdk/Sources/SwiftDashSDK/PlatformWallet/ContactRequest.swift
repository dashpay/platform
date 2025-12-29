import Foundation
import DashSDKFFI

/// Contact Request for DashPay
public class ContactRequest {
    internal let handle: Handle

    internal init(handle: Handle) {
        self.handle = handle
    }

    deinit {
        contact_request_destroy(handle)
    }

    /// Create a new contact request
    public static func create(
        senderId: Identifier,
        recipientId: Identifier,
        senderKeyIndex: UInt32,
        recipientKeyIndex: UInt32,
        accountReference: UInt32,
        encryptedPublicKey: Data,
        createdAt: UInt64
    ) throws -> ContactRequest {
        var handle: Handle = NULL_HANDLE
        var error = PlatformWalletFFIError()
        var ffiSenderId = identifierToFFI(senderId)
        var ffiRecipientId = identifierToFFI(recipientId)

        let result = encryptedPublicKey.withUnsafeBytes { keyPtr in
            contact_request_create(
                ffiSenderId,
                ffiRecipientId,
                senderKeyIndex,
                recipientKeyIndex,
                accountReference,
                keyPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                encryptedPublicKey.count,
                createdAt,
                &handle,
                &error
            )
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return ContactRequest(handle: handle)
    }

    /// Get the sender identity ID
    public func getSenderId() throws -> Identifier {
        var ffiId = IdentifierBytes(bytes: (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0))
        var error = PlatformWalletFFIError()

        let result = contact_request_get_sender_id(handle, &ffiId, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return identifierFromFFI(ffiId)
    }

    /// Get the recipient identity ID
    public func getRecipientId() throws -> Identifier {
        var ffiId = IdentifierBytes(bytes: (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0))
        var error = PlatformWalletFFIError()

        let result = contact_request_get_recipient_id(handle, &ffiId, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return identifierFromFFI(ffiId)
    }

    /// Get the sender key index
    public func getSenderKeyIndex() throws -> UInt32 {
        var keyIndex: UInt32 = 0
        var error = PlatformWalletFFIError()

        let result = contact_request_get_sender_key_index(handle, &keyIndex, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return keyIndex
    }

    /// Get the recipient key index
    public func getRecipientKeyIndex() throws -> UInt32 {
        var keyIndex: UInt32 = 0
        var error = PlatformWalletFFIError()

        let result = contact_request_get_recipient_key_index(handle, &keyIndex, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return keyIndex
    }

    /// Get the account reference
    public func getAccountReference() throws -> UInt32 {
        var accountRef: UInt32 = 0
        var error = PlatformWalletFFIError()

        let result = contact_request_get_account_reference(handle, &accountRef, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return accountRef
    }

    /// Get the encrypted public key
    public func getEncryptedPublicKey() throws -> Data {
        var bytesPtr: UnsafeMutablePointer<UInt8>? = nil
        var length: Int = 0
        var error = PlatformWalletFFIError()

        let result = contact_request_get_encrypted_public_key(handle, &bytesPtr, &length, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        defer {
            if let ptr = bytesPtr {
                platform_wallet_bytes_free(ptr)
            }
        }

        guard let ptr = bytesPtr else {
            throw PlatformWalletError.nullPointer
        }

        return Data(bytes: ptr, count: length)
    }

    /// Get the creation timestamp
    public func getCreatedAt() throws -> UInt64 {
        var createdAt: UInt64 = 0
        var error = PlatformWalletFFIError()

        let result = contact_request_get_created_at(handle, &createdAt, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return createdAt
    }
}
