// IdentityWithdrawalsView.swift
// SwiftExampleApp
//
// Where an identity's credit withdrawals to Core stand. Each row comes from
// `ManagedPlatformWallet.identityWithdrawals`, which reads the identity's
// `withdrawal` documents from the withdrawals system contract with a proof,
// most recently updated first. Rust decides the status, its label and whether
// the withdrawal is final; this view only presents them, pages through them,
// and refreshes.
//
// The Platform side of a withdrawal ends at "Complete": Core included the
// asset unlock transaction in a chain-locked block. Platform state stores the
// transaction's asset unlock index (and the Core height a quorum signed it
// at), not its transaction id, so the row shows the index rather than a txid.

import SwiftUI
import SwiftDashSDK

struct IdentityWithdrawalsView: View {
    /// The identity whose withdrawals are listed. Its owning wallet provides
    /// the SDK the read goes through.
    let identity: PersistentIdentity

    @EnvironmentObject private var walletManager: PlatformWalletManager

    /// Rows per page.
    private static let pageSize: UInt32 = 20
    /// How often the list refreshes itself while a withdrawal is not final.
    private static let autoRefreshInterval: Duration = .seconds(15)
    /// Credits per DASH (1e11), the divisor the app uses for credit amounts.
    private static let creditsPerDash: UInt64 = 100_000_000_000

    @State private var withdrawals: [IdentityWithdrawal] = []
    @State private var nextStartAfter: Data?
    /// Pages loaded beyond the first. Auto-refresh reloads only the first
    /// page, so it pauses once the user loaded more and would lose them.
    @State private var extraPagesLoaded = 0
    @State private var hasLoaded = false
    @State private var isLoading = false
    @State private var isLoadingMore = false
    @State private var errorMessage: String?
    @State private var lastRefreshed: Date?
    /// Bumped on every reload so a late answer from an older read cannot
    /// overwrite a newer one.
    @State private var loadGeneration = 0

    private var wallet: ManagedPlatformWallet? {
        guard let walletId = identity.wallet?.walletId else { return nil }
        return walletManager.wallet(for: walletId)
    }

    /// Whether any listed withdrawal is still moving, as Rust decided it.
    private var hasOpenWithdrawals: Bool {
        withdrawals.contains { !$0.isFinal }
    }

    private var autoRefreshActive: Bool {
        hasOpenWithdrawals && extraPagesLoaded == 0 && errorMessage == nil
    }

