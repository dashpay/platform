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
/// latest preview for the current inputs succeeded, and it hands that
/// preview back to Rust, which refuses to sign anything other than what it
/// shows. When the sheet opens, the wallet's copy of the identity is first
/// refreshed from Platform with a proof, and nothing is offered until that
/// succeeds, so the preview starts from the chain rather than from an older
/// or locally restored copy.
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
        /// Extend expiry is on but the picked date cannot be sent (before
        /// 1970). Kept apart from "no expiry asked for", so the change is
        /// never sent without the expiry the person asked for.
        var expiryUnrepresentable: Bool

        var isInvalid: Bool { budgetTextInvalid || expiryUnrepresentable }
        var isEmpty: Bool { addBudget == nil && expiresAt == nil && !isInvalid }
    }

    /// The preview of the inputs the form describes.
    private enum InputPreview {
        case none
        case checking
        case invalidAmount
        case invalidExpiry
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
    /// The last submission's outcome is unknown: it may have landed. No
    /// further submission is offered from this sheet, since a second one
    /// against the same reviewed limits could only be refused, for a fee.
    @State private var outcomeUnknown = false
    /// The wallet handle whose copy of the identity the opening refresh made
    /// current. Every later preview and the submission use this handle, and
    /// stop if the wallet manager now holds a different one for the id (the
    /// wallet was reloaded and its copy did not go through that refresh).
    @State private var refreshedWallet: ManagedPlatformWallet?
    /// Bumped when a submission starts and when its reviewed preview is
    /// offered again: a form preview started before then may not publish
    /// its answer over the reviewed one.
    @State private var previewEpoch = 0

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
                case .ready(let opening):
                    // Current limits and signer from the same preview as the
                    // new ones when there is one for what the form shows, so
                    // "now", "added" and "after" always agree.
                    currentSection(matchingInputPreview ?? opening)
                    changeSection(opening)
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
        // Frozen while a submission and its recovery run, so no new form
        // preview can start then.
        .disabled(isSubmitting || outcomeUnknown)
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
        case .invalidExpiry:
            Section("After the update") {
                Text("Pick an expiry date after 1970.")
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
                    Text("Signed by key #\(preview.signingKeyId)")
                        .font(.caption)
                        .foregroundColor(.secondary)
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
            .disabled(!canSubmit || isSubmitting || outcomeUnknown)
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
        let pickedExpiry = extendExpiry ? KeyLimitsFormatting.millis(from: newExpiry) : nil
        return Inputs(
            addBudget: parsedBudget,
            expiresAt: pickedExpiry,
            budgetTextInvalid: !text.isEmpty && parsedBudget == nil,
            expiryUnrepresentable: extendExpiry && pickedExpiry == nil
        )
    }

    /// The concrete preview for exactly what the form shows, if any.
    private var matchingInputPreview: ManagedPlatformWallet.IdentityKeyLimitsUpdatePreview? {
        guard case .ready(let preview, let previewed) = inputPreview, previewed == inputs else {
            return nil
        }
        return preview
    }

    private var canSubmit: Bool {
        guard !outcomeUnknown, case .ready(_, let previewed) = inputPreview else { return false }
        let current = inputs
        return previewed == current && !current.isEmpty && !current.isInvalid
    }

    // MARK: Rust calls

    private func resolveWallet() -> Outcome<ManagedPlatformWallet> {
        guard let walletId = identity.wallet?.walletId else {
            return .refused("Identity has no wallet linkage; cannot sign a key limits update.")
        }
        guard let wallet = walletManager.wallet(for: walletId) else {
            return .refused("Wallet not loaded in the wallet manager.")
        }
        if let refreshedWallet, refreshedWallet !== wallet {
            return .refused("The wallet was reloaded after this sheet read the key from Platform. Close the sheet and open it again.")
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
        // Start from the chain: a key added or raised on another client is
        // otherwise unknown to, or stale in, the wallet's copy, and a copy
        // restored from local rows must not choose the signing key.
        switch resolveWallet() {
        case .answered(let wallet):
            do {
                try await wallet.refreshIdentity(identityId: identity.identityId)
                refreshedWallet = wallet
            } catch {
                gate = .refused("Could not read this identity from Platform: \(error.localizedDescription). Close the sheet and try again.")
                return
            }
        case .refused(let message):
            gate = .refused(message)
            return
        }
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
        if inputs.expiryUnrepresentable {
            inputPreview = .invalidExpiry
            return
        }
        if inputs.isEmpty {
            inputPreview = .none
            return
        }
        inputPreview = .checking
        let epoch = previewEpoch
        // Light debounce: a new keystroke cancels this task.
        do {
            try await Task.sleep(nanoseconds: 300_000_000)
        } catch {
            return
        }
        let result = await preview(addBudget: inputs.addBudget, expiresAt: inputs.expiresAt)
        // Only an answer for what the form shows now, asked since the last
        // submission started, may replace the current one.
        guard !Task.isCancelled, inputs == self.inputs, epoch == previewEpoch else { return }
        switch result {
        case .answered(let preview):
            inputPreview = .ready(preview, for: inputs)
        case .refused(let reason):
            inputPreview = .refused(reason, for: inputs)
        }
    }

    @MainActor
    private func submit() async {
        // `canSubmit` is false while a submission is in flight only through
        // the disabled button; a queued tap must not start a second one.
        guard !isSubmitting, canSubmit, case .ready(let reviewed, _) = inputPreview else { return }
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
        previewEpoch += 1
        submitError = nil
        submitConsensusCode = nil

        do {
            let signer = KeychainSigner(modelContainer: modelContext.container)
            try await wallet.updateIdentityKeyLimits(asReviewed: reviewed, signer: signer)
            _ = signer  // keepalive: see KeychainSigner lifetime contract.
        } catch {
            let message = error.localizedDescription
            submitError = message
            submitConsensusCode = (error as? PlatformWalletError)?.consensusError?.code
            if let walletError = error as? PlatformWalletError,
               case .transactionBroadcastUnconfirmed = walletError {
                // It may have landed. Do not offer the same raise again.
                outcomeUnknown = true
                isSubmitting = false
                submitError = "\(message)\nThe update may have landed. Close this sheet and check the key before trying again."
                return
            }
            // Still locked as submitting. Only a refusal (a consensus error,
            // the broadcast node's word) says this transition was not
            // applied; any other failure may have come after the broadcast (a raise proved but not stored, a wallet
            // removed after the proof). Then ask the wallet for the key's
            // limits: only an answer showing exactly the reviewed ones lets
            // the same inputs be offered again without signing the increment
            // on top of a raise that landed.
            let refusedByPlatform = submitConsensusCode != nil
            var current: KeyLimitsRetryGate.Limits?
            if refusedByPlatform,
               case .answered(let now) = await preview(addBudget: nil, expiresAt: nil) {
                current = KeyLimitsRetryGate.Limits(totalBudget: now.currentTotalBudget, expiresAt: now.currentExpiresAt)
            }
            let reviewedLimits = KeyLimitsRetryGate.Limits(
                totalBudget: reviewed.currentTotalBudget,
                expiresAt: reviewed.currentExpiresAt
            )
            guard KeyLimitsRetryGate.mayOfferAgain(
                refusedByPlatform: refusedByPlatform, reviewed: reviewedLimits, current: current
            ) else {
                outcomeUnknown = true
                isSubmitting = false
                submitError = "\(message)\nThe wallet could not confirm that this update did not land. Close this sheet and check the key before trying again."
                return
            }
            isSubmitting = false
            // Refused: offer the reviewed preview again, its absolute limits
            // unchanged, if the form still shows what was submitted. A fresh
            // preview could read a copy a refresh moved meanwhile and offer
            // the increment on top of it; the reviewed one is refused by Rust
            // before a nonce if the copy moved. An edit made meanwhile has
            // its own preview.
            if change == inputs {
                previewEpoch += 1
                inputPreview = .ready(reviewed, for: change)
            }
            return
        }

        // The update landed and the wallet already stored the proved key
        // row through the persister. No second fetch here: a node one block
        // behind could answer with the limits from before the raise.
        isSubmitting = false
        onUpdated()
        dismiss()
    }
}

/// After a failed key limits update, whether the sheet may offer the same
/// change again. A UI guard, not a protocol rule. A consensus error is the
/// broadcast node's own word, not proved, but it is the only failure that
/// says the transition was refused; any other failure may have come after
/// the broadcast. Even then, only a wallet answer showing exactly the limits
/// the person reviewed lets the same change be offered again: moved limits
/// (another client's raise) and no answer at all (the wallet refused the
/// check) leave the outcome unknown. A repeat signs the same absolute limits,
/// so if the first did land after all, the repeat is refused as not a raise
/// (40221) and never added twice.
enum KeyLimitsRetryGate {
    struct Limits: Equatable {
        var totalBudget: UInt64?
        var expiresAt: UInt64?
    }

    static func mayOfferAgain(refusedByPlatform: Bool, reviewed: Limits, current: Limits?) -> Bool {
        refusedByPlatform && current == reviewed
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
