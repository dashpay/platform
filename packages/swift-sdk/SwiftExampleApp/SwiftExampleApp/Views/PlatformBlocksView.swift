import SwiftUI
import SwiftDashSDK

/// Read-only block explorer (protocol version 15): reads up to ten consecutive blocks with
/// `SDK.getBlocks` (`dash_sdk_system_get_blocks`), opens one to list its state transitions,
/// and from a block finds one identity's transitions in it with
/// `SDK.getOwnerTransactionsInBlock`. Covers TEST_PLAN SYS-09 and SYS-10.
///
/// Rust verifies the block proofs and says whether the answer is proved; this view shows that
/// flag and nothing it decides itself. Below protocol version 15 the SDK's refusal is shown as
/// it comes.
struct PlatformBlocksView: View {
    @EnvironmentObject var appState: AppState

    @State private var startHeightText = "1"
    @State private var countText = "10"
    @State private var isRunning = false
    @State private var answer: PlatformBlocks?
    @State private var errorMessage: String?

    var body: some View {
        Form {
            Section {
                TextField("Start height", text: $startHeightText)
                    .keyboardType(.numberPad)
                    .accessibilityIdentifier("blocks.startHeightField")
                    .disabled(isRunning)
                TextField("Count (1 to 10)", text: $countText)
                    .keyboardType(.numberPad)
                    .accessibilityIdentifier("blocks.countField")
                    .disabled(isRunning)
                Button(action: run) {
                    HStack {
                        if isRunning {
                            ProgressView()
                        } else {
                            Image(systemName: "square.stack.3d.up")
                        }
                        Text(isRunning ? "Reading…" : "Read Blocks")
                            .fontWeight(.semibold)
                    }
                    .frame(maxWidth: .infinity)
                }
                .disabled(isRunning || appState.sdk == nil)
                .accessibilityIdentifier("blocks.runButton")
            } header: {
                Text("Blocks")
            } footer: {
                Text("Reads consecutive blocks from the start height. Use a count of 1 for a single block. The node serves 1 to 10 blocks and may return fewer than asked; nothing proves a block left off the end does not exist.")
            }

            if let errorMessage {
                Section("Error") {
                    Text(errorMessage)
                        .foregroundColor(.red)
                        .font(.callout)
                        .textSelection(.enabled)
                        .accessibilityIdentifier("blocks.errorText")
                }
            } else if let answer {
                Section {
                    ProvedBadge(proved: answer.proved, identifier: "blocks.provedBadge")
                    if answer.blocks.isEmpty {
                        Text("No blocks returned")
                            .foregroundColor(.secondary)
                            .accessibilityIdentifier("blocks.emptyText")
                    }
                    ForEach(answer.blocks, id: \.height) { block in
                        NavigationLink(destination: PlatformBlockDetailView(block: block, proved: answer.proved)) {
                            VStack(alignment: .leading, spacing: 4) {
                                Text("Block \(block.height)")
                                    .font(.headline)
                                Text(BlockFormatting.time(ms: block.timeMs))
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                                Text("\(block.transactions.count) state transition\(block.transactions.count == 1 ? "" : "s")")
                                    .font(.caption)
                            }
                        }
                        .accessibilityIdentifier("blocks.row.\(block.height)")
                    }
                } header: {
                    Text("Result (\(answer.blocks.count))")
                }
            }
        }
        .navigationTitle("Block Explorer")
        .navigationBarTitleDisplayMode(.inline)
    }

