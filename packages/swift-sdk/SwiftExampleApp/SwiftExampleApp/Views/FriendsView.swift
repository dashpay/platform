import SwiftUI
import SwiftData
import SwiftDashSDK

struct FriendsView: View {
    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @StateObject private var dashPayService = ObservableDashPayService()
    @State private var selectedIdentityId: String = ""
    @State private var contacts: [DashPayContact] = []
    @State private var incomingRequests: [DashPayContactRequest] = []
    @State private var sentRequests: [DashPayContactRequest] = []
    @State private var isLoading = false
    @State private var showAddFriend = false
    @State private var showIncomingRequests = false
    @State private var errorMessage: String?
    /// Set to the contact the user tapped to open the send-payment
    /// sheet. `.sheet(item:)` presents when non-nil, tears the sheet
    /// down when reset to nil. Also convenient because
    /// `DashPayContact` is `Identifiable` via its `identityId: Data`.
    @State private var paymentTarget: DashPayContact?

    var availableIdentities: [IdentityModel] {
        appState.identities
    }

    var selectedIdentity: IdentityModel? {
        availableIdentities.first { $0.idString == selectedIdentityId }
    }

    var body: some View {
        NavigationStack {
            if availableIdentities.isEmpty {
                // No identities view
                VStack(spacing: 20) {
                    Spacer()

                    Image(systemName: "person.crop.circle.badge.exclamationmark")
                        .font(.system(size: 60))
                        .foregroundColor(.gray)

                    Text("No Identity Found")
                        .font(.title2)
                        .fontWeight(.semibold)

                    Text("Please create or load an identity first\nto manage your friends")
                        .multilineTextAlignment(.center)
                        .foregroundColor(.secondary)

                    HStack(spacing: 20) {
                        NavigationLink(destination: LoadIdentityView()) {
                            Label("Load Identity", systemImage: "square.and.arrow.down")
                                .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(.bordered)

                        NavigationLink(destination: TransitionDetailView(transitionKey: "identityCreate", transitionLabel: "Create Identity")) {
                            Label("Create Identity", systemImage: "plus.circle")
                                .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(.borderedProminent)
                    }
                    .padding(.horizontal)

                    Spacer()
                }
                .navigationTitle("Friends")
                .navigationBarTitleDisplayMode(.large)
            } else {
                VStack(spacing: 0) {
                    // Identity selector
                    VStack(spacing: 0) {
                        HStack {
                            Text("Selected Identity")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            Spacer()
                        }
                        .padding(.horizontal)
                        .padding(.top, 8)

                        Picker("Identity", selection: $selectedIdentityId) {
                            // Placeholder tag matching the initial
                            // empty-string state, so SwiftUI doesn't
                            // warn "the selection '' is invalid and
                            // does not have an associated tag". The
                            // `onAppear` default-selector replaces
                            // this with the first real identity.
                            Text("Select an identity").tag("")
                            ForEach(availableIdentities) { identity in
                                HStack {
                                    VStack(alignment: .leading) {
                                        Text(identity.alias ?? "Identity")
                                            .font(.headline)
                                        Text(identity.idString.prefix(12) + "...")
                                            .font(.caption)
                                            .foregroundColor(.secondary)
                                    }
                                    Spacer()
                                    if identity.balance > 0 {
                                        Text(formatBalance(identity.balance))
                                            .font(.caption)
                                            .foregroundColor(.blue)
                                    }
                                }
                                .tag(identity.idString)
                            }
                        }
                        .pickerStyle(.menu)
                        .padding(.horizontal)
                        .padding(.bottom, 8)
                        .background(Color(UIColor.secondarySystemBackground))
                    }

                    // Incoming requests section
                    if !incomingRequests.isEmpty {
                        Section {
                            ForEach(incomingRequests) { request in
                                ContactRequestRow(request: request, isIncoming: true) {
                                    acceptRequest(request)
                                } onReject: {
                                    rejectRequest(request)
                                }
                            }
                        } header: {
                            Text("Incoming Requests (\(incomingRequests.count))")
                        }
                    }

                    // Friends list
                    if contacts.isEmpty && !isLoading && incomingRequests.isEmpty {
                        VStack(spacing: 20) {
                            Spacer()

                            Image(systemName: "person.2.slash")
                                .font(.system(size: 50))
                                .foregroundColor(.gray)

                            Text("No Friends Yet")
                                .font(.title3)
                                .fontWeight(.medium)

                            Text("Add friends to send messages\nand share documents")
                                .multilineTextAlignment(.center)
                                .font(.caption)
                                .foregroundColor(.secondary)

                            Button {
                                showAddFriend = true
                            } label: {
                                Label("Add Friend", systemImage: "person.badge.plus")
                            }
                            .buttonStyle(.borderedProminent)

                            Spacer()
                        }
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                    } else if isLoading {
                        VStack {
                            Spacer()
                            ProgressView("Loading contacts...")
                            Spacer()
                        }
                    } else {
                        List {
                            ForEach(contacts.filter { !$0.isHidden }) { contact in
                                Button {
                                    paymentTarget = contact
                                } label: {
                                    ContactRowView(contact: contact)
                                }
                                .buttonStyle(.plain)
                            }
                        }
                    }
                }
                .navigationTitle("Friends")
                .navigationBarTitleDisplayMode(.large)
                .toolbar {
                    ToolbarItem(placement: .navigationBarTrailing) {
                        Button {
                            showAddFriend = true
                        } label: {
                            Image(systemName: "person.badge.plus")
                        }
                    }
                }
                .sheet(isPresented: $showAddFriend) {
                    AddFriendView(
                        selectedIdentity: selectedIdentity,
                        onSent: { loadFriends() }
                    )
                    .environmentObject(walletManager)
                }
                .sheet(item: $paymentTarget) { contact in
                    if let identity = selectedIdentity {
                        SendDashPayPaymentSheet(
                            senderIdentity: identity,
                            contact: contact,
                            onSent: { loadFriends() }
                        )
                        .environmentObject(walletManager)
                    }
                }
                .onAppear {
                    // Set initial selected identity if not set
                    if selectedIdentityId.isEmpty && !availableIdentities.isEmpty {
                        selectedIdentityId = availableIdentities[0].idString
                    }
                }
                .onChange(of: selectedIdentityId) { _, newValue in
                    loadFriends()
                }
            }
        }
    }

    /// Resolve the `ManagedPlatformWallet` anchored to `identity.walletId`.
    /// Errors when the identity has no wallet association or the
    /// wallet isn't currently loaded in the manager.
    private func requireWallet(
        for identity: IdentityModel
    ) throws -> ManagedPlatformWallet {
        guard let walletId = identity.walletId else {
            throw PlatformWalletError.walletOperation(
                "Identity \(identity.idString) has no walletId"
            )
        }
        guard let wallet = walletManager.wallet(for: walletId) else {
            throw PlatformWalletError.walletOperation(
                "No ManagedPlatformWallet for this identity's walletId"
            )
        }
        return wallet
    }

    /// Refresh the friends list for the currently-selected identity.
    ///
    /// Two-stage:
    ///   1. `wallet.syncContactRequests()` — fetches incoming
    ///      contact-request documents from Platform and populates
    ///      `ManagedIdentity.incoming_contact_requests` (and
    ///      auto-establishes any bidirectional matches).
    ///   2. Re-read local state off the `ManagedIdentity` snapshot
    ///      (incoming / sent / established ID arrays) and convert
    ///      to the UI value types.
    private func loadFriends() {
        guard let identity = selectedIdentity else { return }
        let wallet: ManagedPlatformWallet
        do {
            wallet = try requireWallet(for: identity)
        } catch {
            errorMessage = error.localizedDescription
            return
        }

        isLoading = true
        Task { @MainActor in
            defer { isLoading = false }

            // Stage 1: sync from Platform. Non-fatal — a sync error
            // doesn't block reading whatever local state we already
            // have.
            do {
                _ = try await wallet.syncContactRequests()
                errorMessage = nil
            } catch {
                errorMessage = "Contact request sync failed: \(error.localizedDescription)"
            }

            // Stage 2: local read via a fresh `ManagedIdentity`
            // snapshot. Every sync invalidates the prior snapshot,
            // so we grab a new one here rather than holding onto one
            // across calls.
            do {
                let managed = try wallet.managedIdentity(identityId: identity.id)
                let incomingIds = try managed.getIncomingContactRequestIds()
                let sentIds = try managed.getSentContactRequestIds()
                let establishedIds = try managed.getEstablishedContactIds()

                incomingRequests = incomingIds.map { senderId in
                    DashPayContactRequest(
                        id: "incoming-\(senderId.toHexString())",
                        senderId: senderId,
                        recipientId: identity.id
                    )
                }
                sentRequests = sentIds.map { recipientId in
                    DashPayContactRequest(
                        id: "sent-\(recipientId.toHexString())",
                        senderId: identity.id,
                        recipientId: recipientId
                    )
                }
                // Resolve display names from the cached DashPay
                // profile when available — falls back to a
                // truncated hex id for contacts without a profile
                // yet (new contacts, contacts whose profile hasn't
                // synced, etc.). The lookup is a sync local-cache
                // read (no network roundtrip per contact).
                contacts = establishedIds.map { contactId in
                    let profile = (try? wallet.getDashPayProfile(identityId: contactId)) ?? nil
                    let trimmedName = profile?.displayName?
                        .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
                    let displayName = trimmedName.isEmpty
                        ? (String(contactId.toHexString().prefix(12)) + "…")
                        : trimmedName
                    return DashPayContact(
                        id: contactId,
                        displayName: displayName,
                        identityId: contactId
                    )
                }
            } catch {
                contacts = []
                incomingRequests = []
                sentRequests = []
                errorMessage = "Failed to read local DashPay state: \(error.localizedDescription)"
            }
        }
    }

    private func acceptRequest(_ request: DashPayContactRequest) {
        guard let identity = selectedIdentity else { return }
        Task { @MainActor in
            do {
                let wallet = try requireWallet(for: identity)
                let managed = try wallet.managedIdentity(identityId: identity.id)
                guard let contactRequest = try managed.getIncomingContactRequest(
                    senderId: request.senderId
                ) else {
                    errorMessage = "Incoming request from \(request.senderId.toHexString().prefix(12))… not in local state"
                    return
                }
                _ = try await wallet.acceptContactRequest(contactRequest)
                errorMessage = nil
                loadFriends()
            } catch {
                errorMessage = "Accept failed: \(error.localizedDescription)"
            }
        }
    }

    private func rejectRequest(_ request: DashPayContactRequest) {
        guard let identity = selectedIdentity else { return }
        Task { @MainActor in
            do {
                let wallet = try requireWallet(for: identity)
                try await wallet.rejectContactRequest(
                    ourIdentityId: identity.id,
                    contactIdentityId: request.senderId
                )
                errorMessage = nil
                loadFriends()
            } catch {
                errorMessage = "Reject failed: \(error.localizedDescription)"
            }
        }
    }

    private func formatBalance(_ amount: UInt64) -> String {
        let dash = Double(amount) / 100_000_000.0

        if dash == 0 {
            return "0 DASH"
        }

        let formatter = NumberFormatter()
        formatter.minimumFractionDigits = 0
        formatter.maximumFractionDigits = 8
        formatter.numberStyle = .decimal
        formatter.groupingSeparator = ","
        formatter.decimalSeparator = "."

        if let formatted = formatter.string(from: NSNumber(value: dash)) {
            return formatted
        }

        return String(format: "%.8f", dash)
    }
}

// MARK: - Contact Row View

struct ContactRowView: View {
    let contact: DashPayContact

    var body: some View {
        HStack {
            // Avatar
            Circle()
                .fill(Color.blue.opacity(0.2))
                .frame(width: 40, height: 40)
                .overlay(
                    Text(contact.displayName.prefix(1).uppercased())
                        .font(.headline)
                        .foregroundColor(.blue)
                )

            VStack(alignment: .leading, spacing: 2) {
                Text(contact.displayName)
                    .font(.headline)

                if let dpnsName = contact.dpnsName {
                    Text(dpnsName)
                        .font(.caption)
                        .foregroundColor(.secondary)
                } else {
                    Text(contact.id.toHexString().prefix(12) + "...")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                if let note = contact.note {
                    Text(note)
                        .font(.caption2)
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                }
            }

            Spacer()
        }
        .padding(.vertical, 4)
    }
}

// MARK: - Contact Request Row View

struct ContactRequestRow: View {
    let request: DashPayContactRequest
    let isIncoming: Bool
    let onAccept: () -> Void
    let onReject: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                VStack(alignment: .leading) {
                    Text(isIncoming ? "From" : "To")
                        .font(.caption)
                        .foregroundColor(.secondary)

                    Text((isIncoming ? request.senderId : request.recipientId).toHexString().prefix(12) + "...")
                        .font(.subheadline)
                        .fontWeight(.medium)
                }

                Spacer()

                Text(request.createdAt, style: .relative)
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }

            if isIncoming {
                HStack(spacing: 12) {
                    Button("Accept") {
                        onAccept()
                    }
                    .buttonStyle(.borderedProminent)
                    .controlSize(.small)

                    Button("Reject") {
                        onReject()
                    }
                    .buttonStyle(.bordered)
                    .controlSize(.small)
                    .tint(.red)
                }
            }
        }
        .padding(.vertical, 4)
    }
}

