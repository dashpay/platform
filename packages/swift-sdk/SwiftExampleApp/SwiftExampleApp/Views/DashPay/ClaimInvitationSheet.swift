import SwiftDashSDK
import SwiftData
import SwiftUI

/// Claim a DashPay invitation (DIP-13): register a NEW identity for the invitee,
/// funded by the imported voucher, then optionally send a contact request back
/// to the inviter.
///
/// The invitee pastes an invitation link; a read-only preview (sender, if the
/// link carries one) is shown before they commit — the amount isn't in the link
/// and is only known once the funding tx is fetched during the claim. Claiming
/// derives the invitee's own
/// identity keys — including a DashPay Encryption/Decryption pair so the new
/// identity can send the contact request back — exactly like a normal
/// registration, but funds the identity from the voucher instead of a wallet
/// UTXO. Mirrors `AddViaQRSheet`'s paste + async + `KeychainSigner` idiom.
struct ClaimInvitationSheet: View {
    /// The wallet the new identity is registered under (must be loaded).
    let walletId: Data
    /// Network the identity is registered on.
    let network: Network

    @EnvironmentObject private var walletManager: PlatformWalletManager
    @EnvironmentObject private var appUIState: AppUIState
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    /// Existing identities on this network: a claim the wallet refused as
    /// already this wallet's continues with the listed identity.
    @Query private var identities: [PersistentIdentity]

    @State private var uri: String
    @State private var preview: ManagedPlatformWallet.InvitationPreview?
    @State private var isClaiming = false
    @State private var errorMessage: String?
    @State private var contactPrompt: ContactPrompt?
    /// The wallet's lease on this claim's identity slot, from the moment it
    /// is acquired until the claim lands. Released when the sheet closes
    /// (see `releaseClaimSlot`).
    @State private var claimSlot: ManagedPlatformWallet.InvitationClaimSlot?

    /// Post-claim "establish contact with <sender>?" prompt payload. Carries the
    /// inviter's DPNS username (`du`) rather than an identity id — the legacy link
    /// has no id, so it is resolved via DPNS when the request is actually sent.
    private struct ContactPrompt: Identifiable {
        let id = UUID()
        let newIdentityId: Identifier
        let username: String
    }

    /// Default identity auth-key count (mirrors `CreateIdentityView`). The
    /// DashPay enc/dec pair is appended at ids `authKeyCount` / `authKeyCount+1`.
    private static let authKeyCount: UInt32 = 4

    init(walletId: Data, network: Network, initialURI: String = "") {
        self.walletId = walletId
        self.network = network
        _uri = State(initialValue: initialURI)
    }

