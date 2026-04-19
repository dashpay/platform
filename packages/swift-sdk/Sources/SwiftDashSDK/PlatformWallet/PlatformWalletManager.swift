import Foundation
import SwiftData
import Combine

/// The one thing SwiftUI needs for all wallet operations.
///
/// Owns the Rust-side `PlatformWalletManager` handle which drives:
/// - Wallet creation from mnemonic/seed
/// - SPV sync (core chain: headers, filters, masternodes)
/// - BLAST address balance sync
/// - Identity, DashPay, asset lock, token-balance tracking
/// - Persistence via SwiftData callbacks
///
/// Use as a root `@StateObject` and pass via `.environmentObject(_:)`.
/// Views observe `@Published` properties directly — no coordinator
/// class in the middle.
@MainActor
public class PlatformWalletManager: ObservableObject {
    // MARK: - Published observables

    /// Whether [`configure`] has been called successfully.
    @Published public private(set) var isConfigured: Bool = false

    /// The current SPV sync progress. Updated by the polling task
    /// started in [`configure`].
    @Published public private(set) var spvProgress: PlatformSpvSyncProgress = .empty

    /// Whether the Rust-owned platform-address sync manager is currently in flight.
    @Published public private(set) var platformAddressSyncIsSyncing: Bool = false

    /// Last completed platform-address sync event emitted by Rust.
    @Published public internal(set) var lastPlatformAddressSyncEvent: PlatformAddressSyncEvent?

    /// The active wallet (at most one per manager for now).
    @Published public private(set) var wallet: ManagedPlatformWallet?

    /// Last error from a wallet operation, if any. Cleared on successful op.
    @Published public private(set) var lastError: Error?

    // MARK: - Internals

    /// FFI handle; `NULL_HANDLE` until [`configure`] is called.
    internal private(set) var handle: Handle = NULL_HANDLE

    /// Retained for the lifetime of the FFI handle so the callback
    /// context pointer remains valid.
    private var persistenceHandler: PlatformWalletPersistenceHandler?

    /// Retained for the lifetime of the FFI handle so the event-handler
    /// context pointer remains valid.
    private var eventHandler: PlatformWalletEventHandler?

    /// Background task that polls SPV progress.
    private var progressPollTask: Task<Void, Never>?

    // MARK: - Init

    /// Empty init for `@StateObject` usage. Call [`configure`] before
    /// any wallet operations.
    public init() {}

    /// Convenience: create and configure in one call.
    public convenience init(sdk: SDK, modelContainer: ModelContainer? = nil) throws {
        self.init()
        try self.configure(sdk: sdk, modelContainer: modelContainer)
    }

    deinit {
        progressPollTask?.cancel()
        if handle != NULL_HANDLE {
            var stopError = PlatformWalletFFIError()
            _ = platform_wallet_manager_platform_address_sync_stop(handle, &stopError)
            var error = PlatformWalletFFIError()
            _ = platform_wallet_manager_destroy(handle, &error)
        }
    }

    // MARK: - Configuration

    /// Configure the manager with an SDK and an optional SwiftData
    /// container. Must be called before any wallet operations.
    ///
    /// Spawns a background task that polls SPV sync progress every
    /// second and publishes it to [`spvProgress`].
    public func configure(sdk: SDK, modelContainer: ModelContainer? = nil) throws {
        precondition(!isConfigured, "PlatformWalletManager already configured")
        guard let sdkHandle = sdk.handle else {
            throw PlatformWalletError.invalidParameter
        }
        guard let innerSdkPtr = dash_sdk_get_inner_sdk_ptr(sdkHandle) else {
            throw PlatformWalletError.invalidParameter
        }
        try configure(sdkPointer: UnsafeRawPointer(innerSdkPtr), modelContainer: modelContainer)
    }

