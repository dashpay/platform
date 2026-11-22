import SwiftUI
import SwiftData
import SwiftDashSDK

/// One multiple choice proposal: its question, its choices with their
/// tallies while the vote runs, how the vote ended and the kept choice once
/// it did, a masternode vote, entering a choice, and the edit for a proposal
/// a local identity filed.
///
/// Everything shown is read from Platform through the `SDK` multiple choice
/// proposal wrappers on appear and on pull-to-refresh. Whether a vote, a
/// choice or an edit is accepted is Platform's call; this screen only
/// collects input and shows the outcome.
struct MultipleChoiceProposalDetailView: View {
    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Query private var identities: [PersistentIdentity]

    @State private var proposal: MultipleChoiceProposal
    @State private var vote: MultipleChoiceProposalVote?
    /// `true` once the vote has been read, so "no choices" is not shown while
    /// it is still loading.
    @State private var voteLoaded = false
    /// `true` when the last read of the vote failed. An earlier read stays on
    /// screen; without one, the vote shows as unavailable.
    @State private var voteLoadFailed = false
    @State private var keptChoice: ProposalChoice?
    @State private var isRefreshing = false
    @State private var errorMessage: String?

    @State private var showVoteSheet = false
    @State private var showEnterChoiceSheet = false
    @State private var showEditSheet = false
    /// Banner after a vote, a choice or an edit.
    @State private var actionMessage: String?

