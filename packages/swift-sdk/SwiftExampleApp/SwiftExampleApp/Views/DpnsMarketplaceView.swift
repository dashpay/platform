import SwiftUI
import SwiftData
import SwiftDashSDK

/// End-to-end DPNS username marketplace for one wallet-managed identity.
///
/// The identity is both the seller for owned-name actions and the buyer for
/// purchases. All protocol rules, signing-key selection, confirmation, and
/// persistence remain in `ManagedPlatformWallet`; this view owns only user
/// input, confirmations, and presentation.
struct DpnsMarketplaceView: View {
    let identity: PersistentIdentity

    @EnvironmentObject private var walletManager: PlatformWalletManager

    @State private var segment: Segment = .browse
    @State private var searchText = ""
    @State private var searchResults: [DpnsMarketplaceName] = []
    @State private var trackedNames: [DpnsNameStateRow] = []
    @State private var isLoading = false
    @State private var syncRunning = false
    @State private var syncInFlight = false
    @State private var lastSync: Date?
    @State private var lastSummary: String?
    @State private var activeAction: DpnsMarketplaceAction?
    @State private var historyRequest: DpnsHistoryRequest?
    @State private var alert: DpnsMarketplaceAlert?

    private enum Segment: String, CaseIterable, Identifiable {
        case browse = "Browse"
        case myNames = "My Names"

        var id: String { rawValue }
    }

    private var wallet: ManagedPlatformWallet? {
        guard let walletId = identity.wallet?.walletId else { return nil }
        return walletManager.wallet(for: walletId)
    }

