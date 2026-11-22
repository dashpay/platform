import SwiftUI
import SwiftDashSDK

/// Checks a state transition's status from its hash (protocol version 15) with
/// `SDK.getStateTransitionStatusByHash` (`dash_sdk_system_get_state_transition_status_by_hash`).
/// Covers TEST_PLAN SYS-12.
///
/// Rust asks the node and, when the node says committed, reads that block with its proof and
/// looks for the transition. Only "in a block" is proved; every other outcome is the node's
/// word and is shown as unproved, with the node's height.
struct TransitionStatusCheckView: View {
    @EnvironmentObject var appState: AppState

    @State private var hashText = ""
    @State private var isRunning = false
    @State private var status: TransitionStatusByHash?
    @State private var errorMessage: String?

    private var trimmedHash: String {
        hashText.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    var body: some View {
        Form {
            Section {
                TextField("64 hex characters", text: $hashText, axis: .vertical)
                    .textInputAutocapitalization(.never)
                    .disableAutocorrection(true)
                    .font(.system(.footnote, design: .monospaced))
                    .accessibilityLabel("State transition hash")
                    .accessibilityIdentifier("txStatus.hashField")
                    .disabled(isRunning)
                Button(action: check) {
                    HStack {
                        if isRunning {
                            ProgressView()
                        } else {
                            Image(systemName: "magnifyingglass")
                        }
                        Text(isRunning ? "Checking…" : "Check Status")
                            .fontWeight(.semibold)
                    }
                    .frame(maxWidth: .infinity)
                }
                .disabled(isRunning || appState.sdk == nil || trimmedHash.isEmpty)
                .accessibilityIdentifier("txStatus.checkButton")
            } header: {
                Text("State transition hash")
            } footer: {
                Text("Proved only when the block the node names holds the transition. For shielded and private document transitions, ask only the node the transition was broadcast to.")
            }

            if let errorMessage {
                Section("Error") {
                    Text(errorMessage)
                        .foregroundColor(.red)
                        .font(.callout)
                        .textSelection(.enabled)
                        .accessibilityIdentifier("txStatus.errorText")
                }
            } else if let status {
                outcomeSection(status)
            }
        }
        .navigationTitle("Transition Status")
        .navigationBarTitleDisplayMode(.inline)
    }

    @ViewBuilder
    private func outcomeSection(_ status: TransitionStatusByHash) -> some View {
        Section("Outcome") {
            ProvedBadge(
                proved: status.proved,
                provedText: "Proved: a block the validator quorum signed holds it",
                unprovedText: "Unproved: one node's word",
                identifier: "txStatus.provedBadge"
            )
            Text(headline(status))
                .font(.headline)
                .accessibilityIdentifier("txStatus.outcomeText")

            if status.status == .inBlock {
                if let height = status.blockHeight {
                    BlockField(label: "Block height", value: "\(height)")
                }
                if let hash = status.blockHash {
                    BlockField(label: "Block hash", value: hash, monospaced: true)
                }
                if let timeMs = status.blockTimeMs {
                    BlockField(label: "Block time", value: BlockFormatting.time(ms: timeMs))
                }
                if let transaction = status.transaction {
                    BlockField(
                        label: "Result",
                        value: transaction.executed
                            ? "Executed (code 0)"
                            : "Failed, paid for error code \(transaction.code)"
                    )
                    .accessibilityIdentifier("txStatus.resultText")
                    BlockTransactionRow(transaction: transaction)
                }
            } else {
                if let height = status.height {
                    BlockField(label: "Height the node named", value: "\(height)")
                }
                if let error = status.error {
                    BlockField(label: "Error the node reports (unproved)", value: "Code \(error.code): \(error.message)")
                    if let consensusError = error.consensusError {
                        BlockField(label: "Consensus error", value: consensusError)
                    }
                }
                BlockField(
                    label: "Node height when it answered",
                    value: status.nodeHeight.map { "\($0)" } ?? "Not reported"
                )
                .accessibilityIdentifier("txStatus.nodeHeightText")
            }
        }
    }

    private func headline(_ status: TransitionStatusByHash) -> String {
        switch status.status {
        case .inBlock:
            return "In block \(status.blockHeight.map { "\($0)" } ?? "")"
        case .committedNotProved:
            return "The node says it is in block \(status.height.map { "\($0)" } ?? ""), but no proved block came back"
        case .committedRefuted:
            return "Refuted: the proved block \(status.height.map { "\($0)" } ?? "") does not hold it"
        case .pending:
            return "Pending in the node's mempool"
        case .unknown:
            return "Unknown to the node"
        }
    }

    private func check() {
        guard let sdk = appState.sdk else {
            errorMessage = "SDK not initialized"
            return
        }
        let hash = trimmedHash
        isRunning = true
        errorMessage = nil
        status = nil
        Task {
            do {
                let answer = try await sdk.getStateTransitionStatusByHash(transitionHash: hash)
                await MainActor.run {
                    status = answer
                    isRunning = false
                }
            } catch {
                await MainActor.run {
                    errorMessage = error.localizedDescription
                    isRunning = false
                }
            }
        }
    }
}