    var body: some View {
        List {
            statusSection
            if let errorMessage {
                errorSection(errorMessage)
            }
            withdrawalsSection
        }
        .accessibilityIdentifier("identityWithdrawals.list")
        .navigationTitle("Withdrawals")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Button {
                    Task { await reload() }
                } label: {
                    if isLoading {
                        ProgressView()
                    } else {
                        Image(systemName: "arrow.clockwise")
                    }
                }
                .disabled(isLoading || wallet == nil)
                .accessibilityLabel("Refresh withdrawals")
                .accessibilityIdentifier("identityWithdrawals.refresh")
            }
        }
        .refreshable {
            await reload()
        }
        .task {
            await reload()
            // Cancelled when the view goes away, so this only polls while the
            // list is visible.
            while !Task.isCancelled {
                try? await Task.sleep(for: Self.autoRefreshInterval)
                guard !Task.isCancelled else { break }
                if autoRefreshActive && !isLoading && !isLoadingMore {
                    await reload()
                }
            }
        }
    }

    // MARK: - Sections

    private var statusSection: some View {
        Section {
            LabeledContent("Identity") {
                Text(identity.displayName)
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .foregroundStyle(.secondary)
            }
            LabeledContent("Last refreshed") {
                Text(lastRefreshed?.formatted(date: .omitted, time: .standard) ?? "Not yet")
                    .foregroundStyle(.secondary)
            }
            .accessibilityIdentifier("identityWithdrawals.lastRefreshed")
            if autoRefreshActive {
                Label(
                    "Refreshing every 15 s while a withdrawal is in progress",
                    systemImage: "clock.arrow.circlepath"
                )
                .font(.caption)
                .foregroundStyle(.secondary)
                .accessibilityIdentifier("identityWithdrawals.autoRefresh")
            }
        } footer: {
            Text(
                "A withdrawal is queued, pooled into an asset unlock transaction that a quorum "
                    + "signs, broadcast to Core, and Complete once Core includes it in a "
                    + "chain-locked block; that is where Platform's side ends. Expired means "
                    + "Core did not mine it in time and Platform signs it again. Failed means "
                    + "the payout is below Core's dust threshold, so Core never mines it.\n\n"
                    + "Platform state stores the asset unlock index of the Core transaction, "
                    + "not its transaction id, so the transaction id is not shown. "
                    + "Withdrawals from Platform addresses or the shielded pool are not listed."
            )
        }
    }

    private func errorSection(_ message: String) -> some View {
        Section {
            Label(message, systemImage: "exclamationmark.triangle.fill")
                .foregroundStyle(.red)
                .font(.subheadline)
                .accessibilityIdentifier("identityWithdrawals.error")
            Button("Try Again") {
                Task { await reload() }
            }
            .disabled(isLoading || wallet == nil)
            .accessibilityIdentifier("identityWithdrawals.retry")
        } header: {
            Text("Could not load withdrawals")
        }
    }

    @ViewBuilder
    private var withdrawalsSection: some View {
        Section("Withdrawals") {
            if wallet == nil {
                Text("The wallet that owns this identity isn't loaded.")
                    .font(.subheadline)
                    .foregroundStyle(.red)
                    .accessibilityIdentifier("identityWithdrawals.walletMissing")
            } else if withdrawals.isEmpty {
                if !hasLoaded || isLoading {
                    HStack(spacing: 8) {
                        ProgressView().controlSize(.small)
                        Text("Loading withdrawals…")
                            .foregroundStyle(.secondary)
                    }
                    .accessibilityIdentifier("identityWithdrawals.loading")
                } else if errorMessage == nil {
                    ContentUnavailableView(
                        "No withdrawals",
                        systemImage: "arrow.up.circle",
                        description: Text("This identity has not withdrawn credits to Core.")
                    )
                    .accessibilityIdentifier("identityWithdrawals.empty")
                }
            } else {
                ForEach(withdrawals) { withdrawal in
                    WithdrawalRow(
                        withdrawal: withdrawal,
                        amountText: Self.formatDash(raw: withdrawal.amountCredits)
                    )
                }
                if nextStartAfter != nil {
                    Button {
                        Task { await loadMore() }
                    } label: {
                        HStack {
                            if isLoadingMore {
                                ProgressView().controlSize(.small)
                            }
                            Text(isLoadingMore ? "Loading…" : "Load more")
                        }
                        .frame(maxWidth: .infinity)
                    }
                    .disabled(isLoadingMore || isLoading)
                    .accessibilityIdentifier("identityWithdrawals.loadMore")
                }
            }
        }
    }

    // MARK: - Loading

    /// Read the first page again, replacing what is listed.
    @MainActor
    private func reload() async {
        guard let wallet else { return }
        loadGeneration &+= 1
        let generation = loadGeneration
        isLoading = true
        defer {
            if generation == loadGeneration {
                isLoading = false
            }
        }
        do {
            let page = try await wallet.identityWithdrawals(
                identityId: identity.identityId,
                limit: Self.pageSize
            )
            guard generation == loadGeneration else { return }
            withdrawals = page.withdrawals
            nextStartAfter = page.nextStartAfter
            extraPagesLoaded = 0
            errorMessage = nil
            lastRefreshed = Date()
        } catch {
            guard generation == loadGeneration else { return }
            errorMessage = error.localizedDescription
        }
        hasLoaded = true
    }

    /// Append the next page.
    @MainActor
    private func loadMore() async {
        guard let wallet, let cursor = nextStartAfter, !isLoadingMore else { return }
        let generation = loadGeneration
        isLoadingMore = true
        defer { isLoadingMore = false }
        do {
            let page = try await wallet.identityWithdrawals(
                identityId: identity.identityId,
                limit: Self.pageSize,
                startAfter: cursor
            )
            // A reload started meanwhile replaced the list; this page belongs
            // to the old one.
            guard generation == loadGeneration else { return }
            withdrawals.append(contentsOf: page.withdrawals)
            nextStartAfter = page.nextStartAfter
            extraPagesLoaded += 1
            errorMessage = nil
        } catch {
            guard generation == loadGeneration else { return }
            errorMessage = error.localizedDescription
        }
    }

    // MARK: - Helpers

    /// Format a credit amount as `… DASH`. Mirrors
    /// `WithdrawCreditsView.formatDash`.
    private static func formatDash(raw: UInt64) -> String {
        let dash = Double(raw) / Double(creditsPerDash)
        let fmt = NumberFormatter()
        fmt.minimumFractionDigits = 0
        fmt.maximumFractionDigits = 8
        fmt.numberStyle = .decimal
        fmt.groupingSeparator = ","
        fmt.decimalSeparator = "."
        return (fmt.string(from: NSNumber(value: dash)) ?? String(format: "%.8f", dash)) + " DASH"
    }
}

