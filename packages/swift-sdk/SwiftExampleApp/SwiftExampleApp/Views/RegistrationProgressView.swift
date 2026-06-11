import SwiftUI
import SwiftData
import SwiftDashSDK

/// Embeddable progress section. Use inside any parent `Form` so the
/// progress UI doesn't nest a second `Form`, which SwiftUI doesn't
/// render cleanly. For the standalone navigation destination see
/// `RegistrationProgressView` below.
///
/// The step list + step derivation depend on
/// `controller.fundingMode`:
///
/// `.assetLock` — 5 steps, driven by the live `PersistentAssetLock`
/// row plus elapsed-time heuristics:
///
///   1. Building asset-lock tx        → activeLock `statusRaw == 0`
///   2. Broadcasting                  → activeLock `statusRaw == 1` and
///                                      < `broadcastingWindow` since
///                                      the row's `updatedAt`
///   3. Waiting for InstantSend proof → activeLock `statusRaw == 1` and
///                                      between `broadcastingWindow`
///                                      and `instantLockTimeout`
///   4. Waiting for ChainLock proof   → activeLock `statusRaw == 1` and
///                                      >= `instantLockTimeout` (the
///                                      Rust side has fallen back to
///                                      ChainLock); also done when
///                                      `statusRaw == 3` because that
///                                      proof type finalised the lock
///   5. Registering identity          → activeLock `statusRaw == 2 or 3`
///                                      AND controller still `.inFlight`
///
/// `.shieldedPool` — 4 steps, Type-20 IdentityCreateFromShieldedPool.
/// There is NO asset-lock row and no per-stage signal from Rust (one
/// opaque async FFI call), so the active step is derived from `phase`
/// plus elapsed time since `lastSubmittedAt` (see
/// `shieldedCurrentStep`):
///
///   1. Selecting shielded notes      → `.idle`/`.preparingKeys`, or
///                                      `.inFlight` < `shieldedSelectWindow`
///   2. Building shielded proof       → `.inFlight` up to
///                                      `shieldedProofWindow` (the Halo 2
///                                      spend proof, ~30 s)
///   3. Broadcasting state transition → `.inFlight` up to
///                                      `shieldedBroadcastWindow`
///   4. Registering identity          → `.inFlight` afterwards
///
/// In both modes `.completed` is the *terminal* state and is not a
/// separate step; `RegistrationProgressView` renders the "Identity
/// created" banner + "View Identity" navigation below this section in
/// its own terminalSection. `.failed` marks the heuristically-current
/// step with the error icon + message. The `.skipped` (faded
/// checkmark) rendering for steps 3/4 is asset-lock-only — it
/// communicates which half of the IS/CL proof round was bypassed.
struct RegistrationProgressSection: View {
    @ObservedObject var controller: IdentityRegistrationController

    /// Asset-lock rows for this slot, queried live so step 2/3/4/5
    /// transitions are reactive to status changes without polling.
    @Query private var activeLocks: [PersistentAssetLock]

    /// Cutoff (seconds since the row transitioned to `Broadcast`)
    /// between the visually-brief "Broadcasting" step (3) and the
    /// "Waiting for InstantSend proof" step (4). Tuned short so
    /// step 3 doesn't visually linger — by the time the user sees
    /// the page, the broadcast is already on the wire.
    private static let broadcastingWindow: TimeInterval = 2.0

    /// Cutoff (seconds since `Broadcast`) where the Rust side falls
    /// back from InstantSend to ChainLock. Mirrors
    /// `AssetLockManager`'s 300 s IS wait. If `statusRaw == 1` is
    /// still the state after this window, the wallet is in the CL
    /// fallback window (180 s); we mark step 4 done and step 5
    /// active to communicate the shift.
    private static let instantLockTimeout: TimeInterval = 300.0

    // MARK: Shielded-pool display windows
    //
    // `.shieldedPool` mode has no progress signal from Rust — the
    // whole Type-20 flow is one opaque async FFI call. These cutoffs
    // are pure display heuristics, measured as elapsed seconds since
    // `controller.lastSubmittedAt`, anchored to the Halo 2 spend
    // proof's typical ~30 s duration (the dominant cost). Same spirit
    // as `broadcastingWindow` on the asset-lock path: the numbers
    // exist to make the step list feel honest, not to gate any logic.

