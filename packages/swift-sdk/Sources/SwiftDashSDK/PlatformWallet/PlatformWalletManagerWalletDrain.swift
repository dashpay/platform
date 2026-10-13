import Foundation
import DashSDKFFI

private struct WalletDrainCallIdentity: Hashable {
    let manager: Handle
    let walletId: Data
    let operationId: UUID
}

/// A read-only Core-to-shielded Max estimate. Candidate balances include
/// reservations held by other operations, so this is never a guaranteed minimum.
public struct WalletDrainPreview: Sendable, Equatable {
    public enum Status: UInt8, Sendable {
        case ready = 0
        case insufficientFunds = 1
        /// The optimistic candidate count exceeds the one-transaction limit.
        /// Execution may still fit after excluding another operation's reservations.
        case inputLimitMayBeExceeded = 2
    }

    public let candidateInputCount: UInt64
    public let candidateBalanceDuffs: UInt64
    public let poolFeeCredits: UInt64
    public let estimatedL1FeeDuffs: UInt64?
    public let estimatedRecipientCredits: UInt64?
    public let status: Status

    public init(
        candidateInputCount: UInt64,
        candidateBalanceDuffs: UInt64,
        poolFeeCredits: UInt64,
        estimatedL1FeeDuffs: UInt64?,
        estimatedRecipientCredits: UInt64?,
        status: Status
    ) {
        self.candidateInputCount = candidateInputCount
        self.candidateBalanceDuffs = candidateBalanceDuffs
        self.poolFeeCredits = poolFeeCredits
        self.estimatedL1FeeDuffs = estimatedL1FeeDuffs
        self.estimatedRecipientCredits = estimatedRecipientCredits
        self.status = status
    }

    init(ffi: WalletDrainPreviewFFI) throws {
        guard let status = Status(rawValue: ffi.status) else {
            throw PlatformWalletError.deserialization("Unknown wallet drain preview status: \(ffi.status)")
        }
        self.init(
            candidateInputCount: ffi.candidate_input_count,
            candidateBalanceDuffs: ffi.candidate_balance_duffs,
            poolFeeCredits: ffi.pool_fee_credits,
            estimatedL1FeeDuffs: ffi.has_estimated_l1_fee_duffs ? ffi.estimated_l1_fee_duffs : nil,
            estimatedRecipientCredits: ffi.has_estimated_recipient_credits ? ffi.estimated_recipient_credits : nil,
            status: status
        )
    }
}

/// Exact asset-lock identity, using the same raw txid byte order as the resume API.
public struct WalletDrainOutpoint: Sendable, Equatable {
    public let txidWire: Data
    public let vout: UInt32

    public init(txidWire: Data, vout: UInt32) {
        self.txidWire = txidWire
        self.vout = vout
    }
}

/// State and error belong to one persisted operation UUID. A pending outcome
/// must be reconciled by that UUID or resumed by its outpoint; never start a
/// replacement drain because a request timed out or the outpoint is absent.
public struct WalletDrainOutcome: Sendable {
    public enum State: UInt8, Sendable {
        case notSent = 0
        case pending = 1
        case completed = 2
    }

    public let state: State
    public let outpoint: WalletDrainOutpoint?
    public let error: PlatformWalletError?

    public init(state: State, outpoint: WalletDrainOutpoint?, error: PlatformWalletError? = nil) {
        self.state = state
        self.outpoint = outpoint
        self.error = error
    }

    init(ffi: WalletDrainOutcomeFFI, error: PlatformWalletError? = nil) {
        var rawOutpoint = ffi.out_point
        let outpoint = ffi.has_out_point
            ? WalletDrainOutpoint(
                txidWire: withUnsafeBytes(of: &rawOutpoint.txid) { Data($0) },
                vout: rawOutpoint.vout)
            : nil
        guard let state = State(rawValue: ffi.state) else {
            self.init(
                state: .pending,
                outpoint: outpoint,
                error: error ?? .deserialization("Unknown wallet drain outcome state: \(ffi.state)")
            )
            return
        }
        self.init(state: state, outpoint: outpoint, error: error)
    }
}

extension PlatformWalletManager {
    // MainActor-isolated, covering the interval before detached execution has
    // entered the Rust operation registry. Count duplicate UUID callers so one
    // returning call cannot clear another caller's protection.
    private static var walletDrainCalls: [WalletDrainCallIdentity: Int] = [:]

