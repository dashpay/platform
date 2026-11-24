import SwiftDashSDK
import SwiftUI

/// Send a shielded tip to a username from `identity`'s own shielded account.
/// With `username` set (a contact) the name is fixed; without it the user
/// types one. The wallet parses the amount, reviews the recipient, refuses
/// what it must and sends; `ShieldedTipFlow` maps its answers to this sheet.
struct ShieldedTipSheet: View {
    let identity: PersistentIdentity
    let username: String?

    @EnvironmentObject private var walletManager: PlatformWalletManager
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        if let walletId = identity.wallet?.walletId {
            ShieldedTipSheetContent(
                identity: identity,
                flow: Self.makeFlow(
                    walletManager: walletManager,
                    walletId: walletId,
                    identityId: identity.identityId,
                    username: username
                )
            )
        } else {
            NavigationStack {
                Text("This identity has no wallet on this device, so it cannot send a tip.")
                    .foregroundColor(.secondary)
                    .padding()
                    .accessibilityIdentifier("dashpay.tip.noWallet")
                    .toolbar {
                        ToolbarItem(placement: .cancellationAction) {
                            Button("Done") { dismiss() }
                                .accessibilityIdentifier("dashpay.tip.done")
                        }
                    }
            }
        }
    }

    /// The flow's wallet calls: parse, review, and send from the identity
    /// the user acts as (`.identity`) to the recipient they reviewed.
    @MainActor
    static func makeFlow(
        walletManager: PlatformWalletManager,
        walletId: Data,
        identityId: Data,
        username: String?
    ) -> ShieldedTipFlow {
        ShieldedTipFlow(
            username: username,
            actions: ShieldedTipFlow.Actions(
                parseAmount: { try PlatformWalletManager.parseShieldedTipAmount($0) },
                review: { name in
                    try await walletManager.reviewShieldedTip(walletId: walletId, username: name)
                },
                send: { tip in
                    try await walletManager.sendShieldedTip(
                        walletId: walletId,
                        source: .identity(identityId),
                        username: tip.username,
                        expectedRecipient: tip.review.recipient,
                        amount: tip.amount,
                        memo: tip.memo,
                        resolver: MnemonicResolver()
                    )
                }
            )
        )
    }
}

private struct ShieldedTipSheetContent: View {
    let identity: PersistentIdentity
    @StateObject private var flow: ShieldedTipFlow

    @EnvironmentObject private var shieldedService: ShieldedService
    @Environment(\.dismiss) private var dismiss

    @State private var syncStatus: ShieldedCheckNowStatus = .idle
    /// Bumped after each send so the balance is read again.
    @State private var balanceReload = 0

    init(identity: PersistentIdentity, flow: @autoclosure @escaping () -> ShieldedTipFlow) {
        self.identity = identity
        _flow = StateObject(wrappedValue: flow())
    }

    private var senderName: String {
        identity.ownedMainDpnsName ?? identity.dpnsName
            ?? String(identity.identityIdBase58.prefix(12)) + "…"
    }