// MARK: - Row

private struct WithdrawalRow: View {
    let withdrawal: IdentityWithdrawal
    let amountText: String

    /// The first four bytes of the document id in hex, used in this row's
    /// accessibility identifiers.
    private var idPrefix: String {
        withdrawal.id.prefix(4).toHexString()
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(alignment: .firstTextBaseline) {
                Text(amountText)
                    .font(.headline)
                    .monospacedDigit()
                Spacer()
                Label(withdrawal.statusLabel, systemImage: statusIcon)
                    .font(.subheadline.weight(.medium))
                    .foregroundStyle(statusColor)
                    .accessibilityIdentifier("identityWithdrawals.status.\(idPrefix)")
            }

            Group {
                if let destination = withdrawal.destination {
                    Text(destination)
                } else {
                    Text("Script \(withdrawal.outputScript.toHexString())")
                }
            }
            .font(.caption.monospaced())
            .lineLimit(2)
            .truncationMode(.middle)
            .textSelection(.enabled)

            if let index = withdrawal.transactionIndex {
                Text(coreTransactionText(index: index))
                    .font(.caption)
                    .accessibilityIdentifier("identityWithdrawals.coreTransaction.\(idPrefix)")
            }

            VStack(alignment: .leading, spacing: 2) {
                Text("Submitted \(Self.format(ms: withdrawal.createdAtMs))")
                Text("Updated \(Self.format(ms: withdrawal.updatedAtMs))")
            }
            .font(.caption2)
            .foregroundStyle(.secondary)
        }
        .padding(.vertical, 4)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("identityWithdrawals.row.\(idPrefix)")
    }

    private func coreTransactionText(index: UInt64) -> String {
        if let height = withdrawal.transactionSignHeight {
            return "Asset unlock #\(index), signed at Core height \(height)"
        }
        return "Asset unlock #\(index)"
    }

    private var statusIcon: String {
        switch withdrawal.status {
        case .queued: return "clock"
        case .pooled: return "tray.full"
        case .broadcasted: return "antenna.radiowaves.left.and.right"
        case .complete: return "checkmark.seal.fill"
        case .expired: return "arrow.clockwise.circle"
        case .failed: return "xmark.octagon.fill"
        }
    }

    private var statusColor: Color {
        switch withdrawal.status {
        case .queued: return .secondary
        case .pooled, .expired: return .orange
        case .broadcasted: return .blue
        case .complete: return .green
        case .failed: return .red
        }
    }

    private static func format(ms: UInt64) -> String {
        Date(timeIntervalSince1970: TimeInterval(ms) / 1000)
            .formatted(date: .abbreviated, time: .shortened)
    }
}
