import DashSDKFFI
import Foundation
import SwiftData

// MARK: - KeychainSigner

/// Production `Signer` implementation backed by `PersistentPublicKey` rows
/// in SwiftData and the `KeychainManager` private-key store.
///
/// # How a signing request flows
///
/// 1. Rust calls the C-ABI `sign_async` trampoline with the **raw**
///    public-key bytes, the [`KeyType`] discriminant byte, and the data
///    to sign (see `SignAsyncCallback` in `rs-sdk-ffi/src/signer.rs`).
/// 2. The trampoline locates the matching `PersistentPublicKey` row by
///    `publicKeyData == receivedPubkeyBytes` on a private background
///    `ModelContext`.
/// 3. It reads the row's `privateKeyKeychainIdentifier`, fetches the
///    32-byte ECDSA scalar from the iOS Keychain via
///    `KeychainManager.retrieveKeyData(identifier:)`.
/// 4. It hands those bytes to `dash_sdk_signer_create_from_private_key`
///    + `dash_sdk_signer_sign` to produce a signature, then immediately
///    destroys the per-call signer and zeroes the local key buffer.
/// 5. The signature is shipped back to Rust through the
///    `SignCompletionCallback`.
///
/// # Why round-trip through `dash_sdk_signer_*` (v1)
///
/// The `swift-sdk/CLAUDE.md` rule forbids *pipelines*
/// (mnemonic → seed → path → derive → sign) on the Swift side, **not**
/// "Swift briefly holds a key for a single sign call." Every signing path
/// — Rust-side, hardware-backed, anywhere — has to materialise the key
/// in process memory at some point. We keep the window narrow:
/// retrieve → sign → zero. v1 reuses the existing
/// `dash_sdk_signer_create_from_private_key` round-trip rather than
/// pulling in a Swift-native secp256k1 dependency just to start; v2 will
/// add `swift-secp256k1` so we can sign without ever building an FFI
/// signer per call.
///
/// TODO(KeychainSigner v2): replace the
/// `dash_sdk_signer_create_from_private_key` + `dash_sdk_signer_sign`
/// + `dash_sdk_signer_destroy` round-trip with a native-Swift
/// `secp256k1` sign, removing one allocation + one `Vec` copy per
/// signature and shrinking the key-material window further.
///
/// # Threading
///
/// `SignAsyncCallback` may fire from any Tokio worker thread. The
/// trampolines run synchronously to completion (the async-callback
/// model is for slow biometric prompts; v1 round-trip is fast enough
/// to invoke the completion before returning). The private background
/// `ModelContext` is created in `init` and pinned to this instance —
/// SwiftData `ModelContext` is not `Sendable` but we serialise access
/// to it through a single internal queue.
///
/// # Lifetime contract
///
/// `init` allocates a `*mut SignerHandle` via
/// `dash_sdk_signer_create_with_ctx` and registers `self` as the
/// opaque ctx via `Unmanaged.passUnretained(self).toOpaque()` —
/// a non-owning pointer. ARC alone controls when this object
/// deallocates; the destroy trampoline is a no-op (no extra
/// retain to balance) and `deinit` calls
/// `dash_sdk_signer_destroy` to free the Rust handle/vtable.
///
/// **Caller responsibility:** the `KeychainSigner` instance
/// MUST stay alive for the duration of any in-flight FFI call
/// that captured the handle. Async wallet APIs that take
/// `signer: KeychainSigner` perform the FFI work inside
/// `Task.detached`; the function parameter holds a strong
/// reference for the whole `await`, but each `Task.detached`
/// closure must additionally do `_ = signer` so the strong
/// reference is captured into the task and survives every
/// possible Swift compiler optimization of "unused after this
/// point". See the `_ = signer` keepalive lines in
/// `ManagedPlatformWallet.swift` call sites for examples.
///
/// Earlier revisions of this file used `passRetained`, which
/// created a circular ownership: Rust held a +1 retain on `self`,
/// the destroy trampoline only released it when the destroy FFI
/// fired, and the destroy FFI was only invoked from `deinit` —
/// which ARC could never enter while the +1 retain was alive.
/// Result: every `KeychainSigner` instance leaked forever. The
/// current `passUnretained` shape removes the leak at the cost
/// of the explicit keepalive contract above.
public final class KeychainSigner: Signer, @unchecked Sendable {
    // MARK: Public surface

    /// FFI signer handle. Pass to any `*_with_signer` entry point;
    /// the underlying pointer is the C-imported
    /// `OpaquePointer` from `platform-wallet-ffi.h`
    /// (and equivalently `rs-sdk-ffi.h`). Owned by this object —
    /// freed in `deinit` via `dash_sdk_signer_destroy`. Caller must
    /// keep the `KeychainSigner` alive for the duration of any FFI
    /// call that captured this pointer (see the keepalive contract
    /// above).
    public var handle: OpaquePointer {
        handlePtr
    }

    // MARK: Errors