    var body: some View {
        NavigationStack {
            Form {
                fromSection
                toSection
                amountSection
                if let tip = flow.reviewed {
                    reviewSection(tip)
                }
                actionSection
            }
            .navigationTitle("Shielded tip")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done") { dismiss() }
                        .disabled(flow.isBusy)
                        .accessibilityIdentifier("dashpay.tip.done")
                }
            }
            .interactiveDismissDisabled(flow.isBusy)
        }
    }

    // MARK: Sections

    private var fromSection: some View {
        Section {
            VStack(alignment: .leading, spacing: 2) {
                Text(senderName)
                    .font(.subheadline)
                    .fontWeight(.medium)
                Text("Paid from this identity's own shielded account.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            .accessibilityElement(children: .combine)
            .accessibilityIdentifier("dashpay.tip.from")
            ShieldedTipBalanceRow(identity: identity, reloadToken: balanceReload)
        } header: {
            Text("From")
        }
    }

    private var toSection: some View {
        Section {
            if flow.usernameIsFixed {
                Text(flow.username)
                    .accessibilityIdentifier("dashpay.tip.username")
            } else {
                TextField("Username", text: $flow.username)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .disabled(flow.inputsLocked)
                    .accessibilityIdentifier("dashpay.tip.username")
            }
        } header: {
            Text("To")
        } footer: {
            Text("The tip pays the shielded address the name's DashPay profile publishes.")
        }
    }

    private var amountSection: some View {
        Section {
            TextField("Amount in DASH", text: $flow.amountText)
                .keyboardType(.decimalPad)
                .disabled(flow.inputsLocked)
                .accessibilityIdentifier("dashpay.tip.amount")
            switch flow.amount {
            case .empty:
                EmptyView()
            case .parsed(let credits):
                Text("Sends \(formatCreditsAsDash(credits))")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .accessibilityIdentifier("dashpay.tip.amount.parsed")
            case .refused(let reason):
                Text(reason)
                    .font(.caption)
                    .foregroundColor(.red)
                    .accessibilityIdentifier("dashpay.tip.amount.error")
            }
            TextField("Memo (optional)", text: $flow.memo)
                .disabled(flow.inputsLocked)
                .accessibilityIdentifier("dashpay.tip.memo")
        } header: {
            Text("Amount")
        } footer: {
            Text("The memo is shown to the recipient. The wallet checks it when the tip is sent.")
        }
    }

    @ViewBuilder
    private func reviewSection(_ tip: ShieldedTipReviewed) -> some View {
        let review = tip.review
        Section {
            labeled("Recipient identity", review.recipient.identityId.toBase58String())
                .accessibilityIdentifier("dashpay.tip.recipient.identity")
            labeled("Shielded address", addressText(review.recipient.address))
                .accessibilityIdentifier("dashpay.tip.recipient.address")
            labeled("Amount", formatCreditsAsDash(tip.amount))
                .accessibilityIdentifier("dashpay.tip.review.amount")
            if let memo = tip.memo {
                labeled("Memo", memo)
                    .accessibilityIdentifier("dashpay.tip.review.memo")
            }

            let warning = ShieldedTipPresentation.changeIsWarning(review.change)
            Label(
                ShieldedTipPresentation.changeText(review.change),
                systemImage: warning ? "exclamationmark.triangle.fill" : "checkmark.circle"
            )
            .font(.caption)
            .foregroundColor(warning ? .orange : .secondary)
            .accessibilityIdentifier("dashpay.tip.change")

            if let previous = ShieldedTipPresentation.previousRecipient(review.change) {
                labeled("Previously identity", previous.identityId.toBase58String())
                    .foregroundColor(.orange)
                    .accessibilityIdentifier("dashpay.tip.change.previous.identity")
                labeled("Previously address", addressText(previous.address))
                    .foregroundColor(.orange)
                    .accessibilityIdentifier("dashpay.tip.change.previous.address")
            }

            if review.unresolved {
                Label(ShieldedTipPresentation.unresolvedMessage, systemImage: "hourglass")
                    .font(.caption)
                    .foregroundColor(.orange)
                    .accessibilityIdentifier("dashpay.tip.unresolved")
            }
        } header: {
            Text("Review")
        }
    }

    @ViewBuilder
    private var actionSection: some View {
        Section {
            if case .reviewFailed(let reason) = flow.phase {
                Text(reason)
                    .font(.caption)
                    .foregroundColor(.red)
                    .accessibilityIdentifier("dashpay.tip.review.error")
            }

            if case .finished(let outcome) = flow.phase {
                outcomeView(outcome)
            }

            if flow.reviewed != nil, !flow.inputsLocked {
                Button {
                    Task {
                        await flow.send()
                        balanceReload += 1
                    }
                } label: {
                    Label("Send tip", systemImage: "paperplane.fill")
                        .fontWeight(.medium)
                }
                .disabled(!flow.canSend)
                .accessibilityIdentifier("dashpay.tip.send")
            } else if !flow.inputsLocked, !flow.offersReviewAgain {
                Button {
                    Task { await flow.review() }
                } label: {
                    Label("Review", systemImage: "magnifyingglass")
                }
                .disabled(!flow.canReview)
                .accessibilityIdentifier("dashpay.tip.review")
            }

            if flow.offersReviewAgain {
                Button("Review again") {
                    Task { await flow.review() }
                }
                .disabled(!flow.canReview)
                .accessibilityIdentifier("dashpay.tip.reviewAgain")
            }

            if flow.offersShieldedSync {
                Button {
                    Task { await syncShielded() }
                } label: {
                    Label("Sync shielded now", systemImage: "arrow.triangle.2.circlepath")
                }
                .disabled(syncStatus == .checking || shieldedService.isSyncing)
                .accessibilityIdentifier("dashpay.tip.sync")
                if let text = syncStatus.text {
                    Text(text)
                        .font(.caption)
                        .foregroundColor(syncStatus.isFailure ? .red : .secondary)
                        .accessibilityIdentifier("dashpay.tip.sync.status")
                }
            }

            if flow.offersTipAgain {
                Button("Send another tip") { flow.tipAgain() }
                    .accessibilityIdentifier("dashpay.tip.tipAgain")
            }

            if flow.isBusy {
                HStack(spacing: 8) {
                    ProgressView()
                    Text(flow.phase == .reviewing ? "Reviewing…" : "Sending…")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .accessibilityIdentifier("dashpay.tip.busy")
            }
        }
    }

    @ViewBuilder
    private func outcomeView(_ outcome: ShieldedTipSendOutcome) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Label(outcome.title, systemImage: outcomeIcon(outcome))
                .font(.subheadline)
                .fontWeight(.medium)
                .foregroundColor(outcomeColor(outcome))
            Text(outcome.message)
                .font(.caption)
            if let detail = outcome.detail {
                Text(detail)
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
        .accessibilityElement(children: .combine)
        .accessibilityIdentifier(outcome.identifier)
    }

    // MARK: Helpers

    private func labeled(_ title: String, _ value: String) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(title)
                .font(.caption2)
                .foregroundColor(.secondary)
            Text(value)
                .font(.caption.monospaced())
                .textSelection(.enabled)
        }
        .accessibilityElement(children: .combine)
    }

    /// The address as the wallet renders it for the identity's network, or
    /// its hex when the wallet cannot render it there.
    private func addressText(_ address: Data) -> String {
        DashPayProfile.paymentAddressString(kind: .shielded, bytes: address, network: identity.network)
            ?? "\(address.toHexString()) (raw; not a shielded address on \(identity.network.displayName))"
    }

    private func outcomeIcon(_ outcome: ShieldedTipSendOutcome) -> String {
        switch outcome {
        case .submitted: return "checkmark.circle.fill"
        case .mayHaveGoneThrough: return "questionmark.circle.fill"
        case .earlierTipPending: return "hourglass"
        case .notSent: return "xmark.octagon.fill"
        }
    }

    private func outcomeColor(_ outcome: ShieldedTipSendOutcome) -> Color {
        switch outcome {
        case .submitted: return .green
        case .mayHaveGoneThrough, .earlierTipPending: return .orange
        case .notSent: return .red
        }
    }

    /// The app's manual shielded sync; it syncs every bound wallet.
    private func syncShielded() async {
        syncStatus = .checking
        let outcome = await shieldedService.manualSync()
        syncStatus = .after(outcome)
    }
}

