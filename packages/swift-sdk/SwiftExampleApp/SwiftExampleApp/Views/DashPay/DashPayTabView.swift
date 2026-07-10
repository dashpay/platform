import SwiftUI
import SwiftData
import SwiftDashSDK

/// Root of the DashPay tab: active-identity picker →
/// profile header card → segmented [Contacts | Requests] → toolbar
/// + (AddContactView) and refresh. Owns its own NavigationStack like
/// the other tab wrappers in `ContentView`.
struct DashPayTabView: View {
    let network: Network
    /// Root tab selection — the empty states deep-link to the
    /// Wallets / Identities tabs.
    @Binding var selectedTab: RootTab

    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject private var appUIState: AppUIState
    @EnvironmentObject var appState: AppState
    @Environment(\.modelContext) private var modelContext
    @Environment(\.scenePhase) private var scenePhase

    /// All persisted identities on the active network. Filtered down
    /// to wallet-backed, on-network identities in `eligibleIdentities`.
    @Query private var identities: [PersistentIdentity]

    /// Selection persists across launches. Stores the base58
    /// id; a stale id (identity deleted / other network) falls back
    /// to the first eligible identity in `activeIdentity`.
    @AppStorage("dashpay.activeIdentityId") private var storedIdentityId: String = ""

    /// Device-local contact metadata (alias / note / hide / DPNS
    /// hint) shared with every child view via the environment.
    @StateObject private var contactMeta = DashPayContactMetaStore()

    @State private var segment: DashPaySegment = .contacts
    @State private var showAddContact = false
    @State private var showAddViaQR = false

    /// Drives the claim sheet via `.sheet(item:)`. A fresh value (new `id`)
    /// re-presents the sheet — so a second `dashpay://invite` link arriving while
    /// the sheet is already open re-seeds it with the new URI instead of being
    /// dropped (`.sheet(isPresented:)` can't re-seed an already-presented sheet
    /// whose `uri` is seeded once at init).
    private struct ClaimInvite: Identifiable {
        let id = UUID()
        let walletId: Data
        let initialURI: String
    }

    @State private var claimInvite: ClaimInvite?

    /// Optimistic overlay for *send*: contact ids whose request
    /// was just broadcast but whose outgoing row hasn't landed via
    /// the persister yet. Rendered as synthetic "Pending" rows in the
    /// Outgoing section; pruned there when the query catches up or a
    /// sync pass completes.
    @State private var optimisticSentIds: Set<Data> = []

    // Own-profile state for the header card (wallet-cache read, same
    // pattern as IdentityDetailView's DashPay Profile section).
    @State private var ownProfile: DashPayProfile?
    @State private var showProfileView = false
    @State private var showProfileEditor = false
    @State private var pendingEditorAfterProfileView = false

    /// DPNS-username prompt state. `showRegisterName` drives the
    /// registration sheet. `usernameResolvedIds` holds identities for
    /// which an on-chain lookup has *confirmed* no name — the prompt only
    /// shows for those, so an identity that has a username we simply
    /// hadn't cached yet never gets nagged to register one.
    @State private var showRegisterName = false
    @State private var usernameResolvedIds: Set<Data> = []
    /// Surfaces a failed/declined "finish setup" unlock so the banner tap
    /// isn't a silent no-op (wrong seed, or a watch-only wallet with no
    /// Keychain mnemonic).
    @State private var unlockError: String?

    /// Effective-foreground tracking for the sync cadence: the tab is on
    /// screen (`tabVisible`) **and** the app is active (`scenePhase`).
    /// `syncForeground` is the last cadence we applied, so we only act on
    /// transitions (and kick at most once per entry).
    @State private var tabVisible = false
    @State private var syncForeground = false

    enum DashPaySegment: Hashable {
        case contacts, requests
    }

    /// Background sync cadence. While the DashPay tab is foreground we
    /// poll fast so a contact's request / acceptance / payment surfaces
    /// in near real time; we relax to the standard interval when the tab
    /// is backgrounded so an idle app isn't sweeping every few seconds.
    private static let foregroundSyncSeconds: UInt64 = 4
    private static let backgroundSyncSeconds: UInt64 = 15

