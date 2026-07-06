import SwiftUI
import SwiftData
import Combine
import SwiftDashSDK

/// Established-contacts list for the DashPay tab.
///
/// `@Query`-driven: a contact is *established* when both direction
/// rows exist for the same `(owner, contact)` pair — the Rust
/// `established` map projects both the sent and the incoming
/// request, so the join on `contactIdentityId` is the local
/// equivalent of that map (see the persister's upsert notes on
/// `PersistentDashpayContactRequest`).
struct ContactsView: View {
    let identity: PersistentIdentity

    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var contactMeta: DashPayContactMetaStore

    /// Every contact-request row owned by this identity, both
    /// directions. Grouped into established pairs in `contacts`.
    @Query private var requestRows: [PersistentDashpayContactRequest]

    @State private var searchText = ""

    init(identity: PersistentIdentity) {
        self.identity = identity
        _requestRows = Query(
            filter: PersistentDashpayContactRequest.predicate(
                ownerIdentityId: identity.identityId
            )
        )
    }

    /// One row per established contact. Alias / hidden come off the
    /// contact rows themselves (contactInfo-backed since M3, so they
    /// re-render reactively through the `requestRows` query); the
    /// DPNS hint stays in the meta store (add-time UI hint, not
    /// protocol state); profile display joins the wallet cache. ORs
    /// the pair's `paymentChannelBroken` flags.
    private var contacts: [EstablishedContactItem] {
        // DPNS hints still read through the meta store — tie the
        // computation to its published `version` for those edits.
        _ = contactMeta.version
        let byContact = Dictionary(grouping: requestRows, by: \.contactIdentityId)
        return byContact.compactMap { contactId, rows -> EstablishedContactItem? in
            guard rows.contains(where: { $0.isOutgoing }),
                  rows.contains(where: { !$0.isOutgoing }) else {
                return nil
            }
            // contactInfo displayHidden — hidden contacts stay
            // established (and payable) but leave the list.
            guard !rows.contains(where: \.contactHidden) else {
                return nil
            }
            let profile = cachedProfile(contactId)
            let dpnsHint = contactMeta.dpnsHint(
                network: identity.network,
                owner: identity.identityId,
                contact: contactId
            )
            let name = dashPayContactDisplayName(
                contactId: contactId,
                alias: rows.compactMap(\.contactAlias).first,
                profileDisplayName: profile?.displayName,
                dpnsLabel: dpnsHint
            )
            return EstablishedContactItem(
                contactId: contactId,
                displayName: name,
                avatarUrl: profile?.avatarUrl,
                dpnsName: dpnsHint,
                paymentChannelBroken: rows.contains(where: \.paymentChannelBroken)
            )
        }
        .sorted {
            $0.displayName.localizedCaseInsensitiveCompare($1.displayName)
                == .orderedAscending
        }
    }

    private func filtered(_ contacts: [EstablishedContactItem]) -> [EstablishedContactItem] {
        let trimmed = searchText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return contacts }
        return contacts.filter { contact in
            contact.displayName.localizedCaseInsensitiveContains(trimmed)
                || (contact.dpnsName?.localizedCaseInsensitiveContains(trimmed) ?? false)
                || contact.contactId.toHexString().hasPrefix(trimmed.lowercased())
        }
    }

    /// Whether this identity has any hidden established contact — gates
    /// the "Hidden contacts" recovery link (the only in-app way back to
    /// a hidden contact's detail view, where the hide toggle lives).
    private var hasHiddenContacts: Bool {
        let byContact = Dictionary(grouping: requestRows, by: \.contactIdentityId)
        return byContact.contains { _, rows in
            rows.contains(where: { $0.isOutgoing })
                && rows.contains(where: { !$0.isOutgoing })
                && rows.contains(where: \.contactHidden)
        }
    }

    var body: some View {
        // Derive the (filtered) row list once per body evaluation — the
        // profile cascade behind `contacts` is expensive, so re-deriving
        // it for the empty check, the header count, and the ForEach would
        // triple the cost on every keystroke.
        let rows = filtered(contacts)
        // `SwiftUI.Group` — unqualified `Group` resolves to the
        // Codable DPP type from SwiftDashSDK.
        return SwiftUI.Group {
            if rows.isEmpty && searchText.isEmpty && !hasHiddenContacts {
                List {
                    DashPayListEmptyRow(
                        icon: "person.2.slash",
                        title: "No contacts yet",
                        message: "Add your first contact to send Dash by username."
                    )
                }
                .listStyle(.insetGrouped)
            } else {
                List {
                    Section {
                        searchField
                        ForEach(rows) { contact in
                            NavigationLink {
                                ContactDetailView(
                                    identity: identity,
                                    contactId: contact.contactId
                                )
                            } label: {
                                ContactListRow(contact: contact)
                            }
                            .accessibilityIdentifier(
                                "dashpay.contact.\(contact.contactId.toBase58String())"
                            )
                        }
                    } header: {
                        Text("Contacts (\(rows.count))")
                    }

                    if hasHiddenContacts {
                        Section {
                            NavigationLink(
                                value: DashPayHiddenContactsRoute(
                                    ownerIdentityId: identity.identityId
                                )
                            ) {
                                Label("Hidden contacts", systemImage: "eye.slash")
                            }
                            .accessibilityIdentifier("dashpay.openHidden")
                        }
                    }
                }
                .listStyle(.insetGrouped)
            }
        }
        .refreshable {
            await attachOrStartSync(walletManager)
        }
    }

    private var searchField: some View {
        HStack(spacing: 8) {
            Image(systemName: "magnifyingglass")
                .foregroundColor(.secondary)
            TextField("Search contacts", text: $searchText)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                .accessibilityIdentifier("dashpay.search")
            if !searchText.isEmpty {
                Button {
                    searchText = ""
                } label: {
                    Image(systemName: "xmark.circle.fill")
                        .foregroundColor(.secondary)
                }
                .buttonStyle(.borderless)
                .accessibilityIdentifier("dashpay.search.clear")
            }
        }
    }

    /// Cache-only profile read off the wallet handle (no network).
    /// Misses are common — contacts' profiles only populate after a
    /// profile sync has seen them.
    private func cachedProfile(_ contactId: Data) -> DashPayProfile? {
        guard let walletId = identity.wallet?.walletId,
              let wallet = walletManager.wallet(for: walletId) else {
            return nil
        }
        return dashPayCachedProfile(
            wallet: wallet,
            ownerIdentityId: identity.identityId,
            contactId: contactId
        )
    }
}