/// The shielded balance of `identity`'s own account, as the wallet reports
/// it, with a way to prove the account when it is not verified yet. Read
/// again when a shielded sync of the identity's wallet ends and when
/// `reloadToken` changes; a busy store keeps the last state and reads again.
struct ShieldedTipBalanceRow: View {
    let identity: PersistentIdentity
    var reloadToken: Int = 0

    @EnvironmentObject private var walletManager: PlatformWalletManager

    @State private var state = ShieldedTipBalanceState()
    @State private var syncs = 0
    @State private var preparing = false
    @State private var prepareError: String?

    private struct ReadInputs: Equatable {
        let reload: Int
        let syncs: Int
    }

    /// The pause before reading again while the store is busy.
    private static let busyRetry: Duration = .seconds(2)

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("Shielded balance")
                .font(.caption2)
                .foregroundColor(.secondary)
            Text(state.text)
                .font(.caption)
                .foregroundColor(state.isFailure ? .red : .primary)
                .accessibilityIdentifier("dashpay.tip.balance")
            if state.offersPrepare {
                Button {
                    Task { await prepare() }
                } label: {
                    HStack(spacing: 8) {
                        if preparing { ProgressView() }
                        Text("Prepare this identity's shielded account")
                    }
                }
                .buttonStyle(.borderless)
                .disabled(preparing)
                .accessibilityIdentifier("dashpay.tip.balance.prepare")
            }
            if let prepareError {
                Text(prepareError)
                    .font(.caption)
                    .foregroundColor(.red)
                    .accessibilityIdentifier("dashpay.tip.balance.prepareError")
            }
        }
        .task(id: ReadInputs(reload: reloadToken, syncs: syncs)) {
            await readUntilNotBusy()
        }
        .onReceive(walletManager.$lastShieldedSyncEvent) { event in
            guard let walletId = identity.wallet?.walletId,
                  ShieldedSyncEventScope.covers(event, walletId: walletId) else { return }
            syncs += 1
        }
    }

    private func read() async {
        guard let walletId = identity.wallet?.walletId else {
            state.record(error: PlatformWalletError.identityNotFound(
                "This identity has no wallet on this device."))
            return
        }
        do {
            let balance = try await walletManager.shieldedIdentityBalance(
                walletId: walletId, identityId: identity.identityId)
            state.record(balance)
        } catch {
            state.record(error: error)
        }
    }

    private func readUntilNotBusy() async {
        await read()
        while state.readsAgain, !Task.isCancelled {
            do {
                try await Task.sleep(for: Self.busyRetry)
            } catch {
                return
            }
            await read()
        }
    }

    /// Prove the identity's shielded account with the seed for this session
    /// (`prepareShieldedIdentityAddress`), then read the balance again.
    private func prepare() async {
        guard let walletId = identity.wallet?.walletId else { return }
        preparing = true
        prepareError = nil
        defer { preparing = false }
        do {
            _ = try await walletManager.prepareShieldedIdentityAddress(
                walletId: walletId,
                identityId: identity.identityId,
                resolver: MnemonicResolver()
            )
        } catch {
            prepareError = "Couldn't prepare the shielded account: \(error.localizedDescription)"
        }
        await readUntilNotBusy()
    }
}
