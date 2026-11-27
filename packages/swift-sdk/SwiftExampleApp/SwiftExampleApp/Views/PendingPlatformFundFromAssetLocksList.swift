// PendingPlatformFundFromAssetLocksList.swift
// SwiftExampleApp
//
// Wallet-scoped "Pending Platform Top Ups" surface that mirrors the
// identity-side `PendingRegistrationsList` + `ResumableRegistrationsList`
// pair. Two distinct row sources are merged here:
//
//   1. In-flight controllers from `AddressFundFromAssetLockCoordinator` — the
//      live submit-still-running case.
//   2. Orphaned `PersistentAssetLock` rows on either top-up funding type —
//      `AssetLockAddressTopUp` (4) or `AssetLockShieldedAddressTopUp` (5) —
//      at a recoverable status (`isVisibleAsResumable`, i.e.
//      `[1, 3] ∪ {5}`): the crash-recovery case where the user killed the
//      app between asset-lock broadcast and ST submission. A wallet
//      drain's Built (0) checkpoint with no Max-transfer marker is also
//      listed (`isOfferedForResume`).
//
// Both top-up funding types belong here because neither has any other
// recovery home: the identity-side surfaces admit only funding types
// `0...2`, and `3` is an invitation voucher owned by the reclaim flow.
// Filtering to `4` alone left every shielded top-up — stalled or
// RecoveredFromChain — invisible on every surface in the app.
//
// Anti-join: an orphaned lock is hidden if its outpoint is already
// claimed by an in-flight controller. (We index by outpoint here
// rather than by the `(walletId, platformAccountIndex, recipientHash)`
// triple because the orphaned lock doesn't know its recipient — the
// recipient is picked at ST-submit time.)

import SwiftUI
import SwiftData
import SwiftDashSDK

/// Section view backing the Wallet Detail screen's "Pending Platform
/// Funding" surface for a single wallet. Observes
/// `AddressFundFromAssetLockCoordinator` directly (`@ObservedObject`) so its
/// `@Published controllers` map mutations trigger SwiftUI re-renders
/// of the in-flight rows.
struct PendingPlatformFundFromAssetLocksList: View {
    @ObservedObject var coordinator: AddressFundFromAssetLockCoordinator
    /// Wallet to scope the section to. The Identities-tab equivalent
    /// is cross-wallet because identities are a global concept; here
    /// the wallet detail screen is already wallet-scoped so we
    /// follow suit.
    let walletId: Data
    /// All asset-lock rows for the wallet. Pre-filtered by the
    /// parent (`WalletDetailView`) so this section doesn't run
    /// another `@Query`.
    let assetLocks: [PersistentAssetLock]
    /// Operation ids of stored Max-transfer markers. Their locks resume
    /// from `UnfinishedWalletDrainsList` (by the outpoint the drain status
    /// names), so they are not offered here as well.
    let drainMarkedOperationIds: Set<UUID>
    /// Bound to the parent's "resume sheet" state. Setting non-nil
    /// presents `FundFromAssetLockPlatformAddressView` in resume mode.
    @Binding var resumingAssetLock: PersistentAssetLock?

