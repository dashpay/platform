import SwiftUI
import SwiftData
import SwiftDashSDK

/// Contracts surface — reached from Settings → Platform (pushed onto
/// that screen's stack), or standalone with its own stack. See
/// `embedsOwnNavigationStack`.
///
/// Lays out two sections, optionally wrapped in its own `NavigationStack`:
///
/// 1. A unified search bar at the top that handles three input
///    shapes:
///      - **32-byte id** (hex / base58 / base64): runs a contract
///        fetch, falling back to `getTokenContractInfo` if the
///        contract isn't found (so a token id resolves to its
///        owning contract).
///      - **Keyword** (>= 3 chars): queries the `contractKeywords`
///        document type on Dash Platform's
///        `keyword-search-contract` system contract, dedupes by
///        contract id, and renders the discovered contracts as
///        tap-to-add rows.
///    Search input is debounced (~700ms) on the keyword path; the
///    id path fires on Submit. The "+" toolbar button still opens
///    `LoadDataContractView` for users who prefer the explicit
///    paste-an-id workflow.
///
/// 2. The existing `@Query`-driven list of locally-persisted
///    `PersistentDataContract` rows, sorted by recency. Tapping a
///    row drills into `DataContractDetailsView`.
///
/// All network work is delegated through `ContractDownloader` so the
/// search-bar tap-to-add path and the manual-paste sheet share the
/// exact same fetch + parse + persist pipeline.
struct ContractsTabView: View {
    @EnvironmentObject var platformState: AppState
    @EnvironmentObject var transitionState: TransitionState
    /// Needed only to forward into the presented `DocumentsView` sheet
    /// (which declares it as an `@EnvironmentObject`). Reading it in this
    /// leaf view re-renders only `ContractsTabView`, not the host screen,
    /// so it doesn't reintroduce the progress-tick churn the
    /// `WalletManagerStore` indirection in `ContentView` guards against.
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext

    /// Active network the parent passed in. Drives both the
    /// `dataContracts` and `allTokens` query predicates so contracts
    /// persisted under another network never leak into the list.
    /// Captured at init time and threaded into the SwiftData
    /// predicates below; the parent re-creates the view when the
    /// active network changes (SwiftUI sees `network` move and
    /// re-runs `init`, which rebuilds the `@Query` predicate).
    private let network: Network

    /// When `false`, the view renders without its own `NavigationStack`
    /// so it can attach to a host stack (e.g. the Settings → Contracts
    /// push) instead of nesting a second navigation bar.
    private let embedsOwnNavigationStack: Bool

    @Query private var dataContracts: [PersistentDataContract]

    /// Flat list of every token across every saved contract on the
    /// active network. Tokens carry their network through the parent
    /// `PersistentDataContract` (no own network field), so the
    /// predicate filters via the relationship. Rows surface the
    /// parent contract in the caption so two tokens with the same
    /// name in different contracts stay distinguishable.
    @Query private var allTokens: [PersistentToken]

