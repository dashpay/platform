import SwiftUI
import SwiftDashSDK

/// Wallet Detail section listing this wallet's saved Max transfers
/// (Core-to-shielded wallet drains), one per stored marker, each with the
/// status `WalletDrainCoordinator` last read from Rust. A marker alone is
/// not an unfinished transfer: only a `pending` report says so. A row whose
/// `pending` report named an asset lock resumes that exact outpoint, which
/// can never build a second lock. Any other row only offers "Check again",
/// which reads the status (and retries clearing a finished one's marker).
struct UnfinishedWalletDrainsList: View {
    @ObservedObject var drainCoordinator: WalletDrainCoordinator
    /// Observed so a row appears or disappears as controllers are added to
    /// or removed from the coordinator. Phase changes are observed per row.
    @ObservedObject var fundingCoordinator: ShieldedFundFromAssetLockCoordinator
    let walletId: Data

    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var platformState: AppState
    @State private var presented: PresentedProgress?
    @State private var resumeError: ResumeError?
    @State private var checking: Set<UUID> = []
    @State private var retrying = false

    var body: some View {
        // The sheet and alert hang off an always-present container: a
        // finished resume removes the last row, and the progress sheet must
        // stay up to show the result.
        VStack(spacing: 0) {
            content
        }
        .sheet(item: $presented) { item in
            NavigationStack {
                ShieldedFundFromAssetLockProgressView(controller: item.controller)
            }
        }
        .alert(item: $resumeError) { err in
            Alert(
                title: Text("Could not resume"),
                message: Text(err.message),
                dismissButton: .default(Text("OK"))
            )
        }
    }

