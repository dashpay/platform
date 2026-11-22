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
///                                      (a resume: "Asset lock built",
///                                      done once a matching row exists)
///   2. Broadcasting                  → activeLock `statusRaw == 1`,
///                                      controller `.inFlight`, and
///                                      < `broadcastingWindow` since
///                                      the row's `updatedAt`; on a
///                                      resume also `statusRaw == 0`,
///                                      titled "Resuming the existing
///                                      asset lock" (Rust decides
///                                      whether anything is sent)
///   3. Waiting for InstantSend or    → activeLock `statusRaw == 1`
///      ChainLock proof                 otherwise, whatever its age;
///                                      once passed, titled by the
///                                      proof the row records (2 or 3)
///   4. ChainLock proof               → never the current step; done
///                                      when `statusRaw == 3`
///                                      (CL-locked), `.notAsserted` at
///                                      2 (InstantSend recorded)
///   5. Shielding                     → activeLock `statusRaw ∈ {2, 3, 5}`
///                                      AND controller still `.inFlight`
///                                      (5, RecoveredFromChain: the lock
///                                      is confirmed on Core, only the
///                                      shield remains)
///
/// The row's age never decides the finality lane: a resumed lock's
/// `updatedAt` is as old as its last status change, so age says nothing
/// about whether Rust is still waiting for InstantSend. The lane is shown
/// only once the row reports it (status 2 or 3).
///
/// A lock at `statusRaw == 6` (DrainCompleted) marks every step done,
/// as a completed controller does.
///
/// A failed attempt is marked on the step its lock reached by the same
/// mapping; with no matching lock nothing was built, so step 1 fails and
/// steps 2 to 5 stay pending (`currentStep`).
///
/// Step 5 is meaningfully longer than its address-funding sibling
/// because Type 18 builds a ~30s Halo 2 proof inside the FFI call. Its
/// title and footer claim no live proving: after Platform rejects a first
/// proof the worker can be waiting for a ChainLock instead.
///
/// Steps 3 and 4 state only what the row records (`stepState`): step 3
/// waits for either proof, so a recorded proof completes it and names
/// it. At InstantSend (2) step 4 is `.notAsserted`: no ChainLock proof is
/// recorded, and none is claimed skipped, since Rust asks for one only if
/// it needs it. When the row records no proof (statuses 5 and 6, whose
/// proof is gone, or no row), both show `.finalityKindUnknown` once
/// passed, never two green checks that would claim both proofs.
struct ShieldedFundFromAssetLockProgressSection: View {
    @ObservedObject var controller: ShieldedFundFromAssetLockController

    /// Asset-lock rows for this wallet, filtered to the
    /// AssetLockShieldedAddressTopUp variant (discriminant `5`).
    /// Queried live so step changes are reactive to status changes
    /// without polling.
    @Query private var activeLocks: [PersistentAssetLock]

    /// Seconds after the row's `updatedAt` that an in-flight `Broadcast`
    /// row shows the visually brief "Broadcasting" step (2) before the
    /// finality wait (3). Same value as the address-funding progress
    /// section.
    nonisolated private static let broadcastingWindow: TimeInterval = 2.0

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
            let isResume = Self.isResumingExistingLock(
                controllerOperationId: controller.operationId,
                markedAsResume: controller.isResumingExistingLock
            )
            let step = Self.currentStep(
                phase: controller.phase,
                lockStatusRaw: lockStatusRaw,
                secondsSinceLockUpdate: lock.map { timeline.date.timeIntervalSince($0.updatedAt) } ?? 0,
                isResumingExistingLock: isResume
            )
            let isFailed = isFailed
            let errorMessage = failureMessage
            let states = (1...5).map {
                Self.stepState($0, currentStep: step, isFailed: isFailed, lockStatusRaw: lockStatusRaw)
            }
            let finalityKindUnknown = states.contains(.finalityKindUnknown)

