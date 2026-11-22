import SwiftUI
import SwiftData
import SwiftDashSDK

/// The fields `ShieldedFundFromAssetLockProgressSection.matchingLock`
/// reads, so the match is testable on plain values.
/// `PersistentAssetLock` conforms with its existing properties.
protocol ShieldedProgressLockRow {
    var outPointHex: String { get }
    var operationId: Data? { get }
}

extension PersistentAssetLock: ShieldedProgressLockRow {}

/// Embeddable 5-step progress section for a shielded
/// fund-from-asset-lock flow. Mirrors
/// [`AddressFundFromAssetLockProgressSection`] but for Type 18
/// (`ShieldFromAssetLockTransition`).
///
/// `activeLock` is the wallet's shielded top-up row that belongs to the
/// controller's operation (see `matchingLock`), never simply the newest
/// one: another operation's lock must not drive this progress.
///
/// Step mapping:
///
///   1. Building asset-lock tx        → no matching row yet, or
///                                      activeLock `statusRaw == 0`
///   2. Broadcasting                  → activeLock `statusRaw == 1` and
///                                      < `broadcastingWindow` since
///                                      the row's `updatedAt`
///   3. Waiting for InstantSend proof → activeLock `statusRaw == 1` and
///                                      between `broadcastingWindow`
///                                      and `instantLockTimeout`
///   4. Waiting for ChainLock proof   → activeLock `statusRaw == 1` and
///                                      >= `instantLockTimeout` (Rust
///                                      side has fallen back to CL);
///                                      also done when
///                                      `statusRaw == 3` (CL-locked).
///   5. Shielding (Halo 2 + submit)   → activeLock `statusRaw ∈ {2, 3, 5}`
///                                      AND controller still `.inFlight`
///                                      (5, RecoveredFromChain: the lock
///                                      is confirmed on Core, only the
///                                      shield remains)
///
/// A lock at `statusRaw == 6` (DrainCompleted) marks every step done,
/// as a completed controller does.
///
/// A failed attempt is marked on the step its lock reached by the same
/// mapping; with no matching lock nothing was built, so step 1 fails and
/// steps 2 to 5 stay pending (`currentStep`).
///
/// Step 5 is meaningfully longer than its address-funding sibling
/// because Type 18 builds a ~30s Halo 2 proof inside the FFI call.
/// The footer explicitly calls that out so users don't think the
/// app is hung.
///
/// One of steps 3/4 is `.skipped` only when the lock shows which
/// finality lane ran (`stepState`); at statuses 5 and 6 the proof is
/// gone, so both show done once passed.
struct ShieldedFundFromAssetLockProgressSection: View {
    @ObservedObject var controller: ShieldedFundFromAssetLockController

    /// Asset-lock rows for this wallet, filtered to the
    /// AssetLockShieldedAddressTopUp variant (discriminant `5`).
    /// Queried live so step 2/3/4 transitions are reactive to
    /// status changes without polling.
    @Query private var activeLocks: [PersistentAssetLock]

    /// Cutoff (seconds since the row transitioned to `Broadcast`)
    /// between the visually-brief "Broadcasting" step (2) and the
    /// "Waiting for InstantSend proof" step (3). Same value as the
    /// address-funding progress section.
    nonisolated private static let broadcastingWindow: TimeInterval = 2.0

    /// Cutoff (seconds since `Broadcast`) where the Rust side falls
    /// back from InstantSend to ChainLock. Mirrors
    /// `AssetLockManager`'s 300 s IS wait.
    nonisolated private static let instantLockTimeout: TimeInterval = 300.0

