import Foundation
import DashSDKFFI

/// A managed wallet created by `PlatformWalletManager`.
///
/// Provides access to sub-wallets (platform addresses, core, identity, etc.)
/// and lock-free balance reads.
///
/// `@unchecked Sendable`: the only instance state is an immutable
/// `Handle` (UInt64) and an immutable 32-byte `walletId`. All mutable
/// state lives behind the Rust-side `Arc<RwLock<WalletManager<...>>>`,
/// which is already thread-safe. Making the class Sendable lets call
/// sites `await` async FFI methods from `@MainActor` contexts without
/// having to juggle `Task.detached { ... }` wrappers.
public final class ManagedPlatformWallet: @unchecked Sendable {
    let handle: Handle

    /// The 32-byte wallet identifier.
    public let walletId: Data

    init(handle: Handle, walletId: Data) {
        self.handle = handle
        self.walletId = walletId
    }

    deinit {
        _ = platform_wallet_destroy(handle)
    }

    // MARK: - Balance (lock-free)

    /// Wallet balance breakdown. These are atomic reads — no lock contention.
    public struct WalletBalance {
        public let spendable: UInt64
        public let unconfirmed: UInt64
        public let immature: UInt64
        public let locked: UInt64

        public var total: UInt64 {
            spendable + unconfirmed + immature + locked
        }
    }

    /// Read the current balance (lock-free atomic reads).
    public func balance() throws -> WalletBalance {
        var spendable: UInt64 = 0
        var unconfirmed: UInt64 = 0
        var immature: UInt64 = 0
        var locked: UInt64 = 0

        let result = platform_wallet_get_balance(
            handle,
            &spendable,
            &unconfirmed,
            &immature,
            &locked
        )

        try result.check()

        return WalletBalance(
            spendable: spendable,
            unconfirmed: unconfirmed,
            immature: immature,
            locked: locked
        )
    }

    // MARK: - Sub-wallet access

    /// Get the platform address wallet for BLAST sync, transfers, and withdrawals.
    ///
    /// Each call returns a new handle (cheap clone — all Arc internals).
    /// The caller is responsible for the returned object's lifetime.
    public func platformAddressWallet() throws -> ManagedPlatformAddressWallet {
        var platformHandle: Handle = NULL_HANDLE

        let result = platform_wallet_get_platform(handle, &platformHandle)

        try result.check()

        return ManagedPlatformAddressWallet(handle: platformHandle)
    }

    /// Get the core wallet for UTXO management, addresses, and transactions.
    public func coreWallet() throws -> ManagedCoreWallet {
        var coreHandle: Handle = NULL_HANDLE

        let result = platform_wallet_get_core(handle, &coreHandle)
        try result.check()

        return ManagedCoreWallet(handle: coreHandle)
    }

    /// Get the asset lock manager for building and tracking asset locks.
    public func assetLockManager() throws -> ManagedAssetLockManager {
        var assetLockHandle: Handle = NULL_HANDLE

        let result = platform_wallet_get_asset_locks(handle, &assetLockHandle)
        try result.check()

        return ManagedAssetLockManager(handle: assetLockHandle)
    }

    // MARK: - Persistence

    /// Flush all queued changesets to the storage backend.
    public func flushPersist() throws {
        let result = platform_wallet_flush_persist(handle)

        try result.check()
    }

    /// Load persisted state and apply it to the in-memory wallet.
    public func loadAndApplyPersisted() throws {
        let result = platform_wallet_load_and_apply_persisted(handle)

        try result.check()
    }

    // MARK: - Identity registration (address-funded)

    /// One contributing address for an address-funded identity.
    ///
    /// Nonces are resolved on the Rust side at submit time (the SDK
    /// fetches each address's on-chain nonce right before building
    /// the transition), so callers do not need to track them.
    public struct IdentityAddressInput: Sendable {
        /// `0` = P2PKH, `1` = P2SH. Mirrors the Rust-side
        /// `PlatformAddress` discriminant.
        public let addressType: UInt8
        /// 20-byte address hash.
        public let hash: Data
        /// Credits to spend from this address.
        public let credits: UInt64

        public init(addressType: UInt8, hash: Data, credits: UInt64) {
            self.addressType = addressType
            self.hash = hash
            self.credits = credits
        }
    }

    /// Optional refund output paired with the address inputs.
    public struct IdentityAddressOutput: Sendable {
        public let addressType: UInt8
        public let hash: Data
        public let credits: UInt64

        public init(addressType: UInt8, hash: Data, credits: UInt64) {
            self.addressType = addressType
            self.hash = hash
            self.credits = credits
        }
    }

    /// One already-derived authentication public key the caller wants
    /// the new identity to register. Marshalled into `IdentityPubkeyFFI`
    /// at the FFI boundary by `registerIdentityFromAddresses`.
    ///
    /// Pre-derived in Swift via `prePersistIdentityKeysForRegistration`
    /// (which routes through `dash_sdk_derive_identity_keys_from_mnemonic`,
    /// the watch-only-safe derivation FFI). Threading these through to
    /// Rust avoids the prior pubkey-derivation pass inside
    /// `platform_wallet_register_identity_with_signer` that fails on
    /// watch-only wallets where Rust has no in-process xpriv loaded.
    public struct IdentityPubkey: Sendable {
        public let keyId: UInt32
        public let keyType: KeyType
        public let purpose: KeyPurpose
        public let securityLevel: SecurityLevel
        /// Serialized public key bytes (33 bytes for compressed
        /// secp256k1; 48 for BLS; etc.).
        public let pubkeyBytes: Data
        public let readOnly: Bool
        /// Optional contract-bounds restriction. Bounds are valid
        /// for any purpose only when present and only if consensus
        /// allows that purpose / contract / document-type shape.
        /// `nil` is valid for every purpose, including Encryption /
        /// Decryption keys.
        public let contractBounds: ContractBounds?

        public init(
            keyId: UInt32,
            keyType: KeyType,
            purpose: KeyPurpose,
            securityLevel: SecurityLevel,
            pubkeyBytes: Data,
            readOnly: Bool = false,
            contractBounds: ContractBounds? = nil
        ) {
            self.keyId = keyId
            self.keyType = keyType
            self.purpose = purpose
            self.securityLevel = securityLevel
            self.pubkeyBytes = pubkeyBytes
            self.readOnly = readOnly
            self.contractBounds = contractBounds
        }
    }

    /// Swift mirror of `dpp::identity::identity_public_key::contract_bounds::ContractBounds`.
    /// Pinned to two variants (no `MultipleContractsOfSameOwner`)
    /// to match the Rust enum's currently-supported shape.
    public enum ContractBounds: Sendable, Equatable {
        /// Key may be used within a specific contract (any
        /// document type). Maps to `kind == 1` on the FFI side.
        case singleContract(id: Data)
        /// Key may be used within a specific contract AND a
        /// specific document type. Maps to `kind == 2` on the
        /// FFI side.
        case singleContractDocumentType(id: Data, documentTypeName: String)
    }

    /// Inspectable fields of a parsed raw `IdentityUpdateTransition`.
    /// The keys intentionally reuse `IdentityPubkey` so callers can
    /// validate and hand them back to `updateIdentity(...)` unchanged.
    public struct ParsedIdentityUpdateTransition: Sendable {
        public let identityId: Identifier
        public let addPublicKeys: [IdentityPubkey]
        public let disablePublicKeyIds: [UInt32]

        public init(
            identityId: Identifier,
            addPublicKeys: [IdentityPubkey],
            disablePublicKeyIds: [UInt32]
        ) {
            self.identityId = identityId
            self.addPublicKeys = addPublicKeys
            self.disablePublicKeyIds = disablePublicKeyIds
        }
    }

    /// Result of a successful identity registration.
    public struct CreatedIdentity: Sendable {
        /// 32-byte identity id.
        public let identityId: Data
        /// Fully-populated `ManagedIdentity` wrapping the new
        /// identity's Rust-side handle. Carries the DPP `Identity`
        /// (public keys, balance, revision) plus the wallet-level
        /// metadata (`identity_index`, labels, block-time stamps).
        /// The returned handle is owned by this object — it is
        /// released automatically when the `ManagedIdentity`
        /// deinitializes.
        public let identity: ManagedIdentity
    }

    /// Register a new identity funded by Platform-address balances,
    /// using TWO external `SignerHandle`s — one for the new identity's
    /// state-transition keys, one for the input platform addresses'
    /// funding-contribution signatures.
    ///
    /// This is the preferred path: the wallet's mnemonic stays in
    /// Keychain throughout, and every signature crosses the FFI
    /// through one of the supplied signers (typically two views of
    /// the same `KeychainSigner` — see the convenience overload
    /// below). Per `swift-sdk/CLAUDE.md`, the seed must not cross
    /// the FFI boundary just so Rust can finish an operation.
    ///
    /// The two-handle FFI shape unblocks watch-only wallets and
    /// future Keychain-backed platform-address keys (each role can
    /// route to its own backing store) without an ABI change later.
    ///
    /// - Parameters:
    ///   - inputs: contributing address rows describing each input
    ///     platform address + the credit amount to spend from it.
    ///   - output: optional refund output.
    ///   - identityIndex: BIP-9 identity index in the HD tree.
    ///   - identityPubkeys: Already-derived authentication pubkeys
    ///     for the new identity (typically produced by
    ///     `prePersistIdentityKeysForRegistration`). The first row
    ///     should be the MASTER key; the remainder are HIGH-security
    ///     authentication keys. Threading the pubkeys in unblocks
    ///     watch-only wallets — Rust no longer needs the seed to
    ///     build the placeholder identity.
    ///   - identitySigner: signer whose `.handle` produces signatures
    ///     for the new identity's authentication keys. Borrowed for
    ///     the duration of the call.
    ///   - addressSigner: signer whose `.handle` produces signatures
    ///     for each input platform address. Borrowed for the
    ///     duration of the call. May be the same instance as
    ///     `identitySigner`.
    public func registerIdentityFromAddresses(
        inputs: [IdentityAddressInput],
        output: IdentityAddressOutput?,
        identityIndex: UInt32,
        identityPubkeys: [IdentityPubkey],
        identitySigner: KeychainSigner,
        addressSigner: KeychainSigner
    ) async throws -> CreatedIdentity {
        guard !inputs.isEmpty else {
            throw PlatformWalletError.invalidParameter("inputs is empty")
        }
        guard !identityPubkeys.isEmpty else {
            throw PlatformWalletError.invalidParameter("identityPubkeys is empty")
        }
        // Reject malformed address hashes up front for the same
        // reason `topUpFromAddresses` does — `.prefix(20)` below
        // would silently truncate / zero-pad and point the FFI at
        // a different address.
        for input in inputs {
            guard input.hash.count == 20 else {
                throw PlatformWalletError.invalidParameter(
                    "input hash must be 20 bytes, got \(input.hash.count)"
                )
            }
        }
        if let output, output.hash.count != 20 {
            throw PlatformWalletError.invalidParameter(
                "output hash must be 20 bytes, got \(output.hash.count)"
            )
        }

        let handle = self.handle
        let identitySignerHandle = identitySigner.handle
        let addressSignerHandle = addressSigner.handle

        let (idData, identityHandle): (Data, Handle) = try await Task.detached(
            priority: .userInitiated
        ) { () -> (Data, Handle) in
            // Keepalive — KeychainSigner now uses
            // `passUnretained`, so the Rust ctx pointer dangles
            // unless we keep the Swift owners alive across this
            // detached work. Implicit-capturing `identitySigner`
            // and `addressSigner` here forces strong references
            // for the duration of the task; the trampoline can
            // safely deref the ctx until this closure returns.
            _ = identitySigner
            _ = addressSigner
            let ffiInputs = inputs.map { input -> IdentityFundingInputFFI in
                var hashTuple = hashTupleInit()
                withUnsafeMutableBytes(of: &hashTuple) { raw in
                    let src = input.hash.prefix(20)
                    for (i, byte) in src.enumerated() {
                        raw[i] = byte
                    }
                }
                return IdentityFundingInputFFI(
                    address_type: input.addressType,
                    hash: hashTuple,
                    credits: input.credits
                )
            }

            var outputFFI: IdentityFundingOutputFFI = {
                if let output {
                    var hashTuple = hashTupleInit()
                    withUnsafeMutableBytes(of: &hashTuple) { raw in
                        let src = output.hash.prefix(20)
                        for (i, byte) in src.enumerated() {
                            raw[i] = byte
                        }
                    }
                    return IdentityFundingOutputFFI(
                        has_output: true,
                        address_type: output.addressType,
                        hash: hashTuple,
                        credits: output.credits
                    )
                } else {
                    return IdentityFundingOutputFFI(
                        has_output: false,
                        address_type: 0,
                        hash: hashTupleInit(),
                        credits: 0
                    )
                }
            }()

            var outIdentityId: (
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
            ) = (
                0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0
            )
            var outIdentityHandle: Handle = NULL_HANDLE

            // Stage each pubkey into a contiguous, owning buffer so
            // the `pubkey_bytes` pointers we hand to Rust stay valid
            // for the full duration of the FFI call. Building two
            // parallel arrays — one of `Data` keeping the bytes
            // alive, one of `IdentityPubkeyFFI` rows pointing into
            // them — and pinning both via nested
            // `withUnsafeBufferPointer` is the simplest shape that
            // matches the Rust side's borrow-for-the-call contract.
            let pubkeyBuffers: [Data] = identityPubkeys.map { $0.pubkeyBytes }

            let result = ffiInputs.withUnsafeBufferPointer { inputsBuf in
                withUnsafePointer(to: &outputFFI) { outputPtr in
                    pubkeyBuffers.withUnsafeBufferPointer { _ -> PlatformWalletFFIResult in
                        // For each row, withUnsafeBytes pins ONE Data
                        // at a time, so we have to assemble the FFI
                        // row array under nested pinning. We use a
                        // recursive helper-via-fold by building the
                        // array of FFI rows lazily through nested
                        // `withUnsafeBytes` calls.
                        return Self.withPubkeyFFIArray(
                            identityPubkeys,
                            buffers: pubkeyBuffers
                        ) { ffiRowsPtr, ffiRowsCount in
                            platform_wallet_register_identity_with_signer(
                                handle,
                                identityIndex,
                                ffiRowsPtr,
                                UInt(ffiRowsCount),
                                identitySignerHandle,
                                addressSignerHandle,
                                inputsBuf.baseAddress,
                                UInt(inputsBuf.count),
                                outputPtr,
                                &outIdentityId,
                                &outIdentityHandle
                            )
                        }
                    }
                }
            }

            try result.check()
            guard outIdentityHandle != NULL_HANDLE else {
                throw PlatformWalletError.walletOperation("identity handle not returned")
            }

            let idData = withUnsafeBytes(of: outIdentityId) { Data($0) }
            return (idData, outIdentityHandle)
        }.value

        return CreatedIdentity(
            identityId: idData,
            identity: ManagedIdentity(handle: identityHandle)
        )
    }

    /// Convenience overload — uses the same `KeychainSigner` for both
    /// the identity-key role and the platform-address role. The
    /// trampoline dispatches by `key_type` byte (KeyType discriminants
    /// 0–4 → `PersistentPublicKey` lookup; `0xFF` → 20-byte address
    /// hash lookup), so a single Swift signer can serve both. This
    /// matches today's iOS wallet shape; the two-signer overload
    /// above is the building block for future watch-only / hardware
    /// flows.
    public func registerIdentityFromAddresses(
        inputs: [IdentityAddressInput],
        output: IdentityAddressOutput?,
        identityIndex: UInt32,
        identityPubkeys: [IdentityPubkey],
        signer: KeychainSigner
    ) async throws -> CreatedIdentity {
        try await registerIdentityFromAddresses(
            inputs: inputs,
            output: output,
            identityIndex: identityIndex,
            identityPubkeys: identityPubkeys,
            identitySigner: signer,
            addressSigner: signer
        )
    }

    // MARK: - Identity top-up (address-funded)

    /// Top up an existing identity's credit balance from one or more
    /// Platform addresses, using an external `KeychainSigner` for the
    /// per-address funding signatures.
    ///
    /// Top-up state-transitions are signed entirely with the input
    /// addresses' private keys (no IdentityCreate to sign), so unlike
    /// `registerIdentityFromAddresses` only one signer is required —
    /// the same `KeychainSigner` you use for registration's
    /// `addressSigner` role.
    ///
    /// - Parameters:
    ///   - identityId: 32-byte identity id of the existing identity
    ///     to top up.
    ///   - inputs: contributing address rows describing each input
    ///     platform address + the credit amount to spend from it.
    ///   - addressSigner: signer whose `.handle` produces signatures
    ///     for each input platform address. Borrowed for the
    ///     duration of the call.
    ///
    /// - Returns: the new credit balance reported by Platform.
    public func topUpFromAddresses(
        identityId: Data,
        inputs: [IdentityAddressInput],
        addressSigner: KeychainSigner
    ) async throws -> UInt64 {
        guard identityId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "identityId must be 32 bytes, got \(identityId.count)"
            )
        }
        guard !inputs.isEmpty else {
            throw PlatformWalletError.invalidParameter("inputs is empty")
        }
        // Reject malformed address hashes up front. Earlier
        // revisions used `.prefix(20)` on the FFI build below,
        // which silently truncates oversize hashes and zero-pads
        // undersized ones — pointing the FFI at a different
        // address than the caller intended. A clean precondition
        // here surfaces the failure as a recoverable error.
        for input in inputs {
            guard input.hash.count == 20 else {
                throw PlatformWalletError.invalidParameter(
                    "input hash must be 20 bytes, got \(input.hash.count)"
                )
            }
        }

        let handle = self.handle
        let addressSignerHandle = addressSigner.handle

        return try await Task.detached(priority: .userInitiated) { () -> UInt64 in
            // Keepalive — see `registerIdentityFromAddresses` for
            // rationale. The trampoline ctx pointer dangles unless
            // the Swift owner stays alive across this detached work.
            _ = addressSigner

            var idTuple: (
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
            ) = (
                0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0
            )
            withUnsafeMutableBytes(of: &idTuple) { raw in
                for (i, byte) in identityId.prefix(32).enumerated() {
                    raw[i] = byte
                }
            }

            let ffiInputs = inputs.map { input -> IdentityFundingInputFFI in
                var hashTuple = hashTupleInit()
                withUnsafeMutableBytes(of: &hashTuple) { raw in
                    let src = input.hash.prefix(20)
                    for (i, byte) in src.enumerated() {
                        raw[i] = byte
                    }
                }
                return IdentityFundingInputFFI(
                    address_type: input.addressType,
                    hash: hashTuple,
                    credits: input.credits
                )
            }

            var newBalance: UInt64 = 0

            let result = ffiInputs.withUnsafeBufferPointer { inputsBuf in
                withUnsafePointer(to: &idTuple) { idPtr in
                    platform_wallet_top_up_from_addresses_with_signer(
                        handle,
                        idPtr,
                        inputsBuf.baseAddress,
                        UInt(inputsBuf.count),
                        addressSignerHandle,
                        &newBalance
                    )
                }
            }

            try result.check()
            return newBalance
        }.value
    }

    /// Pin every pubkey buffer simultaneously and call `body` with a
    /// freshly-built `[IdentityPubkeyFFI]` whose `pubkey_bytes`
    /// pointers all reference the pinned bytes. Recursive shape
    /// because Swift's `withUnsafeBytes` only pins one `Data` at a
    /// time — recursing pins them in order, then runs `body` once at
    /// the deepest frame where every buffer is alive simultaneously.
    ///
    /// `pubkeys[i]` must align with `buffers[i]` (the latter is a
    /// pre-extracted `[Data]` of the same `pubkeyBytes` values, kept
    /// separately so the recursive helper doesn't need to see the
    /// full Swift wrapper struct).
    // `internal` (not `fileprivate`) so the shielded identity-create-from-pool wrapper in
    // `PlatformWalletManagerShieldedSync.swift` can reuse this exact `[IdentityPubkeyFFI]` pinning
    // helper rather than duplicating the recursive lifetime dance.
    static func withPubkeyFFIArray<R>(
        _ pubkeys: [IdentityPubkey],
        buffers: [Data],
        _ body: (UnsafePointer<IdentityPubkeyFFI>?, Int) -> R
    ) -> R {
        precondition(pubkeys.count == buffers.count, "pubkeys / buffers length mismatch")
        var rows: [IdentityPubkeyFFI] = []
        rows.reserveCapacity(pubkeys.count)
        return pinNext(0, &rows, pubkeys, buffers, body)
    }

    /// Inner recursion for `withPubkeyFFIArray`. `index` advances
    /// through `pubkeys`; on each step we pin the next `Data` via
    /// `withUnsafeBytes`, append a matching FFI row, and recurse.
    /// At index == count we hand the assembled row array off to
    /// `body` under one combined pinning frame.
    ///
    /// Contract-bounds pinning extends the same pattern: when the
    /// row carries `.singleContract` or `.singleContractDocumentType`
    /// we open a nested `withUnsafeBytes` (for the 32-byte contract
    /// id) and a `withCString` (for the document type, if any) so
    /// the pointers we hand the FFI stay valid for the entire
    /// `body` invocation. Rows without bounds drop straight through
    /// to the next level of recursion.
    private static func pinNext<R>(
        _ index: Int,
        _ rows: inout [IdentityPubkeyFFI],
        _ pubkeys: [IdentityPubkey],
        _ buffers: [Data],
        _ body: (UnsafePointer<IdentityPubkeyFFI>?, Int) -> R
    ) -> R {
        if index == pubkeys.count {
            return rows.withUnsafeBufferPointer { rowsBuf in
                body(rowsBuf.baseAddress, rowsBuf.count)
            }
        }
        let pk = pubkeys[index]
        return buffers[index].withUnsafeBytes { (raw: UnsafeRawBufferPointer) -> R in
            let basePtr = raw.bindMemory(to: UInt8.self).baseAddress
            return pinContractBounds(pk.contractBounds) { kind, idPtr, docTypePtr in
                rows.append(
                    IdentityPubkeyFFI(
                        key_id: pk.keyId,
                        key_type: pk.keyType.ffiValue,
                        purpose: pk.purpose.ffiValue,
                        security_level: pk.securityLevel.ffiValue,
                        pubkey_bytes: basePtr,
                        pubkey_len: UInt(raw.count),
                        read_only: pk.readOnly,
                        contract_bounds_kind: kind,
                        contract_bounds_id: idPtr,
                        contract_bounds_document_type: docTypePtr
                    )
                )
                return pinNext(index + 1, &rows, pubkeys, buffers, body)
            }
        }
    }

    /// Pin the contract-bounds id buffer + optional document-type
    /// CString, hand the resulting `(kind, idPtr, docTypePtr)` to
    /// `body`. Mirrors the recursive style the rest of the
    /// pubkey-array marshalling uses so the lifetimes nest cleanly
    /// inside `pinNext`.
    private static func pinContractBounds<R>(
        _ bounds: ContractBounds?,
        _ body: (UInt8, UnsafePointer<UInt8>?, UnsafePointer<CChar>?) -> R
    ) -> R {
        switch bounds {
        case .none:
            return body(0, nil, nil)
        case .singleContract(let id):
            // The Rust side reads exactly 32 bytes off
            // `contract_bounds_id`. A short or empty `Data` would
            // either dangle the base pointer (empty) or read past
            // the buffer end (short). Caller-side guard so the
            // failure surfaces as a clean precondition rather than
            // an FFI-side OOB read.
            precondition(
                id.count == 32,
                "ContractBounds.singleContract id must be exactly 32 bytes (got \(id.count))"
            )
            return id.withUnsafeBytes { raw -> R in
                let idPtr = raw.bindMemory(to: UInt8.self).baseAddress
                return body(1, idPtr, nil)
            }
        case .singleContractDocumentType(let id, let documentTypeName):
            precondition(
                id.count == 32,
                "ContractBounds.singleContractDocumentType id must be exactly 32 bytes (got \(id.count))"
            )
            return id.withUnsafeBytes { raw -> R in
                let idPtr = raw.bindMemory(to: UInt8.self).baseAddress
                return documentTypeName.withCString { docTypePtr in
                    body(2, idPtr, docTypePtr)
                }
            }
        }
    }
}