    var body: some View {
        let walletControllers = controllersForWallet
        // A resume names its lock (`assetLockOutPoint`), so while it runs
        // only that row is hidden: it is listed once, as the controller.
        // A fresh funding does not know its outpoint until it is built, and
        // its own lock passes through Broadcast → InstantSendLocked during
        // its finality wait, indistinguishable here from an unrelated
        // proof-ready orphan. While one is in flight Resume is therefore
        // suppressed on every row, which would risk a second consume of the
        // same outpoint otherwise. Only `.isActive` controllers count: a
        // failed resume's lock stays eligible for retry, and phase
        // transitions re-render this view via the coordinator's forwarded
        // `objectWillChange`.
        let activeControllers = walletControllers.filter { $0.phase.isActive }
        let hasActiveFunding = activeControllers.contains { $0.assetLockOutPoint == nil }
        let orphans = resumableLocks(
            excludingControllerOutpoints: Set(activeControllers.compactMap(\.assetLockOutPoint))
        )

        if !walletControllers.isEmpty || !orphans.isEmpty {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Text("Pending Platform Top Ups (\(walletControllers.count + orphans.count))")
                        .font(.headline)
                    Spacer()
                }
                .padding(.horizontal)

                VStack(spacing: 0) {
                    ForEach(Array(walletControllers.enumerated()), id: \.element.platformFundFromAssetLockRowID) { idx, controller in
                        PendingPlatformFundFromAssetLockRow(controller: controller)
                            .padding(.horizontal)
                            .padding(.vertical, 10)
                        if idx < walletControllers.count - 1 || !orphans.isEmpty {
                            Divider()
                        }
                    }
                    ForEach(Array(orphans.enumerated()), id: \.element.id) { idx, lock in
                        ResumablePlatformFundFromAssetLockRow(
                            lock: lock,
                            hasActiveFunding: hasActiveFunding,
                            onResume: { resumingAssetLock = lock }
                        )
                        .padding(.horizontal)
                        .padding(.vertical, 10)
                        if idx < orphans.count - 1 {
                            Divider()
                        }
                    }
                }
                .background(Color(UIColor.secondarySystemBackground))
                .cornerRadius(10)
                .padding(.horizontal)
            }
        }
    }

    /// All funding controllers for this wallet, newest-first — named for
    /// the wallet scope, not a phase. `coordinator.activeControllers()`
    /// returns every slot in the map, including `.completed`/`.failed`
    /// ones that linger until dismissed or swept, so the `.isActive`
    /// filter in `hasActiveFunding` is doing real work, not restating the
    /// name.
    private var controllersForWallet: [AddressFundFromAssetLockController] {
        coordinator.activeControllers().filter { $0.walletId == walletId }
    }

    /// Resumable asset-lock rows for this wallet — either top-up funding
    /// type at a recoverable status. Excludes outpoints already owned by
    /// an in-flight controller.
    private func resumableLocks(
        excludingControllerOutpoints excluded: Set<String>
    ) -> [PersistentAssetLock] {
        assetLocks
            .filter {
                Self.isOfferedForResume(
                    fundingTypeRaw: $0.fundingTypeRaw,
                    statusRaw: $0.statusRaw,
                    operationId: $0.operationId,
                    markedOperationIds: drainMarkedOperationIds
                )
            }
            .filter { !excluded.contains($0.outPointHex) }
    }

    /// The whole row decision for this list, over plain values.
    ///
    /// The drain half is asked of Rust (`WalletDrainRules.lockListingIfKnown`):
    /// a lock whose operation id has a Max-transfer marker is hidden (it
    /// resumes from `UnfinishedWalletDrainsList`), and a markerless drain
    /// checkpoint is offered even at Built, because Rust resumes it by exact
    /// outpoint by rebroadcasting its transaction. A funding type or status
    /// Rust does not know is not offered. Everything else follows
    /// `isResumableTopUp`, which keeps every other Built row out: that one
    /// is owned by a build still in flight.
    nonisolated static func isOfferedForResume(
        fundingTypeRaw: Int,
        statusRaw: Int,
        operationId: Data?,
        markedOperationIds: Set<UUID>
    ) -> Bool {
        let hasOperationId = operationId.flatMap {
            WalletDrainOperationRecord.uuid(fromOperationIdBytes: $0)
        } != nil
        let listing = WalletDrainRules.lockListingIfKnown(
            fundingTypeRaw: fundingTypeRaw,
            statusRaw: statusRaw,
            hasOperationId: hasOperationId,
            hostHoldsStoredId: isClaimedByDrainMarker(
                operationId: operationId,
                markedOperationIds: markedOperationIds
            )
        )
        switch listing {
        case nil, .hidden?:
            return false
        case .offered?:
            return true
        case .unaffected?:
            return isResumableTopUp(fundingTypeRaw: fundingTypeRaw, statusRaw: statusRaw)
        }
    }

    /// Whether the app holds a Max-transfer marker for this lock's
    /// operation id: the host fact `isOfferedForResume` passes to Rust.
    nonisolated static func isClaimedByDrainMarker(
        operationId: Data?,
        markedOperationIds: Set<UUID>
    ) -> Bool {
        guard let operationId,
              let uuid = WalletDrainOperationRecord.uuid(fromOperationIdBytes: operationId)
        else { return false }
        return markedOperationIds.contains(uuid)
    }

    /// Funding-type + status predicate for this surface, extracted as a
    /// pure `static` (generic over `AssetLockResumeRow`) so it is
    /// unit-testable without a SwiftData container — same shape as
    /// `IdentitiesContentView.crossWalletResumableLocks`.
    ///
    /// Admits BOTH top-up funding types:
    ///
    ///   * `4` AssetLockAddressTopUp — resumes into
    ///     `FundFromAssetLockPlatformAddressView`.
    ///   * `5` AssetLockShieldedAddressTopUp — resumes into
    ///     `ShieldedFundFromAssetLockView`.
    ///
    /// `5` was previously filtered out here. Because the identity
    /// surfaces admit only `0...2` and the reclaim flow owns `3`, that
    /// left a stalled or RecoveredFromChain shielded top-up on NO surface
    /// in the app — real recoverable value with no way to reach it. The
    /// status half is delegated to `isVisibleAsResumable`, which excludes
    /// the terminal Consumed (`4`) by name while keeping
    /// RecoveredFromChain (`5`).
    ///
    /// Identity-family types are rejected fail-closed: they have their own
    /// resume surface, and dispatching one here would offer a resume flow
    /// that submits the wrong transition for it.
    nonisolated static func isResumableTopUp<R: AssetLockResumeRow>(_ lock: R) -> Bool {
        isResumableTopUp(fundingTypeRaw: lock.fundingTypeRaw, statusRaw: lock.statusRaw)
    }

    nonisolated static func isResumableTopUp(fundingTypeRaw: Int, statusRaw: Int) -> Bool {
        guard fundingTypeRaw == 4 || fundingTypeRaw == 5 else { return false }
        return PersistentAssetLock.isVisibleAsResumable(statusRaw: statusRaw)
    }
}