// MARK: - Pull-to-refresh sync attach

/// Single sync-in-progress signal: a pull-to-refresh during an
/// in-flight sync *attaches* to it (waits for `dashPaySyncIsSyncing`
/// to clear) instead of double-firing; otherwise it starts one pass.
/// Shared by ContactsView and ContactRequestsView.
@MainActor
func attachOrStartSync(_ walletManager: PlatformWalletManager) async {
    if walletManager.dashPaySyncIsSyncing {
        for await syncing in walletManager.$dashPaySyncIsSyncing.values where !syncing {
            break
        }
    } else {
        _ = try? await walletManager.dashPaySyncNow()
    }
}

/// Fire-and-forget kick of a DashPay sync pass after a local mutation
/// (send request / accept / pay). It pulls the counterparty's state and
/// promotes the established pair without waiting for the next background
/// poll tick — so the user isn't left staring at a stale list right
/// after acting. Non-blocking: callers dismiss/continue immediately and
/// the Rust manager folds an in-flight pass into a no-op.
@MainActor
func kickDashPaySync(_ walletManager: PlatformWalletManager) {
    Task { _ = try? await walletManager.dashPaySyncNow() }
}

// MARK: - Row model + view

/// UI model for one established contact row, resolved from the
/// request-row pair + profile cache + local metadata.
struct EstablishedContactItem: Identifiable {
    let contactId: Data
    let displayName: String
    let avatarUrl: String?
    let dpnsName: String?
    let paymentChannelBroken: Bool

    var id: Data { contactId }
}

struct ContactListRow: View {
    let contact: EstablishedContactItem

    var body: some View {
        HStack(spacing: 10) {
            DashPayAvatarView(
                avatarUrl: contact.avatarUrl,
                displayName: contact.displayName
            )
            VStack(alignment: .leading, spacing: 2) {
                Text(contact.displayName)
                    .font(.headline)
                Text(contact.dpnsName
                    ?? String(contact.contactId.toHexString().prefix(12)) + "…")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            Spacer()
            if contact.paymentChannelBroken {
                // Broken payment channel — warning badge; the
                // detail view explains and disables Send Dash.
                Image(systemName: "exclamationmark.triangle.fill")
                    .foregroundColor(.orange)
                    .accessibilityLabel("Payment channel broken")
            }
        }
        .padding(.vertical, 4)
    }
}

// MARK: - Empty-row helper

/// Inline empty state rendered as a list row, shared by the
/// Contacts / Requests lists so pull-to-refresh keeps working on an
/// empty list (a bare VStack outside a List loses `.refreshable`).
struct DashPayListEmptyRow: View {
    let icon: String
    let title: String
    let message: String

    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: icon)
                .font(.system(size: 40))
                .foregroundColor(.gray)
            Text(title)
                .font(.headline)
            Text(message)
                .font(.caption)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 24)
        .listRowBackground(Color.clear)
    }
}
