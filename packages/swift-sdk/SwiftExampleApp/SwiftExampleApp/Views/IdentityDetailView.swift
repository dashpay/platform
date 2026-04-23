import SwiftUI
import SwiftData
import SwiftDashSDK

struct IdentityDetailView: View {
    let identityId: Data
    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext

    /// Reactively observe the `PersistentIdentity` row for
    /// `identityId`. `@Query` with a targeted predicate — when any
    /// write mutates the row (balance sync, DPNS name refresh,
    /// alias edit), SwiftUI re-renders this view automatically.
    @Query private var identities: [PersistentIdentity]

    init(identityId: Data) {
        self.identityId = identityId
        let target = identityId
        _identities = Query(
            filter: #Predicate<PersistentIdentity> { $0.identityId == target }
        )
    }

    private var identity: PersistentIdentity? {
        identities.first
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

    /// DPNS names owned by this identity, fetched from the owning
    /// wallet's `ManagedIdentity`. Empty until `loadDPNSNames` runs.
    @State private var dpnsNames: [String] = []
    /// Labels this identity is currently contending for.
    @State private var contestedDpnsNames: [String] = []
    /// Contest metadata keyed by name, surfaced to
    /// `ContestDetailView`. Opaque to this view; deliberately
    /// `[String: Any]` because the inspector decodes differently
    /// per vote poll.
    @State private var contestedDpnsInfo: [String: Any] = [:]

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

                    Label(identity.identityIdString, systemImage: "number")
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
                    Text(identity.identityType)
                        .foregroundColor(identity.identityTypeEnum == .user ? .primary :
                                      identity.identityTypeEnum == .masternode ? .purple : .orange)
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
                                currentIdentityId: identity.identityIdBase58
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
                            // Private-key count no longer tracked on
                            // IdentityModel; keys live in the
                            // Keychain, enumerate via
                            // `KeychainManager` if needed.
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
                identityId: identity.identityId,
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
                let fetchedIdentity = try await sdk.identityGet(identityId: identity.identityIdBase58)

                // Update balance
                if let balanceValue = fetchedIdentity["balance"] {
                    if let balanceNum = balanceValue as? NSNumber {
                        PersistentIdentity.updateBalance(
                            in: modelContext,
                            identityId: identity.identityId,
                            balance: balanceNum.uint64Value
                        )
                        try? modelContext.save()
                    } else if let balanceString = balanceValue as? String,
                              let balanceUInt = UInt64(balanceString) {
                        PersistentIdentity.updateBalance(
                            in: modelContext,
                            identityId: identity.identityId,
                            balance: balanceUInt
                        )
                        try? modelContext.save()
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

                // Replace the PersistentIdentity's public key rows
                // with the freshly-fetched set. Carries over the
                // keychain identifier for any public key we already
                // knew about so we don't lose track of the matching
                // private key after a refresh.
                let identifierByKeyId: [Int32: String] = Dictionary(
                    uniqueKeysWithValues: identity.publicKeys.compactMap { key in
                        guard let identifier = key.privateKeyKeychainIdentifier else { return nil }
                        return (key.keyId, identifier)
                    }
                )
                identity.publicKeys.removeAll()
                let identityHex = identity.identityIdBase58
                for publicKey in parsedPublicKeys {
                    guard let persistentKey = PersistentPublicKey.from(publicKey, identityId: identityHex) else {
                        continue
                    }
                    if let identifier = identifierByKeyId[persistentKey.keyId] {
                        persistentKey.privateKeyKeychainIdentifier = identifier
                    }
                    identity.addPublicKey(persistentKey)
                }
                try? modelContext.save()
                print("🔵 Persisted \(parsedPublicKeys.count) public keys for identity")

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

        print("🔵 loadDPNSNamesFromNetwork called for identity \(identity.identityIdBase58)")

        isLoadingDPNS = true
        defer { isLoadingDPNS = false }

        guard appState.sdk != nil else { return }

        // Fetch regular and contested names sequentially to avoid sending non-Sendable results across tasks
        let regular = await fetchRegularDPNSNames(identity: identity)
        let contested = await fetchContestedDPNSNames(identity: identity)

        await MainActor.run {
            // Drive the local @State fields directly — they are the
            // source of truth for this view's DPNS lists. The
            // previous `appState.updateIdentityDPNSNames(...)` call
            // wrote to the IdentityModel cache (which no longer
            // exists post-migration) and was not bound back to this
            // view's state, so nothing actually rendered from it.
            self.dpnsNames = regular.0
            self.contestedDpnsNames = contested.0
            self.contestedDpnsInfo = contested.1

            print("🔵 Updated view with \(dpnsNames.count) regular names and \(contestedDpnsNames.count) contested names")
        }
    }

    /// Fetch the identity's registered DPNS names.
    ///
    /// Prefers the platform-wallet cache path now that
    /// `IdentityWallet::sync_dpns_names` populates
    /// `ManagedIdentity.dpns_names` and routes the update through
    /// the identity changeset / persister callback. Falls back to
    /// the direct `sdk.dpnsGetUsername` RPC when the identity isn't
    /// attached to a loaded `ManagedPlatformWallet` (e.g. a legacy
    /// local-only row that predates walletId denormalization).
    @MainActor
    private func fetchRegularDPNSNames(identity: PersistentIdentity) async -> ([String], [String: Any]) {
        if let walletId = identity.walletId,
           let wallet = walletManager.wallet(for: walletId) {
            do {
                // Refresh Rust-side cache from Platform, then read
                // the cached labels back. Two-round-trip: one sync
                // (Platform RPC) + one cache read (sync, in-memory).
                // The Rust sync path only adds new labels — existing
                // cached entries are preserved, so the read-back is
                // a superset of whatever the cache had before.
                _ = try await wallet.syncDpnsNames(identityId: identity.identityId)
                let managed = try wallet.managedIdentity(identityId: identity.identityId)
                let names = try managed.getDpnsNames()
                print("🔵 Got \(names.count) regular DPNS names via platform-wallet cache")
                return (names, [:])
            } catch {
                // Log and fall through to the direct-SDK path so the
                // list isn't empty when the wallet path hits a
                // transient error.
                print("⚠️ Platform-wallet DPNS path failed, falling back to SDK: \(error)")
            }
        }

        // Legacy / fallback: direct SDK RPC. No local cache update.
        guard let sdk = appState.sdk else { return ([], [:]) }
        do {
            print("🔵 Fetching regular DPNS names via direct SDK RPC...")
            let usernames = try await sdk.dpnsGetUsername(
                identityId: identity.identityIdBase58,
                limit: 10
            )
            print("🔵 Got \(usernames.count) regular DPNS names from SDK")
            return (usernames.compactMap { $0["label"] as? String }, [:])
        } catch {
            print("❌ No regular DPNS names found for identity: \(error)")
            return ([], [:])
        }
    }

    /// Fetch the identity's contested DPNS names.
    ///
    /// Prefers the platform-wallet cache path: `syncContestedDpnsNames`
    /// pulls the canonical non-resolved set from Platform and writes
    /// a full snapshot to `ManagedIdentity.contested_dpns_names` (via
    /// the identity changeset → persister callback →
    /// `PersistentIdentity`), then we read the cached labels back.
    /// Resolved contests (won / locked) drop out automatically
    /// because the sync uses `set_contested_dpns_names` (full
    /// replace), not `add_contested_dpns_name`.
    ///
    /// Per-contest metadata (contenders, vote state, end time) is
    /// NOT cached — contest dynamics change throughout the voting
    /// period, so caching them would go stale fast. `ContestDetailView`
    /// still queries `sdk.dpnsGetNonResolvedContestsForIdentity`
    /// directly when it needs the full details. This view only
    /// uses the label list, so the cache is sufficient.
    ///
    /// Falls back to the direct-SDK RPC when the identity isn't
    /// attached to a loaded `ManagedPlatformWallet`.
    @MainActor
    private func fetchContestedDPNSNames(identity: PersistentIdentity) async -> ([String], [String: Any]) {
        if let walletId = identity.walletId,
           let wallet = walletManager.wallet(for: walletId) {
            do {
                _ = try await wallet.syncContestedDpnsNames(identityId: identity.identityId)
                let managed = try wallet.managedIdentity(identityId: identity.identityId)
                let names = try managed.getContestedDpnsNames()
                print("🔵 Got \(names.count) contested DPNS names via platform-wallet cache")
                // Metadata map intentionally empty — ContestDetailView
                // queries it fresh when opened.
                return (names, [:])
            } catch {
                print("⚠️ Platform-wallet contested DPNS path failed, falling back to SDK: \(error)")
            }
        }

        guard let sdk = appState.sdk else { return ([], [:]) }
        do {
            print("🔵 Fetching contested DPNS names via direct SDK RPC...")
            let contestsResult = try await sdk.dpnsGetNonResolvedContestsForIdentity(
                identityId: identity.identityIdBase58,
                limit: 20
            )
            var contestedNames: [String] = []
            var contestInfo: [String: Any] = [:]
            for (name, info) in contestsResult {
                contestedNames.append(name)
                contestInfo[name] = info
            }
            print("🔵 Found \(contestedNames.count) contested DPNS names from SDK")
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
    private func dashPayProfileCard(identity: PersistentIdentity) -> some View {
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
    private func loadCachedDashPayProfile(for identity: PersistentIdentity) {
        guard let walletId = identity.walletId,
              let wallet = walletManager.wallet(for: walletId) else {
            return
        }
        do {
            dashpayProfile = try wallet.getDashPayProfile(identityId: identity.identityId)
        } catch {
            print("⚠️ Failed to read cached DashPay profile: \(error)")
        }
    }

    /// Drive a Platform-side sync for every identity on the wallet,
    /// then re-read this identity's cache. Runs in a background task
    /// (the FFI dispatches to an 8 MB tokio worker internally).
    @MainActor
    private func refreshDashPayProfilesFromPlatform(for identity: PersistentIdentity) async {
        guard let walletId = identity.walletId,
              let wallet = walletManager.wallet(for: walletId) else {
            return
        }
        isLoadingProfile = dashpayProfile == nil
        defer { isLoadingProfile = false }

        do {
            _ = try await wallet.syncDashPayProfiles()
            // Pick up whatever the sync wrote back into the cache.
            dashpayProfile = try wallet.getDashPayProfile(identityId: identity.identityId)
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

    /// Submit the create / update transition.
    ///
    /// When the user enters an avatar URL, we fetch the image bytes
    /// before submitting so the Rust side can compute the DIP-15
    /// `avatarHash` (SHA-256) + `avatarFingerprint` (dHash 64-bit)
    /// and include them in the on-chain document. Without the
    /// accompanying bytes the URL would land on-chain but the two
    /// integrity fields would be empty, which violates DIP-15 (the
    /// contract requires them whenever `avatarUrl` is set).
    ///
    /// The fetch is skipped on update when the URL hasn't changed —
    /// Rust-side `update_profile` preserves the existing cached hash
    /// + fingerprint in that case, so there's no point re-downloading.
    /// On create the fetch always runs when a URL is present.
    private func save() {
        let cleanedDisplay = displayName.trimmingCharacters(in: .whitespacesAndNewlines)
        let cleanedMsg = publicMessage.trimmingCharacters(in: .whitespacesAndNewlines)
        let cleanedUrl = avatarUrl.trimmingCharacters(in: .whitespacesAndNewlines)

        // Did the user set/change the avatar URL? If so we need to
        // fetch bytes so Rust can compute the DIP-15 integrity hashes.
        // On update + same URL, we skip — Rust preserves the existing
        // cached hash/fingerprint.
        let urlChanged = cleanedUrl != (existing?.avatarUrl ?? "")
        let shouldFetchBytes = !cleanedUrl.isEmpty && (isCreating || urlChanged)

        isSaving = true
        errorMessage = nil

        Task { @MainActor in
            defer { isSaving = false }
            do {
                let avatarBytes: Data?
                if shouldFetchBytes {
                    avatarBytes = try await fetchAvatarBytes(urlString: cleanedUrl)
                } else {
                    avatarBytes = nil
                }

                let update = DashPayProfileUpdate(
                    displayName: cleanedDisplay.isEmpty ? nil : cleanedDisplay,
                    publicMessage: cleanedMsg.isEmpty ? nil : cleanedMsg,
                    avatarUrl: cleanedUrl.isEmpty ? nil : cleanedUrl,
                    avatarBytes: avatarBytes
                )

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

    /// Download the avatar image and return its raw bytes.
    ///
    /// Runs on the MainActor via `URLSession`. The returned bytes get
    /// handed straight to the Rust side, which computes SHA-256 +
    /// dHash and drops the bytes after embedding the hashes in the
    /// profile document (see `DashPayProfileUpdate.avatarBytes` and
    /// the Rust-side `calculate_avatar_hash` /
    /// `calculate_dhash_fingerprint` helpers).
    ///
    /// Validates up-front:
    /// - `urlString` parses as a URL (else `.avatarFetchFailed("invalid URL")`)
    /// - Response is HTTP 2xx (else `.avatarFetchFailed("status N")`)
    /// - Payload is at most [`avatarFetchMaxBytes`] so a hostile / huge
    ///   image can't OOM the app; Rust then double-checks by trying to
    ///   decode it as an image inside `calculate_dhash_fingerprint`.
    ///
    /// A 15-second timeout applies (see `avatarFetchTimeout`) to keep
    /// the save-in-flight UI responsive — the default `URLSession`
    /// timeout is 60s which feels broken inside an editor sheet.
    private func fetchAvatarBytes(urlString: String) async throws -> Data {
        guard let url = URL(string: urlString) else {
            throw DashPayProfileEditorError.avatarFetchFailed("Invalid URL: \(urlString)")
        }
        var request = URLRequest(url: url)
        request.timeoutInterval = Self.avatarFetchTimeout

        let (data, response): (Data, URLResponse)
        do {
            (data, response) = try await URLSession.shared.data(for: request)
        } catch {
            throw DashPayProfileEditorError.avatarFetchFailed(
                "Download failed: \(error.localizedDescription)"
            )
        }

        if let http = response as? HTTPURLResponse, !(200..<300).contains(http.statusCode) {
            throw DashPayProfileEditorError.avatarFetchFailed(
                "HTTP status \(http.statusCode) from \(url.host ?? urlString)"
            )
        }

        if data.count > Self.avatarFetchMaxBytes {
            throw DashPayProfileEditorError.avatarFetchFailed(
                "Image too large (\(data.count) bytes, max \(Self.avatarFetchMaxBytes))"
            )
        }

        return data
    }

    /// 15-second timeout for the avatar download. Tuned for a modal
    /// editor sheet — anything longer starts to feel broken while
    /// `isSaving` is true.
    private static let avatarFetchTimeout: TimeInterval = 15

    /// Max avatar byte count we'll accept from the network. 1 MiB
    /// covers typical PNG/JPEG avatars with headroom; larger files
    /// are almost certainly not a genuine avatar and risk OOM when
    /// ferried through the FFI or decoded by the `image` crate on
    /// the Rust side.
    private static let avatarFetchMaxBytes = 1 * 1024 * 1024
}

/// Typed errors emitted by `DashPayProfileEditorView`.
///
/// Currently only the avatar-fetch path throws; the FFI path throws
/// its own `PlatformWalletError` so we don't wrap those.
enum DashPayProfileEditorError: LocalizedError {
    case avatarFetchFailed(String)

    var errorDescription: String? {
        switch self {
        case .avatarFetchFailed(let detail):
            return "Avatar download failed: \(detail)"
        }
    }
}

struct EditAliasView: View {
    let identity: PersistentIdentity
    @Binding var newAlias: String
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) var dismiss

    var body: some View {
        NavigationStack {
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

        // Direct SwiftData write on the live `PersistentIdentity`
        // row. @Query upstream picks up the change reactively.
        identity.alias = trimmedAlias
        identity.lastUpdated = Date()
        try? modelContext.save()

        dismiss()
    }
}
