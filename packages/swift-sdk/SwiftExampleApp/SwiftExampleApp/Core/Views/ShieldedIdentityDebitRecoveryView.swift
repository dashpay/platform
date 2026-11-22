import SwiftUI
import SwiftData
import SwiftDashSDK

// MARK: - Presentation

/// Plain-language text for a durable identity-funded shield (Type 21)
/// record. The status itself is decided in Rust; this only labels it.
enum ShieldedIdentityDebitRecoveryDisplay {
    static func statusTitle(_ status: ShieldedIdentityDebitRecoveryStatus) -> String {
        switch status {
        case .retrying: return "Retrying"
        case .parked: return "Waiting for the note"
        case .unknown: return "Outcome unknown"
        }
    }

    static func statusDetail(_ status: ShieldedIdentityDebitRecoveryStatus) -> String {
        switch status {
        case .retrying:
            return "The wallet resends this signed shield on each shielded "
                + "sync until its outcome is known."
        case .parked:
            return "Its identity nonce is used up, so it either already "
                + "executed or never will. The wallet is waiting for the "
                + "shielded scan to see its note."
        case .unknown:
            return "Automatic retry was stopped. It may still have executed; "
                + "the wallet keeps the record until the scan sees its note."
        }
    }

    /// Only an active record can be abandoned; `unknown` already was.
    static func canStopRetrying(_ status: ShieldedIdentityDebitRecoveryStatus) -> Bool {
        status != .unknown
    }

    /// Identity alias from the wallet's rows when known, else a short id.
    /// A record whose transition could not be read has no identity id.
    static func identityLabel(
        for record: ShieldedIdentityDebitRecoveryRecord,
        identities: [PersistentIdentity]
    ) -> String {
        guard let identityId = record.identityId else { return "Unreadable record" }
        if let identity = identities.first(where: { $0.identityId == identityId }) {
            return identity.displayName
        }
        let hex = identityId.map { String(format: "%02x", $0) }.joined()
        return String(hex.prefix(12)) + "…"
    }

    static func amountText(_ credits: UInt64) -> String {
        String(format: "%.8f DASH", Double(credits) / 100_000_000_000.0)
    }

    static let stopRetryingWarning =
        "This does not cancel the payment. It may already have executed, or "
        + "it may still execute later. The wallet stops resending it, and a "
        + "new shield from this identity may become an additional debit."
}

// MARK: - Model

/// Loads and abandons the per-wallet identity-funded shield records.
/// Failures stay local to this model so the activity list keeps working.
@MainActor
final class ShieldedIdentityDebitRecoveryModel: ObservableObject {
    @Published private(set) var records: [ShieldedIdentityDebitRecoveryRecord] = []
    @Published private(set) var loadError: String?
    @Published var actionError: String?
    @Published private(set) var isAbandoning = false

    func load(walletManager: PlatformWalletManager, walletId: Data) async {
        do {
            records = try await walletManager.shieldedIdentityDebitRecoveryRecords(
                walletId: walletId
            )
            loadError = nil
        } catch {
            records = []
            loadError = error.localizedDescription
        }
    }

    /// Call only after the user confirmed `stopRetryingWarning`.
    func stopRetrying(
        _ record: ShieldedIdentityDebitRecoveryRecord,
        walletManager: PlatformWalletManager,
        walletId: Data
    ) async {
        isAbandoning = true
        defer { isAbandoning = false }
        do {
            try await walletManager.abandonShieldedIdentityDebit(
                walletId: walletId,
                accountIndex: record.accountIndex,
                activityId: record.activityId,
                acknowledgePossibleExecution: true
            )
        } catch {
            actionError = error.localizedDescription
        }
        await load(walletManager: walletManager, walletId: walletId)
    }
}

// MARK: - Section

/// "In-Progress Identity Shields": the durable Type 21 records, shown above
/// Pending so an interrupted shield is visible after a restart. Renders
/// nothing when there are no records and nothing failed.
struct ShieldedIdentityDebitRecoverySection: View {
    @ObservedObject var model: ShieldedIdentityDebitRecoveryModel
    let identities: [PersistentIdentity]
    let isChecking: Bool
    let canCheck: Bool
    let onCheckNow: () -> Void
    let onStopRetrying: (ShieldedIdentityDebitRecoveryRecord) -> Void

    @State private var pendingStop: ShieldedIdentityDebitRecoveryRecord?

    var body: some View {
        if !model.records.isEmpty {
            recordsSection
        } else if let loadError = model.loadError {
            Section {
                Text("In-progress identity shields could not be loaded: \(loadError)")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .accessibilityIdentifier("shieldedActivity.identityShields.loadError")
            }
        }
    }

    private var recordsSection: some View {
        Section {
            ForEach(model.records, id: \.recoveryKey) { record in
                recordRow(record)
            }
            Button {
                onCheckNow()
            } label: {
                HStack {
                    Label("Check Now", systemImage: "arrow.clockwise")
                    if isChecking {
                        Spacer()
                        ProgressView().controlSize(.small)
                    }
                }
            }
            .disabled(isChecking || !canCheck)
            .accessibilityIdentifier("shieldedActivity.identityShields.checkNow")
        } header: {
            Text("In-Progress Identity Shields")
        } footer: {
            Text("Shields from an identity's balance whose outcome is not known yet. A shielded sync resends and resolves them.")
        }
        .confirmationDialog(
            "Stop retrying this shield?",
            isPresented: Binding(
                get: { pendingStop != nil },
                set: { if !$0 { pendingStop = nil } }
            ),
            titleVisibility: .visible,
            presenting: pendingStop
        ) { record in
            Button("Stop Retrying", role: .destructive) {
                onStopRetrying(record)
            }
            .accessibilityIdentifier("shieldedActivity.identityShields.confirmStop")
            Button("Cancel", role: .cancel) {}
        } message: { _ in
            Text(ShieldedIdentityDebitRecoveryDisplay.stopRetryingWarning)
        }
    }

    @ViewBuilder
    private func recordRow(_ record: ShieldedIdentityDebitRecoveryRecord) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Image(systemName: "person.crop.circle.badge.clock")
                    .foregroundColor(.orange)
                Text(ShieldedIdentityDebitRecoveryDisplay.identityLabel(
                    for: record,
                    identities: identities
                ))
                .font(.subheadline.weight(.medium))
                .lineLimit(1)
                .truncationMode(.middle)
                Spacer()
                if let amount = record.amount {
                    Text(ShieldedIdentityDebitRecoveryDisplay.amountText(amount))
                        .font(.subheadline.weight(.semibold))
                }
            }
            Text(ShieldedIdentityDebitRecoveryDisplay.statusTitle(record.status))
                .font(.caption.weight(.medium))
                .foregroundColor(.orange)
            Text(ShieldedIdentityDebitRecoveryDisplay.statusDetail(record.status))
                .font(.caption)
                .foregroundColor(.secondary)
            if ShieldedIdentityDebitRecoveryDisplay.canStopRetrying(record.status) {
                Button("Stop Retrying", role: .destructive) {
                    pendingStop = record
                }
                .buttonStyle(.borderless)
                .font(.caption)
                .disabled(model.isAbandoning)
                .accessibilityIdentifier("shieldedActivity.identityShields.stopRetrying")
            }
        }
        .padding(.vertical, 2)
        .accessibilityIdentifier("shieldedActivity.identityShields.row")
    }
}

private extension ShieldedIdentityDebitRecoveryRecord {
    /// A record is identified by its account and activity id within a wallet.
    var recoveryKey: String {
        "\(accountIndex)-" + activityId.map { String(format: "%02x", $0) }.joined()
    }
}
