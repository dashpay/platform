import DashSDKFFI
import Foundation
import Security

// MARK: - MnemonicResolver

private func scrubBytes(_ bytes: inout [UInt8]) {
    bytes.withUnsafeMutableBufferPointer { buffer in
        guard let base = buffer.baseAddress else { return }
        memset_s(base, buffer.count, 0, buffer.count)
    }
}

/// Best-effort in-memory obfuscation for mnemonic UTF-8 bytes while
/// they sit on the Swift heap between the Keychain read and the final
/// copy into Rust's `Zeroizing` buffer.
private final class MaskedMnemonicUTF8 {
    private var maskedBytes: [UInt8]
    private var maskBytes: [UInt8]

    init(plaintextUTF8Bytes: Data) throws {
        var plaintext = [UInt8](plaintextUTF8Bytes)
        guard !plaintext.isEmpty else {
            throw WalletStorageError.mnemonicNotFound
        }

        var localMask = [UInt8](repeating: 0, count: plaintext.count)
        let randomStatus = localMask.withUnsafeMutableBufferPointer { buffer -> Int32 in
            guard let base = buffer.baseAddress else { return errSecSuccess }
            return SecRandomCopyBytes(kSecRandomDefault, buffer.count, base)
        }
        guard randomStatus == errSecSuccess else {
            scrubBytes(&plaintext)
            scrubBytes(&localMask)
            throw WalletStorageError.keychainError(randomStatus)
        }

        var localMasked = [UInt8](repeating: 0, count: plaintext.count)
        for i in plaintext.indices {
            localMasked[i] = plaintext[i] ^ localMask[i]
        }

        scrubBytes(&plaintext)
        self.maskedBytes = localMasked
        self.maskBytes = localMask
    }

    deinit {
        scrubBytes(&maskedBytes)
        scrubBytes(&maskBytes)
    }

    func withDeobfuscatedBytes<R>(_ body: (UnsafeBufferPointer<UInt8>) -> R) -> R {
        var plaintext = [UInt8](repeating: 0, count: maskedBytes.count)
        for i in maskedBytes.indices {
            plaintext[i] = maskedBytes[i] ^ maskBytes[i]
        }
        defer { scrubBytes(&plaintext) }
        return plaintext.withUnsafeBufferPointer(body)
    }
}

/// Swift bridge backing the Rust-side `MnemonicResolverHandle`.
///
/// The Rust derivation loop in
/// `dash_sdk_derive_and_persist_identity_keys` (and the
/// platform-address signing path in
/// `dash_sdk_sign_with_mnemonic_resolver_and_path`) calls back
/// into Swift via this resolver to fetch the BIP-39 mnemonic for
/// the wallet whose identity keys it's deriving. The mnemonic is
/// copied directly into a Rust-owned `Zeroizing` stack buffer; it
/// never round-trips back to Swift after this single read. On the
/// Swift side the bytes are masked while idle, then deobfuscated only
/// long enough to copy into the FFI output buffer.
///
/// # Lifetime contract
///
/// Init does NOT retain `self` through the Rust handle —
/// `Unmanaged.passUnretained` is used so that ARC alone controls
/// when this object deallocates. The Swift owner is responsible
/// for keeping the instance alive for the duration of any FFI
/// call that captured `handle`. In practice that's automatic for
/// the synchronous `dash_sdk_derive_and_persist_identity_keys` /
/// `dash_sdk_sign_with_mnemonic_resolver_and_path` paths, where
/// a local `let resolver = MnemonicResolver(...)` variable
/// outlives the call by construction.
///
/// `deinit` calls `dash_sdk_mnemonic_resolver_destroy` to free
/// the Rust-side handle/vtable allocations.
///
/// # Why not `passRetained`?
///
/// Using `passRetained` here would create a circular ownership:
/// Rust holds a +1 retain on `self`, the destroy trampoline only
/// releases that retain when the destroy FFI is invoked, and the
/// destroy FFI is only invoked from `deinit`. ARC won't run
/// `deinit` while the +1 retain is alive → the handle leaks
/// every instance forever. Per-call usage in
/// `prePersistIdentityKeysForRegistration` would leak two of
/// these per registration.
public final class MnemonicResolver: @unchecked Sendable {
    /// Owned for the lifetime of this object. Pass this to
    /// `dash_sdk_derive_and_persist_identity_keys` (or the
    /// resolver-based sign FFI). Caller must keep `self` alive
    /// for the duration of any FFI call that captured the
    /// handle — see the type-level "Lifetime contract" note.
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