    private func run() {
        guard let sdk = appState.sdk else {
            errorMessage = "SDK not initialized"
            return
        }
        guard let startHeight = UInt64(startHeightText.trimmingCharacters(in: .whitespaces)),
              let count = UInt32(countText.trimmingCharacters(in: .whitespaces)) else {
            errorMessage = "Start height and count must be whole numbers"
            answer = nil
            return
        }
        isRunning = true
        errorMessage = nil
        answer = nil
        Task {
            do {
                let blocks = try await sdk.getBlocks(startHeight: startHeight, count: count)
                await MainActor.run {
                    answer = blocks
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

/// One block's fields and state transitions, and a lookup of one identity's transitions in it.
struct PlatformBlockDetailView: View {
    let block: PlatformBlock
    /// Whether Rust proved the block this view shows.
    let proved: Bool

    @EnvironmentObject var appState: AppState

    @State private var ownerIdText = ""
    @State private var isRunning = false
    @State private var ownerAnswer: OwnerTransactionsInBlock?
    @State private var ownerError: String?

    var body: some View {
        Form {
            Section("Block") {
                ProvedBadge(proved: proved, identifier: "blockDetail.provedBadge")
                BlockField(label: "Height", value: "\(block.height)")
                BlockField(label: "Time", value: BlockFormatting.time(ms: block.timeMs))
                BlockField(label: "Hash", value: block.hash, monospaced: true)
                BlockField(label: "Core chain-locked height", value: "\(block.coreChainLockedHeight)")
                BlockField(label: "App hash", value: block.appHash, monospaced: true)
                BlockField(label: "Proposer", value: block.proposerProTxHash, monospaced: true)
            }

            Section("State transitions (\(block.transactions.count))") {
                if block.transactions.isEmpty {
                    Text("This block holds no state transitions")
                        .foregroundColor(.secondary)
                        .accessibilityIdentifier("blockDetail.emptyText")
                }
                ForEach(block.transactions, id: \.hash) { transaction in
                    BlockTransactionRow(transaction: transaction)
                        .accessibilityIdentifier("blockDetail.txRow.\(transaction.hash)")
                }
            }

            Section {
                TextField("Identity ID (base58)", text: $ownerIdText)
                    .textInputAutocapitalization(.never)
                    .disableAutocorrection(true)
                    .font(.system(.footnote, design: .monospaced))
                    .accessibilityIdentifier("blockDetail.ownerIdField")
                    .disabled(isRunning)
                Button(action: findOwnerTransactions) {
                    HStack {
                        if isRunning {
                            ProgressView()
                        } else {
                            Image(systemName: "person.crop.circle.badge.magnifyingglass")
                        }
                        Text(isRunning ? "Reading…" : "Find Identity's Transitions")
                            .fontWeight(.semibold)
                    }
                    .frame(maxWidth: .infinity)
                }
                .disabled(isRunning || appState.sdk == nil
                          || ownerIdText.trimmingCharacters(in: .whitespaces).isEmpty)
                .accessibilityIdentifier("blockDetail.findOwnerButton")

                if let ownerError {
                    Text(ownerError)
                        .foregroundColor(.red)
                        .font(.callout)
                        .textSelection(.enabled)
                        .accessibilityIdentifier("blockDetail.ownerErrorText")
                } else if let ownerAnswer {
                    ProvedBadge(
                        proved: ownerAnswer.proved,
                        provedText: "Each transition proved; the list is not proved complete",
                        identifier: "blockDetail.ownerProvedBadge"
                    )
                    if ownerAnswer.transactions.isEmpty {
                        Text("The node returned no transitions of this identity in block \(ownerAnswer.height)")
                            .foregroundColor(.secondary)
                            .accessibilityIdentifier("blockDetail.ownerEmptyText")
                    }
                    ForEach(ownerAnswer.transactions, id: \.hash) { transaction in
                        BlockTransactionRow(transaction: transaction)
                            .accessibilityIdentifier("blockDetail.ownerTxRow.\(transaction.hash)")
                    }
                }
            } header: {
                Text("An identity's transitions in this block")
            } footer: {
                Text("Asks the node for the transitions this identity owns in this block. Each one returned is proved to be in the block with its result, but a node can leave some out: the block's full list above is the complete one. A recipient is not an owner.")
            }
        }
        .navigationTitle("Block \(block.height)")
        .navigationBarTitleDisplayMode(.inline)
    }

    private func findOwnerTransactions() {
        guard let sdk = appState.sdk else {
            ownerError = "SDK not initialized"
            return
        }
        let ownerId = ownerIdText.trimmingCharacters(in: .whitespacesAndNewlines)
        isRunning = true
        ownerError = nil
        ownerAnswer = nil
        Task {
            do {
                let answer = try await sdk.getOwnerTransactionsInBlock(height: block.height, ownerId: ownerId)
                await MainActor.run {
                    ownerAnswer = answer
                    isRunning = false
                }
            } catch {
                await MainActor.run {
                    ownerError = error.localizedDescription
                    isRunning = false
                }
            }
        }
    }
}

/// A state transition of a block with its result.
struct BlockTransactionRow: View {
    let transaction: PlatformBlockTransaction

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(transaction.type)
                    .font(.subheadline)
                    .fontWeight(.semibold)
                Spacer()
                if transaction.executed {
                    Label("Executed", systemImage: "checkmark.circle.fill")
                        .font(.caption)
                        .foregroundColor(.green)
                } else {
                    Label("Failed, code \(transaction.code)", systemImage: "xmark.octagon.fill")
                        .font(.caption)
                        .foregroundColor(.orange)
                }
            }
            Text(transaction.hash)
                .font(.system(.caption2, design: .monospaced))
                .lineLimit(1)
                .truncationMode(.middle)
                .textSelection(.enabled)
            if let ownerId = transaction.ownerId {
                Text("Owner \(ownerId)")
                    .font(.system(.caption2, design: .monospaced))
                    .foregroundColor(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .textSelection(.enabled)
            }
            Text("Gas used \(transaction.gasUsed)")
                .font(.caption2)
                .foregroundColor(.secondary)
        }
        .padding(.vertical, 2)
    }
}

/// Whether Rust proved what the section shows.
struct ProvedBadge: View {
    let proved: Bool
    var provedText = "Proved by the block's quorum signature"
    var unprovedText = "Unproved: the node's word"
    let identifier: String

    var body: some View {
        Label(proved ? provedText : unprovedText,
              systemImage: proved ? "checkmark.seal.fill" : "exclamationmark.triangle.fill")
            .font(.footnote)
            .foregroundColor(proved ? .green : .orange)
            .accessibilityIdentifier(identifier)
    }
}

/// A labelled value of a block.
struct BlockField: View {
    let label: String
    let value: String
    var monospaced = false

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(label)
                .font(.caption)
                .foregroundColor(.secondary)
            Text(value)
                .font(monospaced ? .system(.footnote, design: .monospaced) : .body)
                .textSelection(.enabled)
        }
    }
}

enum BlockFormatting {
    /// A block time in milliseconds since the Unix epoch, as a local date and time.
    static func time(ms: UInt64) -> String {
        let date = Date(timeIntervalSince1970: TimeInterval(ms) / 1000)
        return date.formatted(date: .abbreviated, time: .standard)
    }
}
