import SwiftUI
import SwiftData
import SwiftDashSDK

/// Value-based navigation route to the shielded activity list, mirroring
/// `TransactionsRoute`. Pushed via `NavigationLink(value:)` and resolved
/// by a `.navigationDestination(for:)` in `WalletsContentView`.
struct ShieldedActivityRoute: Hashable {
    let walletId: Data
}

// MARK: - Presentation model

/// Display metadata for a `PersistentShieldedActivity.kindTag`. Kept in
/// one place so the list row, the detail sheet, and the section grouping
/// stay consistent.
enum ShieldedActivityKindDisplay {
    static func label(_ tag: Int) -> String {
        switch tag {
        case 0: return "Shielded"
        case 1: return "Shielded from Asset Lock"
        case 2: return "Received"
        case 3: return "Sent"
        case 4: return "Unshielded"
        case 5: return "Withdrawn"
        case 6: return "Identity Created"
        default: return "Shielded Spend"
        }
    }

    /// SF Symbol per kind.
    static func icon(_ tag: Int) -> String {
        switch tag {
        case 0, 1: return "lock.fill"                 // Shield / ShieldFromAssetLock
        case 2: return "arrow.down.circle.fill"       // Received
        case 3: return "arrow.up.circle.fill"         // Sent
        case 4: return "lock.open.fill"               // Unshield
        case 5: return "arrow.up.right.circle.fill"   // Withdrawal
        case 6: return "person.crop.circle.badge.plus" // IdentityCreate
        default: return "questionmark.circle.fill"     // ShieldedSpend
        }
    }

    /// `true` for the residual `ShieldedSpend` kind, which a restored
    /// wallet couldn't fully classify — the detail sheet shows a footnote.
    static func isUnclassifiedResidual(_ tag: Int) -> Bool { tag == 7 }
}

extension PersistentShieldedActivity {
    /// 1 DASH = 1e11 Platform credits (distinct from Core's 1e8/DASH).
    fileprivate static let creditsPerDash = 100_000_000_000.0

    /// Signed amount string, e.g. `-0.5000 DASH` for an out, `+1.0000`
    /// for an in. `direction`: 0 In, 1 Out, 2 Self.
    var signedAmountText: String {
        let dash = Double(amount) / Self.creditsPerDash
        let sign: String
        switch direction {
        case 0: sign = "+"
        case 1: sign = "-"
        default: sign = ""
        }
        return String(format: "%@%.4f DASH", sign, dash)
    }

    /// Tint for the amount label by direction.
    var amountColor: Color {
        switch direction {
        case 0: return .green
        case 1: return .primary
        default: return .secondary
        }
    }

    /// Decode the 36-byte Dash memo to text when it is a kind-1 (UTF-8
    /// text) memo: 4-byte LE kind tag == 1, then up to 32 bytes of UTF-8
    /// (trailing zeros trimmed). Returns `nil` for an empty / non-text /
    /// undecodable memo.
    var memoText: String? {
        guard memo.count >= 4 else { return nil }
        let bytes = [UInt8](memo)
        let kind = UInt32(bytes[0]) | (UInt32(bytes[1]) << 8)
            | (UInt32(bytes[2]) << 16) | (UInt32(bytes[3]) << 24)
        guard kind == 1 else { return nil }
        let payload = bytes[4...].prefix(while: { $0 != 0 })
        guard !payload.isEmpty, let s = String(bytes: payload, encoding: .utf8) else { return nil }
        return s
    }

    /// Short hex preview of `entryId` for diagnostics.
    var entryIdHexShort: String {
        let hex = entryId.map { String(format: "%02x", $0) }.joined()
        return hex.count > 12 ? String(hex.prefix(12)) + "…" : hex
    }
}

// MARK: - List