        let ctx = Unmanaged.passUnretained(self).toOpaque()
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
        outCapacity: UInt,
        outLen: UnsafeMutablePointer<UInt>
    ) -> MnemonicResolverResult {
        let mnemonicUTF8Bytes: Data
        do {
            mnemonicUTF8Bytes = try storage.retrieveMnemonicUTF8Bytes(for: walletId)
        } catch WalletStorageError.mnemonicNotFound {
            // Distinct "this wallet has no stored mnemonic" case
            // — Rust surfaces this as the recoverable
            // `mnemonicMissing(walletIdHex:)` error in the Swift
            // signing path, which UI uses to direct the user
            // toward "import this wallet's mnemonic".
            return .notFound
        } catch {
            // Anything else (Keychain access denied, biometric
            // auth failed, OSStatus failure...) is a real
            // operational error. Map to `.other` so the
            // distinction survives across the FFI boundary —
            // collapsing into `.notFound` would point the user
            // toward the wrong remediation.
            return .other
        }

        let maskedMnemonic: MaskedMnemonicUTF8
        do {
            maskedMnemonic = try MaskedMnemonicUTF8(plaintextUTF8Bytes: mnemonicUTF8Bytes)
        } catch {
            return .other
        }

        return maskedMnemonic.withDeobfuscatedBytes { bytes -> MnemonicResolverResult in
            let mnemonicLen = bytes.count
            // Need room for the data plus a trailing NUL byte.
            guard UInt(mnemonicLen) + 1 <= outCapacity else {
                return .bufferTooSmall
            }
            guard let srcBase = bytes.baseAddress else {
                return .other
            }
            if bytes.contains(0) {
                return .other
            }
            srcBase.withMemoryRebound(to: CChar.self, capacity: mnemonicLen) { srcPtr in
                outBuffer.update(from: srcPtr, count: mnemonicLen)
            }
            // Explicit NUL terminator — defensive, the Rust side
            // works off `out_len` not strlen but matching the
            // wire contract is cheap insurance.
            (outBuffer + mnemonicLen).pointee = 0
            outLen.pointee = UInt(mnemonicLen)
            return .success
        }
    }
}

// MARK: - MnemonicResolver C trampolines

private func mnemonicResolverResolveTrampoline(
    ctx: UnsafeRawPointer?,
    walletIdBytes: UnsafePointer<UInt8>?,
    outBuffer: UnsafeMutablePointer<CChar>?,
    outCapacity: UInt,
    outLen: UnsafeMutablePointer<UInt>?
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

/// No-op destructor. The Swift `self` is held by ARC alone — the
/// `passUnretained` ctx pointer carries no refcount to balance.
/// Keeping the trampoline so the Rust vtable's `destroy` slot is
/// always non-null (simplifies the Rust safety contract).
private func mnemonicResolverDestroyTrampoline(_: UnsafeMutableRawPointer?) {}

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
///
/// Same `passUnretained` lifetime contract as
/// [`MnemonicResolver`] — see that type's docs.
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

        let ctx = Unmanaged.passUnretained(self).toOpaque()
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
        let publicKeyData = Data(bytes: pubKeyPtr, count: Int(a.public_key_len))
        let publicKeyHex = publicKeyData.map { String(format: "%02x", $0) }.joined()
        let publicKeyHashData = Data(bytes: pubHashPtr, count: 20)
        let publicKeyHashHex = publicKeyHashData
            .map { String(format: "%02x", $0) }
            .joined()
        // The 32 raw private bytes have to enter Swift `Data` to hand
        // to `KeychainManager.storeIdentityPrivateKey`, which takes
        // `Data` (Keychain APIs are iOS-only and Rust can't call
        // `SecItemAdd` directly). Swift `Data` cannot be securely
        // zeroed, so we keep this allocation as the only place the
        // secret bytes touch the Swift heap and let it drop with the
        // function frame as soon as Keychain has taken its copy.
        //
        // Earlier revisions also hex-encoded the private bytes
        // into a `String` to call
        // `KeyValidation.validatePrivateKeyForPublicKey`. That
        // validation was tautological in this code path — Rust
        // had just derived `publicKey` from `privateKey` via the
        // same `secp256k1` operation a few microseconds earlier,
        // and the `PersistKeyArgs` layout assertion guards
        // against ABI drift between the two sides — so the only
        // observable effect was an extra non-zeroizable Swift
        // `String` of the private scalar. Removed for that reason.
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
    args: UnsafePointer<PersistKeyArgs>?
) -> UInt8 {
    guard let ctx, let args else { return UInt8(PERSIST_KEY_FAILURE) }
    let persister = Unmanaged<IdentityKeyPersister>.fromOpaque(ctx).takeUnretainedValue()
    return persister.persist(args: args) ? UInt8(PERSIST_KEY_SUCCESS) : UInt8(PERSIST_KEY_FAILURE)
}

/// No-op destructor — see `mnemonicResolverDestroyTrampoline`.
private func identityKeyPersisterDestroyTrampoline(_: UnsafeMutableRawPointer?) {}
