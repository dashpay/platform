import Combine
import SwiftUI
import SwiftData
import SwiftDashSDK

/// Value-based navigation route to a wallet's tokens in token shielded pools,
/// mirroring `ShieldedActivityRoute`. Pushed from `WalletDetailView` and
/// resolved by a `.navigationDestination(for:)` in `WalletsContentView`.
struct ShieldedTokensRoute: Hashable {
    let walletId: Data
}

// MARK: - List

/// The tokens a wallet holds in the token shielded pools it watches (SH-25).
///
/// Every value comes from Rust (`walletManager.shieldedTokenPoolBalances`):
/// one entry per watched pool and bound account, as the local scan found it.
/// Rust keeps this state in its shielded store, not in SwiftData. The view
/// only formats it: the token's name and decimals when the app has the token
/// locally (matched by contract id and position, display only), otherwise
/// the Base58 token id and the amount in the token's smallest unit. Notes a
/// submitted transfer still holds are not in the balance; Rust reports them
/// apart and the row shows them as "Pending".
///
/// The watched pools are scanned by the ordinary shielded sync, so Refresh
/// (and pull to refresh) runs the app's manual shielded sync and then reads
/// the balances again. A type 28 purchase watches the bought token's pool by
/// itself; "Watch a Token Pool" adds one of the app's known tokens by hand,
/// and swiping a row stops watching that token. **Send** on a row opens
/// `ShieldedTokenSendSheet` for that token and account (type 26, SH-26).
struct ShieldedTokenPoolsView: View {
    let walletId: Data
    @EnvironmentObject private var walletManager: PlatformWalletManager
    @EnvironmentObject private var shieldedService: ShieldedService
    @Query private var tokens: [PersistentToken]

    /// The balances Rust last returned, in its order; `nil` before the first
    /// read finished.
    @State private var balances: [ShieldedTokenPoolBalance]?
    @State private var loadError: String?
    @State private var checkStatus: ShieldedCheckNowStatus = .idle
    @State private var showWatchSheet = false
    @State private var unwatchError: String?
    /// The row whose Send form is open.
    @State private var sendTarget: ShieldedTokenSendTarget?
    /// Only the newest read may apply its result.
    @State private var readGeneration: UInt64 = 0

    /// The balances grouped by token, in the order Rust listed the tokens.
    private var groups: [TokenGroup] {
        var order: [Data] = []
        var byToken: [Data: [ShieldedTokenPoolBalance]] = [:]
        for balance in balances ?? [] {
            if byToken[balance.tokenId] == nil { order.append(balance.tokenId) }
            byToken[balance.tokenId, default: []].append(balance)
        }
        return order.map { TokenGroup(tokenId: $0, accounts: byToken[$0] ?? []) }
    }

    private struct TokenGroup: Identifiable {
        let tokenId: Data
        let accounts: [ShieldedTokenPoolBalance]
        var id: Data { tokenId }
    }

    /// The app's local record of the token, for its name and decimals only.
    private func knownToken(_ balance: ShieldedTokenPoolBalance) -> PersistentToken? {
        tokens.first {
            $0.contractId == balance.dataContractId
                && $0.position == Int(balance.tokenContractPosition)
        }
    }

