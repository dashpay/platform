import SwiftUI
import SwiftData
import SwiftDashSDK

struct FriendsView: View {
    /// The identity whose DashPay contacts/requests/friends are being
    /// browsed. Always supplied by the parent — the view used to host
    /// its own identity picker but that's been removed; the only entry
    /// point now is the per-identity drill-in from `IdentityDetailView`.
    let identity: PersistentIdentity

    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @StateObject private var dashPayService = ObservableDashPayService()
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

    var body: some View {
        // No outer NavigationStack — this view is always pushed inside
        // the parent's stack (IdentityDetailView's tab NavigationStack).
        // Single `List` so the Incoming Requests `Section` actually
        // gets sectioned styling — a `Section` directly inside a
        // `VStack` is a no-op visually, just rendering its rows
        // without a header/separator.
        //
        // We qualify with `SwiftUI.Group` here because
        // `SwiftDashSDK.Group` is a `Codable` DPP type, and an
        // unqualified `Group { ... }` resolves to its Codable
        // initializer rather than the SwiftUI view builder — Swift
        // surfaces that as a "trailing closure passed to parameter
        // of type 'any Decoder'" diagnostic.
        SwiftUI.Group {
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
            } else if isLoading && contacts.isEmpty && incomingRequests.isEmpty {
                VStack {
                    Spacer()
                    ProgressView("Loading contacts...")
                    Spacer()
                }
            } else {
                List {
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

                    if !contacts.isEmpty {
                        Section {
                            ForEach(contacts.filter { !$0.isHidden }) { contact in
                                Button {
                                    paymentTarget = contact
                                } label: {
                                    ContactRowView(contact: contact)
                                }
                                .buttonStyle(.plain)
                            }
                        } header: {
                            Text("Friends (\(contacts.filter { !$0.isHidden }.count))")
                        }
                    }
                }
            }
        }
        .navigationTitle("Friends")
        .navigationBarTitleDisplayMode(.inline)
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
                selectedIdentity: identity,
                onSent: { loadFriends() }
            )
            .environmentObject(walletManager)
        }
        .sheet(item: $paymentTarget) { contact in
            SendDashPayPaymentSheet(
                senderIdentity: identity,
                contact: contact,
                onSent: { loadFriends() }
            )
            .environmentObject(walletManager)
        }
        .onAppear {
            loadFriends()
        }
    }

    /// Resolve the `ManagedPlatformWallet` anchored to `identity.wallet?.walletId`.
    /// Errors when the identity has no wallet association or the
    /// wallet isn't currently loaded in the manager.
    private func requireWallet(
        for identity: PersistentIdentity
    ) throws -> ManagedPlatformWallet {
        guard let walletId = identity.wallet?.walletId else {
            throw PlatformWalletError.walletOperation(
                "Identity \(identity.identityIdBase58) has no walletId"
            )
        }
        guard let wallet = walletManager.wallet(for: walletId) else {
            throw PlatformWalletError.walletOperation(
                "No ManagedPlatformWallet for this identity's walletId"
            )
        }
        return wallet
    }

    /// Refresh the friends list for this view's identity.
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
                let managed = try wallet.managedIdentity(identityId: identity.identityId)
                let incomingIds = try managed.getIncomingContactRequestIds()
                let sentIds = try managed.getSentContactRequestIds()
                let establishedIds = try managed.getEstablishedContactIds()

                incomingRequests = incomingIds.map { senderId in
                    DashPayContactRequest(
                        id: "incoming-\(senderId.toHexString())",
                        senderId: senderId,
                        recipientId: identity.identityId
                    )
                }
                sentRequests = sentIds.map { recipientId in
                    DashPayContactRequest(
                        id: "sent-\(recipientId.toHexString())",
                        senderId: identity.identityId,
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
        Task { @MainActor in
            do {
                let wallet = try requireWallet(for: identity)
                let managed = try wallet.managedIdentity(identityId: identity.identityId)
                guard let contactRequest = try managed.getIncomingContactRequest(
                    senderId: request.senderId
                ) else {
                    errorMessage = "Incoming request from \(request.senderId.toHexString().prefix(12))… not in local state"
                    return
                }
                let signer = KeychainSigner(modelContainer: modelContext.container)
                _ = try await wallet.acceptContactRequest(contactRequest, signer: signer)
                errorMessage = nil
                loadFriends()
            } catch {
                errorMessage = "Accept failed: \(error.localizedDescription)"
            }
        }
    }

    private func rejectRequest(_ request: DashPayContactRequest) {
        Task { @MainActor in
            do {
                let wallet = try requireWallet(for: identity)
                try await wallet.rejectContactRequest(
                    ourIdentityId: identity.identityId,
                    contactIdentityId: request.senderId
                )
                errorMessage = nil
                loadFriends()
            } catch {
                errorMessage = "Reject failed: \(error.localizedDescription)"
            }
        }
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
    let selectedIdentity: PersistentIdentity?
    /// Fires after a contact request has been successfully broadcast
    /// + persisted. The parent re-runs `loadFriends()` to refresh
    /// the sent-request list.
    let onSent: () -> Void

    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.dismiss) private var dismiss
    @Environment(\.modelContext) private var modelContext
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
              let walletId = identity.wallet?.walletId,
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

                // Construct a fresh `KeychainSigner` and route through
                // the platform-wallet
                // `IdentityWallet::send_contact_request_with_external_signer`
                // path. CAVEAT: the contact-request encryption step
                // still derives the sender's ECDH key Rust-side from
                // the wallet seed (watch-only wallets fail there) —
                // see the docstring on `sendContactRequest(...,signer:)`.
                let signer = KeychainSigner(modelContainer: modelContext.container)
                _ = try await wallet.sendContactRequest(
                    senderIdentityId: identity.identityId,
                    recipientIdentityId: recipientId,
                    signer: signer
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
    let senderIdentity: PersistentIdentity
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
    @State private var isSending = false
    @State private var errorMessage: String?
    @State private var successTxid: Data?

    /// Cached recipient profile resolved from the platform-wallet
    /// cache on appear. Empty until the first lookup completes; the
    /// recipient section falls back to the `DashPayContact` fields
    /// until then so the sheet doesn't flicker.
    @State private var recipientProfile: DashPayProfile?
    @State private var recipientDpnsName: String?

    /// Sender's current Core balance (spendable duffs). Pulled from
    /// the Core wallet's lock-free balance on appear so the user
    /// can see what they actually have before submitting. `nil`
    /// while the async fetch is in flight or if the wallet handle
    /// can't be resolved.
    @State private var senderBalanceDuffs: UInt64?

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

    /// Pretty name to show in the "To" section. Prefers the
    /// DashPay profile's display name, then the identity's DPNS
    /// label, then the contact's stored display name (truncated
    /// hex by default). Recalculated whenever the resolved
    /// profile / DPNS changes.
    private var recipientDisplayName: String {
        if let trimmed = recipientProfile?.displayName?
            .trimmingCharacters(in: .whitespacesAndNewlines),
           !trimmed.isEmpty {
            return trimmed
        }
        if let dpns = recipientDpnsName, !dpns.isEmpty {
            return dpns
        }
        return contact.displayName
    }

    /// Subtitle on the recipient row: public message if present,
    /// else DPNS (when the headline is already the profile name),
    /// else the truncated hex id.
    private var recipientSubtitle: String? {
        if let msg = recipientProfile?.publicMessage?
            .trimmingCharacters(in: .whitespacesAndNewlines),
           !msg.isEmpty {
            return msg
        }
        if let dpns = recipientDpnsName,
           recipientProfile?.displayName?.isEmpty == false {
            return dpns
        }
        return String(contact.identityId.toHexString().prefix(20)) + "…"
    }

    /// "1.23456789 DASH" — only rendered when we have a balance
    /// number to show.
    private var senderBalanceText: String? {
        guard let duffs = senderBalanceDuffs else { return nil }
        let dash = Double(duffs) / 100_000_000
        return String(format: "%.8f DASH", dash)
    }

    /// Duffs available for spending, minus the current amount
    /// input. `nil` when either the balance hasn't loaded or the
    /// amount input doesn't parse. Used to flag over-spends in the
    /// validation row + disable the Send button.
    private var exceedsBalance: Bool {
        guard let balance = senderBalanceDuffs,
              let duffs = amountDuffs else {
            return false
        }
        return duffs > balance
    }

    var body: some View {
        NavigationStack {
            Form {
                Section("To") {
                    VStack(alignment: .leading, spacing: 4) {
                        HStack(spacing: 10) {
                            if let url = recipientProfile?.avatarUrl
                                .flatMap({ URL(string: $0) }) {
                                AsyncImage(url: url) { phase in
                                    if let image = phase.image {
                                        image
                                            .resizable()
                                            .aspectRatio(contentMode: .fill)
                                    } else {
                                        Color.blue.opacity(0.15)
                                    }
                                }
                                .frame(width: 32, height: 32)
                                .clipShape(Circle())
                            } else {
                                Circle()
                                    .fill(Color.blue.opacity(0.2))
                                    .frame(width: 32, height: 32)
                                    .overlay(
                                        Text(recipientDisplayName.prefix(1).uppercased())
                                            .font(.headline)
                                            .foregroundColor(.blue)
                                    )
                            }
                            VStack(alignment: .leading, spacing: 2) {
                                Text(recipientDisplayName)
                                    .font(.headline)
                                if let sub = recipientSubtitle {
                                    Text(sub)
                                        .font(.caption)
                                        .foregroundColor(.secondary)
                                        .lineLimit(1)
                                        .truncationMode(.middle)
                                }
                            }
                        }
                    }
                }

                Section("Amount (DASH)") {
                    TextField("0.001", text: $amountText)
                        .keyboardType(.decimalPad)
                        .autocorrectionDisabled()
                        .textInputAutocapitalization(.never)
                    if let balanceText = senderBalanceText {
                        HStack {
                            Text("Your balance")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            Spacer()
                            Text(balanceText)
                                .font(.caption)
                                .fontWeight(.medium)
                                .foregroundColor(exceedsBalance ? .red : .secondary)
                        }
                    }
                    if !amountText.isEmpty, amountDuffs == nil {
                        Text("Enter a valid decimal Dash amount")
                            .font(.caption)
                            .foregroundColor(.red)
                    } else if exceedsBalance {
                        Text("Amount exceeds your spendable balance")
                            .font(.caption)
                            .foregroundColor(.red)
                    }
                }

                // Memo row intentionally absent. DashPay payments
                // are plain Core-chain transactions — there's no
                // on-chain memo slot and no DashPay document type
                // for per-payment notes — so a memo field here
                // would be misleading. `PaymentEntry.memo` on the
                // Rust side is a local-only record the sender's
                // wallet could populate from elsewhere if needed;
                // the payment sheet stays honest by omitting it.

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
                            .disabled(
                                amountDuffs == nil
                                || (amountDuffs ?? 0) == 0
                                || exceedsBalance
                            )
                    }
                }
            }
            .task {
                await loadRecipientMetadata()
                await loadSenderBalance()
            }
        }
    }

    /// Resolve the recipient's profile + DPNS label from the
    /// platform-wallet cache. Both are in-memory cache reads (no
    /// network roundtrips) — the profile came from
    /// `syncDashPayProfiles` and the DPNS label from any prior
    /// `syncDpnsNames` for that identity. When the cache is empty
    /// we fall back to the hex id display in the computed
    /// properties above.
    ///
    /// We do NOT trigger a network sync here — opening a payment
    /// sheet for every contact would spam the wallet; recipient
    /// profiles refresh whenever the parent `FriendsView` runs its
    /// own sync on appear.
    private func loadRecipientMetadata() async {
        guard let walletId = senderIdentity.wallet?.walletId,
              let wallet = walletManager.wallet(for: walletId) else {
            return
        }
        do {
            recipientProfile = try wallet.getDashPayProfile(identityId: contact.identityId)
        } catch {
            // Profile isn't cached — stay with fallback rendering.
            recipientProfile = nil
        }
        do {
            let managed = try wallet.managedIdentity(identityId: contact.identityId)
            let names = (try? managed.getDpnsNames()) ?? []
            recipientDpnsName = names.first
        } catch {
            // The recipient isn't a managed identity on this
            // wallet (they're a contact, not an owned identity).
            // That's the common case; leave DPNS unset.
            recipientDpnsName = nil
        }
    }

    /// Fetch the sender wallet's current Core balance so the
    /// amount row can show "spendable: X DASH" and block submits
    /// that exceed it. Uses the lock-free balance accessor —
    /// atomic reads, no async work.
    private func loadSenderBalance() async {
        guard let walletId = senderIdentity.wallet?.walletId,
              let wallet = walletManager.wallet(for: walletId) else {
            senderBalanceDuffs = nil
            return
        }
        do {
            let balance = try wallet.balance()
            senderBalanceDuffs = balance.spendable
        } catch {
            senderBalanceDuffs = nil
        }
    }

    /// Broadcast the payment via the platform-wallet FFI. Memo is
    /// currently passed to the Rust side but the signing path
    /// doesn't embed it in the transaction yet — it's recorded on
    /// the local `PaymentEntry` so the payment-history UI (when
    /// wired) has context.
    private func send() {
        guard let walletId = senderIdentity.wallet?.walletId,
              let wallet = walletManager.wallet(for: walletId) else {
            errorMessage = "No wallet available for this identity"
            return
        }
        guard let duffs = amountDuffs, duffs > 0 else {
            errorMessage = "Amount must be greater than zero"
            return
        }

        isSending = true
        errorMessage = nil

        Task { @MainActor in
            defer { isSending = false }
            do {
                // `memo: nil` — DashPay payments don't carry memos
                // on-chain or via a document, so there's nothing
                // useful to pass. The Rust-side
                // `PaymentEntry.memo` slot stays available for
                // future local-note wiring.
                let txid = try await wallet.sendDashPayPayment(
                    fromIdentityId: senderIdentity.identityId,
                    toContactIdentityId: contact.identityId,
                    amountDuffs: duffs,
                    memo: nil
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

// #Preview omitted — FriendsView now requires a live
// `PersistentIdentity`, which isn't easy to fabricate in a preview
// context. Exercise via the IdentityDetailView -> Friends drill-in.