    init(network: Network, selectedTab: Binding<RootTab>) {
        self.network = network
        _selectedTab = selectedTab
        let raw = network.rawValue
        _identities = Query(
            filter: #Predicate<PersistentIdentity> { $0.networkRaw == raw },
            sort: [SortDescriptor(\PersistentIdentity.createdAt)]
        )
    }

    /// Identities the DashPay tab can act as: on-network (not
    /// local-only) and backed by a wallet that's currently loaded in
    /// the manager — every DashPay FFI call resolves through that
    /// wallet handle.
    private var eligibleIdentities: [PersistentIdentity] {
        identities.filter { identity in
            guard !identity.isLocal,
                  let walletId = identity.wallet?.walletId else { return false }
            return walletManager.wallet(for: walletId) != nil
        }
    }

    /// Stale-id fallback: stored selection wins when still
    /// eligible, else the first eligible identity.
    private var activeIdentity: PersistentIdentity? {
        if let match = eligibleIdentities.first(where: {
            $0.identityIdBase58 == storedIdentityId
        }) {
            return match
        }
        return eligibleIdentities.first
    }

    /// Wallet the "Claim invitation" flow registers the new identity under.
    /// Prefers the active identity's wallet, else the first loaded wallet on
    /// this network — so a fresh invitee with no identity yet can still claim.
    private var claimWalletId: Data? {
        activeIdentity?.wallet?.walletId
            ?? walletManager.wallets.keys.sorted { $0.lexicographicallyPrecedes($1) }.first
    }

    /// Present the claim sheet pre-filled for a pending `dashpay://invite` link
    /// (captured by the app's `.onOpenURL` into `AppUIState.pendingInviteURL`)
    /// and clear it so it isn't re-triggered. Invoked on both the warm path
    /// (`.onChange`) and the cold-launch path (`.onAppear`); the nil guard makes
    /// the second call after the first clears it a no-op (no double-present).
    private func consumePendingInviteURL() {
        guard let urlString = appUIState.pendingInviteURL else { return }
        // Clear the bearer URL immediately so it can't linger in @Published; the
        // nil-write re-fires this via .onChange, where the guard above no-ops.
        appUIState.pendingInviteURL = nil
        guard let walletId = claimWalletId else { return }
        // A fresh ClaimInvite (new id) presents the sheet — and RE-presents it if
        // one is already open, so a second invite link arriving mid-claim
        // re-seeds it with the new URI instead of being dropped.
        claimInvite = ClaimInvite(walletId: walletId, initialURI: urlString)
    }

