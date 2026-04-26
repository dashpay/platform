import DashSDKFFI
import Foundation

// MARK: - MnemonicResolver

/// Swift bridge backing the Rust-side `MnemonicResolverHandle`.
///
/// The Rust derivation loop in
/// `dash_sdk_derive_and_persist_identity_keys` calls back into
/// Swift via this resolver to fetch the BIP-39 mnemonic for the
/// wallet whose identity keys it's deriving. The mnemonic is
/// copied directly into a Rust-owned `Zeroizing` stack buffer; it
/// never round-trips back to Swift after this single read, and
/// the Swift `String` from `WalletStorage.retrieveMnemonic` falls
/// out of scope at the end of the trampoline.
///
/// # Lifetime
///
/// Init allocates a `MnemonicResolverHandle*` via
/// `dash_sdk_mnemonic_resolver_create`, handing Rust an
/// `Unmanaged.passRetained(self)` pointer. `deinit` calls
/// `dash_sdk_mnemonic_resolver_destroy`, which fires the
/// destructor trampoline that releases the +1 retain. The handle
/// is safe to pass to FFI calls for the entire lifetime of this
/// instance.
public final class MnemonicResolver: @unchecked Sendable {
    /// Owned for the lifetime of this object. Pass this to
    /// `dash_sdk_derive_and_persist_identity_keys`.
    public var handle: UnsafeMutablePointer<MnemonicResolverHandle>? {
        handlePtr
    }

    private let storage: WalletStorage
    private var handlePtr: UnsafeMutablePointer<MnemonicResolverHandle>?

    /// - Parameter storage: source for the BIP-39 mnemonic per
    ///   `walletId`. Defaults to a fresh `WalletStorage()` so
    ///   tests can substitute a mock.
    public init(storage: WalletStorage = WalletStorage()) {
        self.storage = storage

        let ctx = Unmanaged.passRetained(self).toOpaque()
        self.handlePtr = dash_sdk_mnemonic_resolver_create(
            ctx,
            mnemonicResolverResolveTrampoline,
            mnemonicResolverDestroyTrampoline
        )
        precondition(
            handlePtr != nil,
            "dash_sdk_mnemonic_resolver_create returned NULL"
        )
    }

    deinit {
        if let handlePtr {
            dash_sdk_mnemonic_resolver_destroy(handlePtr)
        }
    }

    /// Trampoline-callable internals.
    fileprivate func resolve(
        walletId: Data,
        outBuffer: UnsafeMutablePointer<CChar>,
        outCapacity: Int,
        outLen: UnsafeMutablePointer<Int>
    ) -> MnemonicResolverResult {
        let mnemonic: String
        do {
            mnemonic = try storage.retrieveMnemonic(for: walletId)
        } catch {
            return .notFound
        }

        // `withCString` materializes a null-terminated UTF-8 byte
        // sequence whose lifetime ends with the closure. We copy
        // (excluding the trailing NUL) into the Rust-owned
        // buffer; the source bytes drop with the Swift `String`
        // at the end of `resolve`.
        return mnemonic.withCString { srcPtr -> MnemonicResolverResult in
            let mnemonicLen = strlen(srcPtr)
            // Need room for the data plus a trailing NUL byte.
            guard mnemonicLen + 1 <= outCapacity else {
                return .bufferTooSmall
            }
            outBuffer.update(from: srcPtr, count: mnemonicLen)
            // Explicit NUL terminator — defensive, the Rust side
            // works off `out_len` not strlen but matching the
            // wire contract is cheap insurance.
            (outBuffer + mnemonicLen).pointee = 0
            outLen.pointee = mnemonicLen
            return .success
        }
    }
}

// MARK: - MnemonicResolver C trampolines
//
// These run on whatever thread the Rust derivation loop is
// executing on (typically a background queue from the iOS
// caller). `WalletStorage.retrieveMnemonic` is thread-safe; no
// further synchronization is required.

private func mnemonicResolverResolveTrampoline(
    ctx: UnsafeRawPointer?,
    walletIdBytes: UnsafePointer<UInt8>?,
    outBuffer: UnsafeMutablePointer<CChar>?,
    outCapacity: Int,
    outLen: UnsafeMutablePointer<Int>?
) -> Int32 {
    guard let ctx, let walletIdBytes, let outBuffer, let outLen else {
        return MnemonicResolverResult.other.rawValue
    }
    let resolver = Unmanaged<MnemonicResolver>.fromOpaque(ctx).takeUnretainedValue()
    let walletId = Data(bytes: walletIdBytes, count: 32)
    let result = resolver.resolve(
        walletId: walletId,
        outBuffer: outBuffer,
        outCapacity: outCapacity,
        outLen: outLen
    )
    return result.rawValue
}

private func mnemonicResolverDestroyTrampoline(ctx: UnsafeMutableRawPointer?) {
    guard let ctx else { return }
    Unmanaged<MnemonicResolver>.fromOpaque(ctx).release()
}

// MARK: - IdentityKeyPersister

/// Swift bridge backing the Rust-side
/// `IdentityKeyPersisterHandle`. The Rust derivation loop calls
/// back here once per derived key with a [`PersistKeyArgs`]
/// payload; this class translates that into a
/// `KeychainManager.storeIdentityPrivateKey(...)` call.
///
/// Per-key DPP metadata (key_type, purpose, security_level) is
/// computed entirely on the Rust side — Swift just stamps
/// whatever bytes it's told into Keychain. This is the inverse of
/// the prior shape, where Swift hardcoded `keyId 0 -> MASTER,
/// else HIGH` plus the DPP discriminant bytes.
public final class IdentityKeyPersister: @unchecked Sendable {
    /// Per-key metadata captured as a side effect of each persist
    /// callback firing during a `dash_sdk_derive_and_persist_identity_keys`
    /// call. Read this AFTER the FFI returns to recover the
    /// `(keyType, purpose, securityLevel)` triple Rust decided per
    /// key — useful for building `IdentityPubkey` rows without
    /// recreating the MASTER-vs-HIGH policy on the Swift side.
    public struct PersistedKeyMetadata: Sendable {
        public let identityIndex: UInt32
        public let keyId: UInt32
        public let keyIndex: UInt32
        public let derivationPath: String
        public let publicKey: Data
        public let publicKeyHash: Data
        public let keyType: UInt8
        public let purpose: UInt8
        public let securityLevel: UInt8
    }

