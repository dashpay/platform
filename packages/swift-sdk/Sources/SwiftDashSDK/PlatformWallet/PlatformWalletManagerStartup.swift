//
//  PlatformWalletManagerStartup.swift
//  SwiftDashSDK
//
//  Ordered wallet bring-up: identity → contacts → contact accounts, run as one
//  bounded call before the host starts Core SPV.
//

import Foundation

/// Why a bring-up stopped where it did.
///
/// Every case is a normal result. A host starts Core SPV on all of them — the
/// partial cases just mean the DIP-15 rescan has work left to do.
public enum WalletStartupStatus: UInt8, Sendable {
    /// Identity resolved, contacts synced, no contact-account builds left
    /// queued. Everything a contact payment needs is in place.
    case ready = 0
    /// Platform answered that this seed owns no identity. Terminal, and not a
    /// failure — there is nothing to sync and nothing to drain.
    case noIdentity = 1
    /// The identity scan never reached Platform inside the budget. The wallet
    /// may well own one; we do not know yet, and asking again may answer it.
    case partialNoIdentity = 2
    /// Identity resolved and synced, but contact-account builds are still
    /// queued. The budget may have run out, the drain may have failed on some
    /// entries, or no contact-crypto provider was available.
    case partialAccountsPending = 3
    /// Discovery failed locally — a wallet or persistence fault, not a
    /// reachability problem. The identity question is unanswered, and another
    /// scan will not answer it: the same fault is still there.
    case discoveryFailed = 4

    /// Whether another discovery scan could change the answer.
    ///
    /// True only for ``partialNoIdentity``. The others are terminal for this
    /// launch — an identity was found, absence was proven, or the failure is
    /// local and will still be there next time.
    public var discoveryWorthRetrying: Bool { self == .partialNoIdentity }

    /// Whether the identity question has an answer.
    ///
    /// Not the inverse of ``discoveryWorthRetrying``: ``discoveryFailed``
    /// leaves the question open *and* is not worth retrying. Use this to decide
    /// what to show, and ``discoveryWorthRetrying`` to decide whether to scan
    /// again.
    public var identityIsSettled: Bool {
        self != .partialNoIdentity && self != .discoveryFailed
    }
}

/// What a bring-up did.
public struct WalletStartupOutcome: Sendable, Equatable {
    public let status: WalletStartupStatus
    /// The wallet's identity, when one is known by the time this returned.
    public let identityId: Data?
    /// Discovery scans performed. `0` when a local identity was already known
    /// and no network scan was needed.
    public let discoveryAttempts: UInt32
    /// Whether the inline contact-request pass ran.
    public let dashPaySyncRan: Bool
    /// Contact-crypto entries the drain completed.
    public let contactAccountsDrained: UInt32
    /// Contact-account builds still queued on return. Non-zero means the
    /// DIP-15 rescan will have to backfill those contacts' payments.
    public let contactAccountsPending: UInt32
    public let elapsed: TimeInterval
}

