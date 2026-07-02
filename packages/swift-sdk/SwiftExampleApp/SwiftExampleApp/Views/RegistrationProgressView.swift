import SwiftUI
import SwiftData
import SwiftDashSDK

/// Embeddable progress section. Use inside any parent `Form` so the
/// progress UI doesn't nest a second `Form`, which SwiftUI doesn't
/// render cleanly. For the standalone navigation destination see
/// `RegistrationProgressView` below.
///
/// Renders one of two step sets keyed off `controller.fundingKind`:
///
/// Asset-lock funding (5 steps, `statusRaw`-driven from the live
/// `PersistentAssetLock` row):
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
/// Shielded-pool funding (5 steps, phase + elapsed-time driven — there
/// is no asset lock and no per-stage signal from Rust during the opaque
/// FFI call; see `shieldedCurrentStep`):
///
///   1. Selecting shielded notes      2. Generating Halo 2 proof
///   3. Broadcasting transition       4. Waiting for platform confirmation
///   5. Registering identity
///
/// Step 4 ("Waiting for platform confirmation") exists so a
/// post-broadcast confirmation failure is attributed there — NOT to the
/// Halo 2 proof step — and so the `.unconfirmed` terminal state (the
/// broadcast landed but its result couldn't be confirmed) has a step to
/// render an orange warning on while steps 1–3 stay done and step 5
/// stays pending.
///
/// `.completed` is the *terminal* state and is not a separate step;
/// `RegistrationProgressView` renders the "Identity created" banner
/// + "View Identity" navigation below this section in its own
/// terminalSection. `.failed` marks the current step with the error
/// icon + message. `.unconfirmed` marks step 4 with an orange warning
/// triangle and renders its own terminalSection banner. (Asset-lock)
/// step 4 is shown as `.skipped` (faded checkmark) when the IS branch
/// came back fast so the user can see step 4 was passed through without
/// engaging the CL fallback.
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

    /// Number of visual steps for the active funding source. The
    /// asset-lock path has 5 (build → broadcast → IS → CL → register);
    /// the shielded-pool path has 5 (select notes → Halo 2 proof →
    /// broadcast → wait for confirmation → register).
    private var stepCount: Int {
        switch controller.fundingKind {
        case .assetLock: return 5
        case .shieldedPool: return 5
        }
    }

    var body: some View {
        // `TimelineView` re-fires the body every 1 s so the
        // elapsed-time heuristic that distinguishes step 2 / 3 / 4
        // (asset-lock) or step 1 / 2 (shielded) refreshes without an
        // external timer. The asset-lock anchor is the lock row's
        // `updatedAt`; the shielded anchor is `controller.lastSubmittedAt`.
        TimelineView(.periodic(from: .now, by: 1.0)) { timeline in
            let now = timeline.date
            let count = stepCount
            let step = currentStep(now: now)
            let isFailed = isFailed
            let isUnconfirmed = isUnconfirmed
            let errorMessage = failureMessage

            Section {
                ForEach(1...count, id: \.self) { idx in
                    stepRow(
                        index: idx,
                        title: stepTitle(idx),
                        state: stepState(
                            idx,
                            currentStep: step,
                            isFailed: isFailed,
                            isUnconfirmed: isUnconfirmed
                        )
                    )
                    // Only `.failed` renders an inline red message under the
                    // last step; `.unconfirmed` gets its own orange
                    // terminalSection banner instead.
                    if idx == count, isFailed, let message = errorMessage {
                        Text(message)
                            .font(.caption)
                            .foregroundColor(.red)
                            .padding(.leading, 32)
                    }
                }
            } header: {
                Text("Registration Progress")
            } footer: {
                Text(footerText(step: step, isFailed: isFailed, isUnconfirmed: isUnconfirmed))
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
    }

    // MARK: - Step computation

    /// 1...5, current active step. On `.completed` we report 6 (one
    /// past the last visual step) so all rows render as `.done`;
    /// the terminal "Identity created" banner is rendered by the
    /// parent `RegistrationProgressView`, not by this section.
    /// `now` is the time-of-rendering, used to drive the
    /// Broadcasting → Waiting-IS → Waiting-CL transition within
    /// `statusRaw == 1`.
    private func currentStep(now: Date) -> Int {
        if controller.fundingKind == .shieldedPool {
            return shieldedCurrentStep(now: now)
        }
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
        case .unconfirmed:
            // `.unconfirmed` is a shielded-only terminal state and is
            // handled in `shieldedCurrentStep`; it never reaches this
            // asset-lock branch. Report the last (register) step so the
            // switch is exhaustive and the rows render fully done.
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

    /// Visually-brief window (seconds since `lastSubmittedAt`) for the
    /// shielded "Selecting notes" step before the long Halo 2 proof
    /// step takes over. Note selection is sub-second on the Rust side;
    /// this just lets the user see step 1 register before step 2 spins.
    private static let shieldedNoteSelectionWindow: TimeInterval = 2.0

    /// Step 1...5 for the shielded-pool funding path. There is NO
    /// per-stage signal from Rust during the opaque
    /// `platform_wallet_manager_shielded_identity_create_from_pool`
    /// call (note-select → Halo 2 proof → broadcast → confirm → register
    /// all run inside one blocking FFI call), so transitions are driven
    /// from `controller.phase` plus elapsed time since `lastSubmittedAt`:
    ///
    ///   1. Selecting shielded notes  → `.idle` / `.preparingKeys`, or
    ///      the first `shieldedNoteSelectionWindow` seconds of `.inFlight`.
    ///   2. Generating Halo 2 proof   → `.inFlight` after that window.
    ///      Kept active for the rest of the call: broadcast / confirm /
    ///      register (steps 3/4/5) can't be observed separately while the
    ///      single FFI call is in flight, so they stay `.pending` rather
    ///      than flipping to a green check for work that may not have
    ///      happened yet.
    ///   On `.completed` return 6 (one past the last step) so all rows
    ///   render `.done`. On `.unconfirmed` return 4 ("Waiting for platform
    ///   confirmation") so that step carries the warning. On `.failed`
    ///   attribute the step: a `.broadcastRejected` failure marks step 3
    ///   ("Broadcasting transition"); an UNATTRIBUTED failure
    ///   (`failureStage == nil` — build / Halo 2 proof errors, or any other
    ///   error we can't confidently place) keeps the note-selection vs Halo 2
    ///   elapsed-time heuristic, measured *at the failure instant* (anchored
    ///   on `controller.terminalAt`) — failed rows are retained until
    ///   dismissed, so measuring against live `now` would let the failed icon
    ///   drift from step 1 to step 2 once the note-selection window lapses on
    ///   the wall clock.
    private func shieldedCurrentStep(now: Date) -> Int {
        switch controller.phase {
        case .idle, .preparingKeys:
            return 1
        case .completed:
            return 6
        case .unconfirmed:
            // Broadcast landed; only the result confirmation failed.
            // Attribute to the "Waiting for platform confirmation" step.
            return 4
        case .inFlight:
            return shieldedStep(elapsedTo: now)
        case .failed:
            // A definitive broadcast rejection (`failureStage ==
            // .broadcastRejected`) is attributed to the broadcast step (3).
            // For an unattributed failure (`failureStage == nil` — build /
            // Halo 2 proof errors, or any other error we can't confidently
            // place), keep the elapsed-time heuristic (note-selection vs
            // proof) — frozen at the failure instant; fall back to `now` only
            // if the terminal timestamp is missing (pre-submit failure shapes
            // never set it).
            if controller.failureStage == .broadcastRejected {
                return 3
            }
            return shieldedStep(elapsedTo: controller.terminalAt ?? now)
        }
    }

    /// Map elapsed time since `lastSubmittedAt` (measured up to
    /// `anchor`) onto shielded step 1 or 2. Without a submit
    /// timestamp we never left note selection.
    private func shieldedStep(elapsedTo anchor: Date) -> Int {
        guard let submittedAt = controller.lastSubmittedAt else { return 1 }
        let elapsed = anchor.timeIntervalSince(submittedAt)
        return elapsed < Self.shieldedNoteSelectionWindow ? 1 : 2
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

    /// True only for the shielded `.unconfirmed` terminal state. Drives
    /// the orange warning on step 4 (and keeps it distinct from
    /// `isFailed`, which `.unconfirmed` is NOT).
    private var isUnconfirmed: Bool {
        if case .unconfirmed = controller.phase { return true }
        return false
    }

    private var failureMessage: String? {
        if case .failed(let msg) = controller.phase { return msg }
        return nil
    }

    private func stepTitle(_ idx: Int) -> String {
        if controller.fundingKind == .shieldedPool {
            switch idx {
            case 1: return "Selecting shielded notes"
            case 2: return "Generating Halo 2 proof"
            case 3: return "Broadcasting transition"
            case 4: return "Waiting for platform confirmation"
            case 5: return "Registering identity"
            default: return ""
            }
        }
        switch idx {
        case 1: return "Building asset-lock transaction"
        case 2: return "Broadcasting"
        case 3: return "Waiting for InstantSend proof"
        case 4: return "Waiting for ChainLock proof"
        case 5: return "Registering identity"
        default: return ""
        }
    }

    /// Step-state classification. Drives the icon + tint on the
    /// row. `.skipped` is a softer pending variant for step 5 when
    /// the IS branch returned the proof without needing ChainLock
    /// fallback — visually distinguishable so users don't think
    /// the step "didn't happen yet" once we've moved past it.
    /// `.warning` (orange triangle) marks the "Waiting for platform
    /// confirmation" step when the broadcast landed but its result
    /// couldn't be confirmed (the shielded `.unconfirmed` terminal
    /// state) — distinct from `.failed` because nothing is actually
    /// wrong; the identity is probably live on chain.
    enum StepState { case done, active, pending, skipped, failed, warning }

    private func stepState(
        _ idx: Int,
        currentStep: Int,
        isFailed: Bool,
        isUnconfirmed: Bool
    ) -> StepState {
        if isFailed && idx == currentStep {
            return .failed
        }
        // `.unconfirmed` warns on its current step (step 4) while leaving
        // the earlier steps `.done` (`idx < currentStep` below) and the
        // register step `.pending` (`idx > currentStep`).
        if isUnconfirmed && idx == currentStep {
            return .warning
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
            // ever observed. Asset-lock only — the shielded path's
            // steps 3/4 (broadcast / register) have no IS/CL duality.
            if controller.fundingKind == .assetLock {
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
        case .warning:
            // Broadcast landed but its result couldn't be confirmed —
            // not an error, so an orange triangle, not the red octagon.
            Image(systemName: "exclamationmark.triangle.fill")
                .foregroundColor(.orange)
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
        case .warning: return .orange
        }
    }

    private func footerText(step: Int, isFailed: Bool, isUnconfirmed: Bool) -> String {
        if isUnconfirmed {
            return "The transition was broadcast, but confirmation of its "
                + "result proof failed. The identity may already exist on "
                + "chain and will appear after the next sync — do not "
                + "re-submit."
        }
        if isFailed {
            return "Tap Dismiss in Pending Registrations to clear this entry."
        }
        if controller.fundingKind == .shieldedPool {
            switch step {
            case 1: return "Selecting shielded notes to spend from the pool."
            case 2: return "Generating the Halo 2 proof — this can take ~1–2 minutes."
            case 3: return "Broadcasting the IdentityCreateFromShieldedPool transition."
            case 4: return "Waiting for Platform to confirm the transition's execution result."
            case 5: return "Registering the proof-verified identity on Platform."
            default: return ""
            }
        }
        switch step {
        case 1: return "Building a Core asset-lock transaction from wallet funds."
        case 2: return "Sending the asset-lock transaction to peers."
        case 3: return "Waiting for the InstantSend lock so the asset-lock proof is final."
        case 4: return "InstantSend timed out; falling back to ChainLock finality (~2 min)."
        case 5: return "Submitting the IdentityCreate state transition to Platform."
        default: return ""
        }
    }
}

/// Standalone navigation destination for a registration in flight,
/// completed, or failed. Pushed from `CreateIdentityView` on submit
/// and from the "Pending Registrations" row on the identities tab.
/// Renders the funding-source-specific progress steps (see
/// `RegistrationProgressSection`), plus the terminal section on
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
        case .unconfirmed(let identityId, let message):
            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Label(
                        "Broadcast succeeded — confirmation pending",
                        systemImage: "exclamationmark.triangle.fill"
                    )
                    .foregroundColor(.orange)
                    .font(.headline)
                    // Same base58 styling as the completed case — the id is
                    // the derived identity id Rust handed back; the identity
                    // is probably already live on chain.
                    Text(identityId.toBase58String())
                        .font(.system(.caption, design: .monospaced))
                        .foregroundColor(.secondary)
                        .textSelection(.enabled)
                    Text(message)
                        .font(.callout)
                        .foregroundColor(.primary)
                        .textSelection(.enabled)
                    Text(
                        "The transition was broadcast and accepted, but its "
                        + "execution-result proof couldn't be confirmed. The "
                        + "identity above will appear in the Identities tab "
                        + "after the next sync. Do NOT re-submit — the slot is "
                        + "held to prevent burning funds against a duplicate "
                        + "registration."
                    )
                    .font(.caption)
                    .foregroundColor(.secondary)
                    // Plain navigation dismiss only — it must NOT drop the
                    // controller from the coordinator (that frees the slot).
                    // The entry stays in Pending Registrations until the
                    // identity row appears via sync and the user dismisses it
                    // there.
                    Button {
                        dismiss()
                    } label: {
                        Text("Close")
                            .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.bordered)
                    .padding(.top, 4)
                }
            }
        default:
            EmptyView()
        }
    }
}
