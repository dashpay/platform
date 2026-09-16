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
/// UI source of truth here (no Rust re-emit). A typed or legacy already-consumed
/// error is not success evidence: it can be an unauthenticated Platform report,
/// so the row and in-flight marker remain unchanged until another source proves
/// the outcome.
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
        // Swipe-dismissal must be gated like the Cancel button: dismissing
        // mid-consume leaves the unstructured task mutating the row after the
        // sheet is gone, and re-opening the still-Created row would permit a
        // second overlapping consume attempt against one irreversible voucher.
        .interactiveDismissDisabled(isReclaiming)
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
                // key pre-persist). The marker does NOT attribute a later "already
                // consumed" rejection (the invitee can race our crash-interrupted
                // consume); it remains crash evidence while the operation outcome
                // is unknown. Setting it earlier would let a purely local failure
                // leave a misleading recovery marker.
                // `hadPriorReclaimInFlight` captures the PERSISTED prior value first.
                // The save must SUCCEED before the consume may run: an unpersisted
                // marker followed by a consume + crash would strand the row (a local
                // "is not tracked" retry classifies as an error, and a Platform
                // "already consumed" as consumption unknown). On a failed save the
                // in-memory flag is rolled back so an unrelated later save can't
                // leak a marker that never reached disk, and the throw aborts the
                // reclaim before anything irreversible.
                // `@MainActor` because a nested func is nonisolated by default in
                // Swift 6 and this touches main-actor `invitation`/`modelContext`.
                @MainActor func markInFlight() throws {
                    hadPriorReclaimInFlight = invitation.reclaimInFlight
                    invitation.reclaimInFlight = true
                    do {
                        try modelContext.save()
                    } catch {
                        invitation.reclaimInFlight = hadPriorReclaimInFlight
                        throw error
                    }
                }

                switch target {
                case .topUp:
                    guard let identityId = selectedIdentityId else {
                        errorMessage = "Pick an identity to top up."
                        return
                    }
                    try markInFlight()
                    _ = try await wallet.resumeTopUpWithAssetLock(
                        identityId: identityId,
                        outPointTxid: txid,
                        outPointVout: vout,
                        // Reclaim IS the explicitly authorized voucher-consume
                        // path; generic resume/top-up flows are refused
                        // invitation locks by the Rust funding resolver.
                        consumeInvitationVoucher: true
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
                    try markInFlight()
                    _ = try await wallet.resumeIdentityWithAssetLock(
                        outPointTxid: txid,
                        outPointVout: vout,
                        identityIndex: identityIndex,
                        identityPubkeys: keys,
                        signer: signer,
                        // Reclaim IS the explicitly authorized voucher-consume
                        // path (see topUp arm above).
                        consumeInvitationVoucher: true
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
                case .consumptionUnknown:
                    // Code 24 can mean either a retained local tombstone or an
                    // unauthenticated Platform report. Retain both status and
                    // the in-flight marker because this reclaim's completion
                    // cannot be inferred safely.
                    errorMessage =
                        "This asset lock was reported as already used, but the "
                        + "wallet could not verify whether this reclaim completed. "
                        + "Sync and check the selected target before retrying."
                case .untrackedAfterOwnAttempt:
                    // The wallet no longer tracks the voucher lock and our own
                    // attempt was in flight — consistent with that attempt's
                    // consume having landed, but there is no on-chain proof the
                    // voucher was consumed at all, so neither the status nor the
                    // marker moves. Explicitly ambiguous by design.
                    errorMessage =
                        "This voucher is no longer tracked by the wallet after an "
                        + "earlier interrupted reclaim attempt. It may already have "
                        + "been consumed by that attempt — check the balance of the "
                        + "identity you targeted then before retrying."
                case .error:
                    if Self.shouldClearInFlightMarker(
                        error: error,
                        hadPriorReclaimInFlight: hadPriorReclaimInFlight
                    ) {
                        // This attempt set the marker itself and then failed the
                        // LOCAL pre-broadcast resume guard — the consume never
                        // started, so the marker is demonstrably stale. Clear it,
                        // or an identical retry would capture it as "prior in
                        // flight" and misreport the same local failure as a
                        // completed self-reclaim.
                        invitation.reclaimInFlight = false
                        try? modelContext.save()
                    }
                    // Otherwise leave the marker as-is: with a prior in-flight
                    // (or any error that may have reached the network) a later
                    // retry that hits "already consumed" must still classify as
                    // ambiguous rather than a provable foreign claim.
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

    /// The state a failed reclaim attempt resolves to.
    ///
    /// Code 24 and the legacy consensus wording share one conservative outcome:
    /// neither distinguishes a local tombstone from an unauthenticated remote
    /// report, so the specific operation's completion remains unknown.
    enum ReclaimOutcome: Equatable {
        /// The lock was reported as consumed, but neither consumption nor this
        /// reclaim's completion is authenticated.
        case consumptionUnknown
        /// The wallet no longer tracks the voucher lock and our own attempt
        /// was in flight — consistent with that attempt's consume having
        /// landed, but with no on-chain proof of consumption at all. Leave
        /// status and marker untouched; surface the ambiguity.
        case untrackedAfterOwnAttempt
        /// An uncertain, unrelated failure — leave state as-is.
        case error
    }

    /// Pure decision for the reclaim `catch`. A typed wallet
    /// `assetLockAlreadyConsumed` and the legacy consensus wording are both
    /// consumption-unknown signals. Kept side-effect-free and `nonisolated` so
    /// it is the unit-tested seam for all outcomes.
    nonisolated static func classifyReclaimFailure(
        error: Error,
        hadPriorReclaimInFlight: Bool
    ) -> ReclaimOutcome {
        if isAlreadyConsumed(error) {
            return .consumptionUnknown
        }
        // A retry after our own crash-interrupted consume can also fail
        // LOCALLY ("…is not tracked") — before Platform, so `isAlreadyConsumed`
        // never sees it. That is consistent with our consume having landed, but
        // is not proof (local tracking state is not an on-chain observation),
        // so it gets its own explicitly ambiguous outcome instead of a
        // Reclaimed recovery. A first attempt (`hadPriorReclaimInFlight ==
        // false`) hitting "not tracked" still resolves to `.error`.
        if hadPriorReclaimInFlight && isLockNoLongerTracked(error) {
            return .untrackedAfterOwnAttempt
        }
        return .error
    }

    /// Whether the `.error` outcome should also CLEAR the persisted
    /// `reclaimInFlight` marker. True only when this attempt set the marker
    /// itself (`hadPriorReclaimInFlight == false`) and then failed the LOCAL
    /// pre-broadcast "is not tracked" resume guard — proof the consume never
    /// started, so the freshly-set marker is stale. Without clearing it, an
    /// identical retry captures the marker as a prior in-flight and degrades
    /// the same purely-local failure into the ambiguous
    /// `.untrackedAfterOwnAttempt` outcome (two-attempt false ambiguity).
    /// Every other error keeps the marker: a failure that may have reached
    /// the network must keep a later "already consumed" classified as
    /// ambiguous rather than a provable foreign claim.
    nonisolated static func shouldClearInFlightMarker(
        error: Error,
        hadPriorReclaimInFlight: Bool
    ) -> Bool {
        !hadPriorReclaimInFlight && isLockNoLongerTracked(error)
    }

    /// Whether an error is the wallet's LOCAL "asset lock is no longer tracked"
    /// resume-guard failure (`rs-platform-wallet` `resume_asset_lock`, Display
    /// "Asset lock … is not tracked"). Distinct from the network's already-consumed
    /// rejection; used only for our-own-reclaim crash recovery above.
    nonisolated static func isLockNoLongerTracked(_ error: Error) -> Bool {
        isLockNoLongerTracked(message: error.localizedDescription)
    }

    nonisolated static func isLockNoLongerTracked(message: String) -> Bool {
        message.lowercased().contains("is not tracked")
    }

    /// Whether an error is the typed signal or legacy wording for consensus
    /// code 10504. The SDK surfaces a legacy consensus
    /// error as `"SDK error: Protocol error: <consensus Display verbatim>"`, so
    /// the canonical Display of
    /// `IdentityAssetLockTransactionOutPointAlreadyConsumedError` —
    /// "Asset lock transaction … already completely used" — appears verbatim.
    /// Matched on that exact phrase ONLY: broader phrases like "already consumed"
    /// never occur in the real Display and would only widen false-positive risk.
    /// Neither form authenticates consumption or proves this reclaim completed;
    /// the typed FFI result is primary and wording is compatibility-only.
    nonisolated static func isAlreadyConsumed(_ error: Error) -> Bool {
        if let walletError = error as? PlatformWalletError,
           case .assetLockAlreadyConsumed = walletError {
            return true
        }
        return isAlreadyConsumed(message: error.localizedDescription)
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
