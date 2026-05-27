import Foundation
import DashSDKFFI

/// Recipient entry for `shieldedFundFromAssetLock(...)`.
///
/// The Rust-side API today enforces exactly one recipient (the
/// `Option<Credits>` is ignored — the single recipient always
/// receives the full asset-lock value minus the protocol minimum
/// fee). The multi-shape signature is exposed here so the call
/// site doesn't have to change when DPP grows multi-output Orchard
/// bundles for Type 18.
public struct ShieldedFundFromAssetLockRecipient: Sendable {
    /// Raw 43-byte Orchard payment address (11-byte diversifier +
    /// 32-byte pk_d). Same shape `platform_wallet_manager_shielded_default_address`
    /// returns and `shieldedTransfer` consumes.
    public let recipientRaw43: Data

    /// Explicit credit amount, or `nil` to receive the remainder.
    /// Today the value is ignored — only the address is consumed.
    public let credits: UInt64?

    public init(recipientRaw43: Data, credits: UInt64? = nil) {
        self.recipientRaw43 = recipientRaw43
        self.credits = credits
    }
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
    ///   - amountDuffs: Amount to lock in Core duffs. The shielded
    ///     pool receives `amountDuffs * CREDITS_PER_DUFF` minus the
    ///     protocol minimum fee.
    ///   - recipients: Destination addresses (exactly one today).
    ///     Preflight rejects empty or multi-recipient lists.
    public func shieldedFundFromAssetLock(
        walletId: Data,
        fundingAccountIndex: UInt32,
        amountDuffs: UInt64,
        recipients: [ShieldedFundFromAssetLockRecipient]
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
                    let result = withExtendedLifetime(coreSigner) {
                        platform_wallet_manager_shielded_fund_from_asset_lock(
                            handle,
                            widPtr,
                            fundingAccountIndex,
                            amountDuffs,
                            recipientPtr,
                            coreSigner.handle
                        )
                    }
                    try result.check()
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
    public func shieldedResumeFundFromAssetLock(
        walletId: Data,
        outPointTxid: Data,
        outPointVout: UInt32,
        recipients: [ShieldedFundFromAssetLockRecipient]
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
                    let result = withExtendedLifetime(coreSigner) {
                        platform_wallet_manager_shielded_resume_fund_from_asset_lock(
                            handle,
                            widPtr,
                            &outPoint,
                            recipientPtr,
                            coreSigner.handle
                        )
                    }
                    try result.check()
                }
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
        }
    }
}
