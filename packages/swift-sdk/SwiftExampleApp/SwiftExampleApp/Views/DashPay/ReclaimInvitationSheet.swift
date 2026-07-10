import SwiftDashSDK
import SwiftData
import SwiftUI

/// Reclaim an unclaimed DashPay invitation (DIP-13): the inviter consumes the
/// still-unclaimed voucher into a Platform identity of their own, recovering the
/// value as **identity credits**. The invitation's DASH was burned into an
/// `OP_RETURN` at create time, so there is nothing on L1 to spend back — reclaim
/// is mechanically "claim your own invitation". The inviter picks a target at
/// reclaim time: top up an existing identity, or register a new one funded by
/// the voucher.
///
/// On success the row's `statusRaw` flips to Reclaimed locally — SwiftData is the
/// UI source of truth here (no Rust re-emit). If the voucher was already consumed
/// (the invitee claimed it), the reclaim is rejected deterministically and the
/// row flips to Claimed with a neutral message instead.
struct ReclaimInvitationSheet: View {
    let invitation: PersistentInvitation
    let walletId: Data
    let network: Network

    @EnvironmentObject private var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    @Query private var identities: [PersistentIdentity]

    @State private var target: Target = .topUp
    @State private var selectedIdentityId: Data?
    @State private var isReclaiming = false
    @State private var errorMessage: String?
    @State private var infoMessage: String?

    private enum Target: Hashable {
        case topUp
        case register
    }

    /// Default identity auth-key count for the register arm (mirrors
    /// `CreateIdentityView` / `ClaimInvitationSheet`). No DashPay enc/dec pair is
    /// appended — a reclaim recovers funds into a fresh identity and sends no
    /// contact request.
    private static let authKeyCount: UInt32 = 4

    init(invitation: PersistentInvitation, walletId: Data, network: Network) {
        self.invitation = invitation
        self.walletId = walletId
        self.network = network
        let raw = network.rawValue
        _identities = Query(
            filter: #Predicate<PersistentIdentity> { $0.networkRaw == raw }
        )
    }

