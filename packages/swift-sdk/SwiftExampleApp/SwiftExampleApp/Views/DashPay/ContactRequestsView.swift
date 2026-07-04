import SwiftUI
import SwiftData
import SwiftDashSDK

/// Incoming + outgoing contact requests. Incoming rows
/// carry Accept / Reject with per-row in-flight state; the Outgoing
/// section renders pending sent requests (previously loaded but
/// never shown anywhere in the app).
///
/// Pending vs established, from the `@Query` rows: a pair with both
/// direction rows is established (shown in ContactsView); a single
/// incoming row is a pending incoming request; a single outgoing row
/// is a pending sent request.
struct ContactRequestsView: View {
    let identity: PersistentIdentity

    /// Optimistic overlay for *send* — owned by the tab root so
    /// AddContactView can insert into it; pruned here when the
    /// `@Query` reflects the new outgoing row or a sync completes.
    @Binding var optimisticSentIds: Set<Data>

    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var contactMeta: DashPayContactMetaStore
    @Environment(\.modelContext) private var modelContext

    @Query private var requestRows: [PersistentDashpayContactRequest]

    /// Contact ids with an Accept/Reject currently in flight — the
    /// row's buttons are replaced by a `ProgressView` (blocks
    /// double-tap → duplicate accepts).
    @State private var inFlightIds: Set<Data> = []

    /// Optimistic overlay for accept/reject: ids whose incoming
    /// row should stop rendering before the persister catches up.
    /// Pruned in `onChange(of: requestRows)` once the query reflects
    /// the change; fallback-cleared after the next completed sync
    /// pass so a lost callback can't hide a row forever.
    @State private var removedOverlayIds: Set<Data> = []

    /// Per-row inline errors (§6.4: failure restores the buttons
    /// with an inline error on the row).
    @State private var rowErrors: [Data: String] = [:]

    init(identity: PersistentIdentity, optimisticSentIds: Binding<Set<Data>>) {
        self.identity = identity
        _optimisticSentIds = optimisticSentIds
        _requestRows = Query(
            filter: PersistentDashpayContactRequest.predicate(
                ownerIdentityId: identity.identityId
            )
        )
    }

    // MARK: - Derived rows

    private var rowsByContact: [Data: [PersistentDashpayContactRequest]] {
        Dictionary(grouping: requestRows, by: \.contactIdentityId)
    }

    /// Equatable change signal for the `@Query` rows — `@Model`
    /// classes aren't `Equatable`, so `onChange` watches this
    /// `(contact, direction)` set instead. Overlay pruning only
    /// cares about rows appearing/disappearing, which this captures.
    private var rowSignature: Set<String> {
        Set(requestRows.map {
            $0.contactIdentityId.toHexString() + ($0.isOutgoing ? ":o" : ":i")
        })
    }

    /// Incoming-only pairs, minus the optimistic-removal overlay.
    private var incomingPending: [PersistentDashpayContactRequest] {
        rowsByContact.compactMap { contactId, rows -> PersistentDashpayContactRequest? in
            guard !removedOverlayIds.contains(contactId),
                  !rows.contains(where: { $0.isOutgoing }),
                  let incoming = rows.first(where: { !$0.isOutgoing }) else {
                return nil
            }
            return incoming
        }
        .sorted { $0.createdAtMillis > $1.createdAtMillis }
    }

    /// Outgoing-only pairs.
    private var outgoingPending: [PersistentDashpayContactRequest] {
        rowsByContact.compactMap { _, rows -> PersistentDashpayContactRequest? in
            guard !rows.contains(where: { !$0.isOutgoing }),
                  let outgoing = rows.first(where: { $0.isOutgoing }) else {
                return nil
            }
            return outgoing
        }
        .sorted { $0.createdAtMillis > $1.createdAtMillis }
    }

    /// Sent requests still riding the optimistic overlay (broadcast
    /// done, persister row not landed yet).
    private var optimisticOutgoing: [Data] {
        optimisticSentIds
            .filter { rowsByContact[$0] == nil }
            .sorted { $0.toHexString() < $1.toHexString() }
    }