struct AddFriendView: View {
    let selectedIdentity: IdentityModel?
    /// Fires after a contact request has been successfully broadcast
    /// + persisted. The parent re-runs `loadFriends()` to refresh
    /// the sent-request list.
    let onSent: () -> Void

    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.dismiss) private var dismiss
    @State private var searchText = ""
    @State private var searchMethod = 0 // 0: DPNS, 1: Identity ID
    @State private var isSending = false
    @State private var errorMessage: String?

    var body: some View {
        NavigationStack {
            VStack {
                Picker("Search by", selection: $searchMethod) {
                    Text("DPNS Name").tag(0)
                    Text("Identity ID").tag(1)
                }
                .pickerStyle(.segmented)
                .padding()

                Form {
                    Section {
                        TextField(
                            searchMethod == 0 ? "Enter DPNS name" : "Enter Identity ID",
                            text: $searchText
                        )
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                    } header: {
                        Text(searchMethod == 0 ? "DPNS Name" : "Identity ID")
                    } footer: {
                        Text(searchMethod == 0 ?
                            "Search for friends by their Dash Platform Name Service (DPNS) username" :
                            "Search for friends by their unique identity identifier (base58)")
                    }

                    Section {
                        Button {
                            sendRequest()
                        } label: {
                            HStack {
                                Spacer()
                                if isSending {
                                    ProgressView()
                                } else {
                                    Label("Send Friend Request", systemImage: "paperplane")
                                }
                                Spacer()
                            }
                        }
                        .disabled(
                            searchText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                            || isSending
                            || selectedIdentity == nil
                        )
                    }

                    if let errorMessage = errorMessage {
                        Section {
                            Text(errorMessage)
                                .foregroundColor(.red)
                                .font(.caption)
                        }
                    }
                }
            }
            .navigationTitle("Add Friend")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") {
                        dismiss()
                    }
                    .disabled(isSending)
                }
            }
        }
    }

    /// Resolve the recipient identity id (via DPNS name lookup or
    /// direct base58 parse) and fire `sendContactRequest` against
    /// the selected identity's wallet. On success, dismisses the
    /// sheet and invokes `onSent` so the parent refreshes.
    private func sendRequest() {
        guard let identity = selectedIdentity,
              let walletId = identity.walletId,
              let wallet = walletManager.wallet(for: walletId) else {
            errorMessage = "No wallet available for this identity"
            return
        }
        let trimmed = searchText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }

        isSending = true
        errorMessage = nil

        Task { @MainActor in
            defer { isSending = false }
            do {
                // Resolve recipient. DPNS mode goes through
                // `resolveDpnsName`; ID mode parses base58 directly.
                let recipientId: Identifier
                if searchMethod == 0 {
                    guard let resolved = try await wallet.resolveDpnsName(trimmed) else {
                        errorMessage = "DPNS name not found"
                        return
                    }
                    recipientId = resolved
                } else {
                    guard let parsed = Data.identifier(fromBase58: trimmed) else {
                        errorMessage = "Invalid identity id (expected base58)"
                        return
                    }
                    recipientId = parsed
                }

                _ = try await wallet.sendContactRequest(
                    senderIdentityId: identity.id,
                    recipientIdentityId: recipientId
                )
                onSent()
                dismiss()
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }
}