    var body: some View {
        NavigationStack {
            Form {
                explainerSection
                targetSection
                if let infoMessage {
                    Section {
                        Text(infoMessage).font(.caption).foregroundColor(.secondary)
                    }
                }
                if let errorMessage {
                    Section {
                        Text(errorMessage).font(.caption).foregroundColor(.red)
                    }
                }
            }
            .navigationTitle("Reclaim Invitation")
            .navigationBarTitleDisplayMode(.inline)
            .onAppear {
                if selectedIdentityId == nil {
                    selectedIdentityId = walletIdentities.first?.identityId
                }
            }
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    if isReclaiming {
                        ProgressView()
                    } else {
                        Button("Reclaim") { reclaim() }
                            .disabled(!canReclaim)
                            .accessibilityIdentifier("dashpay.invite.reclaim.submit")
                    }
                }
            }
        }
    }

    // MARK: - Sections

    @ViewBuilder private var explainerSection: some View {
        Section {
            LabeledContent("Amount", value: formatDash(invitation.amountDuffs))
        } footer: {
            Text(
                "Recovers this unclaimed invitation's value as identity credits — "
                    + "not spendable Dash. The original amount was burned when the "
                    + "invitation was created."
            )
        }
    }

    @ViewBuilder private var targetSection: some View {
        Section {
            Picker("Recover into", selection: $target) {
                Text("Existing identity").tag(Target.topUp)
                Text("New identity").tag(Target.register)
            }
            .pickerStyle(.segmented)
            .accessibilityIdentifier("dashpay.invite.reclaim.target")

            switch target {
            case .topUp:
                if walletIdentities.isEmpty {
                    Text("No identities on this wallet yet — register a new one instead.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                } else {
                    Picker("Identity", selection: $selectedIdentityId) {
                        ForEach(walletIdentities, id: \.identityId) { identity in
                            Text(identity.identityIdBase58.prefix(12) + "…")
                                .tag(Optional(identity.identityId))
                        }
                    }
                    .accessibilityIdentifier("dashpay.invite.reclaim.identityPicker")
                }
            case .register:
                Text("A brand-new identity funded by this voucher.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        } header: {
            Text("Recover into")
        }
    }

    // MARK: - Derived state

    /// Identities owned by this wallet on this network (the topup targets).
    private var walletIdentities: [PersistentIdentity] {
        identities.filter { $0.wallet?.walletId == walletId }
    }

    private var canReclaim: Bool {
        guard !isReclaiming else { return false }
        switch target {
        case .topUp:
            return selectedIdentityId != nil
        case .register:
            return true
        }
    }

    // MARK: - Actions

    private func reclaim() {
        guard canReclaim, !isReclaiming else { return }
        isReclaiming = true
        errorMessage = nil
        infoMessage = nil
        Task { @MainActor in
            defer { isReclaiming = false }
            do {
                guard let wallet = walletManager.wallet(for: walletId) else {
                    errorMessage = "No wallet loaded."
                    return
                }
                let (txid, vout) = try outPointParts()

                switch target {
                case .topUp:
                    guard let identityId = selectedIdentityId else {
                        errorMessage = "Pick an identity to top up."
                        return
                    }
                    _ = try await wallet.topUpIdentityWithExistingAssetLock(
                        outPointTxid: txid,
                        outPointVout: vout,
                        identityId: identityId
                    )
                case .register:
                    let signer = KeychainSigner(modelContainer: modelContext.container)
                    let identityIndex = nextUnusedIdentityIndex()
                    let keys = try wallet.prePersistIdentityKeysForRegistration(
                        identityIndex: identityIndex,
                        keyCount: Self.authKeyCount,
                        network: network
                    )
                    _ = try await wallet.resumeIdentityWithAssetLock(
                        outPointTxid: txid,
                        outPointVout: vout,
                        identityIndex: identityIndex,
                        identityPubkeys: keys,
                        signer: signer
                    )
                }

                // SwiftData is the UI source: flip the local row to Reclaimed.
                invitation.statusRaw = 2
                invitation.updatedAt = Date()
                try? modelContext.save()
                dismiss()
            } catch {
                if isAlreadyConsumed(error) {
                    // Someone already claimed this voucher (or a prior reclaim
                    // consumed it). The consume is deterministically rejected —
                    // no funds are lost. Reflect the terminal state and show a
                    // neutral message (the claimant is intentionally not named).
                    invitation.statusRaw = 1
                    invitation.updatedAt = Date()
                    try? modelContext.save()
                    infoMessage = "This invitation was already claimed."
                } else {
                    errorMessage = error.localizedDescription
                }
            }
        }
    }

    /// Split the stored 36-byte outpoint (`txid_le ‖ vout_le`) into the 32-byte
    /// txid and the little-endian vout. Rebuilt from `rawOutPoint` directly (not
    /// decoded from the display string) to avoid a reverse-parse misalignment.
    private func outPointParts() throws -> (txid: Data, vout: UInt32) {
        let raw = invitation.rawOutPoint
        guard raw.count == 36 else {
            throw PlatformWalletError.invalidParameter(
                "rawOutPoint must be 36 bytes (was \(raw.count))"
            )
        }
        let txid = raw.prefix(32)
        let voutBytes = raw.suffix(4)
        let vout = voutBytes.reversed().reduce(UInt32(0)) { ($0 << 8) | UInt32($1) }
        return (Data(txid), vout)
    }

    /// One past the highest used registration index on this wallet, else 0.
    /// Matches `ClaimInvitationSheet.nextUnusedIdentityIndex`.
    private func nextUnusedIdentityIndex() -> UInt32 {
        let used = walletIdentities.map(\.identityIndex)
        guard let highest = used.max() else { return 0 }
        return highest == UInt32.max ? UInt32.max : highest + 1
    }

    /// Whether an error is the deterministic "asset lock outpoint already
    /// consumed" rejection (consensus code 10504), whose Display is
    /// "Asset lock transaction … already completely used".
    private func isAlreadyConsumed(_ error: Error) -> Bool {
        let text = error.localizedDescription.lowercased()
        return text.contains("already completely used")
            || text.contains("alreadyconsumed")
            || text.contains("already consumed")
    }

    private func formatDash(_ duffs: Int64) -> String {
        String(format: "%.8f DASH", Double(duffs) / 100_000_000)
    }
}
