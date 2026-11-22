import SwiftData
import SwiftDashSDK
import SwiftUI

/// Sheet for raising one key's protocol 14 limits: adding to its budget
/// and moving its expiry later (`IdentityKeyLimitsUpdate`).
///
/// Every rule is Rust's. The sheet asks
/// `previewIdentityKeyLimitsUpdate` with no change when it opens (can this
/// key be raised at all, and by which key), then again whenever the inputs
/// change, and renders the limits the preview reports or Rust's refusal.
/// The only input handling here is turning the DASH text into credits and
/// the picked date into milliseconds. Submit is offered only while the
/// latest preview for the current inputs succeeded.
struct UpdateKeyLimitsView: View {
    let identity: PersistentIdentity
    let publicKey: IdentityPublicKey
    var onUpdated: () -> Void = {}

    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    /// The answer to "can this key be raised at all".
    private enum Gate {
        case loading
        case refused(String)
        case ready(ManagedPlatformWallet.IdentityKeyLimitsUpdatePreview)
    }

    /// The change the form currently describes, in protocol units.
    private struct Inputs: Equatable {
        var addBudget: UInt64?
        var expiresAt: UInt64?
        /// Budget text was typed but is not an amount of DASH.
        var budgetTextInvalid: Bool

        var isEmpty: Bool { addBudget == nil && expiresAt == nil && !budgetTextInvalid }
    }

    /// The preview of the inputs the form describes.
    private enum InputPreview {
        case none
        case checking
        case invalidAmount
        case ready(ManagedPlatformWallet.IdentityKeyLimitsUpdatePreview, for: Inputs)
        case refused(String, for: Inputs)
    }

    /// A wallet answer: a value, or the reason text to show.
    private enum Outcome<Value> {
        case answered(Value)
        case refused(String)
    }

    @State private var gate: Gate = .loading
    @State private var addBudgetText = ""
    @State private var extendExpiry = false
    @State private var newExpiry = Date()
    @State private var inputPreview: InputPreview = .none
    @State private var isSubmitting = false
    @State private var submitError: String?
    @State private var submitConsensusCode: UInt32?