// MARK: - Send payment sheet

/// Modal sheet for sending a Dash payment to an established DashPay
/// contact via the platform-wallet FFI
/// (`ManagedPlatformWallet.sendDashPayPayment`). Rust handles the
/// address derivation (DIP-14) + Core-chain broadcast + recording
/// a `PaymentEntry` on the sender's `ManagedIdentity`; the
/// identity changeset callback (5f5ac06d6) forwards the state
/// update to SwiftData.
struct SendDashPayPaymentSheet: View {
    let senderIdentity: IdentityModel
    let contact: DashPayContact
    /// Fires with the 32-byte txid once the transaction has been
    /// broadcast. Parent uses this to refresh the friends list
    /// (recording a payment may auto-establish a contact on the
    /// Rust side if a reciprocal request just came in).
    let onSent: () -> Void

    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.dismiss) private var dismiss

    /// Input in DASH — we convert to duffs before handing off to
    /// the FFI. Keeping the field in DASH makes the units
    /// human-readable (a 0.001 DASH payment is `0.001` in the
    /// field, not `100000`).
    @State private var amountText = ""
    @State private var memo = ""
    @State private var isSending = false
    @State private var errorMessage: String?
    @State private var successTxid: Data?

    /// DASH → duffs. Returns nil when the input isn't a parseable
    /// non-negative decimal or overflows `UInt64`.
    private var amountDuffs: UInt64? {
        let trimmed = amountText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let dashValue = Decimal(string: trimmed), dashValue >= 0 else {
            return nil
        }
        // 1 DASH = 100_000_000 duffs. Decimal multiply then snap
        // to `UInt64`; a negative or overflowed intermediate
        // yields nil.
        let duffsDecimal = dashValue * 100_000_000
        // `NSDecimalNumber(decimal:).uint64Value` truncates on
        // overflow — detect by re-comparing.
        let duffs = NSDecimalNumber(decimal: duffsDecimal).uint64Value
        let roundTrip = Decimal(duffs)
        return roundTrip == duffsDecimal ? duffs : nil
    }

    var body: some View {
        NavigationStack {
            Form {
                Section("To") {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(contact.displayName)
                            .font(.headline)
                        Text(contact.identityId.toHexString().prefix(20) + "…")
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .lineLimit(1)
                            .truncationMode(.middle)
                    }
                }

                Section("Amount (DASH)") {
                    TextField("0.001", text: $amountText)
                        .keyboardType(.decimalPad)
                        .autocorrectionDisabled()
                        .textInputAutocapitalization(.never)
                    if !amountText.isEmpty, amountDuffs == nil {
                        Text("Enter a valid decimal Dash amount")
                            .font(.caption)
                            .foregroundColor(.red)
                    }
                }

                Section("Memo (optional)") {
                    TextField("Dinner, rent share, …", text: $memo)
                }

                if let successTxid = successTxid {
                    Section {
                        Text("Sent! txid: \(successTxid.toHexString().prefix(16))…")
                            .font(.caption)
                            .foregroundColor(.green)
                    }
                } else if let errorMessage = errorMessage {
                    Section {
                        Text(errorMessage)
                            .font(.caption)
                            .foregroundColor(.red)
                    }
                }
            }
            .navigationTitle("Send Dash")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                        .disabled(isSending)
                }
                ToolbarItem(placement: .navigationBarTrailing) {
                    if isSending {
                        ProgressView()
                    } else {
                        Button("Send") { send() }
                            .disabled(amountDuffs == nil || (amountDuffs ?? 0) == 0)
                    }
                }
            }
        }
    }

    /// Broadcast the payment via the platform-wallet FFI. Memo is
    /// currently passed to the Rust side but the signing path
    /// doesn't embed it in the transaction yet — it's recorded on
    /// the local `PaymentEntry` so the payment-history UI (when
    /// wired) has context.
    private func send() {
        guard let walletId = senderIdentity.walletId,
              let wallet = walletManager.wallet(for: walletId) else {
            errorMessage = "No wallet available for this identity"
            return
        }
        guard let duffs = amountDuffs, duffs > 0 else {
            errorMessage = "Amount must be greater than zero"
            return
        }
        let trimmedMemo = memo.trimmingCharacters(in: .whitespacesAndNewlines)
        let memoCopy: String? = trimmedMemo.isEmpty ? nil : trimmedMemo

        isSending = true
        errorMessage = nil

        Task { @MainActor in
            defer { isSending = false }
            do {
                let txid = try await wallet.sendDashPayPayment(
                    fromIdentityId: senderIdentity.id,
                    toContactIdentityId: contact.identityId,
                    amountDuffs: duffs,
                    memo: memoCopy
                )
                successTxid = txid
                onSent()
                // Small settle-in before dismissing so the user
                // sees the confirmation row. Mirrors the pattern in
                // `RegisterNameView`.
                try? await Task.sleep(nanoseconds: 1_500_000_000)
                dismiss()
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }
}

#Preview {
    FriendsView()
        .environmentObject(AppState())
}