    var body: some View {
        NavigationStack {
            content
                .navigationTitle("DashPay")
                .toolbar {
                    ToolbarItem(placement: .navigationBarTrailing) {
                        Button {
                            refresh()
                        } label: {
                            Image(systemName: "arrow.clockwise")
                                .symbolEffect(
                                    .rotate,
                                    options: .nonRepeating,
                                    isActive: walletManager.dashPaySyncIsSyncing
                                )
                        }
                        .disabled(walletManager.dashPaySyncIsSyncing)
                        .accessibilityIdentifier("dashpay.refresh")
                    }
                    ToolbarItem(placement: .navigationBarTrailing) {
                        Button {
                            showAddContact = true
                        } label: {
                            Image(systemName: "person.badge.plus")
                        }
                        .disabled(activeIdentity == nil)
                        .accessibilityIdentifier("dashpay.addContact")
                    }
                    ToolbarItem(placement: .navigationBarTrailing) {
                        Button {
                            showAddViaQR = true
                        } label: {
                            Image(systemName: "qrcode.viewfinder")
                        }
                        .disabled(activeIdentity == nil)
                        .accessibilityIdentifier("dashpay.addViaQR")
                    }
                    ToolbarItem(placement: .navigationBarTrailing) {
                        Button {
                            if let walletId = claimWalletId {
                                claimInvite = ClaimInvite(walletId: walletId, initialURI: "")
                            }
                        } label: {
                            Image(systemName: "gift")
                        }
                        .disabled(claimWalletId == nil)
                        .accessibilityLabel("Claim invitation")
                        .accessibilityIdentifier("dashpay.claimInvitation")
                    }
                    ToolbarItem(placement: .navigationBarLeading) {
                        if let identity = activeIdentity {
                            NavigationLink {
                                IgnoredContactsView(identity: identity)
                                    .environmentObject(walletManager)
                            } label: {
                                Image(systemName: "person.crop.circle.badge.xmark")
                            }
                            .accessibilityIdentifier("dashpay.openIgnored")
                        }
                    }
                    ToolbarItem(placement: .navigationBarLeading) {
                        if let walletId = claimWalletId {
                            NavigationLink {
                                InvitationsView(walletId: walletId, network: network)
                            } label: {
                                Image(systemName: "paperplane")
                            }
                            .accessibilityLabel("Sent invitations")
                            .accessibilityIdentifier("dashpay.openSentInvitations")
                        }
                    }
                }
                .sheet(isPresented: $showAddViaQR) {
                    if let identity = activeIdentity {
                        AddViaQRSheet(identity: identity)
                            .environmentObject(walletManager)
                    }
                }
                .sheet(item: $claimInvite) { invite in
                    ClaimInvitationSheet(
                        walletId: invite.walletId,
                        network: network,
                        initialURI: invite.initialURI
                    )
                    .environmentObject(walletManager)
                }
                .onChange(of: appUIState.pendingInviteURL) { _, _ in
                    // Warm path: the app is already running, so the tab observes
                    // the nil→url transition set by .onOpenURL.
                    consumePendingInviteURL()
                }
                .onAppear {
                    // Cold-launch path: .onOpenURL fires during scene connection,
                    // before this tab exists (ContentView shows "Initializing…"
                    // until bootstrap finishes), so .onChange never sees the
                    // transition. Consume any already-set pending URL when the tab
                    // first appears (.onOpenURL forces selectedTab = .dashpay, so
                    // this tab does appear).
                    consumePendingInviteURL()
                }
                .sheet(isPresented: $showAddContact) {
                    if let identity = activeIdentity {
                        AddContactView(
                            identity: identity,
                            onSent: { recipientId, dpnsName in
                                optimisticSentIds.insert(recipientId)
                                if let dpnsName {
                                    contactMeta.setDpnsHint(
                                        dpnsName,
                                        network: identity.network,
                                        owner: identity.identityId,
                                        contact: recipientId
                                    )
                                }
                            }
                        )
                        .environmentObject(walletManager)
                    }
                }
                .sheet(
                    isPresented: $showProfileView,
                    onDismiss: {
                        if pendingEditorAfterProfileView {
                            pendingEditorAfterProfileView = false
                            showProfileEditor = true
                        }
                    }
                ) {
                    if let identity = activeIdentity {
                        DashPayProfileView(
                            identity: identity,
                            profile: ownProfile,
                            onEdit: {
                                pendingEditorAfterProfileView = true
                                showProfileView = false
                            }
                        )
                        .environmentObject(walletManager)
                    }
                }
                .sheet(isPresented: $showProfileEditor) {
                    if let identity = activeIdentity {
                        DashPayProfileEditorView(
                            identityId: identity.identityId,
                            walletId: identity.wallet?.walletId,
                            existing: ownProfile,
                            onSaved: { saved in
                                ownProfile = saved
                            }
                        )
                        .environmentObject(walletManager)
                    }
                }
                .sheet(isPresented: $showRegisterName) {
                    if let identity = activeIdentity {
                        // RegisterNameView brings its own NavigationView +
                        // Cancel toolbar. On success, persist the new name onto
                        // the identity's scalar `dpnsName` and drop it from the
                        // resolved set so this prompt hides on the next render.
                        // The Rust register path only upserts `PersistentDPNSName`
                        // relationship rows — not the scalar `dpnsName` /
                        // `mainDpnsName` this prompt (and the header) read — so
                        // without this the CTA would linger until the next
                        // identity switch or app-foreground re-check.
                        RegisterNameView(identity: identity, onRegistered: { label in
                            PersistentIdentity.updateDpnsName(
                                in: modelContext,
                                identityId: identity.identityId,
                                dpnsName: label
                            )
                            usernameResolvedIds.remove(identity.identityId)
                            try? modelContext.save()
                        })
                            .environmentObject(walletManager)
                            .environmentObject(appState)
                    }
                }
                // Value-based push for the hidden-contacts recovery
                // screen. Declared on the stack root (not on ContactsView)
                // so the destination builds only on navigate — a
                // closure-based link inside the frequently-syncing
                // Contacts list would rebuild it on every @Query
                // re-render.
                .navigationDestination(for: DashPayHiddenContactsRoute.self) { route in
                    if let identity = eligibleIdentities.first(where: {
                        $0.identityId == route.ownerIdentityId
                    }) {
                        HiddenContactsView(identity: identity)
                            .environmentObject(walletManager)
                    }
                }
        }
        .environmentObject(contactMeta)
        .onAppear { tabVisible = true; refreshSyncCadence() }
        .onDisappear { tabVisible = false; refreshSyncCadence() }
        .onChange(of: scenePhase) { _, phase in
            refreshSyncCadence()
            // Re-confirm the username on app-foreground: `.task` already
            // re-runs on tab re-appearance, but a name registered on
            // another device while the user sits on this tab would
            // otherwise keep the "register a username" prompt up until
            // the next tab switch. (A name registered in-app hides the
            // prompt immediately via @Query.)
            if phase == .active {
                Task { await resolveUsernameIfNeeded(for: activeIdentity) }
            }
        }
        .alert(
            "Couldn't finish setup",
            isPresented: Binding(
                get: { unlockError != nil },
                set: { if !$0 { unlockError = nil } }
            )
        ) {
            Button("OK", role: .cancel) {}
        } message: {
            Text(unlockError ?? "")
        }
        .task(id: activeIdentity?.identityId) {
            loadOwnProfileFromCache()
            // Kick one sweep so a fresh launch shows current data
            // without waiting for the background loop's next tick.
            // The Rust manager dedupes — an in-flight pass makes
            // this a no-op sentinel return.
            _ = try? await walletManager.dashPaySyncNow()
            loadOwnProfileFromCache()
            await resolveUsernameIfNeeded(for: activeIdentity)
        }
        .onChange(of: walletManager.dashPaySyncIsSyncing) { _, syncing in
            // Re-read the own-profile cache after every completed
            // sync pass — the background loop may have refreshed it.
            if !syncing {
                loadOwnProfileFromCache()
            }
        }
        .onChange(of: storedIdentityId) { _, _ in
            // The optimistic pending-sent overlay is per-identity
            // state — without this reset, a send from identity A
            // ghosts as an outgoing row under identity B after a
            // picker switch.
            optimisticSentIds.removeAll()
        }
    }