    /// Step 1 ("Selecting shielded notes") window. Note selection is
    /// near-instant; kept brief so step 1 doesn't visually linger.
    private static let shieldedSelectWindow: TimeInterval = 2.0

    /// End of step 2 ("Building shielded proof"). The Halo 2 spend
    /// proof typically takes ~30 s; we hold step 2 active until just
    /// past that so the long pause reads as "building the proof".
    private static let shieldedProofWindow: TimeInterval = 35.0

    /// End of step 3 ("Broadcasting state transition") — a short
    /// window after the proof completes before we advance to the
    /// final "Registering identity" step.
    private static let shieldedBroadcastWindow: TimeInterval = 38.0

    init(controller: IdentityRegistrationController) {
        self.controller = controller
        let walletId = controller.walletId
        let identityIndex = controller.identityIndex
        _activeLocks = Query(
            filter: PersistentAssetLock.predicate(
                walletId: walletId,
                identityIndex: identityIndex
            ),
            sort: [SortDescriptor(\PersistentAssetLock.updatedAt, order: .reverse)]
        )
    }

    var body: some View {
        // `TimelineView` re-fires the body every 1 s so the
        // elapsed-time heuristic that distinguishes step 2 / 3 / 4
        // refreshes without an external timer. The lock row's
        // `updatedAt` is the anchor.
        TimelineView(.periodic(from: .now, by: 1.0)) { timeline in
            let now = timeline.date
            let step = currentStep(now: now)
            let isFailed = isFailed
            let errorMessage = failureMessage

            let stepCount = self.stepCount

            Section {
                ForEach(1...stepCount, id: \.self) { idx in
                    stepRow(
                        index: idx,
                        title: stepTitle(idx),
                        state: stepState(idx, currentStep: step, isFailed: isFailed)
                    )
                    // Error text hangs under the last step regardless
                    // of mode (asset-lock = step 5, shielded = step 4).
                    if idx == stepCount, let message = errorMessage {
                        Text(message)
                            .font(.caption)
                            .foregroundColor(.red)
                            .padding(.leading, 32)
                    }
                }
            } header: {
                Text("Registration Progress")
            } footer: {
                Text(footerText(step: step, isFailed: isFailed))
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
    }

    // MARK: - Step computation

    /// Number of visual steps for the active funding mode.
    private var stepCount: Int {
        switch controller.fundingMode {
        case .assetLock: return 5
        case .shieldedPool: return 4
        }
    }

    /// Current active step (1-based). On `.completed` we report
    /// `stepCount + 1` (one past the last visual step) so all rows
    /// render as `.done`; the terminal "Identity created" banner is
    /// rendered by the parent `RegistrationProgressView`. Dispatches
    /// to the per-mode computation.
    private func currentStep(now: Date) -> Int {
        switch controller.fundingMode {
        case .assetLock:
            return assetLockCurrentStep(now: now)
        case .shieldedPool:
            return shieldedCurrentStep(now: now)
        }
    }

    /// `.assetLock` step derivation. `now` is the time-of-rendering,
    /// used to drive the Broadcasting → Waiting-IS → Waiting-CL
    /// transition within `statusRaw == 1`.
    private func assetLockCurrentStep(now: Date) -> Int {
        switch controller.phase {
        case .idle, .preparingKeys:
            return 1
        case .completed:
            // No visible "registered" step — terminalSection on
            // `RegistrationProgressView` carries that state. Return
            // 6 so every step row (1...5) is marked `.done`.
            return 6
        case .failed:
            // Pick the step at which we failed by reading the
            // latest known lock status. The terminal indicator
            // (red on the failed step) is what the user sees,
            // but the partial fill of earlier steps tells them
            // how far we got.
            if let lock = activeLocks.first {
                switch lock.statusRaw {
                case 0: return 1
                case 1: return broadcastSubStep(for: lock, now: now)
                case 2, 3: return 5
                default: return 1
                }
            }
            return 5
        case .inFlight:
            guard let lock = activeLocks.first else {
                // No lock row yet — Rust has the slot but hasn't
                // emitted the first changeset. We're still
                // logically in step 1 (building the asset-lock
                // tx).
                return 1
            }
            switch lock.statusRaw {
            case 0:
                return 1
            case 1:
                return broadcastSubStep(for: lock, now: now)
            case 2:
                // InstantSend-locked. We never went through step 4
                // (CL fallback); it stays as `.skipped`. Step 5
                // active.
                return 5
            case 3:
                // ChainLock-locked. Step 3 (IS) is skipped (no IS
                // proof was observed — either IS timed out and CL
                // fallback ran, or the metadata.last_applied_chain_lock
                // path built a CL proof directly). Step 4 done.
                // Step 5 active. The `.skipped` rendering for step
                // 3 is driven by `step3WasSkipped`.
                return 5
            default:
                return 1
            }
        }
    }

    /// `.shieldedPool` step derivation (1...4). No asset-lock row and
    /// no per-stage signal from Rust, so the active step is a pure
    /// `phase` + elapsed-time heuristic anchored on `lastSubmittedAt`,
    /// using the shielded display windows (mirrors the
    /// `broadcastSubStep` style on the asset-lock path):
    ///   - `.idle`/`.preparingKeys` → step 1.
    ///   - `.inFlight`: < `shieldedSelectWindow` → step 1; <
    ///     `shieldedProofWindow` → step 2 (the ~30 s Halo 2 proof); <
    ///     `shieldedBroadcastWindow` → step 3; else step 4.
    ///   - `.completed` → 5 (past the last step; all rows `.done`).
    ///   - `.failed` → the step we were displaying *at the failure
    ///     instant*, frozen via `controller.terminalAt`. A failed row
    ///     lingers until dismissed, so anchoring on live `now` (as
    ///     `.inFlight` does) would drift the failed icon forward as
    ///     wall-clock time passed.
    private func shieldedCurrentStep(now: Date) -> Int {
        switch controller.phase {
        case .idle, .preparingKeys:
            return 1
        case .completed:
            // One past the last visual step (4) so every row is `.done`.
            return 5
        case .inFlight:
            // `lastSubmittedAt` is set the instant `submit` flips to
            // `.inFlight`; treat a missing anchor as "just started".
            return shieldedStep(elapsedTo: now)
        case .failed:
            // Freeze the elapsed-time clock at the failure instant so
            // the failed icon stays on the step we were showing when it
            // failed, instead of drifting as `now` keeps advancing.
            let anchor = controller.terminalAt ?? now
            return shieldedStep(elapsedTo: anchor)
        }
    }

    /// Map elapsed time since `lastSubmittedAt` (up to `anchor`) onto a
    /// shielded step (1...4) via the display windows. Shared by the
    /// live `.inFlight` path and the frozen `.failed` path.
    private func shieldedStep(elapsedTo anchor: Date) -> Int {
        let elapsed = anchor.timeIntervalSince(controller.lastSubmittedAt ?? anchor)
        if elapsed < Self.shieldedSelectWindow { return 1 }
        if elapsed < Self.shieldedProofWindow { return 2 }
        if elapsed < Self.shieldedBroadcastWindow { return 3 }
        return 4
    }

    /// Resolve which of steps 2/3/4 is "active" while the lock is
    /// at `statusRaw == 1`. Uses elapsed time since the row's last
    /// update as the anchor: brief broadcasting window first, then
    /// IS wait until the Rust-side timeout, then the CL fallback.
    private func broadcastSubStep(for lock: PersistentAssetLock, now: Date) -> Int {
        let elapsed = now.timeIntervalSince(lock.updatedAt)
        if elapsed < Self.broadcastingWindow { return 2 }
        if elapsed < Self.instantLockTimeout { return 3 }
        return 4
    }

    /// True when step 4 should appear "skipped" rather than
    /// "active" — i.e. the lock came back InstantSend-locked
    /// (statusRaw == 2) so the CL fallback was never needed. Drives
    /// a distinct pending visual on step 4 in the success path
    /// without bleeding into other gray rows.
    private var step4WasSkipped: Bool {
        guard let lock = activeLocks.first else { return false }
        return lock.statusRaw == 2
    }

    /// True when step 3 ("Waiting for InstantSend proof") should
    /// appear "skipped" — i.e. no IS proof was observed during the
    /// step-3 window. Symmetric to `step4WasSkipped`: that one fires
    /// when `statusRaw == 2` (IS-locked, CL fallback never needed);
    /// this one fires in every other "moved past step 3" case.
    ///
    /// The qualifying terminal / intermediate states:
    ///
    /// - `statusRaw == 3` — CL-locked. Either IS timed out and the
    ///   CL fallback ran, OR `wait_for_proof`'s
    ///   `metadata.last_applied_chain_lock` fallback (the Option B
    ///   path) built a Chain proof directly without ever attempting
    ///   IS. Either way step 3 was bypassed.
    /// - `statusRaw == 1` + elapsed past the IS deadline. The lock
    ///   is still Broadcast but `broadcastSubStep` has advanced to
    ///   step 4 (CL wait) because IS didn't materialize within
    ///   `instantLockTimeout`. The screenshot bug — the helper text
    ///   already says "InstantSend timed out; falling back to
    ///   ChainLock finality (~2 min)" while step 3 silently renders
    ///   a green ✅. The guard on `idx < currentStep` in `stepState`
    ///   means this branch only matters when we're past step 3
    ///   anyway, so a simple `statusRaw != 2` covers it cleanly —
    ///   `statusRaw == 0/1` with `currentStep <= 3` never asks about
    ///   step 3's left-behind state.
    ///
    /// `.skipped` renders the dashed pending icon, matching step 4's
    /// IS-success rendering.
    private var step3WasSkipped: Bool {
        guard let lock = activeLocks.first else { return false }
        return lock.statusRaw != 2
    }

    private var isFailed: Bool {
        if case .failed = controller.phase { return true }
        return false
    }

    private var failureMessage: String? {
        if case .failed(let msg) = controller.phase { return msg }
        return nil
    }

    private func stepTitle(_ idx: Int) -> String {
        switch controller.fundingMode {
        case .assetLock:
            switch idx {
            case 1: return "Building asset-lock transaction"
            case 2: return "Broadcasting"
            case 3: return "Waiting for InstantSend proof"
            case 4: return "Waiting for ChainLock proof"
            case 5: return "Registering identity"
            default: return ""
            }
        case .shieldedPool:
            switch idx {
            case 1: return "Selecting shielded notes"
            case 2: return "Building shielded proof"
            case 3: return "Broadcasting state transition"
            case 4: return "Registering identity"
            default: return ""
            }
        }
    }

    /// Step-state classification. Drives the icon + tint on the
    /// row. `.skipped` is a softer pending variant for step 5 when
    /// the IS branch returned the proof without needing ChainLock
    /// fallback — visually distinguishable so users don't think
    /// the step "didn't happen yet" once we've moved past it.
    enum StepState { case done, active, pending, skipped, failed }

    private func stepState(_ idx: Int, currentStep: Int, isFailed: Bool) -> StepState {
        if isFailed && idx == currentStep {
            return .failed
        }
        if idx < currentStep {
            // Steps 3 and 4 are the IS / CL halves of the proof
            // round: exactly one of them is skipped on a successful
            // resolution. Step 4 skipped when IS came back first
            // (statusRaw == 2); step 3 skipped when CL did (statusRaw
            // == 3, whether via IS-timeout fallback or the
            // `metadata.last_applied_chain_lock` direct path). The
            // symmetric carve-out keeps the icons honest — without
            // it, the CL-success path renders a green "InstantSend
            // proof received ✅" check even though no IS proof was
            // ever observed. Asset-lock-only: the shielded path has no
            // skipped steps (every stage is genuinely traversed).
            if controller.fundingMode == .assetLock {
                if idx == 3 && step3WasSkipped {
                    return .skipped
                }
                if idx == 4 && step4WasSkipped {
                    return .skipped
                }
            }
            return .done
        }
        if idx == currentStep {
            return .active
        }
        return .pending
    }

    // MARK: - Row UI

    @ViewBuilder
    private func stepRow(index: Int, title: String, state: StepState) -> some View {
        HStack(spacing: 12) {
            stepIcon(index: index, state: state)
            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.callout)
                    .foregroundColor(stepTextColor(state))
            }
            Spacer()
        }
    }

