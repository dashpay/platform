import SwiftUI
import SwiftData
import SwiftDashSDK

/// Per-contact detail: profile header, Send Dash (via
/// the existing `SendDashPayPaymentSheet`), `@Query`-driven payment
/// history, and the alias / note / hide controls — `contactInfo`-
/// backed: edits publish a self-encrypted document so they
/// sync across devices and survive restore-from-seed.
struct ContactDetailView: View {
    let identity: PersistentIdentity
    let contactId: Data

    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var contactMeta: DashPayContactMetaStore
    @Environment(\.modelContext) private var modelContext

    /// Payment history with this contact. Refreshed on demand via
    /// `refreshDashPayPayments` (the Rust map is read → upserted →
    /// observed here reactively).
    @Query private var payments: [PersistentDashpayPayment]

    /// This pair's request rows — drives the broken-channel state
    /// reactively: when a fresh request arrives and the Rust side
    /// clears `payment_channel_broken`, the persister updates the
    /// rows and Send Dash re-enables without a manual refresh.
    @Query private var pairRows: [PersistentDashpayContactRequest]

    @State private var showPaymentSheet = false
    @State private var showAliasEditor = false
    @State private var showNoteEditor = false
    @State private var isRefreshingPayments = false
    @State private var paymentsError: String?