    // MARK: - Content states (§6.4 identity picker)

    @ViewBuilder
    private var content: some View {
        if walletManager.wallets.isEmpty {
            // State 1: no wallet loaded.
            DashPayEmptyStateView(
                icon: "wallet.pass",
                title: "No wallet loaded",
                message: "Load or create a wallet to use DashPay.",
                buttonTitle: "Open Wallets",
                buttonIdentifier: "dashpay.openWallets",
                action: { selectedTab = .wallets }
            )
        } else if eligibleIdentities.isEmpty {
            // State 2: wallet present, zero usable identities.
            DashPayEmptyStateView(
                icon: "person.crop.circle.badge.questionmark",
                title: "No identities yet",
                message: "Register an identity to start using DashPay.",
                buttonTitle: "Open Identities",
                buttonIdentifier: "dashpay.openIdentities",
                action: { selectedTab = .identities }
            )
        } else if let identity = activeIdentity {
            // State 3: ≥1 identity → picker (hidden when exactly one).
            VStack(spacing: 0) {
                if eligibleIdentities.count > 1 {
                    identityPicker(active: identity)
                }

                profileHeaderCard(identity: identity)

                usernamePromptCard(identity: identity)

                dashPayBalanceRow(identity: identity)

                dashPayUnlockBanner(identity: identity)

                Picker("Section", selection: $segment) {
                    Text("Contacts").tag(DashPaySegment.contacts)
                    Text("Requests").tag(DashPaySegment.requests)
                }
                .pickerStyle(.segmented)
                .padding(.horizontal)
                .padding(.bottom, 6)
                .accessibilityIdentifier("dashpay.segment")

                switch segment {
                case .contacts:
                    ContactsView(identity: identity)
                        .id(identity.identityId)
                case .requests:
                    ContactRequestsView(
                        identity: identity,
                        optimisticSentIds: $optimisticSentIds
                    )
                    .id(identity.identityId)
                }
            }
        }
    }

    // MARK: - Needs-unlock / verify-failed banner