/// All-zero 20-byte tuple — used as the `hash` field default when
/// building `IdentityFundingInputFFI` / `IdentityFundingOutputFFI`.
private func hashTupleInit() -> (
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
) {
    (
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    )
}

// MARK: - Identity discovery (HD gap-limit scan)

extension ManagedPlatformWallet {
    /// One MASTER identity-authentication keypair preview, derived
    /// at the DIP-9 identity-authentication path for a given
    /// identity index. Returned by
    /// `previewIdentityRegistrationKeys()` so the UI can surface
    /// which (public, private) keys the discovery scan is walking
    /// when it finds nothing — useful when debugging "why can't my
    /// wallet see this identity?" cases.
    ///
    /// Matches the key material `discoverIdentities` actually
    /// probes: ECDSA key at
    /// `m/9'/coin'/5'/0'/ECDSA'/identity_index'/0'`. Derivation is
    /// performed entirely on the Rust side against the already-
    /// loaded wallet seed; Swift only marshals the result.
    public struct IdentityRegistrationKeyPreview: Sendable, Identifiable {
        /// Identity index (BIP-9 position under the identity branch).
        public let identityIndex: UInt32
        /// Full derivation path string, e.g.
        /// `m/9'/1'/5'/0'/0'/0'/0'`.
        public let derivationPath: String
        /// Compressed public key bytes (raw — 33 bytes for ECDSA
        /// secp256k1). Use `publicKeyHex` for display, `publicKeyData`
        /// for keychain metadata / SwiftData rows.
        public let publicKeyData: Data
        /// Compressed public key bytes as lowercase hex (33 bytes →
        /// 66 hex chars). Convenience over `publicKeyData`.
        public let publicKeyHex: String
        /// Private key in WIF (Wallet Import Format) — network-aware,
        /// compressed. Matches how other views in the example app
        /// accept / display private keys.
        public let privateKeyWIF: String
        /// Raw 32-byte ECDSA private-key scalar. Sensitive material —
        /// the intended use is to immediately persist into the iOS
        /// Keychain via
        /// `KeychainManager.storeIdentityPrivateKey(_:derivationPath:metadata:)`
        /// and drop the local reference. The pre-registration helper
        /// `prePersistIdentityKeysForRegistration` does this for the
        /// caller; surface it here too so other diagnostic views can
        /// inspect / re-stash a key without a second derivation pass.
        public let privateKeyData: Data

        public var id: UInt32 { identityIndex }
    }

    /// Derive the first `count` MASTER identity-authentication
    /// keypairs this wallet would probe during a discovery scan,
    /// starting at `startIndex`.
    ///
    /// This is a read-only preview — nothing is persisted, nothing
    /// is registered on Platform, the wallet's cached
    /// `last_scanned_index` is untouched. The keys are exactly what
    /// `discoverIdentities` internally derives and hashes to query
    /// Platform's unique-pubkey-hash index, so the UI can show the
    /// user "here are the keys we scanned for" when a scan comes
    /// back empty.
    ///
    /// All derivation policy (ECDSA key type, DIP-9 path shape,
    /// MASTER-slot key index, network-aware WIF version byte) lives
    /// on the Rust side; Swift is a thin marshaller. Nothing on the
    /// caller needs to know the mnemonic — the wallet's loaded seed
    /// is reused.
    ///
    /// - Parameters:
    ///   - startIndex: First identity index to derive. Default `0`.
    ///   - count: How many consecutive identity indices to derive.
    ///     Pass `nil` to defer to the Rust default
    ///     (`IDENTITY_GAP_LIMIT`, currently 5) so the preview
    ///     aligns with the scan window `discoverIdentities` walks.
    ///   - storage: `WalletStorage` instance used by the resolver
    ///     callback to read the BIP-39 mnemonic from iOS Keychain.
    ///     Defaults to a fresh `WalletStorage()` — overridable for
    ///     tests.
    ///
    /// - Throws: `PlatformWalletError` if the wallet handle is
    ///   invalid or Rust-side derivation fails.
    ///
    /// # Key source: chosen by wallet capability (Rust-side)
    ///
    /// A [`MnemonicResolver`] is always passed, but Rust decides whether
    /// to use it based on the in-process wallet's shape — it's a
    /// *capability*, not a command. For wallets that hold resident
    /// private keys (e.g. created from a raw seed via
    /// `createWallet(seed:)`, or whose mnemonic was never persisted to
    /// `WalletStorage`), Rust derives the preview rows from the
    /// in-process wallet and never consults the resolver. The resolver
    /// is consulted only when the in-process wallet lacks resident keys
    /// (the iOS Keychain-backed `ExternalSignable` shape whose seed
    /// lives in Keychain, not in the `WalletManager`): Rust resolves the
    /// mnemonic on demand (keyed by this wallet's own `walletId`) and
    /// derives the rows from it — the same mechanism the scan and
    /// registration use. The local `resolver` is pinned across the
    /// synchronous FFI call with `withExtendedLifetime`.
    public func previewIdentityRegistrationKeys(
        startIndex: UInt32 = 0,
        count: UInt32? = nil,
        storage: WalletStorage = WalletStorage()
    ) throws -> [IdentityRegistrationKeyPreview] {
        // `-1` tells Rust to pick the crate-level IDENTITY_GAP_LIMIT
        // default. Any supplied value passes through as-is, clamped
        // to `Int32.max` defensively — the preview scan caps well
        // below that in practice.
        let countOrNeg1: Int32
        if let count {
            countOrNeg1 = count > UInt32(Int32.max) ? Int32.max : Int32(count)
        } else {
            countOrNeg1 = -1
        }

        // The resolver reads the mnemonic from iOS Keychain on demand,
        // pinned by Rust to this wallet handle's own `walletId`. Its FFI
        // ctx is a `passUnretained` pointer (see the type's "Lifetime
        // contract"), and Swift object lifetimes end at last use — not
        // at scope end — so the last use of `resolver` (evaluating
        // `resolver.handle` as an argument) would otherwise let ARC
        // deallocate it while Rust is still mid-call, dangling the ctx.
        // `withExtendedLifetime` pins it for the whole synchronous FFI
        // call + result marshalling. Same shape as `discoverIdentities`.
        let resolver = MnemonicResolver(storage: storage)

        return try withExtendedLifetime(resolver) {
            var out = IdentityKeyPreviewsFFI()
            let result = platform_wallet_preview_identity_registration_keys(
                handle,
                resolver.handle,
                startIndex,
                countOrNeg1,
                &out
            )
            // Free the Rust-owned array whether we succeeded or bailed
            // out — the free function is a no-op on the zero struct.
            defer { platform_wallet_preview_identity_registration_keys_free(&out) }

            try result.check()

            guard let base = out.items, out.count > 0 else {
                return []
            }

            var previews: [IdentityRegistrationKeyPreview] = []
            previews.reserveCapacity(Int(out.count))
            for i in 0..<Int(out.count) {
                let row = base[i]

                let path: String = row.derivation_path.map { String(cString: $0) } ?? ""
                let wif: String = row.private_key_wif.map { String(cString: $0) } ?? ""

                let pubData: Data
                let pubHex: String
                if let pubPtr = row.public_key, row.public_key_len > 0 {
                    pubData = Data(bytes: pubPtr, count: Int(row.public_key_len))
                    pubHex = pubData.map { String(format: "%02x", $0) }.joined()
                } else {
                    pubData = Data()
                    pubHex = ""
                }

                // Inline 32-byte tuple → owned `Data`. We copy because
                // the underlying tuple is freed when the FFI struct is
                // released by the deferred free call.
                var pkTuple = row.private_key_bytes
                let pkData = withUnsafeBytes(of: &pkTuple) { Data($0) }

                previews.append(
                    IdentityRegistrationKeyPreview(
                        identityIndex: row.identity_index,
                        derivationPath: path,
                        publicKeyData: pubData,
                        publicKeyHex: pubHex,
                        privateKeyWIF: wif,
                        privateKeyData: pkData
                    )
                )
            }
            return previews
        }
    }

    /// The private key for one of this wallet's core (Layer-1)
    /// addresses, in the two forms the developer UI renders it.
    public struct CoreAddressPrivateKey: Sendable {
        /// Lowercase hex of the raw 32-byte secp256k1 scalar (64 chars).
        public let hex: String
        /// Private key in WIF (Wallet Import Format) — network-aware,
        /// compressed. Matches how other views in the example app
        /// accept / display private keys.
        public let wif: String
    }

    /// Reveal the private key for one of this wallet's tracked core
    /// addresses, returned as hex + WIF.
    ///
    /// Routes through the resolver-based FFI
    /// `platform_wallet_address_private_key`. All of the
    /// address-lookup + derivation-path work happens on the Rust side;
    /// Swift only supplies the `MnemonicResolver` (so Rust can pull the
    /// BIP-39 mnemonic on demand for the app's external-signable
    /// wallets — the seed never round-trips into a Swift `String`) and
    /// marshals the resulting strings back out. The same
    /// capability-selected key-source contract as
    /// `previewIdentityRegistrationKeys` applies: the resolver is
    /// consulted only when the in-process wallet lacks resident keys.
    ///
    /// - Parameters:
    ///   - address: the core address string to reveal the key for. Must
    ///     be one of this wallet's tracked addresses.
    ///   - storage: defaults to a fresh `WalletStorage()` — overridable
    ///     for tests. Used by the resolver vtable to read the mnemonic.
    /// - Throws: `PlatformWalletError` if the address is not tracked by
    ///   this wallet, the handle is invalid, or derivation fails.
    public func coreAddressPrivateKey(
        address: String,
        storage: WalletStorage = WalletStorage()
    ) throws -> CoreAddressPrivateKey {
        // Same resolver lifetime rationale as
        // `previewIdentityRegistrationKeys`: `withExtendedLifetime`
        // pins the resolver across the whole synchronous FFI call so
        // ARC can't deallocate its `passUnretained` ctx mid-call.
        let resolver = MnemonicResolver(storage: storage)

        return try withExtendedLifetime(resolver) {
            var out = AddressPrivateKeyFFI()
            let result = address.withCString { addressPtr in
                platform_wallet_address_private_key(
                    handle,
                    resolver.handle,
                    addressPtr,
                    &out
                )
            }
            // Free the Rust-owned (zeroizing) strings whether we
            // succeeded or bailed — the free function no-ops on the
            // zero struct.
            defer { platform_wallet_address_private_key_free(&out) }

            try result.check()

            let hex = out.private_key_hex.map { String(cString: $0) } ?? ""
            let wif = out.private_key_wif.map { String(cString: $0) } ?? ""
            return CoreAddressPrivateKey(hex: hex, wif: wif)
        }
    }

    /// Which provider key-material account to derive from. Raw values
    /// match the account `type_tag`s the host already renders with
    /// (`PersistentAccount.accountType` 10 = operator, 11 = platform
    /// node), so callers pass the same discriminator they display.
    public enum ProviderKeyKind: UInt8, Sendable {
        /// BLS masternode operator keys (`ProviderOperatorKeys`, tag 10).
        case operatorBLS = 10
        /// Ed25519 platform-node keys (`ProviderPlatformKeys`, tag 11).
        case platformNodeEdDSA = 11
    }

    /// One provider key derived at a single index, in the hex forms the
    /// developer UI renders.
    public struct ProviderDerivedKey: Sendable {
        /// The key index that was derived (`#0..`).
        public let index: UInt32
        /// Lowercase hex of the raw curve public key in MODERN (IETF)
        /// serialization — 96 chars for a BLS-48 operator key (the bytes a
        /// ProRegTx operator field carries), 64 for an Ed25519-32
        /// platform-node key.
        public let publicKeyHex: String
        /// Lowercase hex of the SAME BLS G1 point in the Dash LEGACY
        /// serialization (96 chars). `nil` for Ed25519 platform-node keys
        /// (no legacy variant). Serialized on the Rust side — never
        /// transformed in Swift.
        public let legacyPublicKeyHex: String?
        /// Lowercase hex of the 20-byte platform node id (`hash160` of
        /// the Ed25519 public key, the ProRegTx `platform_node_id`).
        /// `nil` for operator keys, which have no node id.
        public let nodeIdHex: String?
        /// Lowercase hex of the raw 32-byte private scalar, present only
        /// when the reveal requested it. BLS / Ed25519 keys have no WIF,
        /// so this is the only private form.
        public let privateKeyHex: String?

        /// Public memberwise init so hosts can build display rows from the
        /// persisted platform-node core-address rows (typed
        /// `PersistentCoreAddress` entries with `keyType == 2`) without a
        /// fresh FFI derivation — the synthesized memberwise init is
        /// internal and unreachable from the app module.
        public init(
            index: UInt32,
            publicKeyHex: String,
            legacyPublicKeyHex: String?,
            nodeIdHex: String?,
            privateKeyHex: String?
        ) {
            self.index = index
            self.publicKeyHex = publicKeyHex
            self.legacyPublicKeyHex = legacyPublicKeyHex
            self.nodeIdHex = nodeIdHex
            self.privateKeyHex = privateKeyHex
        }
    }

    /// Derive this wallet's provider key of `kind` at `index`, returned
    /// as hex (public key, optional node id, optional private key).
    ///
    /// Routes through the resolver-based FFI
    /// `platform_wallet_provider_key_at_index`. All of the derivation
    /// (which curve, hardened vs non-hardened, whether a seed is even
    /// needed) happens on the Rust side; Swift only supplies the
    /// `MnemonicResolver` and marshals the resulting strings back out.
    ///
    /// The resolver is only *consulted* when Rust actually needs a seed:
    /// an operator (BLS) public listing derives straight from the
    /// account xpub and never fires the keychain read, whereas a
    /// platform-node (Ed25519, SLIP-10 hardened-only) key needs the seed
    /// even for its public key. Passing the resolver here is therefore
    /// always safe — it stays dormant unless Rust calls it.
    ///
    /// - Parameters:
    ///   - kind: operator (BLS) or platform-node (Ed25519) keys.
    ///   - index: the key index to derive (`#0..`).
    ///   - includePrivate: also return the raw private scalar.
    ///   - storage: defaults to a fresh `WalletStorage()` — overridable
    ///     for tests. Used by the resolver vtable to read the mnemonic.
    /// - Throws: `PlatformWalletError` if the wallet has no account of
    ///   that kind, the handle is invalid, or derivation fails.
    public func providerKeyAtIndex(
        kind: ProviderKeyKind,
        index: UInt32,
        includePrivate: Bool,
        storage: WalletStorage = WalletStorage()
    ) throws -> ProviderDerivedKey {
        // Same resolver lifetime rationale as `coreAddressPrivateKey`:
        // `withExtendedLifetime` pins the resolver across the whole
        // synchronous FFI call so ARC can't deallocate its
        // `passUnretained` ctx mid-call.
        let resolver = MnemonicResolver(storage: storage)

        return try withExtendedLifetime(resolver) {
            var out = ProviderKeyAtIndexFFI()
            let result = platform_wallet_provider_key_at_index(
                handle,
                resolver.handle,
                kind.rawValue,
                index,
                includePrivate,
                &out
            )
            // Free the Rust-owned strings (the private-key hex is
            // zeroized inside) whether we succeeded or bailed — the free
            // function no-ops on the zero struct.
            defer { platform_wallet_provider_key_at_index_free(&out) }

            try result.check()

            let publicKeyHex = out.public_key_hex.map { String(cString: $0) } ?? ""
            let legacyPublicKeyHex = out.legacy_public_key_hex.map { String(cString: $0) }
            let nodeIdHex = out.node_id_hex.map { String(cString: $0) }
            let privateKeyHex = out.private_key_hex.map { String(cString: $0) }
            return ProviderDerivedKey(
                index: out.index,
                publicKeyHex: publicKeyHex,
                legacyPublicKeyHex: legacyPublicKeyHex,
                nodeIdHex: nodeIdHex,
                privateKeyHex: privateKeyHex
            )
        }
    }

