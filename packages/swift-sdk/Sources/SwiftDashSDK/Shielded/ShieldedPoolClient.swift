// ShieldedPoolClient.swift
// SwiftDashSDK
//
// High-level Swift wrapper for the Rust ShieldedPoolClient.
// Manages a local commitment tree (SQLite-backed), note tracking,
// and Orchard bundle construction. All cryptographic operations
// stay in Rust across the FFI boundary.
//
// Usage:
//   let client = try ShieldedPoolClient(dbPath: path, spendingKey: key)
//   let (newNotes, bal) = try await client.syncNotes(sdk: sdk)
//   let balance = try client.balance
//   let bundle = try await client.buildTransferBundle(recipientAddress: addr, amount: 1000)
//   try await sdk.shieldedTransfer(bundle: bundle, valueBalance: ...)

import Foundation
import DashSDKFFI

// MARK: - ShieldedPoolClient

/// Client for the Dash Platform shielded pool.
///
/// Manages the local commitment tree (SQLite-backed), note tracking,
/// and Orchard bundle construction. All crypto stays in Rust.
///
/// The underlying Rust handle is thread-safe. This class is marked
/// `@unchecked Sendable` so it can be passed across actor boundaries.
/// The handle is freed automatically on `deinit`.
public final class ShieldedPoolClient: @unchecked Sendable {

    /// Opaque pointer to the Rust `ShieldedPoolClient`.
    private let handle: OpaquePointer

    // MARK: - Lifecycle

    /// Create a new shielded pool client.
    ///
    /// - Parameters:
    ///   - dbPath: Path to the SQLite database file (created if it does not exist).
    ///   - spendingKey: 32-byte Orchard spending key.
    /// - Throws: `SDKError.invalidParameter` if the spending key is not 32 bytes.
    ///           `SDKError.internalError` if the Rust constructor fails.
    public init(dbPath: String, spendingKey: Data) throws {
        guard spendingKey.count == 32 else {
            throw SDKError.invalidParameter(
                "Spending key must be exactly 32 bytes, got \(spendingKey.count)"
            )
        }

        var skTuple = dataToBytes32(spendingKey)

        let result = dbPath.withCString { pathCStr in
            withUnsafePointer(to: &skTuple) { skPtr in
                dash_sdk_shielded_pool_client_create(pathCStr, skPtr)
            }
        }

        if let error = result.error {
            let errorMessage = error.pointee.message != nil
                ? String(cString: error.pointee.message!)
                : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError("Failed to create ShieldedPoolClient: \(errorMessage)")
        }

        guard let dataPtr = result.data else {
            throw SDKError.internalError("No handle returned from ShieldedPoolClient create")
        }

        self.handle = OpaquePointer(dataPtr)
    }

    deinit {
        dash_sdk_shielded_pool_client_destroy(handle)
    }

    // MARK: - Properties

    /// The default Orchard payment address (raw bytes, typically 43 bytes).
    ///
    /// - Throws: `SDKError` if the underlying Rust call fails or returns invalid hex.
    public var address: Data {
        get throws {
            let result = dash_sdk_shielded_pool_client_get_address(handle)
            let hexString = try Self.extractString(result)
            guard let addressData = hexToData(hexString) else {
                throw SDKError.serializationError("Invalid hex address returned from pool client")
            }
            return addressData
        }
    }

    /// Current shielded balance (sum of unspent note values, in credits).
    ///
    /// - Throws: `SDKError` if the underlying Rust call fails or the balance
    ///           string cannot be parsed.
    public var balance: UInt64 {
        get throws {
            let result = dash_sdk_shielded_pool_client_get_balance(handle)
            let decimalString = try Self.extractString(result)
            guard let value = UInt64(decimalString) else {
                throw SDKError.serializationError(
                    "Invalid balance string returned: \(decimalString)"
                )
            }
            return value
        }
    }

    // MARK: - Sync

