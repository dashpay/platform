import Foundation
import DashSDKFFI

/// Recipient entry for `shieldedFundFromAssetLock(...)`.
///
/// The Rust-side API today enforces exactly one recipient with
/// `nil` credits (= "remainder" semantics — receives
/// `lock_value − protocol_min_fee`). Type 18's Orchard bundle
/// builder is single-output; multi-recipient with explicit per-
/// recipient amounts lands when DPP grows multi-output bundles.
/// The list shape is exposed here so the call site doesn't have
/// to change when that happens.
public struct ShieldedFundFromAssetLockRecipient: Sendable {
    /// Raw 43-byte Orchard payment address (11-byte diversifier +
    /// 32-byte pk_d). Same shape
    /// `platform_wallet_manager_shielded_default_address` returns
    /// and `shieldedTransfer` consumes.
    public let recipientRaw43: Data

    /// Explicit credit amount this recipient receives, or `nil`
    /// for the "remainder" semantics (the recipient gets
    /// `lock_value − min_fee`). Today the wallet enforces `nil`
    /// for the single-recipient case; explicit `Some(_)` values
    /// will be honored once DPP grows multi-output Orchard
    /// bundles for Type 18.
    public let credits: UInt64?

    public init(recipientRaw43: Data, credits: UInt64? = nil) {
        self.recipientRaw43 = recipientRaw43
        self.credits = credits
    }
}

/// Live progress for `seedShieldedPoolNotes(...)`, emitted once before and
/// once after each `ShieldFromAssetLock` batch.
public struct SeedShieldedPoolProgress: Sendable {
    /// 0-based index of the batch about to run / just completed.
    public let batchIndex: UInt64
    /// Estimated total number of batches to reach `target` from the count
    /// observed when seeding started. An estimate only.
    public let batchesTotalEstimate: UInt64
    /// Pool note count observed at this checkpoint.
    public let poolNotesNow: UInt64
    /// The target total note count seeding is driving toward.
    public let target: UInt64
}

/// Box that carries the host's progress handler across the C ABI as an
/// opaque context pointer. Retained for the duration of the FFI call via
/// `Unmanaged.passRetained` and released in the calling Swift frame.
///
/// `@unchecked Sendable`: the only stored value is an `@Sendable` closure,
/// and the box itself is constructed and consumed entirely inside the
/// off-actor detached task that drives the FFI call.
private final class SeedPoolProgressBox: @unchecked Sendable {
    let handler: @Sendable (SeedShieldedPoolProgress) -> Void
    init(_ handler: @escaping @Sendable (SeedShieldedPoolProgress) -> Void) {
        self.handler = handler
    }
}

/// C trampoline matching the Rust
/// `platform_wallet_manager_shielded_seed_pool_notes` progress callback.
/// Re-materializes the `SeedPoolProgressBox` from the opaque context and
/// forwards the counters. Called from a background worker thread.
private func seedPoolProgressTrampoline(
    context: UnsafeMutableRawPointer?,
    batchIndex: UInt64,
    batchesTotalEstimate: UInt64,
    poolNotesNow: UInt64,
    target: UInt64
) {
    guard let context else { return }
    let box = Unmanaged<SeedPoolProgressBox>.fromOpaque(context).takeUnretainedValue()
    box.handler(
        SeedShieldedPoolProgress(
            batchIndex: batchIndex,
            batchesTotalEstimate: batchesTotalEstimate,
            poolNotesNow: poolNotesNow,
            target: target
        )
    )
}