    public enum Error: Swift.Error, LocalizedError {
        case publicKeyNotFound
        case privateKeyMissingFromKeychain(account: String)
        case ffiSignerCreationFailed(message: String)
        case ffiSignFailed(message: String)
        case modelContainerUnavailable
        /// `key_type == 0xFF` (platform-address) branch: no
        /// `PersistentPlatformAddress` row matched the supplied
        /// 20-byte address hash. Either the wallet hasn't synced
        /// the address pool yet or the row was deleted.
        case platformAddressNotFound(addressHashHex: String)
        /// `key_type == 0xFF` branch: the matched
        /// `PersistentPlatformAddress` row has no derivation path.
        /// Indicates a corrupt persister write — every row should
        /// carry a non-empty DIP-17 path.
        case derivationPathMissing(addressHashHex: String)
        /// `key_type == 0xFF` branch: the wallet-mnemonic Keychain
        /// item is missing for the wallet that owns this address.
        case mnemonicMissing(walletIdHex: String)
        /// `key_type == 0xFF` branch: the resolved
        /// `PersistentPlatformAddress.walletId` was not the
        /// expected 32-byte length. Indicates a corrupt persister
        /// write — every row should carry a wallet id matching
        /// the wallet's `walletId` field exactly.
        case walletIdInvalidLength(actual: Int, expected: Int)
        /// `dash_sdk_sign_with_mnemonic_and_path` returned a
        /// non-zero error tag. The tag is the byte written to
        /// `out_error` (see `SIGN_WITH_MNEMONIC_ERR_*` constants in
        /// `signer_simple.rs`).
        case signWithMnemonicFailed(tag: UInt8)

        public var errorDescription: String? {
            switch self {
            case .publicKeyNotFound:
                return "No PersistentPublicKey row matches the supplied public-key bytes."
            case .privateKeyMissingFromKeychain(let account):
                return "Keychain has no entry for account '\(account)'."
            case .ffiSignerCreationFailed(let message):
                return "Failed to construct FFI signer: \(message)"
            case .ffiSignFailed(let message):
                return "FFI sign call failed: \(message)"
            case .modelContainerUnavailable:
                return "ModelContainer is no longer available."
            case .platformAddressNotFound(let hashHex):
                return "No PersistentPlatformAddress row matches address hash \(hashHex)."
            case .derivationPathMissing(let hashHex):
                return "PersistentPlatformAddress row for \(hashHex) has no derivation path."
            case .mnemonicMissing(let walletIdHex):
                return "No Keychain mnemonic stored for wallet \(walletIdHex)."
            case .walletIdInvalidLength(let actual, let expected):
                return "PersistentPlatformAddress.walletId is \(actual) bytes; expected \(expected)."
            case .signWithMnemonicFailed(let tag):
                return "dash_sdk_sign_with_mnemonic_and_path failed with error tag \(tag)."
            }
        }
    }

    // MARK: Storage

    private let modelContainer: ModelContainer
    private let keychain: KeychainManager
    /// Network is only used for `dash_sdk_signer_create_from_private_key`,
    /// which uses it for WIF / address derivation but not for signing
    /// itself. Stored here so the trampoline doesn't have to plumb it.
    private let network: Network
    /// Background `ModelContext` pinned to this signer. Created lazily
    /// per-trampoline-call (see `lookupPrivateKey`) — SwiftData
    /// `ModelContext` instances are cheap and let us stay off the
    /// `@MainActor` even though the parent `ModelContainer` was
    /// constructed there.
    private let queue: DispatchQueue

    /// Resolver handle the platform-address signing path uses to
    /// fetch the wallet's mnemonic out of Keychain. The mnemonic
    /// flows IN to a Rust-owned `Zeroizing` buffer through the
    /// resolver's `resolve` callback; it never lives in a Swift
    /// `String` outside the trampoline's stack frame. Per
    /// swift-sdk/CLAUDE.md "no mnemonic round-tripping".
    private let mnemonicResolver: MnemonicResolver

    /// Raw pointer to the FFI signer handle. Boxed by Rust and freed
    /// in `deinit`.
    private var handlePtr: OpaquePointer!

    // MARK: Init

