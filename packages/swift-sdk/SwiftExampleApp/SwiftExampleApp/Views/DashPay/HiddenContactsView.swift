import SwiftUI
import SwiftData
import SwiftDashSDK

/// Value-based navigation route to `HiddenContactsView`. Declared as a
/// `.navigationDestination(for:)` on the DashPay tab's stack root so the
/// push from the Contacts list builds the destination only on navigate —
/// a closure-based `NavigationLink` would rebuild it on every `@Query`
/// re-render of the frequently-syncing Contacts list.
struct DashPayHiddenContactsRoute: Hashable {
    let ownerIdentityId: Data
}

/// The "Hidden" screen for the DashPay tab.
///
/// Lists every established contact this identity has hidden
/// (`contactInfo.displayHidden`, reversible, synced cross-device) with
/// an **Unhide** action. Hidden ≠ removed — these contacts stay
/// established and payable but leave the main Contacts list; this
/// screen makes them recoverable (without it, hiding a contact removes
/// the only row that links to `ContactDetailView`, where the hide
/// toggle lives, so hiding would be a one-way trip).
///
/// `@Query`-driven off `PersistentDashpayContactRequest` (both
/// directions), grouped into established pairs the same way
/// `ContactsView` does. Name + avatar resolve through the shared
/// `dashPayCachedProfile` cache the Contacts list uses.
struct HiddenContactsView: View {
    let identity: PersistentIdentity

    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var contactMeta: DashPayContactMetaStore
    @Environment(\.modelContext) private var modelContext

    /// Every contact-request row owned by this identity, both
    /// directions. Grouped into established pairs in `hiddenContacts`.
    @Query private var requestRows: [PersistentDashpayContactRequest]

    /// Contact ids with an unhide currently in flight — the row's
    /// button is replaced by a spinner while the round-trip runs.
    @State private var inFlightIds: Set<Data> = []

    /// Optimistic removal overlay: an unhidden contact stops rendering
    /// immediately (the persister clears the flag shortly after).
    @State private var removedOverlayIds: Set<Data> = []

    /// Per-row inline errors.
    @State private var rowErrors: [Data: String] = [:]

    init(identity: PersistentIdentity) {
        self.identity = identity
        _requestRows = Query(
            filter: PersistentDashpayContactRequest.predicate(
                ownerIdentityId: identity.identityId
            )
        )
    }

    /// One row per established, hidden contact — the exact
    /// complement of `ContactsView.contacts` (which drops these).
    private var hiddenContacts: [HiddenContactItem] {
        _ = contactMeta.version
        let byContact = Dictionary(grouping: requestRows, by: \.contactIdentityId)
        return byContact.compactMap { contactId, rows -> HiddenContactItem? in
            guard rows.contains(where: { $0.isOutgoing }),
                  rows.contains(where: { !$0.isOutgoing }),
                  rows.contains(where: \.contactHidden),
                  !removedOverlayIds.contains(contactId) else {
                return nil
            }
            let profile = cachedProfile(contactId)
            let dpnsHint = contactMeta.dpnsHint(
                network: identity.network,
                owner: identity.identityId,
                contact: contactId
            )
            let alias = rows.compactMap(\.contactAlias).first
            let name = dashPayContactDisplayName(
                contactId: contactId,
                alias: alias,
                profileDisplayName: profile?.displayName,
                dpnsLabel: dpnsHint
            )
            return HiddenContactItem(
                contactId: contactId,
                displayName: name,
                avatarUrl: profile?.avatarUrl,
                alias: alias,
                note: rows.compactMap(\.contactNote).first
            )
        }
        .sorted {
            $0.displayName.localizedCaseInsensitiveCompare($1.displayName)
                == .orderedAscending
        }
    }

    var body: some View {
        // `SwiftUI.Group` — unqualified `Group` resolves to the Codable
        // DPP type from SwiftDashSDK.
        SwiftUI.Group {
            if hiddenContacts.isEmpty {
                List {
                    DashPayListEmptyRow(
                        icon: "eye",
                        title: "No hidden contacts",
                        message: "Contacts you hide stay payable but leave your Contacts list, and are listed here so you can unhide them."
                    )
                }
                .listStyle(.insetGrouped)
            } else {
                List {
                    Section {
                        ForEach(hiddenContacts) { contact in
                            HiddenContactRow(
                                displayName: contact.displayName,
                                avatarUrl: contact.avatarUrl,
                                isInFlight: inFlightIds.contains(contact.contactId),
                                errorMessage: rowErrors[contact.contactId],
                                onUnhide: { unhide(contact) }
                            )
                        }
                    } header: {
                        Text("Hidden (\(hiddenContacts.count))")
                    }
                }
                .listStyle(.insetGrouped)
            }
        }
        .navigationTitle("Hidden")
        .navigationBarTitleDisplayMode(.inline)
    }

    // MARK: - Actions

    private func requireWallet() throws -> ManagedPlatformWallet {
        guard let walletId = identity.wallet?.walletId,
              let wallet = walletManager.wallet(for: walletId) else {
            throw PlatformWalletError.walletOperation(
                "No loaded wallet for identity \(identity.identityIdBase58)"
            )
        }
        return wallet
    }

    /// Republish the contact's `contactInfo` with `hidden = false`,
    /// preserving the existing alias/note so unhiding doesn't wipe
    /// them. Same pipeline `ContactDetailView`'s hide toggle uses.
    private func unhide(_ contact: HiddenContactItem) {
        rowErrors[contact.contactId] = nil
        inFlightIds.insert(contact.contactId)
        Task { @MainActor in
            defer { inFlightIds.remove(contact.contactId) }
            do {
                let wallet = try requireWallet()
                let signer = KeychainSigner(modelContainer: modelContext.container)
                _ = try await wallet.setDashPayContactInfo(
                    identityId: identity.identityId,
                    contactId: contact.contactId,
                    alias: contact.alias,
                    note: contact.note,
                    hidden: false,
                    signer: signer
                )
                // Optimistic removal — the persister clears the flag on
                // the rows shortly after and the contact returns to the
                // main Contacts list.
                removedOverlayIds.insert(contact.contactId)
                kickDashPaySync(walletManager)
            } catch {
                rowErrors[contact.contactId] = "Unhide failed: \(error.localizedDescription)"
            }
        }
    }

    // MARK: - Display helpers

    /// Cache-only profile read off the wallet handle (no network). A
    /// miss is common — falls back to the truncated id.
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

// MARK: - Row model + view

/// UI model for one hidden contact row. Carries the current alias/note
/// so unhide can republish `contactInfo` without dropping them.
struct HiddenContactItem: Identifiable {
    let contactId: Data
    let displayName: String
    let avatarUrl: String?
    let alias: String?
    let note: String?

    var id: Data { contactId }
}

struct HiddenContactRow: View {
    let displayName: String
    let avatarUrl: String?
    let isInFlight: Bool
    let errorMessage: String?
    let onUnhide: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 10) {
                DashPayAvatarView(avatarUrl: avatarUrl, displayName: displayName)
                VStack(alignment: .leading, spacing: 2) {
                    Text(displayName)
                        .font(.headline)
                }
                Spacer()
                if isInFlight {
                    ProgressView()
                } else {
                    Button("Unhide", action: onUnhide)
                        .buttonStyle(.bordered)
                        .controlSize(.small)
                        .accessibilityIdentifier("dashpay.hidden.unhide")
                }
            }
            if let errorMessage {
                Text(errorMessage)
                    .font(.caption)
                    .foregroundColor(.red)
            }
        }
        .padding(.vertical, 4)
    }
}
