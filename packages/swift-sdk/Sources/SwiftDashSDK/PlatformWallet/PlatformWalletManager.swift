import Foundation

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

    /// Create a new PlatformWalletManager.
    ///
    /// - Parameters:
    ///   - sdkPointer: Raw pointer to a `Sdk` instance (from the existing SDK FFI).
    ///   - persistenceContext: Optional opaque context for persistence callbacks.
    ///     Pass `nil` for in-memory-only persistence.
    public init(sdkPointer: UnsafeRawPointer) throws {
        var handle: Handle = NULL_HANDLE
        var error = PlatformWalletFFIError()

        // No-op persistence and event callbacks for Phase 1.
        // The Rust side accumulates changesets in-memory.
        var persistence = PersistenceCallbacks(
            context: nil,
            on_store_fn: nil,
            on_flush_fn: nil
        )
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
    }

    deinit {
        var error = PlatformWalletFFIError()
        _ = platform_wallet_manager_destroy(handle, &error)
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
