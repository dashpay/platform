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

/// What one `shieldedWalletDrainStatus` read produced.
public enum WalletDrainStatusRead: Sendable, Equatable {
    /// The read succeeded and reported this state.
    case reported(WalletDrainOutcome.State)
    /// The read threw. That is never evidence that nothing was sent.
    case failed
}

/// The one thing a host offers for a stored drain operation id.
public enum WalletDrainRecoveryAction: UInt8, Sendable, Equatable {
    /// One of the host's attempts for this operation is running.
    case inProgress = 0
    /// Resume the asset lock the last status read named, by its exact outpoint.
    case resume = 1
    /// Read the status again. Sends nothing.
    case checkAgain = 2
}

/// Where a tracked asset lock is listed, as far as wallet drains are concerned.
public enum WalletDrainLockListing: UInt8, Sendable, Equatable {
    /// Not decided here: the host's own rule for resumable locks applies.
    case unaffected = 0
    /// The host's stored-operation row owns this lock: list it nowhere else.
    case hidden = 1
    /// List it with the resumable locks, although it is only Built.
    case offered = 2
}

/// The rules a host follows for the operation ids it stores for wallet
/// drains ("Max" Core-to-shielded transfers). Every function is one call
/// into Rust (`platform_wallet::wallet::shielded::wallet_drain_recovery`),
/// which holds the rules, so the Swift and Kotlin apps cannot drift apart:
/// the caller gathers the facts it owns and does what these return.
///
/// None of them reads a wallet or touches the network, and none throws. The
/// FFI fails only for a raw value these typed arguments cannot produce; a
/// failure returns the fail-closed answer the FFI documents.
public enum WalletDrainRules {
    /// Whether the host keeps the operation id it stored after `read`
    /// (`true`) or removes it (`false`). `attemptOverlappedRead` says whether
    /// one of the host's attempts for the id ran or started while that read
    /// was in progress. Rust holds the rule; a failed call keeps the id.
    public static func keepsStoredId(after read: WalletDrainStatusRead, attemptOverlappedRead: Bool) -> Bool {
        let (succeeded, state) = ffiRead(read)
        var keep = true
        let result = platform_wallet_wallet_drain_marker_decision(
            succeeded, state, attemptOverlappedRead, &keep)
        return PlatformWalletResult(result).isSuccess ? keep : true
    }

    /// What the host offers for a stored operation id. `lastRead` is `nil`
    /// before any status read has finished; `readNamedAssetLock` says
    /// whether that read's outcome carried an outpoint. Rust holds the rule
    /// and returns `.resume` only for a `pending` report that named a lock;
    /// a failed call returns `.checkAgain`.
    public static func recoveryAction(
        attemptInFlight: Bool,
        lastRead: WalletDrainStatusRead?,
        readNamedAssetLock: Bool
    ) -> WalletDrainRecoveryAction {
        let (succeeded, state) = lastRead.map(ffiRead) ?? (false, 0)
        var raw = WalletDrainRecoveryAction.checkAgain.rawValue
        let result = platform_wallet_wallet_drain_recovery_action(
            attemptInFlight, lastRead != nil, succeeded, state, readNamedAssetLock, &raw)
        guard PlatformWalletResult(result).isSuccess else { return .checkAgain }
        // An action this build does not know never offers a spend.
        return WalletDrainRecoveryAction(rawValue: raw) ?? .checkAgain
    }

    /// Whether a returned drain attempt in `state` needs nothing more. An
    /// attempt that is not finished is shown as failed, with the error its
    /// call returned. Rust holds the rule; a failed call returns `false`.
    public static func attemptFinished(_ state: WalletDrainOutcome.State) -> Bool {
        var finished = false
        let result = platform_wallet_wallet_drain_attempt_finished(state.rawValue, &finished)
        return PlatformWalletResult(result).isSuccess ? finished : false
    }

    /// The state the host reports for an operation whose status read returned
    /// `reported`. `hostCallActive` says whether one of the host's own drain
    /// calls for the same operation id has not returned yet. Rust holds the
    /// rule; a failed call or a state this build does not know returns
    /// `.pending`.
    public static func statusDuringHostCall(
        _ reported: WalletDrainOutcome.State,
        hostCallActive: Bool
    ) -> WalletDrainOutcome.State {
        var raw = WalletDrainOutcome.State.pending.rawValue
        let result = platform_wallet_wallet_drain_status_during_host_call(
            reported.rawValue, hostCallActive, &raw)
        guard PlatformWalletResult(result).isSuccess else { return .pending }
        return WalletDrainOutcome.State(rawValue: raw) ?? .pending
    }

    /// Whether a drain may be started after a preview with `status`. Rust
    /// holds the rule; a failed call returns `false`.
    public static func previewAllowsStart(_ status: WalletDrainPreview.Status) -> Bool {
        var allowed = false
        let result = platform_wallet_wallet_drain_preview_allows_start(status.rawValue, &allowed)
        return PlatformWalletResult(result).isSuccess ? allowed : false
    }

    /// Whether a NEW drain is offered on `network`. Recovering an existing
    /// one is offered on every network and does not ask this. Rust holds the
    /// rule.
    public static func startOffered(on network: Network) -> Bool {
        platform_wallet_wallet_drain_start_offered(network.ffiValue)
    }