    @ViewBuilder
    private var content: some View {
        // `unfinished` holds every network's markers, and reconcile reads only
        // the current network's, so another network's would stay unread here.
        let networkRaw = platformState.currentNetwork.rawValue
        let entries = drainCoordinator.unfinished.filter {
            $0.record.walletId == walletId && $0.record.networkRaw == networkRaw
        }
        let storeError = drainCoordinator.storeError
        if let heading = UnfinishedWalletDrainsHeading.make(
            count: entries.count,
            storeErrorPresent: storeError != nil
        ) {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Text(heading.title)
                        .font(.headline)
                    Spacer()
                    if heading.showsRetry {
                        retryButton
                    }
                }
                .padding(.horizontal)

                if let storeError {
                    Text(storeError)
                        .font(.caption)
                        .foregroundColor(.red)
                        .padding(.horizontal)
                        .accessibilityIdentifier("walletDetail.unfinishedMaxTransfers.storeError")
                }

                if let lastKnownNotice = heading.lastKnownNotice {
                    Text(lastKnownNotice)
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .padding(.horizontal)
                }

                VStack(spacing: 0) {
                    ForEach(Array(entries.enumerated()), id: \.element.id) { idx, entry in
                        row(entry)
                            .padding(.horizontal)
                            .padding(.vertical, 10)
                        if idx < entries.count - 1 {
                            Divider()
                        }
                    }
                }
                .background(Color(UIColor.secondarySystemBackground))
                .cornerRadius(10)
                .padding(.horizontal)
            }
            .padding(.top, 8)
            .accessibilityElement(children: .contain)
            .accessibilityIdentifier("walletDetail.unfinishedMaxTransfers")
        }
    }

    private var retryButton: some View {
        Button {
            retryStoreRead()
        } label: {
            if retrying {
                ProgressView()
                    .controlSize(.small)
            } else {
                Label("Try again", systemImage: "arrow.clockwise")
                    .labelStyle(.titleAndIcon)
                    .font(.callout)
            }
        }
        .buttonStyle(.bordered)
        .controlSize(.small)
        .disabled(retrying)
        .accessibilityIdentifier("walletDetail.unfinishedMaxTransfers.retryButton")
    }

    /// A row with a controller observes it, so its action follows the live
    /// phase: the funding coordinator does not republish phase changes.
    @ViewBuilder
    private func row(_ entry: WalletDrainCoordinator.Entry) -> some View {
        let isChecking = checking.contains(entry.id)
        if let controller = controller(for: entry) {
            ObservedDrainRow(
                entry: entry,
                controller: controller,
                isChecking: isChecking,
                onResume: { resume(entry.record, outpoint: $0) },
                onCheckAgain: { checkAgain(entry.record) }
            )
        } else {
            DrainRowContent(
                entry: entry,
                action: WalletDrainCoordinator.rowAction(
                    controllerPhase: nil,
                    reportedState: entry.reportedState,
                    outpoint: entry.outpoint
                ),
                status: WalletDrainCoordinator.rowStatus(controllerPhase: nil, entry: entry),
                controller: nil,
                isChecking: isChecking,
                onResume: { resume(entry.record, outpoint: $0) },
                onCheckAgain: { checkAgain(entry.record) }
            )
        }
    }

    /// This operation's controller in any phase, matched by its id.
    private func controller(
        for entry: WalletDrainCoordinator.Entry
    ) -> ShieldedFundFromAssetLockController? {
        let operationId = WalletDrainCoordinator.controllerOperationId(entry.record.operationId)
        return fundingCoordinator.activeControllers().first { $0.operationId == operationId }
    }

    private func resume(_ record: WalletDrainOperationRecord, outpoint: WalletDrainOutpoint) {
        // A rebuild stops SPV, so a resume that still needs a ChainLock
        // would wait for one that never comes; the rebuild in turn refuses
        // to start while a funding runs.
        guard !platformState.isRebuildingSdk else {
            resumeError = ResumeError(message: WalletDrainCoordinator.rebuildInProgressMessage)
            return
        }
        switch drainCoordinator.resume(record, outpoint: outpoint, walletManager: walletManager) {
        case .started(let controller, _):
            presented = PresentedProgress(controller: controller)
        case .blockedByOtherWalletFunding(let blocker):
            resumeError = ResumeError(
                message: "Another shielded funding is in progress on this wallet "
                    + "(recipient \(hexShort(blocker.recipientRaw43))). Resume this transfer "
                    + "after that one finishes."
            )
        }
    }

    private func checkAgain(_ record: WalletDrainOperationRecord) {
        let id = record.operationId
        guard !checking.contains(id) else { return }
        checking.insert(id)
        Task {
            await drainCoordinator.checkAgain(record, walletManager: walletManager)
            checking.remove(id)
        }
    }

    private func retryStoreRead() {
        guard !retrying else { return }
        retrying = true
        let networkRaw = platformState.currentNetwork.rawValue
        Task {
            await drainCoordinator.retryStoreRead(
                walletManager: walletManager,
                networkRaw: networkRaw,
                walletIds: [walletId]
            )
            retrying = false
        }
    }

    private struct PresentedProgress: Identifiable {
        let id = UUID()
        let controller: ShieldedFundFromAssetLockController
    }

    private struct ResumeError: Identifiable {
        let id = UUID()
        let message: String
    }
}

/// Row for an operation that has a controller: re-renders on its phase.
private struct ObservedDrainRow: View {
    let entry: WalletDrainCoordinator.Entry
    @ObservedObject var controller: ShieldedFundFromAssetLockController
    let isChecking: Bool
    let onResume: (WalletDrainOutpoint) -> Void
    let onCheckAgain: () -> Void

    var body: some View {
        DrainRowContent(
            entry: entry,
            action: WalletDrainCoordinator.rowAction(
                controllerPhase: controller.phase,
                reportedState: entry.reportedState,
                outpoint: entry.outpoint
            ),
            status: WalletDrainCoordinator.rowStatus(controllerPhase: controller.phase, entry: entry),
            controller: controller,
            isChecking: isChecking,
            onResume: onResume,
            onCheckAgain: onCheckAgain
        )
    }
}

private struct DrainRowContent: View {
    let entry: WalletDrainCoordinator.Entry
    let action: WalletDrainCoordinator.RowAction
    let status: WalletDrainCoordinator.RowStatus
    let controller: ShieldedFundFromAssetLockController?
    let isChecking: Bool
    let onResume: (WalletDrainOutpoint) -> Void
    let onCheckAgain: () -> Void