    init(identity: PersistentIdentity, contactId: Data) {
        self.identity = identity
        self.contactId = contactId
        _payments = Query(
            filter: PersistentDashpayPayment.predicate(
                ownerIdentityId: identity.identityId,
                counterpartyIdentityId: contactId
            ),
            sort: [SortDescriptor(\PersistentDashpayPayment.createdAt, order: .reverse)]
        )
        let ownerId = identity.identityId
        let contact = contactId
        _pairRows = Query(
            filter: #Predicate<PersistentDashpayContactRequest> {
                $0.ownerIdentityId == ownerId && $0.contactIdentityId == contact
            }
        )
    }

    // MARK: - Derived

    private var channelBroken: Bool {
        pairRows.contains(where: \.paymentChannelBroken)
    }

    /// contactInfo-backed alias — read off the established contact
    /// rows (both directions carry the same value; first non-nil
    /// wins). Reactive via the `pairRows` `@Query`: the recurring
    /// sync's decrypted contactInfo lands through the persister and
    /// re-renders here.
    private var localAlias: String? {
        pairRows.compactMap(\.contactAlias).first
    }

    private var localNote: String? {
        pairRows.compactMap(\.contactNote).first
    }

    /// The contact's decrypted DIP-15 `encryptedAccountLabel` — the label
    /// the contact chose for the account they shared (a payment-routing
    /// hint). Read off the **incoming** row only: the outgoing row carries
    /// a label *we* sent, which we don't surface. System-derived and
    /// read-only, distinct from the owner-private `localAlias`/`localNote`.
    private var contactAccountLabel: String? {
        pairRows.first(where: { !$0.isOutgoing })?.contactAccountLabel
    }

    private var dpnsHint: String? {
        contactMeta.dpnsHint(
            network: identity.network,
            owner: identity.identityId,
            contact: contactId
        )
    }

    private var profile: DashPayProfile? {
        guard let walletId = identity.wallet?.walletId,
              let wallet = walletManager.wallet(for: walletId) else {
            return nil
        }
        return dashPayCachedProfile(
            wallet: wallet,
            ownerIdentityId: identity.identityId,
            contactId: contactId
        )
    }

    private var displayName: String {
        dashPayContactDisplayName(
            contactId: contactId,
            alias: localAlias,
            profileDisplayName: profile?.displayName,
            dpnsLabel: dpnsHint
        )
    }

    private var isHidden: Bool {
        pairRows.contains(where: \.contactHidden)
    }

    /// In-flight contactInfo save — disables the controls so a slow
    /// publish can't be double-submitted; errors render inline.
    @State private var isSavingContactInfo = false
    @State private var contactInfoError: String?
    /// Set after a save whose document publish was deferred/skipped, so the
    /// UI doesn't claim a cross-device sync that didn't happen (H2).
    @State private var publishNotice: String?

    var body: some View {
        List {
            headerSection
            sendSection
            paymentsSection
            localSettingsSection
        }
        .navigationTitle(displayName)
        .navigationBarTitleDisplayMode(.inline)
        .sheet(isPresented: $showPaymentSheet) {
            SendDashPayPaymentSheet(
                senderIdentity: identity,
                contact: DashPayContact(
                    id: contactId,
                    displayName: displayName,
                    identityId: contactId,
                    dpnsName: dpnsHint
                ),
                onSent: { refreshPayments() }
            )
            .environmentObject(walletManager)
        }
        .sheet(isPresented: $showAliasEditor) {
            ContactLocalFieldEditor(
                title: "Alias",
                prompt: "e.g. Mom",
                footer: "An alias overrides this contact's display name. Encrypted and synced to your other devices once this identity has two or more contacts.",
                initialValue: localAlias ?? "",
                identifierPrefix: "dashpay.detail.alias",
                onSave: { value in
                    saveContactInfo(alias: value, note: localNote, hidden: isHidden)
                }
            )
        }
        .sheet(isPresented: $showNoteEditor) {
            ContactLocalFieldEditor(
                title: "Note",
                prompt: "Anything to remember about this contact",
                footer: "Notes are private (encrypted) and synced to your other devices once this identity has two or more contacts.",
                initialValue: localNote ?? "",
                identifierPrefix: "dashpay.detail.note",
                onSave: { value in
                    saveContactInfo(alias: localAlias, note: value, hidden: isHidden)
                }
            )
        }
        .task {
            refreshPayments()
        }
        .onChange(of: walletManager.dashPaySyncIsSyncing) { _, syncing in
            // A Sent payment is flipped Pending → Confirmed in the
            // in-memory model by a Core block event, and that change
            // reaches SwiftData only through a payment refresh (the
            // changeset/store path does not persist DashPay payments).
            // Re-pull on each completed DashPay sync pass so the status
            // updates live here without a manual Refresh.
            if !syncing {
                refreshPayments()
            }
        }
    }

    // MARK: - Sections

    private var headerSection: some View {
        Section {
            HStack(spacing: 14) {
                DashPayAvatarView(
                    avatarUrl: profile?.avatarUrl,
                    displayName: displayName,
                    size: 56
                )
                VStack(alignment: .leading, spacing: 3) {
                    Text(displayName)
                        .font(.title3)
                        .fontWeight(.semibold)
                    if let dpns = dpnsHint {
                        Text(dpns)
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    Text(contactId.toBase58String().prefix(20) + "…")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                    if let msg = profile?.publicMessage?
                        .trimmingCharacters(in: .whitespacesAndNewlines),
                       !msg.isEmpty {
                        Text(msg)
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .lineLimit(2)
                    }
                }
            }
            .padding(.vertical, 4)

            if let note = localNote {
                Label(note, systemImage: "note.text")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            // The contact's own label for the account they shared (DIP-15
            // encryptedAccountLabel) — a read-only payment-routing hint,
            // distinct from the owner-private alias/note above.
            if let accountLabel = contactAccountLabel {
                VStack(alignment: .leading, spacing: 1) {
                    Text("Their account")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                    Label(accountLabel, systemImage: "wallet.pass")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
        }
    }

    private var sendSection: some View {
        Section {
            Button {
                showPaymentSheet = true
            } label: {
                Label("Send Dash", systemImage: "paperplane.fill")
                    .fontWeight(.medium)
            }
            .disabled(channelBroken)
            .accessibilityIdentifier("dashpay.detail.sendDash")

            if channelBroken {
                // Broken payment channel. Re-enables
                // reactively when a new request flips the flag.
                Label(
                    "Payment channel broken — ask the contact to send a new request",
                    systemImage: "exclamationmark.triangle.fill"
                )
                .font(.caption)
                .foregroundColor(.orange)
            }
        }
    }

    private var paymentsSection: some View {
        Section {
            if payments.isEmpty {
                if isRefreshingPayments {
                    // Loading: single inline ProgressView.
                    HStack(spacing: 10) {
                        ProgressView()
                        Text("Loading payments…")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                } else {
                    Text("No payments yet")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            } else {
                ForEach(payments, id: \.txid) { payment in
                    PaymentHistoryRow(payment: payment)
                }
            }

            if let paymentsError {
                // Error: keep the last-known list, caption only.
                Text(paymentsError)
                    .font(.caption)
                    .foregroundColor(.red)
            }
        } header: {
            HStack {
                Text("Payments (\(payments.count))")
                Spacer()
                Button {
                    refreshPayments()
                } label: {
                    Image(systemName: "arrow.clockwise")
                        .symbolEffect(
                            .rotate,
                            options: .nonRepeating,
                            isActive: isRefreshingPayments
                        )
                }
                .disabled(isRefreshingPayments)
                .accessibilityIdentifier("dashpay.detail.refreshPayments")
            }
        }
    }

    private var localSettingsSection: some View {
        Section {
            Button {
                showAliasEditor = true
            } label: {
                HStack {
                    Label("Alias", systemImage: "person.text.rectangle")
                    Spacer()
                    Text(localAlias ?? "None")
                        .foregroundColor(.secondary)
                }
            }
            .foregroundColor(.primary)
            .accessibilityIdentifier("dashpay.detail.aliasEdit")

            Button {
                showNoteEditor = true
            } label: {
                HStack {
                    Label("Note", systemImage: "note.text")
                    Spacer()
                    Text(localNote == nil ? "None" : "Edit")
                        .foregroundColor(.secondary)
                }
            }
            .foregroundColor(.primary)
            .accessibilityIdentifier("dashpay.detail.noteEdit")

            Toggle(isOn: Binding(
                get: { isHidden },
                set: { hidden in
                    saveContactInfo(alias: localAlias, note: localNote, hidden: hidden)
                }
            )) {
                Label("Hide contact", systemImage: "eye.slash")
            }
            .disabled(isSavingContactInfo)
            .accessibilityIdentifier("dashpay.detail.hideToggle")

            if isSavingContactInfo {
                HStack(spacing: 10) {
                    ProgressView()
                    Text("Saving…")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            if let contactInfoError {
                Text(contactInfoError)
                    .font(.caption)
                    .foregroundColor(.red)
            }
            // Honest publish-state banner (H2): tell the user when an edit
            // was saved locally but NOT published cross-device, instead of
            // the footer claiming an unconditional sync.
            if let publishNotice {
                Label(publishNotice, systemImage: "icloud.slash")
                    .font(.caption)
                    .foregroundColor(.orange)
            }
        } header: {
            Text("Contact settings")
        } footer: {
            // contactInfo-backed (M3): self-encrypted on Platform. The
            // footer states the steady-state behaviour; the per-save
            // `publishNotice` above corrects it when a publish was deferred.
            Text("Alias, note and hide are encrypted and synced to your other devices via Platform once this identity has two or more contacts.")
        }
    }

    /// Persist alias/note/hidden through the contactInfo pipeline:
    /// local state updates immediately (the persister round lands in
    /// the rows the `pairRows` query watches); the document publish
    /// happens in the same call unless deferred by the DIP-15
    /// ≥2-contacts privacy rule.
    private func saveContactInfo(alias: String?, note: String?, hidden: Bool) {
        guard let walletId = identity.wallet?.walletId,
              let wallet = walletManager.wallet(for: walletId) else {
            contactInfoError = "No wallet available for this identity"
            return
        }
        isSavingContactInfo = true
        contactInfoError = nil
        publishNotice = nil
        Task { @MainActor in
            defer { isSavingContactInfo = false }
            do {
                let signer = KeychainSigner(modelContainer: modelContext.container)
                let outcome = try await wallet.setDashPayContactInfo(
                    identityId: identity.identityId,
                    contactId: contactId,
                    alias: alias?.isEmpty == true ? nil : alias,
                    note: note?.isEmpty == true ? nil : note,
                    hidden: hidden,
                    signer: signer
                )
                switch outcome {
                case .published:
                    publishNotice = nil
                case .deferredUntilTwoContacts:
                    publishNotice = "Saved on this device. It will sync to your other devices once this identity has two or more contacts."
                case .skippedWatchOnly:
                    publishNotice = "Saved on this device only — this watch-only identity can't publish to Platform."
                }
            } catch {
                contactInfoError = "Save failed: \(error.localizedDescription)"
            }
        }
    }

    // MARK: - Payment refresh

    /// One FFI read + one persistence pass; the `@Query` above picks
    /// the upserts up reactively.
    private func refreshPayments() {
        // Collapse overlapping triggers (`.task` on appear, `onSent`, the
        // sync falling-edge `onChange`, the manual Refresh button) into one
        // in-flight pass — the FFI read + SwiftData upsert is idempotent, so
        // a concurrent second pass is wasted work and flickers the spinner.
        guard !isRefreshingPayments else { return }
        guard let walletId = identity.wallet?.walletId else {
            paymentsError = "Identity has no wallet association"
            return
        }
        isRefreshingPayments = true
        paymentsError = nil
        Task { @MainActor in
            defer { isRefreshingPayments = false }
            do {
                _ = try walletManager.refreshDashPayPayments(
                    walletId: walletId,
                    identityId: identity.identityId
                )
            } catch {
                paymentsError = "Payment refresh failed: \(error.localizedDescription)"
            }
        }
    }
}

// MARK: - Payment row

/// One payment-history entry. `PaymentEntry` has no timestamp, so
/// the row shows direction + txid prefix + amount + status (§6.4).
struct PaymentHistoryRow: View {
    let payment: PersistentDashpayPayment

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: payment.direction == .sent
                ? "arrow.up.right.circle.fill"
                : "arrow.down.left.circle.fill")
                .foregroundColor(payment.direction == .sent ? .blue : .green)
            VStack(alignment: .leading, spacing: 2) {
                Text(payment.direction == .sent ? "Sent" : "Received")
                    .font(.subheadline)
                    .fontWeight(.medium)
                Text(payment.txid.prefix(16) + "…")
                    .font(.caption2)
                    .foregroundColor(.secondary)
                if let memo = payment.memo, !memo.isEmpty {
                    Text(memo)
                        .font(.caption2)
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                }
            }
            Spacer()
            VStack(alignment: .trailing, spacing: 2) {
                Text(formattedAmount)
                    .font(.subheadline.monospacedDigit())
                    .fontWeight(.semibold)
                Text(statusLabel)
                    .font(.caption2)
                    .foregroundColor(statusColor)
            }
        }
        .padding(.vertical, 2)
    }

    private var formattedAmount: String {
        let dash = Double(payment.amountDuffs) / 100_000_000
        return String(format: "%.8f DASH", dash)
    }

    private var statusLabel: String {
        switch payment.status {
        case .pending: return "Pending"
        case .confirmed: return "Confirmed"
        case .failed: return "Failed"
        }
    }

    private var statusColor: Color {
        switch payment.status {
        case .pending: return .orange
        case .confirmed: return .green
        case .failed: return .red
        }
    }
}

// MARK: - Local field editor

/// Tiny Form-based editor sheet for the device-local alias / note
/// fields — same shape as `EditAliasView` but writing to the
/// `DashPayContactMetaStore` instead of a SwiftData row. Saving an
/// empty value clears the field.
struct ContactLocalFieldEditor: View {
    let title: String
    let prompt: String
    let footer: String
    let initialValue: String
    let identifierPrefix: String
    let onSave: (String?) -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var value: String = ""

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField(prompt, text: $value)
                        .accessibilityIdentifier("\(identifierPrefix).field")
                } footer: {
                    Text(footer)
                }
            }
            .navigationTitle(title)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                        .accessibilityIdentifier("\(identifierPrefix).cancel")
                }
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Save") {
                        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
                        onSave(trimmed.isEmpty ? nil : trimmed)
                        dismiss()
                    }
                    .accessibilityIdentifier("\(identifierPrefix).save")
                }
            }
            .onAppear { value = initialValue }
        }
    }
}
