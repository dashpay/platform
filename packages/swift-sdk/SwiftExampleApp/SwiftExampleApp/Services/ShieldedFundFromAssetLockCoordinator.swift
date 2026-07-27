import Foundation
import SwiftDashSDK

/// Singleton hub for in-flight shielded fund-from-asset-lock attempts,
/// hosted on `PlatformWalletManager` so funds survive view dismissal
/// and network-toggle pressure.
///
/// Mirrors [`AddressFundFromAssetLockCoordinator`] for Type 18
/// (`ShieldFromAssetLockTransition`), with one shape difference:
///
/// **Per-wallet serialization.** The Rust orchestration takes a
/// `shield_guard` mutex on the wallet that serializes ALL
/// shield-class operations (`shielded_fund_from_asset_lock`,
/// `shielded_shield_from_account`, etc.). So while the slot key is
/// `(walletId, recipientRaw43)` — fine for deduplicating same-
/// recipient double-taps — concurrent fundings to *different*
/// recipients on the same wallet would (a) all block on
/// `shield_guard` Rust-side, and (b) confuse the progress view,
/// which queries `PersistentAssetLock` only by `(walletId,
/// fundingTypeRaw == 5)` and would show the same lock's status to
/// both controllers.
///
/// To match Rust-side reality and avoid the UI race, `startFunding`
/// returns `.blockedByOtherWalletFunding` when another controller
/// on the same wallet is still `.inFlight`. The caller surfaces a
/// typed error and points the user at the in-flight funding.
@MainActor
final class ShieldedFundFromAssetLockCoordinator: ObservableObject {
    /// Outcome of `startFunding`. The caller surfaces the
    /// distinction in UI — a blocked start needs a typed error,
    /// a successful start binds the progress view to the
    /// returned controller.
    enum StartFundingResult {
        /// New or restarted controller for this slot. Caller
        /// should bind progress UI to it.
        case started(ShieldedFundFromAssetLockController)
        /// Another shielded funding is already in flight on the
        /// same wallet (different recipient). Caller surfaces
        /// the blocker's recipient in a "wait" message.
        case blockedByOtherWalletFunding(ShieldedFundFromAssetLockController)
    }

    /// Composite key — `walletId` is 32 raw bytes; `recipientRaw43`
    /// is the 43-byte raw Orchard payment address (11-byte
    /// diversifier + 32-byte pk_d).
    struct SlotKey: Hashable {
        let walletId: Data
        let recipientRaw43: Data
    }

    /// Active controllers keyed by slot. Stored as `@Published` so
    /// the "Pending Shielded Top Ups" row on the Wallet Detail
    /// screen can observe map mutations via `objectWillChange`.
    @Published private(set) var controllers: [SlotKey: ShieldedFundFromAssetLockController] = [:]

    /// True when at least one slot is currently in flight (phase
    /// `.inFlight`). Used by the network toggle's `.disabled(_:)`
    /// modifier — switching testnet ↔ mainnet mid-flight tears
    /// down the FFI manager and would abort the in-flight call.
    var hasInFlightFundings: Bool {
        controllers.contains { _, controller in
            if case .inFlight = controller.phase { return true }
            return false
        }
    }

    /// Look up the controller for a slot if one exists. Returns
    /// `nil` when there's no active funding for the slot — callers
    /// use that to decide whether to spawn a new controller or
    /// reuse the existing one.
    func controller(
        walletId: Data,
        recipientRaw43: Data
    ) -> ShieldedFundFromAssetLockController? {
        controllers[SlotKey(walletId: walletId, recipientRaw43: recipientRaw43)]
    }

    /// Snapshot of every active controller, sorted by recency of
    /// last submit (most recent first). Used by the "Pending
    /// Shielded Funding" row so dismissed-but-still-running flows
    /// remain reachable.
    func activeControllers() -> [ShieldedFundFromAssetLockController] {
        controllers.values.sorted { lhs, rhs in
            (lhs.lastSubmittedAt ?? .distantPast) > (rhs.lastSubmittedAt ?? .distantPast)
        }
    }

