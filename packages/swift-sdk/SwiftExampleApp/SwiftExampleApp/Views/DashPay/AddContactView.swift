import SwiftUI
import SwiftData
import SwiftDashSDK

/// Add-contact sheet (restyled from `AddFriendView`).
///
/// Two modes: **Username (DPNS)** with live prefix search, and
/// **Identity ID** with inline base58 validation. Either way the
/// resolved target renders as a preview card that gates "Send
/// Request" (never a dead end: not-found offers
/// clear-and-retry instead of a terminal error).
struct AddContactView: View {
    let identity: PersistentIdentity
    /// Fires after a successful broadcast with the recipient id and
    /// the DPNS name used to find them (nil in ID mode). The tab
    /// root inserts the id into the optimistic-send overlay and
    /// records the DPNS hint.
    let onSent: (Identifier, String?) -> Void

    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.dismiss) private var dismiss
    @Environment(\.modelContext) private var modelContext

    private enum Mode: Hashable {
        case dpns, identityId
    }

    /// DPNS resolution states: typing → searching → not-found →
    /// found. `idle` covers "fewer than 2 characters typed".
    private enum SearchState: Equatable {
        case idle
        case searching
        case notFound
        case found([DpnsSearchResult])
    }

    @State private var mode: Mode = .dpns
    @State private var searchText = ""
    @State private var searchState: SearchState = .idle
    @State private var searchTask: Task<Void, Never>?

    /// DPNS mode: the result row the user picked. Resolution to a
    /// preview card; gates Send.
    @State private var selectedResult: DpnsSearchResult?

    @State private var idText = ""

    /// Optional DIP-15 `encryptedAccountLabel` the sender attaches to the
    /// receiving account they share. The recipient decrypts it and sees it
    /// as the contact's "Their account" hint.
    @State private var accountLabel = ""

    @State private var isSending = false
    @State private var errorMessage: String?

    /// Send-collision flow.
    @State private var showCollisionAlert = false
    @State private var collisionRecipient: Identifier?

    /// Minimum prefix length before firing a search.
    private let minSearchLength = 2
    /// Debounce for the live search.
    private let searchDebounce: Duration = .milliseconds(300)

    // MARK: - Derived

    /// ID mode: parsed base58, nil while invalid.
    ///
    /// The 32-byte length gate is load-bearing: `Data.identifier(fromBase58:)`
    /// decodes partial input to fewer bytes, and a short id reaching
    /// `getDashPayProfile` trips `withFFIBytes`'s 32-byte precondition
    /// (crash observed while typing into this field, 2026-06-12).
    private var parsedIdentityId: Identifier? {
        let trimmed = idText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty,
              let decoded = Data.identifier(fromBase58: trimmed),
              decoded.count == 32
        else { return nil }
        return decoded
    }

    private var resolvedRecipient: Identifier? {
        switch mode {
        case .dpns: return selectedResult?.identityId
        case .identityId: return parsedIdentityId
        }
    }

    private var canSend: Bool {
        resolvedRecipient != nil
            && resolvedRecipient != identity.identityId
            && !isSending
    }

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                Picker("Search by", selection: $mode) {
                    Text("Username (DPNS)").tag(Mode.dpns)
                    Text("Identity ID").tag(Mode.identityId)
                }
                .pickerStyle(.segmented)
                .padding()
                .accessibilityIdentifier("dashpay.addContact.mode")

                Form {
                    switch mode {
                    case .dpns:
                        dpnsSections
                    case .identityId:
                        idSections
                    }

                    if let recipient = resolvedRecipient {
                        previewSection(recipient: recipient)
                        accountLabelSection
                        sendSection
                    }

                    if let errorMessage {
                        Section {
                            Text(errorMessage)
                                .font(.caption)
                                .foregroundColor(.red)
                        }
                    }
                }
            }
            .navigationTitle("Add Contact")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                        .disabled(isSending)
                        .accessibilityIdentifier("dashpay.addContact.cancel")
                }
            }
            .alert(
                "Request already received",
                isPresented: $showCollisionAlert,
                presenting: collisionRecipient
            ) { recipient in
                Button("Accept") {
                    acceptIncoming(from: recipient)
                }
                Button("Continue anyway") {
                    send(to: recipient)
                }
            } message: { _ in
                Text("This person already sent you a request — accept it instead?")
            }
        }
    }

    // MARK: - DPNS mode (§6.4 four-state machine)

    @ViewBuilder
    private var dpnsSections: some View {
        Section {
            HStack(spacing: 8) {
                Image(systemName: "magnifyingglass")
                    .foregroundColor(.secondary)
                TextField("Search usernames", text: $searchText)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .accessibilityIdentifier("dashpay.addContact.input")
                if !searchText.isEmpty {
                    Button {
                        clearSearch()
                    } label: {
                        Image(systemName: "xmark.circle.fill")
                            .foregroundColor(.secondary)
                    }
                    .buttonStyle(.borderless)
                    .accessibilityIdentifier("dashpay.addContact.clear")
                }
            }

            switch searchState {
            case .idle:
                if searchText.trimmingCharacters(in: .whitespacesAndNewlines).count < minSearchLength {
                    Text("Type at least \(minSearchLength) characters to search.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            case .searching:
                HStack(spacing: 10) {
                    ProgressView()
                    Text("Searching…")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            case .notFound:
                // Never a dead end — message + clear-and-retry.
                VStack(alignment: .leading, spacing: 8) {
                    Text("No usernames match \"\(searchText)\".")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Button("Clear and try again") {
                        clearSearch()
                    }
                    .font(.caption)
                    .accessibilityIdentifier("dashpay.addContact.retry")
                }
            case .found(let results):
                ForEach(results) { result in
                    Button {
                        selectedResult = result
                        errorMessage = nil
                    } label: {
                        HStack(spacing: 10) {
                            DashPayAvatarView(
                                avatarUrl: nil,
                                displayName: result.fullName,
                                size: 32
                            )
                            VStack(alignment: .leading, spacing: 2) {
                                Text(result.fullName)
                                    .font(.subheadline)
                                    .foregroundColor(.primary)
                                Text(result.identityId.toBase58String().prefix(16) + "…")
                                    .font(.caption2)
                                    .foregroundColor(.secondary)
                            }
                            Spacer()
                            if selectedResult == result {
                                Image(systemName: "checkmark.circle.fill")
                                    .foregroundColor(.blue)
                            }
                        }
                    }
                    .accessibilityIdentifier(
                        "dashpay.addContact.result.\(result.fullName)"
                    )
                }
            }
        } header: {
            Text("Username")
        } footer: {
            Text("Live search against the Dash Platform Name Service.")
        }
        .onChange(of: searchText) { _, newValue in
            scheduleSearch(for: newValue)
        }
    }

    private func clearSearch() {
        searchTask?.cancel()
        searchText = ""
        searchState = .idle
        selectedResult = nil
    }

    /// Debounced (~300 ms) prefix search; min 2 chars. Cancels any
    /// in-flight lookup when the prefix changes.
    private func scheduleSearch(for text: String) {
        searchTask?.cancel()
        selectedResult = nil
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.count >= minSearchLength else {
            searchState = .idle
            return
        }
        searchTask = Task { @MainActor in
            try? await Task.sleep(for: searchDebounce)
            guard !Task.isCancelled else { return }
            guard let wallet = try? requireWallet() else {
                searchState = .idle
                errorMessage = "No wallet available for this identity"
                return
            }
            searchState = .searching
            do {
                let results = try await wallet.searchDpnsNames(prefix: trimmed, limit: 10)
                guard !Task.isCancelled else { return }
                searchState = results.isEmpty ? .notFound : .found(results)
            } catch {
                guard !Task.isCancelled else { return }
                searchState = .idle
                errorMessage = "Search failed: \(error.localizedDescription)"
            }
        }
    }

    // MARK: - Identity ID mode

    @ViewBuilder
    private var idSections: some View {
        Section {
            TextField("Paste identity ID (base58)", text: $idText)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                .accessibilityIdentifier("dashpay.addContact.idInput")

            // Inline validation gates the send button (§6.4).
            if !idText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                && parsedIdentityId == nil {
                Text("Not a valid identity id (expected base58)")
                    .font(.caption)
                    .foregroundColor(.red)
            }
        } header: {
            Text("Identity ID")
        } footer: {
            Text("The contact's unique Platform identity identifier.")
        }
    }

    // MARK: - Preview + send

    /// Resolved-target preview card — Send is only reachable from
    /// here (§6.4 "found" state). Profile data is a cache-only read;
    /// most unknown identities won't have one, so the card falls
    /// back to the DPNS name / truncated id.
    private func previewSection(recipient: Identifier) -> some View {
        let profile = cachedProfile(recipient)
        let name = previewDisplayName(recipient: recipient, profile: profile)
        return Section("Send to") {
            HStack(spacing: 10) {
                DashPayAvatarView(
                    avatarUrl: profile?.avatarUrl,
                    displayName: name
                )
                VStack(alignment: .leading, spacing: 2) {
                    Text(name)
                        .font(.headline)
                    Text(recipient.toBase58String().prefix(20) + "…")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    if let msg = profile?.publicMessage?
                        .trimmingCharacters(in: .whitespacesAndNewlines),
                       !msg.isEmpty {
                        Text(msg)
                            .font(.caption2)
                            .foregroundColor(.secondary)
                            .lineLimit(2)
                    }
                }
                Spacer()
            }
            if recipient == identity.identityId {
                Text("That's this identity — pick someone else.")
                    .font(.caption)
                    .foregroundColor(.red)
            }
        }
    }

    /// Optional account-label field (DIP-15 `encryptedAccountLabel`). Empty
    /// → no label is sent. Shown once a recipient is resolved.
    private var accountLabelSection: some View {
        Section("Account label (optional)") {
            TextField("e.g. Main wallet", text: $accountLabel)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                .accessibilityIdentifier("dashpay.addContact.accountLabel")
        }
    }

    private var sendSection: some View {
        Section {
            Button {
                attemptSend()
            } label: {
                HStack {
                    Spacer()
                    if isSending {
                        ProgressView()
                    } else {
                        Label("Send Request", systemImage: "paperplane")
                    }
                    Spacer()
                }
            }
            .disabled(!canSend)
            .accessibilityIdentifier("dashpay.addContact.send")
        }
    }

    private func previewDisplayName(
        recipient: Identifier,
        profile: DashPayProfile?
    ) -> String {
        dashPayContactDisplayName(
            contactId: recipient,
            alias: nil,
            profileDisplayName: profile?.displayName,
            dpnsLabel: selectedResult?.fullName
        )
    }

    private func cachedProfile(_ contactId: Identifier) -> DashPayProfile? {
        guard let wallet = try? requireWallet() else { return nil }
        return dashPayCachedProfile(
            wallet: wallet,
            ownerIdentityId: identity.identityId,
            contactId: contactId
        )
    }

    private func requireWallet() throws -> ManagedPlatformWallet {
        guard let walletId = identity.wallet?.walletId,
              let wallet = walletManager.wallet(for: walletId) else {
            throw PlatformWalletError.walletOperation(
                "No loaded wallet for identity \(identity.identityIdBase58)"
            )
        }
        return wallet
    }

    // MARK: - Send flow (§6.4 collision check first)

    /// Check the local store for a pending incoming request from the
    /// target before sending — if one exists, surface the collision
    /// alert (Accept / Continue anyway) instead of silently
    /// double-requesting.
    private func attemptSend() {
        guard let recipient = resolvedRecipient else { return }
        errorMessage = nil

        if hasPendingIncomingRequest(from: recipient) {
            collisionRecipient = recipient
            showCollisionAlert = true
        } else {
            send(to: recipient)
        }
    }

    /// A *pending* incoming request = incoming row present with no
    /// outgoing row for the same contact (an established pair has
    /// both, and re-requesting an established contact is pointless
    /// but harmless — no alert for that).
    private func hasPendingIncomingRequest(from recipient: Identifier) -> Bool {
        let ownerId = identity.identityId
        let contactId = recipient
        let descriptor = FetchDescriptor<PersistentDashpayContactRequest>(
            predicate: #Predicate {
                $0.ownerIdentityId == ownerId && $0.contactIdentityId == contactId
            }
        )
        guard let rows = try? modelContext.fetch(descriptor), !rows.isEmpty else {
            return false
        }
        return rows.contains { !$0.isOutgoing } && !rows.contains { $0.isOutgoing }
    }

    private func send(to recipient: Identifier) {
        isSending = true
        errorMessage = nil
        Task { @MainActor in
            defer { isSending = false }
            do {
                let wallet = try requireWallet()
                let signer = KeychainSigner(modelContainer: modelContext.container)
                let label = accountLabel.trimmingCharacters(in: .whitespacesAndNewlines)
                _ = try await wallet.sendContactRequest(
                    senderIdentityId: identity.identityId,
                    recipientIdentityId: recipient,
                    accountLabel: label.isEmpty ? nil : label,
                    signer: signer
                )
                onSent(recipient, mode == .dpns ? selectedResult?.fullName : nil)
                kickDashPaySync(walletManager)
                dismiss()
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    /// Collision-alert "Accept" path: resolve the live incoming
    /// `ContactRequest` and accept it — establishing the contact
    /// directly instead of sending a redundant request.
    private func acceptIncoming(from recipient: Identifier) {
        isSending = true
        errorMessage = nil
        Task { @MainActor in
            defer { isSending = false }
            do {
                let wallet = try requireWallet()
                let managed = try wallet.managedIdentity(identityId: identity.identityId)
                guard let request = try managed.getIncomingContactRequest(
                    senderId: recipient
                ) else {
                    errorMessage = "Their request isn't in local state — pull to refresh and accept it from Requests."
                    return
                }
                let signer = KeychainSigner(modelContainer: modelContext.container)
                _ = try await wallet.acceptContactRequest(request, signer: signer)
                kickDashPaySync(walletManager)
                dismiss()
            } catch {
                errorMessage = "Accept failed: \(error.localizedDescription)"
            }
        }
    }
}