    var body: some View {
        List {
            Section {
                Button {
                    Task { await refresh() }
                } label: {
                    HStack {
                        Label("Refresh", systemImage: "arrow.clockwise")
                        if checkStatus == .checking || shieldedService.isSyncing {
                            Spacer()
                            ProgressView()
                        }
                    }
                }
                .disabled(checkStatus == .checking || !shieldedService.canResume)
                .accessibilityIdentifier("shieldedTokens.refreshButton")

                Button {
                    showWatchSheet = true
                } label: {
                    Label("Watch a Token Pool", systemImage: "plus.circle")
                }
                .accessibilityIdentifier("shieldedTokens.watchButton")

                if let text = statusText {
                    Text(text)
                        .font(.caption)
                        .foregroundColor(checkStatus.isFailure ? .red : .secondary)
                        .accessibilityIdentifier("shieldedTokens.refreshStatus")
                }
            } footer: {
                Text("Refresh runs a shielded sync, which scans every watched token pool, then reads the balances again.")
            }

            if let loadError {
                Section {
                    Text(loadError)
                        .font(.caption)
                        .foregroundColor(.red)
                        .accessibilityIdentifier("shieldedTokens.loadError")
                }
            }

            if balances == nil, loadError == nil {
                Section {
                    ProgressView()
                        .frame(maxWidth: .infinity)
                        .accessibilityIdentifier("shieldedTokens.loading")
                }
            } else if groups.isEmpty, loadError == nil {
                ContentUnavailableView(
                    "No Watched Token Pools",
                    systemImage: "lock.square.stack",
                    description: Text("Buying a token from the shielded pool watches its pool. You can also watch a token's pool by hand.")
                )
                .accessibilityIdentifier("shieldedTokens.empty")
            }

            ForEach(groups) { group in
                Section {
                    ForEach(group.accounts, id: \.self) { balance in
                        row(balance, showsAccount: group.accounts.count > 1)
                    }
                } header: {
                    tokenHeader(group)
                }
            }
        }
        .accessibilityIdentifier("shieldedTokens.list")
        .navigationTitle("Shielded Tokens")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Button {
                    Task { await refresh() }
                } label: {
                    Image(systemName: "arrow.clockwise")
                }
                .disabled(checkStatus == .checking || !shieldedService.canResume)
                .accessibilityIdentifier("shieldedTokens.toolbarRefreshButton")
            }
        }
        .task(id: ObjectIdentifier(walletManager)) { await loadBalances() }
        .refreshable { await refresh() }
        // A finished sync pass (manual or background) may have scanned the
        // watched pools: read again.
        .onReceive(walletManager.$lastShieldedSyncEvent.dropFirst()) { event in
            if ShieldedSyncEventScope.covers(event, walletId: walletId) {
                Task { await loadBalances() }
            }
        }
        // A Clear purges the shielded store without a sync event.
        .onReceive(shieldedService.$localStateClearCount.dropFirst()) { _ in
            Task { await loadBalances() }
        }
        .sheet(isPresented: $showWatchSheet) {
            ShieldedTokenPoolWatchSheet(
                walletId: walletId,
                watched: balances ?? [],
                onWatched: { Task { await loadBalances() } }
            )
        }
        .sheet(item: $sendTarget) { target in
            ShieldedTokenSendSheet(
                walletId: walletId,
                target: target,
                token: knownToken(target.balance),
                onSubmitted: { Task { await loadBalances() } }
            )
        }
        .alert(
            "Could not stop watching",
            isPresented: Binding(
                get: { unwatchError != nil },
                set: { if !$0 { unwatchError = nil } }
            )
        ) {
            Button("OK") { unwatchError = nil }
        } message: {
            Text(unwatchError ?? "")
        }
    }

    /// The line under Refresh; `nil` when there is nothing to say.
    private var statusText: String? {
        switch checkStatus {
        case .requested:
            return "Sync requested. Each token shows its own last scan result."
        default:
            return checkStatus.text
        }
    }

    @ViewBuilder
    private func tokenHeader(_ group: TokenGroup) -> some View {
        let first = group.accounts.first
        let known = first.flatMap { knownToken($0) }
        VStack(alignment: .leading, spacing: 2) {
            Text(known?.displayName ?? group.tokenId.toBase58String())
                .textCase(nil)
                .lineLimit(1)
                .truncationMode(.middle)
            if known != nil {
                Text(group.tokenId.toBase58String())
                    .font(.caption2.monospaced())
                    .textCase(nil)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }
        }
        .accessibilityIdentifier("shieldedTokens.tokenHeader.\(group.tokenId.toBase58String())")
    }

    @ViewBuilder
    private func row(_ balance: ShieldedTokenPoolBalance, showsAccount: Bool) -> some View {
        let known = knownToken(balance)
        VStack(alignment: .leading, spacing: 4) {
            if showsAccount {
                Text("Account \(balance.accountIndex)")
                    .font(.caption.weight(.semibold))
                    .foregroundColor(.secondary)
            }
            HStack {
                Text("Balance")
                Spacer()
                Text(Self.amountText(balance.balance, token: known))
                    .font(.body.monospacedDigit().weight(.semibold))
                    .accessibilityIdentifier("shieldedTokens.balance")
            }
            Text("\(balance.unspentNoteCount) unspent \(balance.unspentNoteCount == 1 ? "note" : "notes")")
                .font(.caption)
                .foregroundColor(.secondary)
                .accessibilityIdentifier("shieldedTokens.noteCount")
            if balance.reservedBalance > 0 {
                Text("Pending: \(Self.amountText(balance.reservedBalance, token: known))")
                    .font(.caption)
                    .foregroundColor(.orange)
                    .accessibilityIdentifier("shieldedTokens.pendingBalance")
            }
            Text(Self.scanText(balance))
                .font(.caption)
                .foregroundColor(.secondary)
                .accessibilityIdentifier("shieldedTokens.scanState")
            if let lastError = balance.lastError {
                Text("Last scan stopped: \(lastError)")
                    .font(.caption)
                    .foregroundColor(.red)
                    .accessibilityIdentifier("shieldedTokens.lastError")
            }
            Button {
                sendTarget = ShieldedTokenSendTarget(balance: balance, showsAccount: showsAccount)
            } label: {
                Label("Send", systemImage: "paperplane")
            }
            .buttonStyle(.borderless)
            .disabled(balance.balance == 0)
            .accessibilityIdentifier("shieldedTokens.sendButton")
        }
        .padding(.vertical, 2)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier(
            "shieldedTokens.row.\(balance.tokenId.toBase58String()).\(balance.accountIndex)"
        )
        .swipeActions(edge: .trailing) {
            Button(role: .destructive) {
                Task { await unwatch(balance.tokenId) }
            } label: {
                Label("Unwatch", systemImage: "eye.slash")
            }
            .accessibilityIdentifier("shieldedTokens.unwatchButton")
        }
    }

    /// The amount with the token's decimals when the app has the token,
    /// otherwise the raw count in the token's smallest unit.
    static func amountText(_ amount: UInt64, token: PersistentToken?) -> String {
        guard let token else { return "\(amount) (smallest units)" }
        return PersistentToken.formatSupply(String(amount), decimals: token.decimals)
    }

    static func scanText(_ balance: ShieldedTokenPoolBalance) -> String {
        guard let scanned = balance.scannedNoteCount else { return "Not scanned yet" }
        var text: String
        if let pool = balance.poolNoteCount {
            text = "Scanned \(scanned) of \(pool) notes"
        } else {
            text = "Scanned \(scanned) notes"
        }
        if let height = balance.scannedHeight {
            text += " (height \(height))"
        }
        return text
    }

    private func loadBalances() async {
        readGeneration &+= 1
        let generation = readGeneration
        do {
            let result = try await walletManager.shieldedTokenPoolBalances(walletId: walletId)
            guard generation == readGeneration else { return }
            balances = result
            loadError = nil
        } catch {
            guard generation == readGeneration else { return }
            loadError = "Could not read the token pool balances: \(error.localizedDescription)"
        }
    }

    /// The app's existing manual shielded sync (it syncs every bound wallet,
    /// scanning their watched token pools), then a fresh read.
    private func refresh() async {
        checkStatus = .checking
        let outcome = await shieldedService.manualSync()
        checkStatus = .after(outcome)
        await loadBalances()
    }

    private func unwatch(_ tokenId: Data) async {
        do {
            try await walletManager.shieldedTokenPoolUnwatch(walletId: walletId, tokenId: tokenId)
        } catch {
            unwatchError = error.localizedDescription
        }
        await loadBalances()
    }
}

