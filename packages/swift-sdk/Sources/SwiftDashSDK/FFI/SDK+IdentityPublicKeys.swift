import Foundation
import DashSDKFFI

extension SDK {
    /// Read an identity's public keys from Platform.
    ///
    /// A read-only, proved query of the identity's keys: it does not load,
    /// reload or touch any `ManagedIdentity`, so the copy a wallet signs
    /// against stays as it is. Every key is returned, disabled ones
    /// included, with its usage limits where the key carries them.
    /// Contract bounds are not part of `IdentityPublicKeyInfo`.
    ///
    /// ``identityGetKeys(identityId:keyRequestType:specificKeyIds:searchPurposeMap:limit:offset:)``
    /// runs the same query and answers JSON; this one returns typed keys for
    /// callers that need `ManagedIdentity.IdentityPublicKeyInfo`.
    ///
    /// An identity always holds at least one key, so Platform answering
    /// with no keys means it has no such identity, and this returns `nil`.
    ///
    /// The call blocks inside Rust until DAPI answers, so it runs on the
    /// shared Platform query queue and the caller only awaits.
    ///
    /// - Parameter identityId: The identity's 32-byte id.
    /// - Returns: The identity's keys, or `nil` when Platform has no
    ///   identity with that id.
    /// - Throws: `SDKError`, classified like the other Platform queries
    ///   (`.networkError`, `.timeout`, ...). A key whose purpose, security
    ///   level or type this SDK does not know fails the whole call with
    ///   `.serializationError` naming the key's index, rather than
    ///   returning the other keys without it.
    public nonisolated func fetchIdentityPublicKeys(
        identityId: Identifier
    ) async throws -> [ManagedIdentity.IdentityPublicKeyInfo]? {
        guard identityId.count == 32 else {
            throw SDKError.invalidParameter(
                "Identity id must be 32 bytes, got \(identityId.count)"
            )
        }
        return try await performBlockingQuery { handle in
            guard let sdkPtr = dash_sdk_get_inner_sdk_ptr(handle) else {
                throw SDKError.invalidState("SDK has no inner Sdk")
            }
            var found = false
            var outPtr: UnsafeMutablePointer<IdentityPublicKeyFFI>? = nil
            var outCount: UInt = 0
            let error = identityId.withUnsafeBytes { idBuf in
                platform_wallet_fetch_identity_public_keys(
                    sdkPtr,
                    idBuf.baseAddress?.assumingMemoryBound(to: UInt8.self),
                    &found,
                    &outPtr,
                    &outCount
                )
            }
            if let error {
                let sdkError = SDKError.fromDashSDKError(error.pointee)
                dash_sdk_error_free(error)
                // The key out-params hold `(null, 0)` on every error.
                throw sdkError
            }
            guard found else {
                return nil
            }
            do {
                return try ManagedIdentity.IdentityPublicKeyInfo.keys(
                    from: outPtr,
                    count: outCount
                )
            } catch PlatformWalletError.deserialization(let message) {
                // The one error decoding throws: a discriminant this SDK
                // does not know. `.serializationError` is the matching case
                // of the method's one error type; the message is kept.
                throw SDKError.serializationError(message)
            } catch {
                throw SDKError.serializationError(String(describing: error))
            }
        }
    }
}