    /// Compute the 20-byte Tenderdash platform node id
    /// (`SHA256(ed25519 pubkey)[..20]`, rust-dashcore #884) for a raw
    /// 32-byte Ed25519 public key, via the pure Rust helper
    /// `platform_wallet_platform_node_id_from_ed25519_pubkey`.
    ///
    /// The node id is exactly what a ProRegTx `platform_node_id` field
    /// carries; hosts use this to render the node id of a persisted
    /// platform-node public key (which stores only the pubkey) without
    /// re-implementing the SHA-256 digest. Pure bridge — no wallet handle,
    /// no key material beyond the public key.
    ///
    /// - Returns: the 20-byte node id, or `nil` when `publicKey` is not
    ///   exactly 32 bytes or the FFI rejects it.
    public static func platformNodeId(fromEd25519PublicKey publicKey: Data) -> Data? {
        guard publicKey.count == 32 else { return nil }
        var out = Data(count: 20)
        let ok = out.withUnsafeMutableBytes { outRaw -> Bool in
            publicKey.withUnsafeBytes { pkRaw -> Bool in
                platform_wallet_platform_node_id_from_ed25519_pubkey(
                    pkRaw.bindMemory(to: UInt8.self).baseAddress,
                    UInt(pkRaw.count),
                    outRaw.bindMemory(to: UInt8.self).baseAddress
                )
            }
        }
        return ok ? out : nil
    }

    /// Derive a single ECDSA identity-authentication keypair at an
    /// arbitrary `(identityIndex, keyId)` slot — the building block
    /// the "add key to existing identity" flow runs on.
    ///
    /// Routes through the resolver-based FFI
    /// `dash_sdk_derive_identity_key_at_slot_with_resolver`. Rust
    /// pulls the BIP-39 mnemonic across the FFI on demand via the
    /// `MnemonicResolver` callback (whose `resolve` function reads
    /// from iOS Keychain through `WalletStorage`); the seed never
    /// lives in a Swift `String` outside the resolver trampoline's
    /// stack frame, satisfying the `swift-sdk/CLAUDE.md` "no
    /// mnemonic round-tripping" rule. Earlier revisions pulled the
    /// mnemonic into Swift, handed it back to a stateless FFI as a
    /// `withCString` scope, and looped over the result — that
    /// shape is the canonical anti-precedent the rule cites.
    ///
    /// The returned `IdentityRegistrationKeyPreview` carries the
    /// derived public key bytes + the 32-byte private scalar.
    /// Callers building the `addPublicKeys` payload for
    /// `updateIdentity(...)` should:
    ///   1. Call `KeychainManager.storeIdentityPrivateKey(_:
    ///      derivationPath:metadata:)` with the private bytes so
    ///      the `KeychainSigner` trampoline can sign the resulting
    ///      transition.
    ///   2. Build an `IdentityPubkey` from the public key bytes +
    ///      the chosen `purpose / securityLevel / keyType` and
    ///      hand it to `wallet.updateIdentity(addPublicKeys:...,
    ///      signer:)`.
    ///
    /// - Parameters:
    ///   - identityIndex: BIP-9 identity index in the HD tree.
    ///   - keyId: hardened key index. Caller picks
    ///     `max(existingKeyIds) + 1` to extend an existing identity.
    ///   - network: the wallet's network — selects the DIP-9
    ///     coin-type slot in the derivation path AND the WIF version.
    ///   - storage: defaults to a fresh `WalletStorage()` —
    ///     overridable for tests. Used by the resolver vtable.
    ///
    /// - Throws: `PlatformWalletError` from the FFI on any failure
    ///   (mnemonic missing, bad slot, derivation failure).
    @MainActor
    public func deriveIdentityAuthKeyAtSlot(
        identityIndex: UInt32,
        keyId: UInt32,
        network: Network,
        storage: WalletStorage = WalletStorage()
    ) throws -> IdentityRegistrationKeyPreview {
        // The FFI takes a 32-byte wallet id which the resolver uses
        // to look up the mnemonic in iOS Keychain. Pin to
        // `self.walletId` here — the method is already scoped to
        // this `ManagedPlatformWallet` instance, so accepting a
        // separate `walletId` parameter would let a caller derive
        // a key from one wallet's mnemonic while attributing it
        // to a different wallet's `ManagedPlatformWallet`. Defensive
        // length guard for the (unexpected but possible) case where
        // the instance was constructed with a malformed wallet id.
        guard self.walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be 32 bytes, got \(self.walletId.count)"
            )
        }
        let resolver = MnemonicResolver(storage: storage)

        var row = IdentityKeyPreviewFFI()

        let result = self.walletId.withUnsafeBytes { walletBytes -> PlatformWalletFFIResult in
            let walletPtr = walletBytes.bindMemory(to: UInt8.self).baseAddress!
            return dash_sdk_derive_identity_key_at_slot_with_resolver(
                network.ffiValue,
                walletPtr,
                resolver.handle,
                identityIndex,
                keyId,
                &row
            )
        }
        defer { dash_sdk_derive_identity_key_at_slot_free(&row) }

        try result.check()

        let path: String = row.derivation_path.map { String(cString: $0) } ?? ""
        let wif: String = row.private_key_wif.map { String(cString: $0) } ?? ""

        let pubData: Data
        let pubHex: String
        if let pubPtr = row.public_key, row.public_key_len > 0 {
            pubData = Data(bytes: pubPtr, count: Int(row.public_key_len))
            pubHex = pubData.map { String(format: "%02x", $0) }.joined()
        } else {
            pubData = Data()
            pubHex = ""
        }

        // Inline tuple → owned `Data`. Copy because the underlying
        // tuple is zeroed by the deferred free call on scope exit.
        var pkTuple = row.private_key_bytes
        let pkData = withUnsafeBytes(of: &pkTuple) { Data($0) }

        return IdentityRegistrationKeyPreview(
            identityIndex: row.identity_index,
            derivationPath: path,
            publicKeyData: pubData,
            publicKeyHex: pubHex,
            privateKeyWIF: wif,
            privateKeyData: pkData
        )
    }

    /// Pre-derive + pre-persist the authentication keys an upcoming
    /// `registerIdentityFromAddresses(...signer:)` call will use.
    ///
    /// Required because the FFI `KeychainSigner` looks up private
    /// keys by public-key bytes, but the `PersistentPublicKey`
    /// SwiftData rows + their keychain entries are normally only
    /// inserted by the Rust persister callback **after** registration
    /// completes. Calling this method before registration writes the
    /// keychain entries up front so the signer trampoline can find
    /// them mid-registration via the
    /// `KeychainManager.retrieveIdentityPrivateKey(publicKeyHex:)`
    /// fallback path. The matching SwiftData rows are written by
    /// the persister callback once registration succeeds.
    ///
    /// # Architecture: derivation loop lives in Rust
    ///
    /// All derivation, the per-key MASTER-vs-HIGH policy, and the
    /// DPP discriminant bytes (`KeyType`, `Purpose`,
    /// `SecurityLevel`) live on the Rust side. Swift hands the
    /// derivation FFI two callback handles — a
    /// [`MnemonicResolver`] (Rust → Swift "fetch the BIP-39
    /// mnemonic for `walletId`") and an
    /// [`IdentityKeyPersister`] (Rust → Swift "save this derived
    /// key, here's its metadata") — and Rust runs the loop
    /// without Swift orchestrating it. Closes the swift-sdk/CLAUDE.md
    /// "no mnemonic round-tripping" rule that the prior shape
    /// (Swift pulls mnemonic, calls a stateless derivation FFI,
    /// loops over the rows writing each to Keychain) violated.
    ///
    /// # Why this works for watch-only wallets
    ///
    /// The previous implementation that routed through the
    /// wallet-handle variant
    /// (`platform_wallet_derive_identity_keys_for_index`) failed
    /// for restored watch-only wallets because Rust had no
    /// in-process xpriv loaded for them. Routing through the
    /// resolver callback works regardless of wallet shape — the
    /// resolver pulls the mnemonic from iOS Keychain on demand.
    ///
    /// - Parameters:
    ///   - identityIndex: The identity index that will be passed to
    ///     `registerIdentityFromAddresses`.
    ///   - keyCount: The number of authentication keys to pre-derive.
    ///     Must match the `keyCount` argument used at registration
    ///     time so the `(identityIndex, keyId)` slots line up with
    ///     the eventual persister-callback rows.
    ///   - network: The network the upcoming registration will run
    ///     on. Selects the DIP-9 coin-type slot in the derivation
    ///     paths AND the WIF version byte. Must match the network of
    ///     the SDK / `KeychainSigner` used at registration time.
    ///   - keychain: Defaults to `KeychainManager.shared`.
    ///   - storage: `WalletStorage` instance used to read the BIP-39
    ///     mnemonic from Keychain. Defaults to a fresh
    ///     `WalletStorage()` — overridable for tests.
    /// - Returns: One [`IdentityPubkey`] per derived/persisted key,
    ///   ready to thread directly into
    ///   `registerIdentityFromAddresses(...identityPubkeys:...)`.
    ///   The `(keyType, purpose, securityLevel)` triple on each row
    ///   is whatever Rust decided — the caller does NOT recreate
    ///   the MASTER-vs-HIGH policy.
    @discardableResult
    public func prePersistIdentityKeysForRegistration(
        identityIndex: UInt32,
        keyCount: UInt32,
        network: Network,
        keychain: KeychainManager = .shared,
        storage: WalletStorage = WalletStorage()
    ) throws -> [IdentityPubkey] {
        guard keyCount > 0 else { return [] }

        // Single-FFI derivation + persist. The mnemonic is pulled
        // from Keychain by the resolver callback (Rust → Swift),
        // each derived key is written by the persister callback
        // (Rust → Swift), and only pubkeys + paths flow back as
        // the function's return value. Per swift-sdk/CLAUDE.md no
        // pipeline orchestration crosses the FFI boundary; the
        // mnemonic never lives in a Swift `String` outside the
        // resolver trampoline's stack frame, and the 32-byte
        // private scalars never leave the Rust loop.
        let resolver = MnemonicResolver(storage: storage)
        let persister = IdentityKeyPersister(keychain: keychain)

        var out = IdentityRegistrationKeyDerivationsFFI()

        // `walletId` is `Data`; bind into a 32-byte UInt8 pointer
        // so Rust receives a stable address for the duration of
        // the FFI call.
        let result = self.walletId.withUnsafeBytes { walletBytes -> PlatformWalletFFIResult in
            let walletPtr = walletBytes.bindMemory(to: UInt8.self).baseAddress
            return dash_sdk_derive_and_persist_identity_keys(
                network.ffiValue,
                walletPtr,
                identityIndex,
                keyCount,
                resolver.handle,
                persister.handle,
                &out
            )
        }
        defer { dash_sdk_derive_identity_keys_from_mnemonic_free(&out) }

        try result.check()

        guard let base = out.items, out.count > 0 else { return [] }

        // Build `IdentityPubkey` rows from (a) the FFI's pubkey
        // bytes and (b) the persister's captured per-key metadata
        // — both indexed in derivation order. Rust decided
        // (keyType, purpose, securityLevel); Swift just maps each
        // discriminant byte back into its strongly-typed enum.
        //
        // Both an FFI/Swift count drift AND any unknown enum
        // discriminant byte are surfaced as a recoverable
        // `PlatformWalletError.walletOperation` rather than a
        // process abort or a silently-defaulted enum value.
        // Defaulting `securityLevel` etc. to a known constant
        // (the prior shape) would silently change key semantics
        // if Rust's enum discriminants ever drifted; better to
        // fail loudly so the caller learns about the ABI break
        // immediately.
        let captured = persister.persistedKeys
        guard captured.count == Int(out.count), Int(out.count) == Int(keyCount) else {
            throw PlatformWalletError.walletOperation(
                "derive_and_persist_identity_keys returned \(out.count) pubkeys for "
                + "\(keyCount) requested keys; persister captured \(captured.count)"
            )
        }

        var pubkeys: [IdentityPubkey] = []
        pubkeys.reserveCapacity(Int(out.count))
        for i in 0..<Int(out.count) {
            let row = base[i]
            // Empty pubkey rows are an FFI contract violation —
            // the persist callback already accepted a real pubkey
            // for this row, so the parallel out_pubkeys array
            // having `nil` / zero-length here means something
            // dropped between the persister fire and the row
            // serialization. Fail fast with the offending row
            // index rather than silently constructing an
            // unusable `IdentityPubkey` with empty bytes.
            guard let pubPtr = row.public_key, row.public_key_len > 0 else {
                throw PlatformWalletError.walletOperation(
                    "derive_and_persist_identity_keys returned an empty public key for row \(i)"
                )
            }
            let pubData = Data(bytes: pubPtr, count: Int(row.public_key_len))
            let meta = captured[i]
            guard
                let keyType = KeyType(rawValue: meta.keyType),
                let purpose = KeyPurpose(rawValue: meta.purpose),
                let securityLevel = SecurityLevel(rawValue: meta.securityLevel)
            else {
                throw PlatformWalletError.walletOperation(
                    "derive_and_persist_identity_keys returned unknown enum bytes "
                    + "(keyType=\(meta.keyType), purpose=\(meta.purpose), "
                    + "securityLevel=\(meta.securityLevel)) for keyId=\(meta.keyId)"
                )
            }
            pubkeys.append(
                IdentityPubkey(
                    keyId: meta.keyId,
                    keyType: keyType,
                    purpose: purpose,
                    securityLevel: securityLevel,
                    pubkeyBytes: pubData,
                    readOnly: false
                )
            )
        }
        return pubkeys
    }

    // ----------------------------------------------------------------
    // prePersistPlatformAddressPrivateKeysForRegistration — REMOVED
    // ----------------------------------------------------------------
    //
    // Platform-address private keys are NEVER persisted. The FFI
    // signer trampoline derives them on demand per signing call from
    // `(mnemonic-in-Keychain, derivation-path-in-SwiftData)` via
    // `dash_sdk_sign_with_mnemonic_and_path` and zeroes the buffers
    // immediately. See `KeychainSigner.swift`'s `key_type == 0xFF`
    // branch.
    //
    // Identity keys still go through `prePersistIdentityKeysForRegistration`
    // above — those ARE intended to live in Keychain as primary
    // storage.

    /// Scan the wallet's DIP-9 identity authentication derivation
    /// tree for registered identities and fold any matches into the
    /// local identity manager.
    ///
    /// For each identity index in the scan range, derives the MASTER
    /// authentication public key at key index 0 and asks Platform
    /// "is there an identity registered with this pubkey hash?"
    /// (unique-hash lookup). Stops after `gapLimit` consecutive
    /// misses. Newly-discovered identities are persisted via the
    /// existing identity persister callback, so SwiftData `@Query`
    /// views refresh automatically once this call returns.
    ///
    /// - Parameters:
    ///   - startIndex: Pass `nil` (the default) to resume from the
    ///     wallet's cached last-scanned index. Pass `0` (or any
    ///     explicit `UInt32`) to start a full rescan from that
    ///     index. The cached index is never rewound — an explicit
    ///     `startIndex` below the cache just re-walks that range.
    ///   - gapLimit: Maximum consecutive empty identity indices to
    ///     tolerate before stopping. Defaults to the Rust default
    ///     (`IDENTITY_GAP_LIMIT`, currently 5) when omitted.
    ///   - storage: `WalletStorage` instance used by the resolver
    ///     callback to read the BIP-39 mnemonic from iOS Keychain.
    ///     Defaults to a fresh `WalletStorage()` — overridable for
    ///     tests.
    /// - Returns: The identifiers of any identities the scan
    ///   discovered that weren't already in the local manager.
    ///   Identities already tracked are not re-reported.
    ///
    /// # Key source: chosen by wallet capability (Rust-side)
    ///
    /// A [`MnemonicResolver`] is always passed to the FFI, but Rust
    /// decides whether to use it based on the in-process wallet's shape
    /// — it's a *capability*, not a command. iOS Keychain-backed
    /// `ExternalSignable` wallets keep their seed in Keychain, not in
    /// the `WalletManager`, so the resident derive would fail with
    /// `External signable wallet has no private key`; for those, Rust
    /// resolves the mnemonic on demand (keyed by this wallet's own
    /// `walletId`) and derives the scan keys from it — the same
    /// mechanism identity registration uses. The resolver is consulted
    /// only when the in-process wallet lacks resident keys: wallets that
    /// hold resident private keys (e.g. created from a raw seed via
    /// `createWallet(seed:)`, or whose mnemonic was never persisted to
    /// `WalletStorage`) keep scanning via the in-process derive and
    /// never touch the resolver. No mnemonic / derivation pipeline runs
    /// in Swift; this stays a thin bridge per `swift-sdk/CLAUDE.md`.
    public func discoverIdentities(
        startIndex: UInt32? = nil,
        gapLimit: UInt32? = nil,
        storage: WalletStorage = WalletStorage()
    ) async throws -> [Identifier] {
        let handle = self.handle
        let startArg: Int64 = startIndex.map(Int64.init) ?? -1
        let gapArg: UInt32 = gapLimit ?? 0
        // The resolver reads the mnemonic from iOS Keychain on demand;
        // Rust pins it to this wallet handle's own `walletId`, so no
        // wallet-id argument is passed. `MnemonicResolver` is
        // `@unchecked Sendable`; capture it in the detached closure and
        // wrap the FFI call in `withExtendedLifetime` so ARC keeps it
        // alive for the synchronous call's duration (its FFI ctx is a
        // `passUnretained` pointer — see the type's "Lifetime
        // contract").
        let resolver = MnemonicResolver(storage: storage)
        return try await Task.detached(priority: .userInitiated) {
            () -> [Identifier] in
            try withExtendedLifetime(resolver) {
                var found = DiscoveredIdentityIdsFFI()
                let result = platform_wallet_discover_identities(
                    handle,
                    resolver.handle,
                    startArg,
                    gapArg,
                    &found
                )
                defer { platform_wallet_discover_identities_free(&found) }
                try result.check()
                guard let base = found.ids, found.count > 0 else {
                    return []
                }
                var ids: [Identifier] = []
                ids.reserveCapacity(Int(found.count))
                for i in 0..<Int(found.count) {
                    var tuple = base[i]
                    let data = Swift.withUnsafeBytes(of: &tuple) { Data($0) }
                    ids.append(data)
                }
                return ids
            }
        }.value
    }

    /// Load the identity registered for this wallet at a single,
    /// known BIP-9 identity index and fold it into the local identity
    /// manager.
    ///
    /// Derives the MASTER authentication public key at key index 0 for
    /// the given `identityIndex` and asks Platform "is there an identity
    /// registered with this pubkey hash?" (unique-hash lookup). Unlike
    /// `discoverIdentities`, this probes exactly ONE index rather than
    /// gap-limit scanning a range — the two share the same DIP-9 MASTER
    /// slot, so they resolve the same identity at the same index. If an
    /// identity is found it is persisted via the existing identity
    /// persister callback, so SwiftData `@Query` views refresh
    /// automatically once this call returns.
    ///
    /// - Parameters:
    ///   - identityIndex: The BIP-9 identity index to probe.
    ///   - storage: `WalletStorage` instance used by the resolver
    ///     callback to read the BIP-39 mnemonic from iOS Keychain.
    ///     Defaults to a fresh `WalletStorage()` — overridable for
    ///     tests.
    /// - Returns: The identifier of the identity registered at
    ///   `identityIndex`, or `nil` if none is registered there.
    ///
    /// # Key source: chosen by wallet capability (Rust-side)
    ///
    /// A [`MnemonicResolver`] is always passed to the FFI, but Rust
    /// decides whether to use it based on the in-process wallet's shape
    /// — it's a *capability*, not a command. iOS Keychain-backed
    /// `ExternalSignable` wallets keep their seed in Keychain, not in
    /// the `WalletManager`, so the resident derive would fail with
    /// `External signable wallet has no private key`; for those, Rust
    /// resolves the mnemonic on demand (keyed by this wallet's own
    /// `walletId`) and derives the probe key from it — the same
    /// mechanism identity discovery and registration use. Wallets that
    /// hold resident private keys keep probing via the in-process
    /// derive and never touch the resolver. No mnemonic / derivation
    /// pipeline runs in Swift; this stays a thin bridge per
    /// `swift-sdk/CLAUDE.md`.
    public func loadIdentity(
        atIndex identityIndex: UInt32,
        storage: WalletStorage = WalletStorage()
    ) async throws -> Identifier? {
        let handle = self.handle
        // The resolver reads the mnemonic from iOS Keychain on demand;
        // Rust pins it to this wallet handle's own `walletId`, so no
        // wallet-id argument is passed. `MnemonicResolver` is
        // `@unchecked Sendable`; capture it in the detached closure and
        // wrap the FFI call in `withExtendedLifetime` so ARC keeps it
        // alive for the synchronous call's duration (its FFI ctx is a
        // `passUnretained` pointer — see the type's "Lifetime
        // contract").
        let resolver = MnemonicResolver(storage: storage)
        return try await Task.detached(priority: .userInitiated) {
            () -> Identifier? in
            try withExtendedLifetime(resolver) {
                var found = false
                var idBytes = [UInt8](repeating: 0, count: 32)
                let result = idBytes.withUnsafeMutableBufferPointer {
                    idBuf -> PlatformWalletFFIResult in
                    platform_wallet_load_identity_at_index(
                        handle,
                        resolver.handle,
                        identityIndex,
                        &found,
                        idBuf.baseAddress!
                    )
                }
                try result.check()
                guard found else { return nil }
                return Data(idBytes)
            }
        }.value
    }
}

