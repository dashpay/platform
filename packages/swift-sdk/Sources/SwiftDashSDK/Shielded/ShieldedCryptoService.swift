// ShieldedCryptoService.swift
// SwiftDashSDK
//
// High-level Swift API for shielded cryptographic operations.
// Provides key derivation, bundle building, and note decryption as extensions on SDK.
//
// These operations are pure cryptography -- they do NOT require an SDK handle or
// network connection. They are exposed as SDK extensions for API consistency.

import Foundation
import DashSDKFFI

// MARK: - Shielded Crypto Service

@MainActor
extension SDK {

    // MARK: - Proving Key Warmup

    /// Pre-build and cache the Halo 2 proving key.
    ///
    /// The first call takes approximately 30 seconds. Subsequent calls are instant
    /// because the key is cached in process memory. Call this on app startup to avoid
    /// latency during the first bundle building operation.
    ///
    /// - Throws: `SDKError` on failure.
    public func warmupProvingKey() async throws {
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            DispatchQueue.global().async {
                let result = dash_sdk_shielded_warmup_proving_key()

                do {
                    try self.cryptoExtractVoid(result)
                    continuation.resume()
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    // MARK: - Address Derivation

    /// Derive an Orchard payment address from a spending key.
    ///
    /// - Parameters:
    ///   - spendingKey: 32-byte spending key.
    ///   - diversifierIndex: Address diversifier index (default 0).
    /// - Returns: 43-byte raw Orchard payment address.
    /// - Throws: `SDKError` on failure.
    public func deriveShieldedAddress(
        spendingKey: Data,
        diversifierIndex: UInt32 = 0
    ) async throws -> Data {
        guard spendingKey.count == 32 else {
            throw SDKError.invalidParameter("Spending key must be exactly 32 bytes, got \(spendingKey.count)")
        }

        let skCopy = spendingKey
        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                var skTuple = dataToBytes32(skCopy)
                let result = withUnsafePointer(to: &skTuple) { skPtr in
                    dash_sdk_shielded_derive_address(skPtr, diversifierIndex)
                }

                do {
                    let hexString = try self.cryptoExtractString(result)
                    guard let addressData = hexToData(hexString) else {
                        continuation.resume(throwing: SDKError.serializationError("Invalid hex address returned"))
                        return
                    }
                    continuation.resume(returning: addressData)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    // MARK: - Bundle Building

    /// Build an output-only Orchard bundle for shielding funds.
    ///
    /// Creates a bundle with no spends -- only a single output note for the recipient
    /// (derived from the spending key at diversifier index 0).
    ///
    /// - Parameters:
    ///   - spendingKey: 32-byte spending key.
    ///   - amount: Amount in credits to shield.
    ///   - memo: Optional 36-byte memo. If nil, uses zero bytes.
    /// - Returns: `OrchardBundle` ready to pass to `shieldFunds()`.
    /// - Throws: `SDKError` on failure.
    public func buildShieldBundle(
        spendingKey: Data,
        amount: UInt64,
        memo: Data? = nil
    ) async throws -> OrchardBundle {
        guard spendingKey.count == 32 else {
            throw SDKError.invalidParameter("Spending key must be exactly 32 bytes, got \(spendingKey.count)")
        }

        if let memo = memo, memo.count != 36 {
            throw SDKError.invalidParameter("Memo must be exactly 36 bytes, got \(memo.count)")
        }

        let skCopy = spendingKey
        let memoCopy = memo
        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                var skTuple = dataToBytes32(skCopy)
                var memoTuple = dataToBytes36(memoCopy ?? Data(count: 36))

                let result = withUnsafePointer(to: &skTuple) { skPtr in
                    withUnsafePointer(to: &memoTuple) { memoPtr in
                        dash_sdk_shielded_build_shield_bundle(
                            skPtr,
                            amount,
                            memoPtr
                        )
                    }
                }

                do {
                    let jsonString = try self.cryptoExtractString(result)
                    let bundle = try parseBundleJSON(jsonString)
                    continuation.resume(returning: bundle)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// Build a spend+output bundle for shielded transfer (shielded-to-shielded).
    ///
    /// Spends existing notes and creates a new note for the recipient. Change
    /// is returned to the sender's own address (derived at index 0).
    ///
    /// - Parameters:
    ///   - spendingKey: 32-byte spending key of the sender.
    ///   - anchor: 32-byte Sinsemilla anchor of the commitment tree.
    ///   - notes: Spendable notes with Merkle authentication paths.
    ///   - recipientAddress: 43-byte raw Orchard address of the recipient.
    ///   - amount: Amount in credits to transfer.
    ///   - memo: Optional 36-byte memo. If nil, uses zero bytes.
    /// - Returns: `OrchardBundle` ready to pass to `shieldedTransfer()`.
    /// - Throws: `SDKError` on failure.
    public func buildTransferBundle(
        spendingKey: Data,
        anchor: Data,
        notes: [SpendableNoteInfo],
        recipientAddress: Data,
        amount: UInt64,
        memo: Data? = nil
    ) async throws -> OrchardBundle {
        guard spendingKey.count == 32 else {
            throw SDKError.invalidParameter("Spending key must be exactly 32 bytes, got \(spendingKey.count)")
        }
        guard anchor.count == 32 else {
            throw SDKError.invalidParameter("Anchor must be exactly 32 bytes, got \(anchor.count)")
        }
        guard recipientAddress.count == 43 else {
            throw SDKError.invalidParameter("Recipient address must be exactly 43 bytes, got \(recipientAddress.count)")
        }
        guard !notes.isEmpty else {
            throw SDKError.invalidParameter("Notes array must not be empty")
        }
        if let memo = memo, memo.count != 36 {
            throw SDKError.invalidParameter("Memo must be exactly 36 bytes, got \(memo.count)")
        }

        let skCopy = spendingKey
        let anchorCopy = anchor
        let addrCopy = recipientAddress
        let memoCopy = memo
        let notesJSON = spendableNotesToJSON(notes)

        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                var skTuple = dataToBytes32(skCopy)
                var anchorTuple = dataToBytes32(anchorCopy)
                var memoTuple = dataToBytes36(memoCopy ?? Data(count: 36))

                let result = notesJSON.withCString { notesCStr in
                    addrCopy.withUnsafeBytes { addrPtr -> DashSDKResult in
                        guard let addrBase = addrPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                            return DashSDKResult()
                        }
                        return withUnsafePointer(to: &skTuple) { skPtr in
                            withUnsafePointer(to: &anchorTuple) { anchorPtr in
                                withUnsafePointer(to: &memoTuple) { memoPtr in
                                    dash_sdk_shielded_build_transfer_bundle(
                                        skPtr,
                                        anchorPtr,
                                        notesCStr,
                                        addrBase,
                                        addrCopy.count,
                                        amount,
                                        memoPtr
                                    )
                                }
                            }
                        }
                    }
                }

                do {
                    let jsonString = try self.cryptoExtractString(result)
                    let bundle = try parseBundleJSON(jsonString)
                    continuation.resume(returning: bundle)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// Build a spend bundle for unshielding to a platform address.
    ///
    /// Spends existing notes and creates a change output back to the sender.
    /// The platform address receives funds via the state transition's transparent field.
    ///
    /// - Parameters:
    ///   - spendingKey: 32-byte spending key.
    ///   - anchor: 32-byte Sinsemilla anchor of the commitment tree.
    ///   - notes: Spendable notes with Merkle authentication paths.
    ///   - outputAddress: Platform address bytes for the unshield recipient.
    ///   - amount: Amount in credits to unshield.
    ///   - memo: Optional 36-byte memo. If nil, uses zero bytes.
    /// - Returns: `OrchardBundle` ready to pass to `unshieldFunds()`.
    /// - Throws: `SDKError` on failure.
    public func buildUnshieldBundle(
        spendingKey: Data,
        anchor: Data,
        notes: [SpendableNoteInfo],
        outputAddress: Data,
        amount: UInt64,
        memo: Data? = nil
    ) async throws -> OrchardBundle {
        guard spendingKey.count == 32 else {
            throw SDKError.invalidParameter("Spending key must be exactly 32 bytes, got \(spendingKey.count)")
        }
        guard anchor.count == 32 else {
            throw SDKError.invalidParameter("Anchor must be exactly 32 bytes, got \(anchor.count)")
        }
        guard !outputAddress.isEmpty else {
            throw SDKError.invalidParameter("Output address must not be empty")
        }
        guard !notes.isEmpty else {
            throw SDKError.invalidParameter("Notes array must not be empty")
        }
        if let memo = memo, memo.count != 36 {
            throw SDKError.invalidParameter("Memo must be exactly 36 bytes, got \(memo.count)")
        }

        let skCopy = spendingKey
        let anchorCopy = anchor
        let addrCopy = outputAddress
        let memoCopy = memo
        let notesJSON = spendableNotesToJSON(notes)

        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                var skTuple = dataToBytes32(skCopy)
                var anchorTuple = dataToBytes32(anchorCopy)
                var memoTuple = dataToBytes36(memoCopy ?? Data(count: 36))

                let result = notesJSON.withCString { notesCStr in
                    addrCopy.withUnsafeBytes { addrPtr -> DashSDKResult in
                        guard let addrBase = addrPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                            return DashSDKResult()
                        }
                        return withUnsafePointer(to: &skTuple) { skPtr in
                            withUnsafePointer(to: &anchorTuple) { anchorPtr in
                                withUnsafePointer(to: &memoTuple) { memoPtr in
                                    dash_sdk_shielded_build_unshield_bundle(
                                        skPtr,
                                        anchorPtr,
                                        notesCStr,
                                        addrBase,
                                        addrCopy.count,
                                        amount,
                                        memoPtr
                                    )
                                }
                            }
                        }
                    }
                }

                do {
                    let jsonString = try self.cryptoExtractString(result)
                    let bundle = try parseBundleJSON(jsonString)
                    continuation.resume(returning: bundle)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// Build a spend bundle for withdrawing to a Core (L1) address.
    ///
    /// Spends existing notes and creates a change output back to the sender.
    /// The output script receives funds via the state transition's withdrawal mechanism.
    ///
    /// - Parameters:
    ///   - spendingKey: 32-byte spending key.
    ///   - anchor: 32-byte Sinsemilla anchor of the commitment tree.
    ///   - notes: Spendable notes with Merkle authentication paths.
    ///   - outputScript: Core chain output script bytes.
    ///   - amount: Amount in credits to withdraw.
    ///   - memo: Optional 36-byte memo. If nil, uses zero bytes.
    ///   - coreFeePerByte: Core chain fee rate.
    ///   - pooling: Withdrawal pooling strategy.
    /// - Returns: `OrchardBundle` ready to pass to `shieldedWithdraw()`.
    /// - Throws: `SDKError` on failure.
    public func buildWithdrawalBundle(
        spendingKey: Data,
        anchor: Data,
        notes: [SpendableNoteInfo],
        outputScript: Data,
        amount: UInt64,
        memo: Data? = nil,
        coreFeePerByte: UInt32,
        pooling: WithdrawalPooling
    ) async throws -> OrchardBundle {
        guard spendingKey.count == 32 else {
            throw SDKError.invalidParameter("Spending key must be exactly 32 bytes, got \(spendingKey.count)")
        }
        guard anchor.count == 32 else {
            throw SDKError.invalidParameter("Anchor must be exactly 32 bytes, got \(anchor.count)")
        }
        guard !outputScript.isEmpty else {
            throw SDKError.invalidParameter("Output script must not be empty")
        }
        guard !notes.isEmpty else {
            throw SDKError.invalidParameter("Notes array must not be empty")
        }
        if let memo = memo, memo.count != 36 {
            throw SDKError.invalidParameter("Memo must be exactly 36 bytes, got \(memo.count)")
        }

        let skCopy = spendingKey
        let anchorCopy = anchor
        let scriptCopy = outputScript
        let memoCopy = memo
        let notesJSON = spendableNotesToJSON(notes)

        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                var skTuple = dataToBytes32(skCopy)
                var anchorTuple = dataToBytes32(anchorCopy)
                var memoTuple = dataToBytes36(memoCopy ?? Data(count: 36))

                let result = notesJSON.withCString { notesCStr in
                    scriptCopy.withUnsafeBytes { scriptPtr -> DashSDKResult in
                        guard let scriptBase = scriptPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                            return DashSDKResult()
                        }
                        return withUnsafePointer(to: &skTuple) { skPtr in
                            withUnsafePointer(to: &anchorTuple) { anchorPtr in
                                withUnsafePointer(to: &memoTuple) { memoPtr in
                                    dash_sdk_shielded_build_withdrawal_bundle(
                                        skPtr,
                                        anchorPtr,
                                        notesCStr,
                                        scriptBase,
                                        scriptCopy.count,
                                        amount,
                                        memoPtr,
                                        coreFeePerByte,
                                        pooling.rawValue
                                    )
                                }
                            }
                        }
                    }
                }

                do {
                    let jsonString = try self.cryptoExtractString(result)
                    let bundle = try parseBundleJSON(jsonString)
                    continuation.resume(returning: bundle)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    // MARK: - Note Decryption

    /// Decrypt encrypted notes using the spending key's incoming viewing key.
    ///
    /// Performs trial decryption on each note. Only notes that successfully decrypt
    /// (i.e., belong to this spending key) are included in the result.
    ///
    /// - Parameters:
    ///   - spendingKey: 32-byte spending key.
    ///   - encryptedNotes: Array of encrypted notes from the shielded pool.
    /// - Returns: Array of `DecryptedNote` for notes belonging to this key.
    /// - Throws: `SDKError` on failure.
    public func decryptNotes(
        spendingKey: Data,
        encryptedNotes: [EncryptedNote]
    ) async throws -> [DecryptedNote] {
        guard spendingKey.count == 32 else {
            throw SDKError.invalidParameter("Spending key must be exactly 32 bytes, got \(spendingKey.count)")
        }

        guard !encryptedNotes.isEmpty else {
            return []
        }

        let skCopy = spendingKey
        let notesJSON = encryptedNotesToJSON(encryptedNotes)

        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                var skTuple = dataToBytes32(skCopy)

                let result = notesJSON.withCString { notesCStr in
                    withUnsafePointer(to: &skTuple) { skPtr in
                        dash_sdk_shielded_decrypt_notes(skPtr, notesCStr)
                    }
                }

                do {
                    let jsonString = try self.cryptoExtractString(result)
                    let notes = try parseDecryptedNotesJSON(jsonString)
                    continuation.resume(returning: notes)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }
}

// MARK: - Private Helpers

extension SDK {

    /// Extract a string from a DashSDKResult. Thread-safe (no @MainActor).
    /// Used by crypto service methods that do not require an SDK handle.
    nonisolated func cryptoExtractString(_ result: DashSDKResult) throws -> String {
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

    /// Extract void (success/error) from a DashSDKResult. Thread-safe.
    /// Used by crypto service methods that do not require an SDK handle.
    nonisolated func cryptoExtractVoid(_ result: DashSDKResult) throws {
        if let error = result.error {
            let errorMessage = error.pointee.message != nil
                ? String(cString: error.pointee.message!)
                : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError(errorMessage)
        }
    }
}