    init(network: Network, embedsOwnNavigationStack: Bool = true) {
        self.network = network
        self.embedsOwnNavigationStack = embedsOwnNavigationStack
        let target = network.rawValue
        // `PersistentDataContract.predicate(network:)` already exists
        // for this exact use; reuse it here so the keyed-network
        // sentence is in one place.
        _dataContracts = Query(
            filter: PersistentDataContract.predicate(network: network),
            sort: \PersistentDataContract.lastAccessedAt,
            order: .reverse
        )
        // PersistentToken doesn't have its own network field; it
        // inherits via the cascading parent contract relationship.
        // Filter through `dataContract.networkRaw`.
        _allTokens = Query(
            filter: #Predicate<PersistentToken> { token in
                token.dataContract?.networkRaw == target
            },
            sort: \PersistentToken.name
        )
    }

    @State private var showingLoadContract = false
    @State private var showingRegisterContract = false
    /// Drives the documents-browser sheet. `DocumentsView` wraps its own
    /// `NavigationView`, so it's presented as a sheet (not pushed) to
    /// avoid nesting navigation stacks.
    @State private var showingDocuments = false
    @State private var isLoading = false
    @State private var errorMessage: String?
    @State private var showError = false
    /// Saved contract whose details sheet is presented. Driven by the
    /// row tap and presented from this stable `NavigationStack`/`List`
    /// level rather than from inside the per-row `DataContractRow`. A
    /// `.sheet` owned by a `ForEach` row is torn down whenever the
    /// `@Query` re-runs (sync writes invalidate it continuously),
    /// which would collapse a deep drill-down (the document-create
    /// flow) presented inside it. Hoisting it here keeps it stable.
    @State private var selectedContract: PersistentDataContract?

    /// Active preview the user is inspecting. Setting this drives the
    /// sheet presentation; `nil` dismisses the sheet. The struct
    /// pins the in-memory `ModelContainer` alive for the sheet's
    /// lifetime — dropping it tears down every preview-only row.
    @State private var pendingPreview: ContractPreviewState?
    /// In-flight indicator for the Save button inside the preview
    /// sheet. Owned here (not on the sheet view) because the persist
    /// call writes to the *parent's* `modelContext`.
    @State private var isSavingPreview: Bool = false
    /// Inline error rendered alongside the search bar after a failed
    /// save attempt from inside the preview sheet.
    @State private var previewSaveError: String?

    // MARK: Search-bar state

    /// Raw text in the search field. Trimmed before classification.
    @State private var searchQuery: String = ""
    /// Whether a search-driven request (keyword or id) is currently
    /// in flight. Drives the inline `ProgressView`.
    @State private var searchInFlight = false
    /// Inline error rendered under the search field as a `.caption`.
    /// Distinct from `errorMessage` (which feeds the alert used by
    /// the swipe-to-delete handler) so search failures don't open a
    /// modal alert on every keystroke that misses.
    @State private var searchError: String?
    /// Keyword-path results: each row carries a contract id plus the
    /// matching keywords. Resolution to a `PersistentDataContract`
    /// happens on tap.
    @State private var searchResults: [SearchResult] = []
    /// Generation token used to ignore stale async responses. Each
    /// time a new keyword/id query fires we bump this; in-flight
    /// completions check it before mutating UI state.
    @State private var searchGeneration: Int = 0
    /// Most recently dispatched debounce work item for the keyword
    /// path. Cancelled and replaced on every keystroke.
    @State private var pendingKeywordWork: DispatchWorkItem?

    /// Document type on the keyword-search system contract that maps
    /// keywords -> contract ids.
    private static let keywordSearchContractId =
        "BsjE6tQxG47wffZCRQCovFx5rYrAYYC3rTVRWKro27LA"
    private static let keywordSearchDocumentType = "contractKeywords"

    /// Schema-enforced minimum length for the `keyword` field on the
    /// keyword-search contract; firing the query on shorter inputs
    /// would just throw a validation error.
    private static let minKeywordLength = 3
    /// Debounce window for keyword-path queries. Tuned to fire roughly
    /// after the user pauses typing, not on every keystroke.
    private static let keywordDebounceMs = 700
    /// Hard cap on returned keyword-search documents per query.
    private static let keywordSearchLimit: UInt32 = 50

    var body: some View {
        List {
            searchSection
            resultsSection
            contractsSection
            tokensSection
        }
        .navigationTitle("Contracts")
        .navigationBarTitleDisplayMode(.large)
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Button {
                    showingDocuments = true
                } label: {
                    Image(systemName: "doc.text.magnifyingglass")
                }
                .accessibilityLabel("Browse Documents")
                .accessibilityIdentifier("contracts.browseDocuments")
            }
            ToolbarItem(placement: .navigationBarTrailing) {
                Menu {
                    Button {
                        showingLoadContract = true
                    } label: {
                        Label("Load Contract", systemImage: "arrow.down.circle")
                    }
                    Button {
                        showingRegisterContract = true
                    } label: {
                        Label("Register Contract", systemImage: "plus.circle")
                    }
                } label: {
                    Image(systemName: "plus")
                }
                .disabled(isLoading)
                .accessibilityLabel("Add Contract or Token")
            }
        }
        .sheet(isPresented: $showingLoadContract) {
            LoadDataContractView(isLoading: $isLoading)
                .environmentObject(platformState)
                .environment(\.modelContext, modelContext)
        }
        .sheet(isPresented: $showingRegisterContract) {
            RegisterContractSourceView()
                .environmentObject(platformState)
                .environmentObject(transitionState)
                .environment(\.modelContext, modelContext)
        }
        .sheet(isPresented: $showingDocuments) {
            // `DocumentsView` brings its own `NavigationView`, so it's
            // presented as a sheet (never pushed). It declares
            // `AppState`, `PlatformWalletManager`, and `TransitionState`
            // as `@EnvironmentObject`s — forward all three explicitly
            // like the sibling sheets above. The SwiftData
            // `modelContext` is inherited through the sheet, but pin it
            // explicitly to match the surrounding pattern.
            DocumentsView()
                .environmentObject(platformState)
                .environmentObject(walletManager)
                .environmentObject(transitionState)
                .environment(\.modelContext, modelContext)
        }
        .sheet(item: $selectedContract) { contract in
            // Saved-contract details. Presented from this stable
            // container so a deep drill-down inside it (document
            // type -> New Document) isn't torn down when the
            // `@Query` list re-renders.
            DataContractDetailsView(contract: contract)
                .environmentObject(platformState)
                .environment(\.modelContext, modelContext)
        }
        .sheet(item: $pendingPreview) { preview in
            // Render the full saved-contract details view against
            // the throwaway in-memory container so tokens, document
            // types + indexes, and groups all surface via the
            // existing drill-down screens. The container lives on
            // `preview` and is dropped when the sheet dismisses.
            DataContractDetailsView(
                contract: preview.contract,
                onSave: isAlreadySaved(preview.contractIdBase58)
                    ? nil
                    : { Task { await savePreview(preview) } },
                isSaving: isSavingPreview
            )
            .environmentObject(platformState)
            .modelContainer(preview.container)
        }
        .alert("Error", isPresented: $showError) {
            Button("OK") { }
        } message: {
            Text(errorMessage ?? "Unknown error occurred")
        }
        .wrappedInNavigationStack(embedsOwnNavigationStack)
    }

    // MARK: Sections

    /// Top-of-list section holding the search field, inline progress
    /// indicator, and inline error caption.
    @ViewBuilder
    private var searchSection: some View {
        Section {
            HStack(spacing: 8) {
                Image(systemName: "magnifyingglass")
                    .foregroundColor(.secondary)
                TextField(
                    "Search keyword or paste contract / token ID",
                    text: $searchQuery
                )
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled(true)
                .submitLabel(.search)
                .onSubmit {
                    handleSubmit()
                }
                .onChange(of: searchQuery) { _, newValue in
                    handleQueryChange(newValue)
                }

                if searchInFlight {
                    ProgressView()
                        .controlSize(.small)
                } else if !searchQuery.isEmpty {
                    Button {
                        clearSearch()
                    } label: {
                        Image(systemName: "xmark.circle.fill")
                            .foregroundColor(.secondary)
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Clear search")
                }
            }

            if let searchError = searchError {
                Text(searchError)
                    .font(.caption)
                    .foregroundColor(.red)
            }
        }
    }

    /// Section that renders search-result rows above the local
    /// contracts list. Hidden entirely when there's nothing to show
    /// to keep the page quiet on first launch.
    @ViewBuilder
    private var resultsSection: some View {
        if !searchResults.isEmpty {
            Section("Search Results") {
                ForEach(searchResults) { result in
                    SearchResultRow(
                        result: result,
                        alreadySaved: isAlreadySaved(result.contractIdBase58)
                    ) {
                        Task { await tapSearchResult(result) }
                    }
                }
            }
        }
    }

    /// Bottom section: the existing local contracts `@Query` list,
    /// plus the empty-state placeholder when nothing has been
    /// downloaded yet.
    @ViewBuilder
    private var contractsSection: some View {
        if dataContracts.isEmpty {
            Section { emptyState }
        } else {
            Section("My Contracts") {
                ForEach(dataContracts) { contract in
                    DataContractRow(contract: contract) {
                        selectedContract = contract
                    }
                }
                .onDelete(perform: deleteContracts)
            }
        }
    }

    /// Flat "All Tokens" section. Hidden when no tokens are saved —
    /// avoids a noisy empty header on contracts-only setups (e.g.
    /// DPNS, dashpay, withdrawals). Each row drills into the
    /// existing `TokenDetailsView`. Parent contract name shows as a
    /// caption so two same-named tokens in different contracts are
    /// distinguishable; an orphan token whose contract row was
    /// deleted under it falls back to a hex-truncated contract id.
    @ViewBuilder
    private var tokensSection: some View {
        if !allTokens.isEmpty {
            Section("All Tokens (\(allTokens.count))") {
                ForEach(allTokens) { token in
                    NavigationLink(destination: TokenDetailsView(token: token)) {
                        TokenListRow(token: token)
                    }
                }
            }
        }
    }

    /// Empty-state placeholder shown when no contracts have been
    /// downloaded on this device yet. Pushes the user toward the
    /// add sheet via the toolbar "+" button (no second CTA in here
    /// to avoid two add affordances on the same screen).
    @ViewBuilder
    private var emptyState: some View {
        VStack(spacing: 20) {
            Image(systemName: "doc.text")
                .font(.system(size: 60))
                .foregroundColor(.secondary)

            Text("No Contracts")
                .font(.title2)
                .fontWeight(.semibold)

            Text("Search above by keyword or contract ID, " +
                 "or tap + to paste a contract / token ID. Tokens " +
                 "persist as part of the contract that defines them.")
                .font(.subheadline)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .listRowBackground(Color.clear)
        .listRowInsets(EdgeInsets())
    }

    /// Delete handler for swipe-to-delete on the contracts list. The
    /// `PersistentDataContract.tokens` and `.documentTypes`
    /// relationships are cascade-delete, so removing a contract row
    /// also tears down its child token + document-type rows in the
    /// same save.
    private func deleteContracts(at offsets: IndexSet) {
        for index in offsets {
            modelContext.delete(dataContracts[index])
        }

        do {
            try modelContext.save()
        } catch {
            errorMessage = "Failed to delete contract: \(error.localizedDescription)"
            showError = true
        }
    }

    // MARK: Search handling

    /// Reset everything tied to the live search. Called when the
    /// user hits the clear "x" button or swaps to a brand-new query
    /// shape (e.g. typed text -> pasted id).
    private func clearSearch() {
        searchQuery = ""
        searchError = nil
        searchResults = []
        pendingKeywordWork?.cancel()
        pendingKeywordWork = nil
        searchInFlight = false
        searchGeneration &+= 1
    }

    /// Reaction to every keystroke: cancel any pending debounced
    /// keyword query, classify the new input, and either schedule a
    /// keyword search or wait for an explicit Submit (id paths fire
    /// on Submit only).
    private func handleQueryChange(_ raw: String) {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        // New input invalidates any in-flight async work.
        pendingKeywordWork?.cancel()
        pendingKeywordWork = nil
        searchGeneration &+= 1

        if trimmed.isEmpty {
            searchInFlight = false
            searchError = nil
            searchResults = []
            return
        }

        // 32-byte id paths fire on Submit, not on every keystroke —
        // pasting a 64-char hex id shouldn't need a debounce window.
        // Clear stale state — including any keyword-search rows from
        // before the user pasted the id — so the rendered list lines
        // up with the new query shape.
        if isLikelyContractIdBytes(trimmed) != nil {
            searchInFlight = false
            searchError = nil
            searchResults = []
            return
        }

        // Below the schema's minLength=3 we'd just round-trip a
        // validation error. Quietly clear and wait for more chars.
        if trimmed.count < Self.minKeywordLength {
            searchInFlight = false
            searchError = nil
            searchResults = []
            return
        }

        // Debounce the keyword search.
        let generation = searchGeneration
        let work = DispatchWorkItem {
            Task { await runKeywordSearch(trimmed, generation: generation) }
        }
        pendingKeywordWork = work
        DispatchQueue.main.asyncAfter(
            deadline: .now() + .milliseconds(Self.keywordDebounceMs),
            execute: work
        )
    }

    /// Submit handler. ID-shaped inputs run the lookup pipeline
    /// immediately. Keyword-shaped inputs collapse the debounce
    /// window and fire right away (so hitting Return doesn't make the
    /// user wait the full `keywordDebounceMs`).
    private func handleSubmit() {
        let trimmed = searchQuery.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }

        pendingKeywordWork?.cancel()
        pendingKeywordWork = nil
        searchGeneration &+= 1
        let generation = searchGeneration

        if let _ = isLikelyContractIdBytes(trimmed) {
            Task { await runIdLookup(trimmed, generation: generation) }
            return
        }

        if trimmed.count < Self.minKeywordLength {
            searchError = "Keyword search needs at least \(Self.minKeywordLength) characters."
            return
        }

        Task { await runKeywordSearch(trimmed, generation: generation) }
    }

    // MARK: ID-path lookup

    /// Run the id-shaped lookup pipeline. First tries a direct
    /// contract fetch; if that returns a "not found" error, tries
    /// `getTokenContractInfo` to see if the user pasted a token id
    /// instead of a contract id, and follows the resolved contract
    /// id back through the contract fetch path.
    ///
    /// All UI mutations are gated on `generation` so a second query
    /// kicked off mid-flight can't have its older response stomp
    /// the screen.
    private func runIdLookup(_ raw: String, generation: Int) async {
        guard let sdk = platformState.sdk else {
            await MainActor.run {
                guard generation == searchGeneration else { return }
                searchError = "SDK not initialized"
                searchInFlight = false
            }
            return
        }
        // Canonicalize to base58: regardless of which encoding the
        // user pasted, we want a single normalized id string before
        // we hit the network or the duplicate cache.
        guard let bytes = isLikelyContractIdBytes(raw) else {
            await MainActor.run {
                guard generation == searchGeneration else { return }
                searchError = "That ID isn't 32 bytes."
                searchInFlight = false
            }
            return
        }
        let contractIdBase58 = bytes.toBase58String()

        await MainActor.run {
            guard generation == searchGeneration else { return }
            searchInFlight = true
            searchError = nil
            searchResults = []
        }

        // Attempt 1: treat the input as a contract id.
        do {
            let result = try await ContractDownloader.downloadAndPersistContract(
                contractId: contractIdBase58,
                suggestedName: nil,
                sdk: sdk,
                modelContext: modelContext,
                network: platformState.currentNetwork,
                existingContracts: dataContracts
            )
            await MainActor.run {
                guard generation == searchGeneration else { return }
                searchInFlight = false
                searchError = nil
                searchResults = [
                    SearchResult(
                        contractIdBase58: result.contract.idBase58,
                        keywords: [],
                        contextNote: result.alreadyExisted
                            ? "Already saved"
                            : "Saved \(result.contract.name)"
                    )
                ]
                // Don't clear `searchQuery` here. Earlier revisions
                // did, but the resulting `.onChange` fired
                // `handleQueryChange("")` → `searchResults = []`,
                // wiping the success row before SwiftUI rendered
                // it. Leave the field as the user typed; the "x"
                // button is the documented clear path.
            }
            return
        } catch ContractDownloadError.notFound {
            // Fall through to the token-id fallback.
        } catch let err as ContractDownloadError {
            await MainActor.run {
                guard generation == searchGeneration else { return }
                searchInFlight = false
                searchError = err.errorDescription
            }
            return
        } catch {
            await MainActor.run {
                guard generation == searchGeneration else { return }
                searchInFlight = false
                searchError = error.localizedDescription
            }
            return
        }

        // Attempt 2: maybe the input was a token id. Resolve the
        // owning contract id via `getTokenContractInfo` and retry the
        // contract fetch.
        do {
            let info = try await sdk.getTokenContractInfo(tokenId: contractIdBase58)
            guard let resolvedContractId = info["contract_id"] as? String,
                  !resolvedContractId.isEmpty else {
                await MainActor.run {
                    guard generation == searchGeneration else { return }
                    searchInFlight = false
                    searchError = "Not found on \(platformState.currentNetwork.displayName)."
                }
                return
            }
            let position: Int = {
                if let v = info["token_contract_position"] as? Int { return v }
                if let v = info["token_contract_position"] as? NSNumber { return v.intValue }
                return 0
            }()

            let result = try await ContractDownloader.downloadAndPersistContract(
                contractId: resolvedContractId,
                suggestedName: nil,
                sdk: sdk,
                modelContext: modelContext,
                network: platformState.currentNetwork,
                existingContracts: dataContracts
            )
            let truncated = truncateMiddle(resolvedContractId)
            let note: String
            if result.alreadyExisted {
                note = "Token at position \(position) in contract \(truncated) (already saved)"
            } else {
                note = "Token at position \(position) in contract \(truncated) (saved)"
            }
            await MainActor.run {
                guard generation == searchGeneration else { return }
                searchInFlight = false
                searchError = nil
                searchResults = [
                    SearchResult(
                        contractIdBase58: result.contract.idBase58,
                        keywords: [],
                        contextNote: note
                    )
                ]
                // Don't clear `searchQuery` here — see the matching
                // comment in the contract-id success branch above.
            }
        } catch ContractDownloadError.notFound {
            await MainActor.run {
                guard generation == searchGeneration else { return }
                searchInFlight = false
                searchError = "Not found on \(platformState.currentNetwork.displayName)."
            }
        } catch let err as ContractDownloadError {
            await MainActor.run {
                guard generation == searchGeneration else { return }
                searchInFlight = false
                searchError = err.errorDescription
            }
        } catch {
            // The token-info call itself may have errored. Treat
            // anything other than a successful resolution as
            // "nothing here".
            await MainActor.run {
                guard generation == searchGeneration else { return }
                searchInFlight = false
                searchError = "Not found on \(platformState.currentNetwork.displayName)."
            }
        }
    }

    // MARK: Keyword-path search

    /// Query the keyword-search system contract for `contractKeywords`
    /// documents whose `keyword` field starts with the user's query.
    /// Dedupe by contract id, populate `searchResults` for the row
    /// renderer to consume.
    ///
    /// Generation gating mirrors `runIdLookup` — a new keystroke /
    /// submit can supersede this work mid-flight.
    private func runKeywordSearch(_ rawQuery: String, generation: Int) async {
        guard let sdk = platformState.sdk else {
            await MainActor.run {
                guard generation == searchGeneration else { return }
                searchError = "SDK not initialized"
                searchInFlight = false
            }
            return
        }

        // Build the where-clause JSON via JSONSerialization. The FFI
        // accepts the where clause as a JSON *string*, so we need to
        // encode `[{field, operator, value}]` defensively — quoting
        // `value` by string interpolation would break for inputs that
        // contain `"`, `\`, or control characters.
        let whereClause: String
        let orderByClause: String
        do {
            let wherePayload: [[String: Any]] = [[
                "field": "keyword",
                "operator": "startsWith",
                "value": rawQuery
            ]]
            let whereData = try JSONSerialization.data(withJSONObject: wherePayload)
            guard let whereStr = String(data: whereData, encoding: .utf8) else {
                throw ContractDownloadError.fetchFailed("Failed to encode where clause")
            }
            whereClause = whereStr

            // Drive requires an `orderBy` for every range element in
            // the where clause (`startsWith` desugars to a range), and
            // the field must match the byKeyword index on the
            // keyword-search contract — `keyword asc`.
            let orderByPayload: [[String: Any]] = [[
                "field": "keyword",
                "ascending": true
            ]]
            let orderByData = try JSONSerialization.data(withJSONObject: orderByPayload)
            guard let orderByStr = String(data: orderByData, encoding: .utf8) else {
                throw ContractDownloadError.fetchFailed("Failed to encode order by clause")
            }
            orderByClause = orderByStr
        } catch {
            await MainActor.run {
                guard generation == searchGeneration else { return }
                searchError = "Failed to build search query: \(error.localizedDescription)"
                searchInFlight = false
            }
            return
        }

        await MainActor.run {
            guard generation == searchGeneration else { return }
            searchInFlight = true
            searchError = nil
        }

        do {
            let response = try await sdk.documentList(
                dataContractId: Self.keywordSearchContractId,
                documentType: Self.keywordSearchDocumentType,
                whereClause: whereClause,
                orderByClause: orderByClause,
                limit: Self.keywordSearchLimit
            )

            // Dedupe by contract id; collect every keyword hit per
            // contract so the row can show "matches: foo, bar".
            // Preserve first-seen order so the most relevant row
            // (the first returned by drive) renders at the top.
            var orderedIds: [String] = []
            var keywordsById: [String: [String]] = [:]
            if let documents = response["documents"] as? [[String: Any]] {
                for doc in documents {
                    guard let contractIdString = decodeContractIdField(doc["contractId"]) else {
                        continue
                    }
                    let keyword = (doc["keyword"] as? String) ?? ""
                    if keywordsById[contractIdString] == nil {
                        orderedIds.append(contractIdString)
                        keywordsById[contractIdString] = []
                    }
                    if !keyword.isEmpty,
                       !(keywordsById[contractIdString]?.contains(keyword) ?? false) {
                        keywordsById[contractIdString]?.append(keyword)
                    }
                }
            }

            let results: [SearchResult] = orderedIds.map { id in
                SearchResult(
                    contractIdBase58: id,
                    keywords: keywordsById[id] ?? [],
                    contextNote: nil
                )
            }

            await MainActor.run {
                guard generation == searchGeneration else { return }
                searchInFlight = false
                searchError = results.isEmpty ? "No contracts found for \"\(rawQuery)\"." : nil
                searchResults = results
            }
        } catch {
            await MainActor.run {
                guard generation == searchGeneration else { return }
                searchInFlight = false
                searchError = "Search failed: \(error.localizedDescription)"
                searchResults = []
            }
        }
    }

    /// Tap-to-preview for a keyword-search row. Fetches the contract
    /// into a throwaway in-memory `ModelContainer`, then presents
    /// the standard `DataContractDetailsView` against that container
    /// so the user sees every token, document type, index, and group
    /// the contract declares without touching the on-disk store.
    /// Closing the sheet without tapping "Save to Device" drops the
    /// preview entirely.
    private func tapSearchResult(_ result: SearchResult) async {
        guard let sdk = platformState.sdk else {
            await MainActor.run { searchError = "SDK not initialized" }
            return
        }

        await MainActor.run { searchInFlight = true }
        do {
            let preview = try await ContractDownloader.previewContractInMemory(
                contractId: result.contractIdBase58,
                sdk: sdk,
                network: platformState.currentNetwork
            )
            await MainActor.run {
                searchInFlight = false
                searchError = nil
                previewSaveError = nil
                pendingPreview = preview
            }
        } catch let err as ContractDownloadError {
            await MainActor.run {
                searchInFlight = false
                searchError = err.errorDescription
            }
        } catch {
            await MainActor.run {
                searchInFlight = false
                searchError = error.localizedDescription
            }
        }
    }

    /// "Save to Device" handler invoked from the preview sheet. Runs
    /// the standard persist pipeline against this view's
    /// `modelContext` (NOT the preview's in-memory container) so the
    /// new row lands in the on-disk store. Dismisses the sheet on
    /// success; surfaces failures inline under the search bar.
    @MainActor
    private func savePreview(_ preview: ContractPreviewState) async {
        guard let sdk = platformState.sdk else {
            previewSaveError = "SDK not initialized"
            return
        }
        isSavingPreview = true
        previewSaveError = nil
        do {
            _ = try await ContractDownloader.downloadAndPersistContract(
                contractId: preview.contractIdBase58,
                suggestedName: preview.contract.name,
                sdk: sdk,
                modelContext: modelContext,
                network: platformState.currentNetwork,
                existingContracts: dataContracts
            )
            isSavingPreview = false
            pendingPreview = nil
        } catch let err as ContractDownloadError {
            isSavingPreview = false
            previewSaveError = err.errorDescription
            searchError = err.errorDescription
        } catch {
            isSavingPreview = false
            previewSaveError = error.localizedDescription
            searchError = error.localizedDescription
        }
    }

    // MARK: Helpers

    /// Whether a contract id string is already in the local SwiftData
    /// store. Used to badge a "Already saved" indicator on
    /// search-result rows so the user doesn't double-add.
    private func isAlreadySaved(_ idBase58: String) -> Bool {
        guard let bytes = Data.identifier(fromBase58: idBase58) else {
            return false
        }
        return dataContracts.contains(where: { $0.id == bytes })
    }

    /// Try to interpret `raw` as a 32-byte identifier. Accepts hex
    /// (length 64, all hex digits), base58 (any length, decoded form
    /// must be 32 bytes), or base64 (decoded form must be 32 bytes).
    /// Returns the canonical 32-byte representation when one of
    /// those works, or `nil` if none do.
    ///
    /// The check order matters slightly: hex is the cheapest to
    /// rule out, base58 is the user-pasted format we expect 99% of
    /// the time, base64 is the defensive fallback for older
    /// serialized forms.
    private func isLikelyContractIdBytes(_ raw: String) -> Data? {
        let stripped = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        if stripped.isEmpty { return nil }

        // Hex: must be exactly 64 chars and all hex digits.
        if stripped.count == 64,
           stripped.allSatisfy({ $0.isHexDigit }),
           let data = Data(hexString: stripped),
           data.count == 32 {
            return data
        }

        // Base58 — only worth trying for length ranges that could
        // plausibly decode to 32 bytes (~43-44 chars in practice).
        // The decoder happily accepts other lengths but yields the
        // wrong byte count.
        if let data = Data.identifier(fromBase58: stripped), data.count == 32 {
            return data
        }

        // Base64 fallback. `Data(base64Encoded:)` returns nil for
        // most invalid inputs, so the count check is the only filter
        // we need.
        if let data = Data(base64Encoded: stripped), data.count == 32 {
            return data
        }

        return nil
    }

    /// Defensive contract-id field decoder for `contractKeywords`
    /// documents. Drive serializes identifier-typed bytes as base58
    /// JSON strings (verified in `rs-platform-value`'s value
    /// serialization), but older `Value::Bytes32` serializations may
    /// surface as base64 — try both, return the canonical base58 form
    /// when either decodes to 32 bytes.
    private func decodeContractIdField(_ value: Any?) -> String? {
        guard let str = value as? String, !str.isEmpty else {
            return nil
        }
        if let data = Data.identifier(fromBase58: str), data.count == 32 {
            return data.toBase58String()
        }
        if let data = Data(base64Encoded: str), data.count == 32 {
            return data.toBase58String()
        }
        return nil
    }

    /// Compact display form for long base58 ids: keep the first 8
    /// and last 6 chars and bridge them with an ellipsis. Used
    /// inside contextual messages like "Token at position 0 in
    /// contract abcdef12...XyZ987".
    private func truncateMiddle(_ s: String) -> String {
        let head = 8, tail = 6
        guard s.count > head + tail + 1 else { return s }
        let prefix = s.prefix(head)
        let suffix = s.suffix(tail)
        return "\(prefix)...\(suffix)"
    }
}