    /// - Parameters:
    ///   - modelContainer: the SwiftData container holding
    ///     `PersistentPublicKey` rows. We store a strong reference and
    ///     spawn ad-hoc background `ModelContext`s against it from
    ///     trampoline callbacks.
    ///   - network: forwarded to `dash_sdk_signer_create_from_private_key`
    ///     for WIF address derivation; does not affect signature output.
    ///   - keychain: defaults to `KeychainManager.shared`.
    ///   - storage: mnemonic source for the resolver-based signing
    ///     paths. Defaults to a fresh `WalletStorage()` — overridable
    ///     for tests.
    public init(
        modelContainer: ModelContainer,
        network: Network = .testnet,
        keychain: KeychainManager = .shared,
        storage: WalletStorage = WalletStorage()
    ) {
        self.modelContainer = modelContainer
        self.network = network
        self.keychain = keychain
        self.queue = DispatchQueue(label: "org.dashfoundation.swiftdashsdk.KeychainSigner")
        // One resolver per signer instance. Cheap to keep around —
        // it's just an opaque handle + a Swift-side `WalletStorage`
        // reference. Used by the platform-address signing branch.
        self.mnemonicResolver = MnemonicResolver(storage: storage)

        // Hand Rust an opaque NON-owning pointer to self. The
        // Swift owner is responsible for keeping `self` alive
        // for the duration of any in-flight FFI call that
        // captured the handle (the natural pattern: `let signer
        // = KeychainSigner(...)` followed by an `await
        // ...registerIdentity(...signer:)` keeps the local
        // `signer` alive until the await completes). See the
        // type-level "Lifetime" note for why `passRetained`
        // would leak.
        let ctx = Unmanaged.passUnretained(self).toOpaque()

        let handlePtr = dash_sdk_signer_create_with_ctx(
            ctx,
            keychainSignerSignAsyncTrampoline,
            keychainSignerCanSignTrampoline,
            keychainSignerDestroyTrampoline
        )
        precondition(
            handlePtr != nil,
            "dash_sdk_signer_create_with_ctx returned NULL"
        )
        self.handlePtr = handlePtr
    }

    deinit {
        // `dash_sdk_signer_destroy` drops the Rust handle box +
        // vtable allocation. The destroy trampoline is a no-op
        // (init used `passUnretained`, so there's nothing to
        // release). Caller must ensure no in-flight FFI calls
        // still reference `handlePtr` at the moment ARC fires
        // this `deinit`.
        if let handlePtr {
            dash_sdk_signer_destroy(handlePtr)
        }
    }

    // MARK: Trampoline-callable internals

    // MARK: key_type dispatch
    //
    // The Rust signer FFI sends one of two payload shapes through the
    // trampoline, distinguished by the `key_type` discriminant byte:
    //
    //   key_type 0…4   → `Signer<IdentityPublicKey>` flow.
    //                    `pubkey_bytes` are the raw IdentityPublicKey
    //                    `data()` bytes (33-byte compressed secp256k1
    //                    for ECDSA, 20-byte hash for ECDSA_HASH160,
    //                    etc.). Look up `PersistentPublicKey` →
    //                    `privateKeyKeychainIdentifier` → Keychain
    //                    bytes. The identity private key IS persisted
    //                    in the Keychain (primary storage).
    //
    //   key_type 0xFF  → `Signer<PlatformAddress>` flow (the
    //                    `SIGNER_KEY_TYPE_PLATFORM_ADDRESS_HASH`
    //                    constant in `rs-sdk-ffi/src/signer.rs`).
    //                    `pubkey_bytes` are the 20-byte address hash
    //                    of a `PlatformAddress::P2pkh` (or P2SH).
    //                    Platform-address private keys are NOT
    //                    persisted — they are derivation outputs of
    //                    `(mnemonic, derivation_path)` and exist only
    //                    for the duration of one signing call.
    //                    Resolution: SwiftData
    //                    `PersistentPlatformAddress.derivationPath`
    //                    + Keychain `WalletStorage.retrieveMnemonic` →
    //                    one-shot `dash_sdk_sign_with_mnemonic_and_path`
    //                    FFI which derives the key, signs, and zeroes
    //                    the buffers in-place. The derived bytes never
    //                    cross the FFI back to Swift.
    //
    // Picking 0xFF for the platform-address tag keeps the FFI ABI
    // single-byte and outside the standard `KeyType` enum range so
    // future `KeyType` additions can't collide. See the matching
    // `Signer<PlatformAddress>` impl on `VTableSigner` in Rust.

    /// Discriminant byte the Rust FFI uses when shipping a 20-byte
    /// platform-address hash through the trampoline instead of a raw
    /// IdentityPublicKey blob. Mirrors
    /// `SIGNER_KEY_TYPE_PLATFORM_ADDRESS_HASH` in `rs-sdk-ffi/src/signer.rs`.
    fileprivate static let platformAddressHashKeyType: UInt8 = 0xFF

