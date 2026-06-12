import SwiftUI
import SwiftData
import SwiftDashSDK

/// Per-contact detail (SPEC §6.2): profile header, Send Dash (via
/// the existing `SendDashPayPaymentSheet`), `@Query`-driven payment
/// history, and the device-local alias / note / hide controls — all
/// labeled "This device only" until M3's `contactInfo` backing.
struct ContactDetailView: View {
    let identity: PersistentIdentity
    let contactId: Data

    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var contactMeta: DashPayContactMetaStore

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

    private var localAlias: String? {
        _ = contactMeta.version
        return contactMeta.alias(
            network: identity.network,
            owner: identity.identityId,
            contact: contactId
        )
    }

    private var localNote: String? {
        _ = contactMeta.version
        return contactMeta.note(
            network: identity.network,
            owner: identity.identityId,
            contact: contactId
        )
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
        return (try? wallet.getDashPayProfile(identityId: contactId)) ?? nil
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
        _ = contactMeta.version
        return contactMeta.isHidden(
            network: identity.network,
            owner: identity.identityId,
            contact: contactId
        )
    }

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
                footer: "An alias overrides this contact's display name. This device only.",
                initialValue: localAlias ?? "",
                identifierPrefix: "dashpay.detail.alias",
                onSave: { value in
                    contactMeta.setAlias(
                        value,
                        network: identity.network,
                        owner: identity.identityId,
                        contact: contactId
                    )
                }
            )
        }
        .sheet(isPresented: $showNoteEditor) {
            ContactLocalFieldEditor(
                title: "Note",
                prompt: "Anything to remember about this contact",
                footer: "Notes are private. This device only.",
                initialValue: localNote ?? "",
                identifierPrefix: "dashpay.detail.note",
                onSave: { value in
                    contactMeta.setNote(
                        value,
                        network: identity.network,
                        owner: identity.identityId,
                        contact: contactId
                    )
                }
            )
        }
        .task {
            refreshPayments()
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
                // §6.4 broken payment channel (G1c). Re-enables
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
                    // §6.4 loading: single inline ProgressView.
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
                // §6.4 error: keep the last-known list, caption only.
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
                    contactMeta.setHidden(
                        hidden,
                        network: identity.network,
                        owner: identity.identityId,
                        contact: contactId
                    )
                }
            )) {
                Label("Hide contact", systemImage: "eye.slash")
            }
            .accessibilityIdentifier("dashpay.detail.hideToggle")
        } header: {
            Text("Local settings")
        } footer: {
            // M2: device-local only — M3 backs these with synced
            // `contactInfo` documents and drops the label.
            Text("This device only — alias, note and hide are not synced to other devices.")
        }
    }

    // MARK: - Payment refresh

    /// One FFI read + one persistence pass; the `@Query` above picks
    /// the upserts up reactively.
    private func refreshPayments() {
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
