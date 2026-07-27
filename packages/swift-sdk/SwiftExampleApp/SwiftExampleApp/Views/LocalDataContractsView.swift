import SwiftUI
import SwiftData
import SwiftDashSDK

struct LocalDataContractsView: View {
    @EnvironmentObject var platformState: AppState
    @Query(sort: \PersistentDataContract.lastAccessedAt, order: .reverse)
    private var dataContracts: [PersistentDataContract]

    @State private var showingLoadContract = false
    @State private var isLoading = false
    @State private var errorMessage: String?
    @State private var showError = false
    /// Contract whose details sheet is presented. Driven from the row
    /// tap and presented at this stable `List` level (not per-row) so
    /// the sheet survives `@Query` re-renders.
    @State private var selectedContract: PersistentDataContract?

    @Environment(\.modelContext) private var modelContext

    var body: some View {
        List {
            if dataContracts.isEmpty {
                VStack(spacing: 20) {
                    Image(systemName: "doc.text")
                        .font(.system(size: 60))
                        .foregroundColor(.secondary)

                    Text("No Local Contracts")
                        .font(.title2)
                        .fontWeight(.semibold)

                    Text("Load data contracts from the network to use them offline")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                        .multilineTextAlignment(.center)
                        .padding(.horizontal)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .listRowBackground(Color.clear)
                .listRowInsets(EdgeInsets())
            } else {
                ForEach(dataContracts) { contract in
                    DataContractRow(contract: contract) {
                        selectedContract = contract
                    }
                }
                .onDelete(perform: deleteContracts)
            }
        }
        .navigationTitle("Local Data Contracts")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Button(action: { showingLoadContract = true }) {
                    Label("Load Contract", systemImage: "arrow.down.circle")
                }
                .disabled(isLoading)
            }
        }
        .sheet(item: $selectedContract) { contract in
            DataContractDetailsView(contract: contract)
        }
        .sheet(isPresented: $showingLoadContract) {
            LoadDataContractView(isLoading: $isLoading)
                .environmentObject(platformState)
                .environment(\.modelContext, modelContext)
        }
        .alert("Error", isPresented: $showError) {
            Button("OK") { }
        } message: {
            Text(errorMessage ?? "Unknown error occurred")
        }
    }

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
}

struct DataContractRow: View {
    let contract: PersistentDataContract
    /// Tap handler supplied by the parent list. The details sheet is
    /// presented from the *stable* list container (see
    /// `ContractsTabView` / `LocalDataContractsView`), NOT from inside
    /// this row. A `.sheet` owned by a row lives inside the list's
    /// `ForEach`; when the `@Query` re-runs (continuous SwiftData
    /// writes from sync invalidate it) the row struct is recreated /
    /// reordered, tearing the sheet's content — and any view pushed
    /// inside it — down. Hoisting the sheet to the stable parent keeps
    /// the presentation (and a deep drill-down like the document-create
    /// flow) alive across list re-renders.
    var onTap: () -> Void = {}

    var displayName: String {
        // Check if this is a token-only contract
        if let tokens = contract.tokens,
           tokens.count == 1,
           let documentTypes = contract.documentTypes,
           documentTypes.isEmpty,
           let token = tokens.first {
            // Use the token's singular form for display
            if let singularName = token.getSingularForm(languageCode: "en") {
                return "\(singularName) Token Contract"
            } else {
                return "Token Contract"
            }
        }

        // Otherwise use the stored name
        return contract.name
    }

    var body: some View {
        Button(action: onTap) {
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(displayName)
                        .font(.headline)
                        .foregroundColor(.primary)
                    Spacer()
                    Image(systemName: "chevron.right")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                Text(contract.idBase58)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)

                HStack {
                    Text("Size: \(ByteCountFormatter.string(fromByteCount: Int64(contract.serializedContract.count), countStyle: .binary))")
                        .font(.caption2)
                        .foregroundColor(.secondary)

                    Spacer()

                    // Statically-computed relative string. A
                    // `Text(date, style: .relative)` here re-renders on
                    // a sub-minute cadence (~1s for "N seconds ago"),
                    // which churns the whole `@Query` contracts list
                    // every second — re-creating any pushed
                    // `DataContractDetailsView` and popping its
                    // drill-downs (e.g. the document-create flow). A
                    // plain string does not auto-refresh.
                    Text("Last used: \(AppDate.relative(contract.lastAccessedAt))")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
            }
            .padding(.vertical, 4)
        }
        .buttonStyle(PlainButtonStyle())
    }
}