extension PlatformWalletManager {
    /// Fund the shielded pool from a Core L1 asset lock, orchestrated
    /// entirely on the Rust side (build asset-lock tx → wait for
    /// IS-lock or fall back to ChainLock → submit
    /// `ShieldFromAssetLockTransition` → consume the lock on
    /// success). The asset-lock private key never crosses the FFI
    /// boundary — both Core-side derivation and the outer ST
    /// signature route through a local `MnemonicResolver`.
    ///
    /// Mirrors `ManagedPlatformAddressWallet.fundFromAssetLock` for
    /// platform-address funding, with two differences specific to
    /// the shielded pool:
    ///
    /// 1. The recipient list is **single-entry today** (Type 18's
    ///    Orchard bundle builder is single-output; the multi-output
    ///    case is a deferred DPP change). The preflight rejects any
    ///    other length with a typed error.
    /// 2. There is no immediate balance changeset to return — the
    ///    new shielded note arrives via the next sync, not from
    ///    the broadcast call. The function returns `Void` on
    ///    success.
    ///
    /// - Parameters:
    ///   - walletId: 32-byte wallet identifier (the same key
    ///     `bindShielded` uses to look up the bound subwallet).
    ///   - fundingAccountIndex: BIP44 Core account whose UTXOs fund
    ///     the asset lock.
    ///   - amountDuffs: L1 amount to lock in Core duffs. Must be
    ///     large enough to cover `recipients.credits + Platform fee`;
    ///     undersized locks fail at Platform submission.
    ///   - recipients: Destination addresses with explicit credit
    ///     amounts (exactly one entry today; preflight rejects
    ///     empty or multi-recipient lists). Each recipient's
    ///     `credits` becomes the Orchard `value_balance` for that
    ///     output.
    ///   - surplusOutput: Optional platform address (raw 21-byte
    ///     `PlatformAddress` storage bytes: 1-byte variant tag +
    ///     20-byte hash) to receive the asset-lock surplus
    ///     (`lock_value − shield_amount − pool_fee`). Pass `nil` for
    ///     none. In today's single-recipient "remainder" flow the
    ///     surplus is structurally zero (the recipient receives
    ///     `lock_value − pool_fee`), so `nil` is always valid; the
    ///     parameter is exposed for parity with the Rust builder and
    ///     forward-compatibility with multi-output bundles.
    /// - Parameter fundingPath: optional UTF-8 BIP32 derivation-path string
    ///   (dashpay/platform#4184) naming the single funds account whose UTXOs
    ///   fund the lock. `nil` (default) funds from the unmixed BIP44 account at
    ///   `fundingAccountIndex`. Pass an explicit account-level path (e.g. the
    ///   DIP-9 CoinJoin account path, `"m/44'/5'/…"`) to fund strictly from that
    ///   one account — to shield previously-mixed CoinJoin coins, for instance.
    ///   There is no union across accounts and no consent gate: exactly one
    ///   funding source participates, and if it cannot cover the lock the call
    ///   fails with `.assetLockInsufficientFunds`. A malformed path or one that
    ///   is not valid UTF-8 surfaces as `.invalidParameter`.
    public func shieldedFundFromAssetLock(
        walletId: Data,
        fundingAccountIndex: UInt32,
        amountDuffs: UInt64,
        recipients: [ShieldedFundFromAssetLockRecipient],
        surplusOutput: Data? = nil,
        fundingPath: String? = nil
    ) async throws {
        try shieldedFundFromAssetLockPreflight(
            walletId: walletId,
            recipients: recipients
        )

        let handle = self.handle
        let recipientRaw43 = recipients[0].recipientRaw43
        // Constructed on the calling actor so it lives for the
        // entire detached Task. Released after `withExtendedLifetime`
        // returns. See `ManagedPlatformAddressWallet.fundFromAssetLock`
        // for the rationale on why a bare `_ = coreSigner` is NOT
        // a substitute — the -O optimizer can elide the discard
        // and drop the resolver mid-FFI-call, leading to a UAF in
        // the vtable callback.
        let coreSigner = MnemonicResolver()

        try await Task.detached(priority: .userInitiated) {
            try walletId.withUnsafeBytes { widRaw in
                guard
                    let widPtr = widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                else {
                    throw PlatformWalletError.invalidParameter(
                        "walletId baseAddress is nil"
                    )
                }
                try recipientRaw43.withUnsafeBytes { recipientRaw in
                    guard
                        let recipientPtr = recipientRaw.baseAddress?
                            .assumingMemoryBound(to: UInt8.self)
                    else {
                        throw PlatformWalletError.invalidParameter(
                            "recipient baseAddress is nil"
                        )
                    }
                    try Self.withOptionalSurplusOutput(surplusOutput) { surplusPtr, surplusLen in
                        try Self.withOptionalFundingPath(fundingPath) { fundingPathPtr, fundingPathLen in
                            let result = withExtendedLifetime(coreSigner) {
                                platform_wallet_manager_shielded_fund_from_asset_lock(
                                    handle,
                                    widPtr,
                                    fundingAccountIndex,
                                    amountDuffs,
                                    recipientPtr,
                                    surplusPtr,
                                    surplusLen,
                                    coreSigner.handle,
                                    fundingPathPtr,
                                    fundingPathLen
                                )
                            }
                            try result.check()
                        }
                    }
                }
            }
        }.value
    }

