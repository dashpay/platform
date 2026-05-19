import SwiftUI
import SwiftData
import SwiftDashSDK

/// DPNS contest detail screen.
///
/// Drives off the platform-wallet path
/// (`ManagedPlatformWallet.fetchContestVoteState`) which returns a
/// typed `ContestVoteState` instead of the stringly-typed
/// `[String: Any]` dict the view used to parse by hand. That kills
/// the prior regex-based vote-tally extraction (the old SDK path
/// surfaced votes as Rust `Debug` strings like
/// `"ResourceVote { vote_choice: TowardsIdentity(...), strength: 1 }"`)
/// and lets the view render straight off strongly-typed fields.
///
/// The `contestInfo` dict is still accepted on the init to preserve
/// the navigation-link callers, but the view no longer reads from
/// it — fresh state comes from the wallet path on appear + on
/// pull-to-refresh.
struct ContestDetailView: View {
    let contestName: String
    /// Legacy `[String: Any]` payload from callers that predate the
    /// wallet-path migration. Kept for call-site compatibility but
    /// unused — the view reads everything off `voteState`.
    let contestInfo: [String: Any]
    /// Identity viewing the contest. Used both for "You" badging on
    /// the viewer's own contender row and for the wallet-path
    /// lookup filter.
    let currentIdentityId: String

    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Query private var identities: [PersistentIdentity]

    @State private var voteState: ContestVoteState?
    @State private var isRefreshing = false
    @State private var errorMessage: String?

    /// Current identity's 32-byte id, parsed from the base58 input
    /// parameter once. `nil` if the caller passed an unparseable id.
    private var currentIdentityData: Data? {
        Data.identifier(fromBase58: currentIdentityId)
    }

    /// Contenders sorted by vote tally descending (ties broken by
    /// identity id). The Rust-side `contest_vote_state` returns them
    /// ascending by identity id, so we sort here.
    private var sortedContenders: [ContestContender] {
        guard let state = voteState else { return [] }
        return state.contenders.sorted { lhs, rhs in
            if lhs.voteTally != rhs.voteTally {
                return lhs.voteTally > rhs.voteTally
            }
            return lhs.identityId.toHexString() < rhs.identityId.toHexString()
        }
    }