    /// Configure with a raw Sdk pointer (advanced usage).
    public func configure(sdkPointer: UnsafeRawPointer, modelContainer: ModelContainer? = nil) throws {
        var handle: Handle = NULL_HANDLE
        var error = PlatformWalletFFIError()

        let handler: PlatformWalletPersistenceHandler?
        var persistence: PersistenceCallbacks
        if let container = modelContainer {
            let h = PlatformWalletPersistenceHandler(modelContainer: container)
            persistence = h.makeCallbacks()
            handler = h
        } else {
            persistence = PersistenceCallbacks()
            handler = nil
        }

        let eventHandler = PlatformWalletEventHandler(manager: self)
        var eventHandlerCallbacks = eventHandler.makeCallbacks()

        let result = platform_wallet_manager_create(
            sdkPointer,
            &persistence,
            &eventHandlerCallbacks,
            &handle,
            &error
        )

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        self.handle = handle
        self.persistenceHandler = handler
        self.eventHandler = eventHandler
        self.isConfigured = true

        startProgressPolling()
    }

    /// Access the persistence handler for loading cached data.
    public var persistence: PlatformWalletPersistenceHandler? {
        persistenceHandler
    }

    // MARK: - Wallet creation

    /// Create a wallet from a BIP39 mnemonic phrase (English).
    ///
    /// Stores the returned wallet as the active [`wallet`] published
    /// property. If `name` is provided, writes it onto the persisted
    /// [`PersistentWallet`] row so the wallet detail view has a
    /// user-facing label.
    @discardableResult
    public func createWallet(
        mnemonic: String,
        network: PlatformNetwork,
        name: String? = nil,
        createDefaultAccounts: Bool = true
    ) throws -> ManagedPlatformWallet {
        try ensureConfigured()
        var walletHandle: Handle = NULL_HANDLE
        var walletId: (UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                       UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                       UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                       UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8) =
            (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
        var error = PlatformWalletFFIError()

        let accountOptions: UInt32 = createDefaultAccounts ? 1 : 0

        let result = mnemonic.withCString { mnemonicPtr in
            platform_wallet_manager_create_wallet_from_mnemonic(
                handle,
                mnemonicPtr,
                network.rawValue,
                accountOptions,
                &walletHandle,
                &walletId,
                &error
            )
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        let idData = withUnsafeBytes(of: &walletId) { Data($0) }
        if let name = name, !name.isEmpty {
            persistenceHandler?.setWalletName(walletId: idData, name: name)
        }
        let w = ManagedPlatformWallet(handle: walletHandle, walletId: idData)
        self.wallet = w
        return w
    }

    /// Create a wallet from raw 64-byte seed bytes.
    @discardableResult
    public func createWallet(
        seed: Data,
        network: PlatformNetwork,
        name: String? = nil,
        createDefaultAccounts: Bool = true
    ) throws -> ManagedPlatformWallet {
        try ensureConfigured()
        guard seed.count == 64 else {
            throw PlatformWalletError.invalidParameter
        }

        var walletHandle: Handle = NULL_HANDLE
        var walletId: (UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                       UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                       UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                       UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8) =
            (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
        var error = PlatformWalletFFIError()

        let accountOptions: UInt32 = createDefaultAccounts ? 1 : 0

        let result = seed.withUnsafeBytes { seedPtr in
            platform_wallet_manager_create_wallet_from_seed(
                handle,
                network.rawValue,
                seedPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                seed.count,
                accountOptions,
                &walletHandle,
                &walletId,
                &error
            )
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        let idData = withUnsafeBytes(of: &walletId) { Data($0) }
        if let name = name, !name.isEmpty {
            persistenceHandler?.setWalletName(walletId: idData, name: name)
        }
        let w = ManagedPlatformWallet(handle: walletHandle, walletId: idData)
        self.wallet = w
        return w
    }

    // MARK: - Watch-only restore from persister

    /// Rehydrate wallets from SwiftData on app launch.
    ///
    /// Calls `platform_wallet_manager_load_from_persistor` which fires
    /// the Swift-side `on_load_wallet_list_fn` callback. For each
    /// persisted wallet, Rust reconstructs a **watch-only** `Wallet`
    /// plus the wallet's persisted platform-address sync snapshot.
    /// After the FFI returns, we call `platform_wallet_manager_get_wallet`
    /// for each restored id so Swift gets a `ManagedPlatformWallet`
    /// handle.
    ///
    /// Signing operations will fail until a future unlock flow
    /// upgrades a watch-only wallet to a signing wallet via the
    /// mnemonic stored in Keychain.
    ///
    /// Idempotent: if there's no persisted state, does nothing and
    /// leaves `self.wallet` untouched. Safe to call before any
    /// `createWallet` flow.
    @discardableResult
    public func loadFromPersistor() throws -> [ManagedPlatformWallet] {
        try ensureConfigured()

        var error = PlatformWalletFFIError()
        let loadResult = platform_wallet_manager_load_from_persistor(handle, &error)
        guard loadResult == Success else {
            throw PlatformWalletError(result: loadResult, error: error)
        }

        // Ask SwiftData for the list of wallet ids we just told Rust
        // to load. We reuse the same container rather than shipping a
        // separate FFI "list ids" entry, because SwiftData already is
        // the source of truth.
        guard let persistenceHandler = persistenceHandler else {
            return []
        }
        let walletIds = persistenceHandler.restorableWalletIds()
        var restored: [ManagedPlatformWallet] = []
        restored.reserveCapacity(walletIds.count)

        for walletId in walletIds {
            guard walletId.count == 32 else { continue }
            var walletHandle: Handle = NULL_HANDLE
            var fetchError = PlatformWalletFFIError()
            let fetchResult = walletId.withUnsafeBytes { idPtr -> PlatformWalletFFIResult in
                guard let base = idPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                    return ErrorNullPointer
                }
                return platform_wallet_manager_get_wallet(
                    handle,
                    base,
                    &walletHandle,
                    &fetchError
                )
            }
            if fetchResult == Success {
                let managedWallet = ManagedPlatformWallet(handle: walletHandle, walletId: walletId)
                restored.append(managedWallet)
            } else {
                // Log and skip — one wallet failing doesn't fail the
                // whole restore. Usually means wallet_id / xpub
                // disagreement (SwiftData drift vs. Rust recompute).
                self.lastError = PlatformWalletError(result: fetchResult, error: fetchError)
            }
        }

        // Publish the first restored wallet for single-wallet UX
        // compatibility; multi-wallet callers iterate the return
        // value directly.
        if self.wallet == nil, let first = restored.first {
            self.wallet = first
        }
        return restored
    }

    // MARK: - Xpub rendering

    /// Render a bincode-encoded per-account `ExtendedPubKey` (as
    /// stored on `PersistentAccount.accountExtendedPubKeyBytes`) as a
    /// BIP32 base58check string. The encoded key carries its own
    /// network, so `xpub…`/`tpub…` is produced automatically.
    ///
    /// Returns `nil` if the bytes are empty or the decode fails.
    public static func accountExtendedPubKeyString(bytes: Data) -> String? {
        guard !bytes.isEmpty else { return nil }
        var outPtr: UnsafeMutablePointer<CChar>? = nil
        var error = PlatformWalletFFIError()
        let result: PlatformWalletFFIResult = bytes.withUnsafeBytes { raw in
            guard let base = raw.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                return ErrorNullPointer
            }
            return platform_wallet_account_xpub_to_string(
                base,
                bytes.count,
                &outPtr,
                &error
            )
        }
        guard result == Success, let cStr = outPtr else {
            return nil
        }
        let str = String(cString: cStr)
        platform_wallet_free_string(cStr)
        return str
    }

    // MARK: - Internals

    private func ensureConfigured() throws {
        if !isConfigured || handle == NULL_HANDLE {
            throw PlatformWalletError.invalidHandle
        }
    }

    /// Starts the SPV progress polling loop. Cancelled on deinit.
    private func startProgressPolling() {
        progressPollTask?.cancel()
        progressPollTask = Task { [weak self] in
            while !Task.isCancelled {
                guard let self = self else { return }
                if let progress = try? self.syncProgress() {
                    self.spvProgress = progress
                }
                if let isSyncing = try? self.isPlatformAddressSyncing() {
                    self.platformAddressSyncIsSyncing = isSyncing
                }
                try? await Task.sleep(for: .seconds(1))
            }
        }
    }
}
