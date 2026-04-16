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

        var eventHandler = EventHandlerCallbacks(
            context: nil,
            on_wallet_event_fn: nil,
            on_error_fn: nil
        )

        let result = platform_wallet_manager_create(
            sdkPointer,
            &persistence,
            &eventHandler,
            &handle,
            &error
        )

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        self.handle = handle
        self.persistenceHandler = handler
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
    /// property.
    @discardableResult
    public func createWallet(
        mnemonic: String,
        network: PlatformNetwork,
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
        let w = ManagedPlatformWallet(handle: walletHandle, walletId: idData)
        self.wallet = w
        return w
    }

    /// Create a wallet from raw 64-byte seed bytes.
    @discardableResult
    public func createWallet(
        seed: Data,
        network: PlatformNetwork,
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
        let w = ManagedPlatformWallet(handle: walletHandle, walletId: idData)
        self.wallet = w
        return w
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
                try? await Task.sleep(for: .seconds(1))
            }
        }
    }
}
