import Foundation
import SwiftData

/// Manages the wallet lifecycle: creation, persistence, and sub-wallet access.
///
/// This is the main entry point for Phase 1 integration. It replaces the
/// manual setup of separate key-wallet, SDK, and SPV systems with a single
/// coordinated manager.
///
/// Usage:
/// ```swift
/// let manager = try PlatformWalletManager(sdk: sdkPointer)
/// let wallet = try await manager.createWallet(seed: seedData, network: .testnet)
/// let platformAddresses = try wallet.platformAddressWallet()
/// let balances = try await platformAddresses.syncBalances()
/// ```
public class PlatformWalletManager {
    let handle: Handle

    /// Persistence handler — retained for the lifetime of the manager
    /// so the callback context pointer remains valid.
    private var persistenceHandler: PlatformWalletPersistenceHandler?

    /// Create a new PlatformWalletManager from an existing SDK instance.
    ///
    /// - Parameters:
    ///   - sdk: The SDK instance.
    ///   - modelContainer: SwiftData container for incremental persistence.
    ///     Pass `nil` for in-memory-only operation.
    public convenience init(sdk: SDK, modelContainer: ModelContainer? = nil) throws {
        guard let sdkHandle = sdk.handle else {
            throw PlatformWalletError.invalidParameter
        }
        let innerSdkPtr = dash_sdk_get_inner_sdk_ptr(sdkHandle)
        guard innerSdkPtr != nil else {
            throw PlatformWalletError.invalidParameter
        }
        try self.init(sdkPointer: UnsafeRawPointer(innerSdkPtr!), modelContainer: modelContainer)
    }

    /// Create a new PlatformWalletManager from a raw Sdk pointer.
    ///
    /// - Parameters:
    ///   - sdkPointer: Raw pointer to the inner `Sdk`.
    ///   - modelContainer: SwiftData container for incremental persistence.
    ///     Pass `nil` for in-memory-only operation.
    public init(sdkPointer: UnsafeRawPointer, modelContainer: ModelContainer? = nil) throws {
        var handle: Handle = NULL_HANDLE
        var error = PlatformWalletFFIError()

        // Set up persistence: if a SwiftData container is provided, create
        // a handler that receives incremental updates from Rust and upserts
        // into SwiftData. Otherwise, use no-op callbacks.
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
    }

    /// Access the persistence handler for loading cached data.
    public var persistence: PlatformWalletPersistenceHandler? {
        persistenceHandler
    }

    deinit {
        var error = PlatformWalletFFIError()
        _ = platform_wallet_manager_destroy(handle, &error)
    }

    /// Create a wallet from a BIP39 mnemonic phrase (English).
    ///
    /// - Parameters:
    ///   - mnemonic: BIP39 mnemonic phrase (12 or 24 words).
    ///   - network: Target network.
    ///   - createDefaultAccounts: Whether to create default HD accounts.
    /// - Returns: A managed `ManagedPlatformWallet` handle.
    public func createWallet(
        mnemonic: String,
        network: PlatformNetwork,
        createDefaultAccounts: Bool = true
    ) throws -> ManagedPlatformWallet {
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
        return ManagedPlatformWallet(handle: walletHandle, walletId: idData)
    }

    /// Create a wallet from raw seed bytes.
    ///
    /// - Parameters:
    ///   - seed: 64-byte seed (from BIP39 mnemonic derivation).
    ///   - network: Target network.
    ///   - createDefaultAccounts: Whether to create default HD accounts.
    /// - Returns: A managed `ManagedPlatformWallet` handle.
    public func createWallet(
        seed: Data,
        network: PlatformNetwork,
        createDefaultAccounts: Bool = true
    ) throws -> ManagedPlatformWallet {
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
        return ManagedPlatformWallet(handle: walletHandle, walletId: idData)
    }
}