    var body: some View {
        List {
            syncSection

            Section {
                Picker("Marketplace section", selection: $segment) {
                    ForEach(Segment.allCases) { value in
                        Text(value.rawValue).tag(value)
                    }
                }
                .pickerStyle(.segmented)
                .accessibilityIdentifier("dpnsMarketplace.segment")
            }

            switch segment {
            case .browse:
                browseSection
            case .myNames:
                myNamesSection
            }
        }
        .navigationTitle("Name Marketplace")
        .navigationBarTitleDisplayMode(.inline)
        .searchable(
            text: $searchText,
            placement: .navigationBarDrawer(displayMode: .always),
            prompt: "Name prefix (empty browses all)"
        )
        .onSubmit(of: .search) {
            Task { await search() }
        }
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Button {
                    Task { await refresh(runSync: true) }
                } label: {
                    if isLoading {
                        ProgressView()
                    } else {
                        Image(systemName: "arrow.clockwise")
                    }
                }
                .disabled(isLoading || wallet == nil)
                .accessibilityLabel("Refresh DPNS marketplace")
                .accessibilityIdentifier("dpnsMarketplace.refresh")
            }
        }
        .refreshable {
            await refresh(runSync: true)
        }
        .task {
            await refresh(runSync: true)
        }
        .sheet(item: $activeAction) { action in
            if let wallet {
                DpnsMarketplaceActionSheet(
                    identity: identity,
                    wallet: wallet,
                    action: action
                ) { confirmed in
                    lastSummary = "Confirmed \(confirmed.label).dash"
                    Task { await refresh(runSync: false) }
                }
            }
        }
        .sheet(item: $historyRequest) { request in
            if let wallet {
                DpnsMarketplaceHistoryView(wallet: wallet, name: request.name)
            }
        }
        .alert(item: $alert) { item in
            Alert(
                title: Text(item.title),
                message: Text(item.message),
                dismissButton: .default(Text("OK"))
            )
        }
    }

    private var syncSection: some View {
        Section("Sync") {
            LabeledContent("Background sync") {
                Label(
                    syncRunning ? "Running" : "Stopped",
                    systemImage: syncRunning ? "checkmark.circle.fill" : "pause.circle"
                )
                .foregroundStyle(syncRunning ? .green : .secondary)
            }
            if syncInFlight {
                LabeledContent("Current pass") {
                    HStack(spacing: 8) {
                        ProgressView().controlSize(.small)
                        Text("Syncing")
                    }
                }
            }
            LabeledContent("Last completed") {
                Text(lastSync?.formatted(date: .abbreviated, time: .shortened) ?? "Not yet")
                    .foregroundStyle(.secondary)
            }
            if let lastSummary {
                Text(lastSummary)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .accessibilityIdentifier("dpnsMarketplace.syncSummary")
            }
        }
    }

    @ViewBuilder
    private var browseSection: some View {
        Section {
            Button {
                Task { await search() }
            } label: {
                Label(
                    searchText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                        ? "Browse names" : "Search names",
                    systemImage: "magnifyingglass"
                )
            }
            .disabled(isLoading || wallet == nil)
            .accessibilityIdentifier("dpnsMarketplace.searchButton")
        } footer: {
            Text(
                "Search is prefix-based. An empty prefix browses names alphabetically; "
                    + "listings cannot be sorted globally by price."
            )
        }

        Section("Results") {
            if searchResults.isEmpty {
                ContentUnavailableView(
                    isLoading ? "Loading names…" : "No names",
                    systemImage: "storefront",
                    description: Text("Search for a prefix, or leave it empty to browse.")
                )
            } else {
                ForEach(searchResults, id: \.documentId) { name in
                    searchResultRow(name)
                }
            }
        }
    }

    @ViewBuilder
    private var myNamesSection: some View {
        Section("Tracked names") {
            if trackedNames.isEmpty {
                ContentUnavailableView(
                    "No tracked names",
                    systemImage: "person.crop.circle.badge.questionmark",
                    description: Text(
                        syncInFlight
                            ? "The first marketplace sync is still running."
                            : "Refresh after the identity owns a DPNS name."
                    )
                )
            } else {
                ForEach(trackedNames, id: \.documentId) { row in
                    trackedNameRow(row)
                }
            }
        }
    }

    private func searchResultRow(_ name: DpnsMarketplaceName) -> some View {
        let ownedByIdentity = name.ownerId == identity.identityId
        return HStack(alignment: .top, spacing: 12) {
            VStack(alignment: .leading, spacing: 5) {
                Text("\(name.label).dash")
                    .font(.headline)
                if let price = name.priceCredits {
                    Text(DpnsMarketplaceUI.price(price))
                        .font(.subheadline)
                        .foregroundStyle(.blue)
                } else {
                    Text("Not for sale")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }
                HStack(spacing: 6) {
                    if ownedByIdentity {
                        Label("Yours", systemImage: "person.fill.checkmark")
                            .foregroundStyle(.green)
                    }
                    Text("Owner \(DpnsMarketplaceUI.shortIdentifier(name.ownerId))")
                        .foregroundStyle(.secondary)
                }
                .font(.caption)
            }
            Spacer()
            Menu {
                if ownedByIdentity {
                    Button("List or re-price", systemImage: "tag") {
                        activeAction = .setPrice(name: name.label, currentPrice: name.priceCredits)
                    }
                    if name.priceCredits != nil {
                        Button("Delist", systemImage: "tag.slash", role: .destructive) {
                            activeAction = .delist(name: name.label, currentPrice: name.priceCredits)
                        }
                    }
                    Button("Transfer", systemImage: "person.crop.circle.badge.arrow.forward") {
                        activeAction = .transfer(name: name.label)
                    }
                } else if let price = name.priceCredits {
                    Button("Purchase", systemImage: "cart") {
                        activeAction = .purchase(name: name.label, expectedPrice: price)
                    }
                }
                Button("History", systemImage: "clock.arrow.circlepath") {
                    historyRequest = .init(name: name.label)
                }
            } label: {
                Image(systemName: "ellipsis.circle")
                    .font(.title3)
            }
            .accessibilityLabel("Actions for \(name.label).dash")
            .accessibilityIdentifier("dpnsMarketplace.search.\(name.normalizedLabel).actions")
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("dpnsMarketplace.search.\(name.normalizedLabel)")
    }

    private func trackedNameRow(_ row: DpnsNameStateRow) -> some View {
        HStack(alignment: .top, spacing: 12) {
            VStack(alignment: .leading, spacing: 5) {
                Text("\(row.label).dash")
                    .font(.headline)
                Text(DpnsMarketplaceUI.status(row.status))
                    .font(.subheadline)
                    .foregroundStyle(DpnsMarketplaceUI.statusColor(row.status))
                if let price = row.priceCredits {
                    Text(DpnsMarketplaceUI.price(price))
                        .font(.caption)
                        .foregroundStyle(.blue)
                }
                Text("Updated \(DpnsMarketplaceUI.date(row.lastSyncedAtMs))")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            Menu {
                if case .owned = row.status {
                    Button("List or re-price", systemImage: "tag") {
                        activeAction = .setPrice(name: row.label, currentPrice: row.priceCredits)
                    }
                    if row.priceCredits != nil {
                        Button("Delist", systemImage: "tag.slash", role: .destructive) {
                            activeAction = .delist(name: row.label, currentPrice: row.priceCredits)
                        }
                    }
                    Button("Transfer", systemImage: "person.crop.circle.badge.arrow.forward") {
                        activeAction = .transfer(name: row.label)
                    }
                }
                Button("History", systemImage: "clock.arrow.circlepath") {
                    historyRequest = .init(name: row.label)
                }
            } label: {
                Image(systemName: "ellipsis.circle")
                    .font(.title3)
            }
            .accessibilityLabel("Actions for \(row.label).dash")
            .accessibilityIdentifier("dpnsMarketplace.owned.\(row.normalizedLabel).actions")
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("dpnsMarketplace.owned.\(row.normalizedLabel)")
    }

    @MainActor
    private func refresh(runSync: Bool) async {
        guard let wallet else {
            alert = .init(title: "Wallet unavailable", message: "The wallet that owns this identity is not loaded.")
            return
        }
        guard !isLoading else { return }
        isLoading = true
        defer { isLoading = false }

        do {
            if runSync {
                let summary = try await walletManager.dpnsSyncNow()
                if summary.syncUnixSeconds > 0 {
                    lastSummary = "Synced \(summary.success) wallet(s); \(summary.errors) error(s)."
                } else if try walletManager.isDpnsSyncing() {
                    lastSummary = "A marketplace sync is already in progress."
                }
            }
            trackedNames = try await wallet.myDpnsMarketplaceNames(identityId: identity.identityId)
            searchResults = try await wallet.searchDpnsMarketplace(
                prefix: searchText.trimmingCharacters(in: .whitespacesAndNewlines),
                limit: 50
            )
            try readSyncState()
        } catch {
            alert = .init(title: "Marketplace refresh failed", message: DpnsMarketplaceUI.error(error))
            try? readSyncState()
        }
    }

    @MainActor
    private func search() async {
        guard let wallet else {
            alert = .init(title: "Wallet unavailable", message: "The wallet that owns this identity is not loaded.")
            return
        }
        guard !isLoading else { return }
        isLoading = true
        defer { isLoading = false }
        do {
            searchResults = try await wallet.searchDpnsMarketplace(
                prefix: searchText.trimmingCharacters(in: .whitespacesAndNewlines),
                limit: 50
            )
        } catch {
            alert = .init(title: "Search failed", message: DpnsMarketplaceUI.error(error))
        }
    }

    @MainActor
    private func readSyncState() throws {
        syncRunning = try walletManager.isDpnsSyncRunning()
        syncInFlight = try walletManager.isDpnsSyncing()
        let unix = try walletManager.dpnsLastSyncUnixSeconds()
        lastSync = unix == 0 ? nil : Date(timeIntervalSince1970: TimeInterval(unix))
    }
}

// MARK: - Trade confirmation sheet

enum DpnsMarketplaceAction: Identifiable {
    case setPrice(name: String, currentPrice: UInt64?)
    case delist(name: String, currentPrice: UInt64?)
    case transfer(name: String)
    case purchase(name: String, expectedPrice: UInt64)

    var id: String {
        switch self {
        case .setPrice(let name, _): return "price:\(name)"
        case .delist(let name, _): return "delist:\(name)"
        case .transfer(let name): return "transfer:\(name)"
        case .purchase(let name, _): return "purchase:\(name)"
        }
    }

    var name: String {
        switch self {
        case .setPrice(let name, _), .delist(let name, _), .transfer(let name),
             .purchase(let name, _):
            return name
        }
    }
}

private struct DpnsMarketplaceActionSheet: View {
    let identity: PersistentIdentity
    let wallet: ManagedPlatformWallet
    let action: DpnsMarketplaceAction
    let onConfirmed: (DpnsMarketplaceName) -> Void

    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    @State private var priceCredits: String
    @State private var recipient: RecipientSelection?
    @State private var isSubmitting = false
    @State private var confirmed: DpnsMarketplaceName?
    @State private var alert: DpnsMarketplaceAlert?

    init(
        identity: PersistentIdentity,
        wallet: ManagedPlatformWallet,
        action: DpnsMarketplaceAction,
        onConfirmed: @escaping (DpnsMarketplaceName) -> Void
    ) {
        self.identity = identity
        self.wallet = wallet
        self.action = action
        self.onConfirmed = onConfirmed
        if case .setPrice(_, let current) = action {
            _priceCredits = State(initialValue: current.map { String($0) } ?? "")
        } else {
            _priceCredits = State(initialValue: "")
        }
    }

    var body: some View {
        NavigationStack {
            Form {
                Section("Name") {
                    LabeledContent("DPNS name", value: "\(action.name).dash")
                    LabeledContent("Acting identity", value: DpnsMarketplaceUI.shortIdentifier(identity.identityId))
                }

                if let confirmed {
                    Section("Confirmed") {
                        Label("Platform confirmed the transition", systemImage: "checkmark.seal.fill")
                            .foregroundStyle(.green)
                        if let price = confirmed.priceCredits {
                            LabeledContent("Current price", value: DpnsMarketplaceUI.price(price))
                        } else {
                            LabeledContent("Listing", value: "Not for sale")
                        }
                        LabeledContent("Owner", value: DpnsMarketplaceUI.shortIdentifier(confirmed.ownerId))
                    }
                } else {
                    inputSection
                    confirmationSection
                }
            }
            .navigationTitle(title)
            .navigationBarTitleDisplayMode(.inline)
            .interactiveDismissDisabled(isSubmitting)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(confirmed == nil ? "Cancel" : "Done") { dismiss() }
                        .disabled(isSubmitting)
                }
            }
            .alert(item: $alert) { item in
                Alert(
                    title: Text(item.title),
                    message: Text(item.message),
                    dismissButton: .default(Text("OK"))
                )
            }
        }
    }

    @ViewBuilder
    private var inputSection: some View {
        switch action {
        case .setPrice:
            Section("Price") {
                TextField("Price in credits", text: $priceCredits)
                    .keyboardType(.numberPad)
                    .accessibilityIdentifier("dpnsMarketplace.action.priceCredits")
                if let parsedPrice {
                    Text(DpnsMarketplaceUI.price(parsedPrice))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
        case .delist(_, let currentPrice):
            Section("Listing") {
                Text("This removes the current listing while keeping the name on this identity.")
                if let currentPrice {
                    LabeledContent("Current price", value: DpnsMarketplaceUI.price(currentPrice))
                }
            }
        case .transfer:
            Section("Recipient") {
                RecipientPickerView(
                    selection: $recipient,
                    wallet: wallet,
                    network: identity.network,
                    exclude: identity.identityId
                )
                .accessibilityIdentifier("dpnsMarketplace.action.recipient")
            }
        case .purchase(_, let expectedPrice):
            Section("Purchase") {
                LabeledContent("Confirmed price", value: DpnsMarketplaceUI.price(expectedPrice))
                LabeledContent("Available balance", value: identity.formattedBalance)
                Text(
                    "The exact price shown above is sent to Platform. If the listing "
                        + "changes first, the purchase is rejected without buying at the new price."
                )
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
    }

    private var confirmationSection: some View {
        Section {
            Button {
                submit()
            } label: {
                HStack {
                    Spacer()
                    if isSubmitting {
                        ProgressView().controlSize(.small)
                        Text("Confirming…")
                    } else {
                        Text(confirmButtonTitle)
                    }
                    Spacer()
                }
            }
            .buttonStyle(.borderedProminent)
            .disabled(!canSubmit || isSubmitting)
            .accessibilityIdentifier("dpnsMarketplace.action.confirm")
        } footer: {
            Text(
                "The app may ask you to unlock the identity signing key. The result is "
                    + "shown only after Platform confirms the transition."
            )
        }
    }

    private var parsedPrice: UInt64? {
        UInt64(priceCredits.trimmingCharacters(in: .whitespacesAndNewlines))
    }

    private var canSubmit: Bool {
        switch action {
        case .setPrice:
            return parsedPrice != nil
        case .transfer:
            return recipient != nil
        case .delist, .purchase:
            return true
        }
    }

    private var title: String {
        switch action {
        case .setPrice: return "List Name"
        case .delist: return "Delist Name"
        case .transfer: return "Transfer Name"
        case .purchase: return "Purchase Name"
        }
    }

    private var confirmButtonTitle: String {
        switch action {
        case .setPrice: return "Confirm listing"
        case .delist: return "Confirm delist"
        case .transfer: return "Confirm transfer"
        case .purchase: return "Confirm purchase"
        }
    }

    private func submit() {
        guard canSubmit else { return }
        // Snapshot mutable form state before starting the asynchronous request.
        // `canSubmit` guarantees the fallback values are never sent, while the
        // snapshot also prevents an early return from leaving the sheet stuck
        // in its submitting state if SwiftUI updates the form mid-request.
        let confirmedPrice = parsedPrice ?? 0
        let confirmedRecipientId = recipient?.identityId ?? Data()
        isSubmitting = true
        let signer = KeychainSigner(modelContainer: modelContext.container)
        let identityId = identity.identityId

        Task {
            do {
                let state: DpnsMarketplaceName
                switch action {
                case .setPrice(let name, _):
                    state = try await wallet.setDpnsNamePrice(
                        ownerIdentityId: identityId,
                        name: name,
                        priceCredits: confirmedPrice,
                        signer: signer
                    )
                case .delist(let name, _):
                    state = try await wallet.delistDpnsName(
                        ownerIdentityId: identityId,
                        name: name,
                        signer: signer
                    )
                case .transfer(let name):
                    state = try await wallet.transferDpnsName(
                        ownerIdentityId: identityId,
                        name: name,
                        recipientId: confirmedRecipientId,
                        signer: signer
                    )
                case .purchase(let name, let expectedPrice):
                    state = try await wallet.purchaseDpnsName(
                        purchaserIdentityId: identityId,
                        name: name,
                        expectedPriceCredits: expectedPrice,
                        signer: signer
                    )
                }
                await MainActor.run {
                    confirmed = state
                    isSubmitting = false
                    onConfirmed(state)
                }
            } catch {
                await MainActor.run {
                    alert = .init(title: "\(title) failed", message: DpnsMarketplaceUI.error(error))
                    isSubmitting = false
                }
            }
        }
    }
}

// MARK: - History

private struct DpnsHistoryRequest: Identifiable {
    let name: String
    var id: String { name }
}

private struct DpnsMarketplaceHistoryView: View {
    let wallet: ManagedPlatformWallet
    let name: String

    @Environment(\.dismiss) private var dismiss
    @State private var events: [DpnsNameHistoryEvent] = []
    @State private var isLoading = true
    @State private var alert: DpnsMarketplaceAlert?

    var body: some View {
        NavigationStack {
            List {
                if events.isEmpty && !isLoading {
                    ContentUnavailableView(
                        "No history",
                        systemImage: "clock",
                        description: Text("Platform returned no trade events for \(name).dash.")
                    )
                } else {
                    ForEach(Array(events.enumerated()), id: \.offset) { index, event in
                        VStack(alignment: .leading, spacing: 5) {
                            Text(DpnsMarketplaceUI.historyTitle(event))
                                .font(.headline)
                            Text(DpnsMarketplaceUI.historyDetail(event))
                                .font(.subheadline)
                                .foregroundStyle(.secondary)
                            Text(DpnsMarketplaceUI.historyDate(event))
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                        .accessibilityIdentifier("dpnsMarketplace.history.\(index)")
                    }
                }
            }
            .overlay { if isLoading { ProgressView("Loading history…") } }
            .navigationTitle("\(name).dash History")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }
                }
            }
            .task { await load() }
            .alert(item: $alert) { item in
                Alert(
                    title: Text(item.title),
                    message: Text(item.message),
                    dismissButton: .default(Text("OK"))
                )
            }
        }
    }

    @MainActor
    private func load() async {
        isLoading = true
        defer { isLoading = false }
        do {
            events = try await wallet.dpnsNameHistory(name: name)
        } catch {
            alert = .init(title: "History failed", message: DpnsMarketplaceUI.error(error))
        }
    }
}

// MARK: - Testable presentation rules

struct DpnsMarketplaceAlert: Identifiable {
    let id = UUID()
    let title: String
    let message: String
}

enum DpnsMarketplaceUI {
    static func price(_ credits: UInt64) -> String {
        let dash = Double(credits) / 100_000_000_000.0
        return "\(credits.formatted()) credits (\(dash.formatted(.number.precision(.fractionLength(0...8)))) DASH)"
    }

    static func shortIdentifier(_ identifier: Data) -> String {
        let value = identifier.toBase58String()
        guard value.count > 16 else { return value }
        return "\(value.prefix(8))…\(value.suffix(6))"
    }

    static func date(_ unixMs: UInt64) -> String {
        guard unixMs > 0 else { return "unknown" }
        return Date(timeIntervalSince1970: TimeInterval(unixMs) / 1_000)
            .formatted(date: .abbreviated, time: .shortened)
    }

    static func status(_ status: DpnsNameSaleStatus) -> String {
        switch status {
        case .owned:
            return "Owned"
        case .sold(let to):
            return "Sold to \(shortIdentifier(to))"
        case .transferred(let to):
            return "Transferred to \(shortIdentifier(to))"
        }
    }

    static func statusColor(_ status: DpnsNameSaleStatus) -> Color {
        switch status {
        case .owned: return .green
        case .sold: return .blue
        case .transferred: return .orange
        }
    }

    static func error(_ error: Error) -> String {
        guard let walletError = error as? PlatformWalletError else {
            return error.localizedDescription
        }
        switch walletError {
        case .notForSale:
            return "This name is no longer listed for sale. Refresh before trying again."
        case .priceChanged(_, let expected, let actual):
            return "The listing changed from \(price(expected)) to \(price(actual)). "
                + "Nothing was purchased; review the new price and confirm again."
        case .insufficientIdentityCredits(_, let required, let available):
            return "This identity has \(price(available)), but \(price(required)) is required "
                + "including the fee reserve."
        case .contestedNameNotTradable(let label, let endsAtMs):
            if endsAtMs == 0 {
                return "\(label).dash is in an active contest and cannot be traded until it resolves."
            }
            return "\(label).dash is in an active contest until \(date(endsAtMs)) and cannot be traded yet."
        case .signingKeyUnavailable:
            return "The identity signing key is unavailable. Unlock or repair this wallet's keys, then retry."
        default:
            return walletError.localizedDescription
        }
    }

    static func historyTitle(_ event: DpnsNameHistoryEvent) -> String {
        switch event {
        case .registered: return "Registered"
        case .priceSet: return "Price updated"
        case .purchased: return "Purchased"
        case .transferred(let from, let to, _, _):
            return from == to ? "Delisted" : "Transferred"
        }
    }

    static func historyDetail(_ event: DpnsNameHistoryEvent) -> String {
        switch event {
        case .registered:
            return "The name was registered."
        case .priceSet(let price, _, let blockHeight):
            return "Listed for \(self.price(price))\(block(blockHeight))."
        case .purchased(let price, let seller, let buyer, _, let blockHeight):
            return "\(shortIdentifier(seller)) sold to \(shortIdentifier(buyer)) "
                + "for \(self.price(price))\(block(blockHeight))."
        case .transferred(let from, let to, _, let blockHeight):
            if from == to {
                return "The owner removed the listing\(block(blockHeight))."
            }
            return "\(shortIdentifier(from)) transferred to \(shortIdentifier(to))\(block(blockHeight))."
        }
    }

    static func historyDate(_ event: DpnsNameHistoryEvent) -> String {
        date(event.atMs)
    }

    private static func block(_ height: UInt64?) -> String {
        height.map { " at block \($0)" } ?? ""
    }
}
