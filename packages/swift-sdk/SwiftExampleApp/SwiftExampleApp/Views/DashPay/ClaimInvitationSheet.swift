import SwiftDashSDK
import SwiftData
import SwiftUI

/// Claim a DashPay invitation (DIP-13): register a NEW identity for the invitee,
/// funded by the imported voucher, then optionally send a contact request back
/// to the inviter.
///
/// The invitee pastes an invitation link; a read-only preview (amount, sender,
/// expiry) is shown before they commit. Claiming derives the invitee's own
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
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    /// Existing identities on this network — used to pick the next unused
    /// registration index for the new identity.
    @Query private var identities: [PersistentIdentity]

    @State private var uri: String
    @State private var preview: ManagedPlatformWallet.InvitationPreview?
    @State private var isClaiming = false
    @State private var errorMessage: String?
    @State private var contactPrompt: ContactPrompt?

    /// Post-claim "establish contact with <sender>?" prompt payload.
    private struct ContactPrompt: Identifiable {
        let id = UUID()
        let newIdentityId: Identifier
        let inviterId: Identifier
        let username: String
    }

    /// Default identity auth-key count (mirrors `CreateIdentityView`). The
    /// DashPay enc/dec pair is appended at ids `authKeyCount` / `authKeyCount+1`.
    private static let authKeyCount: UInt32 = 4

    init(walletId: Data, network: Network, initialURI: String = "") {
        self.walletId = walletId
        self.network = network
        _uri = State(initialValue: initialURI)
        let raw = network.rawValue
        _identities = Query(
            filter: #Predicate<PersistentIdentity> { $0.networkRaw == raw }
        )
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
                    Button("Cancel") { dismiss() }
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
        return preview.structurallyValid && preview.isInstant && !isExpired(preview)
    }

    private func isExpired(_ p: ManagedPlatformWallet.InvitationPreview) -> Bool {
        UInt32(Date().timeIntervalSince1970) > p.expiryUnix
    }

    // MARK: - Sections

    @ViewBuilder private var inputSection: some View {
        Section {
            TextField("dashpay://invite?data=…", text: $uri, axis: .vertical)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                .lineLimit(2...4)
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
            } else if !p.isInstant {
                Label(
                    "This invitation can't be claimed (missing an InstantSend proof).",
                    systemImage: "xmark.octagon"
                )
                .foregroundColor(.red)
            } else {
                LabeledContent("Amount", value: formatDash(p.amountDuffs))
                if isExpired(p) {
                    Label("Expired — ask the sender for a new link.", systemImage: "clock.badge.xmark")
                        .foregroundColor(.orange)
                }
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
        isClaiming = true
        errorMessage = nil
        Task { @MainActor in
            defer { isClaiming = false }
            do {
                guard let wallet = walletManager.wallet(for: walletId) else {
                    errorMessage = "No wallet loaded."
                    return
                }
                let signer = KeychainSigner(modelContainer: modelContext.container)
                let identityIndex = nextUnusedIdentityIndex()

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

                let managed = try await wallet.claimInvitation(
                    uri: trimmedURI,
                    identityIndex: identityIndex,
                    identityPubkeys: keys,
                    signer: signer,
                    nowUnix: UInt32(Date().timeIntervalSince1970)
                )
                let newIdentityId = try managed.getId()
                kickDashPaySync(walletManager)

                // Offer the contact-bootstrap when the link carried an inviter;
                // otherwise the claim is done.
                if preview.hasInviter,
                   let inviterId = preview.inviterId,
                   let username = preview.inviterUsername {
                    contactPrompt = ContactPrompt(
                        newIdentityId: newIdentityId,
                        inviterId: inviterId,
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

    private func sendContact(_ prompt: ContactPrompt) {
        Task { @MainActor in
            guard let wallet = walletManager.wallet(for: walletId) else {
                dismiss()
                return
            }
            do {
                let signer = KeychainSigner(modelContainer: modelContext.container)
                _ = try await wallet.sendContactRequest(
                    senderIdentityId: prompt.newIdentityId,
                    recipientIdentityId: prompt.inviterId,
                    signer: signer
                )
                kickDashPaySync(walletManager)
                dismiss()
            } catch {
                // The identity is already registered; a failed contact request
                // is non-fatal and re-sendable. Surface it but keep the sheet so
                // the user sees the outcome.
                errorMessage = "Identity claimed, but the contact request failed: \(error.localizedDescription)"
            }
        }
    }

    /// One past the highest used registration index on this wallet, else 0.
    /// Registration keys aren't gap-limited, so "next unused" is `max + 1`.
    private func nextUnusedIdentityIndex() -> UInt32 {
        let used = identities
            .filter { $0.wallet?.walletId == walletId }
            .map(\.identityIndex)
        guard let highest = used.max() else { return 0 }
        return highest == UInt32.max ? UInt32.max : highest + 1
    }

    private func formatDash(_ duffs: UInt64) -> String {
        String(format: "%.8f DASH", Double(duffs) / 100_000_000)
    }
}
