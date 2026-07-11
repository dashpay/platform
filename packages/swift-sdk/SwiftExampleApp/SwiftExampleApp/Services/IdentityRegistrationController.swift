import Foundation
import SwiftDashSDK

/// Per-slot state owned by a single identity registration attempt.
///
/// One controller is created per `(walletId, identityIndex)` slot
/// when the user submits `CreateIdentityView`. The controller owns
/// the in-flight `Task`, exposes its current `phase` via
/// `@Published`, and survives view dismissal via
/// `RegistrationCoordinator` on `PlatformWalletManager`. Multiple
/// controllers can be active simultaneously (one per slot); each
/// runs on `@MainActor` so SwiftUI observers see consistent
/// transitions.
///
/// The progress bar in `RegistrationProgressView` derives its step
/// from `phase` plus `fundingKind`: asset-lock funding combines `phase`
/// with the live `PersistentAssetLock` row (queried via `@Query` on
/// `(walletId, identityIndex)`, driven by `statusRaw`); shielded-pool
/// funding has no asset lock and is driven by `phase` + elapsed time
/// since `lastSubmittedAt` (the Halo 2 proof is the long pole).
@MainActor
final class IdentityRegistrationController: ObservableObject {
    /// How this registration is funded. Drives which step set the
    /// progress view renders: the asset-lock funding paths (Core /
    /// Platform-Payment / resume) emit a `PersistentAssetLock` row and
    /// walk the build → IS/CL → register steps; the shielded-pool path
    /// (Type-20 IdentityCreateFromShieldedPool) has no asset lock — its
    /// long pole is the Halo 2 proof — so it needs its own step set.
    enum FundingKind: Equatable {
        case assetLock
        case shieldedPool
    }

    enum Phase: Equatable {
        /// Pre-submit. The controller exists but `submit` hasn't
        /// fired yet. Not surfaced by `RegistrationProgressView`
        /// (the view only opens after a submit).
        case idle
        /// Step 1: pre-deriving + Keychain-persisting the identity
        /// keys via `prePersistIdentityKeysForRegistration`. The
        /// caller drives this transition before invoking `submit`.
        case preparingKeys
        /// Steps 2–4 inclusive: the FFI registration call is in
        /// flight. Stage within this phase is read from the
        /// matching `PersistentAssetLock.statusRaw` row.
        case inFlight
        /// Step 5: identity is registered. `identityId` is the
        /// 32-byte identifier the caller should persist /
        /// navigate to.
        case completed(identityId: Data)
        /// Failure terminal state. The message is shown inline in
        /// `RegistrationProgressView`'s step 5; the row stays in
        /// the coordinator's map until the user dismisses it
        /// manually.
        case failed(String)
        /// Broadcast-succeeded-but-confirmation-failed terminal state
        /// (shielded Type-20 only). The transition was relayed and
        /// accepted, but its execution-result proof couldn't be
        /// confirmed and a direct fetch of the derived id also came
        /// back empty (Rust already retried). The identity at
        /// `identityId` MAY already be live on chain.
        ///
        /// This is NOT a red failure: treating it as unregistered is
        /// the orphaned-identity hazard. Re-registering the same keys
        /// while the identity is live would fail the
        /// registered-key-hash stateful check and burn the funds to
        /// the fallback address minus a penalty — so the slot must stay
        /// held (see `isActive`) and the entry must linger in Pending
        /// Registrations until the identity row appears via the next
        /// sync and the user dismisses it.
        case unconfirmed(identityId: Data, message: String)

        /// Whether the controller is currently holding its slot
        /// against a fresh registration. Used by the Resumable
        /// Registrations surface to hide orphan asset locks whose
        /// slot is mid-flight — otherwise the same lock could
        /// appear in both Pending and Resumable lists during the
        /// window between asset-lock broadcast and PersistentIdentity
        /// write, letting the user race a duplicate Resume tap
        /// against the original FFI call.
        ///
        /// `.completed` is NOT active here: by the time the
        /// controller flips to `.completed` the persister has
        /// (or is about to) write a `PersistentIdentity` row,
        /// which the identity-slot anti-join already covers.
        /// `.failed` is NOT active either: the user is expected
        /// to retry, so the lock should resurface for selection.
        /// `.unconfirmed` IS active: the identity is probably live on
        /// chain, so the slot must stay held to block a re-submission
        /// that would burn funds against the registered-key-hash check.
        var isActive: Bool {
            switch self {
            case .preparingKeys, .inFlight, .unconfirmed:
                return true
            case .idle, .completed, .failed:
                return false
            }
        }
    }

    /// Which stage a shielded `.failed` terminal state failed at, so
    /// `RegistrationProgressView` can attribute the red marker to the
    /// right step. Only meaningful for `fundingKind == .shieldedPool`.
    ///
    /// Deliberately only ONE case: `.broadcastRejected` is the single
    /// failure shape we can attribute with confidence (a typed
    /// `PlatformWalletError.shieldedBroadcastFailed`). We cannot honestly
    /// claim a "before broadcast" stage by elimination — Rust build/proof
    /// errors arrive as a generic `.walletOperation`, indistinguishable at
    /// this layer from an invalid handle, a marshalling failure, or any other
    /// non-broadcast error. So `failureStage` is left `nil` for everything
    /// else and the progress view falls back to its elapsed-time heuristic.
    /// Extensible: add a case here only when a new typed error actually lets
    /// us attribute the stage with certainty.
    enum FailureStage {
        /// Platform definitively rejected the broadcast transition (a
        /// `PlatformWalletError.shieldedBroadcastFailed`). Attributed to
        /// the "Broadcasting transition" step.
        case broadcastRejected
    }

