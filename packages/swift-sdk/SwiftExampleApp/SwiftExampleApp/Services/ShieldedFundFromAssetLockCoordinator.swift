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
        /// New, restarted or re-presented controller for this slot.
        /// Caller should bind progress UI to it. `bodyScheduled` is
        /// `true` only when this call's `body` was started; a re-tap
        /// that re-presents an existing controller reports `false`, so
        /// callers that count running bodies never infer it.
        case started(ShieldedFundFromAssetLockController, bodyScheduled: Bool)
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
    /// if the SAME operation is already in flight on the same
    /// recipient. Returns a `StartFundingResult` so the caller can
    /// distinguish a fresh start from a wallet-level conflict (see
    /// the type doc for rationale).
    ///
    /// Single-flighting works on two levels:
    /// 1. **Per-operation** (slot key + `operationId` match): a
    ///    second tap on the same operation during the FFI window
    ///    re-presents the existing controller, never races a
    ///    duplicate FFI call. The operation id (the resumed lock's
    ///    outpoint, or a unique per-submission fresh-shield id) is
    ///    required because resumable locks normally default to the
    ///    same wallet-owned shielded recipient — two different locks
    ///    share one slot key, and reusing the first lock's controller
    ///    would silently drop the second lock's resume body. A
    ///    DIFFERENT operation is `.blockedByOtherWalletFunding` while
    ///    the slot is in flight, and replaces the retained controller
    ///    once it has completed. Fresh shields mint a NEW id per
    ///    submission for the same reason: a fixed marker would match
    ///    the retained completed controller and suppress the next
    ///    fresh shield's body.
    /// 2. **Per-wallet**: if any controller on the same wallet is
    ///    `.inFlight` for a *different* recipient, reject with
    ///    `.blockedByOtherWalletFunding`. Mirrors the Rust-side
    ///    `shield_guard` mutex on `PlatformWallet` that
    ///    serializes shield-class operations.
    func startFunding(
        walletId: Data,
        recipientRaw43: Data,
        operationId: String,
        body: @escaping () async throws -> Void
    ) -> StartFundingResult {
        let key = SlotKey(walletId: walletId, recipientRaw43: recipientRaw43)
        if let existing = controllers[key] {
            switch existing.phase {
            case .inFlight:
                if existing.operationId == operationId {
                    // Re-tap of the same operation — bind to its progress.
                    return .started(existing, bodyScheduled: false)
                }
                // A different operation while one is running on this
                // wallet: same serialization verdict as a different
                // recipient — the caller gets the blocker, not a
                // controller that will never run its body.
                return .blockedByOtherWalletFunding(existing)
            case .completed:
                if existing.operationId == operationId {
                    // Re-tap after success — re-present the terminal
                    // state without re-entering.
                    return .started(existing, bodyScheduled: false)
                }
                // Completed slot + different operation: a fresh start,
                // not a re-tap. Fall through past the per-wallet check
                // below and REPLACE the retained controller (its sweep
                // is identity-guarded, so the old retention timer can't
                // evict the replacement).
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
                let scheduled = existing.submit(operationId: operationId, body: body)
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
                if scheduled {
                    scheduleRetentionSweep(key: key, controller: existing)
                }
                return .started(existing, bodyScheduled: scheduled)
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
        let scheduled = controller.submit(operationId: operationId, body: body)
        scheduleRetentionSweep(key: key, controller: controller)
        return .started(controller, bodyScheduled: scheduled)
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
    /// "Done" and "Dismiss" actions (failures stay indefinitely until
    /// acknowledged so the user can read the error).
    ///
    /// Removes the slot only while `controller` still holds it AND is not
    /// in flight; a no-op otherwise. A completed controller's sheet can
    /// stay open after `startFunding` hands the slot to a new operation,
    /// and a failed sheet can stay open after a retry puts the SAME
    /// controller back in flight. Dropping a running controller would
    /// hide it from `hasInFlightFundings`, which the SDK rebuild guard
    /// reads, while it may hold the manager's native admission.
    func dismiss(_ controller: ShieldedFundFromAssetLockController) {
        let key = SlotKey(walletId: controller.walletId, recipientRaw43: controller.recipientRaw43)
        guard controllers[key] === controller, !controller.phase.isActive else { return }
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
                let observedCompletedAt = completedAt
                // Ownership, phase and removal in ONE main-actor step. Split
                // across steps, a retry could put this controller back in
                // flight, or hand the slot to a replacement, between the
                // check and the removal. A replacement's own sweep is then
                // in charge, so this one stops without touching the slot.
                let step: RetentionSweepStep = await MainActor.run {
                    guard let coordinator = self, coordinator.controllers[key] === controller else {
                        return .stop
                    }
                    switch controller.phase {
                    case .completed:
                        let now = Date()
                        guard let at = observedCompletedAt else {
                            return .watch(completedAt: now)
                        }
                        guard now.timeIntervalSince(at) >= 30 else {
                            return .watch(completedAt: at)
                        }
                        coordinator.controllers.removeValue(forKey: key)
                        return .stop
                    case .failed:
                        return .stop
                    case .idle, .inFlight:
                        return .watch(completedAt: nil)
                    }
                }
                guard case .watch(let next) = step else { return }
                completedAt = next
                try? await Task.sleep(nanoseconds: 1_000_000_000)
            }
        }
    }

    /// What one retention-sweep check decided.
    private enum RetentionSweepStep: Sendable {
        /// Keep watching; `completedAt` is when the controller was first
        /// seen completed, `nil` while it is not.
        case watch(completedAt: Date?)
        /// Exit: the slot was purged, failed, or is not this controller's.
        case stop
    }
}