    init(proposal: MultipleChoiceProposal) {
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

    /// Identities on this device held by a loaded wallet, which can sign a
    /// choice.
    private var localSigners: [PersistentIdentity] {
        identities.filter { $0.network == appState.currentNetwork && $0.wallet != nil }
    }

    /// The choices a vote can name: the running ones the vote read.
    private var votableChoices: [ProposalChoice] {
        vote?.choices.compactMap(\.choice) ?? []
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
            choicesSection
            resultSection
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
            MultipleChoiceProposalVoteSheet(
                proposalId: proposal.id,
                proposalTitle: proposal.title,
                choices: votableChoices
            ) { message in
                actionMessage = message
                Task { await refresh() }
            }
            .environmentObject(appState)
        }
        .sheet(isPresented: $showEnterChoiceSheet, onDismiss: { Task { await refresh() } }) {
            EnterProposalChoiceView(proposal: proposal) { message in
                actionMessage = message
            }
            .environmentObject(appState)
            .environmentObject(walletManager)
        }
        // Read the proposal again however the sheet closes: a refused edit
        // may have found it changed, and the form then shows the newer text.
        .sheet(isPresented: $showEditSheet, onDismiss: { Task { await refresh() } }) {
            if let owner = localOwner {
                EditMultipleChoiceProposalView(proposal: proposal, owner: owner) { message in
                    actionMessage = message
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
            if let urlString = proposal.url {
                if let url = URL(string: urlString) {
                    Link(urlString, destination: url)
                        .font(.caption)
                } else {
                    Text(urlString)
                        .font(.caption)
                }
            }
            if let document = proposal.descriptionDocument {
                DetailRow(label: "Description document", value: document.id)
                DetailRow(label: "Its contract", value: document.contractId)
                DetailRow(label: "Its document type", value: document.documentType)
            }
            DetailRow(
                label: "Who may enter choices",
                value: proposal.open ? "Anyone" : "Only the proposer"
            )
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

    @ViewBuilder
    private var choicesSection: some View {
        Section {
            if !voteLoaded && !voteLoadFailed {
                HStack {
                    ProgressView()
                    Text("Loading choices…")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            } else if let vote {
                if vote.choices.isEmpty {
                    Text("No choice has been entered yet. The first choice opens the vote.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                ForEach(vote.choices) { entry in
                    choiceRow(entry)
                }
                if let tally = vote.noneOfTheChoicesVoteTally {
                    tallyRow("None of the choices", systemImage: "nosign", value: tally, color: .red)
                }
                if let tally = vote.abstainVoteTally {
                    tallyRow("Abstain", systemImage: "minus.circle", value: tally, color: .gray)
                }
            } else {
                Text("The vote could not be read.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        } header: {
            Text("Choices")
        } footer: {
            Text("Each choice is named by its document id. A vote counts a masternode 1 and an evonode 4.")
        }
    }

    @ViewBuilder
    private var resultSection: some View {
        if let result = vote?.result {
            Section("Result") {
                HStack {
                    Label("Outcome", systemImage: "flag.fill")
                    Spacer()
                    Text(Self.describe(result.outcome))
                        .foregroundColor(result.outcome == .keptChoice ? .green : .red)
                }
                if let keptChoiceId = result.keptChoiceId {
                    DetailRow(label: "Kept choice ID", value: keptChoiceId)
                }
                if let keptChoice {
                    VStack(alignment: .leading, spacing: 4) {
                        Text(keptChoice.title)
                            .font(.headline)
                        if let description = keptChoice.description {
                            Text(description)
                                .font(.body)
                                .fixedSize(horizontal: false, vertical: true)
                        }
                    }
                    DetailRow(label: "Entered by", value: keptChoice.ownerId)
                }
                DetailRow(
                    label: "Closed",
                    value: "Block \(result.finalizationBlock.height), epoch \(result.finalizationBlock.epoch), \(result.finalizationBlock.time.formatted(date: .abbreviated, time: .shortened))"
                )
            }
        }
    }

    private var actionsSection: some View {
        Section("Actions") {
            if let message = actionMessage {
                Label(message, systemImage: "checkmark.circle.fill")
                    .font(.caption)
                    .foregroundColor(.green)
            }

            Button {
                actionMessage = nil
                showVoteSheet = true
            } label: {
                Label("Vote as Masternode…", systemImage: "hand.raised.fill")
            }
            // An ended vote keeps no votes; Platform refuses one.
            .disabled(appState.sdk == nil || vote?.result != nil)
            .accessibilityIdentifier("multipleChoiceProposal.vote")

            Button {
                actionMessage = nil
                showEnterChoiceSheet = true
            } label: {
                Label("Enter a Choice…", systemImage: "plus.bubble")
            }
            .disabled(appState.sdk == nil || localSigners.isEmpty || vote?.result != nil)
            .accessibilityIdentifier("multipleChoiceProposal.enterChoice")

            if localOwner != nil {
                Button {
                    actionMessage = nil
                    showEditSheet = true
                } label: {
                    Label("Edit…", systemImage: "pencil")
                }
                .disabled(appState.sdk == nil)
                .accessibilityIdentifier("multipleChoiceProposal.edit")

                Text("Platform accepts an edit of the title, url and description document for two hours after the proposal is created. Whether it is open never changes.")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }

            Text("Only masternodes vote, with the voting key of a pro_tx_hash. A choice is never edited; entering one pays the contest fund on top of the fees.")
                .font(.caption2)
                .foregroundColor(.secondary)
        }
    }

    // MARK: - Rows

    private func choiceRow(_ entry: MultipleChoiceProposalVote.ChoiceInVote) -> some View {
        HStack(alignment: .top) {
            VStack(alignment: .leading, spacing: 2) {
                Text(entry.choice?.title ?? "Choice no longer stored")
                    .font(.subheadline)
                    .foregroundColor(entry.choice == nil ? .secondary : .primary)
                if let description = entry.choice?.description {
                    Text(description)
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
                Text(entry.id)
                    .font(.system(.caption2, design: .monospaced))
                    .foregroundColor(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }
            Spacer()
            Text(entry.voteTally.map { "\($0)" } ?? "-")
                .font(.headline)
        }
        .accessibilityIdentifier("multipleChoiceProposal.choice.\(entry.id)")
    }

    private func tallyRow(_ title: String, systemImage: String, value: UInt32, color: Color) -> some View {
        HStack {
            Label(title, systemImage: systemImage)
                .foregroundColor(color)
            Spacer()
            Text("\(value)")
                .font(.headline)
        }
    }

    private static func describe(_ outcome: MultipleChoiceProposalVote.Outcome) -> String {
        switch outcome {
        case .keptChoice: return "A choice was kept"
        case .noneOfTheChoices: return "None of the choices"
        case .noWinner: return "No winner"
        }
    }

    // MARK: - Refresh

    /// Read the proposal, its vote and its kept choice. Each read stands on
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
        async let fetched = captureResult { try await sdk.multipleChoiceProposal(id: id) }
        async let voteValue = captureResult { try await sdk.multipleChoiceProposalVote(proposalId: id) }
        async let kept = captureResult { try await sdk.multipleChoiceProposalKeptChoice(proposalId: id) }

        var failures: [String] = []
        switch await fetched {
        case .success(let value?):
            proposal = value
        case .success(nil):
            failures.append("Platform no longer has this proposal.")
        case .failure(let error):
            failures.append("Proposal: \(error.localizedDescription)")
        }
        switch await voteValue {
        case .success(let value):
            vote = value
            voteLoaded = true
            voteLoadFailed = false
        case .failure(let error):
            voteLoadFailed = true
            failures.append("Vote: \(error.localizedDescription)")
        }
        switch await kept {
        case .success(let value):
            keptChoice = value
        case .failure(let error):
            failures.append("Kept choice: \(error.localizedDescription)")
        }
        errorMessage = failures.isEmpty ? nil : failures.joined(separator: "\n")
    }
}

/// `operation`'s outcome as a `Result`, so reads started together can fail
/// on their own.
private func captureResult<T: Sendable>(
    _ operation: @Sendable () async throws -> T
) async -> Result<T, Error> {
    do {
        return .success(try await operation())
    } catch {
        return .failure(error)
    }
}

// MARK: - Masternode vote

/// Picks the answer and the masternode that votes, the same way the yes/no
/// vote sheet does: a masternode or evonode identity loaded with its voting
/// key, picked in the shared `MasternodeVoterSection`. Also reads the
/// masternode's current answer.
private struct MultipleChoiceProposalVoteSheet: View {
    let proposalId: String
    let proposalTitle: String
    /// The running choices, as the vote read them.
    let choices: [ProposalChoice]
    /// Called with a summary once Platform accepted the vote.
    let onVoted: (String) -> Void

    @EnvironmentObject var appState: AppState
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    /// What the picker holds.
    private enum Answer: Hashable {
        case choice(String)
        case noneOfTheChoices
        case abstain
    }

    @State private var answer: Answer?
    @State private var voter: MasternodeVoter?
    @State private var isCasting = false
    @State private var isLookingUp = false
    @State private var message: String?
    @State private var messageIsError = false

    private var voteChoice: MultipleChoiceVoteChoice? {
        switch answer {
        case .choice(let id): return .choice(id: id)
        case .noneOfTheChoices: return .noneOfTheChoices
        case .abstain: return .abstain
        case nil: return nil
        }
    }

    var body: some View {
        NavigationStack {
            Form {
                Section("Voting on") {
                    Text(proposalTitle)
                        .font(.headline)
                }

                Section {
                    Picker("Answer", selection: $answer) {
                        Text("Select an answer").tag(Answer?.none)
                        ForEach(choices) { choice in
                            Text(choice.title).tag(Answer?.some(.choice(choice.id)))
                        }
                        Text("None of the choices").tag(Answer?.some(.noneOfTheChoices))
                        Text("Abstain").tag(Answer?.some(.abstain))
                    }
                    .pickerStyle(.inline)
                    .labelsHidden()
                    .accessibilityIdentifier("multipleChoiceProposalVote.choice")
                } header: {
                    Text("Your Answer")
                } footer: {
                    Text("When the vote ends the choice with the most votes is kept, unless the votes for none of the choices outnumber it.")
                }

                MasternodeVoterSection(
                    accessibilityPrefix: "multipleChoiceProposalVote",
                    voter: $voter,
                    allowsTypedProTxHash: true
                )

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
                    .disabled(isCasting || isLookingUp || voteChoice == nil || voter?.canCast != true)
                    .accessibilityIdentifier("multipleChoiceProposalVote.submit")

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
                    .disabled(isCasting || isLookingUp || voter == nil)
                    .accessibilityIdentifier("multipleChoiceProposalVote.lookUp")

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
            // A message names the previous voter's answer; drop it.
            .onChange(of: voter) { _, _ in message = nil }
        }
    }

    private func describe(_ choice: MultipleChoiceVoteChoice) -> String {
        switch choice {
        case .choice(let id):
            return choices.first { $0.id == id }.map { "\"\($0.title)\"" } ?? "the choice \(id)"
        case .noneOfTheChoices:
            return "none of the choices"
        case .abstain:
            return "abstain"
        }
    }

    private func castVote() async {
        guard let sdk = appState.sdk else {
            show("SDK not initialized", isError: true)
            return
        }
        guard let choice = voteChoice else {
            show("Select an answer.", isError: true)
            return
        }
        guard let voter else { return }
        let proposalId = proposalId
        let call = MasternodeVoteCall { proTxHash, votingAddress, signer in
            try await sdk.castMultipleChoiceProposalVote(
                proposalId: proposalId,
                choice: choice,
                proTxHash: proTxHash,
                votingAddress: votingAddress,
                signer: signer
            )
        }
        #if DEBUG
        .allowingTypedKey { proTxHash, votingPrivateKey in
            try await sdk.castMultipleChoiceProposalVote(
                proposalId: proposalId,
                choice: choice,
                proTxHash: proTxHash,
                votingPrivateKey: votingPrivateKey
            )
        }
        #endif

        isCasting = true
        defer { isCasting = false }
        do {
            try await call.run(
                as: voter,
                modelContainer: modelContext.container,
                network: appState.currentNetwork
            )
            onVoted("Vote for \(describe(choice)) accepted by Platform.")
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
        guard let voter else { return }
        let proTxHash = voter.wireOrderProTxHash

        isLookingUp = true
        defer { isLookingUp = false }
        do {
            let current = try await sdk.multipleChoiceProposalMasternodeVote(
                proposalId: proposalId,
                proTxHash: proTxHash
            )
            // The voter or the network changed while the read was in
            // flight: this answer is not the picked masternode's on the
            // network now shown.
            guard self.voter?.wireOrderProTxHash == proTxHash,
                  appState.sdk === sdk else { return }
            if let current {
                show("Current answer: \(describe(current)).", isError: false)
            } else {
                show("This masternode has no answer in the vote (none yet, or the vote has ended).", isError: false)
            }
        } catch {
            guard self.voter?.wireOrderProTxHash == proTxHash,
                  appState.sdk === sdk else { return }
            show("Reading the answer failed: \(error.localizedDescription)", isError: true)
        }
    }

    private func show(_ text: String, isError: Bool) {
        message = text
        messageIsError = isError
    }
}