    var body: some View {
        SwiftUI.Group {
            if incomingPending.isEmpty && outgoingPending.isEmpty
                && optimisticOutgoing.isEmpty {
                List {
                    DashPayListEmptyRow(
                        icon: "tray",
                        title: "No pending requests",
                        message: "Incoming contact requests and your pending sent requests show up here."
                    )
                }
                .listStyle(.insetGrouped)
            } else {
                List {
                    if !incomingPending.isEmpty {
                        Section {
                            ForEach(incomingPending, id: \.contactIdentityId) { row in
                                IncomingRequestRow(
                                    displayName: displayName(for: row.contactIdentityId),
                                    // Privacy: do NOT load a pending (unsolicited) sender's
                                    // avatar — it's a sender-chosen URL, and an AsyncImage GET
                                    // before the user accepts would leak the recipient's IP /
                                    // online status to the sender. Show initials until the
                                    // contact is accepted (established rows load it normally).
                                    avatarUrl: nil,
                                    createdAtMillis: row.createdAtMillis,
                                    isInFlight: inFlightIds.contains(row.contactIdentityId),
                                    errorMessage: rowErrors[row.contactIdentityId],
                                    onAccept: { accept(contactId: row.contactIdentityId) },
                                    onIgnore: { ignore(contactId: row.contactIdentityId) }
                                )
                            }
                        } header: {
                            Text("Incoming (\(incomingPending.count))")
                        }
                    }

                    if !outgoingPending.isEmpty || !optimisticOutgoing.isEmpty {
                        Section {
                            ForEach(outgoingPending, id: \.contactIdentityId) { row in
                                OutgoingRequestRow(
                                    displayName: displayName(for: row.contactIdentityId),
                                    avatarUrl: cachedProfile(row.contactIdentityId)?.avatarUrl,
                                    createdAtMillis: row.createdAtMillis
                                )
                            }
                            // Synthetic rows for just-broadcast sends
                            // the persister hasn't projected yet.
                            ForEach(optimisticOutgoing, id: \.self) { contactId in
                                OutgoingRequestRow(
                                    displayName: displayName(for: contactId),
                                    avatarUrl: cachedProfile(contactId)?.avatarUrl,
                                    createdAtMillis: nil
                                )
                            }
                        } header: {
                            Text("Outgoing (\(outgoingPending.count + optimisticOutgoing.count))")
                        }
                    }
                }
                .listStyle(.insetGrouped)
            }
        }
        .refreshable {
            await attachOrStartSync(walletManager)
        }
        .onChange(of: rowSignature) { _, _ in
            pruneOverlays()
        }
        .onChange(of: walletManager.dashPaySyncIsSyncing) { _, syncing in
            // Fallback clearing rule: after the next completed
            // sync pass, expire whatever the query still doesn't
            // reflect — rows must not stay hidden (or synthetically
            // shown) forever on a missed callback.
            if !syncing {
                removedOverlayIds.removeAll()
                optimisticSentIds.removeAll()
            }
        }
    }

    // MARK: - Overlay maintenance

