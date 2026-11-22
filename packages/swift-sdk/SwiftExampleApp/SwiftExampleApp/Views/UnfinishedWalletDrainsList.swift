import SwiftUI
import SwiftDashSDK

/// Wallet Detail section listing this wallet's unfinished Max transfers
/// (Core-to-shielded wallet drains) as `WalletDrainCoordinator` last read
/// them from Rust. A row resumes by the stored operation id, which makes
/// Rust continue the same asset lock rather than build a second one.
struct UnfinishedWalletDrainsList: View {
    @ObservedObject var drainCoordinator: WalletDrainCoordinator
    /// Observed so a row flips between "In progress" and "Resume" as the
    /// shared shielded-funding controller changes phase.
    @ObservedObject var fundingCoordinator: ShieldedFundFromAssetLockCoordinator
    let walletId: Data

    @EnvironmentObject var walletManager: PlatformWalletManager
    @State private var presented: PresentedProgress?
    @State private var resumeError: ResumeError?

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
        let entries = drainCoordinator.unfinished.filter { $0.record.walletId == walletId }
        if !entries.isEmpty || drainCoordinator.storeError != nil {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Text("Unfinished Max Transfers (\(entries.count))")
                        .font(.headline)
                    Spacer()
                }
                .padding(.horizontal)

                Text(
                    "Nothing is lost. A new Max transfer cannot start until this one is finished."
                )
                .font(.caption)
                .foregroundColor(.secondary)
                .padding(.horizontal)

                if let storeError = drainCoordinator.storeError {
                    Text(storeError)
                        .font(.caption)
                        .foregroundColor(.red)
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

    @ViewBuilder
    private func row(_ entry: WalletDrainCoordinator.Entry) -> some View {
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
                if let readError = entry.readError {
                    Text(readError)
                        .font(.caption)
                        .foregroundColor(.red)
                }
            }
            Spacer(minLength: 8)
            if let controller = inFlightController(for: entry) {
                NavigationLink(destination: ShieldedFundFromAssetLockProgressView(controller: controller)) {
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
                .buttonStyle(.plain)
            } else {
                Button {
                    resume(entry.record)
                } label: {
                    Label("Resume", systemImage: "arrow.clockwise")
                        .labelStyle(.titleAndIcon)
                        .font(.callout)
                }
                .buttonStyle(.borderedProminent)
                .controlSize(.small)
                .accessibilityIdentifier("walletDetail.unfinishedMaxTransfer.resumeButton")
            }
        }
        .padding(.vertical, 2)
    }

    private func inFlightController(
        for entry: WalletDrainCoordinator.Entry
    ) -> ShieldedFundFromAssetLockController? {
        let operationId = WalletDrainCoordinator.controllerOperationId(entry.record.operationId)
        return fundingCoordinator.activeControllers().first {
            $0.operationId == operationId && $0.phase.isActive
        }
    }

    private func resume(_ record: WalletDrainOperationRecord) {
        switch drainCoordinator.resume(record, walletManager: walletManager) {
        case .started(let controller):
            presented = PresentedProgress(controller: controller)
        case .blockedByOtherWalletFunding(let blocker):
            resumeError = ResumeError(
                message: "Another shielded funding is in progress on this wallet "
                    + "(recipient \(hexShort(blocker.recipientRaw43))). Resume this transfer "
                    + "after that one finishes."
            )
        }
    }

    /// Display order txid (reversed wire bytes), as asset-lock rows show it.
    private func outpointLabel(_ outpoint: WalletDrainOutpoint) -> String {
        let txid = outpoint.txidWire.reversed().map { String(format: "%02x", $0) }.joined()
        return "\(txid.prefix(8))…:\(outpoint.vout)"
    }

    private func hexShort(_ data: Data) -> String {
        let hex = data.map { String(format: "%02x", $0) }.joined()
        if hex.count <= 16 { return hex }
        return "\(hex.prefix(8))…\(hex.suffix(8))"
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