    init(controller: ShieldedFundFromAssetLockController) {
        self.controller = controller
        let walletId = controller.walletId
        // `fundingTypeRaw == 5` is
        // `AssetLockFundingType::AssetLockShieldedAddressTopUp` per
        // the discriminant comment on `PersistentAssetLock`. We
        // filter on it here so an interleaved Type 14 address-
        // funding asset lock can't be picked up by mistake — both
        // flows produce per-wallet asset-lock rows but only one
        // funding type matches this controller's domain.
        _activeLocks = Query(
            filter: #Predicate<PersistentAssetLock> { entry in
                entry.walletId == walletId && entry.fundingTypeRaw == 5
            },
            sort: [SortDescriptor(\PersistentAssetLock.updatedAt, order: .reverse)]
        )
    }

    var body: some View {
        TimelineView(.periodic(from: .now, by: 1.0)) { timeline in
            let lock = activeLock
            let lockStatusRaw = lock?.statusRaw
            let step = Self.currentStep(
                phase: controller.phase,
                lockStatusRaw: lockStatusRaw,
                secondsSinceLockUpdate: lock.map { timeline.date.timeIntervalSince($0.updatedAt) } ?? 0
            )
            let isFailed = isFailed
            let errorMessage = failureMessage

            Section {
                ForEach(1...5, id: \.self) { idx in
                    stepRow(
                        index: idx,
                        title: stepTitle(idx),
                        state: Self.stepState(
                            idx,
                            currentStep: step,
                            isFailed: isFailed,
                            lockStatusRaw: lockStatusRaw
                        )
                    )
                    // Under the step that failed, which is step 1 when the
                    // attempt was refused before any lock was stored.
                    if idx == step, let message = errorMessage {
                        Text(message)
                            .font(.caption)
                            .foregroundColor(.red)
                            .padding(.leading, 32)
                    }
                }
            } header: {
                Text("Shield Progress")
            } footer: {
                Text(footerText(step: step, isFailed: isFailed))
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
    }

    // MARK: - Lock selection

    /// This controller's lock among the wallet's shielded top-up rows.
    private var activeLock: PersistentAssetLock? {
        Self.matchingLock(controllerOperationId: controller.operationId, newestFirst: activeLocks)
    }

    /// Picks the row that belongs to a controller's operation id:
    /// `"drain:<uuid>"` matches the row whose `operationId` bytes are that
    /// UUID, `"resume:<outPointHex>"` the row with that outpoint, and
    /// anything else (a fresh `"shield:..."`) the newest row without an
    /// operation id. `nil` when nothing matches, which the steps read as
    /// "no lock yet"; another operation's row is never borrowed.
    nonisolated static func matchingLock<R: ShieldedProgressLockRow>(
        controllerOperationId: String?,
        newestFirst rows: [R]
    ) -> R? {
        guard let controllerOperationId else {
            return rows.first { $0.operationId == nil }
        }
        if controllerOperationId.hasPrefix(WalletDrainCoordinator.controllerOperationIdPrefix) {
            guard let drainId = WalletDrainCoordinator.drainOperationId(
                fromControllerOperationId: controllerOperationId
            ) else { return nil }
            return rows.first { row in
                row.operationId.flatMap(WalletDrainOperationRecord.uuid(fromOperationIdBytes:)) == drainId
            }
        }
        // Same prefix `ShieldedFundFromAssetLockView` builds for a resume.
        let resumePrefix = "resume:"
        if controllerOperationId.hasPrefix(resumePrefix) {
            let outPointHex = String(controllerOperationId.dropFirst(resumePrefix.count))
            return rows.first { $0.outPointHex == outPointHex }
        }
        return rows.first { $0.operationId == nil }
    }

    // MARK: - Step computation

    /// The step the progress points at, 1...5, or 6 once every step is
    /// done. Pure over plain values so it is testable: `lockStatusRaw` is
    /// the status of this operation's lock (`matchingLock`), `nil` when
    /// none is stored, and `secondsSinceLockUpdate` is the time since that
    /// lock's `updatedAt`.
    ///
    /// A failed attempt with no lock was refused before anything was
    /// built (for example a second Max that Rust refuses), so its failure
    /// sits on step 1 and no later step is claimed.
    nonisolated static func currentStep(
        phase: ShieldedFundFromAssetLockController.Phase,
        lockStatusRaw: Int?,
        secondsSinceLockUpdate: TimeInterval
    ) -> Int {
        switch phase {
        case .idle:
            return 1
        case .completed:
            // No visible "shielded" step: the parent view carries that
            // state. 6 marks every step row (1...5) as done.
            return 6
        case .inFlight, .failed:
            guard let lockStatusRaw else { return 1 }
            switch lockStatusRaw {
            case 1:
                return broadcastSubStep(secondsSinceLockUpdate: secondsSinceLockUpdate)
            case 2, 3, 5:
                // InstantSend-locked (2), ChainLock-locked (3) or
                // RecoveredFromChain (5): the lock is final on Core and
                // only the shield remains.
                return 5
            case 6:
                // DrainCompleted: Rust recorded the shield as done.
                return 6
            default:
                // Built (0) and any other status.
                return 1
            }
        }
    }

    nonisolated private static func broadcastSubStep(secondsSinceLockUpdate elapsed: TimeInterval) -> Int {
        if elapsed < broadcastingWindow { return 2 }
        if elapsed < instantLockTimeout { return 3 }
        return 4
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
        switch idx {
        case 1: return "Building asset-lock transaction"
        case 2: return "Broadcasting"
        case 3: return "Waiting for InstantSend proof"
        case 4: return "Waiting for ChainLock proof"
        case 5: return "Building Orchard proof and shielding"
        default: return ""
        }
    }

    enum StepState { case done, active, pending, skipped, failed }

    /// The state of step `idx` given `currentStep`. A passed finality lane
    /// is skipped only when the lock shows the other one ran: status 2
    /// (InstantSend proof) skips step 4, status 3 (ChainLock proof) skips
    /// step 3, and a Broadcast lock (1) waiting on step 4 is past the
    /// InstantSend timeout, so step 3 is skipped. Any other status shows
    /// both as done: `complete_wallet_drain` clears the proof, so at
    /// RecoveredFromChain (5), DrainCompleted (6) or with no row the lane
    /// that ran is unknown.
    nonisolated static func stepState(
        _ idx: Int,
        currentStep: Int,
        isFailed: Bool,
        lockStatusRaw: Int?
    ) -> StepState {
        if isFailed && idx == currentStep {
            return .failed
        }
        if idx < currentStep {
            switch (idx, lockStatusRaw) {
            case (3, 3?), (4, 2?):
                return .skipped
            case (3, 1?) where currentStep == 4:
                return .skipped
            default:
                return .done
            }
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
            return "Tap Dismiss to clear this entry."
        }
        switch step {
        case 1: return "Building a Core asset-lock transaction from wallet funds."
        case 2: return "Sending the asset-lock transaction to peers."
        case 3: return "Waiting for the InstantSend lock so the asset-lock proof is final."
        case 4: return "InstantSend timed out; falling back to ChainLock finality (~2 min)."
        case 5: return "Building the Halo 2 proof (~30s) and submitting the ShieldFromAssetLock state transition to Platform. The shielded note appears in your balance after the next sync pass."
        default: return ""
        }
    }
}

/// Standalone navigation destination for a shielded funding in
/// flight, completed, or failed. Pushed from
/// `ShieldedFundFromAssetLockView` on submit.
struct ShieldedFundFromAssetLockProgressView: View {
    @ObservedObject var controller: ShieldedFundFromAssetLockController
    @Environment(\.dismiss) private var dismiss
    @EnvironmentObject var walletManager: PlatformWalletManager

    init(controller: ShieldedFundFromAssetLockController) {
        self.controller = controller
    }

    var body: some View {
        Form {
            ShieldedFundFromAssetLockProgressSection(controller: controller)
            terminalSection
        }
        .navigationTitle("Shield from Asset Lock")
        .navigationBarTitleDisplayMode(.inline)
    }

    @ViewBuilder
    private var terminalSection: some View {
        switch controller.phase {
        case .completed:
            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Label("Shielded", systemImage: "checkmark.seal.fill")
                        .foregroundColor(.green)
                        .font(.headline)
                    Text(
                        "The shielded note will appear in your balance after the "
                            + "next sync pass — broadcast already succeeded; the note is "
                            + "decrypted asynchronously by the shielded sync coordinator."
                    )
                    .font(.caption)
                    .foregroundColor(.secondary)
                    Button {
                        walletManager.shieldedFundFromAssetLockCoordinator.dismiss(controller)
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
                    Label("Shield failed", systemImage: "xmark.octagon.fill")
                        .foregroundColor(.red)
                        .font(.headline)
                    Text(message)
                        .font(.callout)
                        .foregroundColor(.primary)
                        .textSelection(.enabled)
                    Button {
                        walletManager.shieldedFundFromAssetLockCoordinator.dismiss(controller)
                        dismiss()
                    } label: {
                        Text("Dismiss")
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