    /// Resume a stuck shielded fund-from-asset-lock from an
    /// already-tracked outpoint.
    ///
    /// Sibling to [`shieldedFundFromAssetLock`]: the wallet-balance
    /// variant builds a fresh asset-lock transaction; this variant
    /// picks up a lock that's already tracked
    /// (`Broadcast` / `InstantSendLocked` / `ChainLocked`) and
    /// drives whatever stages remain. Use case mirrors the
    /// platform-address resume path — a prior attempt left the lock
    /// in storage but the shield ST never landed, and the user
    /// picks the lock from a "Resumable Funding" surface.
    ///
    /// - Parameters:
    ///   - walletId: 32-byte wallet identifier.
    ///   - outPointTxid: 32-byte raw txid (little-endian wire
    ///     order, same as `OutPointFFI.txid`).
    ///   - outPointVout: Funding output index (always 0 for asset
    ///     locks built by this wallet, but kept for generality).
    ///   - recipients: Destination addresses (single-entry today;
    ///     same preflight as the fresh-build variant).
    ///   - surplusOutput: Optional surplus-output platform address —
    ///     see `shieldedFundFromAssetLock`. The surplus is structurally
    ///     zero in this flow and the Rust side re-derives an identical
    ///     `shield_amount` on every attempt, so a resume cannot desync
    ///     the surplus destination regardless of this value; pass the
    ///     same value used on the original attempt (typically `nil`).
    public func shieldedResumeFundFromAssetLock(
        walletId: Data,
        outPointTxid: Data,
        outPointVout: UInt32,
        recipients: [ShieldedFundFromAssetLockRecipient],
        surplusOutput: Data? = nil
    ) async throws {
        guard outPointTxid.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "outPointTxid must be exactly 32 bytes (was \(outPointTxid.count))"
            )
        }
        try shieldedFundFromAssetLockPreflight(
            walletId: walletId,
            recipients: recipients
        )

        let handle = self.handle
        let recipientRaw43 = recipients[0].recipientRaw43
        let coreSigner = MnemonicResolver()

        try await Task.detached(priority: .userInitiated) {
            var txidTuple:
                (
                    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
                ) = (
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
                )
            outPointTxid.withUnsafeBytes { src in
                Swift.withUnsafeMutableBytes(of: &txidTuple) { dst in
                    dst.copyMemory(from: src)
                }
            }
            var outPoint = OutPointFFI(txid: txidTuple, vout: outPointVout)

            try walletId.withUnsafeBytes { widRaw in
                guard
                    let widPtr = widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                else {
                    throw PlatformWalletError.invalidParameter(
                        "walletId baseAddress is nil"
                    )
                }
                try recipientRaw43.withUnsafeBytes { recipientRaw in
                    guard
                        let recipientPtr = recipientRaw.baseAddress?
                            .assumingMemoryBound(to: UInt8.self)
                    else {
                        throw PlatformWalletError.invalidParameter(
                            "recipient baseAddress is nil"
                        )
                    }
                    try Self.withOptionalSurplusOutput(surplusOutput) { surplusPtr, surplusLen in
                        let result = withExtendedLifetime(coreSigner) {
                            platform_wallet_manager_shielded_resume_fund_from_asset_lock(
                                handle,
                                widPtr,
                                &outPoint,
                                recipientPtr,
                                surplusPtr,
                                surplusLen,
                                coreSigner.handle
                            )
                        }
                        try result.check()
                    }
                }
            }
        }.value
    }

    /// Seed the shielded pool's anonymity set up to `targetTotalNotes`
    /// by submitting a series of `ShieldFromAssetLock` (Type 18) batches,
    /// each adding up to 6 notes (1 real note to the wallet's own default
    /// shielded address + up to 5 zero-value anonymity-set fillers). 6 is
    /// `MAX_ACTIONS_PER_BATCH` in rs-platform-wallet's `seed_pool.rs` —
    /// the most that fits the 20 KiB `max_state_transition_size`, NOT the
    /// 16-action consensus cap.
    ///
    /// **Devnet/testnet only** — the Rust side hard-errors on mainnet
    /// (`Network.mainnet`). It exists so a freshly-reset devnet can satisfy
    /// the 250-note outgoing-transition minimum from the example app in one
    /// action, without a `DRIVE_SHIELDED_SNAPSHOT` genesis ingest.
    ///
    /// Batches run serially and each waits for proven execution, so a
    /// 250-note seed is ~42 batches and can take an hour or more. `progress`
    /// is invoked before and after each batch with the live counters; it is
    /// called from a background worker thread, so hop to your own UI executor
    /// inside the handler if you touch UI state.
    ///
    /// - Parameters:
    ///   - walletId: 32-byte wallet identifier (the same key `bindShielded`
    ///     uses). Must match the wallet that funds the seeding.
    ///   - account: shielded BIP44 account whose default address receives
    ///     each batch's real note (must be bound).
    ///   - targetTotalNotes: drive the on-chain pool note count up to (at
    ///     least) this value. A no-op if the pool already has this many.
    ///   - fundingAccountIndex: Core BIP44 account whose UTXOs fund each
    ///     per-batch asset lock.
    ///   - progress: optional live-progress handler (see above).
    public func seedShieldedPoolNotes(
        walletId: Data,
        account: UInt32 = 0,
        targetTotalNotes: UInt64 = 250,
        fundingAccountIndex: UInt32 = 0,
        progress: (@Sendable (SeedShieldedPoolProgress) -> Void)? = nil
    ) async throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be exactly 32 bytes (was \(walletId.count))"
            )
        }

        let handle = self.handle

        try await Task.detached(priority: .userInitiated) {
            // Constructed inside the detached task so nothing crosses back
            // to the main actor. The `MnemonicResolver` and the progress
            // box live only for this off-actor frame (same rationale as
            // `shieldedFundFromAssetLock`'s resolver).
            let coreSigner = MnemonicResolver()
            // Box the progress handler (if any) so it crosses the C ABI as
            // an opaque context. Retained for the FFI call, released after.
            let progressBox = progress.map { SeedPoolProgressBox($0) }

            return try walletId.withUnsafeBytes { widRaw in
                guard
                    let widPtr = widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                else {
                    throw PlatformWalletError.invalidParameter(
                        "walletId baseAddress is nil"
                    )
                }

                let ctx: UnsafeMutableRawPointer? = progressBox.map {
                    Unmanaged.passRetained($0).toOpaque()
                }
                defer {
                    if let ctx { Unmanaged<SeedPoolProgressBox>.fromOpaque(ctx).release() }
                }

                let result = withExtendedLifetime(coreSigner) {
                    platform_wallet_manager_shielded_seed_pool_notes(
                        handle,
                        widPtr,
                        account,
                        targetTotalNotes,
                        fundingAccountIndex,
                        coreSigner.handle,
                        progressBox == nil ? nil : seedPoolProgressTrampoline,
                        ctx
                    )
                }
                try result.check()
            }
        }.value
    }

    /// Validate the recipient list before the FFI sees it. The Rust
    /// side enforces the same invariants — duplicating them here
    /// produces a synchronous, type-specific error before paying
    /// for the `Task.detached` setup, the resolver allocation, and
    /// the (potentially long-running) Halo 2 proof build.
    private func shieldedFundFromAssetLockPreflight(
        walletId: Data,
        recipients: [ShieldedFundFromAssetLockRecipient]
    ) throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be exactly 32 bytes (was \(walletId.count))"
            )
        }
        guard !recipients.isEmpty else {
            throw PlatformWalletError.invalidParameter("recipients is empty")
        }
        // TODO(multi-output): drop this once DPP grows multi-output
        // Orchard bundles for Type 18 (today
        // `build_output_only_bundle` is single-output and is shared
        // with the Shield Type 15 path, so lifting it on Type 18
        // would also affect Type 15).
        guard recipients.count == 1 else {
            throw PlatformWalletError.invalidParameter(
                "shieldedFundFromAssetLock currently supports exactly one recipient "
                    + "(multi-output Orchard bundles for Type 18 are pending in DPP); got \(recipients.count)"
            )
        }
        for r in recipients {
            guard r.recipientRaw43.count == 43 else {
                throw PlatformWalletError.invalidParameter(
                    "ShieldedFundFromAssetLockRecipient.recipientRaw43 must be exactly 43 bytes (got \(r.recipientRaw43.count))"
                )
            }
            // TODO(multi-output): drop this once DPP grows multi-output
            // Orchard bundles for Type 18 and honors explicit `Some(_)`
            // recipient credits. Today the wallet rejects explicit
            // amounts (the single recipient receives `lock_value - min_fee`),
            // so we catch it here before paying for the FFI roundtrip.
            guard r.credits == nil else {
                throw PlatformWalletError.invalidParameter(
                    "ShieldedFundFromAssetLockRecipient.credits must be nil today "
                        + "(the single recipient receives lock_value - protocol min fee). "
                        + "Explicit amounts will be honored when DPP grows multi-output bundles for Type 18."
                )
            }
        }
    }

    /// Run `body` with a `(pointer, length)` view of the optional
    /// surplus-output address bytes.
    ///
    /// `nil` (or empty) yields `(nil, 0)`, which the FFI reads as "no
    /// surplus output". A non-nil value is pinned for the call via
    /// `withUnsafeBytes` so the pointer is valid for the duration of
    /// `body`. The FFI expects raw `PlatformAddress` storage bytes
    /// (1-byte variant tag + 20-byte hash); it validates the encoding
    /// and returns an error for malformed input, so no length check is
    /// duplicated here.
    ///
    /// `nonisolated` because `PlatformWalletManager` is
    /// `@MainActor`-isolated by default and the call sites run inside
    /// the synchronous, off-main-actor `Task.detached` bodies — this is
    /// pure byte marshalling that reads no `PlatformWalletManager` state.
    nonisolated private static func withOptionalSurplusOutput<R>(
        _ surplusOutput: Data?,
        _ body: (UnsafePointer<UInt8>?, UInt) throws -> R
    ) throws -> R {
        guard let surplusOutput, !surplusOutput.isEmpty else {
            return try body(nil, 0)
        }
        return try surplusOutput.withUnsafeBytes { raw in
            guard let ptr = raw.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                throw PlatformWalletError.invalidParameter(
                    "surplusOutput baseAddress is nil"
                )
            }
            return try body(ptr, UInt(raw.count))
        }
    }

    /// Run `body` with a `(pointer, length)` view of the optional funding
    /// derivation-path string, marshalled as raw UTF-8 bytes (no NUL
    /// terminator — the FFI reads exactly `funding_path_len` bytes).
    ///
    /// `nil` yields `(nil, 0)`, which the FFI reads as "no explicit funding
    /// path" (fund from the unmixed BIP44 account at `fundingAccountIndex`). A
    /// non-nil value is encoded to its UTF-8 byte view and pinned for the call
    /// via `withUnsafeBytes` so the pointer stays valid for the duration of
    /// `body`. The FFI parses the bytes as a BIP32 path and returns an error
    /// for invalid UTF-8 or a malformed path, so no validation is duplicated
    /// here. An empty (but non-nil) string yields `(nil, 0)` — the same "no
    /// path" semantics the FFI applies to a zero length.
    ///
    /// `nonisolated` for the same reason as `withOptionalSurplusOutput`: it is
    /// pure byte marshalling invoked from the off-main-actor `Task.detached`
    /// body and touches no `PlatformWalletManager` state.
    nonisolated private static func withOptionalFundingPath<R>(
        _ fundingPath: String?,
        _ body: (UnsafePointer<UInt8>?, UInt) throws -> R
    ) throws -> R {
        guard let fundingPath, !fundingPath.isEmpty else {
            return try body(nil, 0)
        }
        let utf8 = Array(fundingPath.utf8)
        return try utf8.withUnsafeBufferPointer { raw in
            guard let ptr = raw.baseAddress else {
                throw PlatformWalletError.invalidParameter(
                    "fundingPath baseAddress is nil"
                )
            }
            return try body(ptr, UInt(raw.count))
        }
    }
}
