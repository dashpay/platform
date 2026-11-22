import SwiftUI
import SwiftDashSDK

/// One identity's status on a contract's moderation lists. With no list
/// chosen, rs-sdk reads every list the contract keeps; a list chosen must be
/// one the contract keeps, which the node judges.
struct ContractModerationStatusView: View {
    @EnvironmentObject var appState: AppState
    let contractId: String

    @State private var identityId = ""
    @State private var chosenLists: Set<ContractModerationList> = []
    @State private var load: ModerationLoad<ContractModerationStatus?> = .idle
    /// The identity the shown result (or the read running) is for.
    @State private var submittedIdentityId: String?
    /// Bumped on every submission and edit, so the completion of a superseded
    /// one is dropped.
    @State private var submission = 0

    var body: some View {
        Form {
            Section {
                TextField("Identity ID (base58)", text: $identityId)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .font(.system(.body, design: .monospaced))
                    .accessibilityIdentifier("contractModeration.status.identityId")
                    .onChange(of: identityId) { _, _ in clearResult() }
            } header: {
                Text("Identity")
            }

            Section {
                ForEach(ContractModerationList.allCases) { list in
                    Toggle(list.title, isOn: binding(for: list))
                        .accessibilityIdentifier("contractModeration.status.list.\(list.rawValue)")
                }
            } header: {
                Text("Lists")
            } footer: {
                Text("None chosen reads every list the contract keeps.")
            }

            Section {
                Button {
                    // The inputs are taken now, at the tap: an edit made before
                    // the task runs cannot change what this look up sends
                    if let lookUp = submitLookUp() {
                        Task { await run(lookUp) }
                    }
                } label: {
                    HStack {
                        if case .loading = load { ProgressView().controlSize(.small) }
                        Text("Look Up")
                    }
                    .frame(maxWidth: .infinity)
                }
                .disabled(trimmedIdentityId.isEmpty || isLoading)
                .accessibilityIdentifier("contractModeration.status.lookup")
            }

            result
        }
        // A replaced SDK: the status shown, and one still being read, answered
        // for the old connection
        .onChange(of: appState.sdkGeneration) { _, _ in clearResult() }
        .navigationTitle("Identity Status")
        .navigationBarTitleDisplayMode(.inline)
    }

    @ViewBuilder
    private var result: some View {
        switch load {
        case .idle:
            EmptyView()
        case .loading, .failed, .loaded:
            Section {
                if let submittedIdentityId {
                    ModerationFieldRow(label: "Identity", value: submittedIdentityId, monospaced: true)
                        .accessibilityIdentifier("contractModeration.status.submittedIdentityId")
                }
                resultBody
            } header: {
                Text("Status")
            }
        }
    }

    @ViewBuilder
    private var resultBody: some View {
        switch load {
        case .idle:
            EmptyView()
        case .loading:
            ModerationMessageRow(
                kind: .loading,
                text: "Reading the lists…",
                identifier: "contractModeration.status.loading"
            )
        case .failed(let message):
            ModerationMessageRow(
                kind: .error,
                text: message,
                identifier: "contractModeration.status.error"
            )
        case .loaded(nil):
            ModerationMessageRow(
                kind: .empty,
                text: "No answer for this identity.",
                identifier: "contractModeration.status.empty"
            )
        case .loaded(let status?):
            ContractModerationStatusResult(status: status)
        }
    }

    private var trimmedIdentityId: String {
        identityId.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private var isLoading: Bool {
        if case .loading = load { return true }
        return false
    }

    private func binding(for list: ContractModerationList) -> Binding<Bool> {
        Binding(
            get: { chosenLists.contains(list) },
            set: { isOn in
                if isOn { chosenLists.insert(list) } else { chosenLists.remove(list) }
                clearResult()
            }
        )
    }

    /// Drop the result shown, and whatever a running read brings back: it no
    /// longer answers what the form asks.
    private func clearResult() {
        submission += 1
        submittedIdentityId = nil
        load = .idle
    }

    /// One look up as submitted: its token and the inputs read at the tap.
    private struct LookUp {
        let token: Int
        let identityId: String
        let lists: [ContractModerationList]?
        /// The SDK current at the tap; the answer is shown only while it still is.
        let connection: ModerationConnection
    }

    /// Takes the identity and lists as they are now, makes them the current
    /// submission and shows it as loading; `nil` without an SDK.
    private func submitLookUp() -> LookUp? {
        guard let connection = appState.moderationConnection else {
            load = .failed("SDK not initialized")
            return nil
        }
        submission += 1
        let identityId = trimmedIdentityId
        submittedIdentityId = identityId
        load = .loading
        // In the order the lists are offered; none chosen reads every list.
        let lists = ContractModerationList.allCases.filter { chosenLists.contains($0) }
        return LookUp(
            token: submission,
            identityId: identityId,
            lists: lists.isEmpty ? nil : lists,
            connection: connection
        )
    }

    /// Sends `lookUp` from its captured inputs alone, unless an edit or a
    /// newer submission superseded it before it ran, and shows its answer
    /// only while it is still current.
    private func run(_ lookUp: LookUp) async {
        guard lookUp.token == submission, appState.isCurrent(lookUp.connection) else { return }
        do {
            let status = try await lookUp.connection.sdk.contractModerationStatus(
                contractId: contractId,
                identityId: lookUp.identityId,
                lists: lookUp.lists
            )
            guard lookUp.token == submission, appState.isCurrent(lookUp.connection) else { return }
            load = .loaded(status)
        } catch {
            guard lookUp.token == submission, appState.isCurrent(lookUp.connection) else { return }
            load = .failed(moderationErrorText(error))
        }
    }
}

/// The status read: each list read, and what it says. A list not read is
/// named as such rather than shown as clear.
private struct ContractModerationStatusResult: View {
    let status: ContractModerationStatus

    var body: some View {
        Group {
            ModerationFieldRow(label: "Lists read", value: status.lists.map(\.title).joined(separator: ", "))
                .accessibilityIdentifier("contractModeration.status.result")

            if let banned = status.banned {
                VStack(alignment: .leading, spacing: 2) {
                    ModerationFieldRow(label: "Banned", value: banned ? "Yes" : "No")
                    if let reason = status.banReason {
                        ModerationReasonView(reason: reason)
                    }
                }
                .accessibilityElement(children: .combine)
                .accessibilityIdentifier("contractModeration.status.banned")
            }

            if status.lists.contains(.suspensions) {
                VStack(alignment: .leading, spacing: 2) {
                    ModerationFieldRow(
                        label: "Suspended until",
                        value: status.suspendedUntilMs.map { moderationDate(ms: $0) } ?? "Not suspended"
                    )
                    if let reason = status.suspensionReason {
                        ModerationReasonView(reason: reason)
                    }
                }
                .accessibilityElement(children: .combine)
                .accessibilityIdentifier("contractModeration.status.suspendedUntil")
            }

            if let warnings = status.warnings {
                ModerationFieldRow(label: "Warnings", value: "\(warnings.count)")
                    .accessibilityIdentifier("contractModeration.status.warnings")
                ModerationWarningsView(warnings: warnings, identifierPrefix: "contractModeration.status")
            }
        }
    }
}
