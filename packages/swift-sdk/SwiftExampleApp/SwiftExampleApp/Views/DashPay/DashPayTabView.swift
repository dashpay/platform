import SwiftUI
import SwiftData
import SwiftDashSDK

/// Root of the DashPay tab (SPEC §6.1): active-identity picker →
/// profile header card → segmented [Contacts | Requests] → toolbar
/// + (AddContactView) and refresh. Owns its own NavigationStack like
/// the other tab wrappers in `ContentView`.
struct DashPayTabView: View {
    let network: Network
    /// Root tab selection — the §6.4 empty states deep-link to the
    /// Wallets / Identities tabs.
    @Binding var selectedTab: RootTab

    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var appState: AppState
    @Environment(\.modelContext) private var modelContext

    /// All persisted identities on the active network. Filtered down
    /// to wallet-backed, on-network identities in `eligibleIdentities`.
    @Query private var identities: [PersistentIdentity]

    /// §6.4: selection persists across launches. Stores the base58
    /// id; a stale id (identity deleted / other network) falls back
    /// to the first eligible identity in `activeIdentity`.
    @AppStorage("dashpay.activeIdentityId") private var storedIdentityId: String = ""

    /// Device-local contact metadata (alias / note / hide / DPNS
    /// hint) shared with every child view via the environment.
    @StateObject private var contactMeta = DashPayContactMetaStore()

    @State private var segment: DashPaySegment = .contacts
    @State private var showAddContact = false

    /// §6.4 optimistic overlay for *send*: contact ids whose request
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

    enum DashPaySegment: Hashable {
        case contacts, requests
    }

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

    /// §6.4 stale-id fallback: stored selection wins when still
    /// eligible, else the first eligible identity.
    private var activeIdentity: PersistentIdentity? {
        if let match = eligibleIdentities.first(where: {
            $0.identityIdBase58 == storedIdentityId
        }) {
            return match
        }
        return eligibleIdentities.first
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
        }
        .environmentObject(contactMeta)
        .task(id: activeIdentity?.identityId) {
            loadOwnProfileFromCache()
            // Kick one sweep so a fresh launch shows current data
            // without waiting for the background loop's next tick.
            // The Rust manager dedupes — an in-flight pass makes
            // this a no-op sentinel return.
            _ = try? await walletManager.dashPaySyncNow()
            loadOwnProfileFromCache()
        }
        .onChange(of: walletManager.dashPaySyncIsSyncing) { _, syncing in
            // Re-read the own-profile cache after every completed
            // sync pass — the background loop may have refreshed it.
            if !syncing {
                loadOwnProfileFromCache()
            }
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

                dashPayBalanceRow(identity: identity)

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

    /// §6.1: menu rows show "DPNS name → truncated id".
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
            // (same target as "Edit", per §6.2).
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
                        Text("Add a display name and avatar so contacts can find you.")
                            .font(.caption)
                            .foregroundColor(.secondary)
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

    /// Synchronously read the active identity's cached DashPay
    /// profile off the wallet handle. Lock-free; no network.
    private func loadOwnProfileFromCache() {
        guard let identity = activeIdentity,
              let walletId = identity.wallet?.walletId,
              let wallet = walletManager.wallet(for: walletId) else {
            ownProfile = nil
            return
        }
        do {
            let managed = try wallet.managedIdentity(identityId: identity.identityId)
            ownProfile = try managed.getDashPayProfile()
        } catch {
            // identityNotFound right after a fresh register is
            // expected; keep whatever we last showed.
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
}

// MARK: - Empty-state helper

/// Shared empty-state body for the §6.4 picker states: icon, title,
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
