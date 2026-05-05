import SwiftUI
import SwiftDashSDK

struct OptionsView: View {
    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var platformBalanceSyncService: PlatformBalanceSyncService
    @EnvironmentObject var shieldedService: ShieldedService
    @State private var showingDataManagement = false
    @State private var showingAbout = false
    @State private var showingContracts = false
    @State private var isSwitchingNetwork = false
    @State private var sdkStatus: SDKStatus?
    @State private var isLoadingStatus = false

    /// Live-edit copy of the faucet RPC password. Round-trips through
    /// `UserDefaults` on every `.onChange` so other surfaces
    /// (`ReceiveAddressView.requestFromFaucet`) keep reading the
    /// authoritative value, while a local `@State` lets the
    /// debounced `.task(id:)` validator see edits immediately
    /// without a fetch round-trip per keystroke.
    @State private var faucetPassword: String =
        UserDefaults.standard.string(forKey: "faucetRPCPassword") ?? ""
    @State private var faucetValidation: FaucetValidationStatus = .idle

    /// Driven by the 0.5s-debounced `.task(id: faucetPassword)`.
    /// Runs a single cheap `getblockcount` JSON-RPC against the
    /// dashmate-managed Core (`127.0.0.1:<faucetRPCPort>`) and
    /// classifies the response so the UI can render an inline
    /// state line under the password field.
    private enum FaucetValidationStatus: Equatable {
        case idle
        case checking
        case valid
        case invalid
        case unreachable(String)

        var label: String? {
            switch self {
            case .idle:                return nil
            case .checking:            return "Checking…"
            case .valid:               return "Authorized"
            case .invalid:             return "Wrong password"
            case .unreachable(let r):  return "Unreachable (\(r))"
            }
        }

        var systemImage: String? {
            switch self {
            case .idle:        return nil
            case .checking:    return "ellipsis.circle"
            case .valid:       return "checkmark.circle.fill"
            case .invalid:     return "xmark.circle.fill"
            case .unreachable: return "exclamationmark.triangle.fill"
            }
        }

        var color: Color {
            switch self {
            case .idle, .checking: return .secondary
            case .valid:           return .green
            case .invalid:         return .red
            case .unreachable:     return .orange
            }
        }
    }

    // Bind the SPV peer-override settings directly to the same
    // UserDefaults keys that CoreContentView reads when starting SPV
    // (`useLocalhostCore`, `localCorePeers`). This re-exposes the
    // pre-rewrite "connect SPV to a local rust-dashcore" capability on
    // testnet/mainnet/devnet — regtest still uses the Docker toggle
    // above, which sets these same keys via AppState.useDockerSetup.
    @AppStorage("useLocalhostCore") private var customSpvPeersEnabled: Bool = false
    @AppStorage("localCorePeers") private var customSpvPeers: String = ""

    /// Default localhost peer string for a given network. Used to
    /// pre-populate the peers text field when the user enables the
    /// custom-SPV toggle. The FFI drops bare-IP entries (no port),
    /// so the default must include the network's standard P2P port.
    private func defaultSpvPeers(for network: Network) -> String {
        switch network {
        case .mainnet: return "127.0.0.1:9999"
        case .testnet: return "127.0.0.1:19999"
        case .devnet:  return "127.0.0.1:29999"
        case .regtest: return "127.0.0.1:19899"
        }
    }