    /// Preview a whole-wallet transparent drain without reserving inputs,
    /// deriving addresses, signing, or broadcasting. Display amounts as estimates.
    public func shieldedWalletDrainPreview(
        walletId: Data,
        fundingAccountIndex: UInt32 = 0
    ) async throws -> WalletDrainPreview {
        try walletDrainReadPreflight(walletId: walletId)
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) {
            try Self.withWalletDrainID(walletId) { walletPtr in
                var preview = WalletDrainPreviewFFI()
                try platform_wallet_manager_shielded_wallet_drain_preview(
                    handle, walletPtr, fundingAccountIndex, &preview
                ).check()
                return try WalletDrainPreview(ffi: preview)
            }
        }.value
    }

    /// Drain the permitted transparent accounts into a single Orchard recipient.
    /// Persist `operationId` before calling. A repeated UUID identifies the same
    /// operation and must never be replaced with a new UUID after an unknown result.
    /// The SDK determines the amount and durably associates the lock before dispatch.
    public func shieldedFundFromWalletDrain(
        walletId: Data,
        operationId: UUID,
        fundingAccountIndex: UInt32 = 0,
        recipients: [ShieldedFundFromAssetLockRecipient]
    ) async -> WalletDrainOutcome {
        do {
            try shieldedFundFromAssetLockPreflight(walletId: walletId, recipients: recipients)
        } catch {
            // The ID may belong to an earlier call. Local validation cannot
            // establish that the persisted operation was never dispatched.
            return WalletDrainOutcome(
                state: .pending, outpoint: nil, error: Self.walletDrainError(error))
        }

        let handle = self.handle
        let callIdentity = WalletDrainCallIdentity(
            manager: handle, walletId: walletId, operationId: operationId)
        Self.walletDrainCalls[callIdentity, default: 0] += 1
        defer {
            if let count = Self.walletDrainCalls[callIdentity], count > 1 {
                Self.walletDrainCalls[callIdentity] = count - 1
            } else {
                Self.walletDrainCalls.removeValue(forKey: callIdentity)
            }
        }
        let recipient = recipients[0].recipientRaw43
        let coreSigner = MnemonicResolver()
        return await Task.detached(priority: .userInitiated) {
            do {
                return try Self.withWalletDrainIDs(walletId, operationId: operationId) { walletPtr, operationPtr in
                    try recipient.withUnsafeBytes { raw in
                        guard let recipientPtr = raw.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                            throw PlatformWalletError.invalidParameter("recipient baseAddress is nil")
                        }
                        var outcome = WalletDrainOutcomeFFI()
                        // Fail closed even if a future FFI returns before initializing it.
                        outcome.state = WalletDrainOutcome.State.pending.rawValue
                        let ffiResult = withExtendedLifetime(coreSigner) {
                            platform_wallet_manager_shielded_fund_from_wallet_drain(
                                handle, walletPtr, operationPtr, fundingAccountIndex,
                                recipientPtr, coreSigner.handle, &outcome
                            )
                        }
                        let result = PlatformWalletResult(ffiResult)
                        return WalletDrainOutcome(
                            ffi: outcome,
                            error: result.isSuccess ? nil : PlatformWalletError(result: result)
                        )
                    }
                }
            } catch {
                // These guards precede this call, but cannot disprove an
                // earlier dispatch under the same operation UUID.
                return WalletDrainOutcome(
                    state: .pending, outpoint: nil, error: Self.walletDrainError(error))
            }
        }.value
    }

    /// Read one operation by its UUID, including after restart. A durable-state
    /// read failure throws; unavailable completion proofs leave it pending.
    /// Neither is evidence that the operation was not sent.
    /// Reconciliation may query authenticated network state and never broadcasts.
    public func shieldedWalletDrainStatus(
        walletId: Data,
        operationId: UUID
    ) async throws -> WalletDrainOutcome {
        try walletDrainReadPreflight(walletId: walletId)
        let handle = self.handle
        let callIdentity = WalletDrainCallIdentity(
            manager: handle, walletId: walletId, operationId: operationId)
        if Self.walletDrainCalls[callIdentity] != nil {
            return WalletDrainOutcome(state: .pending, outpoint: nil)
        }
        return try await Task.detached(priority: .userInitiated) {
            try Self.withWalletDrainIDs(walletId, operationId: operationId) { walletPtr, operationPtr in
                var outcome = WalletDrainOutcomeFFI()
                outcome.state = WalletDrainOutcome.State.pending.rawValue
                try platform_wallet_manager_shielded_wallet_drain_status(
                    handle, walletPtr, operationPtr, &outcome
                ).check()
                return WalletDrainOutcome(ffi: outcome)
            }
        }.value
    }

    private func walletDrainReadPreflight(walletId: Data) throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter("walletId must be exactly 32 bytes (was \(walletId.count))")
        }
    }

    nonisolated private static func walletDrainError(_ error: Error) -> PlatformWalletError {
        error as? PlatformWalletError ?? .walletOperation(error.localizedDescription)
    }

    nonisolated private static func withWalletDrainID<R>(
        _ walletId: Data,
        _ body: (UnsafePointer<UInt8>) throws -> R
    ) throws -> R {
        try walletId.withUnsafeBytes { raw in
            guard let walletPtr = raw.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
            }
            return try body(walletPtr)
        }
    }

    nonisolated private static func withWalletDrainIDs<R>(
        _ walletId: Data,
        operationId: UUID,
        _ body: (UnsafePointer<UInt8>, UnsafePointer<UInt8>) throws -> R
    ) throws -> R {
        try withWalletDrainID(walletId) { walletPtr in
            var uuid = operationId.uuid
            return try withUnsafeBytes(of: &uuid) { raw in
                guard let operationPtr = raw.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                    throw PlatformWalletError.invalidParameter("operationId baseAddress is nil")
                }
                return try body(walletPtr, operationPtr)
            }
        }
    }
}
