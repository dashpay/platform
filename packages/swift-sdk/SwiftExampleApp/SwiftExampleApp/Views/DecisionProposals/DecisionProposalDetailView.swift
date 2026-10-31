import SwiftUI
import SwiftData
import SwiftDashSDK

/// One decision proposal: its text and rule, its yes/no poll (open with its
/// tallies, or finished with its result), a masternode vote, and the text
/// edit for a proposal a local identity filed.
///
/// Everything shown is read from Platform through the `SDK` decision
/// proposal wrappers on appear and on pull-to-refresh. Whether a vote or an
/// edit is accepted is Platform's call; this screen only collects input and
/// shows the outcome.
struct DecisionProposalDetailView: View {
    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Query private var identities: [PersistentIdentity]

    @State private var proposal: DecisionProposal
    @State private var pollState: YesNoVotePollState?
    /// `true` once the poll state has been read, so "never opened" is not
    /// shown while it is still loading.
    @State private var pollStateLoaded = false
    @State private var pollId: String?
    @State private var isRefreshing = false
    @State private var errorMessage: String?

    @State private var showVoteSheet = false
    @State private var showEditSheet = false
    /// Banner after a vote or an edit.
    @State private var actionMessage: String?
    @State private var actionMessageIsError = false

    init(proposal: DecisionProposal) {
        _proposal = State(initialValue: proposal)
    }

    /// The proposer, when it is an identity on this device held by a loaded
    /// wallet, so it can sign an edit.
    private var localOwner: PersistentIdentity? {
        identities.first {
            $0.network == appState.currentNetwork
                && $0.wallet != nil
                && $0.identityIdBase58 == proposal.ownerId
        }
    }

    var body: some View {
        List {
            if let errorMessage {
                Section {
                    Label(errorMessage, systemImage: "exclamationmark.triangle")
                        .font(.caption)
                        .foregroundColor(.orange)
                }
            }

            proposalSection
            ruleSection
            pollSection
            actionsSection
        }
        .navigationTitle("Proposal")
        .navigationBarTitleDisplayMode(.inline)
        .refreshable {
            await refresh()
        }
        .task {
            await refresh()
        }
        .sheet(isPresented: $showVoteSheet) {
            DecisionProposalVoteSheet(
                proposalId: proposal.id,
                proposalTitle: proposal.title
            ) { message in
                actionMessage = message
                actionMessageIsError = false
                Task { await refresh() }
            }
            .environmentObject(appState)
        }
        .sheet(isPresented: $showEditSheet) {
            if let owner = localOwner {
                EditDecisionProposalView(proposal: proposal, owner: owner) { message in
                    actionMessage = message
                    actionMessageIsError = false
                    Task { await refresh() }
                }
                .environmentObject(appState)
                .environmentObject(walletManager)
            }
        }
    }

    // MARK: - Sections

    private var proposalSection: some View {
        Section("Proposal") {
            Text(proposal.title)
                .font(.headline)
                .fixedSize(horizontal: false, vertical: true)
            Text(proposal.description)
                .font(.body)
                .fixedSize(horizontal: false, vertical: true)
            if let urlString = proposal.url {
                if let url = URL(string: urlString) {
                    Link(urlString, destination: url)
                        .font(.caption)
                } else {
                    Text(urlString)
                        .font(.caption)
                }
            }
            DetailRow(label: "Proposal ID", value: proposal.id)
            DetailRow(
                label: "Proposer",
                value: localOwner.map { "\($0.alias ?? $0.identityIdBase58) (this device)" }
                    ?? proposal.ownerId
            )
            if let revision = proposal.revision {
                DetailRow(label: "Revision", value: "\(revision)")
            }
            if let createdAt = proposal.createdAt {
                DetailRow(label: "Created", value: createdAt.formatted(date: .abbreviated, time: .shortened))
            }
            if let updatedAt = proposal.updatedAt, updatedAt != proposal.createdAt {
                DetailRow(label: "Edited", value: updatedAt.formatted(date: .abbreviated, time: .shortened))
            }
        }
    }

