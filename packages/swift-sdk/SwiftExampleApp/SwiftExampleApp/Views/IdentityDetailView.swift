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

    /// Reactively observe the confirmed DPNS labels owned by this
    /// identity. Filters by the denormalized `identityId` column on
    /// `PersistentDPNSName` (not the optional relationship traversal
    /// `identity?.identityId`, which SwiftData's predicate engine
    /// chokes on for nullable relationships). Newest acquisition first
    /// — the `acquiredAt` Unix-millis timestamp is `0` when unknown,
    /// so legacy / un-timestamped rows naturally sort to the bottom.
    @Query private var dpnsNamesRows: [PersistentDPNSName]

    init(identityId: Data) {
        self.identityId = identityId
        let target = identityId
        _identities = Query(
            filter: #Predicate<PersistentIdentity> { $0.identityId == target }
        )
        _dpnsNamesRows = Query(
            filter: PersistentDPNSName.predicate(identityId: target),
            sort: [SortDescriptor(\PersistentDPNSName.acquiredAt, order: .reverse)]
        )
    }

    private var identity: PersistentIdentity? {
        identities.first
    }

    private func hasLoadedWallet(for identity: PersistentIdentity) -> Bool {
        guard let walletId = identity.wallet?.walletId else { return false }
        return walletManager.wallet(for: walletId) != nil
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

    /// Bare-label projection of `dpnsNamesRows`. The list views in
    /// this file deal in `[String]`, so this keeps the existing
    /// rendering code shape after we switched the source of truth
    /// from a plain `@State` array to a SwiftData `@Query`.
    private var dpnsNames: [String] {
        dpnsNamesRows.map(\.label)
    }
    /// Labels this identity is currently contending for.
    @State private var contestedDpnsNames: [String] = []
    /// Contest metadata keyed by name, surfaced to
    /// `ContestDetailView`. Opaque to this view; deliberately
    /// `[String: Any]` because the inspector decodes differently
    /// per vote poll.
    @State private var contestedDpnsInfo: [String: Any] = [:]

    /// Tokens this identity holds, paired with their balance + the
    /// originating PersistentToken for display metadata. Populated on
    /// appear and on tap of the section's refresh button. Empty during
    /// the first load and when the identity holds no balances.
    @State private var tokenBalances: [IdentityTokenEntry] = []
    @State private var isLoadingTokens = false
    @State private var tokensError: String?

    /// Drives presentation of `TopUpIdentityView` via the
    /// `.sheet(isPresented:)` modifier below. Tapped from the
    /// "Top Up Balance" button under the Balance row — the flow
    /// itself owns wallet / account / amount selection.
    @State private var showingTopUp = false

    /// Drives presentation of `TransferCreditsView`. Tapped from the
    /// "Transfer Credits" button next to "Top Up Balance" — the flow
    /// owns recipient + amount selection and signs via the Keychain.
    @State private var showingTransferCredits = false

    /// Drives presentation of `WithdrawCreditsView`. Tapped from the
    /// "Withdraw Credits" button below "Transfer Credits" — the flow
    /// owns destination-address + amount entry and signs via the
    /// Keychain. The L1 payout is processed asynchronously by the
    /// network.
    @State private var showingWithdrawCredits = false

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

                    // Hex row: raw byte form most users see when an
                    // identity id surfaces in logs / errors.
                    IdentityIDCopyRow(
                        icon: "number",
                        value: identity.identityIdString,
                        accessibilityLabel: "Copy identity ID (hex)"
                    )
                    // Base58 row. Most platform tooling (contract JSON
                    // group members, FFI args, dashmate) consumes the
                    // base58 form, so it deserves equal visibility next
                    // to hex rather than being hidden behind a menu.
                    IdentityIDCopyRow(
                        icon: "textformat.abc",
                        value: identity.identityIdBase58,
                        accessibilityLabel: "Copy identity ID (base58)"
                    )
                }
                .padding(.vertical, 4)

                HStack {
                    Label("Balance", systemImage: "dollarsign.circle")
                    Spacer()
                    Text(identity.formattedBalance)
                        .foregroundColor(.blue)
                        .fontWeight(.medium)
                }

                // Top-up entry point. Hidden for identities whose
                // owning wallet isn't loaded into the manager — that
                // path would just surface a confusing error from the
                // FFI layer.
                if let walletId = identity.wallet?.walletId,
                   walletManager.wallet(for: walletId) != nil {
                    Button {
                        showingTopUp = true
                    } label: {
                        HStack {
                            Label("Top Up Balance", systemImage: "plus.circle")
                            Spacer()
                            Image(systemName: "chevron.right")
                                .foregroundColor(.secondary)
                                .font(.caption)
                        }
                    }
                    .buttonStyle(.plain)

                    // Credit-to-credit transfer to another identity.
                    // Same gating as Top Up: on-chain identity backed
                    // by a loaded wallet so the signer can derive the
                    // state-transition key.
                    Button {
                        showingTransferCredits = true
                    } label: {
                        HStack {
                            Label("Transfer Credits", systemImage: "arrow.left.arrow.right.circle")
                            Spacer()
                            Image(systemName: "chevron.right")
                                .foregroundColor(.secondary)
                                .font(.caption)
                        }
                    }
                    .buttonStyle(.plain)

                    // Withdraw credits to an L1 Dash address. Same
                    // gating as Top Up / Transfer: on-chain identity
                    // backed by a loaded wallet so the signer can derive
                    // the state-transition key. The L1 payout is
                    // processed asynchronously by the network.
                    Button {
                        showingWithdrawCredits = true
                    } label: {
                        HStack {
                            Label("Withdraw Credits", systemImage: "arrow.up.circle")
                            Spacer()
                            Image(systemName: "chevron.right")
                                .foregroundColor(.secondary)
                                .font(.caption)
                        }
                    }
                    .buttonStyle(.plain)
                }

                HStack {
                    Label("Type", systemImage: "person.badge.shield.checkmark")
                    Spacer()
                    Text(identity.identityType)
                        .foregroundColor(identity.identityTypeEnum == .user ? .primary :
                                      identity.identityTypeEnum == .masternode ? .purple : .orange)
                }

                // Access row — local (signable: wallet-owned or
                // imported keys) vs observed read-only. The separate
                // "No Wallet" badge and wallet rows distinguish the
                // walletless-local case.
                HStack {
                    Label(
                        "Access",
                        systemImage: identity.isLocal ? "key.fill" : "eye"
                    )
                    Spacer()
                    Text(identity.isLocal ? "Local" : "Observed")
                        .foregroundColor(.secondary)
                }
            }

            // DPNS Names Section — every persisted identity exists on
            // Platform, so the section always renders.
            Section("DPNS Names") {
                    if hasLoadedWallet(for: identity) {
                        NavigationLink(destination: DpnsMarketplaceView(identity: identity)) {
                            Label("Username Marketplace", systemImage: "storefront")
                        }
                        .accessibilityIdentifier("identity.dpnsMarketplace")
                    }

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

                    // Register name button — `RegisterNameView`
                    // resolves `identity.wallet` plus a loaded
                    // Platform wallet immediately, so gate on exactly
                    // that (NOT `isLocal`, which also covers
                    // walletless imported-key identities the
                    // registration flow can't serve yet).
                    if hasLoadedWallet(for: identity) {
                        Button(action: { showingRegisterName = true }) {
                            HStack {
                                Image(systemName: "plus.circle")
                                Text(dpnsNames.isEmpty ? "Register a name" : "Register another name")
                            }
                            .foregroundColor(.blue)
                        }
                    }
            }

            // Tokens Section
            //
            // Lists every PersistentToken the identity actually holds
            // a non-zero balance against. Token-id derivation happens
            // via the FFI `dash_sdk_calculate_token_id` (the protocol
            // formula must NOT be mirrored in Swift), then we batch
            // the resulting ids into one `getIdentityTokenBalances`
            // round-trip. Transient @State only — persistence to
            // PersistentTokenBalance lives in the platform-wallet
            // sync path, not here.
            Section {
                    if isLoadingTokens && tokenBalances.isEmpty {
                        HStack(spacing: 10) {
                            ProgressView()
                            Text("Loading token balances…")
                                .foregroundColor(.secondary)
                        }
                    } else if let err = tokensError {
                        Text(err)
                            .font(.caption)
                            .foregroundColor(.red)
                    } else if tokenBalances.isEmpty {
                        Text("No tokens")
                            .foregroundColor(.secondary)
                    } else {
                        ForEach(tokenBalances) { entry in
                            // Tapping a token opens the permissions
                            // view pinned to *this* identity — but
                            // ONLY for local identities: pinning an
                            // observed one would bypass the view's
                            // wallet-owned query and present mutation
                            // and group-action screens the user
                            // cannot sign for. Observed identities
                            // get the read-only row.
                            if identity.isLocal {
                                NavigationLink(
                                    destination: TokenActionPermissionsView(
                                        token: entry.token,
                                        identity: identity,
                                        initialBalance: entry.balance
                                    )
                                ) {
                                    IdentityTokenRow(entry: entry)
                                }
                            } else {
                                IdentityTokenRow(entry: entry)
                            }
                        }
                    }
                } header: {
                    HStack {
                        Text("Tokens")
                        Spacer()
                        Button(action: reloadTokenBalances) {
                            Image(systemName: "arrow.clockwise")
                                .symbolEffect(
                                    .rotate,
                                    options: .nonRepeating,
                                    isActive: isLoadingTokens
                                )
                        }
                        .disabled(isLoadingTokens)
                    }
                }

            // DashPay Profile Section
            //
            // Reads `ManagedIdentity.dashpay_profile` through the
            // platform-wallet FFI. Refreshes from Platform on appear
            // via `syncDashPayProfiles()` so the cache reflects the
            // latest on-chain state without blocking the first paint.
            Section("DashPay Profile") {
                dashPayProfileCard(identity: identity)
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
            // The DPNS name list is now driven by `@Query` over
            // `PersistentDPNSName`. The Rust-side
            // `register_name_with_external_signer` path queues an
            // `IdentityChangeSet` whose persister-callback hop
            // upserts the new label row, which `dpnsNamesRows`
            // observes — no manual @State poke needed. We still pass
            // an `onRegistered` closure so `RegisterNameView` can
            // honor its callback contract, but the body is a no-op.
            RegisterNameView(identity: identity, onRegistered: { _ in })
                .environmentObject(appState)
        }
        .sheet(isPresented: $showingSelectMainName) {
            SelectMainNameView(identity: identity)
                .environmentObject(appState)
        }
        .sheet(isPresented: $showingProfileEditor) {
            DashPayProfileEditorView(
                identityId: identity.identityId,
                walletId: identity.wallet?.walletId,
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
        .sheet(isPresented: $showingTopUp) {
            TopUpIdentityView(identity: identity)
                .environmentObject(walletManager)
        }
        .sheet(isPresented: $showingTransferCredits) {
            TransferCreditsView(identity: identity)
                .environmentObject(walletManager)
        }
        .sheet(isPresented: $showingWithdrawCredits) {
            WithdrawCreditsView(identity: identity)
                .environmentObject(walletManager)
        }
        .onAppear {
            print("🔵 IdentityDetailView onAppear - dpnsName: \(identity.dpnsName ?? "nil"), isLocal: \(identity.isLocal)")

            // Load DPNS names from network if we don't have any cached or if they're empty
            if dpnsNames.isEmpty && contestedDpnsNames.isEmpty {
                print("🔵 No cached DPNS names, loading from network...")
                loadDPNSNames()
            } else {
                print("🔵 Using cached DPNS names: \(dpnsNames.count) regular, \(contestedDpnsNames.count) contested")
            }

            do {
                // Read whatever's currently cached synchronously so the
                // card renders immediately, then kick off a background
                // sync to freshen it. The sync uses the merged
                // `IdentityWallet` `sync_profiles` FFI path.
                loadCachedDashPayProfile(for: identity)
                Task { await refreshDashPayProfilesFromPlatform(for: identity) }

                // Reload the Tokens section on every appear, including
                // when the user pops back from a token action
                // (Transfer / Burn). Those flows mutate the on-chain
                // balance and refresh the local PersistentTokenBalance
                // rows, but this section reads a transient @State
                // snapshot — gating the reload on `tokenBalances.isEmpty`
                // (the old behavior) left a drained sender still showing
                // its pre-transfer figure until a manual refresh
                // (MW-02). An unconditional reload keeps the displayed
                // balance current after returning here; it stays a
                // single batched round-trip and shows the cached numbers
                // while it refreshes.
                reloadTokenBalances()
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
                // Refresh balance + public-key rows from Platform via
                // the shared persist/load helper (also used by
                // KeyDetailView's Disable Key action so the disabled
                // badge appears without a manual refresh).
                try await IdentityKeyRefresher.refreshBalanceAndKeys(
                    identity: identity,
                    sdk: sdk,
                    modelContext: modelContext
                )

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
        guard identity != nil else { return }

        Task {
            await loadDPNSNamesFromNetwork()
        }
    }

    private func loadDPNSNamesFromNetwork() async {
        guard let identity = identity else { return }

        print("🔵 loadDPNSNamesFromNetwork called for identity \(identity.identityIdBase58)")

        isLoadingDPNS = true
        defer { isLoadingDPNS = false }

        guard appState.sdk != nil else { return }

        // Regular DPNS labels: kick a Rust-side
        // `IdentityWallet::sync_dpns_names` so the persister callback
        // receives a fresh `IdentityChangeSet` and upserts our
        // `PersistentDPNSName` rows. The view's `@Query` over
        // `dpnsNamesRows` picks the new rows up reactively — no
        // assignment needed here. The returned tuple's labels are
        // ignored on purpose; SwiftData is the source of truth.
        _ = await fetchRegularDPNSNames(identity: identity)

        // Contested labels still flow through plain `@State` —
        // they aren't part of the `PersistentDPNSName` collection
        // (different lifecycle: in-flight contest churn vs. settled
        // labels). The contested cache stays a per-view cache for
        // now.
        let contested = await fetchContestedDPNSNames(identity: identity)

        await MainActor.run {
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
        if let walletId = identity.wallet?.walletId,
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
        if let walletId = identity.wallet?.walletId,
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

                // Mutations submit a state transition through the
                // owning wallet's manager — read-only display stays
                // available for walletless/observed identities, the
                // editor does not.
                if hasLoadedWallet(for: identity) {
                    Button {
                        showingProfileEditor = true
                    } label: {
                        HStack {
                            Image(systemName: "pencil")
                            Text("Edit Profile")
                        }
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

                // Same wallet gate as the Edit button above —
                // creating a profile broadcasts through the owning
                // wallet's manager.
                if hasLoadedWallet(for: identity) {
                    Button {
                        showingProfileEditor = true
                    } label: {
                        HStack {
                            Image(systemName: "pencil")
                            Text("Set up profile")
                        }
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
    ///
    /// Routes through the `ManagedIdentity` handle rather than the
    /// wallet-level by-id FFI for two reasons:
    ///   1. If the identity isn't in the wallet's manager (e.g. the
    ///      Rust-side changeset for a freshly-registered identity
    ///      hasn't settled into `IdentityManager::identities` by the
    ///      time the detail view appears), we surface that as
    ///      `.identityNotFound` from `managedIdentity(identityId:)`
    ///      and skip silently.
    ///   2. The profile read then goes through the handle-based
    ///      `managed_identity_get_dashpay_profile` rather than the
    ///      wallet-level `platform_wallet_get_dashpay_profile`.
    ///      Only a `Handle` (u64) crosses the boundary — no
    ///      pass-by-value aggregates.
    private func loadCachedDashPayProfile(for identity: PersistentIdentity) {
        guard let walletId = identity.wallet?.walletId,
              let wallet = walletManager.wallet(for: walletId) else {
            return
        }
        do {
            let managed = try wallet.managedIdentity(identityId: identity.identityId)
            dashpayProfile = try managed.getDashPayProfile()
        } catch let error as PlatformWalletError {
            if case .identityNotFound = error {
                // Expected right after a fresh register — the
                // IdentityManager may not have observed the new
                // entry yet. The background `syncDashPayProfiles()`
                // task triggered from onAppear will pick it up once
                // the Rust side has it.
                return
            }
            print("⚠️ Failed to read cached DashPay profile: \(error)")
        } catch {
            print("⚠️ Failed to read cached DashPay profile: \(error)")
        }
    }

    /// Drive a Platform-side sync for every identity on the wallet,
    /// then re-read this identity's cache. Runs in a background task
    /// (the FFI dispatches to an 8 MB tokio worker internally).
    ///
    /// Same handle-based read pattern as `loadCachedDashPayProfile`:
    /// resolve the `ManagedIdentity` first, then call its
    /// `getDashPayProfile()` so only a `Handle` crosses the FFI.
    @MainActor
    private func refreshDashPayProfilesFromPlatform(for identity: PersistentIdentity) async {
        guard let walletId = identity.wallet?.walletId,
              let wallet = walletManager.wallet(for: walletId) else {
            return
        }
        isLoadingProfile = dashpayProfile == nil
        defer { isLoadingProfile = false }

        do {
            _ = try await wallet.syncDashPayProfiles()
            // Pick up whatever the sync wrote back into the cache.
            let managed = try wallet.managedIdentity(identityId: identity.identityId)
            dashpayProfile = try managed.getDashPayProfile()
            profileError = nil
        } catch let error as PlatformWalletError {
            if case .identityNotFound = error {
                // Same race as in the cached-read path — the
                // identity may not be in the Rust manager yet.
                // Don't surface as a failure.
                profileError = nil
                return
            }
            print("⚠️ DashPay profile sync failed: \(error)")
            profileError = "Profile sync failed: \(error.localizedDescription)"
        } catch {
            // Don't blow away a previously-shown profile on transient
            // sync failure — surface the error underneath instead.
            print("⚠️ DashPay profile sync failed: \(error)")
            profileError = "Profile sync failed: \(error.localizedDescription)"
        }
    }

    // MARK: - Tokens

    /// Read every locally-known PersistentToken, compute the canonical
    /// platform token id for each, then ask the SDK for this identity's
    /// balance against that set in one round trip. Filters to non-zero
    /// balances for display — the "tokens this identity has" framing
    /// implies nonzero. The whole pipeline runs in a single Task so
    /// the UI can show a spinner without blocking.
    @MainActor
    private func reloadTokenBalances() {
        guard let identity = identity,
              let sdk = appState.sdk else { return }

        // In-flight guard: `onAppear` now reloads unconditionally and this
        // view can re-appear rapidly (background-sync @Query churn), so bail
        // if a refresh is already running to avoid stacking redundant
        // network/FFI round-trips that race `tokenBalances`. Reliable on the
        // main actor — everything up to `isLoadingTokens = true` runs
        // synchronously, so a second call sees the flag before the first
        // call's Task yields.
        guard !isLoadingTokens else { return }

        isLoadingTokens = true
        tokensError = nil

        Task { @MainActor in
            defer { isLoadingTokens = false }

            // Pull tokens off the live PersistentToken store, scoped
            // to the active network via the parent contract's
            // `networkRaw`. `PersistentToken` itself has no network
            // field — every saved contract's tokens are children of
            // a `PersistentDataContract`, and that's where the
            // network lives. Without this filter we'd compute token
            // ids that don't exist on the active network (off-network
            // ids return zero balances and surface confusing
            // empty-state UX after a network switch).
            let target = appState.currentNetwork.rawValue
            let descriptor = FetchDescriptor<PersistentToken>(
                predicate: #Predicate<PersistentToken> { token in
                    token.dataContract?.networkRaw == target
                }
            )
            // Use an explicit do-catch (not `try?`) so a thrown
            // SwiftData fetch error surfaces in `tokensError`
            // instead of collapsing into the same "no tokens"
            // branch as a legitimately empty result. Earlier
            // `try?` revisions also wiped any previously-loaded
            // balances on a transient failure; preserve them
            // here so a flaky reload doesn't blank out the
            // section.
            let allTokens: [PersistentToken]
            do {
                allTokens = try modelContext.fetch(descriptor)
            } catch {
                tokensError =
                    "Failed to load local tokens: \(error.localizedDescription)"
                return
            }
            guard !allTokens.isEmpty else {
                tokenBalances = []
                return
            }

            // Compute token ids in one pass. Skip tokens whose
            // position is out of u16 range (shouldn't happen — that
            // would be a malformed row) so we don't crash on a bad
            // downcast.
            var idToToken: [String: PersistentToken] = [:]
            for token in allTokens {
                guard token.position >= 0, token.position <= Int(UInt16.max) else { continue }
                let pos = UInt16(token.position)
                let cidBase58 = token.contractId.toBase58String()
                if let canonical = try? sdk.calculateTokenId(contractId: cidBase58, position: pos) {
                    idToToken[canonical] = token
                }
            }
            guard !idToToken.isEmpty else {
                tokenBalances = []
                return
            }

            // Persist balances into `PersistentTokenBalance` via the
            // manager-level identity-sync pipeline. We register the
            // identity with this view's token list, kick a single
            // sync pass, and let the Rust persister fire the
            // `on_persist_token_balances_fn` callback — the Swift
            // handler maps that onto SwiftData rows that the rest of
            // the app reads via @Query (recipient pickers, Burn /
            // Transfer / DestroyFrozen views). Failures here are
            // non-fatal: the display fetch below still surfaces the
            // numbers, and the next reload tries again.
            //
            // (`registerIdentityForTokenSync` is idempotent — calling
            // again with a different token list replaces the watched
            // set; balances for tokens kept across the swap survive.)
            let identityBytes = identity.identityId
            let tokenIdData: [Identifier] = idToToken.keys.compactMap { tokenIdBase58 in
                Data.identifier(fromBase58: tokenIdBase58)
            }
            do {
                try walletManager.registerIdentityForTokenSync(
                    identityId: identityBytes,
                    tokenIds: tokenIdData
                )
                try await walletManager.syncIdentityTokensNow()
            } catch {
                print("⚠️ identity token sync failed: \(error)")
            }

            do {
                let balances = try await sdk.getIdentityTokenBalances(
                    identityId: identity.identityIdBase58,
                    tokenIds: Array(idToToken.keys)
                )
                // Filter to non-zero, sort by token name for stability.
                let entries: [IdentityTokenEntry] = balances.compactMap { (tokenId, balance) -> IdentityTokenEntry? in
                    guard balance > 0, let token = idToToken[tokenId] else { return nil }
                    return IdentityTokenEntry(tokenId: tokenId, token: token, balance: balance)
                }.sorted { (lhs, rhs) in
                    let lname = lhs.token.getPluralForm() ?? lhs.token.displayName
                    let rname = rhs.token.getPluralForm() ?? rhs.token.displayName
                    return lname.localizedCaseInsensitiveCompare(rname) == .orderedAscending
                }
                tokenBalances = entries
                tokensError = nil
            } catch {
                tokensError = "Failed to load token balances: \(error.localizedDescription)"
            }
        }
    }
}

// MARK: - Identity ID row

/// Compact tap-to-copy row for an identity id rendered in some
/// representation (hex, base58, …). Two of these stack in the Identity
/// Details header so users can grab whichever shape the next tool
/// downstream needs. `.contextMenu` on a `Label` inside a `List` row
/// is unreliable (the row's own gesture eats the long-press), so the
/// copy target is a visible borderless button instead.
private struct IdentityIDCopyRow: View {
    let icon: String
    let value: String
    let accessibilityLabel: String

    var body: some View {
        HStack(alignment: .top, spacing: 6) {
            Image(systemName: icon)
                .foregroundColor(.secondary)
                .font(.caption)
            Text(value)
                .font(.caption)
                .foregroundColor(.secondary)
                .textSelection(.enabled)
            Spacer(minLength: 4)
            Button {
                UIPasteboard.general.string = value
                UINotificationFeedbackGenerator().notificationOccurred(.success)
            } label: {
                Image(systemName: "doc.on.doc")
                    .font(.caption)
                    .foregroundColor(.blue)
            }
            .buttonStyle(.borderless)
            .accessibilityLabel(accessibilityLabel)
        }
    }
}

// MARK: - Token row

/// One token + balance entry, keyed by the canonical base58 token id
/// so SwiftUI's `ForEach` has a stable identifier across reloads.
private struct IdentityTokenEntry: Identifiable, Hashable {
    let tokenId: String
    let token: PersistentToken
    let balance: UInt64
    var id: String { tokenId }
}

/// Row view for one token holding. Displays the token's plural-form
/// (or fallback display name), its balance scaled by `decimals`, and
/// the parent contract's name as a caption.
private struct IdentityTokenRow: View {
    let entry: IdentityTokenEntry

    var body: some View {
        HStack(alignment: .firstTextBaseline) {
            VStack(alignment: .leading, spacing: 2) {
                Text(entry.token.getPluralForm() ?? entry.token.displayName)
                    .font(.subheadline)
                    .fontWeight(.medium)
                Text(contractCaption)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }
            Spacer()
            Text(formattedBalance)
                .font(.subheadline.monospacedDigit())
                .fontWeight(.semibold)
        }
        .padding(.vertical, 2)
    }

    /// Prefer the contract's friendly name; fall back to a truncated
    /// base58 contract id when the contract isn't loaded locally.
    private var contractCaption: String {
        if let name = entry.token.dataContract?.name, !name.isEmpty {
            return name
        }
        let cid = entry.token.contractIdBase58
        if cid.count > 12 {
            let prefix = cid.prefix(6)
            let suffix = cid.suffix(4)
            return "\(prefix)…\(suffix)"
        }
        return cid
    }

    /// Format the raw u64 balance with the token's `decimals`. Uses
    /// `Decimal` so we don't lose precision converting through Double
    /// for high-decimal tokens with large balances.
    private var formattedBalance: String {
        let decimals = max(0, entry.token.decimals)
        let raw = Decimal(entry.balance)
        let divisor = pow(Decimal(10), decimals)
        let scaled = divisor == 0 ? raw : (raw / divisor)

        let formatter = NumberFormatter()
        formatter.numberStyle = .decimal
        formatter.maximumFractionDigits = decimals
        formatter.minimumFractionDigits = 0
        formatter.usesGroupingSeparator = true
        return formatter.string(from: scaled as NSNumber) ?? "\(entry.balance)"
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
    /// Wallet that owns `identityId`. Required to resolve the
    /// submitting manager — a `nil` (or unloaded) wallet surfaces an
    /// error rather than falling back to an arbitrary wallet that
    /// doesn't manage this identity. Presenting views gate the editor
    /// on `hasLoadedWallet`, so the error path is belt-and-braces.
    let walletId: Data?
    let existing: DashPayProfile?
    let onSaved: (DashPayProfile) -> Void

    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.dismiss) var dismiss
    @Environment(\.modelContext) private var modelContext

    @State private var displayName: String = ""
    @State private var publicMessage: String = ""
    @State private var avatarUrl: String = ""
    @State private var isSaving = false
    @State private var errorMessage: String?

    private var isCreating: Bool { existing == nil }

    /// DashPay `profile` contract limits — live counters below gate
    /// Save instead of failing at broadcast time.
    private static let displayNameLimit = 25
    private static let publicMessageLimit = 140

    private var overLimit: Bool {
        displayName.count > Self.displayNameLimit
            || publicMessage.count > Self.publicMessageLimit
    }

    var body: some View {
        NavigationView {
            Form {
                Section {
                    TextField("e.g. Alice", text: $displayName)
                        .textInputAutocapitalization(.words)
                        .accessibilityIdentifier("dashpay.profile.displayName")
                } header: {
                    Text("Display name")
                } footer: {
                    Text("\(displayName.count)/\(Self.displayNameLimit)")
                        .foregroundColor(
                            displayName.count > Self.displayNameLimit ? .red : .secondary
                        )
                }

                Section {
                    TextField("A short bio that contacts can see", text: $publicMessage, axis: .vertical)
                        .lineLimit(3, reservesSpace: true)
                        .accessibilityIdentifier("dashpay.profile.publicMessage")
                } header: {
                    Text("Public message")
                } footer: {
                    Text("\(publicMessage.count)/\(Self.publicMessageLimit)")
                        .foregroundColor(
                            publicMessage.count > Self.publicMessageLimit ? .red : .secondary
                        )
                }

                Section("Avatar URL") {
                    TextField("https://…", text: $avatarUrl)
                        .keyboardType(.URL)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .accessibilityIdentifier("dashpay.profile.avatarUrl")
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
                        .accessibilityIdentifier("dashpay.profile.cancel")
                }
                ToolbarItem(placement: .navigationBarTrailing) {
                    // Save flow: Save replaced by a ProgressView
                    // while in flight; success dismisses; failure
                    // re-enables with the red caption in the form.
                    if isSaving {
                        ProgressView()
                    } else {
                        Button(isCreating ? "Create" : "Save") { save() }
                            .disabled(overLimit)
                            .accessibilityIdentifier("dashpay.profile.save")
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

        // Enforce the HTTPS-only rule the form promises, in code: the DIP-15
        // avatar pipeline fetches the image to compute its integrity hashes,
        // and a plaintext-http (or non-http scheme) URL is both a privacy
        // leak and not reliably fetchable. Reject it here rather than relying
        // on the helper text alone. Scheme-parse (not a prefix check) so
        // "HTTPS://" and odd casings are handled.
        if !cleanedUrl.isEmpty,
           URL(string: cleanedUrl)?.scheme?.lowercased() != "https" {
            errorMessage = "Avatar URL must be an https:// link."
            return
        }

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

                // Resolve the wallet via the identity's `walletId`
                // only — an arbitrary fallback wallet doesn't manage
                // this identity and would fail (or worse, submit
                // through the wrong manager).
                guard let wallet = walletId.flatMap({ walletManager.wallet(for: $0) }) else {
                    errorMessage = "This identity's wallet isn't loaded, so the profile can't be submitted"
                    return
                }
                // Construct a fresh `KeychainSigner` for this submit
                // pass the same way `RegisterNameView.registerName()`
                // does. Routes the document state-transition signature
                // through the iOS Keychain so watch-only wallets work
                // end-to-end.
                let signer = KeychainSigner(modelContainer: modelContext.container)
                let saved: DashPayProfile
                if isCreating {
                    saved = try await wallet.createDashPayProfile(
                        identityId: identityId,
                        update: update,
                        signer: signer
                    )
                } else {
                    saved = try await wallet.updateDashPayProfile(
                        identityId: identityId,
                        update: update,
                        signer: signer
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