    /// Surfaces the DashPay needs-unlock / verify-failed signal for the active
    /// identity's wallet. Priority: a seed mismatch (signing disabled) supersedes
    /// everything; an in-flight drain shows a non-actionable "finishing" state;
    /// otherwise a pending account-build backlog offers Unlock. Nothing renders
    /// on a healthy wallet. The count is wallet-scoped (may include sibling
    /// identities on the same wallet), so the copy says "waiting," not a promise.
    @ViewBuilder
    private func dashPayUnlockBanner(identity: PersistentIdentity) -> some View {
        if let walletId = identity.wallet?.walletId,
           let status = walletManager.dashPayUnlockStatus[walletId],
           status.hasSignal {
            if status.seedMismatch {
                unlockBannerRow(
                    text: "Seed verification failed — this wallet's Keychain seed "
                        + "doesn't match. DashPay signing is disabled.",
                    systemImage: "exclamationmark.triangle.fill",
                    tint: .red,
                    action: nil
                )
            } else if status.draining {
                unlockBannerRow(
                    text: "Finishing contact setup…",
                    systemImage: "hourglass",
                    tint: .orange,
                    action: nil
                )
            } else if status.pendingAccountBuilds > 0 {
                let n = Int(status.pendingAccountBuilds)
                unlockBannerRow(
                    text: "\(n) contact\(n == 1 ? "" : "s") waiting to finish setup",
                    systemImage: "lock.fill",
                    tint: .orange,
                    action: walletManager.wallet(for: walletId).map { wallet in
                        {
                            // Don't swallow: a wrong-seed mismatch throws and a
                            // watch-only wallet returns false — both must tell
                            // the user why tapping "finish setup" did nothing.
                            do {
                                let unlocked = try walletManager.unlockWalletFromKeychain(wallet)
                                if !unlocked {
                                    unlockError = "This wallet is watch-only on this device "
                                        + "(no mnemonic in the Keychain), so contact setup "
                                        + "can't be finished here."
                                }
                            } catch {
                                unlockError = error.localizedDescription
                            }
                        }
                    }
                )
            }
        }
    }

    /// One banner row: an icon + message tinted by severity, with an optional
    /// trailing Unlock button. Mirrors the inline `paymentChannelBroken` warning
    /// styling (icon + `.orange`/`.red`), promoted to a tappable container.
    private func unlockBannerRow(
        text: String,
        systemImage: String,
        tint: Color,
        action: (() -> Void)?
    ) -> some View {
        HStack(spacing: 8) {
            Image(systemName: systemImage)
                .foregroundColor(tint)
            Text(text)
                .font(.caption)
                .foregroundColor(.primary)
                .fixedSize(horizontal: false, vertical: true)
            Spacer()
            if let action {
                Button("Unlock", action: action)
                    .font(.caption.bold())
                    .buttonStyle(.borderedProminent)
                    .controlSize(.small)
            }
        }
        .padding(10)
        .background(tint.opacity(0.12), in: RoundedRectangle(cornerRadius: 10))
        .padding(.horizontal)
        .padding(.bottom, 6)
        .accessibilityIdentifier("dashpay.unlockBanner")
    }

    // MARK: - Identity picker

    private func identityPicker(active: PersistentIdentity) -> some View {
        Menu {
            ForEach(eligibleIdentities, id: \.identityId) { identity in
                Button {
                    storedIdentityId = identity.identityIdBase58
                } label: {
                    if identity.identityId == active.identityId {
                        Label(pickerLabel(for: identity), systemImage: "checkmark")
                    } else {
                        Text(pickerLabel(for: identity))
                    }
                }
            }
        } label: {
            HStack(spacing: 6) {
                Image(systemName: "person.crop.circle")
                Text(pickerLabel(for: active))
                    .lineLimit(1)
                Image(systemName: "chevron.up.chevron.down")
                    .font(.caption2)
            }
            .font(.subheadline)
            .padding(.horizontal, 12)
            .padding(.vertical, 6)
            .background(Color.blue.opacity(0.1))
            .cornerRadius(8)
        }
        .padding(.horizontal)
        .padding(.top, 4)
        .frame(maxWidth: .infinity, alignment: .leading)
        .accessibilityIdentifier("dashpay.identityPicker")
    }

    /// Menu rows show "DPNS name → truncated id".
    private func pickerLabel(for identity: PersistentIdentity) -> String {
        if let name = identity.mainDpnsName ?? identity.dpnsName, !name.isEmpty {
            return name
        }
        if let alias = identity.alias, !alias.isEmpty {
            return alias
        }
        return String(identity.identityIdBase58.prefix(12)) + "…"
    }