// MARK: - Search result types

/// A single row in the contracts search results list.
///
/// Pure value type (no Rust handle) — resolution to a
/// `PersistentDataContract` is deferred until the user taps the row,
/// so we never run a contract fetch we don't need.
struct SearchResult: Identifiable, Hashable {
    let contractIdBase58: String
    /// Matching keywords from the keyword-search response. Empty for
    /// id-path results.
    let keywords: [String]
    /// Optional contextual line shown beneath the id — e.g.
    /// "Token at position 2 in contract abcd...XYZ" for token-id
    /// resolutions, or "Already saved" / "Saved <name>" after a
    /// tap-to-add.
    let contextNote: String?

    var id: String {
        // Two rows showing the same contract id should never coexist
        // (we dedupe on insert), so the contract id is stable enough
        // to use as the identity key.
        contractIdBase58
    }
}

/// Renderer for one search result row. Tapping the row triggers the
/// caller-supplied `onTap` closure (which routes through
/// `ContractDownloader` to fetch + persist the contract).
struct SearchResultRow: View {
    let result: SearchResult
    let alreadySaved: Bool
    let onTap: () -> Void

    var body: some View {
        Button(action: onTap) {
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(truncatedId)
                        .font(.subheadline.monospaced())
                        .foregroundColor(.primary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                    Spacer()
                    if alreadySaved {
                        Text("Already saved")
                            .font(.caption2)
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(
                                Capsule().fill(Color.secondary.opacity(0.15))
                            )
                            .foregroundColor(.secondary)
                    } else {
                        Image(systemName: "arrow.down.circle")
                            .foregroundColor(.blue)
                    }
                }

                if !result.keywords.isEmpty {
                    Text("matches: \(result.keywords.joined(separator: ", "))")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .lineLimit(2)
                }

                if let note = result.contextNote {
                    Text(note)
                        .font(.caption2)
                        .foregroundColor(.secondary)
                        .lineLimit(2)
                }
            }
            .padding(.vertical, 2)
        }
        .buttonStyle(PlainButtonStyle())
    }

    /// Visual id form: keep the head + tail, drop the middle. The
    /// row is fixed-width and the full id won't fit anyway.
    private var truncatedId: String {
        let s = result.contractIdBase58
        guard s.count > 16 else { return s }
        return "\(s.prefix(10))...\(s.suffix(6))"
    }
}