    /// Sync notes from the platform shielded pool.
    ///
    /// Fetches encrypted notes from the network, decrypts those belonging to this
    /// spending key, and appends them to the local commitment tree.
    ///
    /// This method blocks on the Rust side (`block_on`), so it is dispatched to a
    /// background queue and bridged back via `withCheckedThrowingContinuation`.
    ///
    /// - Parameter sdk: An initialized `SDK` instance for network access.
    /// - Returns: Tuple of (number of new notes found, current balance in credits).
    /// - Throws: `SDKError` on network or processing failure.
    @MainActor
    public func syncNotes(sdk: SDK) async throws -> (newNotes: Int, balance: UInt64) {
        guard let sdkHandle = sdk.handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let poolHandle = self.handle
        let sdkPtr = PoolClientSendableSdkPtr(sdkHandle)

        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                let result = dash_sdk_shielded_pool_client_sync_notes(
                    poolHandle,
                    UnsafePointer(sdkPtr.ptr)
                )

                do {
                    let jsonString = try Self.extractString(result)
                    let parsed = try Self.parseSyncNotesJSON(jsonString)
                    continuation.resume(returning: parsed)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// Check which notes have been spent on-chain (nullifier sync).
    ///
    /// Queries the network for nullifier statuses and marks spent notes in the
    /// local database.
    ///
    /// This method blocks on the Rust side (`block_on`), so it is dispatched to a
    /// background queue and bridged back via `withCheckedThrowingContinuation`.
    ///
    /// - Parameter sdk: An initialized `SDK` instance for network access.
    /// - Returns: Tuple of (number of notes found spent, current balance in credits).
    /// - Throws: `SDKError` on network or processing failure.
    @MainActor
    public func syncNullifiers(sdk: SDK) async throws -> (spentCount: Int, balance: UInt64) {
        guard let sdkHandle = sdk.handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let poolHandle = self.handle
        let sdkPtr = PoolClientSendableSdkPtr(sdkHandle)

        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                let result = dash_sdk_shielded_pool_client_sync_nullifiers(
                    poolHandle,
                    UnsafePointer(sdkPtr.ptr)
                )

                do {
                    let jsonString = try Self.extractString(result)
                    let parsed = try Self.parseSyncNullifiersJSON(jsonString)
                    continuation.resume(returning: parsed)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    // MARK: - Bundle Building

    /// Build a shield bundle (transparent -> shielded).
    ///
    /// Creates an output-only Orchard bundle that moves credits from a transparent
    /// platform balance into the shielded pool.
    ///
    /// This may block for up to ~30 seconds on the first call while the Halo 2
    /// proving key is generated and cached. Subsequent calls are fast.
    ///
    /// - Parameters:
    ///   - amount: Amount in credits to shield.
    ///   - memo: Optional 36-byte memo (OutPoint). If nil, uses zero bytes.
    /// - Returns: `ShieldedBundleHandle` to pass to a transition function.
    /// - Throws: `SDKError` on failure.
    public func buildShieldBundle(
        amount: UInt64,
        memo: Data? = nil
    ) async throws -> ShieldedBundleHandle {
        if let memo = memo, memo.count != 36 {
            throw SDKError.invalidParameter("Memo must be exactly 36 bytes, got \(memo.count)")
        }

        let poolHandle = self.handle
        let memoCopy = memo

        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                var memoTuple = dataToBytes36(memoCopy ?? Data(count: 36))

                let result = withUnsafePointer(to: &memoTuple) { memoPtr in
                    dash_sdk_shielded_pool_client_build_shield_bundle(
                        poolHandle,
                        amount,
                        memoPtr
                    )
                }

                do {
                    let handle = try Self.extractBundleHandle(result)
                    continuation.resume(returning: handle)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// Build a transfer bundle (shielded -> shielded).
    ///
    /// Selects notes and computes Merkle authentication paths automatically from
    /// the local commitment tree. Creates a spend+output bundle that transfers
    /// credits to the recipient within the shielded pool.
    ///
    /// - Parameters:
    ///   - recipientAddress: Raw Orchard payment address bytes of the recipient.
    ///   - amount: Amount in credits to transfer.
    ///   - memo: Optional 36-byte memo. If nil, uses zero bytes.
    /// - Returns: `ShieldedBundleHandle` to pass to `shieldedTransfer(bundle:)`.
    /// - Throws: `SDKError` on failure.
    public func buildTransferBundle(
        recipientAddress: Data,
        amount: UInt64,
        memo: Data? = nil
    ) async throws -> ShieldedBundleHandle {
        guard !recipientAddress.isEmpty else {
            throw SDKError.invalidParameter("Recipient address must not be empty")
        }
        if let memo = memo, memo.count != 36 {
            throw SDKError.invalidParameter("Memo must be exactly 36 bytes, got \(memo.count)")
        }

        let poolHandle = self.handle
        let addrCopy = recipientAddress
        let memoCopy = memo

        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                var memoTuple = dataToBytes36(memoCopy ?? Data(count: 36))

                let result = addrCopy.withUnsafeBytes { addrPtr -> DashSDKResult in
                    guard let addrBase = addrPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                        return DashSDKResult()
                    }
                    return withUnsafePointer(to: &memoTuple) { memoPtr in
                        dash_sdk_shielded_pool_client_build_transfer_bundle(
                            poolHandle,
                            addrBase,
                            addrCopy.count,
                            amount,
                            memoPtr
                        )
                    }
                }

                do {
                    let handle = try Self.extractBundleHandle(result)
                    continuation.resume(returning: handle)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// Build an unshield bundle (shielded -> platform address).
    ///
    /// Selects notes and computes Merkle paths automatically. The platform address
    /// receives funds via the state transition's transparent output field.
    ///
    /// - Parameters:
    ///   - outputAddress: Platform address bytes for the unshield recipient.
    ///   - amount: Amount in credits to unshield.
    ///   - memo: Optional 36-byte memo. If nil, uses zero bytes.
    /// - Returns: `ShieldedBundleHandle` to pass to `unshieldFunds(bundle:)`.
    /// - Throws: `SDKError` on failure.
    public func buildUnshieldBundle(
        outputAddress: Data,
        amount: UInt64,
        memo: Data? = nil
    ) async throws -> ShieldedBundleHandle {
        guard !outputAddress.isEmpty else {
            throw SDKError.invalidParameter("Output address must not be empty")
        }
        if let memo = memo, memo.count != 36 {
            throw SDKError.invalidParameter("Memo must be exactly 36 bytes, got \(memo.count)")
        }

        let poolHandle = self.handle
        let addrCopy = outputAddress
        let memoCopy = memo

        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                var memoTuple = dataToBytes36(memoCopy ?? Data(count: 36))

                let result = addrCopy.withUnsafeBytes { addrPtr -> DashSDKResult in
                    guard let addrBase = addrPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                        return DashSDKResult()
                    }
                    return withUnsafePointer(to: &memoTuple) { memoPtr in
                        dash_sdk_shielded_pool_client_build_unshield_bundle(
                            poolHandle,
                            addrBase,
                            addrCopy.count,
                            amount,
                            memoPtr
                        )
                    }
                }

                do {
                    let handle = try Self.extractBundleHandle(result)
                    continuation.resume(returning: handle)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// Build a withdrawal bundle (shielded -> Core L1 address).
    ///
    /// Selects notes and computes Merkle paths automatically. The output script
    /// receives funds via the state transition's withdrawal mechanism on the
    /// Core (L1) chain.
    ///
    /// - Parameters:
    ///   - outputScript: Core chain output script bytes (e.g. P2PKH or P2SH).
    ///   - amount: Amount in credits to withdraw.
    ///   - memo: Optional 36-byte memo. If nil, uses zero bytes.
    ///   - coreFeePerByte: Core chain fee rate in duffs per byte.
    ///   - pooling: Withdrawal pooling strategy.
    /// - Returns: `ShieldedBundleHandle` to pass to `shieldedWithdraw(bundle:)`.
    /// - Throws: `SDKError` on failure.
    public func buildWithdrawalBundle(
        outputScript: Data,
        amount: UInt64,
        memo: Data? = nil,
        coreFeePerByte: UInt32,
        pooling: WithdrawalPooling
    ) async throws -> ShieldedBundleHandle {
        guard !outputScript.isEmpty else {
            throw SDKError.invalidParameter("Output script must not be empty")
        }
        if let memo = memo, memo.count != 36 {
            throw SDKError.invalidParameter("Memo must be exactly 36 bytes, got \(memo.count)")
        }

        let poolHandle = self.handle
        let scriptCopy = outputScript
        let memoCopy = memo

        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                var memoTuple = dataToBytes36(memoCopy ?? Data(count: 36))

                let result = scriptCopy.withUnsafeBytes { scriptPtr -> DashSDKResult in
                    guard let scriptBase = scriptPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                        return DashSDKResult()
                    }
                    return withUnsafePointer(to: &memoTuple) { memoPtr in
                        dash_sdk_shielded_pool_client_build_withdrawal_bundle(
                            poolHandle,
                            scriptBase,
                            scriptCopy.count,
                            amount,
                            memoPtr,
                            coreFeePerByte,
                            pooling.rawValue
                        )
                    }
                }

                do {
                    let handle = try Self.extractBundleHandle(result)
                    continuation.resume(returning: handle)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }
}

// MARK: - Private Helpers

extension ShieldedPoolClient {

    /// Extract a string from a DashSDKResult, freeing the error or data pointer as needed.
    private static func extractString(_ result: DashSDKResult) throws -> String {
        if let error = result.error {
            let errorMessage = error.pointee.message != nil
                ? String(cString: error.pointee.message!)
                : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError(errorMessage)
        }

        guard let dataPtr = result.data else {
            throw SDKError.notFound("No data returned")
        }

        let string = String(cString: dataPtr.assumingMemoryBound(to: CChar.self))
        dash_sdk_string_free(dataPtr)
        return string
    }

    /// Extract a ShieldedBundleHandle from a DashSDKResult.
    /// The result.data must be a *mut DashSDKOrchardBundleParams allocated by Rust.
    private static func extractBundleHandle(_ result: DashSDKResult) throws -> ShieldedBundleHandle {
        if let error = result.error {
            let errorMessage = error.pointee.message != nil
                ? String(cString: error.pointee.message!)
                : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError(errorMessage)
        }

        guard let dataPtr = result.data else {
            throw SDKError.internalError("No bundle data returned")
        }

        let typedPtr = dataPtr.assumingMemoryBound(to: FFIOrchardBundleParams.self)
        return ShieldedBundleHandle(typedPtr)
    }

    /// Parse the JSON returned by sync_notes: {"newNotes":N,"balance":N}.
    private static func parseSyncNotesJSON(
        _ jsonString: String
    ) throws -> (newNotes: Int, balance: UInt64) {
        guard let jsonData = jsonString.data(using: .utf8) else {
            throw SDKError.serializationError("Sync notes JSON is not valid UTF-8")
        }

        guard let obj = try JSONSerialization.jsonObject(with: jsonData) as? [String: Any] else {
            throw SDKError.serializationError("Sync notes JSON root is not an object")
        }

        guard let newNotesNum = obj["newNotes"] as? NSNumber else {
            throw SDKError.serializationError("Missing 'newNotes' field in sync result")
        }

        guard let balanceNum = obj["balance"] as? NSNumber else {
            throw SDKError.serializationError("Missing 'balance' field in sync result")
        }

        return (newNotes: newNotesNum.intValue, balance: balanceNum.uint64Value)
    }

    /// Parse the JSON returned by sync_nullifiers: {"spentCount":N,"balance":N}.
    private static func parseSyncNullifiersJSON(
        _ jsonString: String
    ) throws -> (spentCount: Int, balance: UInt64) {
        guard let jsonData = jsonString.data(using: .utf8) else {
            throw SDKError.serializationError("Sync nullifiers JSON is not valid UTF-8")
        }

        guard let obj = try JSONSerialization.jsonObject(with: jsonData) as? [String: Any] else {
            throw SDKError.serializationError("Sync nullifiers JSON root is not an object")
        }

        guard let spentCountNum = obj["spentCount"] as? NSNumber else {
            throw SDKError.serializationError("Missing 'spentCount' field in sync result")
        }

        guard let balanceNum = obj["balance"] as? NSNumber else {
            throw SDKError.serializationError("Missing 'balance' field in sync result")
        }

        return (spentCount: spentCountNum.intValue, balance: balanceNum.uint64Value)
    }
}

// MARK: - Sendable SDK Pointer Wrapper

/// Sendable wrapper for the SDK handle pointer, for use in pool client async closures.
/// This is private to this file to avoid collisions with similar wrappers elsewhere.
private final class PoolClientSendableSdkPtr: @unchecked Sendable {
    let ptr: UnsafeMutablePointer<SDKHandle>
    init(_ p: UnsafeMutablePointer<SDKHandle>) { self.ptr = p }
}