            Section {
                ForEach(1...5, id: \.self) { idx in
                    stepRow(
                        index: idx,
                        title: Self.stepTitle(
                            idx,
                            isResumingExistingLock: isResume,
                            lockStatusRaw: lockStatusRaw
                        ),
                        state: states[idx - 1]
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
                Text(
                    Self.footerText(
                        step: step,
                        isFailed: isFailed,
                        finalityKindUnknown: finalityKindUnknown,
                        isResumingExistingLock: isResume
                    )
                )
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
    }

    // MARK: - Lock selection

    /// This controller's lock among the wallet's shielded top-up rows.
    /// The recorder is read on every render; it is filled before the
    /// lock's row is inserted, and that insertion updates `activeLocks`,
    /// so no extra publishing is needed to pick the row up.
    private var activeLock: PersistentAssetLock? {
        Self.matchingLock(
            controllerOperationId: controller.operationId,
            builtOutPointHex: controller.builtOutPoint?.outPointHex,
            newestFirst: activeLocks
        )
    }

    /// Picks the row that belongs to a controller's operation id:
    /// `"drain:<uuid>"` matches the row whose `operationId` bytes are that
    /// UUID, `"resume:<outPointHex>"` the row with that outpoint, and
    /// anything else (a fresh `"shield:..."`) only the row at
    /// `builtOutPointHex`, the outpoint the SDK reported for the lock this
    /// attempt built. An untagged row carries nothing else that ties it to
    /// an attempt: neither recency nor creation time does, since a
    /// historical lock first stored during catch-up is new too. `nil` when
    /// nothing matches, which the steps read as "no lock yet"; another
    /// operation's row is never borrowed.
    nonisolated static func matchingLock<R: ShieldedProgressLockRow>(
        controllerOperationId: String?,
        builtOutPointHex: String?,
        newestFirst rows: [R]
    ) -> R? {
        guard let controllerOperationId else {
            return builtLock(builtOutPointHex: builtOutPointHex, newestFirst: rows)
        }
        if controllerOperationId.hasPrefix(WalletDrainCoordinator.controllerOperationIdPrefix) {
            guard let drainId = WalletDrainCoordinator.drainOperationId(
                fromControllerOperationId: controllerOperationId
            ) else { return nil }
            return rows.first { row in
                row.operationId.flatMap(WalletDrainOperationRecord.uuid(fromOperationIdBytes:)) == drainId
            }
        }
        if controllerOperationId.hasPrefix(resumePrefix) {
            let outPointHex = String(controllerOperationId.dropFirst(resumePrefix.count))
            return rows.first { $0.outPointHex == outPointHex }
        }
        return builtLock(builtOutPointHex: builtOutPointHex, newestFirst: rows)
    }

    /// Same prefix `ShieldedFundFromAssetLockView` builds for a resume.
    nonisolated private static let resumePrefix = "resume:"

    /// Whether the controller only finishes an asset lock that already
    /// exists: a `"resume:"` operation, or one its caller marked (a Max
    /// Resume shares the `"drain:"` id with the Max that built the lock).
    nonisolated static func isResumingExistingLock(
        controllerOperationId: String?,
        markedAsResume: Bool
    ) -> Bool {
        markedAsResume || (controllerOperationId?.hasPrefix(resumePrefix) ?? false)
    }

    /// The row at the outpoint the attempt's lock was built with; none
    /// before the SDK has reported one (step 1).
    nonisolated private static func builtLock<R: ShieldedProgressLockRow>(
        builtOutPointHex: String?,
        newestFirst rows: [R]
    ) -> R? {
        guard let builtOutPointHex else { return nil }
        return rows.first { $0.outPointHex == builtOutPointHex }
    }

    // MARK: - Step computation

    /// The step the progress points at, 1...5, or 6 once every step is
    /// done. Pure over plain values so it is testable: `lockStatusRaw` is
    /// the status of this operation's lock (`matchingLock`), `nil` when
    /// none is stored, and `secondsSinceLockUpdate` is the time since that
    /// lock's `updatedAt`. The time only splits a `Broadcast` row of an
    /// in-flight attempt between steps 2 and 3; it never picks step 4.
    ///
    /// A failed attempt with no lock was refused before anything was
    /// built (for example a second Max that Rust refuses), so its failure
    /// sits on step 1 and no later step is claimed.
    ///
    /// A resume builds nothing: a `Built` (0) row is past step 1, and the
    /// step is resuming that existing lock (2). Whether Rust sends it again
    /// or finds it already final is not known here, so nothing claims it.
    nonisolated static func currentStep(
        phase: ShieldedFundFromAssetLockController.Phase,
        lockStatusRaw: Int?,
        secondsSinceLockUpdate: TimeInterval,
        isResumingExistingLock: Bool
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
            case 0:
                // Built. A fresh attempt is still building it; a resume is
                // past building and resumes the existing lock.
                return isResumingExistingLock ? 2 : 1
            case 1:
                // A failure stays where the lock was, so the clock must not
                // move it once the attempt has stopped.
                if case .inFlight = phase, secondsSinceLockUpdate < broadcastingWindow {
                    return 2
                }
                return 3
            case 2, 3, 5:
                // InstantSend-locked (2), ChainLock-locked (3) or
                // RecoveredFromChain (5): the lock is final on Core and
                // only the shield remains.
                return 5
            case 6:
                // DrainCompleted: Rust recorded the shield as done.
                return 6
            default:
                // Any other status.
                return 1
            }
        }
    }