// MARK: - DPNS operations

/// Simple search-result struct surfaced by `searchDpnsNames`. Mirrors
/// the Rust `DpnsSearchResultFFI` row shape in a Sendable Swift value.
public struct DpnsSearchResult: Sendable, Equatable, Identifiable {
    public let identityId: Identifier
    public let fullName: String
    /// Unique per row: a single identity can own several names that
    /// match a prefix, and a contested name shares `fullName` across
    /// contenders — so neither field alone is unique. Combine both.
    public var id: String { "\(fullName)|\(identityId.toBase58String())" }
}

extension ManagedPlatformWallet {
    /// Register a DPNS name for `identityId` on Platform using an
    /// externally-supplied `KeychainSigner`.
    ///
    /// Architecturally aligned with
    /// `registerIdentityFromAddresses(...identitySigner:addressSigner:)`:
    /// every signature on this path crosses the FFI boundary via the
    /// Swift-side signer's vtable, so the wallet's own seed never
    /// participates. Required for watch-only wallets restored from
    /// SwiftData state (where the seed lives in iOS Keychain rather
    /// than the in-process `WalletManager`).
    ///
    /// Goes through `IdentityWallet::register_name_with_external_signer`,
    /// which on success:
    ///   1. broadcasts the DPNS preorder + domain documents (signing
    ///      via the supplied `signer.handle`),
    ///   2. appends the new `DpnsNameInfo` to
    ///      `ManagedIdentity.dpns_names`,
    ///   3. queues the updated identity in the persister so the
    ///      SwiftData `PersistentIdentity` row refreshes via the
    ///      `on_persist_identities_fn` callback.
    ///
    /// The Rust side picks the HIGH/CRITICAL authentication key the
    /// document state transition requires from the identity's own
    /// `public_keys` map; the signer's role is to sign with whatever
    /// key was picked.
    ///
    /// Returns the full domain name (e.g. `"alice.dash"`).
    @discardableResult
    public func registerDpnsName(
        identityId: Identifier,
        name: String,
        signer: KeychainSigner
    ) async throws -> String {
        let handle = self.handle
        // Take the raw signer handle outside the Task. `KeychainSigner`
        // uses `passUnretained` for its FFI ctx, so the pointer is
        // only safe while the Swift `signer` object is alive. The
        // explicit `_ = signer` inside the Task keeps it alive for
        // the duration of the detached work — see the "Lifetime
        // contract" note on `KeychainSigner`.
        let signerHandle = signer.handle
        // Capture the 32-byte payload by value into a Sendable
        // `[UInt8]` so the detached Task can hand a fresh pointer
        // back to the FFI.
        let idBytes: [UInt8] = identityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        return try await Task.detached(priority: .userInitiated) { () -> String in
            _ = signer
            var outPtr: UnsafeMutablePointer<CChar>? = nil
            let result = idBytes.withUnsafeBufferPointer { idBp in
                name.withCString { namePtr in
                    platform_wallet_register_dpns_name_with_signer(
                        handle,
                        idBp.baseAddress!,
                        namePtr,
                        signerHandle,
                        &outPtr
                    )
                }
            }
            try result.check()
            defer { if let p = outPtr { platform_wallet_string_free(p) } }
            guard let p = outPtr else {
                throw PlatformWalletError.walletOperation(
                    "register_dpns_name_with_signer returned a null full-domain-name pointer"
                )
            }
            return String(cString: p)
        }.value
    }

    /// Resolve a DPNS name (`"alice"` or `"alice.dash"`) to an
    /// identity id. Returns `nil` when the name is unregistered.
    public func resolveDpnsName(_ name: String) async throws -> Identifier? {
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) { () -> Identifier? in
            var buf = [UInt8](repeating: 0, count: 32)
            var found = false
            let result = buf.withUnsafeMutableBufferPointer { bp -> PlatformWalletFFIResult in
                name.withCString { namePtr in
                    platform_wallet_resolve_dpns_name(
                        handle,
                        namePtr,
                        bp.baseAddress!,
                        &found
                    )
                }
            }
            try result.check()
            guard found else { return nil }
            return Data(buf)
        }.value
    }

    /// Prefix-search DPNS documents on Platform.
    ///
    /// `limit == 0` defers to the SDK's default cap (currently 100).
    public func searchDpnsNames(
        prefix: String,
        limit: UInt32 = 0
    ) async throws -> [DpnsSearchResult] {
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) { () -> [DpnsSearchResult] in
            var outPtr: UnsafeMutablePointer<DpnsSearchResultFFI>? = nil
            var outCount: UInt = 0
            let result = prefix.withCString { prefixPtr in
                platform_wallet_search_dpns_names(
                    handle,
                    prefixPtr,
                    limit,
                    &outPtr,
                    &outCount
                )
            }
            try result.check()
            guard let ptr = outPtr, outCount > 0 else {
                return []
            }
            defer { dpns_search_results_free(ptr, outCount) }
            var results: [DpnsSearchResult] = []
            results.reserveCapacity(Int(outCount))
            for i in 0..<Int(outCount) {
                let entry = ptr[i]
                let label = entry.label.map { String(cString: $0) } ?? ""
                var idTuple = entry.identity_id
                let identityId = Swift.withUnsafeBytes(of: &idTuple) { Data($0) }
                results.append(.init(identityId: identityId, fullName: label))
            }
            return results
        }.value
    }
}

// MARK: - DPNS name cache sync

extension ManagedPlatformWallet {
    /// Fetch DPNS usernames owned by `identityId` from Platform and
    /// merge them into the local cache
    /// (`ManagedIdentity.dpns_names`). Returns the number of
    /// newly-added labels.
    ///
    /// Prefer this over `sdk.dpnsGetUsername(...)` — this path goes
    /// through the wallet's identity changeset so the
    /// `PersistentIdentity` row refreshes via the persister
    /// callback, and subsequent reads can come off the local cache
    /// via `ManagedIdentity.getDpnsNames()` without an RPC.
    @discardableResult
    public func syncDpnsNames(identityId: Identifier) async throws -> UInt32 {
        let handle = self.handle
        let idBytes: [UInt8] = identityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        return try await Task.detached(priority: .userInitiated) { () -> UInt32 in
            var added: UInt32 = 0
            let result = idBytes.withUnsafeBufferPointer { bp in
                platform_wallet_sync_dpns_names(handle, bp.baseAddress!, &added)
            }
            try result.check()
            return added
        }.value
    }

    /// Fetch the current vote state for a contested DPNS label
    /// `identityId` is contending for. Returns `nil` when the
    /// lookup doesn't hit (contest doesn't exist, identity isn't
    /// contending, or contest already resolved).
    ///
    /// Ephemeral — never cached. Vote tallies, winner state, and
    /// contender set change throughout the voting period, so the
    /// caller asks fresh whenever it needs the details. Safe to
    /// call on pull-to-refresh. Prefer this over
    /// `sdk.dpnsGetContestedVoteState` directly so the unified
    /// DashPay/DPNS read surface stays consistent across the UI.
    public func fetchContestVoteState(
        identityId: Identifier,
        label: String
    ) async throws -> ContestVoteState? {
        let handle = self.handle
        let idBytes: [UInt8] = identityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        return try await Task.detached(priority: .userInitiated) {
            () -> ContestVoteState? in
            var state = ContestVoteStateFFI()
            var found = false
            let result = idBytes.withUnsafeBufferPointer { idBp -> PlatformWalletFFIResult in
                label.withCString { labelPtr in
                    platform_wallet_fetch_contest_vote_state(
                        handle,
                        idBp.baseAddress!,
                        labelPtr,
                        &state,
                        &found
                    )
                }
            }
            defer { contest_vote_state_ffi_free(&state) }
            try result.check()
            guard found else { return nil }
            return ContestVoteState(ffi: state)
        }.value
    }

    /// Fetch the labels of DPNS contests `identityId` is currently
    /// contending for (voting period active, not resolved) and
    /// replace `ManagedIdentity.contested_dpns_names` wholesale.
    ///
    /// Use this as the read source for contested-name lists —
    /// resolved contests automatically drop out of the local cache
    /// because Rust writes a full snapshot, not a dedup-append.
    /// Contest metadata (contenders, vote state, end time) isn't
    /// cached; callers that need those details should still query
    /// `Sdk::get_contested_dpns_vote_state` directly since they
    /// change throughout the voting period.
    @discardableResult
    public func syncContestedDpnsNames(identityId: Identifier) async throws -> UInt32 {
        let handle = self.handle
        let idBytes: [UInt8] = identityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        return try await Task.detached(priority: .userInitiated) { () -> UInt32 in
            var count: UInt32 = 0
            let result = idBytes.withUnsafeBufferPointer { bp in
                platform_wallet_sync_contested_dpns_names(
                    handle,
                    bp.baseAddress!,
                    &count
                )
            }
            try result.check()
            return count
        }.value
    }
}

// MARK: - DashPay contact requests + payments

extension ManagedPlatformWallet {
    /// Return a Swift-owned `ManagedIdentity` handle for `identityId`
    /// by looking it up inside this wallet's `IdentityManager`.
    ///
    /// The returned handle is a snapshot clone — it doesn't track
    /// further Rust-side mutations on the live identity. Call again
    /// after each sync round to pick up fresh contact-request state
    /// etc. Throws `.identityNotFound` when the wallet doesn't know
    /// this identity.
    public func managedIdentity(identityId: Identifier) throws -> ManagedIdentity {
        var outHandle: Handle = NULL_HANDLE
        let result = identityId.withFFIBytes { idPtr in
            platform_wallet_get_managed_identity(
                handle,
                idPtr,
                &outHandle
            )
        }
        try result.check()
        return ManagedIdentity(handle: outHandle)
    }