    /// Identity-key lookup: walk SwiftData → Keychain identifier →
    /// Keychain bytes. Two-stage to handle both steady-state (after
    /// registration, both the row and the keychain item are in place)
    /// and mid-registration (the keychain item exists but the row is
    /// inserted later by the Rust persister callback).
    ///
    /// Used by the `key_type < 5` branch only. The platform-address
    /// branch (`key_type == 0xFF`) takes a different path — see
    /// [`signPlatformAddressOnDemand`] — because platform-address
    /// private keys are NEVER persisted; they're derived per call
    /// from `(mnemonic, path)` inside Rust.
    fileprivate func lookupIdentityPrivateKey(
        publicKey: Data
    ) -> Result<Data, Error> {
        var captured: Result<Data, Error> = .failure(.publicKeyNotFound)
        queue.sync {
            let context = ModelContext(self.modelContainer)
            let descriptor = FetchDescriptor<PersistentPublicKey>(
                predicate: #Predicate<PersistentPublicKey> { row in
                    row.publicKeyData == publicKey
                }
            )

            if let row = try? context.fetch(descriptor).first,
                let identifier = row.privateKeyKeychainIdentifier,
                let keyData = self.keychain.retrieveKeyData(identifier: identifier)
            {
                captured = .success(keyData)
                return
            }

            // Fallback: scan keychain entries by pubkey hex metadata.
            // Used during identity registration when the SwiftData row
            // hasn't landed yet but the keychain item has.
            let pubkeyHex = publicKey.map { String(format: "%02x", $0) }.joined()
            if let keyData = self.keychain.retrieveIdentityPrivateKey(publicKeyHex: pubkeyHex) {
                captured = .success(keyData)
                return
            }

            captured = .failure(.publicKeyNotFound)
        }
        return captured
    }

    /// Whether the mnemonic resolver derive-signs an identity `keyType`.
    ///
    /// Preflight-only. The sign trampoline itself does NOT consult this — it
    /// attempts the resolver unconditionally and lets Rust decide via the
    /// `UNSUPPORTED_KEY_TYPE` tag. But `canSign` has no data to sign and no
    /// data-free way to ask Rust the same question, so it approximates the
    /// resolver's supported set here to stay consistent with sign-time routing:
    /// a breadcrumb-only row (no stored scalar) is only reported signable when
    /// its type is one the resolver handles.
    ///
    /// Delegates the supported-set decision to the resolver's own FFI
    /// predicate `dash_sdk_resolver_supports_key_type`, so this preflight
    /// answer can never drift from the sign path's `UNSUPPORTED_KEY_TYPE`
    /// rejection — both read the one Rust source of truth.
    static func resolverCanDeriveSign(keyType: UInt8) -> Bool {
        dash_sdk_resolver_supports_key_type(keyType)
    }

    /// True iff this signer can produce a signature for the
    /// supplied `(publicKey, keyType)` pair. Mirrors the dispatch in
    /// [`signOnDemand`].
    ///
    /// For `keyType == 0xFF` we check that both the
    /// `PersistentPlatformAddress.derivationPath` and the
    /// per-wallet mnemonic Keychain item exist — the two inputs the
    /// derive-and-sign FFI requires. We do NOT actually derive a key
    /// here; the check is purely "are the prerequisites in place".
    func canSign(publicKey: Data, keyType: UInt8) -> Bool {
        if keyType == Self.platformAddressHashKeyType {
            // Resolve the address row first (synchronous lookup);
            // mnemonic check is gated on having the wallet id.
            // For `canSign` purposes, both "no row" and "corrupt
            // row with empty path" mean the same thing: we can't
            // sign for this address. The richer signing path
            // (`signPlatformAddressOnDemand`) distinguishes them
            // for diagnostic reasons.
            guard case .found(let resolved) =
                resolvePlatformAddressContext(addressHash: publicKey)
            else {
                return false
            }
            // Existence check only — do NOT materialize the mnemonic
            // bytes on the preflight path.
            return WalletStorage().hasMnemonic(for: resolved.walletId)
        }

        var found = false
        queue.sync {
            let context = ModelContext(self.modelContainer)
            let descriptor = FetchDescriptor<PersistentPublicKey>(
                predicate: #Predicate<PersistentPublicKey> { row in
                    row.publicKeyData == publicKey
                }
            )
            if let row = try? context.fetch(descriptor).first {
                // Stored scalar present (legacy / not-yet-backfilled key).
                if row.privateKeyKeychainIdentifier != nil {
                    found = true
                    return
                }
                // Resolver-derivable: a breadcrumb plus a readable mnemonic are
                // the two inputs `signIdentityKeyOnDemand` needs to derive-sign.
                // Match its `wid.count == 32` precondition so this preflight
                // doesn't report a corrupt-walletId row as signable. The key
                // type must also be one the resolver actually derive-signs — a
                // breadcrumb-only row (no stored scalar) whose type the resolver
                // rejects would pass preflight but fail at sign time with
                // `publicKeyNotFound` (the sign path routes an unsupported type
                // to the — here absent — stored scalar).
                if Self.resolverCanDeriveSign(keyType: keyType),
                    let wid = row.walletId,
                    wid.count == 32,
                    let path = row.identityDerivationPath,
                    !path.isEmpty,
                    WalletStorage().hasMnemonic(for: wid)
                {
                    found = true
                    return
                }
            }
            // Mirror `lookupIdentityPrivateKey`'s fallback: pre-
            // registration the SwiftData row may not exist yet but
            // the keychain item does.
            let pubkeyHex = publicKey.map { String(format: "%02x", $0) }.joined()
            if self.keychain.retrieveIdentityPrivateKey(publicKeyHex: pubkeyHex) != nil {
                found = true
            }
        }
        return found
    }

    /// Resolved context for a platform-address signing request:
    /// the matching `PersistentPlatformAddress`'s wallet id +
    /// derivation path. `nil` when no row matches the supplied
    /// 20-byte hash.
    fileprivate struct PlatformAddressContext {
        let walletId: Data
        let derivationPath: String
    }

    /// Result of a `resolvePlatformAddressContext` lookup.
    /// Distinguishes "no row matched" (the caller surfaces this as
    /// `.platformAddressNotFound` — e.g. address pool not yet
    /// synced) from "row exists but its `derivationPath` is empty"
    /// (a corrupt persister write — surfaced as
    /// `.derivationPathMissing` so the failure is diagnosable).
    fileprivate enum PlatformAddressResolution {
        case found(PlatformAddressContext)
        case noMatch
        case rowMatchedButPathEmpty
    }

    /// SwiftData lookup: 20-byte address hash →
    /// `(walletId, derivationPath)`. Pinned to the signer's serial
    /// queue + a per-call private `ModelContext` so the fetch is safe
    /// off the main actor.
    fileprivate func resolvePlatformAddressContext(
        addressHash: Data
    ) -> PlatformAddressResolution {
        var resolved: PlatformAddressResolution = .noMatch
        queue.sync {
            let context = ModelContext(self.modelContainer)
            let descriptor = FetchDescriptor<PersistentPlatformAddress>(
                predicate: #Predicate<PersistentPlatformAddress> { row in
                    row.addressHash == addressHash
                }
            )
            guard let row = try? context.fetch(descriptor).first else {
                resolved = .noMatch
                return
            }
            // Empty path = corrupt persister write
            // (`PersistentPlatformAddress.derivationPath` is
            // non-optional and should always carry a DIP-17 path).
            // Surface as a distinct case so the caller can map to
            // `.derivationPathMissing` rather than a misleading
            // `.platformAddressNotFound`.
            guard !row.derivationPath.isEmpty else {
                resolved = .rowMatchedButPathEmpty
                return
            }
            resolved = .found(PlatformAddressContext(
                walletId: row.walletId,
                derivationPath: row.derivationPath
            ))
        }
        return resolved
    }

    /// One-shot derive-and-sign for the `key_type == 0xFF` branch.
    /// Resolves the SwiftData `(walletId, derivationPath)` row and
    /// hands the Rust `dash_sdk_sign_with_mnemonic_resolver_and_path`
    /// FFI the wallet id + the resolver handle. The Rust side
    /// fires the resolver callback at the moment the mnemonic is
    /// needed — the mnemonic is copied into a Rust-owned
    /// `Zeroizing` buffer and the derived ECDSA key bytes never
    /// cross the FFI back to Swift. Only the signature does.
    ///
    /// Per swift-sdk/CLAUDE.md "no mnemonic round-tripping": the
    /// Swift caller does NOT pull the mnemonic into a `String`
    /// and hand it to a stateless FFI; the resolver callback is
    /// the only place the mnemonic crosses any boundary, and its
    /// lifetime is bounded by the trampoline's stack frame.
    fileprivate func signPlatformAddressOnDemand(
        addressHash: Data,
        keyType: UInt8,
        data: Data
    ) -> Result<Data, Error> {
        let hashHex = addressHash.map { String(format: "%02x", $0) }.joined()

        // 1. Look up `(walletId, derivationPath)` on a private
        //    background ModelContext (off the main actor). Mnemonic
        //    fetch is now Rust's job (via the resolver callback),
        //    not Swift's. The two failure modes — "no row matched"
        //    and "row exists but path is empty" — surface as
        //    distinct typed errors so a corrupt persister write is
        //    diagnosable rather than masquerading as an unsynced
        //    address pool.
        let ctx: PlatformAddressContext
        switch resolvePlatformAddressContext(addressHash: addressHash) {
        case .found(let resolved):
            ctx = resolved
        case .noMatch:
            return .failure(.platformAddressNotFound(addressHashHex: hashHex))
        case .rowMatchedButPathEmpty:
            return .failure(.derivationPathMissing(addressHashHex: hashHex))
        }

        // Validate the wallet-id length BEFORE shipping the
        // pointer across the FFI. Rust's
        // `dash_sdk_sign_with_mnemonic_resolver_and_path` reads
        // exactly 32 bytes from `wallet_id_bytes`; a truncated /
        // corrupt `PersistentPlatformAddress.walletId` row would
        // cause a read past the buffer rather than a clean
        // failure. Surface the precise reason here so the
        // explorer / log makes the corruption obvious.
        let expectedWalletIdLen = 32
        guard ctx.walletId.count == expectedWalletIdLen else {
            return .failure(.walletIdInvalidLength(
                actual: ctx.walletId.count,
                expected: expectedWalletIdLen
            ))
        }

        // Buffer sized generously (128B) — ECDSA compact-recoverable
        // signatures are 65 bytes; the cap leaves room for future
        // signature-shape additions without an ABI change. Defer-
        // scrubbed below regardless of success/failure.
        var sigBuf = [UInt8](repeating: 0, count: 128)
        var sigLen: UInt = 0
        var errTag: UInt8 = 0
        defer {
            sigBuf.withUnsafeMutableBufferPointer { ptr in
                if let base = ptr.baseAddress {
                    memset_s(UnsafeMutableRawPointer(base), ptr.count, 0, ptr.count)
                }
            }
        }

        // Platform addresses are always ECDSA secp256k1 P2PKH. The
        // `0xFF` value the trampoline received is the dispatch tag
        // (`platformAddressHashKeyType`), used only to route to this
        // branch — it is NOT a `KeyType` discriminant. Hardcode the
        // real key type here.
        let ecdsaSecp256k1KeyType: UInt8 = 0
        let rc = ctx.walletId.withUnsafeBytes { walletBytes -> Int32 in
            let walletPtr = walletBytes.bindMemory(to: UInt8.self).baseAddress
            return ctx.derivationPath.withCString { pPtr -> Int32 in
                return data.withUnsafeBytes { dataRaw -> Int32 in
                    return sigBuf.withUnsafeMutableBufferPointer { bufPtr -> Int32 in
                        let dataBase = dataRaw.bindMemory(to: UInt8.self).baseAddress
                        return dash_sdk_sign_with_mnemonic_resolver_and_path(
                            self.mnemonicResolver.handle,
                            walletPtr,
                            pPtr,
                            dataBase,
                            UInt(dataRaw.count),
                            ecdsaSecp256k1KeyType,
                            self.network.ffiValue,
                            // Address keys are bound by their own DIP-17
                            // derivation; no extra pubkey-binding needed.
                            nil,
                            0,
                            bufPtr.baseAddress,
                            UInt(bufPtr.count),
                            &sigLen,
                            &errTag
                        )
                    }
                }
            }
        }

        guard rc == 0 else {
            // `RESOLVER_NOT_FOUND` (9) is the recoverable
            // user-visible case — surface it through the existing
            // typed `mnemonicMissing` error so the UI message
            // stays specific.
            if errTag == SignWithMnemonicResolverError.resolverNotFound.rawValue {
                let walletHex = ctx.walletId.map { String(format: "%02x", $0) }.joined()
                return .failure(.mnemonicMissing(walletIdHex: walletHex))
            }
            return .failure(.signWithMnemonicFailed(tag: errTag))
        }

        // Copy out the leading `sigLen` bytes BEFORE the deferred
        // scrub erases them.
        let signature = Data(sigBuf.prefix(Int(sigLen)))
        return .success(signature)
    }

    /// SwiftData lookup: identity public-key bytes →
    /// `(walletId, identityDerivationPath)` breadcrumb. `nil` when the row
    /// is absent or carries no breadcrumb yet (an un-backfilled key) — the
    /// caller then falls back to the stored scalar. Pinned to the serial
    /// queue + a per-call `ModelContext`, like the platform-address resolver.
    func resolveIdentityKeyContext(
        publicKey: Data
    ) -> (walletId: Data, derivationPath: String)? {
        var resolved: (walletId: Data, derivationPath: String)?
        queue.sync {
            let context = ModelContext(self.modelContainer)
            let descriptor = FetchDescriptor<PersistentPublicKey>(
                predicate: #Predicate<PersistentPublicKey> { row in
                    row.publicKeyData == publicKey
                }
            )
            guard let row = try? context.fetch(descriptor).first,
                let wid = row.walletId,
                let path = row.identityDerivationPath,
                !path.isEmpty,
                wid.count == 32
            else {
                return
            }
            resolved = (wid, path)
        }
        return resolved
    }

    /// One-shot derive-and-sign for an identity key (`keyType < 5`), the
    /// derive-sign-destroy counterpart of [`signPlatformAddressOnDemand`].
    /// Resolves the key's `(walletId, derivationPath)` breadcrumb and signs
    /// via the resolver, passing the on-chain key bytes as the binding so the
    /// FFI rejects (before signing) if the key derived at the path doesn't
    /// reproduce this exact key.
    ///
    /// Returns `nil` when the key carries no breadcrumb yet — the trampoline
    /// then falls back to the stored scalar so an un-backfilled key still
    /// signs. A `.failure` means a breadcrumb was present but the resolver
    /// couldn't sign (mnemonic missing, binding mismatch); the trampoline
    /// logs it and still falls back to the verified stored scalar.
    func signIdentityKeyOnDemand(
        publicKey: Data,
        keyType: UInt8,
        data: Data
    ) -> Result<Data, Error>? {
        guard let ctx = resolveIdentityKeyContext(publicKey: publicKey) else {
            return nil
        }

        var sigBuf = [UInt8](repeating: 0, count: 128)
        var sigLen: UInt = 0
        var errTag: UInt8 = 0
        defer {
            sigBuf.withUnsafeMutableBufferPointer { ptr in
                if let base = ptr.baseAddress {
                    memset_s(UnsafeMutableRawPointer(base), ptr.count, 0, ptr.count)
                }
            }
        }

        let rc = ctx.walletId.withUnsafeBytes { walletBytes -> Int32 in
            let walletPtr = walletBytes.bindMemory(to: UInt8.self).baseAddress
            return ctx.derivationPath.withCString { pPtr -> Int32 in
                return data.withUnsafeBytes { dataRaw -> Int32 in
                    return publicKey.withUnsafeBytes { expRaw -> Int32 in
                        return sigBuf.withUnsafeMutableBufferPointer { bufPtr -> Int32 in
                            let dataBase = dataRaw.bindMemory(to: UInt8.self).baseAddress
                            let expBase = expRaw.bindMemory(to: UInt8.self).baseAddress
                            return dash_sdk_sign_with_mnemonic_resolver_and_path(
                                self.mnemonicResolver.handle,
                                walletPtr,
                                pPtr,
                                dataBase,
                                UInt(dataRaw.count),
                                keyType,
                                self.network.ffiValue,
                                expBase,
                                UInt(expRaw.count),
                                bufPtr.baseAddress,
                                UInt(bufPtr.count),
                                &sigLen,
                                &errTag
                            )
                        }
                    }
                }
            }
        }

        guard rc == 0 else {
            if errTag == SignWithMnemonicResolverError.resolverNotFound.rawValue {
                let walletHex = ctx.walletId.map { String(format: "%02x", $0) }.joined()
                return .failure(.mnemonicMissing(walletIdHex: walletHex))
            }
            return .failure(.signWithMnemonicFailed(tag: errTag))
        }

        let signature = Data(sigBuf.prefix(Int(sigLen)))
        return .success(signature)
    }

    /// v1 sign primitive. Delegates to `RawKeySigner.sign` (the shared
    /// create-signer → sign → destroy round-trip; the key copy is zeroed
    /// there), mapping its typed errors onto this signer's error space.
    ///
    /// TODO(KeychainSigner v2): replace `RawKeySigner`'s FFI round-trip
    /// with a native-Swift `secp256k1` invocation once we add the
    /// `swift-secp256k1` SPM dep.
    fileprivate func ffiSign(
        privateKey: Data,
        data: Data
    ) -> Result<Data, Error> {
        do {
            return .success(
                try RawKeySigner.sign(data: data, privateKey: privateKey, network: self.network))
        } catch KeyManagerError.signerCreationFailed(let message) {
            return .failure(.ffiSignerCreationFailed(message: message))
        } catch KeyManagerError.invalidKeyFormat(let message) {
            return .failure(.ffiSignerCreationFailed(message: "invalid key format: \(message)"))
        } catch KeyManagerError.signingFailed(let message) {
            return .failure(.ffiSignFailed(message: message))
        } catch {
            return .failure(.ffiSignFailed(message: String(describing: error)))
        }
    }

    // MARK: - Signer protocol conformance (legacy)

    public func canSign(identityPublicKey: Data) -> Bool {
        canSign(publicKey: identityPublicKey, keyType: KeyType.ecdsaSecp256k1.rawValue)
    }
}