    /// Current phase. Updates flow:
    /// `.idle` → `.preparingKeys` (caller) → `.inFlight` (submit) →
    /// `.completed(id) | .failed(message) | .unconfirmed(id, message)`.
    @Published private(set) var phase: Phase = .idle

    /// Stage attribution for a shielded `.failed` phase. `nil` means
    /// "unattributed" — either the phase isn't a shielded failure, or the
    /// failure wasn't a confidently-attributable broadcast rejection. On a
    /// `nil` shielded failure the progress view falls back to its
    /// elapsed-time heuristic (note-selection vs Halo 2 proof). Reset at the
    /// start of every `submit` so a retry doesn't inherit the previous
    /// attempt's stage.
    @Published private(set) var failureStage: FailureStage?

    /// Slot this controller is bound to. Stored so the coordinator
    /// and the progress view can filter `PersistentAssetLock` rows
    /// by `(walletId, identityIndex)`.
    let walletId: Data
    let identityIndex: UInt32

    /// Funding source for this registration. Read by
    /// `RegistrationProgressSection` to pick the asset-lock vs shielded
    /// step set. Defaults to `.assetLock` so existing call sites are
    /// unchanged.
    let fundingKind: FundingKind

    /// Timestamp of the most recent `submit` call. Used by the
    /// coordinator's TTL-based retention policy (`.completed` rows
    /// purge ~30s after the success transition).
    private(set) var lastSubmittedAt: Date?

    /// Timestamp of the most recent terminal transition
    /// (`.completed` / `.failed` / `.unconfirmed`). Freezes the
    /// elapsed-time anchor
    /// for `RegistrationProgressView`'s shielded step heuristic: a
    /// `.failed` row is retained until the user dismisses it, and
    /// deriving its step from live `Date()` would let the failed
    /// icon drift to a later step as wall-clock time passes. Reset
    /// on every `submit` so a retried slot re-measures from the new
    /// attempt.
    private(set) var terminalAt: Date?

    /// Active registration task. Holds a reference so the
    /// coordinator's stash retains the work until completion;
    /// cancellation isn't wired today (the FFI call doesn't yet
    /// support clean abort), but the field lets future work hang
    /// off the same shape.
    private var task: Task<Void, Never>?

    init(
        walletId: Data,
        identityIndex: UInt32,
        fundingKind: FundingKind = .assetLock
    ) {
        self.walletId = walletId
        self.identityIndex = identityIndex
        self.fundingKind = fundingKind
    }

    /// Transition to `.preparingKeys`. Called by the caller before
    /// `submit` while it pre-derives the identity public keys via
    /// `prePersistIdentityKeysForRegistration`.
    func enterPreparingKeys() {
        phase = .preparingKeys
    }

    /// Submit the registration. Defensively rejects any phase that
    /// shouldn't fire a fresh FFI call:
    ///   - `.inFlight`: a second FFI call would race the first,
    ///     ending with a misleading "asset lock consumed" failure.
    ///   - `.completed`: re-submitting after success would flip the
    ///     UI from "Done" back to a spinner before failing on the
    ///     consumed lock.
    ///   - `.unconfirmed`: the identity is probably already live on
    ///     chain; re-submitting the same keys would fail the
    ///     registered-key-hash stateful check and burn the funds.
    /// `.idle`, `.preparingKeys`, and `.failed` are allowed — the
    /// coordinator drives the legitimate-restart flow through them
    /// (callers must call `enterPreparingKeys()` before `submit()`,
    /// `failed → preparingKeys → submit` is how a user retries).
    ///
    /// `body` performs the actual FFI call. It runs detached on a
    /// background priority and reports the identity id on success
    /// or rethrows on failure. The controller flips `phase` to
    /// `.completed` / `.unconfirmed` / `.failed` accordingly. On a shielded
    /// `.failed` it records `failureStage = .broadcastRejected` ONLY for a
    /// `PlatformWalletError.shieldedBroadcastFailed`; every other error leaves
    /// `failureStage` nil (unattributed — the progress view uses its
    /// elapsed-time heuristic) rather than guessing a stage by elimination.
    func submit(body: @escaping () async throws -> Data) {
        switch phase {
        case .idle, .preparingKeys, .failed:
            break
        case .inFlight, .completed, .unconfirmed:
            return
        }
        phase = .inFlight
        // Clear any stage attribution carried over from a previous failed
        // attempt before this one runs.
        failureStage = nil
        lastSubmittedAt = Date()
        terminalAt = nil
        task = Task { [weak self] in
            do {
                let identityId = try await body()
                await MainActor.run {
                    self?.terminalAt = Date()
                    self?.phase = .completed(identityId: identityId)
                }
            } catch let unconfirmed as ShieldedIdentityCreateUnconfirmedError {
                // Broadcast landed but its result couldn't be confirmed and
                // Rust's direct fetch came back empty. Hold the slot; the
                // identity probably exists and will surface on the next sync.
                await MainActor.run {
                    self?.terminalAt = Date()
                    self?.phase = .unconfirmed(
                        identityId: unconfirmed.identityId,
                        message: unconfirmed.message
                    )
                }
            } catch {
                // Only a `shieldedBroadcastFailed` is a confidently-attributable
                // broadcast rejection → mark the broadcast step. Everything else
                // (Rust build / Halo 2 proof errors, which surface as a generic
                // `.walletOperation`, plus invalid-handle / marshalling failures)
                // is indistinguishable at this layer, so leave `failureStage` nil
                // and let the progress view's elapsed-time heuristic place it.
                let stage: FailureStage?
                if case PlatformWalletError.shieldedBroadcastFailed = error {
                    stage = .broadcastRejected
                } else {
                    stage = nil
                }
                await MainActor.run {
                    self?.failureStage = stage
                    self?.terminalAt = Date()
                    self?.phase = .failed(error.localizedDescription)
                }
            }
        }
    }
}