    /// Start a funding for the slot, or reuse an existing controller
    /// if one is already in flight on the same recipient. Returns a
    /// `StartFundingResult` so the caller can distinguish a fresh
    /// start from a wallet-level conflict (see the type doc for
    /// rationale).
    ///
    /// Single-flighting works on two levels:
    /// 1. **Per-recipient** (slot key match): a second tap on the
    ///    same recipient during the FFI window re-presents the
    ///    existing controller, never races a duplicate FFI call.
    /// 2. **Per-wallet** (new in this revision): if any controller
    ///    on the same wallet is `.inFlight` for a *different*
    ///    recipient, reject with
    ///    `.blockedByOtherWalletFunding`. Mirrors the Rust-side
    ///    `shield_guard` mutex on `PlatformWallet` that
    ///    serializes shield-class operations.
    func startFunding(
        walletId: Data,
        recipientRaw43: Data,
        body: @escaping () async throws -> Void
    ) -> StartFundingResult {
        let key = SlotKey(walletId: walletId, recipientRaw43: recipientRaw43)
        if let existing = controllers[key] {
            switch existing.phase {
            case .inFlight, .completed:
                // Active or just-completed — don't re-enter.
                // Returning the existing controller lets the caller
                // bind to its progress / terminal state without
                // disrupting it.
                return .started(existing)
            case .idle, .failed:
                // Legitimate restart paths. We've already checked
                // the same-recipient slot here; before letting the
                // restart proceed, fall through to the per-wallet
                // check below so two concurrent .failed retries on
                // different recipients can't both unblock at the
                // same time.
                if let blocker = inFlightOnWalletExcluding(
                    walletId: walletId,
                    excludingRecipient: recipientRaw43
                ) {
                    return .blockedByOtherWalletFunding(blocker)
                }
                existing.submit(body: body)
                // Spawn a fresh retention sweep. The original sweep
                // exited the moment the first attempt hit `.failed`
                // (see `scheduleRetentionSweep`'s `.failed: return`
                // arm) — without scheduling a new one, a retry that
                // succeeds would transition to `.completed` with no
                // Task watching for the 30s auto-purge, leaving the
                // slot stuck in `controllers` indefinitely and
                // locking that recipient out of new fundings until
                // app restart. Idempotent once the previous sweep
                // returned, so no risk of duplicate observers.
                scheduleRetentionSweep(key: key, controller: existing)
                return .started(existing)
            }
        }
        // Per-wallet serialization: if another controller on this
        // wallet is in flight (different recipient), surface the
        // blocker so the caller can render a typed "wait" message.
        if let blocker = inFlightOnWalletExcluding(
            walletId: walletId,
            excludingRecipient: recipientRaw43
        ) {
            return .blockedByOtherWalletFunding(blocker)
        }
        let controller = ShieldedFundFromAssetLockController(
            walletId: walletId,
            recipientRaw43: recipientRaw43
        )
        controllers[key] = controller
        controller.submit(body: body)
        scheduleRetentionSweep(key: key, controller: controller)
        return .started(controller)
    }

    /// Find any in-flight controller on `walletId` for a recipient
    /// other than `excludingRecipient`. Used by the per-wallet
    /// serialization check.
    private func inFlightOnWalletExcluding(
        walletId: Data,
        excludingRecipient: Data
    ) -> ShieldedFundFromAssetLockController? {
        for (key, controller) in controllers {
            guard key.walletId == walletId else { continue }
            guard key.recipientRaw43 != excludingRecipient else { continue }
            if case .inFlight = controller.phase {
                return controller
            }
        }
        return nil
    }

    /// Manually drop a controller from the map. Used by the UI's
    /// "Dismiss" action on a `.failed` row (failures stay
    /// indefinitely until acknowledged so the user can read the
    /// error).
    func dismiss(walletId: Data, recipientRaw43: Data) {
        let key = SlotKey(walletId: walletId, recipientRaw43: recipientRaw43)
        controllers.removeValue(forKey: key)
    }

    // MARK: - Retention sweep

    /// Auto-purge `.completed` controllers ~30s after the success
    /// transition so the wallet's Pending list doesn't accumulate
    /// stale rows. `.failed` controllers stay indefinitely until
    /// the user dismisses them. Same shape as the address-funding
    /// sibling.
    private func scheduleRetentionSweep(
        key: SlotKey,
        controller: ShieldedFundFromAssetLockController
    ) {
        Task { [weak self, weak controller] in
            guard let controller = controller else { return }
            var completedAt: Date?
            while !Task.isCancelled {
                let phase = await MainActor.run { controller.phase }
                switch phase {
                case .completed:
                    if completedAt == nil {
                        completedAt = Date()
                    } else if let at = completedAt,
                              Date().timeIntervalSince(at) >= 30 {
                        await MainActor.run {
                            _ = self?.controllers.removeValue(forKey: key)
                        }
                        return
                    }
                case .failed:
                    return
                default:
                    completedAt = nil
                }
                try? await Task.sleep(nanoseconds: 1_000_000_000)
            }
        }
    }
}