    @ViewBuilder
    private func stepIcon(index: Int, state: StepState) -> some View {
        switch state {
        case .done:
            Image(systemName: "checkmark.circle.fill")
                .foregroundColor(.green)
                .font(.title3)
        case .active:
            ProgressView()
                .scaleEffect(0.7)
                .frame(width: 22, height: 22)
        case .pending:
            ZStack {
                Circle()
                    .stroke(Color.secondary.opacity(0.4), lineWidth: 1)
                    .frame(width: 22, height: 22)
                Text("\(index)")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        case .skipped:
            // Lighter checkmark to communicate "we passed this
            // step but didn't need it" — typically step 5 when
            // the InstantSend branch returned the proof and the
            // ChainLock fallback was never engaged.
            Image(systemName: "checkmark.circle")
                .foregroundColor(.secondary)
                .font(.title3)
        case .failed:
            Image(systemName: "xmark.octagon.fill")
                .foregroundColor(.red)
                .font(.title3)
        }
    }

    private func stepTextColor(_ state: StepState) -> Color {
        switch state {
        case .done: return .primary
        case .active: return .primary
        case .pending: return .secondary
        case .skipped: return .secondary
        case .failed: return .red
        }
    }

    private func footerText(step: Int, isFailed: Bool) -> String {
        if isFailed {
            return "Tap Dismiss in Pending Registrations to clear this entry."
        }
        switch controller.fundingMode {
        case .assetLock:
            switch step {
            case 1: return "Building a Core asset-lock transaction from wallet funds."
            case 2: return "Sending the asset-lock transaction to peers."
            case 3: return "Waiting for the InstantSend lock so the asset-lock proof is final."
            case 4: return "InstantSend timed out; falling back to ChainLock finality (~2 min)."
            case 5: return "Submitting the IdentityCreate state transition to Platform."
            default: return ""
            }
        case .shieldedPool:
            switch step {
            case 1: return "Selecting notes from the wallet's shielded (Orchard) pool."
            case 2: return "Building the Halo 2 spend proof — this typically takes ~30 seconds."
            case 3: return "Submitting the IdentityCreateFromShieldedPool state transition to Platform."
            case 4: return "Waiting for Platform to confirm the new identity."
            default: return ""
            }
        }
    }
}

/// Standalone navigation destination for a registration in flight,
/// completed, or failed. Pushed from `CreateIdentityView` on submit
/// and from the "Pending Registrations" row on the identities tab.
/// Renders the 7-step progress, plus the terminal section on
/// `.completed` (success banner + "View Identity" navigation) or
/// `.failed` (inline error). Embedders that already render a
/// `Form` should use `RegistrationProgressSection` directly.
struct RegistrationProgressView: View {
    @ObservedObject var controller: IdentityRegistrationController
    @Environment(\.dismiss) private var dismiss
    @EnvironmentObject var walletManager: PlatformWalletManager