struct LoadDataContractView: View {
    /// Two ways to identify the contract the user wants to download:
    ///   - `.contractId` — paste the contract id directly.
    ///   - `.tokenId`    — paste a token id; the view resolves the
    ///     owning contract id (and token contract position) via
    ///     `sdk.getTokenContractInfo(tokenId:)` and then runs the
    ///     same contract-fetch path. Tokens are extracted by
    ///     `DataContractParser` as part of the contract ingest, so a
    ///     successful contract download persists the token too — no
    ///     separate "Add Token" model is needed.
    enum LoadMode: String, CaseIterable, Identifiable {
        case contractId = "Contract ID"
        case tokenId = "Token ID"
        var id: String { rawValue }
    }

    @EnvironmentObject var platformState: AppState
    @Environment(\.dismiss) var dismiss
    @Environment(\.modelContext) private var modelContext
    @Binding var isLoading: Bool

    @Query private var existingContracts: [PersistentDataContract]

    @State private var loadMode: LoadMode = .contractId
    /// Free-form input; semantics depend on `loadMode`. Either a
    /// base58 contract id (contractId mode) or a base58 token id
    /// (tokenId mode).
    @State private var inputId = ""
    @State private var contractName = ""
    @State private var errorMessage: String?
    @State private var showError = false
    @State private var fetchedContract: [String: Any]?
    @State private var showExampleContracts = false
    @State private var currentNetwork: String = "Unknown"

    /// In tokenId mode, the resolved owning contract id (base58)
    /// after `getTokenContractInfo`. Surfaced to the user before the
    /// contract download kicks off.
    @State private var resolvedContractId: String?
    /// In tokenId mode, the resolved token-contract-position (the
    /// slot inside the owning contract that holds this token).
    @State private var resolvedTokenPosition: Int?

    // Known testnet contracts - these are the common system contracts
    let exampleContracts = [
        ("DPNS Contract", "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"),
        ("DashPay Contract", "Bwr4WHCPz5rFVAD87RqTs3izo4zpzwsEdKPWUT1NS1C7"),
        ("Withdrawals Contract", "4fJLR2GYTPFdomuTVvNy3VRrvWgvkKPzqehEBpNf2nk6"),
        ("Wallet Utils", "7CSFGeF4WNzgDmx94zwvHkYaG3Dx4XEe5LFsFgJswLbm"),
        ("Token History", "43gujrzZgXqcKBiScLa4T8XTDnRhenR9BLx8GWVHjPxF"),
        ("Keyword Search", "BsjE6tQxG47wffZCRQCovFx5rYrAYYC3rTVRWKro27LA")
    ]

