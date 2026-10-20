import SwiftDashSDK
import SwiftData
import SwiftUI

/// "Sent invitations" list (DIP-13): every invitation this wallet created,
/// newest first. Rows are `PersistentInvitation` records upserted by the
/// `on_persist_invitations_fn` bridge whenever `create_invitation` flushes its
/// changeset. A still-unclaimed (`Created`) row can be reclaimed — recovering the
/// voucher's value as identity credits — via a swipe action.
struct InvitationsView: View {
    let network: Network
    /// The active identity, if any. Reclaim needs no identity (it uses each
    /// invitation's own `walletId`); `identity` is required solely to *create* a
    /// new invitation, so it is optional — a wallet whose last identity was deleted
    /// can still reclaim its funded invitations, only the "+" create is hidden.
    let identity: PersistentIdentity?

    // ALL sent invitations, newest first, then filtered below to wallets the
    // ACTIVE manager has loaded. Not scoped to one wallet: the paperplane is
    // reached from a single-wallet context, but a device may have several
    // wallets loaded and each invitation is reclaimed via its OWN `walletId`
    // (below), so scoping the list to one wallet would strand the others'
    // funded, reclaimable invitations with no way to reach them.
    @Query(sort: [SortDescriptor(\PersistentInvitation.createdAtSecs, order: .reverse)])
    private var allInvitations: [PersistentInvitation]

    // Reclaim resolves each row's wallet through the active network's manager,
    // so rows whose wallet is NOT loaded there (another network's wallet — the
    // row carries no network discriminator — or a deleted wallet) would only
    // render a dead "No wallet loaded" reclaim. Hide them instead.
    private var invitations: [PersistentInvitation] {
        allInvitations.filter { walletManager.wallet(for: $0.walletId) != nil }
    }

    @EnvironmentObject private var walletManager: PlatformWalletManager

    @State private var reclaimTarget: PersistentInvitation?
    @State private var showCreateInvitation = false

    init(network: Network, identity: PersistentIdentity?) {
        self.network = network
        self.identity = identity
    }

    var body: some View {
        List {
            if invitations.isEmpty {
                ContentUnavailableView(
                    "No invitations yet",
                    systemImage: "gift",
                    description: Text("Tap + to invite a friend.")
                )
            } else {
                ForEach(invitations) { invitation in
                    row(invitation)
                        .swipeActions(edge: .trailing, allowsFullSwipe: false) {
                            if invitation.statusRaw == 0 {
                                Button {
                                    reclaimTarget = invitation
                                } label: {
                                    Label("Reclaim", systemImage: "arrow.uturn.backward.circle")
                                }
                                .tint(.orange)
                                .accessibilityIdentifier("dashpay.invitations.reclaim")
                            }
                        }
                }
            }
        }
        .navigationTitle("Sent Invitations")
        .navigationBarTitleDisplayMode(.inline)
        .accessibilityIdentifier("dashpay.invitations.list")
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                // Creating an invitation needs an inviter identity; reclaiming does
                // not. Hide "+" when there's no active identity so the reclaim list
                // stays usable.
                if identity != nil {
                    Button {
                        showCreateInvitation = true
                    } label: {
                        Image(systemName: "plus.circle")
                    }
                    .accessibilityLabel("Create invitation")
                    .accessibilityIdentifier("dashpay.invitations.create")
                }
            }
        }
        .sheet(item: $reclaimTarget) { invitation in
            ReclaimInvitationSheet(
                invitation: invitation,
                // Reclaim targets the invitation's OWN wallet, so a multi-wallet
                // list reclaims each row against the correct wallet.
                walletId: invitation.walletId,
                network: network
            )
        }
        .sheet(isPresented: $showCreateInvitation) {
            if let identity {
                CreateInvitationSheet(identity: identity)
            }
        }
    }

    @ViewBuilder
    private func row(_ invitation: PersistentInvitation) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(formatDash(invitation.amountDuffs))
                    .font(.headline)
                Spacer()
                statusBadge(invitation.statusRaw)
            }
            Text(shortOutPoint(invitation.outPointHex))
                .font(.caption)
                .foregroundColor(.secondary)
                .textSelection(.enabled)
            HStack(spacing: 8) {
                if invitation.hasInviter {
                    Label("Contact request", systemImage: "person.crop.circle.badge.plus")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
                Text(expiryText(invitation.expiryUnix))
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
        .padding(.vertical, 2)
    }
}

// MARK: - Inline display helpers
//
// Kept private here rather than in a shared `…Display.swift` file — invitations
// have a single consumer today (extract only if a second view appears, the way
// the asset-lock display file was extracted for its multi-view duplication).

private extension InvitationsView {
    func formatDash(_ duffs: Int64) -> String {
        String(format: "%.8f DASH", Double(duffs) / 100_000_000)
    }

    /// `<txid display hex>:<vout>` → `<first8>…<last6>:<vout>` for compact rows.
    func shortOutPoint(_ hex: String) -> String {
        guard let colon = hex.lastIndex(of: ":") else { return hex }
        let txid = hex[hex.startIndex..<colon]
        let vout = hex[colon...]
        guard txid.count > 14 else { return hex }
        return "\(txid.prefix(8))…\(txid.suffix(6))\(vout)"
    }

    func expiryText(_ expiryUnix: Int) -> String {
        let now = Int(Date().timeIntervalSince1970)
        if now > expiryUnix { return "Expired" }
        let date = Date(timeIntervalSince1970: TimeInterval(expiryUnix))
        return "Expires \(date.formatted(.relative(presentation: .named)))"
    }

    @ViewBuilder
    func statusBadge(_ statusRaw: Int) -> some View {
        let info = statusLabel(statusRaw)
        Text(info.label)
            .font(.caption2.weight(.semibold))
            .padding(.horizontal, 8)
            .padding(.vertical, 2)
            .background(info.color.opacity(0.15))
            .foregroundColor(info.color)
            .clipShape(Capsule())
    }

    /// Maps the status discriminant to a label. An unknown value falls back to
    /// an explicit "Unknown" — the Swift `Int` side has no compiler
    /// exhaustiveness (unlike the wildcard-free Rust `status_to_u8`).
    func statusLabel(_ statusRaw: Int) -> (label: String, color: Color) {
        switch statusRaw {
        case 0: return ("Created", .blue)
        case 1: return ("Claimed", .green)
        case 2: return ("Reclaimed", .orange)
        default: return ("Unknown", .gray)
        }
    }
}
