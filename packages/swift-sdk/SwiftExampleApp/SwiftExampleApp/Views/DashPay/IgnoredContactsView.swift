import SwiftUI
import SwiftData
import SwiftDashSDK

/// The "Ignored" screen for the DashPay tab.
///
/// Lists every sender this identity has ignored (per-sender mute, = block,
/// reversible, local-only) with an **Un-ignore** action. Ignored ≠
/// invisible — these senders are just hidden from the main pending list;
/// this screen makes them recoverable.
///
/// `@Query`-driven off `PersistentDashpayIgnoredSender` (the SwiftData
/// mirror of the Rust `ignored_senders` set). Name + avatar resolve through
/// the same `getContactProfile` cache the Contacts list uses (falling back
/// to the truncated id when the contact's profile hasn't been fetched).
struct IgnoredContactsView: View {
    let identity: PersistentIdentity

    @EnvironmentObject var walletManager: PlatformWalletManager

    /// Every ignored-sender row owned by this identity.
    @Query private var ignoredRows: [PersistentDashpayIgnoredSender]

    /// Sender ids with an un-ignore currently in flight — the row's
    /// button is replaced by a spinner while the round-trip runs.
    @State private var inFlightIds: Set<Data> = []

    /// Optimistic removal overlay: an un-ignored sender stops rendering
    /// immediately (the persister deletes the row shortly after).
    @State private var removedOverlayIds: Set<Data> = []

    /// Per-row inline errors.
    @State private var rowErrors: [Data: String] = [:]

    init(identity: PersistentIdentity) {
        self.identity = identity
        _ignoredRows = Query(
            filter: PersistentDashpayIgnoredSender.predicate(
                ownerIdentityId: identity.identityId
            ),
            sort: \PersistentDashpayIgnoredSender.ignoredAt,
            order: .reverse
        )
    }

    private var visibleRows: [PersistentDashpayIgnoredSender] {
        ignoredRows.filter { !removedOverlayIds.contains($0.ignoredSenderId) }
    }

    var body: some View {
        // `SwiftUI.Group` — unqualified `Group` resolves to the Codable
        // DPP type from SwiftDashSDK.
        SwiftUI.Group {
            if visibleRows.isEmpty {
                List {
                    DashPayListEmptyRow(
                        icon: "person.crop.circle.badge.checkmark",
                        title: "No ignored contacts",
                        message: "Senders you ignore are hidden from your pending requests and listed here so you can un-ignore them."
                    )
                }
                .listStyle(.insetGrouped)
            } else {
                List {
                    Section {
                        ForEach(visibleRows, id: \.ignoredSenderId) { row in
                            IgnoredSenderRow(
                                displayName: displayName(for: row.ignoredSenderId),
                                avatarUrl: cachedProfile(row.ignoredSenderId)?.avatarUrl,
                                isInFlight: inFlightIds.contains(row.ignoredSenderId),
                                errorMessage: rowErrors[row.ignoredSenderId],
                                onUnignore: { unignore(senderId: row.ignoredSenderId) }
                            )
                        }
                    } header: {
                        Text("Ignored (\(visibleRows.count))")
                    }
                }
                .listStyle(.insetGrouped)
            }
        }
        .navigationTitle("Ignored")
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

    private func unignore(senderId: Data) {
        rowErrors[senderId] = nil
        inFlightIds.insert(senderId)
        Task { @MainActor in
            defer { inFlightIds.remove(senderId) }
            do {
                let wallet = try requireWallet()
                try await wallet.unignoreContactSender(
                    ourIdentityId: identity.identityId,
                    contactIdentityId: senderId
                )
                // Optimistic removal — the persister deletes the row and
                // the Rust side rewinds the cursor so the sender's
                // requests re-fetch on the next sweep.
                removedOverlayIds.insert(senderId)
            } catch {
                rowErrors[senderId] = "Un-ignore failed: \(error.localizedDescription)"
            }
        }
    }

    // MARK: - Display helpers

    private func displayName(for contactId: Data) -> String {
        dashPayContactDisplayName(
            contactId: contactId,
            alias: nil,
            profileDisplayName: cachedProfile(contactId)?.displayName,
            dpnsLabel: nil
        )
    }

    /// Cache-only profile read off the wallet handle (no network). A miss
    /// is common — falls back to the truncated id via
    /// `dashPayContactDisplayName`.
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

// MARK: - Row

struct IgnoredSenderRow: View {
    let displayName: String
    let avatarUrl: String?
    let isInFlight: Bool
    let errorMessage: String?
    let onUnignore: () -> Void

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
                    Button("Un-ignore", action: onUnignore)
                        .buttonStyle(.bordered)
                        .controlSize(.small)
                        .accessibilityIdentifier("dashpay.ignored.unignore")
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