    var body: some View {
        NavigationView {
            Form {
                Section(footer: Text("Connected to: \(platformState.currentNetwork.displayName)")) {
                    Picker("Add by", selection: $loadMode) {
                        ForEach(LoadMode.allCases) { mode in
                            Text(mode.rawValue).tag(mode)
                        }
                    }
                    .pickerStyle(.segmented)
                    .disabled(isLoading)
                }

                Section(loadMode == .contractId ? "Contract Details" : "Token Details") {
                    HStack {
                        TextField(
                            loadMode == .contractId
                                ? "Contract ID (Base58)"
                                : "Token ID (Base58)",
                            text: $inputId
                        )
                        .textContentType(.none)
                        .autocapitalization(.none)
                        .disabled(isLoading)

                        // Examples list only makes sense for the
                        // contract-id flow — the seeded list is well-known
                        // contract ids, not token ids.
                        if loadMode == .contractId {
                            Button(action: { showExampleContracts.toggle() }) {
                                Image(systemName: "list.bullet")
                                    .foregroundColor(.blue)
                            }
                            .disabled(isLoading)
                        }
                    }

                    if loadMode == .contractId {
                        TextField("Name (Optional)", text: $contractName)
                            .textContentType(.none)
                            .disabled(isLoading)
                    }

                    if showExampleContracts && loadMode == .contractId {
                        Section(header: Text("Common System Contracts (\(platformState.currentNetwork.displayName))")) {
                            ForEach(exampleContracts, id: \.1) { example in
                                Button(action: {
                                    inputId = example.1
                                    contractName = example.0
                                    showExampleContracts = false
                                }) {
                                    HStack {
                                        VStack(alignment: .leading) {
                                            Text(example.0)
                                                .font(.subheadline)
                                            Text(example.1)
                                                .font(.caption)
                                                .foregroundColor(.secondary)
                                                .lineLimit(1)
                                                .truncationMode(.middle)
                                        }
                                        Spacer()
                                        Image(systemName: "arrow.right.circle")
                                            .foregroundColor(.secondary)
                                    }
                                }
                                .disabled(isLoading)
                            }
                        }
                    }
                }

                // In token-id mode, surface the resolved
                // contract id + token position before the contract
                // download fires. Lets the user verify they pasted
                // the right token id.
                if loadMode == .tokenId,
                   let resolvedContractId,
                   let resolvedTokenPosition {
                    Section("Resolved Token") {
                        HStack {
                            Text("Contract ID")
                                .foregroundColor(.secondary)
                            Spacer()
                            Text(resolvedContractId)
                                .font(.caption)
                                .lineLimit(1)
                                .truncationMode(.middle)
                        }
                        HStack {
                            Text("Token Position")
                                .foregroundColor(.secondary)
                            Spacer()
                            Text("\(resolvedTokenPosition)")
                                .font(.caption)
                        }
                    }
                }

                if isLoading {
                    Section {
                        HStack {
                            ProgressView()
                                .progressViewStyle(CircularProgressViewStyle())
                            Text(
                                loadMode == .tokenId && resolvedContractId == nil
                                    ? "Resolving token contract info..."
                                    : "Loading contract from network..."
                            )
                            .foregroundColor(.secondary)
                        }
                    }
                }

                if let contract = fetchedContract {
                    Section("Fetched Contract") {
                        if let id = contract["id"] as? String {
                            HStack {
                                Text("ID")
                                    .foregroundColor(.secondary)
                                Spacer()
                                Text(id)
                                    .font(.caption)
                                    .lineLimit(1)
                                    .truncationMode(.middle)
                            }
                        }

                        if let schema = contract["schema"] as? [String: Any],
                           let documentTypes = schema["documents"] as? [String: Any] {
                            HStack {
                                Text("Document Types")
                                    .foregroundColor(.secondary)
                                Spacer()
                                Text("\(documentTypes.count)")
                            }
                        }
                    }
                }
            }
            .navigationTitle(loadMode == .contractId ? "Add Contract" : "Add Token")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") {
                        dismiss()
                    }
                    .disabled(isLoading)
                }

                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Load") {
                        Task {
                            // Reset the resolved-token banner across
                            // attempts so a stale one doesn't carry
                            // over from the previous load.
                            await MainActor.run {
                                resolvedContractId = nil
                                resolvedTokenPosition = nil
                            }
                            await runLoadFlow()
                        }
                    }
                    .disabled(inputId.isEmpty || isLoading)
                }
            }
            .alert("Error", isPresented: $showError) {
                Button("OK") { }
            } message: {
                Text(errorMessage ?? "Unknown error occurred")
            }
        }
    }

    /// Top-level entry point for the Load button. Handles whichever
    /// of the two flows the user picked:
    ///   - `.contractId` → load the contract directly using whatever
    ///     they pasted as the contract id.
    ///   - `.tokenId`    → resolve the owning contract id via
    ///     `getTokenContractInfo`, then load that contract.
    private func runLoadFlow() async {
        switch loadMode {
        case .contractId:
            await loadContract(contractIdString: inputId)
        case .tokenId:
            await loadFromTokenId()
        }
    }

    /// Token-id flow: resolve the owning contract id (and remember
    /// the token contract position for display), then call into the
    /// shared contract-fetch path. The downloaded contract carries
    /// the token's full configuration as part of its `tokens` map,
    /// so persisting the contract auto-persists the token via
    /// `DataContractParser.parseDataContract`.
    private func loadFromTokenId() async {
        guard let sdk = platformState.sdk else {
            await MainActor.run {
                errorMessage = "SDK not initialized"
                showError = true
            }
            return
        }
        let trimmedTokenId = inputId.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedTokenId.isEmpty else { return }

        await MainActor.run { isLoading = true }

        do {
            let info = try await sdk.getTokenContractInfo(tokenId: trimmedTokenId)
            // Empty / missing contract_id is the "token not found"
            // case — surface a clear message rather than crashing on
            // a force-unwrap of a nil dictionary value.
            guard let resolvedId = info["contract_id"] as? String,
                  !resolvedId.isEmpty else {
                await MainActor.run {
                    errorMessage = "Token not found on \(platformState.currentNetwork.displayName). Double-check the Token ID."
                    showError = true
                    isLoading = false
                }
                return
            }
            // Token contract position can come back as Int or
            // NSNumber depending on JSONSerialization's number
            // bridging. Both round-trip through `intValue`.
            let position: Int
            if let intVal = info["token_contract_position"] as? Int {
                position = intVal
            } else if let numVal = info["token_contract_position"] as? NSNumber {
                position = numVal.intValue
            } else {
                position = 0
            }

            await MainActor.run {
                resolvedContractId = resolvedId
                resolvedTokenPosition = position
            }

            // Hand off to the shared contract-fetch path. It manages
            // `isLoading` itself.
            await loadContract(contractIdString: resolvedId)
        } catch {
            await MainActor.run {
                errorMessage = "Failed to resolve token: \(error.localizedDescription)"
                showError = true
                isLoading = false
            }
        }
    }

    /// Shared contract-fetch + persist path. The caller supplies the
    /// contract id string (either from the user's direct paste in
    /// `.contractId` mode or the `getTokenContractInfo`-resolved id
    /// in `.tokenId` mode). Sets `isLoading` to true on entry and
    /// flips it back to false on every exit path; downstream logic
    /// is identical regardless of how we obtained `contractIdString`.
    ///
    /// The actual fetch / parse / persist work lives in
    /// `ContractDownloader.downloadAndPersistContract` so the
    /// search-bar tap-to-add path in `ContractsTabView` can reuse the
    /// exact same pipeline. This wrapper is the modal-sheet-specific
    /// glue: input validation, `isLoading` + `errorMessage` UI state,
    /// the "already saved" toast, and `fetchedContract` preview.
    private func loadContract(contractIdString: String) async {
        guard let sdk = platformState.sdk else {
            // Match the empty-input branch below: bounce the error
            // mutations through `MainActor.run` so we don't write
            // to `@State` from a non-main actor on the SDK-nil
            // path either.
            await MainActor.run {
                errorMessage = "SDK not initialized"
                showError = true
            }
            return
        }

        let trimmedId = contractIdString.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedId.isEmpty else {
            await MainActor.run {
                errorMessage = "Please enter a contract ID"
                showError = true
                isLoading = false
            }
            return
        }

        await MainActor.run { isLoading = true }
        print("🔵 Attempting to load contract with ID: \(trimmedId)")

        do {
            let result = try await ContractDownloader.downloadAndPersistContract(
                contractId: trimmedId,
                suggestedName: contractName,
                sdk: sdk,
                modelContext: modelContext,
                network: platformState.currentNetwork,
                existingContracts: existingContracts
            )

            // Best-effort preview of the parsed contract for the
            // sheet's "Fetched Contract" section. The persistent row
            // already carries this data, but the existing UI binds to
            // the raw JSON dictionary.
            await MainActor.run {
                fetchedContract = (try? JSONSerialization.jsonObject(
                    with: result.contract.serializedContract
                )) as? [String: Any]
            }

            if result.alreadyExisted {
                await MainActor.run {
                    errorMessage = "This contract is already saved locally"
                    showError = true
                    isLoading = false
                }
                return
            }

            print("✅ Contract fetched and persisted successfully")
            await MainActor.run {
                isLoading = false
                dismiss()
            }
        } catch let downloadError as ContractDownloadError {
            await MainActor.run {
                switch downloadError {
                case .notFound:
                    errorMessage = "Contract not found on \(platformState.currentNetwork.displayName). This contract may exist on a different network or the ID may be incorrect."
                case .invalidContractId:
                    errorMessage = "Please enter a valid contract ID"
                case .fetchFailed(let msg):
                    errorMessage = "Failed to load contract: \(msg)"
                }
                showError = true
                isLoading = false
            }
        } catch {
            print("❌ Failed to load contract: \(error)")
            await MainActor.run {
                errorMessage = "Failed to load contract: \(error.localizedDescription)"
                showError = true
                isLoading = false
            }
        }
    }
}

#Preview {
    NavigationStack {
        LocalDataContractsView()
            .environmentObject(AppState())
    }
}