/// Row used by the flat "All Tokens" section on the Contracts tab.
/// Headline prefers the token's English plural form (matching the
/// rest of the app's display convention), falling back to its
/// computed `displayName`. Caption shows the parent contract's name
/// so identically-named tokens in different contracts stay
/// distinguishable; truncated contract id is the fallback when the
/// parent row is missing (orphan token, mid-cascade-delete, etc.).
struct TokenListRow: View {
    let token: PersistentToken

    private var headline: String {
        token.getPluralForm() ?? token.displayName
    }

    private var contractCaption: String {
        if let contract = token.dataContract {
            return contract.name
        }
        let id = token.contractId.toBase58String()
        guard id.count > 16 else { return id }
        return "Contract \(id.prefix(8))…\(id.suffix(6))"
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(headline)
                    .font(.headline)
                Spacer()
                Text("pos \(token.position)")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(
                        Capsule().fill(Color.secondary.opacity(0.12))
                    )
            }
            Text(contractCaption)
                .font(.caption)
                .foregroundColor(.secondary)
                .lineLimit(1)
                .truncationMode(.middle)
        }
        .padding(.vertical, 2)
    }
}

#Preview {
    ContractsTabView(network: .testnet)
        .environmentObject(AppState())
        .environmentObject(TransitionState())
        .environmentObject(PlatformWalletManager())
}

private extension View {
    /// Wrap in a `NavigationStack` only when `wrap` is true. Lets a view
    /// own its navigation chrome standalone yet attach to a host stack
    /// when embedded (avoiding a nested second navigation bar).
    @ViewBuilder
    func wrappedInNavigationStack(_ wrap: Bool) -> some View {
        if wrap {
            NavigationStack { self }
        } else {
            self
        }
    }
}
