import Foundation
import DashSDKFFI

/// One row of the per-(identity, token) sync cache held by the
/// Rust-side `IdentitySyncManager`. Mirrors the FFI
/// `IdentityTokenSyncInfoFFI` shape — `identity_id` is replicated on
/// every row so the whole-store snapshot stays a flat array.
public struct IdentityTokenSyncRow: Sendable {
    public let identityId: Identifier
    public let tokenId: Identifier
    public let contractId: Identifier
    public let balance: UInt64
    public let identityContractNonce: UInt64

    init(ffi: IdentityTokenSyncInfoFFI) {
        var identity = ffi.identity_id
        self.identityId = withUnsafeBytes(of: &identity) { Data($0) }
        var token = ffi.token_id
        self.tokenId = withUnsafeBytes(of: &token) { Data($0) }
        var contract = ffi.contract_id
        self.contractId = withUnsafeBytes(of: &contract) { Data($0) }
        self.balance = ffi.balance
        self.identityContractNonce = ffi.identity_contract_nonce
    }
}

/// Snapshot of one identity's sync row (rows + last-sync timestamp)
/// returned by `identitySyncStateForIdentity`.
public struct IdentityTokenSyncSnapshot: Sendable {
    public let rows: [IdentityTokenSyncRow]
    public let lastSyncUnixSeconds: UInt64

    public init(rows: [IdentityTokenSyncRow], lastSyncUnixSeconds: UInt64) {
        self.rows = rows
        self.lastSyncUnixSeconds = lastSyncUnixSeconds
    }
}