    var body: some View {
        NavigationStack {
            Form {
                switch gate {
                case .loading:
                    Section {
                        HStack {
                            ProgressView().controlSize(.small)
                            Text("Checking this key")
                                .foregroundColor(.secondary)
                        }
                    }
                case .refused(let reason):
                    Section {
                        Text(reason)
                            .font(.callout)
                            .foregroundColor(.red)
                            .accessibilityIdentifier("keyLimits.update.error")
                    } header: {
                        Text("Key #\(publicKey.id) limits cannot be raised")
                    }
                case .ready(let current):
                    currentSection(current)
                    changeSection(current)
                    previewSection
                    submitSection
                }
            }
            .navigationTitle("Update Limits")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                        .disabled(isSubmitting)
                }
            }
            .task { await loadGate() }
            .task(id: inputs) { await previewInputs(inputs) }
        }
        .interactiveDismissDisabled(isSubmitting)
    }

    // MARK: Sections

    private func currentSection(_ current: ManagedPlatformWallet.IdentityKeyLimitsUpdatePreview) -> some View {
        Section("Key #\(publicKey.id) now") {
            if let total = current.currentTotalBudget {
                LabeledContent("Total budget", value: KeyLimitsFormatting.dash(total))
            }
            if let expiresAt = current.currentExpiresAt {
                LabeledContent("Expires") {
                    VStack(alignment: .trailing, spacing: 2) {
                        Text(KeyLimitsFormatting.expiryDate(expiresAt))
                        Text(KeyLimitsFormatting.expiryRelative(expiresAt, now: Date()))
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
            }
            LabeledContent("Signed by", value: "Key #\(current.signingKeyId)")
                .accessibilityIdentifier("keyLimits.update.signingKey")
        }
    }

    @ViewBuilder
    private func changeSection(_ current: ManagedPlatformWallet.IdentityKeyLimitsUpdatePreview) -> some View {
        Section {
            if current.currentTotalBudget != nil {
                TextField("Add to budget (DASH)", text: $addBudgetText)
                    .keyboardType(.decimalPad)
                    .autocorrectionDisabled()
                    .accessibilityIdentifier("keyLimits.update.addBudget")
            }
            if current.currentExpiresAt != nil {
                Toggle("Extend expiry", isOn: $extendExpiry)
                    .accessibilityIdentifier("keyLimits.update.extendExpiryToggle")
                if extendExpiry {
                    DatePicker(
                        "New expiry",
                        selection: $newExpiry,
                        displayedComponents: [.date, .hourAndMinute]
                    )
                    .accessibilityIdentifier("keyLimits.update.expiryPicker")
                }
            }
        } header: {
            Text("Change")
        } footer: {
            Text("The amount is added to the total budget and to what is left of it. Limits can only be raised.")
        }
    }

    @ViewBuilder
    private var previewSection: some View {
        switch inputPreview {
        case .none:
            EmptyView()
        case .checking:
            Section("After the update") {
                HStack {
                    ProgressView().controlSize(.small)
                    Text("Checking")
                        .foregroundColor(.secondary)
                }
            }
        case .invalidAmount:
            Section("After the update") {
                Text("Enter an amount of DASH, with at most 11 decimals.")
                    .font(.callout)
                    .foregroundColor(.red)
                    .accessibilityIdentifier("keyLimits.update.error")
            }
        case .refused(let reason, _):
            Section("After the update") {
                Text(reason)
                    .font(.callout)
                    .foregroundColor(.red)
                    .accessibilityIdentifier("keyLimits.update.error")
            }
        case .ready(let preview, _):
            Section("After the update") {
                VStack(alignment: .leading, spacing: 6) {
                    if let total = preview.newTotalBudget {
                        Text("New total budget \(KeyLimitsFormatting.dash(total))")
                    }
                    if let expiresAt = preview.newExpiresAt {
                        Text("New expiry \(KeyLimitsFormatting.expiryDate(expiresAt))")
                    }
                }
                .accessibilityElement(children: .combine)
                .accessibilityIdentifier("keyLimits.update.preview")
            }
        }
    }

    private var submitSection: some View {
        Section {
            Button {
                Task { await submit() }
            } label: {
                HStack {
                    if isSubmitting {
                        ProgressView().controlSize(.small)
                    }
                    Text("Update Limits")
                        .frame(maxWidth: .infinity)
                }
            }
            .buttonStyle(.borderedProminent)
            .disabled(!canSubmit || isSubmitting)
            .accessibilityIdentifier("keyLimits.update.submit")

            if let submitError {
                VStack(alignment: .leading, spacing: 4) {
                    Text(submitError)
                        .font(.callout)
                        .foregroundColor(.red)
                    if let code = submitConsensusCode {
                        Text("Consensus error \(code)")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
                .accessibilityElement(children: .combine)
                .accessibilityIdentifier("keyLimits.update.submitError")
            }
        }
    }

    // MARK: Inputs

    /// The form's inputs in protocol units. Fields the sheet does not show
    /// for this key are left empty, so they never reach the preview.
    private var inputs: Inputs {
        let text = addBudgetText.trimmingCharacters(in: .whitespacesAndNewlines)
        let parsedBudget = text.isEmpty ? nil : KeyLimitsFormatting.credits(fromDash: text)
        return Inputs(
            addBudget: parsedBudget,
            expiresAt: extendExpiry ? KeyLimitsFormatting.millis(from: newExpiry) : nil,
            budgetTextInvalid: !text.isEmpty && parsedBudget == nil
        )
    }

    private var canSubmit: Bool {
        guard case .ready(_, let previewed) = inputPreview else { return false }
        let current = inputs
        return previewed == current && !current.isEmpty && !current.budgetTextInvalid
    }

    // MARK: Rust calls

    private func resolveWallet() -> Outcome<ManagedPlatformWallet> {
        guard let walletId = identity.wallet?.walletId else {
            return .refused("Identity has no wallet linkage; cannot sign a key limits update.")
        }
        guard let wallet = walletManager.wallet(for: walletId) else {
            return .refused("Wallet not loaded in the wallet manager.")
        }
        return .answered(wallet)
    }

    @MainActor
    private func preview(addBudget: UInt64?, expiresAt: UInt64?) async -> Outcome<ManagedPlatformWallet.IdentityKeyLimitsUpdatePreview> {
        let wallet: ManagedPlatformWallet
        switch resolveWallet() {
        case .answered(let resolved): wallet = resolved
        case .refused(let message): return .refused(message)
        }
        do {
            let signer = KeychainSigner(modelContainer: modelContext.container)
            let preview = try await wallet.previewIdentityKeyLimitsUpdate(
                identityId: identity.identityId,
                keyId: publicKey.id,
                addBudget: addBudget,
                expiresAt: expiresAt,
                signer: signer
            )
            _ = signer  // keepalive: see KeychainSigner lifetime contract.
            return .answered(preview)
        } catch {
            return .refused(error.localizedDescription)
        }
    }

    @MainActor
    private func loadGate() async {
        switch await preview(addBudget: nil, expiresAt: nil) {
        case .answered(let current):
            if let expiresAt = current.currentExpiresAt {
                // A starting point for the picker only; Rust judges the pick.
                let base = max(Date(timeIntervalSince1970: TimeInterval(expiresAt) / 1000), Date())
                newExpiry = base.addingTimeInterval(30 * 24 * 60 * 60)
            }
            gate = .ready(current)
        case .refused(let reason):
            gate = .refused(reason)
        }
    }

    @MainActor
    private func previewInputs(_ inputs: Inputs) async {
        guard case .ready = gate else { return }
        if inputs.budgetTextInvalid {
            inputPreview = .invalidAmount
            return
        }
        if inputs.isEmpty {
            inputPreview = .none
            return
        }
        inputPreview = .checking
        // Light debounce: a new keystroke cancels this task.
        do {
            try await Task.sleep(nanoseconds: 300_000_000)
        } catch {
            return
        }
        let result = await preview(addBudget: inputs.addBudget, expiresAt: inputs.expiresAt)
        guard !Task.isCancelled else { return }
        switch result {
        case .answered(let preview):
            inputPreview = .ready(preview, for: inputs)
        case .refused(let reason):
            inputPreview = .refused(reason, for: inputs)
        }
    }

    @MainActor
    private func submit() async {
        guard canSubmit else { return }
        let change = inputs
        let wallet: ManagedPlatformWallet
        switch resolveWallet() {
        case .answered(let resolved): wallet = resolved
        case .refused(let message):
            submitError = message
            submitConsensusCode = nil
            return
        }

        isSubmitting = true
        submitError = nil
        submitConsensusCode = nil

        do {
            let signer = KeychainSigner(modelContainer: modelContext.container)
            try await wallet.updateIdentityKeyLimits(
                identityId: identity.identityId,
                keyId: publicKey.id,
                addBudget: change.addBudget,
                expiresAt: change.expiresAt,
                signer: signer
            )
            _ = signer  // keepalive: see KeychainSigner lifetime contract.
        } catch {
            isSubmitting = false
            submitError = error.localizedDescription
            submitConsensusCode = (error as? PlatformWalletError)?.consensusError?.code
            return
        }

        // The update landed. A failed refresh must not hide that: the
        // persister already wrote the key row with the new limits.
        if let sdk = appState.sdk {
            do {
                try await IdentityKeyRefresher.refreshBalanceAndKeys(
                    identity: identity,
                    sdk: sdk,
                    modelContext: modelContext
                )
            } catch {
                print("🔑 UpdateKeyLimitsView: refresh after the update failed: \(error.localizedDescription)")
            }
        }

        isSubmitting = false
        onUpdated()
        dismiss()
    }
}

/// Settings builder entry: the keys of this network's identities that
/// carry limits, each opening `UpdateKeyLimitsView`.
struct KeyLimitsKeysListView: View {
    @EnvironmentObject var appState: AppState
    @Query(sort: \PersistentIdentity.lastUpdated, order: .reverse)
    private var identities: [PersistentIdentity]

    private struct Selection: Identifiable {
        let identity: PersistentIdentity
        let key: IdentityPublicKey
        var id: String { "\(identity.identityIdBase58)/\(key.id)" }
    }

    private struct Row: Identifiable {
        let identity: PersistentIdentity
        let keys: [IdentityPublicKey]
        var id: String { identity.identityIdBase58 }
    }

    @State private var selection: Selection?

    private var rows: [Row] {
        let network = appState.currentNetwork.rawValue
        return identities
            .filter { $0.networkRaw == network && $0.wallet != nil }
            .map { identity in
                Row(
                    identity: identity,
                    keys: identity.identityPublicKeys.filter(\.hasLimits).sorted { $0.id < $1.id }
                )
            }
            .filter { !$0.keys.isEmpty }
    }

    var body: some View {
        List {
            if rows.isEmpty {
                Text("No identity on this device has a key with a budget or an expiry. Add one from an identity's keys first.")
                    .foregroundColor(.secondary)
            }
            ForEach(rows) { row in
                Section(row.identity.displayName) {
                    ForEach(row.keys, id: \.id) { key in
                        Button {
                            selection = Selection(identity: row.identity, key: key)
                        } label: {
                            KeyRowView(publicKey: key, privateKeyAvailable: false)
                        }
                        .foregroundColor(.primary)
                        .accessibilityIdentifier("keyLimits.update.keyRow.\(key.id)")
                    }
                }
            }
        }
        .navigationTitle("Update Key Limits")
        .navigationBarTitleDisplayMode(.inline)
        .sheet(item: $selection) { selection in
            UpdateKeyLimitsView(identity: selection.identity, publicKey: selection.key)
        }
    }
}