    /// Sync received contact requests for every managed identity on
    /// this wallet from Platform. Returns wrappers for each
    /// newly-discovered request (an empty array when nothing new
    /// arrived).
    @discardableResult
    public func syncContactRequests() async throws -> [ContactRequest] {
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) { () -> [ContactRequest] in
            var array = ContactRequestHandleArray()
            let result = platform_wallet_sync_contact_requests(handle, &array)
            try result.check()
            defer { platform_wallet_contact_request_handle_array_free(&array) }
            guard let handles = array.handles, array.count > 0 else {
                return []
            }
            var requests: [ContactRequest] = []
            requests.reserveCapacity(Int(array.count))
            for i in 0..<Int(array.count) {
                requests.append(ContactRequest(handle: handles[i]))
            }
            return requests
        }.value
    }

    /// Send a contact request using an externally-supplied
    /// `KeychainSigner` for the document state-transition.
    ///
    /// Architecturally aligned with the other `_with_signer` flows
    /// (DPNS register, identity register, ...). Required for
    /// watch-only wallets and the architecturally correct path per
    /// `swift-sdk/CLAUDE.md` — no signing happens via the wallet's
    /// own seed on this path.
    ///
    /// CAVEAT — ECDH derivation: the contact-request encryption step
    /// still derives the sender's ECDH private key from the wallet
    /// seed Rust-side. Watch-only wallets will fail at that step
    /// until a follow-up FFI pushes ECDH across as well.
    public func sendContactRequest(
        senderIdentityId: Identifier,
        recipientIdentityId: Identifier,
        accountLabel: String? = nil,
        autoAcceptProof: Data? = nil,
        signer: KeychainSigner
    ) async throws -> ContactRequest {
        let handle = self.handle
        let signerHandle = signer.handle
        // Resolver-backed core signer: the contact-request crypto (friendship
        // xpub, ECDH shared secret, DIP-15 accountReference) is derived through
        // the Keychain mnemonic resolver Rust-side, so no resident seed is
        // needed and watch-only / external-signable wallets work.
        let coreSigner = MnemonicResolver()
        let senderBytes: [UInt8] = senderIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let recipientBytes: [UInt8] = recipientIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let accountLabel = accountLabel
        let autoAcceptProof = autoAcceptProof

        let requestHandle: Handle = try await Task.detached(priority: .userInitiated) {
            () -> Handle in
            var outHandle: Handle = NULL_HANDLE
            // `withExtendedLifetime` keeps both the document signer and the
            // resolver alive across the synchronous FFI call — the optimizer
            // can otherwise drop them mid-call and a vtable callback would
            // use-after-free.
            let result: PlatformWalletFFIResult = withExtendedLifetime((signer, coreSigner)) {
                senderBytes.withUnsafeBufferPointer {
                    senderBp -> PlatformWalletFFIResult in
                    recipientBytes.withUnsafeBufferPointer { recipientBp -> PlatformWalletFFIResult in
                        let callWithLabel: (UnsafePointer<CChar>?) -> PlatformWalletFFIResult = {
                            labelPtr in
                            if let autoAcceptProof, !autoAcceptProof.isEmpty {
                                return autoAcceptProof.withUnsafeBytes { rawBuf in
                                    let bytesPtr = rawBuf.baseAddress?
                                        .assumingMemoryBound(to: UInt8.self)
                                    return platform_wallet_send_contact_request_with_signer(
                                        handle,
                                        senderBp.baseAddress!,
                                        recipientBp.baseAddress!,
                                        labelPtr,
                                        bytesPtr,
                                        UInt(autoAcceptProof.count),
                                        signerHandle,
                                        coreSigner.handle,
                                        &outHandle
                                    )
                                }
                            } else {
                                return platform_wallet_send_contact_request_with_signer(
                                    handle,
                                    senderBp.baseAddress!,
                                    recipientBp.baseAddress!,
                                    labelPtr,
                                    nil,
                                    0,
                                    signerHandle,
                                    coreSigner.handle,
                                    &outHandle
                                )
                            }
                        }
                        if let accountLabel {
                            return accountLabel.withCString { callWithLabel($0) }
                        } else {
                            return callWithLabel(nil)
                        }
                    }
                }
            }
            try result.check()
            return outHandle
        }.value

        return ContactRequest(handle: requestHandle)
    }

    /// Build a DIP-15 auto-accept QR URI (`dash:?du=<username>&dapk=<key_blob>`)
    /// for `ownerIdentityId`, valid for 1 hour. The QR's `du` is the owner's DPNS
    /// name; pass the locally-cached `username` when known, or an empty string to
    /// have Rust resolve it on-chain (needed for imported/restored identities
    /// whose name isn't cached locally). The returned URI is rendered as a QR; a
    /// scanner sends a contact request the owner auto-accepts. All
    /// derivation/encoding/resolution happens Rust-side.
    public func buildAutoAcceptQR(
        ownerIdentityId: Identifier,
        username: String
    ) async throws -> String {
        let handle = self.handle
        let coreSigner = MnemonicResolver()
        let ownerBytes: [UInt8] = ownerIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        return try await Task.detached(priority: .userInitiated) { () -> String in
            var outURI: UnsafeMutablePointer<CChar>?
            let result: PlatformWalletFFIResult = withExtendedLifetime(coreSigner) {
                ownerBytes.withUnsafeBufferPointer { ownerBp in
                    username.withCString { uPtr in
                        platform_wallet_build_auto_accept_qr(
                            handle,
                            ownerBp.baseAddress!,
                            uPtr,
                            coreSigner.handle,
                            &outURI
                        )
                    }
                }
            }
            try result.check()
            guard let outURI else {
                throw PlatformWalletError.nullPointer("auto-accept QR returned a null URI")
            }
            let uri = String(cString: outURI)
            platform_wallet_string_free(outURI)
            return uri
        }.value
    }

    /// Send a contact request from a scanned DIP-15 auto-accept QR
    /// (`dash:?du=<username>&dapk=<key_blob>`): resolve the username, decode the
    /// handed key, sign the proof, and broadcast — so the owner auto-accepts it.
    public func sendContactRequestFromQR(
        senderIdentityId: Identifier,
        uri: String,
        signer: KeychainSigner
    ) async throws -> ContactRequest {
        let handle = self.handle
        let signerHandle = signer.handle
        let coreSigner = MnemonicResolver()
        let senderBytes: [UInt8] = senderIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }

        let requestHandle: Handle = try await Task.detached(priority: .userInitiated) {
            () -> Handle in
            var outHandle: Handle = NULL_HANDLE
            let result: PlatformWalletFFIResult = withExtendedLifetime((signer, coreSigner)) {
                senderBytes.withUnsafeBufferPointer { senderBp -> PlatformWalletFFIResult in
                    uri.withCString { uriPtr in
                        platform_wallet_send_contact_request_from_qr(
                            handle,
                            senderBp.baseAddress!,
                            uriPtr,
                            signerHandle,
                            coreSigner.handle,
                            &outHandle
                        )
                    }
                }
            }
            try result.check()
            return outHandle
        }.value

        return ContactRequest(handle: requestHandle)
    }

    // MARK: - DashPay invitations (DIP-13)

    /// Read-only preview of a `dashpay://invite` link, decoded via
    /// `parseInvitation(uri:)` without claiming it. Drives the claim sheet's
    /// pre-claim summary + the contact-bootstrap decision.
    public struct InvitationPreview: Sendable {
        /// The link decoded structurally. When false, every other field is unset
        /// and the link is malformed / unreadable.
        public let structurallyValid: Bool
        /// The link carried an `islock`, so the claim will build an InstantSend
        /// proof; `false` is a ChainLock-confirmed invite (still claimable). Not a
        /// claimability gate — the proof is reconstructed at claim time.
        public let isInstant: Bool
        /// The link carried inviter METADATA (a username, display name, or
        /// avatar). Presence does NOT mean the contact bootstrap is available:
        /// a metadata-only link (display-name/avatar without a `du` username)
        /// still sets this flag while `inviterUsername` stays nil, and the
        /// bootstrap needs the username. Gate contact features on a non-nil
        /// `inviterUsername`, not on this flag.
        public let hasInviter: Bool
        /// Always nil: the legacy link carries only the inviter's username, not an
        /// identity id (resolve it via `resolveDpnsName` at contact-bootstrap).
        public let inviterId: Data?
        /// Inviter DPNS username when the link carried `du`, else nil — the
        /// contact-bootstrap precondition (may be nil even when `hasInviter`).
        public let inviterUsername: String?
        /// Always 0: the amount isn't in the link (it carries the funding txid,
        /// not the proof) and is only known after the tx is fetched at claim time.
        public let amountDuffs: UInt64
        /// Always 0: the legacy link carries no expiry field.
        public let expiryUnix: UInt32
    }

    /// Create a DashPay invitation (DIP-13): fund a one-time asset-lock voucher
    /// at the invitation derivation path and return a shareable
    /// `dashpay://invite` link.
    ///
    /// **The returned link contains the voucher private key — it is a bearer
    /// credential.** Do NOT log it, and copy it with a sensitive-pasteboard flag
    /// so it isn't synced across devices.
    ///
    /// Pass `inviterIdentityId` + `inviterUsername` to opt into the
    /// contact-bootstrap (the link then carries the inviter so the invitee can
    /// send a contact request back); pass `nil` for both for a pure funding
    /// voucher. `nowUnix` is the current unix time in seconds (e.g.
    /// `UInt32(Date().timeIntervalSince1970)`); the advisory expiry is derived
    /// Rust-side as `nowUnix + MAX_INVITATION_TTL_SECS` (~24h). A zero `nowUnix`
    /// is rejected.
    ///
    /// Builds an L1 asset-lock transaction Rust-side from the `fundingAccount`
    /// (which must have spendable Core UTXOs), so this is a long-running call.
    /// Only the Core-side `MnemonicResolver` is used (no identity signer): pure
    /// voucher creation registers no identity.
    public func createInvitation(
        amountDuffs: UInt64,
        fundingAccount: UInt32,
        inviterIdentityId: Identifier?,
        inviterUsername: String?,
        nowUnix: UInt32
    ) async throws -> String {
        if inviterIdentityId != nil && inviterUsername == nil {
            throw PlatformWalletError.invalidParameter(
                "inviterUsername is required when inviterIdentityId is provided"
            )
        }
        let handle = self.handle
        let coreSigner = MnemonicResolver()
        // Pre-extract the inviter id bytes (nil ⇒ pure funding voucher). The FFI
        // takes the `*const u8` 32-byte identity-id shape shared with
        // `buildAutoAcceptQR` / `read_identifier`.
        let inviterBytes: [UInt8]? = inviterIdentityId.map { id in
            id.withFFIBytes { ptr in Array(UnsafeBufferPointer(start: ptr, count: 32)) }
        }
        let username = inviterUsername
        return try await Task.detached(priority: .userInitiated) { () -> String in
            var outURI: UnsafeMutablePointer<CChar>?
            // `out_outpoint` is required by the FFI but the funding outpoint is
            // not surfaced through this wrapper (the persistence layer tracks it
            // via the asset-lock manager); pass a scratch struct.
            var outOutpoint = OutPointFFI()
            let result: PlatformWalletFFIResult = withExtendedLifetime(coreSigner) {
                () -> PlatformWalletFFIResult in
                // Pin the optional inviter-id buffer + username CString, then call.
                func callWithInviter(
                    _ idPtr: UnsafePointer<UInt8>?
                ) -> PlatformWalletFFIResult {
                    ManagedPlatformWallet.withOptionalCString(username) { usernamePtr in
                        platform_wallet_create_invitation(
                            handle,
                            amountDuffs,
                            fundingAccount,
                            idPtr,
                            usernamePtr,
                            nowUnix,
                            coreSigner.handle,
                            &outURI,
                            &outOutpoint
                        )
                    }
                }
                if let inviterBytes {
                    return inviterBytes.withUnsafeBufferPointer { bp in
                        callWithInviter(bp.baseAddress)
                    }
                } else {
                    return callWithInviter(nil)
                }
            }
            try result.check()
            guard let outURI else {
                throw PlatformWalletError.nullPointer("createInvitation returned a null URI")
            }
            let uri = String(cString: outURI)
            platform_wallet_string_free(outURI)
            return uri
        }.value
    }

    /// Claim a DashPay invitation (DIP-13): register a NEW identity for the
    /// invitee, funded by the imported voucher carried in `uri`.
    ///
    /// This is ordinary identity registration whose *funding* is imported from
    /// the link — so, exactly like `registerIdentityWithFunding`, the caller
    /// MUST pre-derive `identityPubkeys` (the invitee's own new-identity keys)
    /// AND pre-persist each key's private material to the Keychain (via
    /// `prePersistIdentityKeysForRegistration`) BEFORE calling; `signer` produces
    /// the per-identity-key witnesses. The asset-lock's outer signature uses the
    /// imported raw voucher key, so no Core-side resolver signer is needed here.
    ///
    /// `nowUnix` is retained for C ABI compatibility but currently ignored:
    /// the legacy invitation link carries no expiry field, so claim has no
    /// time gate. The contact-bootstrap ("establish contact with the
    /// sender?") is NOT done here — after a successful claim the UI asks the
    /// invitee and, on confirm, calls `sendContactRequest` for the reciprocal.
    ///
    /// Returns the freshly-registered invitee `ManagedIdentity`.
    public func claimInvitation(
        uri: String,
        identityIndex: UInt32,
        identityPubkeys: [ManagedPlatformWallet.IdentityPubkey],
        signer: KeychainSigner,
        nowUnix: UInt32
    ) async throws -> ManagedIdentity {
        guard !identityPubkeys.isEmpty else {
            throw PlatformWalletError.invalidParameter("identityPubkeys is empty")
        }
        let handle = self.handle
        let signerHandle = signer.handle
        let pubkeys = identityPubkeys
        return try await Task.detached(priority: .userInitiated) { () -> ManagedIdentity in
            var idTuple: (
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
            ) = (
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            )
            var outManagedHandle: Handle = NULL_HANDLE
            let pubkeyBuffers: [Data] = pubkeys.map { $0.pubkeyBytes }
            let result = withExtendedLifetime(signer) {
                () -> PlatformWalletFFIResult in
                uri.withCString { uriPtr in
                    ManagedPlatformWallet.withPubkeyFFIArray(
                        pubkeys,
                        buffers: pubkeyBuffers
                    ) { ffiRowsPtr, ffiRowsCount in
                        platform_wallet_claim_invitation(
                            handle,
                            uriPtr,
                            identityIndex,
                            ffiRowsPtr,
                            UInt(ffiRowsCount),
                            signerHandle,
                            nowUnix,
                            &idTuple,
                            &outManagedHandle
                        )
                    }
                }
            }
            try result.check()
            // On Success the managed-identity handle must be non-NULL; wrapping
            // NULL_HANDLE would defer the failure to a harder-to-debug point.
            guard outManagedHandle != NULL_HANDLE else {
                throw PlatformWalletError.walletOperation(
                    "FFI returned success but managed-identity handle was NULL"
                )
            }
            return ManagedIdentity(handle: outManagedHandle)
        }.value
    }

    /// Read-only preview of a DashPay invitation link (DIP-13): decode a
    /// `dashpay://invite` URI and surface its metadata WITHOUT claiming it — no
    /// network, no identity registered. The claim UI uses this to show the
    /// amount, sender, and expiry before the user commits, and to decide whether
    /// to offer the "establish contact with <sender>?" bootstrap.
    ///
    /// A malformed link is reported as `structurallyValid == false` rather than
    /// throwing, so the UI can render a clean "invalid link" state.
    public func parseInvitation(uri: String) throws -> InvitationPreview {
        var out = InvitationPreviewFFI()
        let result = uri.withCString { uriPtr in
            platform_wallet_parse_invitation(uriPtr, &out)
        }
        try result.check()
        // The Rust side heap-allocates the username C string when the link
        // carries an inviter; free it once we've copied it into Swift.
        defer {
            if out.inviter_username != nil {
                platform_wallet_string_free(out.inviter_username)
            }
        }
        // Always nil, matching the documented contract: the legacy link
        // carries no inviter identity id (the ABI's `inviter_id` is
        // deliberately all-zero), and surfacing a zero sentinel here would let
        // a consumer skip the required DPNS username resolution.
        let inviterId: Data? = nil
        let inviterUsername: String? = out.inviter_username.map { String(cString: $0) }
        return InvitationPreview(
            structurallyValid: out.structurally_valid,
            isInstant: out.is_instant,
            hasInviter: out.has_inviter,
            inviterId: inviterId,
            inviterUsername: inviterUsername,
            amountDuffs: out.amount_duffs,
            expiryUnix: out.expiry_unix
        )
    }

    /// Accept an incoming contact request using an externally-supplied
    /// `KeychainSigner` for the reciprocal request's document
    /// state-transition.
    public func acceptContactRequest(
        _ request: ContactRequest,
        signer: KeychainSigner
    ) async throws -> EstablishedContact {
        let walletHandle = self.handle
        let requestHandle = request.handle
        let signerHandle = signer.handle
        // Resolver-backed core signer: the reciprocal request's contact crypto
        // (ECDH + external-account registration) is derived through the Keychain
        // mnemonic resolver Rust-side, so no resident seed is needed.
        let coreSigner = MnemonicResolver()
        let establishedHandle: Handle = try await Task.detached(
            priority: .userInitiated
        ) { () -> Handle in
            var outHandle: Handle = NULL_HANDLE
            // Keep both signers alive across the FFI call (vtable callbacks fire
            // during it); a bare `_ = ...` lets the optimizer drop them.
            let result = withExtendedLifetime((signer, coreSigner)) {
                platform_wallet_accept_contact_request_with_signer(
                    walletHandle,
                    requestHandle,
                    signerHandle,
                    coreSigner.handle,
                    &outHandle
                )
            }
            try result.check()
            return outHandle
        }.value
        return EstablishedContact(handle: establishedHandle)
    }

    /// Ignore a contact sender (per-sender mute, = block, reversible).
    ///
    /// Drops the sender's pending incoming request and suppresses ALL of
    /// their requests (including rotated ones) from the main pending list
    /// on every future sync sweep. Ignore is **local-only** — no on-chain
    /// artifact; it's persisted through the changeset → SwiftData pipeline
    /// so it survives a relaunch. Reverse with `unignoreContactSender`.
    public func ignoreContactSender(
        ourIdentityId: Identifier,
        contactIdentityId: Identifier
    ) async throws {
        let handle = self.handle
        let ourBytes: [UInt8] = ourIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let contactBytes: [UInt8] = contactIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        try await Task.detached(priority: .userInitiated) {
            let result = ourBytes.withUnsafeBufferPointer {
                ourBp -> PlatformWalletFFIResult in
                contactBytes.withUnsafeBufferPointer { contactBp in
                    platform_wallet_ignore_contact_sender(
                        handle,
                        ourBp.baseAddress!,
                        contactBp.baseAddress!
                    )
                }
            }
            try result.check()
        }.value
    }

    /// Un-ignore a contact sender (reverse `ignoreContactSender`).
    ///
    /// Removes the sender from the ignore set and rewinds the received
    /// sync cursor so the next sweep re-fetches their on-chain requests
    /// (otherwise the cursor has already passed them and they'd never
    /// reappear). A no-op when the sender wasn't ignored.
    public func unignoreContactSender(
        ourIdentityId: Identifier,
        contactIdentityId: Identifier
    ) async throws {
        let handle = self.handle
        let ourBytes: [UInt8] = ourIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let contactBytes: [UInt8] = contactIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        try await Task.detached(priority: .userInitiated) {
            let result = ourBytes.withUnsafeBufferPointer {
                ourBp -> PlatformWalletFFIResult in
                contactBytes.withUnsafeBufferPointer { contactBp in
                    platform_wallet_unignore_contact_sender(
                        handle,
                        ourBp.baseAddress!,
                        contactBp.baseAddress!
                    )
                }
            }
            try result.check()
        }.value
    }

    /// Query Platform for contact requests sent by `identityId`.
    public func fetchSentContactRequests(
        identityId: Identifier
    ) async throws -> [ContactRequest] {
        let handle = self.handle
        let idBytes: [UInt8] = identityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        return try await Task.detached(priority: .userInitiated) { () -> [ContactRequest] in
            var array = ContactRequestHandleArray(handles: nil, count: 0)
            let result = idBytes.withUnsafeBufferPointer { idBp in
                platform_wallet_fetch_sent_contact_requests(
                    handle,
                    idBp.baseAddress!,
                    &array
                )
            }
            try result.check()
            defer { platform_wallet_contact_request_handle_array_free(&array) }
            guard let handles = array.handles, array.count > 0 else {
                return []
            }
            var requests: [ContactRequest] = []
            requests.reserveCapacity(Int(array.count))
            for i in 0..<Int(array.count) {
                requests.append(ContactRequest(handle: handles[i]))
            }
            return requests
        }.value
    }

    /// Send a Dash payment to an established DashPay contact.
    /// `amountDuffs` is in duffs (1 DASH = 100_000_000 duffs).
    /// Returns the 32-byte transaction id plus the exact network fee
    /// (duffs) of the broadcast transaction, computed Rust-side as
    /// Σ(selected input values) − Σ(output values) — so any sub-dust
    /// change the builder folds into the fee is reflected, not the
    /// builder's size-based estimate.
    ///
    /// Prerequisite: `register_external_contact_account` must have
    /// run for the `(fromIdentityId, toContactIdentityId)` pair on
    /// the Rust side. The Rust side handles that automatically when
    /// contacts are established via `acceptContactRequest`.
    @discardableResult
    public func sendDashPayPayment(
        fromIdentityId: Identifier,
        toContactIdentityId: Identifier,
        amountDuffs: UInt64,
        memo: String? = nil
    ) async throws -> (txid: Data, feeDuffs: UInt64) {
        let handle = self.handle
        let fromBytes: [UInt8] = fromIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let toBytes: [UInt8] = toContactIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let memoCopy = memo
        // Resolver-backed core signer owns mnemonic access for the lifetime
        // of this call. Each funding-input ECDSA signature happens atomically
        // inside the resolver vtable (mnemonic fetched from Keychain, key
        // derived, digest signed, buffers zeroed) — the seed never becomes
        // resident and no private key leaves Swift.
        let coreSigner = MnemonicResolver()
        return try await Task.detached(priority: .userInitiated) { () -> (txid: Data, feeDuffs: UInt64) in
            var feeDuffs: UInt64 = 0
            var txidTuple: (
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
            ) = (
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            )
            // `withExtendedLifetime` (not a bare `_ = coreSigner`) keeps the
            // resolver alive across the synchronous FFI call — the optimizer
            // can otherwise drop it mid-call and the vtable callback would
            // use-after-free.
            let result: PlatformWalletFFIResult = withExtendedLifetime(coreSigner) {
                fromBytes.withUnsafeBufferPointer { fromBp -> PlatformWalletFFIResult in
                    toBytes.withUnsafeBufferPointer { toBp -> PlatformWalletFFIResult in
                        let call: (UnsafePointer<CChar>?) -> PlatformWalletFFIResult = { memoPtr in
                            platform_wallet_send_dashpay_payment(
                                handle,
                                fromBp.baseAddress!,
                                toBp.baseAddress!,
                                amountDuffs,
                                memoPtr,
                                coreSigner.handle,
                                &txidTuple,
                                &feeDuffs
                            )
                        }
                        if let memoCopy {
                            return memoCopy.withCString { call($0) }
                        } else {
                            return call(nil)
                        }
                    }
                }
            }
            try result.check()
            let txid = Swift.withUnsafeBytes(of: &txidTuple) { Data($0) }
            return (txid: txid, feeDuffs: feeDuffs)
        }.value
    }
}

// MARK: - DashPay Profile operations

extension ManagedPlatformWallet {
    /// Read the cached DashPay profile for `identityId` directly from
    /// this wallet's live state.
    ///
    /// Convenient for UI layers that track identities by ID and don't
    /// hold a live `ManagedIdentity` handle. Returns `nil` when the
    /// identity has no cached profile; throws `.identityNotFound`
    /// when the wallet doesn't know this identity.
    ///
    /// Sync, lock-free — call `syncDashPayProfiles()` first when you
    /// want the freshest on-chain data.
    public func getDashPayProfile(identityId: Identifier) throws -> DashPayProfile? {
        var ffiProfile = DashPayProfileFFI()
        var hasProfile: Bool = false

        let result = identityId.withFFIBytes { idPtr in
            platform_wallet_get_dashpay_profile(
                handle,
                idPtr,
                &ffiProfile,
                &hasProfile
            )
        }
        defer { dashpay_profile_ffi_free(&ffiProfile) }

        try result.check()
        guard hasProfile else { return nil }
        return DashPayProfile(ffi: ffiProfile)
    }

    /// Read the cached profile of a **contact** (by contact identity id)
    /// under `ownerIdentityId`, from this wallet's live state.
    ///
    /// Returns `nil` when the owner has no cached entry for that contact, or
    /// the contact published no profile on Platform. The cache is populated by
    /// the background contact-profile sync and covers established contacts and
    /// pending senders. For a contact that is itself one of the wallet's own
    /// identities, use `getDashPayProfile(identityId:)` (its own profile is
    /// authoritative) — the contact cache intentionally skips such ids.
    ///
    /// Sync, lock-free read of the in-memory cache.
    public func getContactProfile(
        ownerIdentityId: Identifier,
        contactIdentityId: Identifier
    ) throws -> DashPayProfile? {
        var ffiProfile = DashPayProfileFFI()
        var hasProfile: Bool = false

        let result = ownerIdentityId.withFFIBytes { ownerPtr in
            contactIdentityId.withFFIBytes { contactPtr in
                platform_wallet_get_contact_profile(
                    handle,
                    ownerPtr,
                    contactPtr,
                    &ffiProfile,
                    &hasProfile
                )
            }
        }
        defer { dashpay_profile_ffi_free(&ffiProfile) }

        try result.check()
        guard hasProfile else { return nil }
        return DashPayProfile(ffi: ffiProfile)
    }

    /// Read the DashPay payment history for `identityId` directly
    /// from this wallet's live state.
    ///
    /// Convenient for UI layers that track identities by ID and don't
    /// hold a live `ManagedIdentity` handle. Throws
    /// `.identityNotFound` when the wallet doesn't know this
    /// identity; returns an empty array when no payments have been
    /// recorded.
    ///
    /// Sync, lock-free read of the in-memory cache. To land the rows
    /// in SwiftData for `@Query` consumption, go through
    /// `PlatformWalletManager.refreshDashPayPayments(walletId:identityId:)`
    /// instead.
    public func getDashPayPayments(identityId: Identifier) throws -> [DashPayPayment] {
        try managedIdentity(identityId: identityId).getDashPayPayments()
    }

    /// Refresh every managed identity's DashPay profile cache from
    /// Platform.
    ///
    /// Returns the number of identities for which a `profile` document
    /// was found on-chain. Identities with no on-chain profile have
    /// their local cache cleared (if any).
    ///
    /// Blocks the Rust side on an 8 MB tokio worker until the sync
    /// completes — expected to be driven from an async context so the
    /// main thread isn't blocked on proof verification.
    @discardableResult
    public func syncDashPayProfiles() async throws -> UInt32 {
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) { () -> UInt32 in
            var syncedCount: UInt32 = 0
            let result = platform_wallet_sync_dashpay_profiles(
                handle,
                &syncedCount
            )
            try result.check()
            return syncedCount
        }.value
    }

    /// Create a new DashPay profile using an externally-supplied
    /// `KeychainSigner` for the document state-transition.
    ///
    /// Architecturally aligned with `registerDpnsName(...signer:)` —
    /// the wallet's own seed never participates Rust-side.
    @discardableResult
    public func createDashPayProfile(
        identityId: Identifier,
        update: DashPayProfileUpdate,
        signer: KeychainSigner
    ) async throws -> DashPayProfile {
        try await submitDashPayProfileWithSigner(
            identityId: identityId,
            update: update,
            doCreate: true,
            signer: signer
        )
    }

    /// Update an existing DashPay profile using an
    /// externally-supplied `KeychainSigner`.
    @discardableResult
    public func updateDashPayProfile(
        identityId: Identifier,
        update: DashPayProfileUpdate,
        signer: KeychainSigner
    ) async throws -> DashPayProfile {
        try await submitDashPayProfileWithSigner(
            identityId: identityId,
            update: update,
            doCreate: false,
            signer: signer
        )
    }

    /// Shared submit path for the signer-driven profile flows. Mirrors
    /// `submitDashPayProfile` but routes through
    /// `platform_wallet_create_or_update_dashpay_profile_with_signer`.
    private func submitDashPayProfileWithSigner(
        identityId: Identifier,
        update: DashPayProfileUpdate,
        doCreate: Bool,
        signer: KeychainSigner
    ) async throws -> DashPayProfile {
        let handle = self.handle
        let signerHandle = signer.handle
        let idBytes: [UInt8] = identityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let displayName = update.displayName
        let publicMessage = update.publicMessage
        let avatarUrl = update.avatarUrl
        let avatarBytes = update.avatarBytes

        return try await Task.detached(priority: .userInitiated) { () -> DashPayProfile in
            var outProfile = DashPayProfileFFI()

            // Pin the KeychainSigner across the whole synchronous FFI call:
            // Rust holds a `passUnretained` ctx pointer to it via `signerHandle`,
            // so a bare `_ = signer` keepalive can be released under -O before the
            // call returns. `withExtendedLifetime` keeps ARC holding it for the
            // call's duration (matches the other *_with_signer wrappers).
            let result: PlatformWalletFFIResult = withExtendedLifetime(signer) {
                idBytes.withUnsafeBufferPointer {
                    idBp -> PlatformWalletFFIResult in
                    let idPtr = idBp.baseAddress!
                    return invokeWithOptionalCStrings(
                        displayName,
                        publicMessage,
                        avatarUrl
                    ) { namePtr, msgPtr, urlPtr -> PlatformWalletFFIResult in
                        let bytes = avatarBytes ?? Data()
                        if let avatarBytes, !avatarBytes.isEmpty {
                            return avatarBytes.withUnsafeBytes { rawBuf -> PlatformWalletFFIResult in
                                let bytesPtr = rawBuf.baseAddress?.assumingMemoryBound(to: UInt8.self)
                                return platform_wallet_create_or_update_dashpay_profile_with_signer(
                                    handle,
                                    idPtr,
                                    namePtr,
                                    msgPtr,
                                    urlPtr,
                                    bytesPtr,
                                    UInt(avatarBytes.count),
                                    doCreate,
                                    signerHandle,
                                    &outProfile
                                )
                            }
                        } else {
                            _ = bytes
                            return platform_wallet_create_or_update_dashpay_profile_with_signer(
                                handle,
                                idPtr,
                                namePtr,
                                msgPtr,
                                urlPtr,
                                nil,
                                0,
                                doCreate,
                                signerHandle,
                                &outProfile
                            )
                        }
                    }
                }
            }

            defer { dashpay_profile_ffi_free(&outProfile) }

            try result.check()
            return DashPayProfile(ffi: outProfile)
        }.value
    }

    /// Set the owner-private alias / note / hidden flag for an
    /// established contact and publish the self-encrypted
    /// `contactInfo` document. Local state (and hence
    /// the SwiftData contact rows) updates immediately; the network
    /// write is deferred by the Rust side under DIP-15's
    /// ≥2-established-contacts privacy rule.
    /// Outcome of `setDashPayContactInfo`: local state is always updated,
    /// but the cross-device document publish may be deferred or skipped.
    /// Mirrors the Rust `ContactInfoPublishOutcome` / the FFI
    /// `CONTACT_INFO_*` discriminants.
    public enum ContactInfoPublishOutcome: Sendable {
        /// Published on Platform — synced cross-device.
        case published
        /// Saved locally; publish deferred by DIP-15 until the identity
        /// has at least two established contacts.
        case deferredUntilTwoContacts
        /// Saved locally; publish not possible for a watch-only identity.
        case skippedWatchOnly
    }

    @discardableResult
    public func setDashPayContactInfo(
        identityId: Identifier,
        contactId: Identifier,
        alias: String?,
        note: String?,
        hidden: Bool,
        signer: KeychainSigner
    ) async throws -> ContactInfoPublishOutcome {
        let handle = self.handle
        let signerHandle = signer.handle
        // Resolver-backed core signer: the contactInfo seal/find-existing crypto
        // is derived through the Keychain mnemonic resolver Rust-side, so no
        // resident seed is needed.
        let coreSigner = MnemonicResolver()
        let idBytes: [UInt8] = identityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let contactBytes: [UInt8] = contactId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }

        let outcomeRaw: UInt8 = try await Task.detached(priority: .userInitiated) {
            var outcomeRaw: UInt8 = 0
            // Keep both signers alive across the FFI call (vtable callbacks fire
            // during it); a bare `_ = ...` lets the optimizer drop them.
            let result: PlatformWalletFFIResult = withExtendedLifetime((signer, coreSigner)) {
                idBytes.withUnsafeBufferPointer { idBp in
                    contactBytes.withUnsafeBufferPointer { contactBp in
                        invokeWithOptionalCStrings(alias, note, nil) { aliasPtr, notePtr, _ in
                            platform_wallet_set_dashpay_contact_info_with_signer(
                                handle,
                                idBp.baseAddress!,
                                contactBp.baseAddress!,
                                aliasPtr,
                                notePtr,
                                hidden,
                                signerHandle,
                                coreSigner.handle,
                                &outcomeRaw
                            )
                        }
                    }
                }
            }
            try result.check()
            return outcomeRaw
        }.value

        switch outcomeRaw {
        case UInt8(CONTACT_INFO_DEFERRED_UNTIL_TWO_CONTACTS):
            return .deferredUntilTwoContacts
        case UInt8(CONTACT_INFO_SKIPPED_WATCH_ONLY):
            return .skippedWatchOnly
        default:
            return .published
        }
    }

}

