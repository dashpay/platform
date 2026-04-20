import Foundation

/// A managed wallet created by `PlatformWalletManager`.
///
/// Provides access to sub-wallets (platform addresses, core, identity, etc.)
/// and lock-free balance reads.
public class ManagedPlatformWallet {
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
    public struct IdentityAddressInput: Sendable {
        /// `0` = P2PKH, `1` = P2SH. Mirrors the Rust-side
        /// `PlatformAddress` discriminant.
        public let addressType: UInt8
        /// 20-byte address hash.
        public let hash: Data
        /// Current anti-replay nonce for this address.
        public let nonce: UInt32
        /// Credits to spend from this address.
        public let credits: UInt64

        public init(addressType: UInt8, hash: Data, nonce: UInt32, credits: UInt64) {
            self.addressType = addressType
            self.hash = hash
            self.nonce = nonce
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
        /// Platform-serialized identity bytes. Round-trips via
        /// `Identity::deserialize_from_bytes_no_limit` on the Rust
        /// side and is the canonical form used for persistence.
        public let serializedBytes: Data
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
    ///   - inputs: Contributing addresses with their current nonce and
    ///     credits to spend.
    ///   - output: Optional refund address + credits. `nil` when any
    ///     residual should go into the new identity.
    ///   - identityIndex: BIP-9 identity index in the HD tree.
    ///   - keyCount: Number of authentication keys to register
    ///     (must be ≥ 1). First key is MASTER, the rest HIGH.
    public func registerIdentityFromAddresses(
        inputs: [IdentityAddressInput],
        output: IdentityAddressOutput?,
        identityIndex: UInt32,
        keyCount: UInt32
    ) async throws -> CreatedIdentity {
        guard !inputs.isEmpty else {
            throw PlatformWalletError.invalidParameter
        }

        // Copy inputs into the flat FFI shape. Do it off the main
        // actor via `Task.detached` because `platform_wallet_register_
        // identity_from_addresses` internally drives a tokio runtime
        // via `block_on`; calling from the main thread would park
        // user-interactive QoS on default-QoS workers (same pattern
        // we use for BLAST sync).
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) { () -> CreatedIdentity in
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
                    nonce: input.nonce,
                    credits: input.credits
                )
            }

            let outputFFI: IdentityOutputAddressFFI = {
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
            var outBytesPtr: UnsafeMutablePointer<UInt8>? = nil
            var outBytesLen: Int = 0
            var error = PlatformWalletFFIError()

            let result = ffiInputs.withUnsafeBufferPointer { inputsBuf in
                platform_wallet_register_identity_from_addresses(
                    handle,
                    identityIndex,
                    keyCount,
                    inputsBuf.baseAddress,
                    inputsBuf.count,
                    outputFFI,
                    &outIdentityId,
                    &outBytesPtr,
                    &outBytesLen,
                    &error
                )
            }

            guard result == Success else {
                throw PlatformWalletError(result: result, error: error)
            }
            guard let bytesPtr = outBytesPtr, outBytesLen > 0 else {
                throw PlatformWalletError.serialization("identity bytes not returned")
            }

            let identityBytes = Data(bytes: bytesPtr, count: outBytesLen)
            free_identity_bytes(bytesPtr, outBytesLen)

            let idData = withUnsafeBytes(of: outIdentityId) { Data($0) }
            return CreatedIdentity(
                identityId: idData,
                serializedBytes: identityBytes
            )
        }.value
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