    /// Where the host lists a tracked asset lock with these stored values,
    /// or `nil` when Rust refuses `fundingTypeRaw` or `statusRaw` as outside
    /// the funding types and statuses it knows (a value that does not fit a
    /// byte is refused the same way). `hasOperationId` says whether the lock
    /// carries a valid operation id, `hostHoldsStoredId` whether the host
    /// holds a stored operation id equal to it. Rust holds the rule.
    public static func lockListingIfKnown(
        fundingTypeRaw: Int,
        statusRaw: Int,
        hasOperationId: Bool,
        hostHoldsStoredId: Bool
    ) -> WalletDrainLockListing? {
        guard let fundingType = UInt8(exactly: fundingTypeRaw),
              let status = UInt8(exactly: statusRaw)
        else { return nil }
        var raw = WalletDrainLockListing.unaffected.rawValue
        let result = platform_wallet_wallet_drain_lock_listing(
            fundingType, status, hasOperationId, hostHoldsStoredId, &raw)
        guard PlatformWalletResult(result).isSuccess else { return nil }
        return WalletDrainLockListing(rawValue: raw) ?? .unaffected
    }

    /// `lockListingIfKnown`, with the FFI's fail-closed `.unaffected` for a
    /// funding type or status Rust refuses.
    public static func lockListing(
        fundingTypeRaw: Int,
        statusRaw: Int,
        hasOperationId: Bool,
        hostHoldsStoredId: Bool
    ) -> WalletDrainLockListing {
        lockListingIfKnown(
            fundingTypeRaw: fundingTypeRaw,
            statusRaw: statusRaw,
            hasOperationId: hasOperationId,
            hostHoldsStoredId: hostHoldsStoredId
        ) ?? .unaffected
    }

    /// `read` as the FFI's `(read_succeeded, reported_state)` pair. The state
    /// byte is ignored for a failed read.
    private static func ffiRead(_ read: WalletDrainStatusRead) -> (Bool, UInt8) {
        switch read {
        case .reported(let state):
            return (true, state.rawValue)
        case .failed:
            return (false, 0)
        }
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
        return try await withAdmittedNativeOperation("shieldedWalletDrainPreview") { handle in
            try Self.withWalletDrainID(walletId) { walletPtr in
                var preview = WalletDrainPreviewFFI()
                try platform_wallet_manager_shielded_wallet_drain_preview(
                    handle, walletPtr, fundingAccountIndex, &preview
                ).check()
                return try WalletDrainPreview(ffi: preview)
            }
        }
    }

    /// Drain the permitted transparent accounts into a single Orchard recipient.
    /// Persist `operationId` before calling. A repeated UUID identifies the same
    /// operation and must never be replaced with a new UUID after an unknown result.
    /// The SDK determines the amount and durably associates the lock before dispatch.
    public func shieldedFundFromWalletDrain(
        walletId: Data,
        operationId: UUID,
        fundingAccountIndex: UInt32 = 0,
        recipientRaw43: Data
    ) async -> WalletDrainOutcome {
        do {
            try walletDrainReadPreflight(walletId: walletId)
            guard recipientRaw43.count == 43 else {
                throw PlatformWalletError.invalidParameter(
                    "recipientRaw43 must be exactly 43 bytes (was \(recipientRaw43.count))")
            }
            try admitNativeOp("shieldedFundFromWalletDrain")
        } catch {
            // The ID may belong to an earlier call. Local validation cannot
            // establish that the persisted operation was never dispatched.
            return WalletDrainOutcome(
                state: .pending, outpoint: nil, error: Self.walletDrainError(error))
        }

        defer { finishNativeOp() }
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
        let coreSigner = MnemonicResolver()
        return await Task.detached(priority: .userInitiated) {
            do {
                return try Self.withWalletDrainIDs(walletId, operationId: operationId) { walletPtr, operationPtr in
                    try recipientRaw43.withUnsafeBytes { raw in
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
        let callActive = Self.walletDrainCalls[callIdentity] != nil
        let native = try await withAdmittedNativeOperation("shieldedWalletDrainStatus") { handle in
            try Self.withWalletDrainIDs(walletId, operationId: operationId) { walletPtr, operationPtr in
                var outcome = WalletDrainOutcomeFFI()
                outcome.state = WalletDrainOutcome.State.pending.rawValue
                try platform_wallet_manager_shielded_wallet_drain_status(
                    handle, walletPtr, operationPtr, &outcome
                ).check()
                return WalletDrainOutcome(ffi: outcome)
            }
        }
        return callActive ? Self.walletDrainStatusDuringActiveCall(native: native) : native
    }

    /// The native snapshot as reported while one of this host's drain calls
    /// for the same operation id is active. Rust decides the state
    /// (`WalletDrainRules.statusDuringHostCall`); when it keeps the native
    /// state, the snapshot is returned unchanged, otherwise the answered state
    /// is returned with the native outpoint.
    nonisolated static func walletDrainStatusDuringActiveCall(native: WalletDrainOutcome) -> WalletDrainOutcome {
        let state = WalletDrainRules.statusDuringHostCall(native.state, hostCallActive: true)
        return state == native.state ? native : WalletDrainOutcome(state: state, outpoint: native.outpoint)
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
