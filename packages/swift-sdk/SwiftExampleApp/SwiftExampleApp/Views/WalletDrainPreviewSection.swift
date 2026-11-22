import SwiftUI
import SwiftDashSDK

/// Read-only estimate for a Core-to-shielded Max transfer, shown in place
/// of the amount field. Every figure is Rust's
/// `shieldedWalletDrainPreview` except "Stays in Core" (see `staysInCore`).
/// The loaded preview is written to `preview` so the parent can gate submit.
struct WalletDrainPreviewSection: View {
    @EnvironmentObject var walletManager: PlatformWalletManager

    let walletId: Data
    let fundingAccountIndex: UInt32
    @Binding var preview: WalletDrainPreview?

    @State private var isLoading = false
    @State private var loadError: String?
    @State private var refreshTick = 0

    private struct LoadKey: Hashable {
        let fundingAccountIndex: UInt32
        let refreshTick: Int
    }

    var body: some View {
        Section {
            if let preview {
                rows(preview)
            } else if let loadError {
                Text(loadError)
                    .font(.caption)
                    .foregroundColor(.red)
                    .accessibilityIdentifier("shieldFromCore.maxPreview.status")
            } else if isLoading {
                HStack(spacing: 12) {
                    ProgressView()
                    Text("Estimating…")
                        .foregroundColor(.secondary)
                }
            }
            Button {
                refreshTick += 1
            } label: {
                Label("Refresh", systemImage: "arrow.clockwise")
            }
            .disabled(isLoading)
            .accessibilityIdentifier("shieldFromCore.maxPreview.refreshButton")
        } header: {
            Text("Max (estimates)")
        } footer: {
            Text(
                "No change output is created. Coins that are not final yet, locked coins "
                    + "and the CoinJoin account are not swept. The figures can differ when the "
                    + "transfer runs, because inputs reserved by other operations are counted here."
            )
        }
        .task(id: LoadKey(fundingAccountIndex: fundingAccountIndex, refreshTick: refreshTick)) {
            await load()
        }
    }

    @ViewBuilder
    private func rows(_ preview: WalletDrainPreview) -> some View {
        valueRow(
            "Swept from Core (est.)",
            "\(formatDuffs(preview.candidateBalanceDuffs)), \(preview.candidateInputCount) inputs",
            id: "shieldFromCore.maxPreview.swept"
        )
        valueRow(
            "Core network fee (est.)",
            preview.estimatedL1FeeDuffs.map { formatDuffs($0) } ?? "not available",
            id: "shieldFromCore.maxPreview.l1Fee"
        )
        valueRow(
            "Shielded pool fee (est.)",
            formatCredits(preview.poolFeeCredits),
            id: "shieldFromCore.maxPreview.poolFee"
        )
        valueRow(
            "You receive, shielded (est.)",
            preview.estimatedRecipientCredits.map { formatCredits($0) } ?? "not available",
            id: "shieldFromCore.maxPreview.receive"
        )
        valueRow(
            "Stays in Core (est.)",
            formatDuffs(staysInCore(preview)),
            id: "shieldFromCore.maxPreview.staysInCore"
        )
        statusRow(preview.status)
    }

    @ViewBuilder
    private func statusRow(_ status: WalletDrainPreview.Status) -> some View {
        Group {
            switch status {
            case .ready:
                Label("Ready", systemImage: "checkmark.circle")
                    .foregroundColor(.secondary)
            case .insufficientFunds:
                Label(
                    "The balance does not cover the fees, so there is nothing to shield.",
                    systemImage: "xmark.octagon"
                )
                .foregroundColor(.red)
            case .inputLimitMayBeExceeded:
                Label(
                    "This wallet has more inputs than one asset-lock transaction can take. "
                        + "The transfer may be refused.",
                    systemImage: "exclamationmark.triangle"
                )
                .foregroundColor(.orange)
            }
        }
        .font(.caption)
        .accessibilityIdentifier("shieldFromCore.maxPreview.status")
    }

    private func valueRow(_ label: String, _ value: String, id: String) -> some View {
        HStack {
            Text(label)
            Spacer()
            Text(value)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.trailing)
        }
        .font(.callout)
        .accessibilityElement(children: .combine)
        .accessibilityIdentifier(id)
    }

    /// Display-only difference of two Rust-reported balances: the wallet's
    /// Core total as the balance card sums it, minus the preview's swept
    /// candidate balance. Not a Rust figure; saturates at zero.
    private func staysInCore(_ preview: WalletDrainPreview) -> UInt64 {
        let total = WalletCoreBalance(
            accountBalances: walletManager.accountBalances(for: walletId)
        ).total
        return total > preview.candidateBalanceDuffs ? total - preview.candidateBalanceDuffs : 0
    }

    private func load() async {
        isLoading = true
        loadError = nil
        preview = nil
        do {
            let loaded = try await walletManager.shieldedWalletDrainPreview(
                walletId: walletId,
                fundingAccountIndex: fundingAccountIndex
            )
            guard !Task.isCancelled else { return }
            preview = loaded
        } catch {
            guard !Task.isCancelled else { return }
            loadError = "Could not estimate: \(error.localizedDescription)"
        }
        isLoading = false
    }

    private func formatDuffs(_ duffs: UInt64) -> String {
        String(format: "%.8f DASH", Double(duffs) / 100_000_000.0)
    }

    private func formatCredits(_ credits: UInt64) -> String {
        String(format: "%.8f DASH (credits)", Double(credits) / 100_000_000_000.0)
    }
}
