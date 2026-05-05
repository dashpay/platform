import Foundation
import DashSDKFFI

/// Contact Request for DashPay
///
/// `@unchecked Sendable`: the only instance state is an immutable
/// `Handle` (UInt64). All mutable state behind the handle lives in
/// the Rust-side `CONTACT_REQUEST_STORAGE` (parking_lot RwLock).
/// Same rationale as `ManagedPlatformWallet`.
public final class ContactRequest: @unchecked Sendable {
    internal let handle: Handle

    internal init(handle: Handle) {
        self.handle = handle
    }

    deinit {
        contact_request_destroy(handle).discard()
    }

    /// Create a new contact request
    public static func create(
        senderId: Identifier,
        recipientId: Identifier,
        senderKeyIndex: UInt32,
        recipientKeyIndex: UInt32,
        accountReference: UInt32,
        encryptedPublicKey: Data,
        coreHeightCreatedAt: UInt32,
        createdAt: UInt64
    ) throws -> ContactRequest {
        var handle: Handle = NULL_HANDLE

        // Nest the two `withFFIBytes` closures + `withUnsafeBytes`
        // so all three buffers stay live for the FFI call window.
        try senderId.withFFIBytes { senderPtr in
            try recipientId.withFFIBytes { recipientPtr in
                try encryptedPublicKey.withUnsafeBytes { keyPtr in
                    try contact_request_create(
                        senderPtr,
                        recipientPtr,
                        senderKeyIndex,
                        recipientKeyIndex,
                        accountReference,
                        keyPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                        UInt(encryptedPublicKey.count),
                        coreHeightCreatedAt,
                        createdAt,
                        &handle
                    ).check()
                }
            }
        }

        return ContactRequest(handle: handle)
    }

    /// Get the sender identity ID
    public func getSenderId() throws -> Identifier {
        var buf = [UInt8](repeating: 0, count: 32)
        try buf.withUnsafeMutableBufferPointer { bp in
            try contact_request_get_sender_id(handle, bp.baseAddress!).check()
        }
        return Data(buf)
    }

    /// Get the recipient identity ID
    public func getRecipientId() throws -> Identifier {
        var buf = [UInt8](repeating: 0, count: 32)
        try buf.withUnsafeMutableBufferPointer { bp in
            try contact_request_get_recipient_id(handle, bp.baseAddress!).check()
        }
        return Data(buf)
    }

    /// Get the sender key index
    public func getSenderKeyIndex() throws -> UInt32 {
        var keyIndex: UInt32 = 0
        try contact_request_get_sender_key_index(handle, &keyIndex).check()
        return keyIndex
    }

    /// Get the recipient key index
    public func getRecipientKeyIndex() throws -> UInt32 {
        var keyIndex: UInt32 = 0
        try contact_request_get_recipient_key_index(handle, &keyIndex).check()
        return keyIndex
    }

    /// Get the account reference
    public func getAccountReference() throws -> UInt32 {
        var accountRef: UInt32 = 0
        try contact_request_get_account_reference(handle, &accountRef).check()
        return accountRef
    }

    /// Get the encrypted public key
    public func getEncryptedPublicKey() throws -> Data {
        var bytesPtr: UnsafeMutablePointer<UInt8>? = nil
        var length: UInt = 0
        try contact_request_get_encrypted_public_key(handle, &bytesPtr, &length).check()

        defer {
            if let ptr = bytesPtr {
                platform_wallet_bytes_free(ptr, length)
            }
        }

        guard let ptr = bytesPtr else {
            throw PlatformWalletError.nullPointer(
                "contact_request_get_encrypted_public_key returned a NULL bytes pointer"
            )
        }

        return Data(bytes: ptr, count: Int(length))
    }

    /// Get the creation timestamp
    public func getCreatedAt() throws -> UInt64 {
        var createdAt: UInt64 = 0
        try contact_request_get_created_at(handle, &createdAt).check()
        return createdAt
    }
}
