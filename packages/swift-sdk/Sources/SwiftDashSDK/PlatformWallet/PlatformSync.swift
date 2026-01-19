import Foundation
import DashSDKFFI

/// Platform address sync manager for privacy-preserving balance synchronization
///
/// This class manages the synchronization of platform payment address balances
/// using a privacy-preserving protocol with trunk/branch queries and terminal sync.
public class PlatformSyncStateManager {
    private var handle: Handle

    private init(handle: Handle) {
        self.handle = handle
    }

    deinit {
        _ = platform_sync_state_destroy(handle)
    }

    /// Create a new sync state (for initial sync)
    public static func create() throws -> PlatformSyncStateManager {
        var handle: Handle = NULL_HANDLE
        let result = platform_sync_state_create(&handle)

        guard result == Success else {
            throw PlatformWalletError.walletOperation("Failed to create sync state")
        }

        return PlatformSyncStateManager(handle: handle)
    }

    /// Create a sync state from saved state (for resuming sync)
    public static func create(from state: PlatformSyncState) throws -> PlatformSyncStateManager {
        var handle: Handle = NULL_HANDLE
        let ffiState = state.ffiValue
        let result = platform_sync_state_create_from_ffi(ffiState, &handle)

        guard result == Success else {
            throw PlatformWalletError.walletOperation("Failed to create sync state from saved state")
        }

        return PlatformSyncStateManager(handle: handle)
    }

    /// Get the current sync state (for persistence)
    public func getState() throws -> PlatformSyncState {
        var ffiState = PlatformSyncStateFFI(
            last_full_sync_timestamp: 0,
            checkpoint_height: 0,
            last_terminal_block: 0,
            highest_found_index: 0
        )

        let result = platform_sync_state_get(handle, &ffiState)

        guard result == Success else {
            throw PlatformWalletError.invalidHandle
        }

        return PlatformSyncState(ffiState: ffiState)
    }

    /// Check if a full sync is needed
    ///
    /// Returns true if more than 6 days 20 hours have passed since the last full sync,
    /// or if no full sync has ever been performed.
    public func needsFullSync() throws -> Bool {
        var needsSync: Bool = false
        let result = platform_sync_state_needs_full_sync(handle, &needsSync)

        guard result == Success else {
            throw PlatformWalletError.invalidHandle
        }

        return needsSync
    }

    /// Synchronize platform payment addresses using pre-generated addresses
    ///
    /// This performs privacy-preserving address balance synchronization using
    /// addresses already generated in the account's address pool. No key material required.
    ///
    /// 1. Full sync (if needed): trunk + branch queries with privacy protection
    /// 2. Terminal sync: fetch balance changes after checkpoint
    ///
    /// - Parameters:
    ///   - wallet: The platform wallet to sync
    ///   - sdk: The SDK instance to use for network calls
    /// - Returns: The sync result with balance information
    public func syncAddresses(
        wallet: PlatformWallet,
        sdk: SDK
    ) throws -> PlatformSyncResult {
        guard let sdkHandle = sdk.handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        var ffiResult = PlatformSyncResultFFI(
            full_sync_performed: false,
            checkpoint_height: 0,
            highest_terminal_block: 0,
            total_balance: 0,
            funded_address_count: 0,
            highest_found_index: 0
        )
        var ffiError = PlatformWalletFFIError()

        let result = platform_wallet_sync_addresses(
            wallet.walletHandle,
            UnsafeMutableRawPointer(sdkHandle),
            handle,
            &ffiResult,
            &ffiError
        )

        guard result == Success else {
            throw PlatformWalletError(result: result, error: ffiError)
        }

        return PlatformSyncResult(ffiResult: ffiResult)
    }

    /// The underlying FFI handle (for advanced use)
    internal var syncStateHandle: Handle {
        return handle
    }
}