private extension AddressFundFromAssetLockController {
    /// Composite ForEach id: `(walletId hex)-(platformAccountIndex)-(recipientHash hex)`,
    /// plus `-(outpoint)` for a resume (the outpoint is part of the slot).
    /// The recipient hash is the within-account discriminator: two
    /// concurrent fund calls to different addresses on the same
    /// account otherwise collide on `(walletId, accountIndex)`. An external
    /// resume has neither (`-`): its outpoint alone tells it apart.
    var platformFundFromAssetLockRowID: String {
        let walletHex = walletId.map { String(format: "%02x", $0) }.joined()
        let account = platformAccountIndex.map(String.init) ?? "-"
        let recipientHex = recipientHash.map { $0.map { String(format: "%02x", $0) }.joined() } ?? "-"
        let base = "\(walletHex)-\(account)-\(recipientHex)"
        return assetLockOutPoint.map { "\(base)-\($0)" } ?? base
    }
}

/// Single row representing an in-flight `AddressFundFromAssetLockController`.
/// Tappable navigation pushes to `AddressFundFromAssetLockProgressView`.
struct PendingPlatformFundFromAssetLockRow: View {
    @ObservedObject var controller: AddressFundFromAssetLockController
    @EnvironmentObject var walletManager: PlatformWalletManager
    @State private var confirmingRelease = false

    var body: some View {
        HStack(spacing: 8) {
            NavigationLink(destination: AddressFundFromAssetLockProgressView(controller: controller)) {
                VStack(alignment: .leading, spacing: 4) {
                    HStack {
                        Image(systemName: phaseIcon)
                            .foregroundColor(phaseTint)
                        Text(accountLabel)
                            .font(.body)
                        Spacer()
                        Text(phaseLabel)
                            .font(.caption)
                            .foregroundColor(.secondary)
                        // Manual disclosure indicator — without a
                        // List ancestor SwiftUI doesn't auto-render
                        // the chevron, and the row would read as a
                        // static label.
                        Image(systemName: "chevron.right")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    Text(recipientLabel)
                        .font(.caption2)
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                }
                .padding(.vertical, 2)
            }
            .buttonStyle(.plain)
            // Inline Dismiss for `.failed` controllers. The earlier
            // `.swipeActions` modifier was dead — that modifier only
            // takes effect when the row is inside a List/Form, and
            // this row renders inside a VStack card on the wallet
            // detail screen. Without an inline button the user had
            // no way to clear a failed funding short of an app
            // restart. Only a failure that releases its slot
            // (`PaymentFailure.holdsSlot`) offers it; the coordinator
            // refuses any other.
            if case .failed(let failure) = controller.phase, !failure.holdsSlot {
                Button {
                    walletManager.addressFundFromAssetLockCoordinator.dismiss(
                        walletId: controller.walletId,
                        platformAccountIndex: controller.platformAccountIndex,
                        recipientHash: controller.recipientHash,
                        assetLockOutPoint: controller.assetLockOutPoint
                    )
                } label: {
                    Image(systemName: "trash")
                        .foregroundColor(.red)
                }
                .buttonStyle(.borderless)
                .accessibilityLabel("Dismiss failed funding")
            }
            // A hold without a lock to resume
            // (`PaymentFailure.releasableAfterReconciliation`) is released
            // only once the user confirmed the original payment in the history.
            if case .failed(let failure) = controller.phase, failure.releasableAfterReconciliation {
                Button {
                    confirmingRelease = true
                } label: {
                    Image(systemName: "checkmark.circle")
                        .foregroundColor(.orange)
                }
                .buttonStyle(.borderless)
                .accessibilityLabel("Payment checked in history — release")
            }
        }
        .confirmationDialog(
            "Release only after the transaction history shows this payment as completed or failed. A new payment for this slot may otherwise pay twice.",
            isPresented: $confirmingRelease,
            titleVisibility: .visible
        ) {
            Button("Release", role: .destructive) {
                walletManager.addressFundFromAssetLockCoordinator.release(
                    walletId: controller.walletId,
                    platformAccountIndex: controller.platformAccountIndex,
                    recipientHash: controller.recipientHash
                )
            }
        }
    }

    /// An external resume picked no account: it pays the lock's recorded
    /// recipients.
    private var accountLabel: String {
        guard let account = controller.platformAccountIndex else { return "External payment" }
        return "Platform Account #\(account)"
    }

    private var recipientLabel: String {
        guard let hash = controller.recipientHash else { return "→ recorded payees" }
        let prefix = hash
            .prefix(6)
            .map { String(format: "%02x", $0) }
            .joined()
        return "→ addr \(prefix)…"
    }

    private var phaseIcon: String {
        switch controller.phase {
        case .idle: return "circle.dashed"
        case .inFlight: return "arrow.triangle.2.circlepath"
        case .completed: return "checkmark.seal.fill"
        case .failed: return "xmark.octagon.fill"
        }
    }

    private var phaseTint: Color {
        switch controller.phase {
        case .idle, .inFlight: return .blue
        case .completed: return .green
        case .failed: return .red
        }
    }

    private var phaseLabel: String {
        switch controller.phase {
        case .idle: return "Idle"
        case .inFlight: return "Topping up…"
        case .completed: return "Done"
        case .failed: return "Failed"
        }
    }
}

/// Single row in the orphaned-asset-lock section. Renders the lock
/// summary (txid prefix, amount, status) plus a compact Resume button
/// that opens `FundFromAssetLockPlatformAddressView` in resume mode pre-seeded with
/// the outpoint.
struct ResumablePlatformFundFromAssetLockRow: View {
    let lock: PersistentAssetLock
    /// `true` when some funding is actively in flight on this wallet.
    /// Suppresses the Resume affordance on this row (at ANY status) — the
    /// in-flight funding might be driving this very lock (the controller
    /// doesn't expose the outpoint it's consuming), and resuming it would
    /// race a second consume on the same outpoint. See
    /// `PendingPlatformFundFromAssetLocksList.body`.
    let hasActiveFunding: Bool
    let onResume: () -> Void