    var body: some View {
        HStack(alignment: .top) {
            VStack(alignment: .leading, spacing: 4) {
                Text("Max to \(hexShort(entry.record.recipientRaw43))")
                    .font(.body)
                    .lineLimit(1)
                if let outpoint = entry.outpoint {
                    Text("Asset lock \(outpointLabel(outpoint))")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
                statusLine
            }
            Spacer(minLength: 8)
            actionView
        }
        .padding(.vertical, 2)
    }

    /// Before the first read the row offers no Resume. Check again stays:
    /// if no reconcile reaches this marker, it is the only way to read it.
    @ViewBuilder
    private var statusLine: some View {
        let text = Text(status.text)
            .font(.caption)
            .foregroundColor(statusColor)
        if status == .checking {
            text.accessibilityIdentifier("walletDetail.unfinishedMaxTransfer.checkingStatus")
        } else {
            text.accessibilityIdentifier("walletDetail.unfinishedMaxTransfer.status")
        }
    }

    private var statusColor: Color {
        switch status {
        case .readFailed, .finishedNotCleared, .notSentNotCleared:
            return .red
        case .unfinished, .unfinishedLockUnknown:
            return .orange
        case .inProgress, .checking, .notSentWhileAttemptStarted:
            return .secondary
        }
    }

    @ViewBuilder
    private var actionView: some View {
        switch action {
        case .inProgress:
            if let controller {
                NavigationLink(destination: ShieldedFundFromAssetLockProgressView(controller: controller)) {
                    inProgressLabel
                }
                .buttonStyle(.plain)
            } else {
                inProgressLabel
            }
        case .resume(let outpoint):
            Button {
                onResume(outpoint)
            } label: {
                Label("Resume", systemImage: "arrow.clockwise")
                    .labelStyle(.titleAndIcon)
                    .font(.callout)
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.small)
            .accessibilityIdentifier("walletDetail.unfinishedMaxTransfer.resumeButton")
        case .checkAgain:
            Button {
                onCheckAgain()
            } label: {
                if isChecking {
                    ProgressView()
                        .controlSize(.small)
                } else {
                    Label("Check again", systemImage: "magnifyingglass")
                        .labelStyle(.titleAndIcon)
                        .font(.callout)
                }
            }
            .buttonStyle(.bordered)
            .controlSize(.small)
            .disabled(isChecking)
            .accessibilityIdentifier("walletDetail.unfinishedMaxTransfer.checkAgainButton")
        }
    }

    private var inProgressLabel: some View {
        HStack(spacing: 6) {
            ProgressView()
                .controlSize(.small)
            Text("In progress")
                .font(.caption)
                .foregroundColor(.secondary)
            Image(systemName: "chevron.right")
                .font(.caption)
                .foregroundColor(.secondary)
        }
    }

    /// Display order txid (reversed wire bytes), as asset-lock rows show it.
    private func outpointLabel(_ outpoint: WalletDrainOutpoint) -> String {
        let txid = outpoint.txidWire.reversed().map { String(format: "%02x", $0) }.joined()
        return "\(txid.prefix(8))…:\(outpoint.vout)"
    }
}

/// What the section's heading claims. Pure over plain values so it is
/// testable. It counts saved records, not unfinished transfers: each row
/// says what Rust reported for it. A failed marker-file read leaves the
/// coordinator's list as an earlier read left it (empty on a cold launch),
/// so while it is failed the heading gives no count.
struct UnfinishedWalletDrainsHeading: Equatable {
    let title: String
    /// Says that the rows listed come from an earlier successful read.
    let lastKnownNotice: String?
    /// Section-level "Try again", which works without any row.
    let showsRetry: Bool

    /// `nil` when there is nothing to show: no rows and the read succeeded.
    static func make(count: Int, storeErrorPresent: Bool) -> UnfinishedWalletDrainsHeading? {
        if storeErrorPresent {
            return UnfinishedWalletDrainsHeading(
                title: "Saved Max transfers",
                lastKnownNotice: count > 0
                    ? "Last known from an earlier read. This list may be out of date."
                    : nil,
                showsRetry: true
            )
        }
        guard count > 0 else { return nil }
        return UnfinishedWalletDrainsHeading(
            title: "Saved Max transfers (\(count))",
            lastKnownNotice: nil,
            showsRetry: false
        )
    }
}

private func hexShort(_ data: Data) -> String {
    let hex = data.map { String(format: "%02x", $0) }.joined()
    if hex.count <= 16 { return hex }
    return "\(hex.prefix(8))…\(hex.suffix(8))"
}