// MARK: - Send

/// The row a Send form is open for: one token and one account.
struct ShieldedTokenSendTarget: Identifiable {
    let balance: ShieldedTokenPoolBalance
    /// The token has more than one account row, so the form names the
    /// account it sends from.
    let showsAccount: Bool
    var id: Data {
        var id = balance.tokenId
        withUnsafeBytes(of: balance.accountIndex.bigEndian) { id.append(contentsOf: $0) }
        return id
    }
}

/// Send tokens from one account's notes in a token's shielded pool to an
/// Orchard address in that pool (type 26, SH-26) through
/// `walletManager.shieldedTokenTransfer`. Rust decides everything: it checks
/// the amount, the token and the network, picks the token notes and the
/// credit pool notes that pay the fee, proves, broadcasts and records the
/// activity. The form only reads the recipient (a pasted bech32m Orchard
/// address, or this account's own one) and the amount, and shows Rust's
/// answer.
///
/// The amount is typed in the token's smallest unit, as a whole number, and
/// passed to Rust unchanged. The app's cached token decimals can be stale or
/// from another network, so they never convert the amount; when the app has
/// them they only give a non-authoritative hint under the field.
struct ShieldedTokenSendSheet: View {
    let walletId: Data
    let target: ShieldedTokenSendTarget
    /// The app's local record of the token, for its name and decimals only.
    let token: PersistentToken?
    let onSubmitted: () -> Void