    init(controller: IdentityRegistrationController) {
        self.controller = controller
    }

    var body: some View {
        Form {
            RegistrationProgressSection(controller: controller)
            terminalSection
        }
        .navigationTitle("Registration")
        .navigationBarTitleDisplayMode(.inline)
    }

    @ViewBuilder
    private var terminalSection: some View {
        switch controller.phase {
        case .completed(let identityId):
            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Label("Identity created", systemImage: "checkmark.seal.fill")
                        .foregroundColor(.green)
                        .font(.headline)
                    Text(identityId.toBase58String())
                        .font(.system(.caption, design: .monospaced))
                        .foregroundColor(.secondary)
                        .textSelection(.enabled)
                    // Dismisses the pushed progress view AND drops
                    // the controller from the coordinator so the
                    // "Pending Registrations" row on the Identities
                    // tab clears immediately (instead of lingering
                    // ~30 s for the retention sweep).
                    Button {
                        walletManager.registrationCoordinator.dismiss(
                            walletId: controller.walletId,
                            identityIndex: controller.identityIndex
                        )
                        dismiss()
                    } label: {
                        Text("Done")
                            .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.borderedProminent)
                    .padding(.top, 4)
                }
            }
        case .failed(let message):
            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Label("Registration failed", systemImage: "xmark.octagon.fill")
                        .foregroundColor(.red)
                        .font(.headline)
                    Text(message)
                        .font(.callout)
                        .foregroundColor(.primary)
                        .textSelection(.enabled)
                }
            }
        default:
            EmptyView()
        }
    }
}