    public var handle: UnsafeMutablePointer<IdentityKeyPersisterHandle>? {
        handlePtr
    }

    /// Snapshot of every key the persister wrote during the most
    /// recent FFI call. Indexed in callback-fire order (= the
    /// derivation order Rust used: key_index 0..key_count). Reset
    /// implicitly per `IdentityKeyPersister` instance — typical
    /// usage constructs a fresh persister per derive call so the
    /// list always describes one logical batch.
    public var persistedKeys: [PersistedKeyMetadata] {
        // Trampolines fire synchronously on the same thread the
        // FFI call was invoked on, so a plain unsynchronized read
        // is safe at the natural use site (after FFI returns).
        // Marked @unchecked Sendable for the same reason.
        _persistedKeys
    }
    private var _persistedKeys: [PersistedKeyMetadata] = []

    private let keychain: KeychainManager
    private var handlePtr: UnsafeMutablePointer<IdentityKeyPersisterHandle>?

    public init(keychain: KeychainManager = .shared) {
        self.keychain = keychain

        let ctx = Unmanaged.passRetained(self).toOpaque()
        self.handlePtr = dash_sdk_identity_key_persister_create(
            ctx,
            identityKeyPersisterPersistTrampoline,
            identityKeyPersisterDestroyTrampoline
        )
        precondition(
            handlePtr != nil,
            "dash_sdk_identity_key_persister_create returned NULL"
        )
    }

    deinit {
        if let handlePtr {
            dash_sdk_identity_key_persister_destroy(handlePtr)
        }
    }

    /// Trampoline-callable internal. Returns `false` to abort
    /// the rest of the Rust derivation loop with an
    /// `ErrorWalletOperation`; returns `true` to let it continue.
    fileprivate func persist(args: UnsafePointer<PersistKeyArgs>) -> Bool {
        let a = args.pointee
        guard
            let walletIdPtr = a.wallet_id_bytes,
            let pathPtr = a.derivation_path_cstr,
            let pubKeyPtr = a.public_key_bytes,
            let pubHashPtr = a.public_key_hash_bytes,
            let privKeyPtr = a.private_key_bytes
        else {
            return false
        }

        let walletIdBytes = Data(bytes: walletIdPtr, count: 32)
        let walletIdHex = walletIdBytes.map { String(format: "%02x", $0) }.joined()
        let derivationPath = String(cString: pathPtr)
        let publicKeyData = Data(bytes: pubKeyPtr, count: a.public_key_len)
        let publicKeyHex = publicKeyData.map { String(format: "%02x", $0) }.joined()
        let publicKeyHashData = Data(bytes: pubHashPtr, count: 20)
        let publicKeyHashHex = publicKeyHashData
            .map { String(format: "%02x", $0) }
            .joined()
        let privateKeyData = Data(bytes: privKeyPtr, count: 32)

        // Identity-id is unknown pre-registration — Rust will
        // recompute it from the input addresses at submit time.
        // Use the marker `pending` so the keychain explorer makes
        // it obvious which rows are pre-registered slots.
        let metadata = KeychainManager.IdentityPrivateKeyMetadata(
            identityId: "pending",
            keyId: a.key_id,
            walletId: walletIdHex,
            identityIndex: a.identity_index,
            keyIndex: a.key_index,
            derivationPath: derivationPath,
            publicKey: publicKeyHex,
            publicKeyHash: publicKeyHashHex,
            keyType: a.key_type,
            purpose: a.purpose,
            securityLevel: a.security_level
        )

        let stored = keychain.storeIdentityPrivateKey(
            privateKeyData,
            derivationPath: derivationPath,
            metadata: metadata
        )
        if stored == nil {
            return false
        }

        // Capture the per-key metadata so the caller can build
        // `IdentityPubkey` rows from it without re-deciding the
        // MASTER-vs-HIGH policy that already lives in Rust.
        _persistedKeys.append(PersistedKeyMetadata(
            identityIndex: a.identity_index,
            keyId: a.key_id,
            keyIndex: a.key_index,
            derivationPath: derivationPath,
            publicKey: publicKeyData,
            publicKeyHash: publicKeyHashData,
            keyType: a.key_type,
            purpose: a.purpose,
            securityLevel: a.security_level
        ))
        return true
    }
}

private func identityKeyPersisterPersistTrampoline(
    ctx: UnsafeRawPointer?,
    args: UnsafeRawPointer?
) -> UInt8 {
    guard let ctx, let args else { return PERSIST_KEY_FAILURE }
    let persister = Unmanaged<IdentityKeyPersister>.fromOpaque(ctx).takeUnretainedValue()
    let typedArgs = args.assumingMemoryBound(to: PersistKeyArgs.self)
    return persister.persist(args: typedArgs) ? PERSIST_KEY_SUCCESS : PERSIST_KEY_FAILURE
}

private func identityKeyPersisterDestroyTrampoline(ctx: UnsafeMutableRawPointer?) {
    guard let ctx else { return }
    Unmanaged<IdentityKeyPersister>.fromOpaque(ctx).release()
}
