import SwiftUI
import SwiftDashSDK
import SwiftDashSDK

struct IdentityDetailView: View {
    let identityId: Data
    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager

    private var identity: IdentityModel? {
        appState.identities.first { $0.id == identityId }
    }
    @State private var isRefreshing = false
    @State private var showingEditAlias = false
    @State private var newAlias = ""
    @State private var isLoadingDPNS = false
    @State private var showingRegisterName = false
    @State private var showingSelectMainName = false

    // DashPay profile state — read from the platform-wallet cache,
    // refreshed on appear via `syncDashPayProfiles()`.
    @State private var dashpayProfile: DashPayProfile?
    @State private var isLoadingProfile = false
    @State private var showingProfileEditor = false
    @State private var profileError: String?

    // Computed properties that get DPNS names from the identity model
    private var dpnsNames: [String] {
        identity?.dpnsNames ?? []
    }

    private var contestedDpnsNames: [String] {
        identity?.contestedDpnsNames ?? []
    }

    private var contestedDpnsInfo: [String: Any] {
        identity?.contestedDpnsInfo ?? [:]
    }

    var body: some View {
        if let identity = identity {
            List {
                // Basic Info Section
                Section("Identity Information") {
                    VStack(alignment: .leading, spacing: 8) {
                        if let alias = identity.alias {
                            Label(alias, systemImage: "person.text.rectangle")
                                .font(.headline)
                        }

                    // Show the main name if selected, otherwise show first registered name
                    if let mainName = identity.mainDpnsName {
                        HStack {
                            Label(mainName, systemImage: "star.fill")
                                .font(.subheadline)
                                .foregroundColor(.blue)
                            Spacer()
                            Text("Main")
                                .font(.caption)
                                .foregroundColor(.white)
                                .padding(.horizontal, 8)
                                .padding(.vertical, 2)
                                .background(Color.blue)
                                .cornerRadius(4)
                        }
                    } else if let dpnsName = identity.dpnsName {
                        Label(dpnsName, systemImage: "at")
                            .font(.subheadline)
                            .foregroundColor(.blue)
                    }

                    Label(identity.idHexString, systemImage: "number")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .padding(.vertical, 4)

                HStack {
                    Label("Balance", systemImage: "dollarsign.circle")
                    Spacer()
                    Text(identity.formattedBalance)
                        .foregroundColor(.blue)
                        .fontWeight(.medium)
                }

                HStack {
                    Label("Type", systemImage: "person.badge.shield.checkmark")
                    Spacer()
                    Text(identity.type.rawValue)
                        .foregroundColor(identity.type == .user ? .primary :
                                      identity.type == .masternode ? .purple : .orange)
                }

                if identity.isLocal {
                    HStack {
                        Label("Status", systemImage: "location")
                        Spacer()
                        Text("Local Only")
                            .foregroundColor(.secondary)
                    }
                }
            }

            // DPNS Names Section
            if !dpnsNames.isEmpty || !contestedDpnsNames.isEmpty || !identity.isLocal {
                Section("DPNS Names") {
                    if isLoadingDPNS {
                        HStack {
                            ProgressView()
                            Text("Loading DPNS names...")
                                .foregroundColor(.secondary)
                        }
                    } else if dpnsNames.isEmpty && contestedDpnsNames.isEmpty {
                        Text("No DPNS names found")
                            .foregroundColor(.secondary)
                    } else {
                        // Show registered names
                        ForEach(dpnsNames, id: \.self) { name in
                            HStack {
                                Text(name)
                                Spacer()
                                Image(systemName: "checkmark.circle.fill")
                                    .foregroundColor(.green)
                            }
                        }

                        // Show contested names
                        ForEach(contestedDpnsNames, id: \.self) { name in
                            NavigationLink(destination: ContestDetailView(
                                contestName: name,
                                contestInfo: contestedDpnsInfo[name] as? [String: Any] ?? [:],
                                currentIdentityId: identity.idString
                            ).environmentObject(appState)) {
                                HStack {
                                    Text(name)
                                    Spacer()
                                    Label("Contested", systemImage: "flag.fill")
                                        .font(.caption)
                                        .foregroundColor(.orange)
                                }
                            }
                        }
                    }

                    // Select main name button (only show if user has registered names)
                    if !dpnsNames.isEmpty {
                        Button(action: { showingSelectMainName = true }) {
                            HStack {
                                Image(systemName: "star.circle")
                                Text("Select Main Name")
                            }
                            .foregroundColor(.purple)
                        }
                    }

                    // Register name button
                    if !identity.isLocal {
                        Button(action: { showingRegisterName = true }) {
                            HStack {
                                Image(systemName: "plus.circle")
                                Text(dpnsNames.isEmpty ? "Register a name" : "Register another name")
                            }
                            .foregroundColor(.blue)
                        }
                    }
                }
            }

            // DashPay Profile Section
            //
            // Reads `ManagedIdentity.dashpay_profile` through the
            // platform-wallet FFI. Refreshes from Platform on appear
            // via `syncDashPayProfiles()` so the cache reflects the
            // latest on-chain state without blocking the first paint.
            Section("DashPay Profile") {
                if !identity.isLocal {
                    dashPayProfileCard(identity: identity)
                } else {
                    Text("Available once the identity is on the network.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }

            // Keys Section
            Section("Keys") {
                NavigationLink(destination: KeysListView(identity: identity)) {
                    VStack(alignment: .leading, spacing: 4) {
                        HStack {
                            Image(systemName: "key.fill")
                            Text("Identity Keys")
                                .fontWeight(.medium)
                        }

                        HStack(spacing: 16) {
                            Label("\(identity.publicKeys.count) public", systemImage: "key")
                                .font(.caption)
                                .foregroundColor(.secondary)

                            if !identity.privateKeys.isEmpty {
                                Label("\(identity.privateKeys.count) private", systemImage: "key.fill")
                                    .font(.caption)
                                    .foregroundColor(.green)
                            }
                        }
                    }
                    .padding(.vertical, 4)
                }
            }

            // Actions Section
            if !identity.isLocal {
                Section {
                    Button(action: refreshIdentityData) {
                        HStack {
                            Image(systemName: "arrow.clockwise")
                            Text("Refresh Identity Data")
                            Spacer()
                            if isRefreshing {
                                ProgressView()
                            }
                        }
                    }
                    .disabled(isRefreshing)
                }
            }
        }
        .navigationTitle("Identity Details")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            if identity.alias == nil {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Add Alias") {
                        newAlias = ""
                        showingEditAlias = true
                    }
                }
            }
        }
        .sheet(isPresented: $showingEditAlias) {
            EditAliasView(identity: identity, newAlias: $newAlias)
        }
        .sheet(isPresented: $showingRegisterName) {
            RegisterNameView(identity: identity)
                .environmentObject(appState)
        }
        .sheet(isPresented: $showingSelectMainName) {
            SelectMainNameView(identity: identity)
                .environmentObject(appState)
        }
        .sheet(isPresented: $showingProfileEditor) {
            DashPayProfileEditorView(
                identityId: identity.id,
                walletId: identity.walletId,
                existing: dashpayProfile,
                onSaved: { saved in
                    // Adopt the freshly-broadcast profile into the UI
                    // cache immediately — syncDashPayProfiles would
                    // pick it up on the next refresh, but this keeps
                    // the detail view in-sync without a round-trip.
                    dashpayProfile = saved
                }
            )
            .environmentObject(walletManager)
        }
        .onAppear {
            print("🔵 IdentityDetailView onAppear - dpnsName: \(identity.dpnsName ?? "nil"), isLocal: \(identity.isLocal)")

            // Load DPNS names from network if we don't have any cached or if they're empty
            if (dpnsNames.isEmpty && contestedDpnsNames.isEmpty) && !identity.isLocal {
                print("🔵 No cached DPNS names, loading from network...")
                loadDPNSNames()
            } else if !dpnsNames.isEmpty || !contestedDpnsNames.isEmpty {
                print("🔵 Using cached DPNS names: \(dpnsNames.count) regular, \(contestedDpnsNames.count) contested")
            }

            if !identity.isLocal {
                // Read whatever's currently cached synchronously so the
                // card renders immediately, then kick off a background
                // sync to freshen it. The sync uses the merged
                // `IdentityWallet` `sync_profiles` FFI path.
                loadCachedDashPayProfile(for: identity)
                Task { await refreshDashPayProfilesFromPlatform(for: identity) }
            }
        }
        } else {
            // No identity found view
            VStack(spacing: 20) {
                Spacer()

                Image(systemName: "person.crop.circle.badge.questionmark")
                    .font(.system(size: 60))
                    .foregroundColor(.gray)

                Text("No Identity Found")
                    .font(.title2)
                    .fontWeight(.semibold)

                Text("The identity could not be found.\nIt may have been deleted or doesn't exist.")
                    .multilineTextAlignment(.center)
                    .foregroundColor(.secondary)

                Spacer()
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .navigationTitle("Identity Details")
            .navigationBarTitleDisplayMode(.inline)
        }
    }

    private func refreshIdentityData() {
        Task {
            isRefreshing = true
            defer { isRefreshing = false }

            guard let sdk = appState.sdk,
                  let identity = identity else { return }

            do {
                // Refresh identity data
                let fetchedIdentity = try await sdk.identityGet(identityId: identity.idString)

                // Update balance
                if let balanceValue = fetchedIdentity["balance"] {
                    if let balanceNum = balanceValue as? NSNumber {
                        appState.updateIdentityBalance(id: identity.id, newBalance: balanceNum.uint64Value)
                    } else if let balanceString = balanceValue as? String,
                              let balanceUInt = UInt64(balanceString) {
                        appState.updateIdentityBalance(id: identity.id, newBalance: balanceUInt)
                    }
                }

                // Parse and update public keys
                var parsedPublicKeys: [IdentityPublicKey] = []
                print("🔵 Checking for public keys in fetched identity...")
                if let publicKeysArray = fetchedIdentity["publicKeys"] as? [[String: Any]] {
                    print("🔵 Found \(publicKeysArray.count) public keys")
                    parsedPublicKeys = publicKeysArray.compactMap { keyData -> IdentityPublicKey? in
                        print("🔵 Parsing key data: \(keyData)")
                        guard let id = keyData["id"] as? Int,
                              let purpose = keyData["purpose"] as? Int,
                              let securityLevel = keyData["securityLevel"] as? Int,
                              let keyType = keyData["type"] as? Int,
                              let dataStr = keyData["data"] as? String,
                              let data = Data(base64Encoded: dataStr) else {
                            return nil
                        }

                        let readOnly = keyData["readOnly"] as? Bool ?? false
                        let disabledAt = keyData["disabledAt"] as? UInt64

                        return IdentityPublicKey(
                            id: UInt32(id),
                            purpose: KeyPurpose(rawValue: UInt8(purpose)) ?? .authentication,
                            securityLevel: SecurityLevel(rawValue: UInt8(securityLevel)) ?? .high,
                            contractBounds: nil,
                            keyType: KeyType(rawValue: UInt8(keyType)) ?? .ecdsaSecp256k1,
                            readOnly: readOnly,
                            data: data,
                            disabledAt: disabledAt
                        )
                    }
                } else {
                    print("❌ No public keys found in fetched identity")
                }

                print("🔵 Parsed \(parsedPublicKeys.count) public keys total")

                // Update the identity with public keys
                appState.updateIdentityPublicKeys(id: identity.id, publicKeys: parsedPublicKeys)
                print("🔵 Called updateIdentityPublicKeys")

                // Refresh DPNS names from network
                await loadDPNSNamesFromNetwork()
            } catch {
                await MainActor.run {
                    appState.showError(message: "Failed to refresh identity: \(error.localizedDescription)")
                }
            }
        }
    }

    private func loadDPNSNames() {
        guard let identity = identity,
              !identity.isLocal else { return }

        Task {
            await loadDPNSNamesFromNetwork()
        }
    }

    private func loadDPNSNamesFromNetwork() async {
        guard let identity = identity,
              !identity.isLocal else { return }

        print("🔵 loadDPNSNamesFromNetwork called for identity \(identity.idString)")

        isLoadingDPNS = true
        defer { isLoadingDPNS = false }

        guard appState.sdk != nil else { return }

        // Fetch regular and contested names sequentially to avoid sending non-Sendable results across tasks
        let regular = await fetchRegularDPNSNames(identity: identity)
        let contested = await fetchContestedDPNSNames(identity: identity)

        await MainActor.run {
            let regularNames = regular.0
            let contestedNames = contested.0
            let contestedInfo = contested.1

            // Update all DPNS names in the identity model
            appState.updateIdentityDPNSNames(
                id: identity.id,
                dpnsNames: regularNames,
                contestedNames: contestedNames,
                contestedInfo: contestedInfo
            )

            print("🔵 Updated identity with \(regularNames.count) regular names and \(contestedNames.count) contested names")
        }
    }

    @MainActor
    private func fetchRegularDPNSNames(identity: IdentityModel) async -> ([String], [String: Any]) {
        guard let sdk = appState.sdk else { return ([], [:]) }

        do {
            print("🔵 Fetching regular DPNS names from network...")
            let usernames = try await sdk.dpnsGetUsername(
                identityId: identity.idString,
                limit: 10
            )

            print("🔵 Got \(usernames.count) regular DPNS names from network")
            return (usernames.compactMap { $0["label"] as? String }, [:])
        } catch {
            print("❌ No regular DPNS names found for identity: \(error)")
            return ([], [:])
        }
    }

    @MainActor
    private func fetchContestedDPNSNames(identity: IdentityModel) async -> ([String], [String: Any]) {
        guard let sdk = appState.sdk else { return ([], [:]) }

        do {
            print("🔵 Fetching contested DPNS names from network...")

            // Use the new dedicated FFI function for getting non-resolved contests for this identity
            let contestsResult = try await sdk.dpnsGetNonResolvedContestsForIdentity(
                identityId: identity.idString,
                limit: 20
            )

            var contestedNames: [String] = []
            var contestInfo: [String: Any] = [:]

            // Parse the result - it's a dictionary where keys are the contested names
            for (name, info) in contestsResult {
                contestedNames.append(name)
                contestInfo[name] = info
            }

            print("🔵 Found \(contestedNames.count) contested DPNS names")
            return (contestedNames, contestInfo)
        } catch {
            print("❌ Failed to fetch contested DPNS names: \(error)")
            return ([], [:])
        }
    }

    // MARK: - DashPay profile

    /// Card contents for the DashPay Profile section. Renders whichever
    /// of the three states applies: populated profile, empty placeholder,
    /// or "loading" spinner. Also surfaces the most recent profile error
    /// (sync or save) under a caption.
    @ViewBuilder
    private func dashPayProfileCard(identity: IdentityModel) -> some View {
        if let profile = dashpayProfile {
            VStack(alignment: .leading, spacing: 10) {
                HStack(alignment: .top, spacing: 12) {
                    Image(systemName: profile.avatarUrl != nil
                        ? "person.crop.circle.fill"
                        : "person.crop.circle")
                        .font(.title2)
                        .foregroundColor(.blue)
                        .frame(width: 28)
                    VStack(alignment: .leading, spacing: 4) {
                        Text(profile.displayName?.nilIfEmpty ?? "(no display name)")
                            .font(.subheadline)
                            .fontWeight(.medium)
                        if let msg = profile.publicMessage?.nilIfEmpty {
                            Text(msg)
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                        if let url = profile.avatarUrl?.nilIfEmpty {
                            Text(url)
                                .font(.caption2)
                                .foregroundColor(.secondary)
                                .lineLimit(1)
                                .truncationMode(.middle)
                        }
                    }
                }

                Button {
                    showingProfileEditor = true
                } label: {
                    HStack {
                        Image(systemName: "pencil")
                        Text("Edit Profile")
                    }
                }
            }
            .padding(.vertical, 4)
        } else if isLoadingProfile {
            HStack(spacing: 10) {
                ProgressView()
                Text("Loading DashPay profile…")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        } else {
            VStack(alignment: .leading, spacing: 10) {
                HStack(spacing: 12) {
                    Image(systemName: "person.crop.circle.dashed")
                        .font(.title2)
                        .foregroundColor(.secondary)
                        .frame(width: 28)
                    VStack(alignment: .leading, spacing: 2) {
                        Text("No DashPay profile yet")
                            .font(.subheadline)
                            .fontWeight(.medium)
                        Text("Add a display name and avatar so friends can find you on DashPay.")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }

                Button {
                    showingProfileEditor = true
                } label: {
                    HStack {
                        Image(systemName: "pencil")
                        Text("Set up profile")
                    }
                }
            }
            .padding(.vertical, 4)
        }

        if let err = profileError {
            Text(err)
                .font(.caption)
                .foregroundColor(.red)
        }
    }

    /// Synchronously read the cached DashPay profile from the Rust side
    /// without a network roundtrip. Fires on view appear so the card
    /// renders filled-in immediately when the app already sync'd this
    /// identity on a previous session. Silently no-ops when the
    /// identity doesn't have a wallet associated yet — that only
    /// happens for local-only identities, which we gate out upstream.
    private func loadCachedDashPayProfile(for identity: IdentityModel) {
        guard let walletId = identity.walletId,
              let wallet = walletManager.wallet(for: walletId) else {
            return
        }
        do {
            dashpayProfile = try wallet.getDashPayProfile(identityId: identity.id)
        } catch {
            print("⚠️ Failed to read cached DashPay profile: \(error)")
        }
    }

    /// Drive a Platform-side sync for every identity on the wallet,
    /// then re-read this identity's cache. Runs in a background task
    /// (the FFI dispatches to an 8 MB tokio worker internally).
    @MainActor
    private func refreshDashPayProfilesFromPlatform(for identity: IdentityModel) async {
        guard let walletId = identity.walletId,
              let wallet = walletManager.wallet(for: walletId) else {
            return
        }
        isLoadingProfile = dashpayProfile == nil
        defer { isLoadingProfile = false }

        do {
            _ = try await wallet.syncDashPayProfiles()
            // Pick up whatever the sync wrote back into the cache.
            dashpayProfile = try wallet.getDashPayProfile(identityId: identity.id)
            profileError = nil
        } catch {
            // Don't blow away a previously-shown profile on transient
            // sync failure — surface the error underneath instead.
            print("⚠️ DashPay profile sync failed: \(error)")
            profileError = "Profile sync failed: \(error.localizedDescription)"
        }
    }
}

// MARK: - String nil-if-empty helper

private extension String {
    /// Returns `nil` when the string is empty (after trimming), so
    /// caller sites don't render blank rows for explicitly-present
    /// but empty profile fields.
    var nilIfEmpty: String? {
        let trimmed = trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }
}

// MARK: - DashPay profile editor sheet

/// Modal editor for a DashPay profile. Fields correspond 1:1 with the
/// on-chain DashPay `profile` document; every field is optional.
/// `onSaved` fires with the post-broadcast cached profile so the
/// parent view can adopt it without re-fetching.
struct DashPayProfileEditorView: View {
    let identityId: Data
    /// Wallet that owns `identityId`. When `nil` the editor falls
    /// back to `walletManager.firstWallet` — acceptable for the
    /// current single-wallet UI but will need tightening once the
    /// app supports multiple concurrent wallets.
    let walletId: Data?
    let existing: DashPayProfile?
    let onSaved: (DashPayProfile) -> Void

    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.dismiss) var dismiss

    @State private var displayName: String = ""
    @State private var publicMessage: String = ""
    @State private var avatarUrl: String = ""
    @State private var isSaving = false
    @State private var errorMessage: String?

    private var isCreating: Bool { existing == nil }

    var body: some View {
        NavigationView {
            Form {
                Section("Display name") {
                    TextField("e.g. Alice", text: $displayName)
                        .textInputAutocapitalization(.words)
                }

                Section("Public message") {
                    TextField("A short bio that contacts can see", text: $publicMessage, axis: .vertical)
                        .lineLimit(3, reservesSpace: true)
                }

                Section("Avatar URL") {
                    TextField("https://…", text: $avatarUrl)
                        .keyboardType(.URL)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                    Text("Paste an HTTPS image URL. SHA-256 + dHash " +
                         "are computed client-side when you save — see " +
                         "DIP-15.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                if let err = errorMessage {
                    Section {
                        Text(err)
                            .foregroundColor(.red)
                            .font(.caption)
                    }
                }
            }
            .navigationTitle(isCreating ? "Set Up Profile" : "Edit Profile")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                        .disabled(isSaving)
                }
                ToolbarItem(placement: .navigationBarTrailing) {
                    if isSaving {
                        ProgressView()
                    } else {
                        Button(isCreating ? "Create" : "Save") { save() }
                    }
                }
            }
            .onAppear {
                // Pre-fill fields on edit; create mode starts blank.
                if let existing {
                    displayName = existing.displayName ?? ""
                    publicMessage = existing.publicMessage ?? ""
                    avatarUrl = existing.avatarUrl ?? ""
                }
            }
        }
    }

    /// Submit the create / update transition. Avatar byte download is
    /// not implemented yet — when the user supplies an `avatarUrl` the
    /// on-chain `avatarHash` + `avatarFingerprint` are left empty,
    /// matching what the Rust FFI does when `avatar_bytes` is null.
    /// A follow-up can fetch the URL, compute the hashes, and pass
    /// the raw bytes to the same entry point.
    private func save() {
        let cleanedDisplay = displayName.trimmingCharacters(in: .whitespacesAndNewlines)
        let cleanedMsg = publicMessage.trimmingCharacters(in: .whitespacesAndNewlines)
        let cleanedUrl = avatarUrl.trimmingCharacters(in: .whitespacesAndNewlines)

        let update = DashPayProfileUpdate(
            displayName: cleanedDisplay.isEmpty ? nil : cleanedDisplay,
            publicMessage: cleanedMsg.isEmpty ? nil : cleanedMsg,
            avatarUrl: cleanedUrl.isEmpty ? nil : cleanedUrl,
            avatarBytes: nil
        )

        isSaving = true
        errorMessage = nil

        Task { @MainActor in
            defer { isSaving = false }
            do {
                // Resolve the wallet via the identity's `walletId`;
                // fall back to `firstWallet` for legacy data rows
                // that predate walletId denormalization.
                let wallet = walletId.flatMap { walletManager.wallet(for: $0) }
                    ?? walletManager.firstWallet
                guard let wallet else {
                    errorMessage = "No wallet available for this identity"
                    return
                }
                let saved: DashPayProfile
                if isCreating {
                    saved = try await wallet.createDashPayProfile(
                        identityId: identityId,
                        update: update
                    )
                } else {
                    saved = try await wallet.updateDashPayProfile(
                        identityId: identityId,
                        update: update
                    )
                }
                onSaved(saved)
                dismiss()
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }
}

struct EditAliasView: View {
    let identity: IdentityModel
    @Binding var newAlias: String
    @EnvironmentObject var appState: AppState
    @Environment(\.dismiss) var dismiss

    var body: some View {
        NavigationView {
            Form {
                Section("Set Alias") {
                    TextField("Enter alias", text: $newAlias)
                        .textFieldStyle(RoundedBorderTextFieldStyle())
                }

                Section {
                    Text("An alias helps you identify this identity in the app. It's stored locally and not saved to the network.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            .navigationTitle("Add Alias")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") {
                        dismiss()
                    }
                }
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Save") {
                        saveAlias()
                    }
                    .disabled(newAlias.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                }
            }
        }
    }

    private func saveAlias() {
        let trimmedAlias = newAlias.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedAlias.isEmpty else { return }

        // Create updated identity with alias
        var updatedIdentity = identity
        updatedIdentity = IdentityModel(
            id: identity.id,
            balance: identity.balance,
            isLocal: identity.isLocal,
            alias: trimmedAlias,
            type: identity.type,
            privateKeys: identity.privateKeys,
            votingPrivateKey: identity.votingPrivateKey,
            ownerPrivateKey: identity.ownerPrivateKey,
            payoutPrivateKey: identity.payoutPrivateKey,
            dpnsName: identity.dpnsName,
            mainDpnsName: identity.mainDpnsName,
            dpnsNames: identity.dpnsNames,
            contestedDpnsNames: identity.contestedDpnsNames,
            contestedDpnsInfo: identity.contestedDpnsInfo,
            publicKeys: identity.publicKeys
        )

        // Update in app state
        appState.updateIdentity(updatedIdentity)

        dismiss()
    }
}