    private var ruleSection: some View {
        Section {
            DetailRow(label: "Pass", value: "Yes is more than \(proposal.passPercent)% of yes plus no")
            DetailRow(
                label: "Minimum turnout",
                value: "Yes plus no reaches \(proposal.minimumTurnoutPercent)% of the total voting power"
            )
            HStack {
                Label("Voting ends", systemImage: "clock")
                Spacer()
                VStack(alignment: .trailing, spacing: 2) {
                    Text(proposal.endDate, style: .relative)
                        .font(.caption)
                        .foregroundColor(.orange)
                    Text(proposal.endDate, format: .dateTime.month().day().hour().minute())
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
            }
            if let poll = pollState?.votePoll {
                DetailRow(
                    label: "Poll supermajority",
                    value: "\(poll.supermajorityNumerator)/\(poll.supermajorityDenominator)"
                )
                DetailRow(label: "Poll minimum voting power", value: Self.describe(poll.minimumVotingPower))
            }
        } header: {
            Text("Rule")
        } footer: {
            Text("The rule and the end date never change after the proposal is created.")
        }
    }

    @ViewBuilder
    private var pollSection: some View {
        Section {
            if !pollStateLoaded {
                HStack {
                    ProgressView()
                    Text("Loading poll…")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            } else if let pollState {
                switch pollState.status {
                case .started(let startBlock):
                    statusRow(text: "Voting open", color: .orange)
                    blockRow(label: "Opened", block: startBlock)
                    tallyRows(
                        yes: pollState.yesVotingPower,
                        no: pollState.noVotingPower,
                        abstain: pollState.abstainVotingPower
                    )
                case .finished(let result):
                    statusRow(
                        text: result.passed ? "Passed" : "Did not pass",
                        color: result.passed ? .green : .red
                    )
                    tallyRows(
                        yes: result.yesVotingPower,
                        no: result.noVotingPower,
                        abstain: result.abstainVotingPower
                    )
                    HStack {
                        Label("Required yes plus no", systemImage: "person.3")
                        Spacer()
                        Text("\(result.requiredVotingPower)")
                            .font(.headline)
                    }
                    blockRow(label: "Opened", block: result.startBlock)
                    blockRow(label: "Closed", block: result.finalizationBlock)
                }
            } else {
                Text("Platform has no poll for this proposal.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            if let pollId {
                DetailRow(label: "Poll ID", value: pollId)
            }
        } header: {
            Text("Poll")
        } footer: {
            Text("Voting power counts a masternode 1 and an evonode 4. Abstaining is recorded but plays no part in the result.")
        }
    }

    private var actionsSection: some View {
        Section("Actions") {
            if let message = actionMessage {
                Label(
                    message,
                    systemImage: actionMessageIsError ? "exclamationmark.triangle" : "checkmark.circle.fill"
                )
                .font(.caption)
                .foregroundColor(actionMessageIsError ? .orange : .green)
            }

            Button {
                actionMessage = nil
                showVoteSheet = true
            } label: {
                Label("Vote as Masternode…", systemImage: "hand.raised.fill")
            }
            // A finished poll keeps no votes; Platform refuses one.
            .disabled(appState.sdk == nil || pollState?.result != nil)
            .accessibilityIdentifier("decisionProposal.vote")

            if localOwner != nil {
                Button {
                    actionMessage = nil
                    showEditSheet = true
                } label: {
                    Label("Edit Text…", systemImage: "pencil")
                }
                .disabled(appState.sdk == nil)
                .accessibilityIdentifier("decisionProposal.edit")

                Text("Platform accepts an edit of the title, description and url for a day after the proposal is created (or a fifth of its voting time, if that is shorter).")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }

            Text("Only masternodes vote, with the voting key of a pro_tx_hash. A regular wallet has no voting key, so Platform refuses its vote.")
                .font(.caption2)
                .foregroundColor(.secondary)
        }
    }

    // MARK: - Rows

    private func statusRow(text: String, color: Color) -> some View {
        HStack {
            Label("Status", systemImage: "flag.fill")
            Spacer()
            Text(text)
                .foregroundColor(color)
        }
    }

    private func blockRow(label: String, block: PlatformBlockInfo) -> some View {
        DetailRow(
            label: label,
            value: "Block \(block.height), epoch \(block.epoch), \(block.time.formatted(date: .abbreviated, time: .shortened))"
        )
    }

    @ViewBuilder
    private func tallyRows(yes: UInt64, no: UInt64, abstain: UInt64) -> some View {
        tallyRow("Yes", systemImage: "hand.thumbsup.fill", value: yes, color: .green)
        tallyRow("No", systemImage: "hand.thumbsdown.fill", value: no, color: .red)
        tallyRow("Abstain", systemImage: "minus.circle", value: abstain, color: .gray)
    }

    private func tallyRow(_ title: String, systemImage: String, value: UInt64, color: Color) -> some View {
        HStack {
            Label(title, systemImage: systemImage)
                .foregroundColor(color)
            Spacer()
            Text("\(value)")
                .font(.headline)
        }
    }

    private static func describe(_ minimum: YesNoMinimumVotingPower) -> String {
        switch minimum {
        case .absolute(let votingPower):
            return "\(votingPower) voting power"
        case .fractionOfTotal(let numerator, let denominator, let rounding):
            return "\(numerator)/\(denominator) of the total, rounded \(describe(rounding))"
        }
    }

    private static func describe(_ rounding: VotingPowerRounding) -> String {
        switch rounding {
        case .down: return "down"
        case .up: return "up"
        case .downPlusOne: return "down plus one"
        case .upMinusOne: return "up minus one"
        }
    }

    // MARK: - Refresh

    /// Read the proposal, its poll state and its poll id. Each read stands on
    /// its own, so one failing still shows what the others returned.
    private func refresh() async {
        guard !isRefreshing else { return }
        guard let sdk = appState.sdk else {
            errorMessage = "SDK not initialized"
            return
        }
        isRefreshing = true
        defer { isRefreshing = false }

        let id = proposal.id
        async let fetched = capture { try await sdk.decisionProposal(id: id) }
        async let state = capture { try await sdk.decisionProposalVotePollState(proposalId: id) }
        async let pollIdValue = capture { try await sdk.decisionProposalVotePollId(proposalId: id) }

        var failures: [String] = []
        switch await fetched {
        case .success(let value?):
            proposal = value
        case .success(nil):
            failures.append("Platform no longer has this proposal.")
        case .failure(let error):
            failures.append("Proposal: \(error.localizedDescription)")
        }
        switch await state {
        case .success(let value):
            pollState = value
            pollStateLoaded = true
        case .failure(let error):
            failures.append("Poll state: \(error.localizedDescription)")
        }
        switch await pollIdValue {
        case .success(let value):
            pollId = value
        case .failure(let error):
            failures.append("Poll id: \(error.localizedDescription)")
        }
        errorMessage = failures.isEmpty ? nil : failures.joined(separator: "\n")
    }
}

/// `operation`'s outcome as a `Result`, so reads started together can fail
/// on their own.
private func capture<T: Sendable>(
    _ operation: @Sendable () async throws -> T
) async -> Result<T, Error> {
    do {
        return .success(try await operation())
    } catch {
        return .failure(error)
    }
}

// MARK: - Masternode vote

/// Picks the answer and takes the masternode credentials, the same way the
/// DPNS contest vote sheet does: the pro_tx_hash and the voting private key
/// as hex. Also reads the masternode's current answer.
private struct DecisionProposalVoteSheet: View {
    let proposalId: String
    let proposalTitle: String
    /// Called with a summary once Platform accepted the vote.
    let onVoted: (String) -> Void

    @EnvironmentObject var appState: AppState
    @Environment(\.dismiss) private var dismiss

    @State private var choice: YesNoAbstainVoteChoice = .yes
    @State private var proTxHashHex = ""
    @State private var votingKeyHex = ""
    @State private var isCasting = false
    @State private var isLookingUp = false
    @State private var message: String?
    @State private var messageIsError = false

    var body: some View {
        NavigationStack {
            Form {
                Section("Voting on") {
                    Text(proposalTitle)
                        .font(.headline)
                }

                Section("Your Answer") {
                    Picker("Answer", selection: $choice) {
                        ForEach(YesNoAbstainVoteChoice.allCases) { choice in
                            Text(choice.rawValue.capitalized).tag(choice)
                        }
                    }
                    .pickerStyle(.segmented)
                    .accessibilityIdentifier("decisionProposalVote.choice")
                }

                Section {
                    TextField("pro_tx_hash (64 hex chars)", text: $proTxHashHex)
                        .font(.system(.caption, design: .monospaced))
                        .autocorrectionDisabled()
                        .textInputAutocapitalization(.never)
                        .accessibilityIdentifier("decisionProposalVote.proTxHash")
                    SecureField("voting private key (64 hex chars)", text: $votingKeyHex)
                        .font(.system(.caption, design: .monospaced))
                        .autocorrectionDisabled()
                        .textInputAutocapitalization(.never)
                        .accessibilityIdentifier("decisionProposalVote.votingKey")
                } header: {
                    Text("Masternode Credentials")
                } footer: {
                    Text("The pro_tx_hash goes in wire order, the reverse of the hex Core displays. The voting public key and signer are derived from the private key on the Rust side; the key bytes are not stored.")
                }

                Section {
                    Button {
                        Task { await castVote() }
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
                    .disabled(isCasting || isLookingUp || proTxHashHex.isEmpty || votingKeyHex.isEmpty)
                    .accessibilityIdentifier("decisionProposalVote.submit")

                    Button {
                        Task { await lookUpAnswer() }
                    } label: {
                        if isLookingUp {
                            HStack {
                                ProgressView()
                                Text("Reading…")
                            }
                        } else {
                            Text("Read This Masternode's Answer")
                        }
                    }
                    .disabled(isCasting || isLookingUp || proTxHashHex.isEmpty)
                    .accessibilityIdentifier("decisionProposalVote.lookUp")

                    if let message {
                        Text(message)
                            .font(.caption)
                            .foregroundColor(messageIsError ? .orange : .secondary)
                    }
                }
            }
            .navigationTitle("Vote")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                        .disabled(isCasting)
                }
            }
            .interactiveDismissDisabled(isCasting)
        }
    }

    /// Decode a 32-byte value typed as 64 hex characters. `Data(hexString:)`
    /// drops a trailing odd nibble, so the length is checked first, as on the
    /// contest vote sheet.
    private static func decode32(_ hex: String) -> Data? {
        let trimmed = hex.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.count == 64, let data = Data(hexString: trimmed), data.count == 32 else {
            return nil
        }
        return data
    }

    private func castVote() async {
        guard let sdk = appState.sdk else {
            show("SDK not initialized", isError: true)
            return
        }
        guard let proTxHash = Self.decode32(proTxHashHex) else {
            show("pro_tx_hash must be 32 bytes (64 hex characters).", isError: true)
            return
        }
        guard let votingKey = Self.decode32(votingKeyHex) else {
            show("Voting private key must be 32 bytes (64 hex characters).", isError: true)
            return
        }

        isCasting = true
        defer { isCasting = false }
        do {
            try await sdk.castDecisionProposalVote(
                proposalId: proposalId,
                choice: choice,
                proTxHash: proTxHash,
                votingPrivateKey: votingKey
            )
            onVoted("Vote \"\(choice.rawValue)\" accepted by Platform.")
            dismiss()
        } catch {
            show("Vote rejected: \(error.localizedDescription)", isError: true)
        }
    }

    private func lookUpAnswer() async {
        guard let sdk = appState.sdk else {
            show("SDK not initialized", isError: true)
            return
        }
        guard let proTxHash = Self.decode32(proTxHashHex) else {
            show("pro_tx_hash must be 32 bytes (64 hex characters).", isError: true)
            return
        }

        isLookingUp = true
        defer { isLookingUp = false }
        do {
            if let vote = try await sdk.decisionProposalMasternodeVote(
                proposalId: proposalId,
                proTxHash: proTxHash
            ) {
                show(
                    "Current answer: \(vote.choice.rawValue), voted \(vote.timesVoted) time\(vote.timesVoted == 1 ? "" : "s").",
                    isError: false
                )
            } else {
                show("This masternode has no answer on the poll (none yet, or the poll has finished).", isError: false)
            }
        } catch {
            show("Reading the answer failed: \(error.localizedDescription)", isError: true)
        }
    }

    private func show(_ text: String, isError: Bool) {
        message = text
        messageIsError = isError
    }
}