extension PlatformWalletManager {
    /// Bring one wallet's DashPay state up in dependency order, then return so
    /// the caller can start Core SPV.
    ///
    /// A contact's DIP-15 payment addresses are derived from its contact
    /// account, and an address the wallet is not watching when the
    /// compact-filter scan passes its funding height produces no transaction at
    /// all. This runs identity discovery, one contact-request pass, and the
    /// signer-present drain that turns queued contact-crypto into real accounts
    /// — so the first filter set already covers them.
    ///
    /// The ordering, the retry policy and the budget all live Rust-side; this
    /// is a thin bridge. Call it once per wallet load, immediately before
    /// `startSpv`.
    ///
    /// - Parameters:
    ///   - wallet: the wallet to bring up.
    ///   - budget: ceiling for the whole sequence. `nil` uses the SDK default
    ///     (20s). Expiry is reported in the outcome, never thrown: Core sync is
    ///     the wallet's primary function and must not be held hostage to
    ///     Platform being slow.
    ///   - gapLimit: identity-discovery gap limit; `nil` uses the default.
    ///   - storage: `WalletStorage` used by the resolver callback to read the
    ///     BIP-39 mnemonic from the Keychain. Overridable for tests.
    ///
    /// - Returns: what the sequence achieved. Start Core SPV regardless of the
    ///   status; inspect `contactAccountsPending` for diagnostics.
    ///
    /// - Throws: only for a malformed request — an unconfigured manager, a
    ///   malformed wallet id, an unknown wallet, or a `budget` outside the
    ///   supported range — or when the stored seed does not bind to this
    ///   wallet, which is a malformed pairing of the two and the one condition
    ///   under which none of this may run at all. An unreachable Platform, a
    ///   failed sync pass and an unfinished drain are all reported through the
    ///   returned status instead, because Core sync must start regardless.
    ///
    /// # Key material
    ///
    /// The mnemonic resolver and identity signer are built for this call and
    /// pinned across it with `withExtendedLifetime`; Rust borrows and never
    /// retains them. That per-call contract is deliberate — a standing
    /// Keychain capability driven by an unattended loop would be a different
    /// security posture, not a convenience.
    @discardableResult
    public func startWalletSubsystems(
        wallet: ManagedPlatformWallet,
        budget: TimeInterval? = nil,
        gapLimit: UInt32? = nil,
        storage: WalletStorage = WalletStorage()
    ) async throws -> WalletStartupOutcome {
        try ensureConfigured()

        let walletId = wallet.walletId
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be 32 bytes, got \(walletId.count)"
            )
        }

        // Nothing below may run on a seed that does not own this wallet.
        // Everything this call does with key material is unauthenticated —
        // the resolver derives whatever the Keychain holds, and
        // `register_contact_account` keys its existence check on the contact
        // pair rather than the xpub, so a wrong receiving xpub is written
        // once and no later correct-seed pass revisits it. The wallet then
        // watches addresses nobody pays to, with no symptom but payments that
        // never arrive.
        //
        // Enforced here rather than left to the host: a client that has to
        // remember to gate this call is a client that will eventually forget,
        // and the published `seedMismatch` flag cannot be that gate anyway —
        // hosts commonly kick the unlock off asynchronously, so the flag
        // races this call. The verify is marker-cached, so the common path
        // costs a string comparison.
        //
        // Against `storage`, not a default one: the resolver below reads that
        // store, and verifying a different Keychain than the work will use
        // would approve one mnemonic while another derives the accounts.
        try verifySeedBinding(wallet, storage: storage)

        let handle = self.handle
        // Only a definitive "no such item" means watch-only. A lookup that
        // failed for another reason — locked device, denied access — must still
        // hand Rust a resolver: it can then report the scan as deferred rather
        // than being told the wallet has no seed at all, which would classify a
        // temporary condition as terminal.
        let coreSigner: MnemonicResolver?
        switch storage.mnemonicAvailability(for: walletId) {
        case .absent:
            coreSigner = nil
        case .present:
            coreSigner = MnemonicResolver(storage: storage)
        case .unavailable(let status):
            SDKLogger.log(
                "startup: mnemonic availability unknown (OSStatus \(status)); "
                    + "passing a resolver so the scan can defer rather than fail",
                minimumLevel: .medium)
            coreSigner = MnemonicResolver(storage: storage)
        }
        // Identity signer for the DIP-15 auto-accept pass. Nil without a
        // SwiftData container → the drain runs provider-only, matching
        // `unlockWalletFromKeychain`.
        let identitySigner: KeychainSigner? = self.modelContainer.map {
            KeychainSigner(modelContainer: $0, network: self.signerNetwork ?? .testnet)
        }
        // `0` means "SDK default" across the boundary, so a caller asking for a
        // deliberately short budget must not round down into it — clamp to one
        // second instead. Non-finite and negative values are rejected outright
        // rather than trapping in the `UInt64` initializer, which is reachable
        // whenever the budget comes out of arithmetic.
        let budgetSecs: UInt64
        switch budget {
        case .none:
            budgetSecs = 0
        case .some(let requested) where !requested.isFinite || requested < 0:
            throw PlatformWalletError.invalidParameter(
                "budget must be a finite, non-negative number of seconds, got \(requested)"
            )
        case .some(let requested):
            // `UInt64(_:)` traps on anything outside its range, which a finite
            // `TimeInterval` can still be — `.greatestFiniteMagnitude` among
            // them. The failable initializer turns that into an error instead
            // of a crash. Rust validates the deadline independently: plenty of
            // representable `UInt64` seconds still cannot be added to an
            // `Instant`.
            guard let converted = UInt64(exactly: requested.rounded()) else {
                throw PlatformWalletError.invalidParameter(
                    "budget is outside the supported range of whole seconds: \(requested)"
                )
            }
            budgetSecs = max(1, converted)
        }

        return try await Task.detached(priority: .userInitiated) { () -> WalletStartupOutcome in
            try withExtendedLifetime((coreSigner, identitySigner)) {
                var out = WalletStartupOutcomeFFI()
                try walletId.withUnsafeBytes { raw in
                    try platform_wallet_manager_start_wallet_subsystems(
                        handle,
                        raw.bindMemory(to: UInt8.self).baseAddress,
                        coreSigner?.handle,
                        identitySigner?.handle,
                        budgetSecs,
                        gapLimit ?? 0,
                        &out
                    ).check()
                }
                return WalletStartupOutcome(out)
            }
        }.value
    }
}

extension WalletStartupOutcome {
    /// Map the flat FFI struct. An unrecognised status discriminant is read as
    /// ``WalletStartupStatus/partialNoIdentity`` — the conservative choice,
    /// since it is the one status that does not claim the identity question is
    /// settled.
    init(_ ffi: WalletStartupOutcomeFFI) {
        self.status = WalletStartupStatus(rawValue: ffi.status) ?? .partialNoIdentity
        self.identityId =
            ffi.has_identity_id
            ? Swift.withUnsafeBytes(of: ffi.identity_id) { Data($0) }
            : nil
        self.discoveryAttempts = ffi.discovery_attempts
        self.dashPaySyncRan = ffi.dashpay_sync_ran
        self.contactAccountsDrained = ffi.contact_accounts_drained
        self.contactAccountsPending = ffi.contact_accounts_pending
        self.elapsed = TimeInterval(ffi.elapsed_ms) / 1000
    }
}