    var body: some View {
        HStack(alignment: .top) {
            VStack(alignment: .leading, spacing: 4) {
                Text("Asset Lock \(lock.shortOutPointDisplay)")
                    .font(.body)
                    .lineLimit(1)
                HStack(spacing: 6) {
                    Text(formatDuffs(lock.amountDuffs))
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text("·")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text(lock.statusLabel)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            Spacer(minLength: 8)
            trailingAffordance
        }
        .padding(.vertical, 2)
    }

    @ViewBuilder
    private var trailingAffordance: some View {
        if hasActiveFunding {
            // A funding is in flight on this wallet. We can't tell whether
            // it's driving THIS lock (the controller doesn't expose its
            // outpoint), so offering Resume — at any status, including a
            // proof-ready (2/3) lock whose ST is mid-consume — could race a
            // second consume on the same outpoint. Keep the non-interactive
            // spinner until the funding clears. Over-suppresses a genuinely
            // orphaned lock during an unrelated fund, but only transiently.
            waitingIndicator
        } else {
            // Nothing is driving a fund on this wallet, so every visible row
            // (statusRaw 1/2/3/5, per the list's `isVisibleAsResumable`
            // filter, plus a markerless drain checkpoint at 0, which Rust
            // rebroadcasts) is a genuinely-resumable orphan with no driver: a
            // proof-ready (`canFundIdentity`, statusRaw 2/3/5) lock submits
            // as soon as a recipient is picked; a Broadcast (statusRaw 1)
            // lock re-enters the finality wait via `resume_asset_lock`.
            // Without this the Broadcast case was a permanent "Waiting…"
            // dead end.
            resumeButton
        }
    }

    private var resumeButton: some View {
        Button(action: onResume) {
            Label("Resume", systemImage: "arrow.clockwise")
                .labelStyle(.titleAndIcon)
                .font(.callout)
        }
        .buttonStyle(.borderedProminent)
        .controlSize(.small)
    }

    private var waitingIndicator: some View {
        HStack(spacing: 6) {
            ProgressView()
                .controlSize(.small)
            Text("Waiting for InstantSend / ChainLock…")
                .font(.caption)
                .foregroundColor(.secondary)
        }
    }

    private func formatDuffs(_ amountDuffs: Int64) -> String {
        let dash = Double(amountDuffs) / 1e8
        return String(format: "%g DASH", dash)
    }
}
