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
    public struct CreatedIdentity {
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
            var ffiInputs = inputs.map { input -> IdentityInputAddressFFI in
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
        let ffiId = identifierToFFI(identityId)
        return try await Task.detached(priority: .userInitiated) { () -> String in
            var outPtr: UnsafeMutablePointer<CChar>? = nil
            var error = PlatformWalletFFIError()
            let result = name.withCString { namePtr in
                platform_wallet_register_dpns_name(
                    handle,
                    ffiId,
                    namePtr,
                    &outPtr,
                    &error
                )
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
            var outId = IdentifierBytes(
                bytes: (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
            )
            var found = false
            var error = PlatformWalletFFIError()
            let result = name.withCString { namePtr in
                platform_wallet_resolve_dpns_name(
                    handle,
                    namePtr,
                    &outId,
                    &found,
                    &error
                )
            }
            guard result == Success else {
                throw PlatformWalletError(result: result, error: error)
            }
            guard found else { return nil }
            return identifierFromFFI(outId)
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

// MARK: - DashPay contact requests + payments

extension ManagedPlatformWallet {
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
        let senderFFI = identifierToFFI(senderIdentityId)
        let recipientFFI = identifierToFFI(recipientIdentityId)
        let accountLabel = accountLabel
        let autoAcceptProof = autoAcceptProof

        let requestHandle: Handle = try await Task.detached(priority: .userInitiated) {
            () -> Handle in
            var outHandle: Handle = NULL_HANDLE
            var error = PlatformWalletFFIError()

            // `withCString` is closure-scoped — nest optional proof
            // access inside so both string + byte buffer are live
            // across the FFI call window.
            let result: PlatformWalletFFIResult = {
                let callWithLabel: (UnsafePointer<CChar>?) -> PlatformWalletFFIResult = { labelPtr in
                    if let autoAcceptProof, !autoAcceptProof.isEmpty {
                        return autoAcceptProof.withUnsafeBytes { rawBuf in
                            let bytesPtr = rawBuf.baseAddress?.assumingMemoryBound(to: UInt8.self)
                            return platform_wallet_send_contact_request(
                                handle,
                                senderFFI,
                                recipientFFI,
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
                            senderFFI,
                            recipientFFI,
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
            }()

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
            defer { platform_wallet_contact_request_handle_array_free(array) }
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
        let ourFFI = identifierToFFI(ourIdentityId)
        let contactFFI = identifierToFFI(contactIdentityId)
        try await Task.detached(priority: .userInitiated) {
            var error = PlatformWalletFFIError()
            let result = platform_wallet_reject_contact_request(
                handle,
                ourFFI,
                contactFFI,
                &error
            )
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
        let idFFI = identifierToFFI(identityId)
        return try await Task.detached(priority: .userInitiated) { () -> [ContactRequest] in
            var array = ContactRequestHandleArray(handles: nil, count: 0)
            var error = PlatformWalletFFIError()
            let result = platform_wallet_fetch_sent_contact_requests(
                handle,
                idFFI,
                &array,
                &error
            )
            guard result == Success else {
                throw PlatformWalletError(result: result, error: error)
            }
            defer { platform_wallet_contact_request_handle_array_free(array) }
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
        let fromFFI = identifierToFFI(fromIdentityId)
        let toFFI = identifierToFFI(toContactIdentityId)
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
            let result: PlatformWalletFFIResult = {
                let call: (UnsafePointer<CChar>?) -> PlatformWalletFFIResult = { memoPtr in
                    platform_wallet_send_dashpay_payment(
                        handle,
                        fromFFI,
                        toFFI,
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
            }()
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
        let ffiId = identifierToFFI(identityId)

        let result = platform_wallet_get_dashpay_profile(
            handle,
            ffiId,
            &ffiProfile,
            &hasProfile,
            &error
        )
        defer { dashpay_profile_ffi_free(ffiProfile) }

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
        let ffiId = identifierToFFI(identityId)
        let displayName = update.displayName
        let publicMessage = update.publicMessage
        let avatarUrl = update.avatarUrl
        let avatarBytes = update.avatarBytes

        return try await Task.detached(priority: .userInitiated) { () -> DashPayProfile in
            var outProfile = dashPayProfileFFIEmpty()
            var error = PlatformWalletFFIError()

            // Each optional CString gets its own `withCString` scope —
            // nested closures keep all three string buffers alive
            // across the FFI call (their lifetime is bounded by the
            // closure, not by the `let` binding). Absent fields pass
            // `nil` directly.
            let result = invokeWithOptionalCStrings(
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
                                ffiId,
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
                                ffiId,
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
                            ffiId,
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
                            ffiId,
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

            defer { dashpay_profile_ffi_free(outProfile) }

            guard result == Success else {
                throw PlatformWalletError(result: result, error: error)
            }
            return DashPayProfile(ffi: outProfile)
        }.value
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