    /// Drop overlay entries the `@Query` already reflects:
    /// - removal overlay: the incoming-only pair is gone (row
    ///   deleted on reject, or promoted to established on accept);
    /// - send overlay: the outgoing row landed.
    private func pruneOverlays() {
        let byContact = rowsByContact
        removedOverlayIds = removedOverlayIds.filter { contactId in
            guard let rows = byContact[contactId] else { return false }
            // Still an incoming-only pair → keep hiding it.
            return rows.contains { !$0.isOutgoing }
                && !rows.contains { $0.isOutgoing }
        }
        optimisticSentIds = optimisticSentIds.filter { byContact[$0] == nil }
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

    private func accept(contactId: Data) {
        rowErrors[contactId] = nil
        inFlightIds.insert(contactId)
        Task { @MainActor in
            defer { inFlightIds.remove(contactId) }
            do {
                let wallet = try requireWallet()
                let managed = try wallet.managedIdentity(identityId: identity.identityId)
                guard let request = try managed.getIncomingContactRequest(
                    senderId: contactId
                ) else {
                    rowErrors[contactId] = "Request not in local state — pull to refresh"
                    return
                }
                let signer = KeychainSigner(modelContainer: modelContext.container)
                _ = try await wallet.acceptContactRequest(request, signer: signer)
                // Optimistic removal — the persister will promote the
                // pair to established shortly.
                removedOverlayIds.insert(contactId)
                kickDashPaySync(walletManager)
            } catch {
                rowErrors[contactId] = "Accept failed: \(error.localizedDescription)"
            }
        }
    }

    private func ignore(contactId: Data) {
        rowErrors[contactId] = nil
        inFlightIds.insert(contactId)
        Task { @MainActor in
            defer { inFlightIds.remove(contactId) }
            do {
                let wallet = try requireWallet()
                try await wallet.ignoreContactSender(
                    ourIdentityId: identity.identityId,
                    contactIdentityId: contactId
                )
                removedOverlayIds.insert(contactId)
            } catch {
                rowErrors[contactId] = "Ignore failed: \(error.localizedDescription)"
            }
        }
    }

    // MARK: - Display helpers

    private func displayName(for contactId: Data) -> String {
        _ = contactMeta.version
        return dashPayContactDisplayName(
            contactId: contactId,
            alias: contactMeta.alias(
                network: identity.network,
                owner: identity.identityId,
                contact: contactId
            ),
            profileDisplayName: cachedProfile(contactId)?.displayName,
            dpnsLabel: contactMeta.dpnsHint(
                network: identity.network,
                owner: identity.identityId,
                contact: contactId
            )
        )
    }

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

// MARK: - Incoming row

struct IncomingRequestRow: View {
    let displayName: String
    let avatarUrl: String?
    let createdAtMillis: UInt64
    let isInFlight: Bool
    let errorMessage: String?
    let onAccept: () -> Void
    let onIgnore: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 10) {
                DashPayAvatarView(avatarUrl: avatarUrl, displayName: displayName)
                VStack(alignment: .leading, spacing: 2) {
                    Text(displayName)
                        .font(.headline)
                    Text(relativeTimestamp(millis: createdAtMillis))
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
                Spacer()
            }

            if isInFlight {
                // Both buttons replaced by a spinner while the
                // accept/ignore round-trips.
                HStack {
                    Spacer()
                    ProgressView()
                    Spacer()
                }
            } else {
                HStack(spacing: 12) {
                    Button("Accept", action: onAccept)
                        .buttonStyle(.borderedProminent)
                        .controlSize(.small)
                        .accessibilityIdentifier("dashpay.request.accept")
                    Button("Ignore", action: onIgnore)
                        .buttonStyle(.bordered)
                        .controlSize(.small)
                        .tint(.red)
                        .accessibilityIdentifier("dashpay.request.ignore")
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

// MARK: - Outgoing row

struct OutgoingRequestRow: View {
    let displayName: String
    let avatarUrl: String?
    /// `nil` for synthetic optimistic rows (no persisted timestamp yet).
    let createdAtMillis: UInt64?

    var body: some View {
        HStack(spacing: 10) {
            DashPayAvatarView(avatarUrl: avatarUrl, displayName: displayName)
            VStack(alignment: .leading, spacing: 2) {
                Text(displayName)
                    .font(.headline)
                if let millis = createdAtMillis {
                    Text(relativeTimestamp(millis: millis))
                        .font(.caption2)
                        .foregroundColor(.secondary)
                } else {
                    Text("Just now")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
            }
            Spacer()
            Text("Pending")
                .font(.caption)
                .fontWeight(.medium)
                .foregroundColor(.orange)
                .padding(.horizontal, 8)
                .padding(.vertical, 2)
                .background(Color.orange.opacity(0.15))
                .cornerRadius(4)
        }
        .padding(.vertical, 4)
    }
}

/// "3 min. ago"-style relative timestamp from a Unix-millis value;
/// falls back to "—" for the zero sentinel.
func relativeTimestamp(millis: UInt64) -> String {
    guard millis > 0 else { return "—" }
    let date = Date(timeIntervalSince1970: TimeInterval(millis) / 1000)
    return date.formatted(.relative(presentation: .named))
}