extension PlatformWalletManager {
    /// Start the identity-token sync background loop.
    public func startIdentityTokenSync() throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        try platform_wallet_manager_identity_sync_start(handle).check()
    }

    /// Stop the identity-token sync background loop.
    public func stopIdentityTokenSync() throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        try platform_wallet_manager_identity_sync_stop(handle).check()
    }

    /// Whether the identity-token sync background loop is running.
    public func isIdentityTokenSyncRunning() throws -> Bool {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        var running = false
        try platform_wallet_manager_identity_sync_is_running(handle, &running).check()
        return running
    }

    /// Whether an identity-token sync pass is currently in flight.
    public func isIdentityTokenSyncing() throws -> Bool {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        var syncing = false
        try platform_wallet_manager_identity_sync_is_syncing(handle, &syncing).check()
        return syncing
    }

    /// Unix seconds of the last completed identity-token sync pass for
    /// the given identity, or 0 if it has never been synced.
    public func lastIdentityTokenSyncUnixSeconds(for identityId: Identifier) throws -> UInt64 {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        guard identityId.count == 32 else {
            throw PlatformWalletError.invalidIdentifier(
                "identityId must be 32 bytes, got \(identityId.count)"
            )
        }
        var lastSync: UInt64 = 0
        try identityId.withUnsafeBytes { idPtr in
            try platform_wallet_manager_identity_sync_last_sync_unix_seconds(
                handle,
                idPtr.bindMemory(to: UInt8.self).baseAddress,
                &lastSync
            ).check()
        }
        return lastSync
    }

    /// Set the polling interval (clamped to >= 1 second on the Rust side).
    public func setIdentityTokenSyncInterval(seconds: UInt64) throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        try platform_wallet_manager_identity_sync_set_interval(handle, seconds).check()
    }

    /// Run one identity-token sync pass across every registered
    /// identity. Synchronous from the FFI side — runs on a worker
    /// detached `Task`. If a pass is already in flight, returns
    /// without doing extra work.
    public func syncIdentityTokensNow() async throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        let handle = self.handle
        try await Task.detached(priority: .userInitiated) {
            try platform_wallet_manager_identity_sync_sync_now(handle).check()
        }.value
    }

    /// Add or replace the sync registry row for `identityId`. Each
    /// entry in `tokenIds` becomes a watched-token row with
    /// placeholder balance/contract/nonce until the next sync pass
    /// populates real values. Idempotent — calling with the same
    /// identity replaces the row.
    public func registerIdentityForTokenSync(
        identityId: Identifier,
        tokenIds: [Identifier]
    ) throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        guard identityId.count == 32 else {
            throw PlatformWalletError.invalidIdentifier(
                "identityId must be 32 bytes, got \(identityId.count)"
            )
        }
        // Flatten token ids into one contiguous 32*N buffer so the
        // FFI can read them as back-to-back chunks.
        var flat = Data(capacity: 32 * tokenIds.count)
        for tid in tokenIds {
            guard tid.count == 32 else {
                throw PlatformWalletError.invalidIdentifier(
                    "token id must be 32 bytes, got \(tid.count)"
                )
            }
            flat.append(tid)
        }
        try identityId.withUnsafeBytes { idPtr in
            try flat.withUnsafeBytes { tokensPtr in
                try platform_wallet_manager_identity_sync_register_identity(
                    handle,
                    idPtr.bindMemory(to: UInt8.self).baseAddress,
                    tokensPtr.bindMemory(to: UInt8.self).baseAddress,
                    UInt(tokenIds.count)
                ).check()
            }
        }
    }

    /// Remove `identityId` from the sync registry. Idempotent.
    public func unregisterIdentityForTokenSync(identityId: Identifier) throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        guard identityId.count == 32 else {
            throw PlatformWalletError.invalidIdentifier(
                "identityId must be 32 bytes, got \(identityId.count)"
            )
        }
        try identityId.withUnsafeBytes { idPtr in
            try platform_wallet_manager_identity_sync_unregister_identity(
                handle,
                idPtr.bindMemory(to: UInt8.self).baseAddress
            ).check()
        }
    }

    /// Replace the watched-token list on an already-registered
    /// identity. No-op when the identity isn't registered (call
    /// `registerIdentityForTokenSync` first if you want promotion).
    public func updateWatchedTokensForTokenSync(
        identityId: Identifier,
        tokenIds: [Identifier]
    ) throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        guard identityId.count == 32 else {
            throw PlatformWalletError.invalidIdentifier(
                "identityId must be 32 bytes, got \(identityId.count)"
            )
        }
        var flat = Data(capacity: 32 * tokenIds.count)
        for tid in tokenIds {
            guard tid.count == 32 else {
                throw PlatformWalletError.invalidIdentifier(
                    "token id must be 32 bytes, got \(tid.count)"
                )
            }
            flat.append(tid)
        }
        try identityId.withUnsafeBytes { idPtr in
            try flat.withUnsafeBytes { tokensPtr in
                try platform_wallet_manager_identity_sync_update_watched_tokens(
                    handle,
                    idPtr.bindMemory(to: UInt8.self).baseAddress,
                    tokensPtr.bindMemory(to: UInt8.self).baseAddress,
                    UInt(tokenIds.count)
                ).check()
            }
        }
    }

    /// Snapshot the per-identity token sync state for one identity.
    /// Returns `nil` when the identity has no cached state.
    public func identityTokenSyncState(
        for identityId: Identifier
    ) throws -> IdentityTokenSyncSnapshot? {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        guard identityId.count == 32 else {
            throw PlatformWalletError.invalidIdentifier(
                "identityId must be 32 bytes, got \(identityId.count)"
            )
        }

        var rowsPtr: UnsafeMutablePointer<IdentityTokenSyncInfoFFI>? = nil
        var rowsCount: UInt = 0
        var lastSync: UInt64 = 0
        try identityId.withUnsafeBytes { idPtr in
            try platform_wallet_manager_identity_sync_state_for_identity(
                handle,
                idPtr.bindMemory(to: UInt8.self).baseAddress,
                &rowsPtr,
                &rowsCount,
                &lastSync
            ).check()
        }

        defer {
            if let rowsPtr {
                platform_wallet_manager_identity_sync_state_free(rowsPtr, rowsCount)
            }
        }

        guard let rowsPtr else {
            return nil
        }
        let buffer = UnsafeBufferPointer(start: rowsPtr, count: Int(rowsCount))
        let rows = buffer.map { IdentityTokenSyncRow(ffi: $0) }
        return IdentityTokenSyncSnapshot(rows: rows, lastSyncUnixSeconds: lastSync)
    }

    /// Snapshot the per-identity token sync state for every cached
    /// identity in one flat array.
    public func allIdentityTokenSyncRows() throws -> [IdentityTokenSyncRow] {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        var rowsPtr: UnsafeMutablePointer<IdentityTokenSyncInfoFFI>? = nil
        var rowsCount: UInt = 0
        try platform_wallet_manager_identity_sync_state_all(
            handle,
            &rowsPtr,
            &rowsCount
        ).check()

        defer {
            if let rowsPtr {
                platform_wallet_manager_identity_sync_state_free(rowsPtr, rowsCount)
            }
        }

        guard let rowsPtr else {
            return []
        }
        let buffer = UnsafeBufferPointer(start: rowsPtr, count: Int(rowsCount))
        return buffer.map { IdentityTokenSyncRow(ffi: $0) }
    }
}