// MARK: - In-memory state (Wallet Memory Explorer)

/// Snapshot of the in-memory state a `ManagedPlatformWallet` holds
/// for diagnostic display. Returned by
/// `ManagedPlatformWallet.inMemorySummary()`. Mirrors
/// `PlatformWalletMemorySummaryFFI` on the Rust side.
public struct InMemoryWalletSummary: Sendable {
    /// Number of signing-capable identities the wallet manages
    /// (i.e. lives in `wallet_identities[wallet_id]`).
    public let identitiesCount: Int
    /// Number of read-only / observed identities
    /// (`out_of_wallet_identities`).
    public let watchedCount: Int
    /// One past the wallet's highest already-registered identity
    /// index — the resume position the gap-limit scanner uses next.
    /// `0` when the wallet has no managed identities yet.
    public let lastScannedIndex: UInt32
    /// 32-byte primary identity id, or `nil` when no primary is set.
    ///
    /// Always `nil` from the Rust side now — primary-identity
    /// selection moved to the UI layer. The field is retained on
    /// this snapshot struct so existing call sites keep compiling;
    /// the Wallet Memory Explorer view should source the user's
    /// pick from app state rather than from this struct.
    public let primaryIdentityId: Identifier?
    /// Number of asset locks tracked in
    /// `PlatformWalletInfo.tracked_asset_locks`.
    public let trackedAssetLocksCount: Int

    public init(
        identitiesCount: Int,
        watchedCount: Int,
        lastScannedIndex: UInt32,
        primaryIdentityId: Identifier?,
        trackedAssetLocksCount: Int
    ) {
        self.identitiesCount = identitiesCount
        self.watchedCount = watchedCount
        self.lastScannedIndex = lastScannedIndex
        self.primaryIdentityId = primaryIdentityId
        self.trackedAssetLocksCount = trackedAssetLocksCount
    }
}

extension ManagedPlatformWallet {
    /// List the ids of every identity the wallet currently manages
    /// (signing-capable identities — not watched ones).
    ///
    /// Reads `info.identity_manager.identities` directly. This is
    /// what the in-memory state holds *right now*; differs from
    /// SwiftData's `PersistentIdentity` query when the wallet hasn't
    /// rehydrated all of its persisted identities into memory.
    public func inMemoryIdentityIds() throws -> [Identifier] {
        try readIdentifierArray { array in
            platform_wallet_list_in_memory_identity_ids(handle, &array)
        }
    }

    /// List the ids of every watched (read-only / observed) identity
    /// the wallet currently knows about. Reads
    /// `info.identity_manager.watched_identities` directly.
    public func inMemoryWatchedIdentityIds() throws -> [Identifier] {
        try readIdentifierArray { array in
            platform_wallet_list_in_memory_watched_identity_ids(handle, &array)
        }
    }

    /// Read a one-shot summary of the wallet's in-memory state — the
    /// counts and watermarks the Wallet Memory Explorer view surfaces
    /// at the top of the per-wallet detail screen.
    public func inMemorySummary() throws -> InMemoryWalletSummary {
        var ffi = PlatformWalletMemorySummaryFFI()

        let result = platform_wallet_get_in_memory_summary(handle, &ffi)
        try result.check()

        return InMemoryWalletSummary(
            identitiesCount: Int(ffi.identities_count),
            watchedCount: Int(ffi.watched_count),
            lastScannedIndex: ffi.last_scanned_index,
            // Primary-identity selection no longer lives on the Rust
            // side; UI layer owns it now.
            primaryIdentityId: nil,
            trackedAssetLocksCount: Int(ffi.tracked_asset_locks_count)
        )
    }

    /// Shared array-marshalling body for the two id-list explorer
    /// readers above. Both wrap the same `IdentifierArray` FFI shape +
    /// free helper, so the per-method variance is just the FFI call
    /// itself.
    private func readIdentifierArray(
        _ fetch: (inout IdentifierArray) -> PlatformWalletFFIResult
    ) throws -> [Identifier] {
        var array = IdentifierArray()
        try fetch(&array).check()
        defer { platform_wallet_identifier_array_free(&array) }
        guard array.items != nil, array.count > 0 else {
            return []
        }
        var ids: [Identifier] = []
        ids.reserveCapacity(Int(array.count))
        for i in 0..<Int(array.count) {
            ids.append(identifierFromFFIArray(array, at: i))
        }
        return ids
    }
}

/// Run `body` with three optional C-string pointers, each `nil` when
/// the corresponding input string is `nil`. Swift's `withCString` has
/// closure-bounded lifetime, so this helper keeps all three buffers
/// alive across a single FFI call without deeply nested ternaries.
///
/// Deliberately non-escaping: the closure runs synchronously while
/// the enclosing `withCString` frames are live.
private func invokeWithOptionalCStrings<R>(
    _ a: String?,
    _ b: String?,
    _ c: String?,
    body: (UnsafePointer<CChar>?, UnsafePointer<CChar>?, UnsafePointer<CChar>?) -> R
) -> R {
    func step2(
        _ aPtr: UnsafePointer<CChar>?,
        _ bPtr: UnsafePointer<CChar>?
    ) -> R {
        if let c {
            return c.withCString { cPtr in body(aPtr, bPtr, cPtr) }
        } else {
            return body(aPtr, bPtr, nil)
        }
    }

    func step1(_ aPtr: UnsafePointer<CChar>?) -> R {
        if let b {
            return b.withCString { bPtr in step2(aPtr, bPtr) }
        } else {
            return step2(aPtr, nil)
        }
    }

    if let a {
        return a.withCString { aPtr in step1(aPtr) }
    } else {
        return step1(nil)
    }
}

// MARK: - Identity transfer / withdraw / update — external-signer wrappers

/// One recipient row for `transferCreditsToAddresses(...)`.
public struct PlatformAddressCreditOutput: Sendable {
    /// Discriminant: `0 = P2PKH`, `1 = P2SH`. Mirrors the Rust-side
    /// `PlatformAddress` enum variant tag.
    public let addressType: UInt8
    /// 20-byte address hash.
    public let hash: Data
    /// Credits to transfer to this address.
    public let credits: UInt64

    public init(addressType: UInt8, hash: Data, credits: UInt64) {
        self.addressType = addressType
        self.hash = hash
        self.credits = credits
    }
}

extension ManagedPlatformWallet {
    /// Transfer `amount` credits from `fromIdentityId` to
    /// `toIdentityId` using the supplied `KeychainSigner` for the
    /// identity-state-transition signature.
    ///
    /// Architecturally aligned with `registerDpnsName(...signer:)` —
    /// no signing happens via the wallet's own seed Rust-side, so
    /// watch-only wallets work end-to-end on this path.
    public func transferCredits(
        fromIdentityId: Identifier,
        toIdentityId: Identifier,
        amount: UInt64,
        signer: KeychainSigner
    ) async throws {
        let handle = self.handle
        let signerHandle = signer.handle
        let fromBytes: [UInt8] = fromIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let toBytes: [UInt8] = toIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        try await Task.detached(priority: .userInitiated) {
            _ = signer
            let result = fromBytes.withUnsafeBufferPointer { fromBp -> PlatformWalletFFIResult in
                toBytes.withUnsafeBufferPointer { toBp in
                    platform_wallet_transfer_credits_with_signer(
                        handle,
                        fromBp.baseAddress!,
                        toBp.baseAddress!,
                        amount,
                        signerHandle
                    )
                }
            }
            try result.check()
        }.value
    }

    /// Transfer credits from `fromIdentityId` to one or more platform
    /// addresses using the supplied `KeychainSigner`.
    ///
    /// Returns the sender's remaining balance after the transfer.
    @discardableResult
    public func transferCreditsToAddresses(
        fromIdentityId: Identifier,
        recipients: [PlatformAddressCreditOutput],
        signer: KeychainSigner
    ) async throws -> UInt64 {
        guard !recipients.isEmpty else {
            throw PlatformWalletError.invalidParameter("recipients is empty")
        }
        let handle = self.handle
        let signerHandle = signer.handle
        let fromBytes: [UInt8] = fromIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }

