import SwiftDashSDK
import SwiftData
import SwiftUI

/// "Sent invitations" list (DIP-13): every invitation this wallet created,
/// newest first. Rows are `PersistentInvitation` records upserted by the
/// `on_persist_invitations_fn` bridge whenever `create_invitation` flushes its
/// changeset. A still-unclaimed (`Created`) row can be reclaimed — recovering the
/// voucher's value as identity credits — via a swipe action.
struct InvitationsView: View {
    let walletId: Data
    let network: Network
    let identity: PersistentIdentity

    @Query private var invitations: [PersistentInvitation]

    @State private var reclaimTarget: PersistentInvitation?
    @State private var showCreateInvitation = false

    init(walletId: Data, network: Network, identity: PersistentIdentity) {
        self.walletId = walletId
        self.network = network
        self.identity = identity
        _invitations = Query(
            filter: PersistentInvitation.predicate(walletId: walletId),
            sort: [SortDescriptor(\PersistentInvitation.createdAtSecs, order: .reverse)]
        )
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
                Button {
                    showCreateInvitation = true
                } label: {
                    Image(systemName: "plus.circle")
                }
                .accessibilityLabel("Create invitation")
                .accessibilityIdentifier("dashpay.invitations.create")
            }
        }
        .sheet(item: $reclaimTarget) { invitation in
            ReclaimInvitationSheet(
                invitation: invitation,
                walletId: walletId,
                network: network
            )
        }
        .sheet(isPresented: $showCreateInvitation) {
            CreateInvitationSheet(identity: identity)
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