// MARK: - C-ABI trampolines

/// `SignAsyncCallback` trampoline. Resolves the owning Swift instance
/// from the opaque `ctx` (which holds a `+1`-retained `KeychainSigner`),
/// performs the SwiftData lookup + sign, and invokes the Rust
/// completion synchronously before returning.
///
/// Synchronous completion is fine here — the async-callback model is
/// designed for slow biometric prompts. v1 sign is fast enough that
/// dragging a thread back to a Tokio worker via `oneshot` is wasted
/// motion.
private func keychainSignerSignAsyncTrampoline(
    ctx: UnsafeRawPointer?,
    pubkeyBytes: UnsafePointer<UInt8>?,
    pubkeyLen: UInt,
    keyType: UInt8,
    data: UnsafePointer<UInt8>?,
    dataLen: UInt,
    completionCtx: UnsafeMutableRawPointer?,
    completion: SignCompletionCallback?
) {
    guard let ctx, let completion else { return }
    let signer = Unmanaged<KeychainSigner>.fromOpaque(ctx).takeUnretainedValue()

    let pubkeyData: Data
    if let pubkeyBytes, pubkeyLen > 0 {
        pubkeyData = Data(bytes: pubkeyBytes, count: Int(pubkeyLen))
    } else {
        pubkeyData = Data()
    }
    let dataToSign: Data
    if let data, dataLen > 0 {
        dataToSign = Data(bytes: data, count: Int(dataLen))
    } else {
        dataToSign = Data()
    }

    func reportError(_ message: String, code: Int32 = KeychainSignerCompletionErrorCode.generic) {
        // C strings have to outlive the call. `withCString` does this
        // for us. `code` is the structured DashSDKSignerErrorCode
        // discriminator (dashpay/platform#4060 finding 7).
        message.withCString { errPtr in
            completion(completionCtx, nil, 0, code, errPtr)
        }
    }

    func reportSuccess(_ sig: Data) {
        sig.withUnsafeBytes { sigBuf in
            let base = sigBuf.bindMemory(to: UInt8.self).baseAddress
            completion(completionCtx, base, UInt(sigBuf.count), 0, nil)
        }
    }

    // Dispatch on key_type. Platform-address signing (`0xFF`) is a
    // single-call derive-and-sign path — no separate key lookup,
    // because the derived bytes never come back to Swift. Identity
    // signing (`< 5`) prefers the same derive-sign-destroy path (from
    // the key's stored breadcrumb) and falls back to the stored scalar
    // when a key has no breadcrumb yet.
    if keyType == KeychainSigner.platformAddressHashKeyType {
        switch signer.signPlatformAddressOnDemand(
            addressHash: pubkeyData,
            keyType: keyType,
            data: dataToSign
        ) {
        case .failure(let err):
            reportError(err.localizedDescription)
        case .success(let sig):
            reportSuccess(sig)
        }
        return
    }

    // Identity signing (`keyType < 5`): derive-sign-destroy via the resolver
    // when the key carries a derivation breadcrumb; otherwise fall back to the
    // stored scalar. The fallback keeps already-materialized keys — and any not
    // yet backfilled — signable, so the cutover is non-lockout by construction.
    // Every fallback is logged so the zero-fallback acceptance gate can catch
    // un-migrated rows or resolver failures before the stored scalar is removed.
    //
    // Rust owns the supported-key-type decision: we attempt the resolver for
    // any identity key and treat its `UNSUPPORTED_KEY_TYPE` tag as the routing
    // signal (fall through to the stored scalar silently, no fallback log —
    // the resolver simply doesn't handle this type). This avoids mirroring the
    // Rust ECDSA-only set in Swift, so a future Rust-derivable key type is
    // automatically routed through the resolver without a matching Swift edit.
    switch signer.signIdentityKeyOnDemand(
        publicKey: pubkeyData,
        keyType: keyType,
        data: dataToSign
    ) {
    case .success(let sig)?:
        reportSuccess(sig)
        return
    case .failure(.signWithMnemonicFailed(let tag))?
    where tag == SignWithMnemonicResolverError.unsupportedKeyType.rawValue:
        // Rust does not derive-sign this key type — route to the stored
        // scalar with no spurious fallback log.
        break
    case .failure(let err)?:
        print("⚠️ IDENTITY_SIGN_FALLBACK resolver-failed: \(err.localizedDescription)")
    case nil:
        print("⚠️ IDENTITY_SIGN_FALLBACK no-breadcrumb")
    }

    let privateKey: Data
    switch signer.lookupIdentityPrivateKey(publicKey: pubkeyData) {
    case .failure(let err):
        // "No stored key" outcomes carry the structured
        // SigningKeyUnavailable code so hosts get the typed
        // PlatformWalletError.signingKeyUnavailable without message
        // sniffing (dashpay/platform#4060 finding 7).
        reportError(
            err.localizedDescription,
            code: keychainSignerCompletionErrorCode(for: err)
        )
        return
    case .success(let priv):
        privateKey = priv
    }

    switch signer.ffiSign(privateKey: privateKey, data: dataToSign) {
    case .failure(let err):
        reportError(err.localizedDescription)
    case .success(let sig):
        reportSuccess(sig)
    }
}