    var body: some View {
        NavigationStack {
            Form {
                inputSection
                if let preview {
                    previewSection(preview)
                }
                if let errorMessage {
                    Section {
                        Text(errorMessage).font(.caption).foregroundColor(.red)
                    }
                }
            }
            .navigationTitle("Claim Invitation")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    // Gated while a claim is in flight: dismissing mid-claim
                    // would leave the unstructured task to finish (and show its
                    // contact prompt) behind a gone sheet, and a re-open could
                    // start an overlapping claim of the same invitation.
                    // Mirrors ReclaimInvitationSheet.
                    Button("Cancel") { dismiss() }
                        .disabled(isClaiming)
                }
                ToolbarItem(placement: .confirmationAction) {
                    if isClaiming {
                        ProgressView()
                    } else {
                        Button("Claim") { claim() }
                            .disabled(!canClaim)
                            .accessibilityIdentifier("dashpay.invite.claim.submit")
                    }
                }
            }
            .onChange(of: uri) { _, _ in refreshPreview() }
            .onAppear { refreshPreview() }
            .onDisappear { releaseClaimSlot() }
            // Swipe-to-dismiss is gated for the same reason as Cancel above.
            .interactiveDismissDisabled(isClaiming)
            .alert(
                contactPrompt.map { "Add \($0.username)?" } ?? "",
                isPresented: Binding(
                    get: { contactPrompt != nil },
                    set: { if !$0 { contactPrompt = nil } }
                ),
                presenting: contactPrompt
            ) { prompt in
                Button("Add") { sendContact(prompt) }
                Button("Not now", role: .cancel) { dismiss() }
            } message: { _ in
                Text("Send a contact request to the person who invited you.")
            }
        }
    }

    private var trimmedURI: String {
        uri.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private var canClaim: Bool {
        guard let preview, !isClaiming, !trimmedURI.isEmpty else { return false }
        // The legacy link carries only the funding txid — the amount, proof type,
        // and (for a chainlock invite) the instant lock are resolved at claim time
        // by fetching the tx on-chain. So the only pre-claim gate is a
        // structurally valid link; instant-vs-chainlock and the exact amount are
        // no longer known here, and there is no expiry to check.
        return preview.structurallyValid
    }

    // MARK: - Sections

    @ViewBuilder private var inputSection: some View {
        Section {
            TextField("dashpay://invite?…", text: $uri, axis: .vertical)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                .lineLimit(2...4)
                .disabled(isClaiming)
                .accessibilityIdentifier("dashpay.invite.claim.uriField")
        } header: {
            Text("Paste an invitation link")
        } footer: {
            Text("From a friend's “Invite a friend”. It funds a brand-new identity for you.")
        }
    }

    @ViewBuilder
    private func previewSection(_ p: ManagedPlatformWallet.InvitationPreview) -> some View {
        Section("Invitation") {
            if !p.structurallyValid {
                Label("This link isn't a valid invitation.", systemImage: "xmark.octagon")
                    .foregroundColor(.red)
            } else {
                // The amount isn't in the link — it's read from the funding tx
                // during the claim — so show a placeholder until then.
                LabeledContent("Amount", value: "—")
                if p.hasInviter, let name = p.inviterUsername {
                    LabeledContent("From", value: name)
                }
            }
        }
    }

    // MARK: - Actions

    private func refreshPreview() {
        let trimmed = trimmedURI
        guard !trimmed.isEmpty else {
            preview = nil
            return
        }
        guard let wallet = walletManager.wallet(for: walletId) else {
            errorMessage = "No wallet loaded."
            return
        }
        // A malformed link surfaces as `structurallyValid == false` (not a
        // throw); a genuine parse error leaves the preview nil.
        preview = try? wallet.parseInvitation(uri: trimmed)
    }

    private func claim() {
        guard canClaim, !isClaiming, let preview else { return }
        // Freeze the URI alongside the already-frozen `preview` so the claim
        // submits exactly what was previewed (the field is also disabled while
        // claiming, so this is belt-and-suspenders coherence).
        let submittedURI = trimmedURI
        isClaiming = true
        // Published so DashPayTabView defers a second invite link instead of
        // re-presenting (and thereby recreating) this sheet mid-claim.
        appUIState.invitationClaimInFlight = true
        errorMessage = nil
        Task { @MainActor in
            defer {
                isClaiming = false
                appUIState.invitationClaimInFlight = false
            }
            do {
                guard let wallet = walletManager.wallet(for: walletId) else {
                    errorMessage = "No wallet loaded."
                    return
                }
                let signer = KeychainSigner(modelContainer: modelContext.container)
                // The wallet leases the slot the claim's keys are derived at;
                // a retry of this invitation gets the same slot back.
                var slot = try await wallet.acquireInvitationClaimSlot(uri: submittedURI)
                // A lease this sheet held for another invitation (the URI was
                // edited after a refused claim) is given back first: nothing
                // else would ever release it. A refusal means it may have
                // reached Platform, and its outcome frees it.
                if let previous = claimSlot, previous.invitationOutPoint != slot.invitationOutPoint {
                    try? await wallet.releaseInvitationClaimSlot(previous)
                }
                claimSlot = slot
                var slotTakenRetries = 0
                var newIdentityId: Identifier?
                while newIdentityId == nil {
                    do {
                        guard let claimed = try await claim(
                            submittedURI, at: slot.identityIndex, in: wallet, signer: signer
                        ) else { return }
                        newIdentityId = claimed
                    } catch PlatformWalletError.identitySlotTakenOnChain
                        where slotTakenRetries < ManagedPlatformWallet.identitySlotTakenRetryLimit {
                        // The keys at this slot belong to this wallet's own
                        // identity: the wallet recorded it there and released
                        // the lease; nothing was sent. Claim at the next slot.
                        slotTakenRetries += 1
                        claimSlot = nil
                        slot = try await wallet.acquireInvitationClaimSlot(uri: submittedURI)
                        claimSlot = slot
                    }
                }
                guard let newIdentityId else { return }
                // A landed claim's identity holds the slot now.
                claimSlot = nil
                kickDashPaySync(walletManager)

                // Offer the contact-bootstrap when the link carried an inviter
                // (its username); otherwise the claim is done. The inviter's
                // identity id is resolved from the username via DPNS when the
                // request is sent, not from the link.
                if preview.hasInviter, let username = preview.inviterUsername {
                    contactPrompt = ContactPrompt(
                        newIdentityId: newIdentityId,
                        username: username
                    )
                } else {
                    dismiss()
                }
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    /// Claim the invitation `uri` with keys derived at `identityIndex`, the
    /// slot the wallet leased to it. Returns the new identity's id, or `nil`
    /// after surfacing a refusal in `errorMessage`.
    private func claim(
        _ uri: String,
        at identityIndex: UInt32,
        in wallet: ManagedPlatformWallet,
        signer: KeychainSigner
    ) async throws -> Identifier? {
        // Register the invitee's own keys exactly like a normal
        // registration: master + auth keys, plus a DashPay enc/dec pair
        // so the new identity can send the contact request back.
        var keys = try wallet.prePersistIdentityKeysForRegistration(
            identityIndex: identityIndex,
            keyCount: Self.authKeyCount,
            network: network
        )
        keys.append(contentsOf: try IdentityRegistrationKeys.makeDashpayKeyPair(
            managedWallet: wallet,
            walletId: walletId,
            identityIndex: identityIndex,
            firstKeyId: Self.authKeyCount,
            network: network
        ))

        do {
            let managed = try await wallet.claimInvitation(
                uri: uri,
                identityIndex: identityIndex,
                identityPubkeys: keys,
                signer: signer,
                nowUnix: UInt32(Date().timeIntervalSince1970)
            )
            return try managed.getId()
        } catch PlatformWalletError.invitationClaimedByThisWallet(let claimedId) {
            // The invitation's identity is this wallet's. When the wallet
            // already lists it, the claim continues with it; otherwise a
            // rescan loads it.
            guard let recorded = identities.first(where: {
                $0.identityIdBase58 == claimedId && $0.wallet?.walletId == walletId
            }) else {
                errorMessage = "This invitation already created identity \(claimedId) "
                    + "in this wallet. Run Re-scan for Identities to load it."
                return nil
            }
            return recorded.identityId
        }
    }

    private func sendContact(_ prompt: ContactPrompt) {
        Task { @MainActor in
            guard let wallet = walletManager.wallet(for: walletId) else {
                dismiss()
                return
            }
            do {
                // The legacy link carries only the inviter's username, not their
                // identity id — resolve it via DPNS before sending the request
                // (mirrors Android's identityRepository.getUser(invite.user)).
                guard let inviterId = try await wallet.resolveDpnsName(prompt.username) else {
                    errorMessage = "Identity claimed, but \(prompt.username) couldn't be found to add."
                    return
                }
                let signer = KeychainSigner(modelContainer: modelContext.container)
                _ = try await wallet.sendContactRequest(
                    senderIdentityId: prompt.newIdentityId,
                    recipientIdentityId: inviterId,
                    signer: signer
                )
                kickDashPaySync(walletManager)
                dismiss()
            } catch {
                // The identity is already registered; a failed contact request
                // is non-fatal. Surface it but keep the sheet so the user sees
                // the outcome. One that may have gone through (18, 20), or that
                // the wallet refused while an earlier one awaits its outcome
                // (76), is not a failure: the sync settles it.
                switch ContactRequestSubmissionOutcome.of(error) {
                case .mayHaveGoneThrough, .publicRequestUnconfirmed:
                    kickDashPaySync(walletManager)
                    errorMessage = "Identity claimed. "
                        + ContactRequestSubmissionOutcome.publicRequestUnconfirmedMessage
                default:
                    errorMessage = "Identity claimed, but the contact request failed: \(error.localizedDescription)"
                }
            }
        }
    }

    /// Give up the claim's slot when the sheet closes before the claim
    /// landed. The wallet refuses while the claim may have reached Platform
    /// (its outcome frees the slot then); that refusal keeps the slot, so it
    /// is ignored here like any other failure.
    private func releaseClaimSlot() {
        guard let slot = claimSlot, let wallet = walletManager.wallet(for: walletId) else {
            return
        }
        claimSlot = nil
        Task {
            try? await wallet.releaseInvitationClaimSlot(slot)
        }
    }
}