    var body: some View {
        List {
            if isRefreshing && voteState == nil {
                HStack {
                    Spacer()
                    ProgressView()
                    Text("Loading contest…")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .padding(.leading, 8)
                    Spacer()
                }
                .padding(.vertical, 8)
            } else if let errorMessage = errorMessage, voteState == nil {
                HStack(alignment: .top, spacing: 8) {
                    Image(systemName: "exclamationmark.triangle")
                        .foregroundColor(.orange)
                    Text(errorMessage)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .padding(.vertical, 8)
            }

            contestHeaderSection
            contendersSection
            voteSummarySection

            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Text("About Contested Names")
                        .font(.headline)
                    Text("When multiple identities want the same DPNS username, masternodes vote to decide the winner. The identity with the most votes will be awarded the name when voting ends.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .padding(.vertical, 4)
            }
        }
        .navigationTitle("Contest Details")
        .navigationBarTitleDisplayMode(.inline)
        .refreshable {
            await refreshVoteState()
        }
        .task {
            if voteState == nil {
                await refreshVoteState()
            }
        }
    }

    // MARK: - Sections

    @ViewBuilder
    private var contestHeaderSection: some View {
        Section("Contest Information") {
            HStack {
                Label("Name", systemImage: "at")
                Spacer()
                Text(contestName)
                    .font(.headline)
                    .foregroundColor(.blue)
            }

            if let state = voteState {
                HStack {
                    Label("Status", systemImage: "flag.fill")
                    Spacer()
                    switch state.winner {
                    case .none:
                        if state.endTime.timeIntervalSinceNow > 0 {
                            Text("Voting Ongoing")
                                .foregroundColor(.orange)
                        } else {
                            // End time passed but winner hasn't been
                            // written yet — Platform resolution lags
                            // the timestamp by a few blocks.
                            Text("Awaiting Resolution")
                                .foregroundColor(.orange)
                        }
                    case .wonByIdentity:
                        Text("Resolved")
                            .foregroundColor(.green)
                    case .locked:
                        Text("Locked")
                            .foregroundColor(.red)
                    }
                }

                HStack {
                    Label("Voting Ends", systemImage: "clock")
                    Spacer()
                    VStack(alignment: .trailing, spacing: 2) {
                        Text(state.endTime, style: .relative)
                            .font(.caption)
                            .foregroundColor(.orange)
                        Text(state.endTime, format: .dateTime.month().day().hour().minute())
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                }

                if case .none = state.winner {
                    VStack(spacing: 4) {
                        GeometryReader { geometry in
                            ZStack(alignment: .leading) {
                                Rectangle()
                                    .fill(Color.gray.opacity(0.2))
                                    .frame(height: 4)
                                    .cornerRadius(2)

                                Rectangle()
                                    .fill(timeRemainingColor(for: state.endTime))
                                    .frame(
                                        width: progressWidth(
                                            for: state.endTime,
                                            in: geometry.size.width
                                        ),
                                        height: 4
                                    )
                                    .cornerRadius(2)
                                    .animation(.easeInOut, value: state.endTime)
                            }
                        }
                        .frame(height: 4)

                        Text(timeRemainingText(for: state.endTime))
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                    .padding(.top, 4)
                }
            }
        }
    }

    @ViewBuilder
    private var contendersSection: some View {
        Section("Contenders") {
            if let state = voteState {
                // Newly-registered vs single-contender messaging —
                // shown only when the viewer is the sole contender.
                if sortedContenders.count == 1,
                   let only = sortedContenders.first,
                   only.identityId == currentIdentityData {
                    singleContenderBanner(endTime: state.endTime)
                }

                if sortedContenders.isEmpty {
                    Text("No contenders yet")
                        .font(.caption)
                        .foregroundColor(.secondary)
                } else {
                    ForEach(sortedContenders) { contender in
                        contenderRow(contender)
                    }
                }
            } else if !isRefreshing {
                Text("Contenders unavailable")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
    }

    @ViewBuilder
    private var voteSummarySection: some View {
        Section("Vote Summary") {
            let abstain = voteState?.abstainVotes ?? 0
            let lock = voteState?.lockVotes ?? 0
            let total = voteState?.totalVotes ?? 0

            HStack {
                Label("Abstain Votes", systemImage: "minus.circle")
                    .foregroundColor(.gray)
                Spacer()
                Text("\(abstain)")
                    .font(.headline)
                    .foregroundColor(abstain > 0 ? .orange : .secondary)
            }

            HStack {
                Label("Lock Votes", systemImage: "lock.fill")
                    .foregroundColor(.red)
                Spacer()
                Text("\(lock)")
                    .font(.headline)
                    .foregroundColor(lock > 0 ? .red : .secondary)
            }

            Divider()

            HStack {
                Label("Total Votes", systemImage: "sum")
                    .foregroundColor(.primary)
                    .font(.headline)
                Spacer()
                Text("\(total)")
                    .font(.headline)
                    .foregroundColor(.primary)
            }
        }
    }

    @ViewBuilder
    private func singleContenderBanner(endTime: Date) -> some View {
        let totalDuration: TimeInterval = appState.currentNetwork == .mainnet
            ? (14 * 24 * 60 * 60)       // 14 days for mainnet
            : (90 * 60)                 // 90 minutes for testnet
        let timeRemaining = endTime.timeIntervalSinceNow
        let elapsedTime = totalDuration - timeRemaining
        // Less than 5% elapsed → "newly registered"; past that →
        // "you're still the only contender".
        let isNewlyRegistered = elapsedTime < (totalDuration * 0.05)

        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Image(systemName: isNewlyRegistered ? "sparkles" : "person.fill")
                    .foregroundColor(isNewlyRegistered ? .yellow : .blue)
                Text(isNewlyRegistered ? "Newly Registered Contest" : "Only Contender")
                    .font(.headline)
            }
            Text(
                isNewlyRegistered
                    ? "You just started this contest. Other users can join as contenders until the halfway point."
                    : "You are currently the only contender for this name. Other users can still join until the halfway point."
            )
            .font(.caption)
            .foregroundColor(.secondary)
        }
        .padding(.vertical, 4)
    }

    @ViewBuilder
    private func contenderRow(_ contender: ContestContender) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                if contender.identityId == currentIdentityData {
                    Label("You", systemImage: "person.fill")
                        .font(.caption)
                        .foregroundColor(.blue)
                }
                Text(contender.identityId.toHexString())
                    .font(.system(.caption, design: .monospaced))
                    .lineLimit(1)
                    .truncationMode(.middle)
            }

            HStack {
                Label("Votes", systemImage: "hand.thumbsup.fill")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Spacer()
                Text("\(contender.voteTally) vote\(contender.voteTally == 1 ? "" : "s")")
                    .font(.caption)
                    .foregroundColor(.primary)
            }
        }
        .padding(.vertical, 4)
    }