/// Per-wallet shielded activity list, grouped by status (Pending on top)
/// then ordered by block height descending. Reads `PersistentShielded
/// Activity` directly from SwiftData (the Rust persister writes the rows;
/// there is no read-side FFI — same pattern as the shielded notes /
/// outgoing-notes UI).
struct ShieldedActivityListView: View {
    let walletId: Data
    @Query private var entries: [PersistentShieldedActivity]
    @State private var selected: PersistentShieldedActivity?

    init(walletId: Data) {
        self.walletId = walletId
        let descriptor = FetchDescriptor<PersistentShieldedActivity>(
            predicate: #Predicate { $0.walletId == walletId },
            sortBy: [
                // Confirmed/failed by height desc; pendings (hasBlockHeight
                // == false, blockHeight 0) and the createdAtMs tiebreak are
                // re-ordered in `sorted` below so pendings float to the top.
                SortDescriptor(\PersistentShieldedActivity.blockHeight, order: .reverse),
                SortDescriptor(\PersistentShieldedActivity.createdAtMs, order: .reverse),
            ]
        )
        _entries = Query(descriptor)
    }

    /// Pending rows first (most recent record time first), then confirmed
    /// / failed by block height descending — mirrors the Rust
    /// `sort_activity_for_display` contract.
    ///
    /// Partition by STATUS, not by `hasBlockHeight`: the live recorders
    /// mark a successful broadcast `Confirmed` with `block_height: None`
    /// (the scan backfills the height later), so a height-less row is
    /// the COMMON success shape — only `status == 0` is actually
    /// pending. Height-less confirmed rows sort by record time among
    /// the settled (they're the newest).
    private var pending: [PersistentShieldedActivity] {
        entries
            .filter { $0.status == 0 }
            .sorted { $0.createdAtMs > $1.createdAtMs }
    }

    private var settled: [PersistentShieldedActivity] {
        entries
            .filter { $0.status != 0 }
            .sorted {
                // Height-less (just-confirmed) rows float above
                // height-bearing ones, then height desc, then recency.
                let lh = $0.hasBlockHeight ? $0.blockHeight : UInt64.max
                let rh = $1.hasBlockHeight ? $1.blockHeight : UInt64.max
                if lh != rh { return lh > rh }
                return $0.createdAtMs > $1.createdAtMs
            }
    }

    var body: some View {
        List {
            if entries.isEmpty {
                ContentUnavailableView(
                    "No Shielded Activity",
                    systemImage: "lock.rectangle.stack",
                    description: Text("Shielded sends, receives, and other private operations appear here.")
                )
            }
            // Key rows by MODEL identity, not `entryId`: the table is
            // uniqued on (walletId, accountIndex, entryId), so one
            // wallet-level list can legitimately hold the same entryId
            // twice — e.g. an intra-wallet transfer writes a Sent row on
            // the sending account and a Received row on the receiving
            // account for the same visible outputs.
            if !pending.isEmpty {
                Section("Pending") {
                    ForEach(pending, id: \.persistentModelID) { row($0) }
                }
            }
            if !settled.isEmpty {
                Section("History") {
                    ForEach(settled, id: \.persistentModelID) { row($0) }
                }
            }
        }
        .navigationTitle("Shielded Activity")
        .navigationBarTitleDisplayMode(.inline)
        .sheet(item: $selected) { ShieldedActivityDetailView(entry: $0) }
    }

