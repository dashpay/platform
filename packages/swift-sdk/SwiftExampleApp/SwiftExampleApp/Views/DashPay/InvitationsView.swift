import SwiftDashSDK
import SwiftData
import SwiftUI

/// "Sent invitations" list (DIP-13): every invitation this wallet created,
/// newest first. Read-only in commit 1 (reclaim is a follow-up commit). Rows are
/// `PersistentInvitation` records upserted by the `on_persist_invitations_fn`
/// bridge whenever `create_invitation` flushes its changeset.
struct InvitationsView: View {
    let walletId: Data

    @Query private var invitations: [PersistentInvitation]

    init(walletId: Data) {
        self.walletId = walletId
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
                    description: Text("Invitations you create appear here.")
                )
            } else {
                ForEach(invitations) { invitation in
                    row(invitation)
                }
            }
        }
        .navigationTitle("Sent Invitations")
        .navigationBarTitleDisplayMode(.inline)
        .accessibilityIdentifier("dashpay.invitations.list")
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
