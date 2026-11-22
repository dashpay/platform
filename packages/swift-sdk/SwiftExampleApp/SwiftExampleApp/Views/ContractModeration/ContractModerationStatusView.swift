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

    var body: some View {
        Form {
            Section {
                TextField("Identity ID (base58)", text: $identityId)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .font(.system(.body, design: .monospaced))
                    .accessibilityIdentifier("contractModeration.status.identityId")
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
                    Task { await lookUp() }
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
        .navigationTitle("Identity Status")
        .navigationBarTitleDisplayMode(.inline)
    }

    @ViewBuilder
    private var result: some View {
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
            }
        )
    }

    private func lookUp() async {
        guard let sdk = appState.sdk else {
            load = .failed("SDK not initialized")
            return
        }
        load = .loading
        // In the order the lists are offered; none means every list.
        let lists = ContractModerationList.allCases.filter { chosenLists.contains($0) }
        do {
            let status = try await sdk.contractModerationStatus(
                contractId: contractId,
                identityId: trimmedIdentityId,
                lists: lists.isEmpty ? nil : lists
            )
            load = .loaded(status)
        } catch {
            load = .failed(moderationErrorText(error))
        }
    }
}

/// The status read: each list read, and what it says. A list not read is
/// named as such rather than shown as clear.
private struct ContractModerationStatusResult: View {
    let status: ContractModerationStatus

    var body: some View {
        Section {
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
        } header: {
            Text("Status")
        }
    }
}