    var body: some View {
        // `NavigationStack` (iOS 16+) instead of the deprecated
        // `NavigationView`. With `NavigationView`, `NavigationLink`s
        // embedded in a `List` (e.g. under Settings → Storage /
        // Keychain Explorer, whose detail views themselves list
        // NavigationLinks) push-then-immediately-pop on iOS 16+ —
        // visible to users as "I tap a row and it bounces me out."
        // `NavigationStack` has reliable push semantics.
        NavigationStack {
            Form {
                Section("Network") {
                    Picker("Current Network", selection: Binding(
                        get: { appState.currentNetwork },
                        set: { newNetwork in
                            if newNetwork != appState.currentNetwork {
                                isSwitchingNetwork = true
                                Task {
                                    // Auto-disable Docker when leaving Local
                                    if newNetwork != .regtest && appState.useDockerSetup {
                                        appState.useDockerSetup = false
                                    }

                                    // Update platform state (which will trigger SDK switch)
                                    appState.currentNetwork = newNetwork

                                    // Reset per-network services. TODO(platform-wallet):
                                    // Once PlatformWalletManager supports network
                                    // switching cleanly, call into it here.
                                    try? walletManager.stopSpv()
                                    platformBalanceSyncService.reset()
                                    shieldedService.reset()

                                    await MainActor.run {
                                        isSwitchingNetwork = false
                                    }
                                }
                            }
                        }
                    )) {
                        ForEach(Network.allCases, id: \.self) { network in
                            Text(network.displayName).tag(network)
                        }
                    }
                    .pickerStyle(SegmentedPickerStyle())
                    .disabled(isSwitchingNetwork)

                    if appState.currentNetwork == .regtest {
                        Toggle("Use Docker Setup", isOn: $appState.useDockerSetup)
                            .onChange(of: appState.useDockerSetup) { _, _ in
                                isSwitchingNetwork = true
                                Task {
                                    await appState.switchNetwork(to: appState.currentNetwork)
                                    await MainActor.run { isSwitchingNetwork = false }
                                }
                            }
                            .help("Connect to local dashmate Docker network.")

                        if appState.useDockerSetup {
                            TextField("Faucet RPC Password", text: $faucetPassword)
                                .font(.system(.body, design: .monospaced))
                                .textInputAutocapitalization(.never)
                                .autocorrectionDisabled()
                                .onChange(of: faucetPassword) { _, newValue in
                                    UserDefaults.standard.set(newValue, forKey: "faucetRPCPassword")
                                }
                                // Debounce keystrokes via `.task(id:)`: every
                                // edit cancels the prior task (the sleep
                                // throws `CancellationError`), so validation
                                // only fires when the user pauses for 0.5s.
                                .task(id: faucetPassword) {
                                    do {
                                        try await Task.sleep(nanoseconds: 500_000_000)
                                    } catch {
                                        return
                                    }
                                    await validateFaucetPassword(faucetPassword)
                                }

                            if let label = faucetValidation.label,
                               let icon = faucetValidation.systemImage {
                                HStack(spacing: 6) {
                                    if faucetValidation == .checking {
                                        ProgressView()
                                            .scaleEffect(0.7)
                                    } else {
                                        Image(systemName: icon)
                                            .foregroundColor(faucetValidation.color)
                                    }
                                    Text(label)
                                        .font(.caption)
                                        .foregroundColor(faucetValidation.color)
                                    Spacer()
                                }
                            }
                        }
                    } else {
                        Toggle("Use Custom SPV Peers", isOn: $customSpvPeersEnabled)
                            .onChange(of: customSpvPeersEnabled) { _, isOn in
                                // When enabling, seed the peers field with
                                // the network's localhost default if the
                                // user hasn't entered anything (or has the
                                // legacy port-less "127.0.0.1" the FFI
                                // would otherwise drop).
                                if isOn && (customSpvPeers.isEmpty || !customSpvPeers.contains(":")) {
                                    customSpvPeers = defaultSpvPeers(for: appState.currentNetwork)
                                }
                                // Stop SPV so the next start picks up the
                                // new peer config in CoreContentView.
                                try? walletManager.stopSpv()
                            }
                            .help("Connect Core SPV to specific peers (e.g. a local rust-dashcore) instead of the public seed nodes. Restart SPV from the Wallet tab to apply.")

                        if customSpvPeersEnabled {
                            TextField(defaultSpvPeers(for: appState.currentNetwork), text: $customSpvPeers)
                                .font(.system(.body, design: .monospaced))
                                .textInputAutocapitalization(.never)
                                .autocorrectionDisabled()
                        }
                    }

                    HStack {
                        Text("Network Status")
                        Spacer()
                        if isSwitchingNetwork {
                            HStack(spacing: 4) {
                                ProgressView()
                                    .scaleEffect(0.8)
                                Text("Switching...")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                        } else if appState.sdk != nil {
                            Label("Connected", systemImage: "checkmark.circle.fill")
                                .font(.caption)
                                .foregroundColor(.green)
                        } else {
                            Label("Disconnected", systemImage: "xmark.circle.fill")
                                .font(.caption)
                                .foregroundColor(.red)
                        }
                    }

                }

                Section("Data") {
                    NavigationLink(destination: StorageExplorerView()) {
                        Label("Storage Explorer", systemImage: "cylinder.split.1x2")
                    }

                    NavigationLink(destination: KeychainExplorerView()) {
                        Label("Keychain Explorer", systemImage: "key.viewfinder")
                    }

                    NavigationLink(destination: WalletMemoryExplorerView()) {
                        Label("Wallet Memory Explorer", systemImage: "memorychip")
                    }

                    NavigationLink(destination: ContractsView()) {
                        Label("Browse Contracts", systemImage: "doc.plaintext")
                    }

                    Button(action: { showingDataManagement = true }) {
                        Label("Manage Local Data", systemImage: "internaldrive")
                    }

                    if let stats = appState.dataStatistics {
                        VStack(alignment: .leading, spacing: 8) {
                            Text("Storage Statistics")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            HStack {
                                Text("Identities:")
                                Spacer()
                                Text("\(stats.identities)")
                            }
                            .font(.caption)
                            HStack {
                                Text("Documents:")
                                Spacer()
                                Text("\(stats.documents)")
                            }
                            .font(.caption)
                            HStack {
                                Text("Contracts:")
                                Spacer()
                                Text("\(stats.contracts)")
                            }
                            .font(.caption)
                            HStack {
                                Text("Token Balances:")
                                Spacer()
                                Text("\(stats.tokenBalances)")
                            }
                            .font(.caption)
                        }
                        .padding(.vertical, 4)
                    }
                }

                Section(header: Text("Platform")) {
                    NavigationLink(destination: PlatformQueriesView()) {
                        Label("Queries", systemImage: "magnifyingglass")
                    }

                    NavigationLink(destination: PlatformStateTransitionsView()) {
                        Label("State Transitions", systemImage: "arrow.up.arrow.down")
                    }

                    HStack {
                        Text("SDK Initialized")
                        Spacer()
                        Image(systemName: appState.sdk != nil ? "checkmark.circle.fill" : "xmark.circle.fill")
                            .foregroundColor(appState.sdk != nil ? .green : .red)
                    }

                    if let status = sdkStatus {
                        HStack {
                            Text("Version")
                            Spacer()
                            Text(status.version)
                                .foregroundColor(.secondary)
                        }

                        HStack {
                            Text("Network")
                            Spacer()
                            Text(status.network.capitalized)
                                .foregroundColor(.secondary)
                        }

                        HStack {
                            Text("Mode")
                            Spacer()
                            Text(status.mode.uppercased())
                                .foregroundColor(status.mode == "trusted" ? .blue : .orange)
                        }

                        HStack {
                            Text("Quorums in Memory")
                            Spacer()
                            Text("\(status.quorumCount)")
                                .foregroundColor(status.quorumCount > 0 ? .green : .red)
                        }
                    }

                    Button(action: loadSDKStatus) {
                        HStack {
                            Label("Refresh SDK Status", systemImage: "arrow.clockwise")
                            Spacer()
                            if isLoadingStatus {
                                ProgressView()
                                    .scaleEffect(0.8)
                            }
                        }
                    }
                    .disabled(isLoadingStatus)
                }

                Section("About") {
                    Button(action: { showingAbout = true }) {
                        HStack {
                            Text("About Dash SDK Example")
                            Spacer()
                            Image(systemName: "chevron.right")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }

                    HStack {
                        Text("SDK Version")
                        Spacer()
                        Text("1.0.0")
                            .foregroundColor(.secondary)
                    }

                    HStack {
                        Text("App Version")
                        Spacer()
                        Text("1.0.0")
                            .foregroundColor(.secondary)
                    }
                }
            }
            .navigationTitle("Options")
            .task {
                await loadDataStatistics()
                loadSDKStatus()
            }
            .sheet(isPresented: $showingDataManagement) {
                DataManagementView()
                    .environmentObject(appState)
            }
            .sheet(isPresented: $showingAbout) {
                AboutView()
            }
        }
    }

    private func loadDataStatistics() async {
        if let stats = await appState.getDataStatistics() {
            await MainActor.run {
                appState.dataStatistics = stats
            }
        }
    }

    /// Validate `password` against the dashmate-managed Core RPC.
    ///
    /// Issues a single `getblockcount` JSON-RPC call to
    /// `http://127.0.0.1:<faucetRPCPort>/` with HTTP basic auth and
    /// classifies the response into the `FaucetValidationStatus`
    /// the UI renders. Empty password short-circuits to `.idle` so
    /// the inline status row stays hidden until the user types.
    ///
    /// Called from the password field's `.task(id: faucetPassword)`,
    /// which already provides the 0.5s debounce — repeat keystrokes
    /// auto-cancel the in-flight task before it gets here.
    @MainActor
    private func validateFaucetPassword(_ password: String) async {
        guard !password.isEmpty else {
            faucetValidation = .idle
            return
        }
        faucetValidation = .checking

        let rpcPort = UserDefaults.standard.string(forKey: "faucetRPCPort") ?? "20302"
        let rpcUser = UserDefaults.standard.string(forKey: "faucetRPCUser") ?? "dashmate"

        guard let url = URL(string: "http://127.0.0.1:\(rpcPort)/") else {
            faucetValidation = .unreachable("invalid URL")
            return
        }

        let body: [String: Any] = [
            "jsonrpc": "1.0",
            "id": "validate",
            "method": "getblockcount",
            "params": []
        ]
        guard let jsonData = try? JSONSerialization.data(withJSONObject: body) else {
            faucetValidation = .unreachable("encode failed")
            return
        }

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.httpBody = jsonData
        request.setValue("text/plain", forHTTPHeaderField: "Content-Type")
        // Short timeout — Docker not running surfaces as a fast
        // failure rather than a hung "Checking…" spinner.
        request.timeoutInterval = 3

        let credentials = "\(rpcUser):\(password)"
        if let credData = credentials.data(using: .utf8) {
            request.setValue(
                "Basic \(credData.base64EncodedString())",
                forHTTPHeaderField: "Authorization"
            )
        }

        do {
            let (_, response) = try await URLSession.shared.data(for: request)
            // Bail if the user kept typing while we were waiting on
            // the network — the next `.task(id:)` invocation will
            // re-classify with the fresh value.
            guard !Task.isCancelled else { return }
            guard let http = response as? HTTPURLResponse else {
                faucetValidation = .unreachable("invalid response")
                return
            }
            switch http.statusCode {
            case 200:
                faucetValidation = .valid
            case 401, 403:
                faucetValidation = .invalid
            default:
                faucetValidation = .unreachable("HTTP \(http.statusCode)")
            }
        } catch is CancellationError {
            // Superseded by a fresher keystroke; the new task will
            // produce the next status.
            return
        } catch {
            faucetValidation = .unreachable(error.localizedDescription)
        }
    }

    /// Fetch the SDK version / network / mode / quorum count for
    /// display in the Platform section. Called once on appear and
    /// on demand via the refresh button.
    private func loadSDKStatus() {
        guard let sdk = appState.sdk else { return }

        isLoadingStatus = true

        Task {
            do {
                let status: SwiftDashSDK.SDKStatus = try sdk.getStatus()
                await MainActor.run {
                    self.sdkStatus = status
                    self.isLoadingStatus = false
                }
            } catch {
                print("Failed to get SDK status: \(error)")
                await MainActor.run {
                    self.isLoadingStatus = false
                }
            }
        }
    }
}

struct DataManagementView: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.dismiss) var dismiss
    @State private var showingClearConfirmation = false