        // Materialize recipients into FFI rows. Each `hash` Data must
        // contribute a 20-byte tuple to the FFI struct; pad/truncate
        // would silently corrupt addresses, so reject on mismatch.
        var ffiRows: [PlatformAddressCreditOutputFFI] = []
        ffiRows.reserveCapacity(recipients.count)
        for r in recipients {
            guard r.hash.count == 20 else {
                throw PlatformWalletError.walletOperation(
                    "PlatformAddressCreditOutput.hash must be exactly 20 bytes (got \(r.hash.count))"
                )
            }
            // Build a 20-byte tuple from the hash data.
            let bytes = [UInt8](r.hash)
            let tuple = (
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4],
                bytes[5], bytes[6], bytes[7], bytes[8], bytes[9],
                bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                bytes[15], bytes[16], bytes[17], bytes[18], bytes[19]
            )
            ffiRows.append(
                PlatformAddressCreditOutputFFI(
                    address_type: r.addressType,
                    hash: tuple,
                    credits: r.credits
                )
            )
        }
        let rows = ffiRows
        return try await Task.detached(priority: .userInitiated) { () -> UInt64 in
            _ = signer
            var newBalance: UInt64 = 0
            let result = fromBytes.withUnsafeBufferPointer {
                fromBp -> PlatformWalletFFIResult in
                rows.withUnsafeBufferPointer { rowsBp in
                    platform_wallet_transfer_credits_to_addresses_with_signer(
                        handle,
                        fromBp.baseAddress!,
                        rowsBp.baseAddress,
                        UInt(rowsBp.count),
                        signerHandle,
                        &newBalance
                    )
                }
            }
            try result.check()
            return newBalance
        }.value
    }

    /// Withdraw `amount` credits from `identityId` to `toAddress` (a
    /// network-aware Dash P2PKH address string) using the supplied
    /// `KeychainSigner` for the identity-state-transition signature.
    public func withdrawCredits(
        identityId: Identifier,
        amount: UInt64,
        toAddress: String,
        signer: KeychainSigner
    ) async throws {
        let handle = self.handle
        let signerHandle = signer.handle
        let idBytes: [UInt8] = identityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        try await Task.detached(priority: .userInitiated) {
            _ = signer
            let result = idBytes.withUnsafeBufferPointer {
                idBp -> PlatformWalletFFIResult in
                toAddress.withCString { addrPtr in
                    platform_wallet_withdraw_credits_with_signer(
                        handle,
                        idBp.baseAddress!,
                        amount,
                        addrPtr,
                        signerHandle
                    )
                }
            }
            try result.check()
        }.value
    }

    /// Update an identity by adding new public keys and/or disabling
    /// existing key IDs, signing the resulting
    /// `IdentityUpdateTransition` with the identity's MASTER auth key
    /// via the supplied `KeychainSigner`.
    ///
    /// `addPublicKeys` rows MUST already have their matching private
    /// material persisted to the signer's store BEFORE calling this —
    /// otherwise subsequent operations that try to sign with the
    /// newly-added keys will fail. The signer here only signs the
    /// update transition itself with an existing MASTER key.
    public func updateIdentity(
        identityId: Identifier,
        addPublicKeys: [ManagedPlatformWallet.IdentityPubkey] = [],
        disablePublicKeyIds: [UInt32] = [],
        signer: KeychainSigner
    ) async throws {
        let handle = self.handle
        let signerHandle = signer.handle
        let idBytes: [UInt8] = identityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let addPubkeys = addPublicKeys
        let disableIds = disablePublicKeyIds
        try await Task.detached(priority: .userInitiated) {
            _ = signer
            // Mirror the registration FFI's pubkey-pinning pattern via
            // `withPubkeyFFIArray` so each `pubkey_bytes` pointer the
            // FFI sees stays valid for the call duration.
            let pubkeyBuffers: [Data] = addPubkeys.map { $0.pubkeyBytes }
            let result = idBytes.withUnsafeBufferPointer {
                idBp -> PlatformWalletFFIResult in
                disableIds.withUnsafeBufferPointer { disableBp in
                    if addPubkeys.isEmpty {
                        return platform_wallet_update_identity_with_signer(
                            handle,
                            idBp.baseAddress!,
                            nil,
                            0,
                            disableIds.isEmpty ? nil : disableBp.baseAddress,
                            UInt(disableIds.count),
                            signerHandle
                        )
                    }
                    return ManagedPlatformWallet.withPubkeyFFIArray(
                        addPubkeys,
                        buffers: pubkeyBuffers
                    ) { ffiRowsPtr, ffiRowsCount in
                        platform_wallet_update_identity_with_signer(
                            handle,
                            idBp.baseAddress!,
                            ffiRowsPtr,
                            UInt(ffiRowsCount),
                            disableIds.isEmpty ? nil : disableBp.baseAddress,
                            UInt(disableIds.count),
                            signerHandle
                        )
                    }
                }
            }
            try result.check()
        }.value
    }

    /// Parse a raw `IdentityUpdateTransition` from DPP bytes without
    /// signing or broadcasting it. Accepts both standard tagged bytes
    /// and Yappr's tagless `dash-st:` framing.
    public func parseIdentityUpdateTransition(_ bytes: Data) throws -> ParsedIdentityUpdateTransition {
        guard !bytes.isEmpty else {
            throw PlatformWalletError.deserialization(
                "IdentityUpdateTransition bytes are empty"
            )
        }

        var out = ParsedIdentityUpdateFFI(
            identity_id: (
                0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0
            ),
            add_public_keys: nil,
            add_public_keys_count: 0,
            disable_public_key_ids: nil,
            disable_public_key_ids_count: 0
        )

        let result = bytes.withUnsafeBytes { rawBuffer -> PlatformWalletFFIResult in
            let byteBuffer = rawBuffer.bindMemory(to: UInt8.self)
            return platform_wallet_parse_identity_update_transition(
                byteBuffer.baseAddress,
                UInt(byteBuffer.count),
                &out
            )
        }
        try result.check()
        defer { platform_wallet_parse_identity_update_transition_free(&out) }

        var identityTuple = out.identity_id
        let identityId = Swift.withUnsafeBytes(of: &identityTuple) { Data($0) }

        let addPublicKeys: [IdentityPubkey]
        if let pointer = out.add_public_keys, out.add_public_keys_count > 0 {
            let buffer = UnsafeBufferPointer(start: pointer, count: Int(out.add_public_keys_count))
            addPublicKeys = try buffer.enumerated().map { index, entry in
                try Self.makeParsedIdentityPubkey(from: entry, index: index)
            }
        } else {
            addPublicKeys = []
        }

        let disablePublicKeyIds: [UInt32]
        if let pointer = out.disable_public_key_ids, out.disable_public_key_ids_count > 0 {
            disablePublicKeyIds = Array(
                UnsafeBufferPointer(
                    start: pointer,
                    count: Int(out.disable_public_key_ids_count)
                )
            )
        } else {
            disablePublicKeyIds = []
        }

        return ParsedIdentityUpdateTransition(
            identityId: identityId,
            addPublicKeys: addPublicKeys,
            disablePublicKeyIds: disablePublicKeyIds
        )
    }

    private static func makeParsedIdentityPubkey(
        from entry: ParsedIdentityUpdatePublicKeyFFI,
        index: Int
    ) throws -> IdentityPubkey {
        guard let keyType = KeyType(rawValue: entry.key_type),
              let purpose = KeyPurpose(rawValue: entry.purpose),
              let securityLevel = SecurityLevel(rawValue: entry.security_level) else {
            throw PlatformWalletError.deserialization(
                "Unknown IdentityUpdateTransition public-key enum discriminant at index \(index)"
            )
        }

        let pubkeyBytes: Data
        if let dataPtr = entry.data_ptr {
            pubkeyBytes = Data(bytes: dataPtr, count: Int(entry.data_len))
        } else if entry.data_len == 0 {
            pubkeyBytes = Data()
        } else {
            throw PlatformWalletError.deserialization(
                "IdentityUpdateTransition public key \(index) had a null data pointer for \(entry.data_len) bytes"
            )
        }

        let contractBounds = try parsedContractBounds(from: entry, index: index)

        return IdentityPubkey(
            keyId: entry.key_id,
            keyType: keyType,
            purpose: purpose,
            securityLevel: securityLevel,
            pubkeyBytes: pubkeyBytes,
            readOnly: entry.read_only,
            contractBounds: contractBounds
        )
    }

    private static func parsedContractBounds(
        from entry: ParsedIdentityUpdatePublicKeyFFI,
        index: Int
    ) throws -> ContractBounds? {
        switch entry.contract_bounds_kind {
        case 0:
            return nil
        case 1:
            var idTuple = entry.contract_bounds_id
            return .singleContract(
                id: Swift.withUnsafeBytes(of: &idTuple) { Data($0) }
            )
        case 2:
            guard let documentTypePtr = entry.contract_bounds_document_type,
                  let documentTypeName = String(validatingCString: documentTypePtr) else {
                throw PlatformWalletError.deserialization(
                    "IdentityUpdateTransition contract bounds at key \(index) are missing a valid UTF-8 document type"
                )
            }
            var idTuple = entry.contract_bounds_id
            return .singleContractDocumentType(
                id: Swift.withUnsafeBytes(of: &idTuple) { Data($0) },
                documentTypeName: documentTypeName
            )
        default:
            throw PlatformWalletError.deserialization(
                "Unknown IdentityUpdateTransition contract-bounds kind \(entry.contract_bounds_kind) at key \(index)"
            )
        }
    }

    /// Create + broadcast a new data contract owned by
    /// `ownerIdentityId`. Returns the 32-byte contract id once
    /// Platform confirms the transition.
    ///
    /// Replaces the older `sdk.dataContractCreate(...)` call site:
    /// the rs-sdk-ffi runtime's mobile-tuned default thread stack
    /// is too small for `rs-drive`'s post-broadcast GroveDB proof
    /// recursion (`EXC_BAD_ACCESS` at the `Op::decode` prologue).
    /// The platform-wallet runtime uses an 8 MB worker stack and
    /// fits the recursion comfortably. Architecturally this also
    /// follows the `swift-sdk/CLAUDE.md` "high-level operations
    /// go through platform-wallet" rule — contract creation
    /// spans an identity (the owner), needs the wallet's signer,
    /// and changes persistent state.
    ///
    /// JSON shapes (every input beyond `documentSchemasJSON` is
    /// optional — pass `nil` to skip):
    ///   - `documentSchemasJSON`: object keyed by document type
    ///     name; `"{}"` for token-only contracts.
    ///   - `tokenSchemasJSON`: object keyed by stringified slot
    ///     index. Caller supplies the three-level
    ///     `$formatVersion: "0"` tags (`TokenConfiguration` /
    ///     `TokenConfigurationConvention` /
    ///     `TokenConfigurationLocalization`); the V1 deserializer
    ///     can't dispatch without them.
    ///   - `groupsJSON`: object keyed by stringified group
    ///     position.
    ///   - `keywordsJSON`: JSON array of strings.
    ///   - `description`: plain string.
    ///   - `contractConfigJSON`: `DataContractConfig` JSON. The
    ///     library injects `$formatVersion: "0"` if missing so a
    ///     bare flags dict round-trips cleanly.
    ///
    /// Lifetime contract: the `signer` instance MUST stay alive
    /// for the duration of the `await` (Rust holds a `passUnretained`
    /// ctx pointer to the underlying `KeychainSigner`). A `_ = signer`
    /// keepalive at the call site is the canonical way to pin it.
    public func createDataContract(
        ownerIdentityId: Identifier,
        documentSchemasJSON: String,
        tokenSchemasJSON: String? = nil,
        groupsJSON: String? = nil,
        keywordsJSON: String? = nil,
        description: String? = nil,
        contractConfigJSON: String? = nil,
        signer: KeychainSigner
    ) async throws -> Identifier {
        let handle = self.handle
        let signerHandle = signer.handle
        let idBytes: [UInt8] = ownerIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        return try await Task.detached(priority: .userInitiated) {
            // Pin every borrowed payload across the FFI call: the
            // owner-id bytes, the required documents string, and
            // each optional JSON / description string. The Rust
            // side dereferences the C-string pointers
            // synchronously inside `block_on_worker`, so the
            // `withCString` scopes here are sufficient — the
            // pointers don't need to outlive the call.
            _ = signer
            var contractIdBytes = [UInt8](repeating: 0, count: 32)

            let result = idBytes.withUnsafeBufferPointer { idBp -> PlatformWalletFFIResult in
                documentSchemasJSON.withCString { docsPtr in
                    Self.withOptionalCString(tokenSchemasJSON) { tokensPtr in
                        Self.withOptionalCString(groupsJSON) { groupsPtr in
                            Self.withOptionalCString(keywordsJSON) { keywordsPtr in
                                Self.withOptionalCString(description) { descriptionPtr in
                                    Self.withOptionalCString(contractConfigJSON) { configPtr in
                                        contractIdBytes.withUnsafeMutableBufferPointer { outBp in
                                            platform_wallet_create_data_contract_with_signer(
                                                handle,
                                                idBp.baseAddress!,
                                                docsPtr,
                                                tokensPtr,
                                                groupsPtr,
                                                keywordsPtr,
                                                descriptionPtr,
                                                configPtr,
                                                signerHandle,
                                                outBp.baseAddress!
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            try result.check()
            return Data(contractIdBytes)
        }.value
    }

    /// Update an existing data contract owned by `ownerIdentityId`
    /// and broadcast the change to Platform.
    ///
    /// Companion to `createDataContract`. The wallet fetches the
    /// live contract from Platform, bumps its version, and applies
    /// the supplied schemas / config at the next version — the
    /// caller does NOT track contract version state. Every JSON
    /// input has the same shape as `createDataContract`.
    ///
    /// `contractId` is the id of the existing contract to update.
    /// Returns the (unchanged) contract id of the updated contract.
    ///
    /// Lifetime contract: the `signer` instance MUST stay alive for
    /// the duration of the `await` (Rust holds a `passUnretained`
    /// ctx pointer to the underlying `KeychainSigner`). A
    /// `_ = signer` keepalive at the call site is the canonical way
    /// to pin it.
    public func updateDataContract(
        ownerIdentityId: Identifier,
        contractId: Identifier,
        documentSchemasJSON: String,
        tokenSchemasJSON: String? = nil,
        groupsJSON: String? = nil,
        keywordsJSON: String? = nil,
        description: String? = nil,
        contractConfigJSON: String? = nil,
        signer: KeychainSigner
    ) async throws -> Identifier {
        let handle = self.handle
        let signerHandle = signer.handle
        let ownerBytes: [UInt8] = ownerIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let contractBytes: [UInt8] = contractId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        return try await Task.detached(priority: .userInitiated) {
            // Pin every borrowed payload across the FFI call: the
            // owner-id + contract-id bytes, the required documents
            // string, and each optional JSON / description string.
            // Rust dereferences the C-string pointers synchronously
            // inside `block_on_worker`, so the `withCString` scopes
            // here are sufficient — the pointers don't need to
            // outlive the call.
            _ = signer
            var contractIdBytes = [UInt8](repeating: 0, count: 32)

            let result = ownerBytes.withUnsafeBufferPointer { ownerBp -> PlatformWalletFFIResult in
                contractBytes.withUnsafeBufferPointer { contractBp -> PlatformWalletFFIResult in
                    documentSchemasJSON.withCString { docsPtr in
                        Self.withOptionalCString(tokenSchemasJSON) { tokensPtr in
                            Self.withOptionalCString(groupsJSON) { groupsPtr in
                                Self.withOptionalCString(keywordsJSON) { keywordsPtr in
                                    Self.withOptionalCString(description) { descriptionPtr in
                                        Self.withOptionalCString(contractConfigJSON) { configPtr in
                                            contractIdBytes.withUnsafeMutableBufferPointer { outBp in
                                                platform_wallet_update_data_contract_with_signer(
                                                    handle,
                                                    ownerBp.baseAddress!,
                                                    contractBp.baseAddress!,
                                                    docsPtr,
                                                    tokensPtr,
                                                    groupsPtr,
                                                    keywordsPtr,
                                                    descriptionPtr,
                                                    configPtr,
                                                    signerHandle,
                                                    outBp.baseAddress!
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            try result.check()
            return Data(contractIdBytes)
        }.value
    }

    /// Create + broadcast a new revision-1 document on `contractId`'s
    /// `documentType`, owned by `ownerIdentityId`. Returns the 32-byte
    /// document id and the confirmed document's canonical query-side
    /// JSON once Platform confirms the transition.
    ///
    /// The returned JSON is DPP's canonical representation of the
    /// confirmed document — the same bytes a DOC-01 list query
    /// (`dash_sdk_document_search`) returns: `$formatVersion` present,
    /// `$id`/`$ownerId` as base58 strings, binary properties as base64,
    /// and unset system fields as `null`. Callers persist this verbatim
    /// so the local cache matches the on-chain document rather than the
    /// user's raw form input. (A single-document `documentGet` fetch
    /// uses a different, per-field shape.)
    ///
    /// Routes through `IdentityWallet::create_document_with_signer`
    /// (via `platform_wallet_create_document_with_signer`), the
    /// production document-create path. The Rust side fetches the
    /// on-chain contract, builds the document from `propertiesJSON`,
    /// selects an AUTHENTICATION + ECDSA key whose security level
    /// satisfies the document type's requirement, broadcasts on the
    /// platform-wallet 8 MB worker stack, and waits for confirmation.
    /// This deliberately does NOT use the rs-sdk-ffi test-signer
    /// builder path (`dash_sdk_document_create` /
    /// `dash_sdk_document_put_to_platform_and_wait`): per
    /// `swift-sdk/CLAUDE.md`, the state-transition flow lives in the
    /// `platform-wallet` library and the signing key never crosses
    /// into Swift logic.
    ///
    /// `propertiesJSON` is a JSON object keyed by property name.
    /// Byte-array fields must be encoded as hex strings and identifier
    /// fields as base58 strings (the Rust schema-driven sanitize step
    /// converts them to native bytes / identifiers). Pass `"{}"` for a
    /// document type with no required properties.
    ///
    /// Lifetime contract: the `signer` instance MUST stay alive for the
    /// duration of the synchronous FFI call inside this async wrapper
    /// (Rust holds a `passUnretained` ctx pointer to the underlying
    /// `KeychainSigner`). The wrapper pins it with
    /// `withExtendedLifetime(signer)` around the full marshalling chain —
    /// a bare `_ = signer` is unreliable (the optimizer may elide it).
    public func createDocument(
        ownerIdentityId: Identifier,
        contractId: Identifier,
        documentType: String,
        propertiesJSON: String,
        signer: KeychainSigner
    ) async throws -> (Identifier, String) {
        let handle = self.handle
        let signerHandle = signer.handle
        let ownerBytes: [UInt8] = ownerIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let contractBytes: [UInt8] = contractId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        return try await Task.detached(priority: .userInitiated) {
            // Pin every borrowed payload across the FFI call: the
            // owner-id + contract-id bytes, the document-type name,
            // and the properties JSON. Rust dereferences the
            // C-string pointers synchronously inside
            // `block_on_worker`, so the `withCString` scopes here are
            // sufficient — the pointers don't need to outlive the
            // call.
            var documentIdBytes = [UInt8](repeating: 0, count: 32)
            // Receives an owned canonical-document JSON C string on
            // success; freed with `platform_wallet_string_free` below.
            var documentJsonPtr: UnsafeMutablePointer<CChar>? = nil

            // Pin `signer` for the whole FFI call. A bare `_ = signer` is
            // unreliable folklore — the optimizer may elide it in -O and
            // release the signer before Rust dereferences `signerHandle`
            // (especially in a detached task), causing a use-after-free.
            // `withExtendedLifetime` guarantees it, matching the other
            // `*_with_signer` wrappers in this file.
            let result = withExtendedLifetime(signer) {
                ownerBytes.withUnsafeBufferPointer { ownerBp -> PlatformWalletFFIResult in
                    contractBytes.withUnsafeBufferPointer { contractBp -> PlatformWalletFFIResult in
                        documentType.withCString { typePtr in
                            propertiesJSON.withCString { propsPtr in
                                documentIdBytes.withUnsafeMutableBufferPointer { outBp in
                                    platform_wallet_create_document_with_signer(
                                        handle,
                                        ownerBp.baseAddress!,
                                        contractBp.baseAddress!,
                                        typePtr,
                                        propsPtr,
                                        signerHandle,
                                        outBp.baseAddress!,
                                        &documentJsonPtr
                                    )
                                }
                            }
                        }
                    }
                }
            }

            try result.check()
            // Take ownership of the JSON and release the Rust allocation.
            defer { if let p = documentJsonPtr { platform_wallet_string_free(p) } }
            // On a successful broadcast the Rust side always writes the
            // canonical JSON; a null pointer here is an FFI/ABI contract
            // violation. Fail loudly rather than persist an empty body as
            // if it were the canonical document.
            guard let jsonPtr = documentJsonPtr else {
                throw PlatformWalletError.walletOperation(
                    "create_document_with_signer returned no canonical document JSON"
                )
            }
            let canonicalJSON = String(cString: jsonPtr)
            return (Data(documentIdBytes), canonicalJSON)
        }.value
    }

    /// Replace + broadcast `documentId`'s properties on `contractId`'s
    /// `documentType`, owned by `ownerIdentityId`, signed with the
    /// explicit AUTHENTICATION + ECDSA key `signingKeyId`. Returns the
    /// 32-byte document id and the confirmed document's canonical
    /// query-side JSON (now at the bumped revision) once Platform
    /// confirms the transition.
    ///
    /// Sibling to `createDocument`. Routes through
    /// `IdentityWallet::replace_document_with_signer` (via
    /// `platform_wallet_document_replace`): the Rust side fetches the
    /// current document, applies `propertiesJSON` (the full replacement
    /// property object, same hex/base58 encoding rules as create),
    /// bumps the revision, validates `signingKeyId` is an
    /// AUTHENTICATION + ECDSA key on the owner, broadcasts on the
    /// platform-wallet 8 MB worker stack, and waits for confirmation.
    /// The signing key never crosses into Swift logic — the
    /// `KeychainSigner` trampoline services the signature on demand.
    /// Callers persist the returned JSON verbatim so the local cache
    /// matches the on-chain document.
    ///
    /// Lifetime contract: identical to `createDocument` — the `signer`
    /// is pinned with `withExtendedLifetime` across the synchronous FFI
    /// call (Rust holds a `passUnretained` ctx pointer to the
    /// underlying `KeychainSigner`).
    public func replaceDocument(
        ownerIdentityId: Identifier,
        contractId: Identifier,
        documentType: String,
        documentId: Identifier,
        propertiesJSON: String,
        signingKeyId: UInt32,
        signer: KeychainSigner
    ) async throws -> (Identifier, String) {
        let handle = self.handle
        let signerHandle = signer.handle
        let ownerBytes: [UInt8] = ownerIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let contractBytes: [UInt8] = contractId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let documentBytes: [UInt8] = documentId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        return try await Task.detached(priority: .userInitiated) {
            var documentIdBytes = [UInt8](repeating: 0, count: 32)
            var documentJsonPtr: UnsafeMutablePointer<CChar>? = nil

            // Pin `signer` for the whole FFI call (see `createDocument`
            // for why a bare `_ = signer` is unreliable under -O).
            let result = withExtendedLifetime(signer) {
                ownerBytes.withUnsafeBufferPointer { ownerBp -> PlatformWalletFFIResult in
                    contractBytes.withUnsafeBufferPointer { contractBp -> PlatformWalletFFIResult in
                        documentType.withCString { typePtr in
                            documentBytes.withUnsafeBufferPointer { docBp -> PlatformWalletFFIResult in
                                propertiesJSON.withCString { propsPtr in
                                    documentIdBytes.withUnsafeMutableBufferPointer { outBp in
                                        platform_wallet_document_replace(
                                            handle,
                                            ownerBp.baseAddress!,
                                            contractBp.baseAddress!,
                                            typePtr,
                                            docBp.baseAddress!,
                                            propsPtr,
                                            signingKeyId,
                                            signerHandle,
                                            outBp.baseAddress!,
                                            &documentJsonPtr
                                        )
                                    }
                                }
                            }
                        }
                    }
                }
            }

            try result.check()
            defer { if let p = documentJsonPtr { platform_wallet_string_free(p) } }
            guard let jsonPtr = documentJsonPtr else {
                throw PlatformWalletError.walletOperation(
                    "document_replace returned no canonical document JSON"
                )
            }
            let canonicalJSON = String(cString: jsonPtr)
            return (Data(documentIdBytes), canonicalJSON)
        }.value
    }

    /// Delete + broadcast `documentId` on `contractId`'s `documentType`,
    /// owned by `ownerIdentityId`, signed with the explicit
    /// AUTHENTICATION + ECDSA key `signingKeyId`. Returns the deleted
    /// document's 32-byte id once Platform confirms the transition.
    ///
    /// Sibling to `createDocument`. Routes through
    /// `IdentityWallet::delete_document_with_signer` (via
    /// `platform_wallet_document_delete`). Delete returns no document
    /// body, so there is no canonical JSON — callers remove the local
    /// `PersistentDocument` row by id.
    public func deleteDocument(
        ownerIdentityId: Identifier,
        contractId: Identifier,
        documentType: String,
        documentId: Identifier,
        signingKeyId: UInt32,
        signer: KeychainSigner
    ) async throws -> Identifier {
        let handle = self.handle
        let signerHandle = signer.handle
        let ownerBytes: [UInt8] = ownerIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let contractBytes: [UInt8] = contractId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let documentBytes: [UInt8] = documentId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        return try await Task.detached(priority: .userInitiated) {
            var documentIdBytes = [UInt8](repeating: 0, count: 32)

            let result = withExtendedLifetime(signer) {
                ownerBytes.withUnsafeBufferPointer { ownerBp -> PlatformWalletFFIResult in
                    contractBytes.withUnsafeBufferPointer { contractBp -> PlatformWalletFFIResult in
                        documentType.withCString { typePtr in
                            documentBytes.withUnsafeBufferPointer { docBp -> PlatformWalletFFIResult in
                                documentIdBytes.withUnsafeMutableBufferPointer { outBp in
                                    platform_wallet_document_delete(
                                        handle,
                                        ownerBp.baseAddress!,
                                        contractBp.baseAddress!,
                                        typePtr,
                                        docBp.baseAddress!,
                                        signingKeyId,
                                        signerHandle,
                                        outBp.baseAddress!
                                    )
                                }
                            }
                        }
                    }
                }
            }

            try result.check()
            return Data(documentIdBytes)
        }.value
    }

    /// Transfer + broadcast `documentId` on `contractId`'s
    /// `documentType`, from `ownerIdentityId` to `recipientId`, signed
    /// with the explicit AUTHENTICATION + ECDSA key `signingKeyId`.
    /// Returns the 32-byte document id and the confirmed document's
    /// canonical JSON (now reflecting the new owner) once Platform
    /// confirms the transition.
    ///
    /// Sibling to `createDocument`. Routes through
    /// `IdentityWallet::transfer_document_with_signer` (via
    /// `platform_wallet_document_transfer`). Only valid for a document
    /// type whose schema marks it `transferable`; the caller gates the
    /// action against that flag.
    public func transferDocument(
        ownerIdentityId: Identifier,
        contractId: Identifier,
        documentType: String,
        documentId: Identifier,
        recipientId: Identifier,
        signingKeyId: UInt32,
        signer: KeychainSigner
    ) async throws -> (Identifier, String) {
        let handle = self.handle
        let signerHandle = signer.handle
        let ownerBytes: [UInt8] = ownerIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let contractBytes: [UInt8] = contractId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let documentBytes: [UInt8] = documentId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let recipientBytes: [UInt8] = recipientId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        return try await Task.detached(priority: .userInitiated) {
            var documentIdBytes = [UInt8](repeating: 0, count: 32)
            var documentJsonPtr: UnsafeMutablePointer<CChar>? = nil

            let result = withExtendedLifetime(signer) {
                ownerBytes.withUnsafeBufferPointer { ownerBp -> PlatformWalletFFIResult in
                    contractBytes.withUnsafeBufferPointer { contractBp -> PlatformWalletFFIResult in
                        documentType.withCString { typePtr in
                            documentBytes.withUnsafeBufferPointer { docBp -> PlatformWalletFFIResult in
                                recipientBytes.withUnsafeBufferPointer { recipBp -> PlatformWalletFFIResult in
                                    documentIdBytes.withUnsafeMutableBufferPointer { outBp in
                                        platform_wallet_document_transfer(
                                            handle,
                                            ownerBp.baseAddress!,
                                            contractBp.baseAddress!,
                                            typePtr,
                                            docBp.baseAddress!,
                                            recipBp.baseAddress!,
                                            signingKeyId,
                                            signerHandle,
                                            outBp.baseAddress!,
                                            &documentJsonPtr
                                        )
                                    }
                                }
                            }
                        }
                    }
                }
            }

            try result.check()
            defer { if let p = documentJsonPtr { platform_wallet_string_free(p) } }
            guard let jsonPtr = documentJsonPtr else {
                throw PlatformWalletError.walletOperation(
                    "document_transfer returned no canonical document JSON"
                )
            }
            let canonicalJSON = String(cString: jsonPtr)
            return (Data(documentIdBytes), canonicalJSON)
        }.value
    }

    /// Set (update) the trade price of `documentId` on `contractId`'s
    /// `documentType` to `price` credits, owned by `ownerIdentityId`,
    /// signed with the explicit AUTHENTICATION + ECDSA key
    /// `signingKeyId`. Returns the 32-byte document id and the confirmed
    /// document's canonical JSON (now carrying `$price`) once Platform
    /// confirms the transition.
    ///
    /// Sibling to `createDocument`. Routes through
    /// `IdentityWallet::set_document_price_with_signer` (via
    /// `platform_wallet_document_set_price`). Only valid for a document
    /// type whose schema enables a trade mode; the caller gates the
    /// action against that flag.
    public func setDocumentPrice(
        ownerIdentityId: Identifier,
        contractId: Identifier,
        documentType: String,
        documentId: Identifier,
        price: UInt64,
        signingKeyId: UInt32,
        signer: KeychainSigner
    ) async throws -> (Identifier, String) {
        let handle = self.handle
        let signerHandle = signer.handle
        let ownerBytes: [UInt8] = ownerIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let contractBytes: [UInt8] = contractId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let documentBytes: [UInt8] = documentId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        return try await Task.detached(priority: .userInitiated) {
            var documentIdBytes = [UInt8](repeating: 0, count: 32)
            var documentJsonPtr: UnsafeMutablePointer<CChar>? = nil

            let result = withExtendedLifetime(signer) {
                ownerBytes.withUnsafeBufferPointer { ownerBp -> PlatformWalletFFIResult in
                    contractBytes.withUnsafeBufferPointer { contractBp -> PlatformWalletFFIResult in
                        documentType.withCString { typePtr in
                            documentBytes.withUnsafeBufferPointer { docBp -> PlatformWalletFFIResult in
                                documentIdBytes.withUnsafeMutableBufferPointer { outBp in
                                    platform_wallet_document_set_price(
                                        handle,
                                        ownerBp.baseAddress!,
                                        contractBp.baseAddress!,
                                        typePtr,
                                        docBp.baseAddress!,
                                        price,
                                        signingKeyId,
                                        signerHandle,
                                        outBp.baseAddress!,
                                        &documentJsonPtr
                                    )
                                }
                            }
                        }
                    }
                }
            }

            try result.check()
            defer { if let p = documentJsonPtr { platform_wallet_string_free(p) } }
            guard let jsonPtr = documentJsonPtr else {
                throw PlatformWalletError.walletOperation(
                    "document_set_price returned no canonical document JSON"
                )
            }
            let canonicalJSON = String(cString: jsonPtr)
            return (Data(documentIdBytes), canonicalJSON)
        }.value
    }

    /// Purchase + broadcast for-sale `documentId` on `contractId`'s
    /// `documentType` for `price` credits, with `purchaserId` as the
    /// buyer (and new owner), signed with the explicit AUTHENTICATION +
    /// ECDSA key `signingKeyId` resolved on the purchaser. Returns the
    /// 32-byte document id and the confirmed document's canonical JSON
    /// (now owned by the purchaser) once Platform confirms the
    /// transition.
    ///
    /// Sibling to `createDocument`. Routes through
    /// `IdentityWallet::purchase_document_with_signer` (via
    /// `platform_wallet_document_purchase`). Consensus rejects a
    /// purchase where the buyer is the current owner — the caller's UI
    /// gates against that self-buy case.
    public func purchaseDocument(
        purchaserId: Identifier,
        contractId: Identifier,
        documentType: String,
        documentId: Identifier,
        price: UInt64,
        signingKeyId: UInt32,
        signer: KeychainSigner
    ) async throws -> (Identifier, String) {
        let handle = self.handle
        let signerHandle = signer.handle
        let purchaserBytes: [UInt8] = purchaserId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let contractBytes: [UInt8] = contractId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let documentBytes: [UInt8] = documentId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        return try await Task.detached(priority: .userInitiated) {
            var documentIdBytes = [UInt8](repeating: 0, count: 32)
            var documentJsonPtr: UnsafeMutablePointer<CChar>? = nil

            let result = withExtendedLifetime(signer) {
                purchaserBytes.withUnsafeBufferPointer { purchaserBp -> PlatformWalletFFIResult in
                    contractBytes.withUnsafeBufferPointer { contractBp -> PlatformWalletFFIResult in
                        documentType.withCString { typePtr in
                            documentBytes.withUnsafeBufferPointer { docBp -> PlatformWalletFFIResult in
                                documentIdBytes.withUnsafeMutableBufferPointer { outBp in
                                    platform_wallet_document_purchase(
                                        handle,
                                        purchaserBp.baseAddress!,
                                        contractBp.baseAddress!,
                                        typePtr,
                                        docBp.baseAddress!,
                                        price,
                                        signingKeyId,
                                        signerHandle,
                                        outBp.baseAddress!,
                                        &documentJsonPtr
                                    )
                                }
                            }
                        }
                    }
                }
            }

            try result.check()
            defer { if let p = documentJsonPtr { platform_wallet_string_free(p) } }
            guard let jsonPtr = documentJsonPtr else {
                throw PlatformWalletError.walletOperation(
                    "document_purchase returned no canonical document JSON"
                )
            }
            let canonicalJSON = String(cString: jsonPtr)
            return (Data(documentIdBytes), canonicalJSON)
        }.value
    }

    /// Run `body` with a NUL-terminated C string for `value`, or
    /// `nil` when `value` is nil. Mirrors the `withCString`
    /// pattern but terminates the chain when the optional is
    /// absent so the FFI receives a NULL pointer.
    fileprivate static func withOptionalCString<R>(
        _ value: String?,
        _ body: (UnsafePointer<CChar>?) -> R
    ) -> R {
        if let value = value {
            return value.withCString { body($0) }
        }
        return body(nil)
    }

    /// Register a new asset-lock-funded identity using an external
    /// `KeychainSigner`. Asset-lock proof is built Rust-side from
    /// `amountDuffs` (wallet must have spendable Core UTXOs).
    ///
    /// `accountIndex` selects which BIP44 *standard* account (by
    /// BIP44 account index) supplies the funding UTXOs. Only BIP44
    /// standard accounts are supported today; the caller is
    /// responsible for filtering its account picker accordingly —
    /// CoinJoin / BIP32 funding for new-identity registration is not
    /// yet wired through `create_funded_asset_lock_proof` on the Rust
    /// side.
    ///
    /// Caller MUST pre-derive `identityPubkeys` (typically via
    /// `dash_sdk_derive_identity_keys_from_mnemonic`) AND pre-persist
    /// each key's private material to the Keychain using
    /// `prePersistIdentityKeysForRegistration` BEFORE calling this —
    /// otherwise the IdentityCreate signature can't complete.
    ///
    /// Returns `(identityId, ManagedIdentity)` for the freshly
    /// registered identity.
    public func registerIdentityWithFunding(
        amountDuffs: UInt64,
        accountIndex: UInt32,
        identityIndex: UInt32,
        identityPubkeys: [ManagedPlatformWallet.IdentityPubkey],
        signer: KeychainSigner
    ) async throws -> (Identifier, ManagedIdentity) {
        guard !identityPubkeys.isEmpty else {
            throw PlatformWalletError.invalidParameter("identityPubkeys is empty")
        }
        let handle = self.handle
        let signerHandle = signer.handle
        let pubkeys = identityPubkeys
        // Create a `MnemonicResolver` owned for the lifetime of the
        // FFI call — Rust constructs a `MnemonicResolverCoreSigner`
        // from this handle to sign the asset-lock proof's
        // credit-spending signature on the IdentityCreate transition.
        // The resolver's vtable callback fetches the mnemonic from
        // Keychain, derives the priv key at the credit-output path,
        // signs the digest, and zeroes — atomic per call. No priv
        // key ever lives in Rust memory across operations.
        let coreSigner = MnemonicResolver()
        return try await Task.detached(priority: .userInitiated) {
            () -> (Identifier, ManagedIdentity) in
            var idTuple: (
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
            ) = (
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            )
            var outManagedHandle: Handle = NULL_HANDLE
            // Pin each pubkey buffer simultaneously via the existing
            // helper, then hand the assembled row array to the FFI.
            //
            // `withExtendedLifetime` is the canonical Swift idiom for
            // "keep this ARC-managed object alive across an FFI call
            // that captures a raw handle to it". `_ = signer` /
            // `_ = coreSigner` is folklore; the optimizer may elide
            // the discard in -O builds, releasing the resolver mid-
            // FFI-call → use-after-free in the vtable callback.
            let pubkeyBuffers: [Data] = pubkeys.map { $0.pubkeyBytes }
            let result = withExtendedLifetime((signer, coreSigner)) {
                ManagedPlatformWallet.withPubkeyFFIArray(
                    pubkeys,
                    buffers: pubkeyBuffers
                ) { ffiRowsPtr, ffiRowsCount in
                    platform_wallet_register_identity_with_funding_signer(
                        handle,
                        amountDuffs,
                        accountIndex,
                        identityIndex,
                        ffiRowsPtr,
                        UInt(ffiRowsCount),
                        signerHandle,
                        coreSigner.handle,
                        &idTuple,
                        &outManagedHandle
                    )
                }
            }
            try result.check()
            // Defend against an FFI contract violation: on Success
            // `outManagedHandle` must be non-NULL. Wrapping
            // `NULL_HANDLE` would push the failure to a later, harder-
            // to-debug point (`ManagedIdentity.deinit` calling
            // `managed_identity_destroy(NULL)` or any downstream FFI
            // accessor crashing on a NULL slot).
            guard outManagedHandle != NULL_HANDLE else {
                throw PlatformWalletError.walletOperation(
                    "FFI returned success but managed-identity handle was NULL"
                )
            }
            // Copy the 32-byte tuple into a Data via withUnsafeBytes.
            let identityId = Swift.withUnsafeBytes(of: idTuple) { Data($0) }
            return (identityId, ManagedIdentity(handle: outManagedHandle))
        }.value
    }

    /// Resume identity registration from an existing tracked asset lock.
    ///
    /// Sibling to
    /// [`registerIdentityWithFunding(amountDuffs:identityIndex:identityPubkeys:signer:)`]:
    /// the wallet-balance variant builds a fresh asset-lock transaction;
    /// this variant picks up a lock that's already tracked (status
    /// `InstantSendLocked` / `ChainLocked`) and drives whatever stages
    /// remain. Use case is crash recovery — a prior attempt left the
    /// lock in storage but the IdentityCreate transition never landed,
    /// and the user picks the lock from the
    /// "Fund from unused Asset Lock" surface in `CreateIdentityView`.
    ///
    /// `outPointTxid` is the 32-byte raw txid (little-endian wire order,
    /// same shape as `OutPointFFI.txid` on the Rust side and what
    /// `PersistentAssetLock.outPointHex` reverses for display); the
    /// caller is responsible for decoding back from the display-order
    /// hex before passing in.
    ///
    /// Caller MUST pre-derive `identityPubkeys` (typically via
    /// `dash_sdk_derive_identity_keys_from_mnemonic`) AND pre-persist
    /// each key's private material to the Keychain using
    /// `prePersistIdentityKeysForRegistration` BEFORE calling this —
    /// same precondition as `registerIdentityWithFunding`.
    ///
    /// Returns `(identityId, ManagedIdentity)` for the freshly
    /// registered identity.
    ///
    /// `consumeInvitationVoucher` is the explicit authorization to consume an
    /// `IdentityInvitation`-typed lock (a DashPay bearer voucher whose key is
    /// shared in the invitation link). Defaults to `false`: generic resume
    /// surfaces are refused invitation locks by the Rust funding resolver.
    /// Only the invitation reclaim flow passes `true`.
    public func resumeIdentityWithAssetLock(
        outPointTxid: Data,
        outPointVout: UInt32,
        identityIndex: UInt32,
        identityPubkeys: [ManagedPlatformWallet.IdentityPubkey],
        signer: KeychainSigner,
        consumeInvitationVoucher: Bool = false
    ) async throws -> (Identifier, ManagedIdentity) {
        guard outPointTxid.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "outPointTxid must be exactly 32 bytes (was \(outPointTxid.count))"
            )
        }
        guard !identityPubkeys.isEmpty else {
            throw PlatformWalletError.invalidParameter("identityPubkeys is empty")
        }
        let handle = self.handle
        let signerHandle = signer.handle
        let pubkeys = identityPubkeys
        // Same `MnemonicResolver` lifetime + vtable rationale as
        // `registerIdentityWithFunding` — the credit-output private key
        // is fetched per-call from Keychain, signed, zeroed; no priv
        // key ever lives in Rust memory across operations.
        let coreSigner = MnemonicResolver()
        return try await Task.detached(priority: .userInitiated) {
            () -> (Identifier, ManagedIdentity) in
            var idTuple: (
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
            ) = (
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            )
            var outManagedHandle: Handle = NULL_HANDLE
            var txidTuple: (
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
            ) = (
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            )
            outPointTxid.withUnsafeBytes { src in
                Swift.withUnsafeMutableBytes(of: &txidTuple) { dst in
                    dst.copyMemory(from: src)
                }
            }
            var outPoint = OutPointFFI(txid: txidTuple, vout: outPointVout)
            let pubkeyBuffers: [Data] = pubkeys.map { $0.pubkeyBytes }
            // `withExtendedLifetime` pins `signer` and `coreSigner`
            // through the closure body. The FFI call inside is
            // synchronous (Rust uses `block_on_worker` under the
            // hood), so the closure returns before the lifetime
            // wrapper exits — invariant holds. If anyone refactors
            // this to spawn an unawaited Task inside, the resolver
            // could be dropped mid-flight and Rust would see a
            // dangling pointer; keep the FFI call inline.
            let result = withExtendedLifetime((signer, coreSigner)) {
                ManagedPlatformWallet.withPubkeyFFIArray(
                    pubkeys,
                    buffers: pubkeyBuffers
                ) { ffiRowsPtr, ffiRowsCount in
                    platform_wallet_resume_identity_with_existing_asset_lock_signer(
                        handle,
                        &outPoint,
                        identityIndex,
                        ffiRowsPtr,
                        UInt(ffiRowsCount),
                        signerHandle,
                        coreSigner.handle,
                        consumeInvitationVoucher,
                        &idTuple,
                        &outManagedHandle
                    )
                }
            }
            try result.check()
            // FFI contract: on Success `outManagedHandle` is non-NULL.
            // Same defense as `registerIdentityWithFunding`.
            guard outManagedHandle != NULL_HANDLE else {
                throw PlatformWalletError.walletOperation(
                    "FFI returned success but managed-identity handle was NULL"
                )
            }
            let identityId = Swift.withUnsafeBytes(of: idTuple) { Data($0) }
            return (identityId, ManagedIdentity(handle: outManagedHandle))
        }.value
    }

    /// Top up an existing identity by building and broadcasting a **new
    /// Core asset lock** from the wallet's own balance — the top-up twin of
    /// [`registerIdentityWithFunding(amountDuffs:accountIndex:identityIndex:identityPubkeys:signer:)`].
    ///
    /// Simpler than registration: an `IdentityTopUp` creates no identity
    /// keys, so there is no per-identity-key `KeychainSigner` and no pubkey
    /// array — the transition is signed entirely by the asset lock's
    /// Core-side key via a `MnemonicResolver`. `accountIndex` selects which
    /// BIP44 *standard* account supplies the funding UTXOs (same constraint
    /// as registration).
    ///
    /// `amountDuffs` must meet the Rust-side minimum top-up asset-lock
    /// balance; a smaller amount is rejected before any lock is broadcast
    /// (callers should also gate on the minimum in the UI so a sub-floor
    /// amount never reaches here). Returns the identity's post-transition
    /// credit balance; the local `ManagedIdentity` balance is updated inside
    /// the FFI call.
    public func topUpIdentityWithFunding(
        identityId: Data,
        amountDuffs: UInt64,
        accountIndex: UInt32
    ) async throws -> UInt64 {
        guard identityId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "identityId must be 32 bytes, got \(identityId.count)"
            )
        }
        let handle = self.handle
        // Core-side asset-lock signer. Same `MnemonicResolver` lifetime +
        // vtable rationale as `registerIdentityWithFunding`: the
        // credit-output private key is fetched per-call from Keychain,
        // signed, and zeroed — no private key ever lives in Rust memory
        // across operations.
        let coreSigner = MnemonicResolver()
        return try await Task.detached(priority: .userInitiated) { () -> UInt64 in
            var idTuple: (
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
            ) = (
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            )
            withUnsafeMutableBytes(of: &idTuple) { raw in
                for (i, byte) in identityId.prefix(32).enumerated() {
                    raw[i] = byte
                }
            }

            var newBalance: UInt64 = 0
            // `withExtendedLifetime` pins `coreSigner` across the
            // synchronous FFI call (Rust uses `block_on_worker`). Keep the
            // call inline — an unawaited Task inside would let the resolver
            // drop mid-flight and dangle its trampoline ctx pointer.
            let result = withExtendedLifetime(coreSigner) {
                withUnsafePointer(to: &idTuple) { idPtr in
                    platform_wallet_top_up_identity_with_funding_signer(
                        handle,
                        idPtr,
                        amountDuffs,
                        accountIndex,
                        coreSigner.handle,
                        &newBalance
                    )
                }
            }
            try result.check()
            return newBalance
        }.value
    }

    /// Recover a stuck top-up by consuming an already-tracked Core asset
    /// lock — the top-up twin of
    /// [`resumeIdentityWithAssetLock(outPointTxid:outPointVout:identityIndex:identityPubkeys:signer:)`].
    ///
    /// Use case is crash recovery: a prior `topUpIdentityWithFunding`
    /// confirmed its lock on Core but the `IdentityTopUp` never reached
    /// Platform (app killed / network drop). This picks up that lock by
    /// outpoint and completes the top-up against `identityId`. It is also
    /// the DashPay invitation "reclaim into an existing identity" path —
    /// see `consumeInvitationVoucher`.
    ///
    /// `outPointTxid` is the 32-byte raw txid (little-endian wire order,
    /// same shape as `OutPointFFI.txid`; the caller decodes from
    /// display-order hex first). Returns the post-transition credit balance.
    ///
    /// If the lock was already consumed on Platform (double-resume), the FFI
    /// surfaces an opaque consensus rejection — the caller should classify
    /// and message it ("asset lock already consumed") rather than showing
    /// the raw error.
    ///
    /// `consumeInvitationVoucher` is the explicit authorization to consume an
    /// `IdentityInvitation`-typed lock (a DashPay bearer voucher whose key is
    /// shared in the invitation link). Defaults to `false`: generic top-up
    /// crash-recovery surfaces are refused invitation locks by the Rust
    /// funding resolver. Only the invitation reclaim flow passes `true`.
    public func resumeTopUpWithAssetLock(
        identityId: Data,
        outPointTxid: Data,
        outPointVout: UInt32,
        consumeInvitationVoucher: Bool = false
    ) async throws -> UInt64 {
        guard identityId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "identityId must be 32 bytes, got \(identityId.count)"
            )
        }
        guard outPointTxid.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "outPointTxid must be exactly 32 bytes (was \(outPointTxid.count))"
            )
        }
        let handle = self.handle
        // Same `MnemonicResolver` rationale as `topUpIdentityWithFunding`.
        let coreSigner = MnemonicResolver()
        return try await Task.detached(priority: .userInitiated) { () -> UInt64 in
            var idTuple: (
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
            ) = (
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            )
            withUnsafeMutableBytes(of: &idTuple) { raw in
                for (i, byte) in identityId.prefix(32).enumerated() {
                    raw[i] = byte
                }
            }
            var txidTuple: (
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
            ) = (
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            )
            outPointTxid.withUnsafeBytes { src in
                Swift.withUnsafeMutableBytes(of: &txidTuple) { dst in
                    dst.copyMemory(from: src)
                }
            }
            var outPoint = OutPointFFI(txid: txidTuple, vout: outPointVout)

            var newBalance: UInt64 = 0
            let result = withExtendedLifetime(coreSigner) {
                withUnsafePointer(to: &idTuple) { idPtr in
                    platform_wallet_topup_identity_with_existing_asset_lock_signer(
                        handle,
                        &outPoint,
                        idPtr,
                        coreSigner.handle,
                        consumeInvitationVoucher,
                        &newBalance
                    )
                }
            }
            try result.check()
            return newBalance
        }.value
    }
}