    @ViewBuilder
    private func row(_ entry: PersistentShieldedActivity) -> some View {
        Button {
            selected = entry
        } label: {
            HStack(spacing: 12) {
                Image(systemName: ShieldedActivityKindDisplay.icon(entry.kindTag))
                    .font(.title3)
                    .foregroundColor(.accentColor)
                    .frame(width: 28)
                VStack(alignment: .leading, spacing: 2) {
                    Text(ShieldedActivityKindDisplay.label(entry.kindTag))
                        .font(.subheadline.weight(.medium))
                    if let memo = entry.memoText {
                        Text(memo)
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .lineLimit(1)
                    } else if let height = entry.hasBlockHeight ? entry.blockHeight : nil {
                        Text("Block \(height)")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
                Spacer()
                VStack(alignment: .trailing, spacing: 2) {
                    Text(entry.signedAmountText)
                        .font(.subheadline.weight(.semibold))
                        .foregroundColor(entry.amountColor)
                    // Badge by STATUS — a Confirmed row without a height
                    // (live success awaiting the scan's height backfill)
                    // must not read as Pending.
                    if entry.status == 0 {
                        Text("Pending")
                            .font(.caption2)
                            .foregroundColor(.orange)
                    } else if entry.status == 2 {
                        Text("Failed")
                            .font(.caption2)
                            .foregroundColor(.red)
                    }
                }
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityIdentifier("shieldedActivity.row")
    }
}

// MARK: - Detail

/// Full detail sheet for one activity entry. Shows every field; for
/// `IdentityCreate` the identity id is copyable, and `ShieldedSpend`
/// (restore-path residual) carries a clarifying footnote.
struct ShieldedActivityDetailView: View {
    let entry: PersistentShieldedActivity
    @Environment(\.dismiss) private var dismiss

    private var statusText: String {
        switch entry.status {
        case 0: return "Pending"
        case 1: return "Confirmed"
        default: return "Failed"
        }
    }

    private var directionText: String {
        switch entry.direction {
        case 0: return "In"
        case 1: return "Out"
        default: return "Self"
        }
    }

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    LabeledContent("Type", value: ShieldedActivityKindDisplay.label(entry.kindTag))
                    LabeledContent("Amount", value: entry.signedAmountText)
                    if entry.hasFee {
                        LabeledContent("Fee", value: String(format: "%.6f DASH", Double(entry.fee) / 100_000_000_000.0))
                    }
                    LabeledContent("Status", value: statusText)
                    LabeledContent("Direction", value: directionText)
                    if entry.hasBlockHeight {
                        LabeledContent("Block Height", value: "\(entry.blockHeight)")
                    }
                }

                if let memo = entry.memoText {
                    Section("Memo") {
                        Text(memo).textSelection(.enabled)
                    }
                }

                if entry.kindTag == 6, entry.identityId.count == 32 {
                    Section("Created Identity") {
                        let idHex = entry.identityId.map { String(format: "%02x", $0) }.joined()
                        Text(idHex)
                            .font(.caption.monospaced())
                            .textSelection(.enabled)
                        Button {
                            UIPasteboard.general.string = idHex
                        } label: {
                            Label("Copy Identity ID", systemImage: "doc.on.doc")
                        }
                    }
                } else if !entry.counterparty.isEmpty {
                    Section("Counterparty") {
                        let cpHex = entry.counterparty.map { String(format: "%02x", $0) }.joined()
                        Text(cpHex)
                            .font(.caption.monospaced())
                            .textSelection(.enabled)
                    }
                }

                Section("Details") {
                    LabeledContent("Entry ID", value: entry.entryIdHexShort)
                    let when = Date(timeIntervalSince1970: Double(entry.createdAtMs) / 1000.0)
                    LabeledContent("Recorded", value: when.formatted(date: .abbreviated, time: .shortened))
                    if entry.noteCmxs.count >= 32 {
                        LabeledContent("Visible Outputs", value: "\(entry.noteCmxs.count / 32)")
                    }
                    if entry.spentNullifiers.count >= 32 {
                        LabeledContent("Spent Notes", value: "\(entry.spentNullifiers.count / 32)")
                    }
                }

                if ShieldedActivityKindDisplay.isUnclassifiedResidual(entry.kindTag) {
                    Section {
                        Text("Restored wallets can't always recover the operation type. This entry was reconstructed from on-chain note data.")
                            .font(.footnote)
                            .foregroundColor(.secondary)
                    }
                }
            }
            .navigationTitle("Activity Detail")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }
                }
            }
        }
    }
}