    var body: some View {
        NavigationStack {
            Form {
                Section("Clear Data by Type") {
                    Button(role: .destructive, action: {
                        // Clear identities
                    }) {
                        Label("Clear All Identities", systemImage: "person.crop.circle.badge.xmark")
                    }

                    Button(role: .destructive, action: {
                        // Clear documents
                    }) {
                        Label("Clear All Documents", systemImage: "doc.badge.xmark")
                    }

                    Button(role: .destructive, action: {
                        // Clear contracts
                    }) {
                        Label("Clear All Contracts", systemImage: "doc.plaintext.badge.xmark")
                    }
                }

                Section("Clear All Data") {
                    Button(role: .destructive, action: {
                        showingClearConfirmation = true
                    }) {
                        Label("Clear All Data", systemImage: "trash")
                            .foregroundColor(.red)
                    }
                }

                Section {
                    Text("Warning: Clearing data will remove all locally stored information for the current network. This action cannot be undone.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            .navigationTitle("Manage Data")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") {
                        dismiss()
                    }
                }
            }
            .alert("Clear All Data?", isPresented: $showingClearConfirmation) {
                Button("Cancel", role: .cancel) { }
                Button("Clear", role: .destructive) {
                    // Implement clear all data
                }
            } message: {
                Text("This will permanently delete all data for the \(appState.currentNetwork.displayName) network. This action cannot be undone.")
            }
        }
    }
}