    @EnvironmentObject private var walletManager: PlatformWalletManager
    @EnvironmentObject private var platformState: AppState
    @Environment(\.dismiss) private var dismiss

    @State private var recipientText = ""
    @State private var amountText = ""
    @State private var isSubmitting = false
    /// Rust accepted the transfer, or it may have gone through: either way
    /// the form must not send it again.
    @State private var resultMessage: String?
    @State private var resultMayHaveGoneThrough = false
    @State private var errorMessage: String?

    private var balance: ShieldedTokenPoolBalance { target.balance }

    /// The 43 raw bytes of the typed recipient when it is an Orchard address
    /// on the current network.
    private var recipientRaw: Data? {
        let trimmed = recipientText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return nil }
        guard case .orchard(let raw) = DashAddress.parse(trimmed, network: platformState.currentNetwork).type
        else { return nil }
        return raw
    }

    private var trimmedAmountText: String {
        amountText.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    /// The amount in the token's smallest unit: the typed whole number,
    /// exactly. `nil` for empty text and for anything else (a decimal point,
    /// a sign, other characters, or a number past `UInt64`).
    private var amount: UInt64? { Self.smallestUnitAmount(trimmedAmountText) }

    /// The typed text is not a whole number of smallest units.
    private var amountRefused: Bool { !trimmedAmountText.isEmpty && amount == nil }

    /// Parse `text` as a whole number of the token's smallest unit: ASCII
    /// digits only, nothing rounded or converted.
    static func smallestUnitAmount(_ text: String) -> UInt64? {
        guard !text.isEmpty, text.utf8.allSatisfy({ $0 >= 0x30 && $0 <= 0x39 }) else {
            return nil
        }
        return UInt64(text)
    }

    /// This account's own default Orchard address, when the wallet's
    /// shielded account is bound.
    private var ownAddress: String? {
        walletManager.shieldedDisplayAddress(
            walletId: walletId,
            account: balance.accountIndex,
            network: platformState.currentNetwork
        )
    }

    private var canSubmit: Bool {
        !isSubmitting && resultMessage == nil && recipientRaw != nil && amount != nil
    }

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    LabeledContent("Token", value: token?.displayName ?? balance.tokenId.toBase58String())
                    if target.showsAccount {
                        LabeledContent("From account", value: "\(balance.accountIndex)")
                    }
                    // In smallest units, the unit the amount is typed in.
                    LabeledContent("Available", value: "\(balance.balance) (smallest units)")
                    if balance.reservedBalance > 0 {
                        LabeledContent("Pending", value: "\(balance.reservedBalance) (smallest units)")
                    }
                }

                Section {
                    TextField("tdash1… or dash1…", text: $recipientText, axis: .vertical)
                        .font(.caption.monospaced())
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .accessibilityIdentifier("shieldedTokenSend.recipient")
                    HStack {
                        Button {
                            if let pasted = UIPasteboard.general.string {
                                recipientText = pasted.trimmingCharacters(in: .whitespacesAndNewlines)
                            }
                        } label: {
                            Label("Paste", systemImage: "doc.on.clipboard")
                        }
                        .buttonStyle(.borderless)
                        .accessibilityIdentifier("shieldedTokenSend.pasteButton")
                        Spacer()
                        if let ownAddress {
                            Button {
                                recipientText = ownAddress
                            } label: {
                                Label("My Address", systemImage: "person.crop.circle")
                            }
                            .buttonStyle(.borderless)
                            .accessibilityIdentifier("shieldedTokenSend.ownAddressButton")
                        }
                    }
                    if !recipientText.isEmpty, recipientRaw == nil {
                        Text("Not a shielded (Orchard) address on this network.")
                            .font(.caption)
                            .foregroundColor(.red)
                    }
                } header: {
                    Text("Recipient")
                } footer: {
                    Text("A shielded (Orchard) address. The tokens stay in the token's shielded pool.")
                }

                Section {
                    TextField("0", text: $amountText)
                        .keyboardType(.numberPad)
                        .font(.body.monospacedDigit())
                        .accessibilityIdentifier("shieldedTokenSend.amount")
                    if amountRefused {
                        Text("Enter a whole number of the token's smallest unit: no decimal point or other characters.")
                            .font(.caption)
                            .foregroundColor(.red)
                            .accessibilityIdentifier("shieldedTokenSend.precisionError")
                    } else if let amount, let token {
                        // Display only: the cached decimals may be stale or
                        // from another network, so they never change what is
                        // sent.
                        Text("≈ \(formatTokenAmount(amount, decimals: token.decimals)) tokens if the token has \(token.decimals) decimals (from this app's token cache).")
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .accessibilityIdentifier("shieldedTokenSend.decimalsHint")
                    }
                } header: {
                    Text("Amount (smallest unit)")
                } footer: {
                    Text("A whole number of the token's smallest unit, sent exactly as typed.")
                }

                Section {
                    Button {
                        Task { await submit() }
                    } label: {
                        HStack {
                            Text(isSubmitting ? "Proving and broadcasting…" : "Send")
                            if isSubmitting {
                                Spacer()
                                ProgressView()
                            }
                        }
                    }
                    .disabled(!canSubmit)
                    .accessibilityIdentifier("shieldedTokenSend.submit")
                } footer: {
                    Text("The fee is paid from this account's shielded credit balance.")
                }

                if let resultMessage {
                    Section {
                        Text(resultMessage)
                            .font(.callout)
                            .foregroundColor(resultMayHaveGoneThrough ? .orange : .green)
                            .accessibilityIdentifier("shieldedTokenSend.result")
                    }
                }
                if let errorMessage {
                    Section {
                        Text(errorMessage)
                            .font(.callout)
                            .foregroundColor(.red)
                            .accessibilityIdentifier("shieldedTokenSend.error")
                    }
                }
            }
            .accessibilityIdentifier("shieldedTokenSend.form")
            .navigationTitle("Send Shielded Tokens")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(resultMessage == nil ? "Cancel" : "Done") { dismiss() }
                        .disabled(isSubmitting)
                        .accessibilityIdentifier("shieldedTokenSend.close")
                }
            }
            .interactiveDismissDisabled(isSubmitting)
        }
    }

    private func submit() async {
        guard let recipient = recipientRaw, let amount else { return }
        isSubmitting = true
        errorMessage = nil
        defer { isSubmitting = false }
        do {
            // Per-operation spend authority: the resolver fires for this
            // spend only.
            try await walletManager.shieldedTokenTransfer(
                walletId: walletId,
                resolver: MnemonicResolver(),
                account: balance.accountIndex,
                dataContractId: balance.dataContractId,
                tokenContractPosition: balance.tokenContractPosition,
                recipientRaw43: recipient,
                amount: amount
            )
            resultMayHaveGoneThrough = false
            resultMessage = "Submitted, pending until the next shielded sync. Do not send it again."
            onSubmitted()
        } catch let unconfirmed as PlatformWalletError
            where SendViewModel.mayHaveGoneThrough(unconfirmed) {
            // The transfer may already be on chain and Rust keeps its notes
            // reserved: report it without inviting a retry, which would sign
            // a second transfer.
            resultMayHaveGoneThrough = true
            resultMessage = "The transfer may have gone through; a shielded sync confirms it once it lands. Do not retry. (\(unconfirmed.localizedDescription))"
            onSubmitted()
        } catch {
            errorMessage = error.localizedDescription
        }
    }
}