    // MARK: - Countdown helpers

    private func timeRemainingColor(for endTime: Date) -> Color {
        let timeRemaining = endTime.timeIntervalSinceNow
        let oneDay: TimeInterval = 24 * 60 * 60

        if timeRemaining < 0 { return .red }
        if timeRemaining < oneDay { return .orange }
        if timeRemaining < oneDay * 3 { return .yellow }
        return .green
    }

    private func progressWidth(for endTime: Date, in totalWidth: CGFloat) -> CGFloat {
        let totalDuration: TimeInterval = appState.currentNetwork == .mainnet
            ? (14 * 24 * 60 * 60)
            : (90 * 60)

        let timeRemaining = max(0, endTime.timeIntervalSinceNow)
        let elapsedTime = totalDuration - timeRemaining
        let progress = min(1.0, max(0, elapsedTime / totalDuration))
        return totalWidth * CGFloat(progress)
    }

    private func timeRemainingText(for endTime: Date) -> String {
        let timeRemaining = endTime.timeIntervalSinceNow
        if timeRemaining < 0 {
            return "Contest has ended"
        }

        let formatter = DateComponentsFormatter()
        formatter.allowedUnits = [.day, .hour, .minute]
        formatter.unitsStyle = .abbreviated
        formatter.maximumUnitCount = 2

        if let formattedTime = formatter.string(from: timeRemaining) {
            return "Time remaining: \(formattedTime)"
        }
        return "Contest ending soon"
    }

    // MARK: - Refresh

    /// Fetch a fresh `ContestVoteState` via the platform-wallet
    /// path. `isRefreshing` gates both the header spinner (when we
    /// have no state yet) and the `refreshable` sheet (when pulled
    /// to refresh after initial load).
    private func refreshVoteState() async {
        guard !isRefreshing else { return }
        guard let identityData = currentIdentityData else {
            errorMessage = "Invalid identity id"
            return
        }

        // Look up the identity row via `@Query` and then hop the
        // `wallet` relationship to get the owning wallet's raw id.
        let identity = identities.first {
            $0.identityId == identityData
        }
        guard let walletId = identity?.wallet?.walletId,
              let wallet = walletManager.wallet(for: walletId) else {
            errorMessage = "Identity not attached to a loaded wallet"
            return
        }

        isRefreshing = true
        defer { isRefreshing = false }

        do {
            let state = try await wallet.fetchContestVoteState(
                identityId: identityData,
                label: contestName
            )
            voteState = state
            if state == nil {
                errorMessage = "Contest no longer visible — it may have resolved or the identity stopped contending."
            } else {
                errorMessage = nil
            }
        } catch {
            errorMessage = "Refresh failed: \(error.localizedDescription)"
        }
    }
}