struct AboutView: View {
    @Environment(\.dismiss) var dismiss

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(spacing: 20) {
                    Image(systemName: "app.fill")
                        .font(.system(size: 80))
                        .foregroundColor(.blue)

                    Text("Dash SDK Example")
                        .font(.title)
                        .fontWeight(.bold)

                    Text("A demonstration app showcasing the capabilities of the Dash Platform SDK for iOS.")
                        .multilineTextAlignment(.center)
                        .padding(.horizontal)

                    VStack(alignment: .leading, spacing: 16) {
                        FeatureRow(
                            icon: "person.3.fill",
                            title: "Identity Management",
                            description: "Create and manage Dash Platform identities"
                        )

                        FeatureRow(
                            icon: "doc.text.fill",
                            title: "Document Storage",
                            description: "Store and retrieve documents on the platform"
                        )

                        FeatureRow(
                            icon: "dollarsign.circle.fill",
                            title: "Token Support",
                            description: "Manage tokens and token balances"
                        )

                        FeatureRow(
                            icon: "network",
                            title: "Multi-Network",
                            description: "Switch between mainnet, testnet, and devnet"
                        )
                    }
                    .padding()

                    Link("Learn More", destination: URL(string: "https://www.dash.org/platform/")!)
                        .buttonStyle(.borderedProminent)
                }
                .padding()
            }
            .navigationTitle("About")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") {
                        dismiss()
                    }
                }
            }
        }
    }
}

struct FeatureRow: View {
    let icon: String
    let title: String
    let description: String

    var body: some View {
        HStack(alignment: .top, spacing: 16) {
            Image(systemName: icon)
                .font(.title2)
                .foregroundColor(.blue)
                .frame(width: 40)

            VStack(alignment: .leading, spacing: 4) {
                Text(title)
                    .font(.headline)
                Text(description)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            Spacer()
        }
    }
}