// MARK: - Watch

/// Pick one of the app's known tokens on the current network and ask Rust to
/// watch its shielded pool. Rust checks that the token has one (a verified
/// read of its configuration) and refuses it otherwise; the sheet shows
/// Rust's reason.
struct ShieldedTokenPoolWatchSheet: View {
    let walletId: Data
    /// The rows Rust already lists for the wallet, to mark their tokens in
    /// the list (display only: watching again changes nothing).
    let watched: [ShieldedTokenPoolBalance]
    let onWatched: () -> Void

    @EnvironmentObject private var walletManager: PlatformWalletManager
    @EnvironmentObject private var platformState: AppState
    @Environment(\.dismiss) private var dismiss
    @Query(sort: \PersistentToken.name) private var tokens: [PersistentToken]

    /// The token being watched, keyed by its local id.
    @State private var watching: Data?
    @State private var watchError: String?

    /// The app's tokens whose contract is on the current network.
    private var networkTokens: [PersistentToken] {
        let networkRaw = platformState.currentNetwork.rawValue
        return tokens.filter { $0.dataContract?.networkRaw == networkRaw }
    }

    var body: some View {
        NavigationStack {
            List {
                if let watchError {
                    Section {
                        Text(watchError)
                            .font(.caption)
                            .foregroundColor(.red)
                            .accessibilityIdentifier("shieldedTokens.watchSheet.error")
                    }
                }
                if networkTokens.isEmpty {
                    ContentUnavailableView(
                        "No Known Tokens",
                        systemImage: "circle.hexagongrid",
                        description: Text("Load a data contract that defines tokens to pick one here.")
                    )
                    .accessibilityIdentifier("shieldedTokens.watchSheet.empty")
                }
                Section {
                    ForEach(networkTokens, id: \.id) { token in
                        tokenRow(token)
                    }
                } footer: {
                    if !networkTokens.isEmpty {
                        Text("Only a token whose configuration has a shielded pool can be watched.")
                    }
                }
            }
            .accessibilityIdentifier("shieldedTokens.watchSheet")
            .navigationTitle("Watch a Token Pool")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Close") { dismiss() }
                        .accessibilityIdentifier("shieldedTokens.watchSheet.close")
                }
            }
        }
    }

    @ViewBuilder
    private func tokenRow(_ token: PersistentToken) -> some View {
        let isWatched = isListed(token)
        Button {
            Task { await watch(token) }
        } label: {
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text(token.displayName)
                    Text("\(token.contractIdBase58) #\(token.position)")
                        .font(.caption2.monospaced())
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
                Spacer()
                if watching == token.id {
                    ProgressView()
                } else if isWatched {
                    Text("Watching")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
        }
        .disabled(watching != nil)
        .accessibilityIdentifier(
            "shieldedTokens.watchSheet.token.\(token.contractIdBase58).\(token.position)"
        )
    }

    /// Whether Rust already lists this token for the wallet, matched by the
    /// rows' contract id and position (the local token has no token id).
    private func isListed(_ token: PersistentToken) -> Bool {
        watched.contains {
            $0.dataContractId == token.contractId
                && Int($0.tokenContractPosition) == token.position
        }
    }

    private func watch(_ token: PersistentToken) async {
        guard let position = UInt16(exactly: token.position) else {
            watchError = "Token position \(token.position) is out of range."
            return
        }
        watching = token.id
        watchError = nil
        defer { watching = nil }
        do {
            _ = try await walletManager.shieldedTokenPoolWatch(
                walletId: walletId,
                dataContractId: token.contractId,
                tokenContractPosition: position
            )
            onWatched()
            dismiss()
        } catch {
            watchError = error.localizedDescription
        }
    }
}