    private var isFailed: Bool {
        if case .failed = controller.phase { return true }
        return false
    }

    private var failureMessage: String? {
        if case .failed(let msg) = controller.phase { return msg }
        return nil
    }

    /// Step titles state recorded facts. A resume of a `Built` (0) row is
    /// "Resuming" it, not sending it: Rust may find it already final. Step 3
    /// names the proof the row records (2 InstantSend, 3 ChainLock).
    nonisolated static func stepTitle(
        _ idx: Int,
        isResumingExistingLock: Bool,
        lockStatusRaw: Int?
    ) -> String {
        switch idx {
        case 1: return isResumingExistingLock ? "Asset lock built" : "Building asset-lock transaction"
        case 2:
            if isResumingExistingLock && lockStatusRaw == 0 {
                return "Resuming the existing asset lock"
            }
            return "Broadcasting"
        case 3:
            switch lockStatusRaw {
            case 2?: return "InstantSend proof recorded"
            case 3?: return "ChainLock proof recorded"
            default: return "Waiting for InstantSend or ChainLock proof"
            }
        case 4: return "ChainLock proof"
        case 5: return "Shielding"
        default: return ""
        }
    }

    enum StepState {
        case done, active, pending, failed
        /// A passed finality step (3 or 4) whose lane is not recorded:
        /// finality was reached, but by which proof is unknown.
        case finalityKindUnknown
        /// Step 4 once InstantSend is recorded: no ChainLock proof is
        /// recorded, and none is claimed used or skipped.
        case notAsserted
    }

    /// The state of step `idx` given `currentStep`. Step 3 waits for either
    /// proof, so status 2 (InstantSend) or 3 (ChainLock) completes it;
    /// status 3 also completes step 4 (ChainLock), status 2 leaves it
    /// `.notAsserted`. With any other status both passed steps are
    /// `.finalityKindUnknown`: `complete_wallet_drain` clears the proof, so
    /// at RecoveredFromChain (5), DrainCompleted (6), with no row, or with
    /// a row that lags a completed attempt, the lane that ran is unknown.
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
            // Step 3 waits for either proof, so any recorded proof completes
            // it; step 4 (ChainLock) is neither done nor skipped when only
            // InstantSend is recorded.
            switch (idx, lockStatusRaw) {
            case (4, 2?):
                return .notAsserted
            case (3, 2?), (3, 3?), (4, 3?):
                return .done
            case (3, _), (4, _):
                return .finalityKindUnknown
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
        case .notAsserted:
            // A dash, not a check: nothing about this proof is recorded.
            Image(systemName: "minus.circle")
                .foregroundColor(.secondary)
                .font(.title3)
                .accessibilityLabel("ChainLock proof only if needed")
        case .finalityKindUnknown:
            // Filled but grey: passed, yet not the green check that would
            // claim this proof was used.
            Image(systemName: "checkmark.circle.fill")
                .foregroundColor(.secondary)
                .font(.title3)
                .accessibilityLabel("Finality confirmed, proof kind unknown")
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
        case .notAsserted: return .secondary
        case .finalityKindUnknown: return .primary
        case .failed: return .red
        }
    }

    /// A resume never uses the "from wallet funds" text: it builds nothing.
    nonisolated static func footerText(
        step: Int,
        isFailed: Bool,
        finalityKindUnknown: Bool,
        isResumingExistingLock: Bool
    ) -> String {
        let stepText: String
        if isFailed {
            stepText = "Tap Dismiss to clear this entry."
        } else {
            switch step {
            case 1 where isResumingExistingLock:
                stepText = "Resuming an existing asset lock. No new one is built."
            case 1: stepText = "Building a Core asset-lock transaction from wallet funds."
            case 2 where isResumingExistingLock:
                stepText = "The asset lock already exists; no new one is built."
            case 2: stepText = "Sending the asset-lock transaction to peers."
            case 3:
                stepText = "Waiting for the asset-lock transaction to become final, by an "
                    + "InstantSend lock or, if none arrives, a ChainLock. A ChainLock can take "
                    + "a few minutes."
            case 5:
                stepText = "The asset lock has a proof; the wallet is proving the shield (about 30 s) "
                    + "and submitting it to Platform, which can include waiting for a ChainLock if "
                    + "Platform asks for one. The shielded note appears in your balance after the "
                    + "next sync."
            default: stepText = ""
            }
        }
        guard finalityKindUnknown else { return stepText }
        let note = "The asset lock reached finality, but whether by InstantSend or ChainLock "
            + "is not recorded for this lock, so steps 3 and 4 do not name a proof."
        return stepText.isEmpty ? note : stepText + "\n" + note
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
