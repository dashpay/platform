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
                    // Gated while a reclaim is in flight: dismissing mid-flight
                    // would leave the fire-and-forget Task to mutate the row and
                    // save after the sheet is gone, and a re-open could launch an
                    // overlapping reclaim. Mirrors the Reclaim submit gate.
                    Button("Cancel") { dismiss() }
                        .disabled(isReclaiming)
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
        // Whether a *previous* reclaim attempt was already in flight when this
        // one started. Captured before the marker is set below so the catch can
        // read it (a `do`-local binding wouldn't be visible there).
        var hadPriorReclaimInFlight = false
        Task { @MainActor in
            defer { isReclaiming = false }
            do {
                guard let wallet = walletManager.wallet(for: walletId) else {
                    errorMessage = "No wallet loaded."
                    return
                }
                let (txid, vout) = try outPointParts()

                // Persist the in-flight marker ONLY immediately before the on-chain
                // consume — never before pre-broadcast local work (e.g. register's
                // key pre-persist). A crash between the consume and the terminal
                // save is then recoverable: a later "already consumed" that finds
                // this marker set was *our own* reclaim, not a foreign claim. Setting
                // it earlier would let a purely local failure leave the marker set,
                // misclassifying a subsequent genuine foreign claim as self-reclaim.
                // `hadPriorReclaimInFlight` captures the PERSISTED prior value first.
                // `@MainActor` because a nested func is nonisolated by default in
                // Swift 6 and this touches main-actor `invitation`/`modelContext`.
                @MainActor func markInFlight() {
                    hadPriorReclaimInFlight = invitation.reclaimInFlight
                    invitation.reclaimInFlight = true
                    try? modelContext.save()
                }

                switch target {
                case .topUp:
                    guard let identityId = selectedIdentityId else {
                        errorMessage = "Pick an identity to top up."
                        return
                    }
                    markInFlight()
                    _ = try await wallet.topUpIdentityWithExistingAssetLock(
                        outPointTxid: txid,
                        outPointVout: vout,
                        identityId: identityId
                    )
                case .register:
                    let signer = KeychainSigner(modelContainer: modelContext.container)
                    let identityIndex = nextUnusedIdentityIndex()
                    // Pre-broadcast local work — BEFORE the marker is set, so a
                    // failure here leaves no in-flight marker.
                    let keys = try wallet.prePersistIdentityKeysForRegistration(
                        identityIndex: identityIndex,
                        keyCount: Self.authKeyCount,
                        network: network
                    )
                    markInFlight()
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
                invitation.reclaimInFlight = false
                invitation.updatedAt = Date()
                try? modelContext.save()
                dismiss()
            } catch {
                switch Self.classifyReclaimFailure(
                    error: error,
                    hadPriorReclaimInFlight: hadPriorReclaimInFlight
                ) {
                case .reclaimed:
                    // Our own earlier reclaim landed on-chain but crashed before
                    // saving the terminal status. Recover it as Reclaimed and clear
                    // the marker.
                    invitation.statusRaw = 2
                    invitation.reclaimInFlight = false
                    invitation.updatedAt = Date()
                    try? modelContext.save()
                    infoMessage = "This invitation was already reclaimed."
                case .claimed:
                    // Someone else claimed the voucher first. Reflect the terminal
                    // state with a neutral message (the claimant is intentionally
                    // not named).
                    invitation.statusRaw = 1
                    invitation.updatedAt = Date()
                    try? modelContext.save()
                    infoMessage = "This invitation was already claimed."
                case .error:
                    // Uncertain outcome: leave the in-flight marker as-is so a later
                    // retry that hits "already consumed" classifies it as our own
                    // reclaim rather than a foreign claim.
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

    /// The terminal state a reclaim attempt resolves to.
    enum ReclaimOutcome: Equatable {
        /// The voucher was consumed by our own earlier (crash-interrupted) reclaim.
        case reclaimed
        /// The voucher was consumed by the invitee's claim.
        case claimed
        /// An uncertain, non-"already-consumed" failure — leave state as-is.
        case error
    }

    /// Pure decision for the reclaim `catch`: an "already consumed" rejection is
    /// disambiguated by whether *our own* reclaim was already in flight when this
    /// attempt started (persisted `reclaimInFlight` marker). Kept side-effect-free
    /// and `nonisolated` so it is the unit-tested seam for all three outcomes; the
    /// view maps the outcome to `statusRaw`/message/save.
    nonisolated static func classifyReclaimFailure(
        error: Error,
        hadPriorReclaimInFlight: Bool
    ) -> ReclaimOutcome {
        guard isAlreadyConsumed(error) else { return .error }
        return hadPriorReclaimInFlight ? .reclaimed : .claimed
    }

    /// Whether an error is the deterministic "asset lock outpoint already
    /// consumed" rejection (consensus code 10504). The SDK surfaces a consensus
    /// error as `"SDK error: Protocol error: <consensus Display verbatim>"`, so
    /// the canonical Display of
    /// `IdentityAssetLockTransactionOutPointAlreadyConsumedError` —
    /// "Asset lock transaction … already completely used" — appears verbatim.
    /// Matched on that exact phrase ONLY: broader phrases like "already consumed"
    /// never occur in the real Display and would only widen false-positive risk
    /// (misclassifying an unrelated failure as a benign "already claimed", which
    /// would wrongly flip the row to Claimed). A typed FFI result code is the
    /// robust long-term fix.
    nonisolated static func isAlreadyConsumed(_ error: Error) -> Bool {
        isAlreadyConsumed(message: error.localizedDescription)
    }

    /// Pure classifier over the surfaced error message — the testable seam for
    /// the false-positive-safety unit test. `nonisolated` so the test (and any
    /// caller) can invoke it off the main actor; it touches no view state.
    nonisolated static func isAlreadyConsumed(message: String) -> Bool {
        message.lowercased().contains("already completely used")
    }

    private func formatDash(_ duffs: Int64) -> String {
        String(format: "%.8f DASH", Double(duffs) / 100_000_000)
    }
}