    // MARK: - Profile header card

    @ViewBuilder
    private func profileHeaderCard(identity: PersistentIdentity) -> some View {
        if let profile = ownProfile {
            Button {
                showProfileView = true
            } label: {
                HStack(spacing: 12) {
                    DashPayAvatarView(
                        avatarUrl: profile.avatarUrl,
                        displayName: headerDisplayName(identity: identity, profile: profile),
                        size: 48
                    )
                    VStack(alignment: .leading, spacing: 2) {
                        Text(headerDisplayName(identity: identity, profile: profile))
                            .font(.headline)
                            .foregroundColor(.primary)
                        if let dpns = identity.mainDpnsName ?? identity.dpnsName {
                            Text(dpns)
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                        if let msg = profile.publicMessage?
                            .trimmingCharacters(in: .whitespacesAndNewlines),
                           !msg.isEmpty {
                            Text(msg)
                                .font(.caption2)
                                .foregroundColor(.secondary)
                                .lineLimit(1)
                        }
                    }
                    Spacer()
                    Image(systemName: "chevron.right")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .padding(12)
                .background(Color.blue.opacity(0.06))
                .cornerRadius(12)
            }
            .buttonStyle(.plain)
            .padding(.horizontal)
            .padding(.vertical, 8)
            .accessibilityIdentifier("dashpay.profileHeader")
        } else {
            // Empty state → CTA straight into the editor sheet
            // (same target as "Edit").
            Button {
                showProfileEditor = true
            } label: {
                HStack(spacing: 12) {
                    Image(systemName: "person.crop.circle.dashed")
                        .font(.title2)
                        .foregroundColor(.blue)
                    VStack(alignment: .leading, spacing: 2) {
                        Text("Set up your DashPay profile")
                            .font(.subheadline)
                            .fontWeight(.medium)
                            .foregroundColor(.primary)
                    }
                    Spacer()
                    Image(systemName: "chevron.right")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .padding(12)
                .background(Color.blue.opacity(0.06))
                .cornerRadius(12)
            }
            .buttonStyle(.plain)
            .padding(.horizontal)
            .padding(.vertical, 8)
            .accessibilityIdentifier("dashpay.profileHeader.setup")
        }
    }

    // MARK: - Username (DPNS) prompt

    /// Prompt to register a DPNS username — the searchable handle other
    /// users type to find and add you. Shown only once an on-chain check
    /// has confirmed the active identity has no name (see
    /// `resolveUsernameIfNeeded`), so an identity that already has one is
    /// never nagged. Distinct from the profile card: the profile's
    /// display name is cosmetic and not searchable.
    @ViewBuilder
    private func usernamePromptCard(identity: PersistentIdentity) -> some View {
        let hasName = (identity.mainDpnsName ?? identity.dpnsName)
            .map { !$0.isEmpty } ?? false
        if !hasName, usernameResolvedIds.contains(identity.identityId) {
            Button {
                showRegisterName = true
            } label: {
                HStack(spacing: 12) {
                    Image(systemName: "at.badge.plus")
                        .font(.title2)
                        .foregroundColor(.blue)
                    VStack(alignment: .leading, spacing: 2) {
                        Text("Register a username")
                            .font(.subheadline)
                            .fontWeight(.medium)
                            .foregroundColor(.primary)
                        Text(
                            "Without a username, people can't find you by name to send "
                                + "a request. Your profile name is just a display name — "
                                + "it isn't searchable."
                        )
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                    }
                    Spacer()
                    Image(systemName: "chevron.right")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .padding(12)
                .background(Color.blue.opacity(0.06))
                .cornerRadius(12)
            }
            .buttonStyle(.plain)
            .padding(.horizontal)
            .padding(.vertical, 8)
            .accessibilityIdentifier("dashpay.usernamePrompt")
        }
    }

    // MARK: - DashPay balance

    /// Funds received from contacts — the sum of this identity's
    /// `DashpayReceivingFunds` account balances (type tag 12), read
    /// lock-free from Rust's in-memory account state (same call the
    /// wallet account list uses). These coins already count toward
    /// the wallet's Core Balance; this row answers the
    /// DashPay-specific question "how much have contacts sent me".
    private func dashPayBalanceRow(identity: PersistentIdentity) -> some View {
        let duffs: UInt64 = {
            guard let walletId = identity.wallet?.walletId else { return 0 }
            return walletManager.accountBalances(for: walletId)
                .filter { $0.typeTag == 12 && $0.userIdentityId == identity.identityId }
                .reduce(0) { $0 + $1.confirmed + $1.unconfirmed }
        }()
        return HStack {
            Label("Received from contacts", systemImage: "arrow.down.left.circle")
                .font(.caption)
                .foregroundColor(.secondary)
            Spacer()
            Text(String(format: "%.8f DASH", Double(duffs) / 100_000_000))
                .font(.caption)
                .fontWeight(.medium)
        }
        .padding(.horizontal)
        .padding(.bottom, 6)
        .accessibilityIdentifier("dashpay.receivedBalance")
    }

    private func headerDisplayName(
        identity: PersistentIdentity,
        profile: DashPayProfile
    ) -> String {
        if let name = profile.displayName?
            .trimmingCharacters(in: .whitespacesAndNewlines),
           !name.isEmpty {
            return name
        }
        if let dpns = identity.mainDpnsName ?? identity.dpnsName {
            return dpns
        }
        return String(identity.identityIdBase58.prefix(12)) + "…"
    }

    // MARK: - Actions

    /// Resolve the active identity's DashPay profile. Prefers the live
    /// wallet-handle cache (freshest), but falls back to the PERSISTED
    /// profile so an identity that already has a profile never shows the
    /// "set up profile" CTA just because its profile hasn't been synced
    /// into the in-memory cache this session — which happens on cold
    /// restore or right after switching the picker among many identities.
    /// `PersistentIdentity.dashpayProfile` is the source of truth for
    /// "does this identity have a profile"; the persister only writes it
    /// after a profile has been created/synced. Lock-free; no network.
    private func loadOwnProfileFromCache() {
        guard let identity = activeIdentity else {
            ownProfile = nil
            return
        }
        if let walletId = identity.wallet?.walletId,
           let wallet = walletManager.wallet(for: walletId),
           let managed = try? wallet.managedIdentity(identityId: identity.identityId),
           let cached = try? managed.getDashPayProfile() {
            ownProfile = cached
            return
        }
        ownProfile = identity.dashpayProfile.map { persisted in
            DashPayProfile(
                displayName: persisted.displayName,
                publicMessage: persisted.publicMessage,
                avatarUrl: persisted.avatarUrl,
                avatarHash: persisted.avatarHash,
                avatarFingerprint: persisted.avatarFingerprint
            )
        }
    }

    /// Confirm whether the active identity has a DPNS username, so the
    /// "register a username" prompt only nags identities that genuinely
    /// have none. If a name exists on-chain but wasn't cached, persist it
    /// (the prompt stays hidden); if the lookup definitively returns
    /// none, mark the id resolved so the prompt can show. A thrown error
    /// (offline / transient) leaves it unresolved to retry on a later
    /// pass — we never prompt on an unverified guess. Mirrors the lazy
    /// DPNS fetch in `IdentitiesView`.
    private func resolveUsernameIfNeeded(for identity: PersistentIdentity?) async {
        guard let identity else { return }
        if let name = identity.mainDpnsName ?? identity.dpnsName, !name.isEmpty {
            return
        }
        guard let sdk = appState.sdk else { return }
        do {
            let usernames = try await sdk.dpnsGetUsername(
                identityId: identity.identityIdBase58,
                limit: 1
            )
            if let label = usernames.first?["label"] as? String, !label.isEmpty {
                PersistentIdentity.updateDpnsName(
                    in: modelContext,
                    identityId: identity.identityId,
                    dpnsName: label
                )
                // updateDpnsName leaves persistence to the caller (matches
                // IdentitiesView's lazy fetch) — flush so a name we just
                // re-discovered survives a kill before the next autosave.
                try? modelContext.save()
            } else {
                usernameResolvedIds.insert(identity.identityId)
            }
        } catch {
            // Unverified (offline / transient) — leave unresolved so a
            // later pass retries rather than nagging on a guess.
        }
    }

    /// Toolbar refresh — fires one sweep through the manager. The
    /// Rust side skips if a pass is already in flight (§6.4 single
    /// sync-in-progress signal), and the button is disabled while
    /// `dashPaySyncIsSyncing` anyway.
    private func refresh() {
        Task { @MainActor in
            _ = try? await walletManager.dashPaySyncNow()
        }
    }

    /// Tune the background sync loop's cadence to *effective foreground* =
    /// the tab is on screen AND the app is active. Fast (4s) while the
    /// user is actually looking, relaxed (15s) otherwise — so neither a
    /// tab switch nor app-backgrounding leaves an idle app sweeping every
    /// few seconds. Driven from the NavigationStack's appear/disappear
    /// (so drilling into a child screen or presenting a sheet, which don't
    /// fire the stack's `onDisappear`, keep the fast cadence) plus
    /// `scenePhase`.
    ///
    /// On entering the foreground we also kick one sweep:
    /// `setDashPaySyncInterval` only takes effect on the loop's *next*
    /// sleep, so without the kick a tab re-entry could wait out a leftover
    /// long sleep before the first fast tick. Acts only on transitions, so
    /// the kick fires at most once per entry. Best-effort — a
    /// not-yet-configured manager keeps its interval, and the kick no-ops
    /// when a pass is already in flight.
    private func refreshSyncCadence() {
        let foreground = tabVisible && scenePhase == .active
        guard foreground != syncForeground else { return }
        syncForeground = foreground
        try? walletManager.setDashPaySyncInterval(
            seconds: foreground ? Self.foregroundSyncSeconds : Self.backgroundSyncSeconds
        )
        if foreground {
            kickDashPaySync(walletManager)
        }
    }
}

// MARK: - Empty-state helper

/// Shared empty-state body for the picker states: icon, title,
/// message, and a single CTA that deep-links to another tab.
struct DashPayEmptyStateView: View {
    let icon: String
    let title: String
    let message: String
    let buttonTitle: String
    let buttonIdentifier: String
    let action: () -> Void

    var body: some View {
        VStack(spacing: 16) {
            Spacer()
            Image(systemName: icon)
                .font(.system(size: 50))
                .foregroundColor(.gray)
            Text(title)
                .font(.title3)
                .fontWeight(.medium)
            Text(message)
                .font(.caption)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
            Button(buttonTitle, action: action)
                .buttonStyle(.borderedProminent)
                .accessibilityIdentifier(buttonIdentifier)
            Spacer()
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding()
    }
}

/// Send a contact request from a DIP-15 auto-accept QR URI. The user pastes the
/// `dash:?du=…&dapk=…` URI (from another user's "Add me" QR; a camera scan would
/// produce the same string); the Rust side resolves the username, signs the
/// proof, and broadcasts, so the QR owner auto-accepts.
private struct AddViaQRSheet: View {
    let identity: PersistentIdentity

    @EnvironmentObject private var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    @State private var uri = ""
    @State private var isSending = false
    @State private var errorMessage: String?

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField("dash:?du=…&dapk=…", text: $uri, axis: .vertical)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .lineLimit(2...4)
                        .accessibilityIdentifier("dashpay.qr.uriField")
                } header: {
                    Text("Paste an auto-accept QR URI")
                } footer: {
                    Text(
                        "From another user's “Add me (DIP-15 QR)”. They auto-accept "
                            + "your request for as long as their QR is valid."
                    )
                }
                if let errorMessage {
                    Text(errorMessage)
                        .font(.caption)
                        .foregroundColor(.red)
                }
            }
            .navigationTitle("Add via QR")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    if isSending {
                        ProgressView()
                    } else {
                        Button("Send") { send() }
                            .disabled(uri.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                            .accessibilityIdentifier("dashpay.qr.send")
                    }
                }
            }
        }
    }

    private func send() {
        let trimmed = uri.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        isSending = true
        errorMessage = nil
        Task { @MainActor in
            defer { isSending = false }
            do {
                guard let walletId = identity.wallet?.walletId,
                      let wallet = walletManager.wallet(for: walletId) else {
                    errorMessage = "No wallet loaded for this identity."
                    return
                }
                let signer = KeychainSigner(modelContainer: modelContext.container)
                _ = try await wallet.sendContactRequestFromQR(
                    senderIdentityId: identity.identityId,
                    uri: trimmed,
                    signer: signer
                )
                kickDashPaySync(walletManager)
                dismiss()
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }
}