private func keychainSignerCanSignTrampoline(
    ctx: UnsafeRawPointer?,
    pubkeyBytes: UnsafePointer<UInt8>?,
    pubkeyLen: UInt,
    keyType: UInt8
) -> Bool {
    guard let ctx else { return false }
    let signer = Unmanaged<KeychainSigner>.fromOpaque(ctx).takeUnretainedValue()
    let pubkey: Data
    if let pubkeyBytes, pubkeyLen > 0 {
        pubkey = Data(bytes: pubkeyBytes, count: Int(pubkeyLen))
    } else {
        pubkey = Data()
    }
    return signer.canSign(publicKey: pubkey, keyType: keyType)
}

/// Vtable destructor — invoked exactly once when the FFI handle is
/// destroyed. No-op now that init uses `passUnretained`: there is
/// no extra retain to balance, and ARC has already deallocated
/// (or is in the process of deallocating) `self` by the time
/// `dash_sdk_signer_destroy` runs from `deinit`. Kept around so
/// the Rust vtable's `destroy` slot is always non-null.
private func keychainSignerDestroyTrampoline(_: UnsafeMutableRawPointer?) {}

/// Mirror of `rs-sdk-ffi`'s `DashSDKSignerErrorCode` — the structured
/// completion-failure discriminator (dashpay/platform#4060 finding 7).
/// Only `generic` and `signingKeyUnavailable` are emitted today.
enum KeychainSignerCompletionErrorCode {
    static let generic: Int32 = 0
    static let signingKeyUnavailable: Int32 = 1
}

/// Classify a `KeychainSigner.Error` for the completion's structured
/// `error_code`: the "no stored key" outcomes — missing row/scalar or a
/// keychain entry the identifier no longer resolves — are
/// `signingKeyUnavailable`; everything else stays `generic`.
func keychainSignerCompletionErrorCode(for error: KeychainSigner.Error) -> Int32 {
    switch error {
    case .publicKeyNotFound, .privateKeyMissingFromKeychain:
        return KeychainSignerCompletionErrorCode.signingKeyUnavailable
    default:
        return KeychainSignerCompletionErrorCode.generic
    }
}
