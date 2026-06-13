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

    // MARK: - Vote-casting state
    /// Presents the masternode vote sheet.
    @State private var showVoteSheet = false
    /// In-flight broadcast guard.
    @State private var isCastingVote = false
    /// Result banner shown after a cast attempt (success or the
    /// deterministic authorization rejection a non-masternode hits).
    @State private var voteResultMessage: String?
    @State private var voteResultIsError = false

    /// DPNS contest poll shape — the same `(contract, document type,
    /// index, index values)` tuple the read path uses. The contested
    /// label lives at the second index value (`["dash", label]`).
    private static let dpnsContractId = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"
    private static let dpnsDocumentType = "domain"
    private static let dpnsIndexName = "parentNameAndLabel"
    private var dpnsIndexValues: [String] { ["dash", contestName] }

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
            castVoteSection

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
        .sheet(isPresented: $showVoteSheet) {
            CastVoteSheet(
                contestName: contestName,
                contenders: sortedContenders,
                currentIdentityData: currentIdentityData,
                isCasting: isCastingVote,
                onSubmit: { choice, proTxHashHex, votingKeyHex in
                    await castVote(
                        choice: choice,
                        proTxHashHex: proTxHashHex,
                        votingKeyHex: votingKeyHex
                    )
                }
            )
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
    private var castVoteSection: some View {
        Section("Cast a Masternode Vote") {
            if let message = voteResultMessage {
                HStack(alignment: .top, spacing: 8) {
                    Image(systemName: voteResultIsError
                        ? "exclamationmark.triangle"
                        : "checkmark.circle.fill")
                        .foregroundColor(voteResultIsError ? .orange : .green)
                    Text(message)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .padding(.vertical, 4)
            }

            Button {
                voteResultMessage = nil
                showVoteSheet = true
            } label: {
                Label("Cast Vote…", systemImage: "hand.raised.fill")
            }
            .disabled(appState.sdk == nil || isCastingVote)

            // Contested-resource votes are masternode-only. Spell that
            // out so a regular-wallet user understands why a broadcast
            // is expected to be rejected, and what credentials make it
            // succeed.
            Text("Only masternodes can vote on contested names, using a masternode voting key tied to a pro_tx_hash. A regular wallet has no voting key, so a vote here will be rejected by Platform. Supply real masternode credentials in the sheet to cast an accepted vote.")
                .font(.caption2)
                .foregroundColor(.secondary)
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

    // MARK: - Vote casting

    /// Broadcast a masternode contested-resource vote through the SDK.
    ///
    /// Marshals the hex inputs into `Data` and forwards to
    /// `SDK.castContestedResourceVote`, which bridges straight to the
    /// rs-sdk `PutVote` path. The transition is fully assembled, signed
    /// and broadcast; a non-masternode caller reaches a deterministic
    /// authorization rejection (surfaced here as an error banner).
    private func castVote(
        choice: ContestedResourceVoteChoice,
        proTxHashHex: String,
        votingKeyHex: String
    ) async {
        guard let sdk = appState.sdk else {
            voteResultMessage = "SDK not initialized"
            voteResultIsError = true
            return
        }
        // `Data(hexString:)` decodes `count / 2` bytes stepping by two, so an
        // odd-length (e.g. 65-char) string silently drops its trailing nibble
        // and still yields 32 bytes — the `count == 32` guard alone would pass
        // a malformed key. Validate the trimmed hex length is exactly 64 (and
        // therefore even) before decoding so a wrong key can't slip through.
        let normalizedProTxHashHex = proTxHashHex.trimmingCharacters(in: .whitespacesAndNewlines)
        guard normalizedProTxHashHex.count == 64,
              let proTxHash = Data(hexString: normalizedProTxHashHex),
              proTxHash.count == 32 else {
            voteResultMessage = "pro_tx_hash must be 32 bytes (64 hex characters)."
            voteResultIsError = true
            return
        }
        let normalizedVotingKeyHex = votingKeyHex.trimmingCharacters(in: .whitespacesAndNewlines)
        guard normalizedVotingKeyHex.count == 64,
              let votingKey = Data(hexString: normalizedVotingKeyHex),
              votingKey.count == 32 else {
            voteResultMessage = "Voting private key must be 32 bytes (64 hex characters)."
            voteResultIsError = true
            return
        }

        isCastingVote = true
        defer { isCastingVote = false }

        do {
            try await sdk.castContestedResourceVote(
                dataContractId: Self.dpnsContractId,
                documentTypeName: Self.dpnsDocumentType,
                indexName: Self.dpnsIndexName,
                indexValues: dpnsIndexValues,
                choice: choice,
                proTxHash: proTxHash,
                votingPrivateKey: votingKey
            )
            voteResultMessage = "Vote accepted by Platform."
            voteResultIsError = false
            showVoteSheet = false
            // Pull fresh tallies after a successful cast.
            await refreshVoteState()
        } catch {
            voteResultMessage = "Vote rejected: \(error.localizedDescription)"
            voteResultIsError = true
            showVoteSheet = false
        }
    }
}

/// Modal for picking a vote choice and entering masternode credentials.
///
/// Kept deliberately explicit about the masternode-key requirement —
/// the example app's regular wallets cannot satisfy it, so the inputs
/// are where a masternode operator would paste their pro_tx_hash and
/// voting private key (both hex).
private struct CastVoteSheet: View {
    let contestName: String
    let contenders: [ContestContender]
    let currentIdentityData: Data?
    let isCasting: Bool
    let onSubmit: (ContestedResourceVoteChoice, String, String) async -> Void

    @Environment(\.dismiss) private var dismiss

    /// Selected contender identity (hex) or one of the special choices.
    private enum Selection: Hashable {
        case contender(String) // identity id hex
        case abstain
        case lock
    }

    @State private var selection: Selection = .abstain
    @State private var proTxHashHex = ""
    @State private var votingKeyHex = ""
    /// Surfaced when the selected contender's hex id fails to decode, so we
    /// never pass un-decodable hex downstream to the FFI as if it were base58.
    @State private var choiceError: String?

    var body: some View {
        NavigationView {
            Form {
                Section("Voting on") {
                    Text("\(contestName).dash")
                        .font(.headline)
                        .foregroundColor(.blue)
                }

                Section("Your Choice") {
                    Picker("Vote", selection: $selection) {
                        ForEach(contenders) { contender in
                            let hex = contender.identityId.toHexString()
                            HStack {
                                if contender.identityId == currentIdentityData {
                                    Text("You")
                                } else {
                                    Text(hex.prefix(12) + "…")
                                }
                            }
                            .tag(Selection.contender(hex))
                        }
                        Text("Abstain").tag(Selection.abstain)
                        Text("Lock (no winner)").tag(Selection.lock)
                    }
                    .pickerStyle(.inline)
                }

                Section("Masternode Credentials") {
                    TextField("pro_tx_hash (64 hex chars)", text: $proTxHashHex)
                        .font(.system(.caption, design: .monospaced))
                        .autocorrectionDisabled()
                        .textInputAutocapitalization(.never)
                    SecureField("voting private key (64 hex chars)", text: $votingKeyHex)
                        .font(.system(.caption, design: .monospaced))
                        .autocorrectionDisabled()
                        .textInputAutocapitalization(.never)
                    Text("These belong to a masternode. The voting public key (ECDSA_HASH160) and signer are derived from the private key on the Rust side; the key bytes are not stored.")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }

                Section {
                    Button {
                        // Guarded decode: if the selected contender's hex id
                        // can't be decoded, surface a clear error instead of
                        // passing raw hex to the FFI as if it were base58.
                        guard let choice = resolvedChoice else {
                            choiceError = "Could not decode the selected contender's identity id."
                            return
                        }
                        choiceError = nil
                        Task {
                            await onSubmit(choice, proTxHashHex, votingKeyHex)
                        }
                    } label: {
                        if isCasting {
                            HStack {
                                ProgressView()
                                Text("Broadcasting…")
                            }
                        } else {
                            Text("Submit Vote")
                        }
                    }
                    .disabled(isCasting || proTxHashHex.isEmpty || votingKeyHex.isEmpty)

                    if let choiceError {
                        Text(choiceError)
                            .font(.caption)
                            .foregroundColor(.orange)
                    }
                }
            }
            .navigationTitle("Cast Vote")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                        .disabled(isCasting)
                }
            }
        }
    }

    /// Translate the picker selection into the SDK vote-choice enum.
    ///
    /// Returns `nil` when a contender's hex identity id fails to decode, so the
    /// caller can surface a clear error instead of passing un-decodable hex
    /// downstream to the FFI as if it were base58.
    private var resolvedChoice: ContestedResourceVoteChoice? {
        switch selection {
        case .contender(let hex):
            // The FFI expects a base58 identity id; convert from the hex the
            // contender rows carry. Guard the decode rather than falling back
            // to the raw hex string.
            guard let base58 = Data(hexString: hex)?.toBase58String() else {
                return nil
            }
            return .towardsIdentity(base58)
        case .abstain:
            return .abstain
        case .lock:
            return .lock
        }
    }
}
