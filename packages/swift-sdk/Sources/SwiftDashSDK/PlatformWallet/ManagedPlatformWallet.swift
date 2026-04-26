import Foundation

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
        var error = PlatformWalletFFIError()
        _ = platform_wallet_destroy(handle, &error)
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
        var error = PlatformWalletFFIError()

        let result = platform_wallet_get_balance(
            handle,
            &spendable,
            &unconfirmed,
            &immature,
            &locked,
            &error
        )

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

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
        var error = PlatformWalletFFIError()

        let result = platform_wallet_get_platform(handle, &platformHandle, &error)

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return ManagedPlatformAddressWallet(handle: platformHandle)
    }

    /// Get the core wallet for UTXO management, addresses, and transactions.
    public func coreWallet() throws -> ManagedCoreWallet {
        var coreHandle: Handle = NULL_HANDLE
        var error = PlatformWalletFFIError()

        let result = platform_wallet_get_core(handle, &coreHandle, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return ManagedCoreWallet(handle: coreHandle)
    }

    /// Get the asset lock manager for building and tracking asset locks.
    public func assetLockManager() throws -> ManagedAssetLockManager {
        var assetLockHandle: Handle = NULL_HANDLE
        var error = PlatformWalletFFIError()

        let result = platform_wallet_get_asset_locks(handle, &assetLockHandle, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return ManagedAssetLockManager(handle: assetLockHandle)
    }

    // MARK: - Persistence

    /// Flush all queued changesets to the storage backend.
    public func flushPersist() throws {
        var error = PlatformWalletFFIError()
        let result = platform_wallet_flush_persist(handle, &error)

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    /// Load persisted state and apply it to the in-memory wallet.
    public func loadAndApplyPersisted() throws {
        var error = PlatformWalletFFIError()
        let result = platform_wallet_load_and_apply_persisted(handle, &error)

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
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

    /// Register a new identity funded by Platform-address balances.
    ///
    /// The Rust side runs `IdentityWallet::register_from_addresses`
    /// which derives DIP-9 authentication keys under
    /// `m/9'/…/identityIndex'/key_index'`, builds an identity, and
    /// submits via `IdentityCreateFromAddressesTransition`. The
    /// call blocks until Platform confirms the transition, which can
    /// take several seconds — it's expected to be driven from an
    /// `async` context.
    ///
    /// - Parameters:
    ///   - inputs: Contributing addresses with the credit amount to
    ///     spend from each. Nonces are resolved by the SDK at submit
    ///     time — the caller does not track them.
    ///   - output: Optional refund address + credits. `nil` when any
    ///     residual should go into the new identity.
    ///   - identityIndex: BIP-9 identity index in the HD tree.
    ///   - keyCount: Number of authentication keys to register
    ///     (must be ≥ 1). First key is MASTER, the rest HIGH.
    ///   - mnemonic: BIP-39 mnemonic phrase (English). The Rust
    ///     side uses it to derive DIP-9 auth keys + sign both the
    ///     new identity and each spent platform address. Not
    ///     retained beyond this call.
    ///   - passphrase: Optional BIP-39 passphrase. `nil` encodes
    ///     the empty passphrase.
    public func registerIdentityFromAddresses(
        inputs: [IdentityAddressInput],
        output: IdentityAddressOutput?,
        identityIndex: UInt32,
        keyCount: UInt32,
        mnemonic: String,
        passphrase: String? = nil
    ) async throws -> CreatedIdentity {
        guard !inputs.isEmpty else {
            throw PlatformWalletError.invalidParameter
        }
        guard !mnemonic.isEmpty else {
            throw PlatformWalletError.invalidParameter
        }

        // Copy inputs into the flat FFI shape. Do it off the main
        // actor via `Task.detached` because `platform_wallet_register_
        // identity_from_addresses` internally drives a tokio runtime
        // via `block_on`; calling from the main thread would park
        // user-interactive QoS on default-QoS workers (same pattern
        // we use for BLAST sync).
        //
        // The detached task returns primitive `(Data, Handle)` which
        // are `Sendable`. The `ManagedIdentity` wrapper is constructed
        // back in the calling isolation domain so we don't need to
        // add a Sendable bound on the non-sendable FFI wrapper type.
        let handle = self.handle
        let (idData, identityHandle): (Data, Handle) = try await Task.detached(
            priority: .userInitiated
        ) { () -> (Data, Handle) in
            let ffiInputs = inputs.map { input -> IdentityInputAddressFFI in
                var hashTuple = hashTupleInit()
                withUnsafeMutableBytes(of: &hashTuple) { raw in
                    let src = input.hash.prefix(20)
                    for (i, byte) in src.enumerated() {
                        raw[i] = byte
                    }
                }
                return IdentityInputAddressFFI(
                    address_type: input.addressType,
                    hash: hashTuple,
                    credits: input.credits
                )
            }

            // Build a single FFI output struct; we pass it by
            // pointer (`&outputFFI`) below. `has_output=false`
            // tells Rust to ignore the rest of the fields.
            var outputFFI: IdentityOutputAddressFFI = {
                if let output {
                    var hashTuple = hashTupleInit()
                    withUnsafeMutableBytes(of: &hashTuple) { raw in
                        let src = output.hash.prefix(20)
                        for (i, byte) in src.enumerated() {
                            raw[i] = byte
                        }
                    }
                    return IdentityOutputAddressFFI(
                        has_output: true,
                        address_type: output.addressType,
                        hash: hashTuple,
                        credits: output.credits
                    )
                } else {
                    return IdentityOutputAddressFFI(
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
            var error = PlatformWalletFFIError()

            // `withCString` guarantees a null-terminated UTF-8
            // buffer valid for the closure's lifetime. The Rust
            // side copies what it needs and does not retain the
            // pointer.
            let result = mnemonic.withCString { mnemonicPtr -> PlatformWalletFFIResult in
                let call: (UnsafePointer<CChar>?) -> PlatformWalletFFIResult = { passphrasePtr in
                    ffiInputs.withUnsafeBufferPointer { inputsBuf in
                        withUnsafePointer(to: &outputFFI) { outputPtr in
                            platform_wallet_register_identity_from_addresses(
                                handle,
                                identityIndex,
                                keyCount,
                                mnemonicPtr,
                                passphrasePtr,
                                inputsBuf.baseAddress,
                                inputsBuf.count,
                                outputPtr,
                                &outIdentityId,
                                &outIdentityHandle,
                                &error
                            )
                        }
                    }
                }
                if let passphrase {
                    return passphrase.withCString { passphrasePtr in call(passphrasePtr) }
                } else {
                    return call(nil)
                }
            }

            guard result == Success else {
                throw PlatformWalletError(result: result, error: error)
            }
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
}

/// All-zero 20-byte tuple — used as the `hash` field default when
/// building `IdentityInputAddressFFI` / `IdentityOutputAddressFFI`.
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
        /// Compressed public key bytes as lowercase hex (33 bytes →
        /// 66 hex chars).
        public let publicKeyHex: String
        /// Private key in WIF (Wallet Import Format) — network-aware,
        /// compressed. Matches how other views in the example app
        /// accept / display private keys.
        public let privateKeyWIF: String

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
    ///
    /// - Throws: `PlatformWalletError` if the wallet handle is
    ///   invalid or Rust-side derivation fails.
    public func previewIdentityRegistrationKeys(
        startIndex: UInt32 = 0,
        count: UInt32? = nil
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

        var out = identityKeyPreviewsFFIEmpty()
        var error = PlatformWalletFFIError()
        let result = platform_wallet_preview_identity_registration_keys(
            handle,
            startIndex,
            countOrNeg1,
            &out,
            &error
        )
        // Free the Rust-owned array whether we succeeded or bailed
        // out — the free function is a no-op on the zero struct.
        defer { platform_wallet_preview_identity_registration_keys_free(&out) }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        guard let base = out.items, out.count > 0 else {
            return []
        }

        var previews: [IdentityRegistrationKeyPreview] = []
        previews.reserveCapacity(out.count)
        for i in 0..<out.count {
            let row = base[i]

            let path: String = row.derivation_path.map { String(cString: $0) } ?? ""
            let wif: String = row.private_key_wif.map { String(cString: $0) } ?? ""

            let pubHex: String
            if let pubPtr = row.public_key, row.public_key_len > 0 {
                let buf = UnsafeBufferPointer(start: pubPtr, count: row.public_key_len)
                pubHex = buf.map { String(format: "%02x", $0) }.joined()
            } else {
                pubHex = ""
            }

            previews.append(
                IdentityRegistrationKeyPreview(
                    identityIndex: row.identity_index,
                    derivationPath: path,
                    publicKeyHex: pubHex,
                    privateKeyWIF: wif
                )
            )
        }
        return previews
    }

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
    /// - Returns: The identifiers of any identities the scan
    ///   discovered that weren't already in the local manager.
    ///   Identities already tracked are not re-reported.
    public func discoverIdentities(
        startIndex: UInt32? = nil,
        gapLimit: UInt32? = nil
    ) async throws -> [Identifier] {
        let handle = self.handle
        let startArg: Int64 = startIndex.map(Int64.init) ?? -1
        let gapArg: UInt32 = gapLimit ?? 0
        return try await Task.detached(priority: .userInitiated) {
            () -> [Identifier] in
            var found = discoveredIdentityIdsFFIEmpty()
            var error = PlatformWalletFFIError()
            let result = platform_wallet_discover_identities(
                handle,
                startArg,
                gapArg,
                &found,
                &error
            )
            defer { platform_wallet_discover_identities_free(&found) }
            guard result == Success else {
                throw PlatformWalletError(result: result, error: error)
            }
            guard let base = found.ids, found.count > 0 else {
                return []
            }
            var ids: [Identifier] = []
            ids.reserveCapacity(found.count)
            for i in 0..<found.count {
                var tuple = base[i]
                let data = Swift.withUnsafeBytes(of: &tuple) { Data($0) }
                ids.append(data)
            }
            return ids
        }.value
    }
}

// MARK: - DPNS operations

/// Simple search-result struct surfaced by `searchDpnsNames`. Mirrors
/// the Rust `DpnsSearchResultFFI` row shape in a Sendable Swift value.
public struct DpnsSearchResult: Sendable, Equatable {
    public let identityId: Identifier
    public let fullName: String
}

extension ManagedPlatformWallet {
    /// Register a DPNS name for `identityId` on Platform.
    ///
    /// Goes through `IdentityWallet::register_name`, which on success:
    ///   1. broadcasts the DPNS preorder + domain documents
    ///   2. appends the new `DpnsNameInfo` to
    ///      `ManagedIdentity.dpns_names`
    ///   3. queues the updated identity in the persister so the
    ///      SwiftData `PersistentIdentity` row refreshes via the
    ///      `on_persist_identities_fn` callback.
    ///
    /// Returns the full domain name (e.g. `"alice.dash"`).
    @discardableResult
    public func registerDpnsName(
        identityId: Identifier,
        name: String
    ) async throws -> String {
        let handle = self.handle
        // Capture the 32-byte payload by value into a Sendable
        // `[UInt8]` so the detached Task can hand a fresh pointer
        // back to the FFI (the source `Identifier`/`Data` is itself
        // not Sendable across the suspension point).
        let idBytes: [UInt8] = identityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        return try await Task.detached(priority: .userInitiated) { () -> String in
            var outPtr: UnsafeMutablePointer<CChar>? = nil
            var error = PlatformWalletFFIError()
            let result = idBytes.withUnsafeBufferPointer { idBp in
                name.withCString { namePtr in
                    platform_wallet_register_dpns_name(
                        handle,
                        idBp.baseAddress!,
                        namePtr,
                        &outPtr,
                        &error
                    )
                }
            }
            guard result == Success else {
                throw PlatformWalletError(result: result, error: error)
            }
            defer { if let p = outPtr { platform_wallet_string_free(p) } }
            guard let p = outPtr else {
                throw PlatformWalletError.walletOperation(
                    "register_dpns_name returned a null full-domain-name pointer"
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
            var error = PlatformWalletFFIError()
            let result = buf.withUnsafeMutableBufferPointer { bp -> PlatformWalletFFIResult in
                name.withCString { namePtr in
                    platform_wallet_resolve_dpns_name(
                        handle,
                        namePtr,
                        bp.baseAddress!,
                        &found,
                        &error
                    )
                }
            }
            guard result == Success else {
                throw PlatformWalletError(result: result, error: error)
            }
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
            var outCount: Int = 0
            var error = PlatformWalletFFIError()
            let result = prefix.withCString { prefixPtr in
                platform_wallet_search_dpns_names(
                    handle,
                    prefixPtr,
                    limit,
                    &outPtr,
                    &outCount,
                    &error
                )
            }
            guard result == Success else {
                throw PlatformWalletError(result: result, error: error)
            }
            guard let ptr = outPtr, outCount > 0 else {
                return []
            }
            defer { dpns_search_results_free(ptr, outCount) }
            var results: [DpnsSearchResult] = []
            results.reserveCapacity(outCount)
            for i in 0..<outCount {
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
            var error = PlatformWalletFFIError()
            let result = idBytes.withUnsafeBufferPointer { bp in
                platform_wallet_sync_dpns_names(handle, bp.baseAddress!, &added, &error)
            }
            guard result == Success else {
                throw PlatformWalletError(result: result, error: error)
            }
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
            var state = contestVoteStateFFIEmpty()
            var found = false
            var error = PlatformWalletFFIError()
            let result = idBytes.withUnsafeBufferPointer { idBp -> PlatformWalletFFIResult in
                label.withCString { labelPtr in
                    platform_wallet_fetch_contest_vote_state(
                        handle,
                        idBp.baseAddress!,
                        labelPtr,
                        &state,
                        &found,
                        &error
                    )
                }
            }
            defer { contest_vote_state_ffi_free(&state) }
            guard result == Success else {
                throw PlatformWalletError(result: result, error: error)
            }
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
            var error = PlatformWalletFFIError()
            let result = idBytes.withUnsafeBufferPointer { bp in
                platform_wallet_sync_contested_dpns_names(
                    handle,
                    bp.baseAddress!,
                    &count,
                    &error
                )
            }
            guard result == Success else {
                throw PlatformWalletError(result: result, error: error)
            }
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
        var error = PlatformWalletFFIError()
        let result = identityId.withFFIBytes { idPtr in
            platform_wallet_get_managed_identity(
                handle,
                idPtr,
                &outHandle,
                &error
            )
        }
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
        return ManagedIdentity(handle: outHandle)
    }

    /// Send a contact request to `recipientIdentityId` owned by
    /// `senderIdentityId`.
    ///
    /// Routes through `IdentityWallet::send_contact_request`, which
    /// resolves signing keys internally, broadcasts the DashPay
    /// contactRequest document, and adds the result to
    /// `ManagedIdentity.sent_contact_requests` via the persister.
    /// Swift persister callback (5f5ac06d6) forwards the identity
    /// update to SwiftData.
    ///
    /// Returns a fresh `ContactRequest` wrapper owning a handle into
    /// the Rust-side `CONTACT_REQUEST_STORAGE`.
    public func sendContactRequest(
        senderIdentityId: Identifier,
        recipientIdentityId: Identifier,
        accountLabel: String? = nil,
        autoAcceptProof: Data? = nil
    ) async throws -> ContactRequest {
        let handle = self.handle
        // Snapshot identifiers as Sendable byte arrays before the
        // Task.detached suspension point — Identifier (= Data) is
        // not Sendable across boundaries by itself.
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
            var error = PlatformWalletFFIError()

            // Nest the buffer pointers + optional CString + optional
            // proof bytes so every FFI argument stays live across
            // the call window.
            let result: PlatformWalletFFIResult = senderBytes.withUnsafeBufferPointer {
                senderBp -> PlatformWalletFFIResult in
                recipientBytes.withUnsafeBufferPointer { recipientBp -> PlatformWalletFFIResult in
                    let callWithLabel: (UnsafePointer<CChar>?) -> PlatformWalletFFIResult = {
                        labelPtr in
                        if let autoAcceptProof, !autoAcceptProof.isEmpty {
                            return autoAcceptProof.withUnsafeBytes { rawBuf in
                                let bytesPtr = rawBuf.baseAddress?
                                    .assumingMemoryBound(to: UInt8.self)
                                return platform_wallet_send_contact_request(
                                    handle,
                                    senderBp.baseAddress!,
                                    recipientBp.baseAddress!,
                                    labelPtr,
                                    bytesPtr,
                                    autoAcceptProof.count,
                                    &outHandle,
                                    &error
                                )
                            }
                        } else {
                            return platform_wallet_send_contact_request(
                                handle,
                                senderBp.baseAddress!,
                                recipientBp.baseAddress!,
                                labelPtr,
                                nil,
                                0,
                                &outHandle,
                                &error
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

            guard result == Success else {
                throw PlatformWalletError(result: result, error: error)
            }
            return outHandle
        }.value

        return ContactRequest(handle: requestHandle)
    }

    /// Sync received contact requests for every managed identity on
    /// this wallet from Platform. Returns wrappers for each
    /// newly-discovered request (an empty array when nothing new
    /// arrived).
    @discardableResult
    public func syncContactRequests() async throws -> [ContactRequest] {
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) { () -> [ContactRequest] in
            var array = ContactRequestHandleArray(handles: nil, count: 0)
            var error = PlatformWalletFFIError()
            let result = platform_wallet_sync_contact_requests(handle, &array, &error)
            guard result == Success else {
                throw PlatformWalletError(result: result, error: error)
            }
            defer { platform_wallet_contact_request_handle_array_free(&array) }
            guard let handles = array.handles, array.count > 0 else {
                return []
            }
            var requests: [ContactRequest] = []
            requests.reserveCapacity(array.count)
            for i in 0..<array.count {
                requests.append(ContactRequest(handle: handles[i]))
            }
            return requests
        }.value
    }

    /// Accept an incoming contact request by sending a reciprocal
    /// request. Returns the established contact.
    public func acceptContactRequest(
        _ request: ContactRequest
    ) async throws -> EstablishedContact {
        let walletHandle = self.handle
        let requestHandle = request.handle
        let establishedHandle: Handle = try await Task.detached(
            priority: .userInitiated
        ) { () -> Handle in
            var outHandle: Handle = NULL_HANDLE
            var error = PlatformWalletFFIError()
            let result = platform_wallet_accept_contact_request(
                walletHandle,
                requestHandle,
                &outHandle,
                &error
            )
            guard result == Success else {
                throw PlatformWalletError(result: result, error: error)
            }
            return outHandle
        }.value
        return EstablishedContact(handle: establishedHandle)
    }

    /// Reject an incoming contact request. Today the effect is
    /// local — drops it from `ManagedIdentity.incoming_contact_requests`.
    /// A future follow-up (TODO in the Rust `reject_contact_request`)
    /// will also write a `display_hidden` contactInfo document so
    /// the rejection persists across devices.
    public func rejectContactRequest(
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
            var error = PlatformWalletFFIError()
            let result = ourBytes.withUnsafeBufferPointer {
                ourBp -> PlatformWalletFFIResult in
                contactBytes.withUnsafeBufferPointer { contactBp in
                    platform_wallet_reject_contact_request(
                        handle,
                        ourBp.baseAddress!,
                        contactBp.baseAddress!,
                        &error
                    )
                }
            }
            guard result == Success else {
                throw PlatformWalletError(result: result, error: error)
            }
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
            var error = PlatformWalletFFIError()
            let result = idBytes.withUnsafeBufferPointer { idBp in
                platform_wallet_fetch_sent_contact_requests(
                    handle,
                    idBp.baseAddress!,
                    &array,
                    &error
                )
            }
            guard result == Success else {
                throw PlatformWalletError(result: result, error: error)
            }
            defer { platform_wallet_contact_request_handle_array_free(&array) }
            guard let handles = array.handles, array.count > 0 else {
                return []
            }
            var requests: [ContactRequest] = []
            requests.reserveCapacity(array.count)
            for i in 0..<array.count {
                requests.append(ContactRequest(handle: handles[i]))
            }
            return requests
        }.value
    }

    /// Send a Dash payment to an established DashPay contact.
    /// `amountDuffs` is in duffs (1 DASH = 100_000_000 duffs).
    /// Returns the 32-byte transaction id.
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
    ) async throws -> Data {
        let handle = self.handle
        let fromBytes: [UInt8] = fromIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let toBytes: [UInt8] = toContactIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let memoCopy = memo
        return try await Task.detached(priority: .userInitiated) { () -> Data in
            var txidTuple: (
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
            ) = (
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            )
            var error = PlatformWalletFFIError()
            let result: PlatformWalletFFIResult = fromBytes.withUnsafeBufferPointer {
                fromBp -> PlatformWalletFFIResult in
                toBytes.withUnsafeBufferPointer { toBp -> PlatformWalletFFIResult in
                    let call: (UnsafePointer<CChar>?) -> PlatformWalletFFIResult = { memoPtr in
                        platform_wallet_send_dashpay_payment(
                            handle,
                            fromBp.baseAddress!,
                            toBp.baseAddress!,
                            amountDuffs,
                            memoPtr,
                            &txidTuple,
                            &error
                        )
                    }
                    if let memoCopy {
                        return memoCopy.withCString { call($0) }
                    } else {
                        return call(nil)
                    }
                }
            }
            guard result == Success else {
                throw PlatformWalletError(result: result, error: error)
            }
            return Swift.withUnsafeBytes(of: &txidTuple) { Data($0) }
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
        var ffiProfile = dashPayProfileFFIEmpty()
        var hasProfile: Bool = false
        var error = PlatformWalletFFIError()

        let result = identityId.withFFIBytes { idPtr in
            platform_wallet_get_dashpay_profile(
                handle,
                idPtr,
                &ffiProfile,
                &hasProfile,
                &error
            )
        }
        defer { dashpay_profile_ffi_free(&ffiProfile) }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
        guard hasProfile else { return nil }
        return DashPayProfile(ffi: ffiProfile)
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
            var error = PlatformWalletFFIError()
            let result = platform_wallet_sync_dashpay_profiles(
                handle,
                &syncedCount,
                &error
            )
            guard result == Success else {
                throw PlatformWalletError(result: result, error: error)
            }
            return syncedCount
        }.value
    }

    /// Create a new DashPay profile document on Platform for
    /// `identityId`, then refresh the local cache with the result.
    ///
    /// Fails with `.walletOperation` when a profile already exists —
    /// use `updateDashPayProfile` in that case. All fields in `update`
    /// are optional; `nil` fields are simply omitted from the outgoing
    /// document. `avatarBytes` triggers SHA-256 + dHash computation
    /// on the Rust side.
    @discardableResult
    public func createDashPayProfile(
        identityId: Identifier,
        update: DashPayProfileUpdate
    ) async throws -> DashPayProfile {
        try await submitDashPayProfile(
            identityId: identityId,
            update: update,
            doCreate: true
        )
    }

    /// Update an existing DashPay profile document. Errors with
    /// `.walletOperation` when no profile is on Platform yet — use
    /// `createDashPayProfile` in that case.
    @discardableResult
    public func updateDashPayProfile(
        identityId: Identifier,
        update: DashPayProfileUpdate
    ) async throws -> DashPayProfile {
        try await submitDashPayProfile(
            identityId: identityId,
            update: update,
            doCreate: false
        )
    }

    /// Shared submit path for create / update — same inputs, same
    /// error mapping; only the routed FFI function differs.
    ///
    /// `Task.detached` keeps the tokio-driven blocking call off the
    /// calling (user-interactive) thread, mirroring the pattern used
    /// by `registerIdentityFromAddresses`.
    private func submitDashPayProfile(
        identityId: Identifier,
        update: DashPayProfileUpdate,
        doCreate: Bool
    ) async throws -> DashPayProfile {
        let handle = self.handle
        let idBytes: [UInt8] = identityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let displayName = update.displayName
        let publicMessage = update.publicMessage
        let avatarUrl = update.avatarUrl
        let avatarBytes = update.avatarBytes

        return try await Task.detached(priority: .userInitiated) { () -> DashPayProfile in
            var outProfile = dashPayProfileFFIEmpty()
            var error = PlatformWalletFFIError()

            // Hold the identifier buffer pointer live across the
            // entire optional-CString / optional-avatar-bytes call
            // tree by sinking the FFI invocation inside
            // `withUnsafeBufferPointer`.
            let result: PlatformWalletFFIResult = idBytes.withUnsafeBufferPointer {
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
                            if doCreate {
                                return platform_wallet_create_dashpay_profile(
                                    handle,
                                    idPtr,
                                    namePtr,
                                    msgPtr,
                                    urlPtr,
                                    bytesPtr,
                                    avatarBytes.count,
                                    &outProfile,
                                    &error
                                )
                            } else {
                                return platform_wallet_update_dashpay_profile(
                                    handle,
                                    idPtr,
                                    namePtr,
                                    msgPtr,
                                    urlPtr,
                                    bytesPtr,
                                    avatarBytes.count,
                                    &outProfile,
                                    &error
                                )
                            }
                        }
                    } else {
                        // Referenced only so the compiler keeps the
                        // enclosing `bytes` scope in a consistent type.
                        _ = bytes
                        if doCreate {
                            return platform_wallet_create_dashpay_profile(
                                handle,
                                idPtr,
                                namePtr,
                                msgPtr,
                                urlPtr,
                                nil,
                                0,
                                &outProfile,
                                &error
                            )
                        } else {
                            return platform_wallet_update_dashpay_profile(
                                handle,
                                idPtr,
                                namePtr,
                                msgPtr,
                                urlPtr,
                                nil,
                                0,
                                &outProfile,
                                &error
                            )
                        }
                    }
                }
            }

            defer { dashpay_profile_ffi_free(&outProfile) }

            guard result == Success else {
                throw PlatformWalletError(result: result, error: error)
            }
            return DashPayProfile(ffi: outProfile)
        }.value
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
    /// Number of `(identity_id, token_id) -> amount` rows on
    /// `PlatformWalletInfo.token_balances`.
    public let tokenBalancesCount: Int

    public init(
        identitiesCount: Int,
        watchedCount: Int,
        lastScannedIndex: UInt32,
        primaryIdentityId: Identifier?,
        trackedAssetLocksCount: Int,
        tokenBalancesCount: Int
    ) {
        self.identitiesCount = identitiesCount
        self.watchedCount = watchedCount
        self.lastScannedIndex = lastScannedIndex
        self.primaryIdentityId = primaryIdentityId
        self.trackedAssetLocksCount = trackedAssetLocksCount
        self.tokenBalancesCount = tokenBalancesCount
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
        try readIdentifierArray { array, error in
            platform_wallet_list_in_memory_identity_ids(handle, &array, &error)
        }
    }

    /// List the ids of every watched (read-only / observed) identity
    /// the wallet currently knows about. Reads
    /// `info.identity_manager.watched_identities` directly.
    public func inMemoryWatchedIdentityIds() throws -> [Identifier] {
        try readIdentifierArray { array, error in
            platform_wallet_list_in_memory_watched_identity_ids(handle, &array, &error)
        }
    }

    /// Read a one-shot summary of the wallet's in-memory state — the
    /// counts and watermarks the Wallet Memory Explorer view surfaces
    /// at the top of the per-wallet detail screen.
    public func inMemorySummary() throws -> InMemoryWalletSummary {
        var ffi = platformWalletMemorySummaryFFIEmpty()
        var error = PlatformWalletFFIError()

        let result = platform_wallet_get_in_memory_summary(handle, &ffi, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return InMemoryWalletSummary(
            identitiesCount: ffi.identities_count,
            watchedCount: ffi.watched_count,
            lastScannedIndex: ffi.last_scanned_index,
            // Primary-identity selection no longer lives on the Rust
            // side; UI layer owns it now.
            primaryIdentityId: nil,
            trackedAssetLocksCount: ffi.tracked_asset_locks_count,
            tokenBalancesCount: ffi.token_balances_count
        )
    }

    /// Shared array-marshalling body for the two id-list explorer
    /// readers above. Both wrap the same `IdentifierArray` FFI shape +
    /// free helper, so the per-method variance is just the FFI call
    /// itself.
    private func readIdentifierArray(
        _ fetch: (inout IdentifierArray, inout PlatformWalletFFIError) -> PlatformWalletFFIResult
    ) throws -> [Identifier] {
        var array = IdentifierArray(items: nil, count: 0)
        var error = PlatformWalletFFIError()
        let result = fetch(&array, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
        defer { platform_wallet_identifier_array_free(&array) }
        guard array.items != nil, array.count > 0 else {
            return []
        }
        var ids: [Identifier] = []
        ids.reserveCapacity(array.count)
        for i in 0..<array.count {
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
