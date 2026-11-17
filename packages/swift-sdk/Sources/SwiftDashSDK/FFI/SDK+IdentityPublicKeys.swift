import Foundation
import DashSDKFFI

/// One key of ``FetchedIdentityPublicKeys``: the key, and whether it had
/// expired at the proof's block time.
public struct FetchedIdentityPublicKey: Sendable {
    public let key: ManagedIdentity.IdentityPublicKeyInfo

    /// Whether the key had expired at
    /// ``FetchedIdentityPublicKeys/blockTimeMs``, decided in Rust by dpp's
    /// rule (the expiry instant included). `true` means the key can no
    /// longer sign. `false` means it had not expired at that time; it may
    /// have since, because Platform's time is at least the proof's.
    public let expiredAtProofTime: Bool

    public init(key: ManagedIdentity.IdentityPublicKeyInfo, expiredAtProofTime: Bool) {
        self.key = key
        self.expiredAtProofTime = expiredAtProofTime
    }
}

/// An identity's public keys as Platform proved them, with the Platform
/// block time of the proof. Returned by
/// ``SDK/fetchIdentityPublicKeys(identityId:)``.
public struct FetchedIdentityPublicKeys: Sendable {
    /// Every key of the identity, disabled ones included, or `nil` when
    /// Platform has no such identity.
    public let keys: [FetchedIdentityPublicKey]?

    /// The Platform block time, in milliseconds since the epoch, of the
    /// state the answer was proved against, also when the identity was not
    /// found. The proof covers it: the quorum signature is over a state that
    /// includes this time, so a node cannot report another one.
    ///
    /// It is a lower bound on Platform's current time, not the current
    /// time: the SDK accepts a proof up to its time tolerance old, and any
    /// proof when it has none, so a caller that needs a fresh answer must
    /// build the SDK with a time tolerance. A key whose
    /// `expiresAt <= blockTimeMs` can no longer sign
    /// (``FetchedIdentityPublicKey/expiredAtProofTime``); any other key may
    /// still have expired since.
    public let blockTimeMs: TimestampMillis

    /// ``blockTimeMs`` as a `Date`.
    public var blockTime: Date {
        Date(timeIntervalSince1970: TimeInterval(blockTimeMs) / 1000)
    }

    public init(keys: [FetchedIdentityPublicKey]?, blockTimeMs: TimestampMillis) {
        self.keys = keys
        self.blockTimeMs = blockTimeMs
    }
}

extension FetchedIdentityPublicKey {
    /// Decode the array `platform_wallet_fetch_identity_public_keys`
    /// returns, releasing the FFI buffer on every path (success or throw).
    static func keys(
        from ptr: UnsafeMutablePointer<FetchedIdentityPublicKeyFFI>?,
        count: UInt
    ) throws -> [Self] {
        try consumeFFIArray(
            ptr,
            count: count,
            free: { platform_wallet_free_fetched_identity_public_keys($0, $1) },
            decode: decode
        )
    }

    /// Copy each FFI entry, with its proof-time expiry. Reads `entries`
    /// only; the caller keeps ownership of the buffer. A key with an
    /// unknown discriminant fails the whole array.
    static func decode(_ entries: UnsafeBufferPointer<FetchedIdentityPublicKeyFFI>) throws -> [Self] {
        try entries.enumerated().map { i, ffi in
            Self(
                key: try ManagedIdentity.IdentityPublicKeyInfo(ffi: ffi.key, index: i),
                expiredAtProofTime: ffi.expired_at_proof_time
            )
        }
    }
}

extension FetchedIdentityPublicKeys {
    /// The result of a successful `platform_wallet_fetch_identity_public_keys`
    /// call, built from what it wrote to its out-params. Takes ownership of
    /// the key array and releases it on every path.
    ///
    /// `found == false` gives `keys == nil` without decoding anything (the
    /// FFI then writes `(null, 0)`). A found identity always holds a key, so
    /// `found` with no keys breaks the FFI's contract and throws
    /// `PlatformWalletError.deserialization` instead of reading as an
    /// identity without keys.
    init(
        found: Bool,
        blockTimeMs: TimestampMillis,
        keys ptr: UnsafeMutablePointer<FetchedIdentityPublicKeyFFI>?,
        count: UInt
    ) throws {
        guard found else {
            platform_wallet_free_fetched_identity_public_keys(ptr, count)
            self.init(keys: nil, blockTimeMs: blockTimeMs)
            return
        }
        let keys = try FetchedIdentityPublicKey.keys(from: ptr, count: count)
        guard !keys.isEmpty else {
            throw PlatformWalletError.deserialization(
                "A found identity came back with no public keys"
            )
        }
        self.init(keys: keys, blockTimeMs: blockTimeMs)
    }
}

extension SDK {
    /// Read an identity's public keys from Platform, with the Platform time
    /// they were proved at and whether each key had expired then.
    ///
    /// A read-only, proved query of the identity's keys: it does not load,
    /// reload or touch any `ManagedIdentity`, so the copy a wallet signs
    /// against stays as it is. Every key is returned, disabled ones
    /// included, with its usage limits where the key carries them.
    /// Contract bounds are not part of `IdentityPublicKeyInfo`.
    ///
    /// ``identityGetKeys(identityId:keyRequestType:specificKeyIds:searchPurposeMap:limit:offset:)``
    /// runs the same query and answers JSON; this one returns typed keys for
    /// callers that need `ManagedIdentity.IdentityPublicKeyInfo`, with the
    /// proof's block time (``FetchedIdentityPublicKeys/blockTimeMs``), a
    /// lower bound on Platform's current time, and each key's expiry at it.
    ///
    /// An identity always holds at least one key, so Platform answering
    /// with no keys means it has no such identity: `keys` is then `nil`,
    /// and the block time is still the proof's.
    ///
    /// The call blocks inside Rust until DAPI answers, so it runs on the
    /// shared Platform query queue and the caller only awaits.
    ///
    /// - Parameter identityId: The identity's 32-byte id.
    /// - Returns: The identity's keys (`nil` when Platform has no identity
    ///   with that id) and the proof's block time.
    /// - Throws: `SDKError`, classified like the other Platform queries
    ///   (`.networkError`, `.timeout`, ...). A key whose purpose, security
    ///   level or type this SDK does not know fails the whole call with
    ///   `.serializationError` naming the key's index, rather than
    ///   returning the other keys without it.
    public nonisolated func fetchIdentityPublicKeys(
        identityId: Identifier
    ) async throws -> FetchedIdentityPublicKeys {
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
            var blockTimeMs: UInt64 = 0
            var outPtr: UnsafeMutablePointer<FetchedIdentityPublicKeyFFI>? = nil
            var outCount: UInt = 0
            let error = identityId.withUnsafeBytes { idBuf in
                platform_wallet_fetch_identity_public_keys(
                    sdkPtr,
                    idBuf.baseAddress?.assumingMemoryBound(to: UInt8.self),
                    &found,
                    &blockTimeMs,
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
            do {
                return try FetchedIdentityPublicKeys(
                    found: found,
                    blockTimeMs: blockTimeMs,
                    keys: outPtr,
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
