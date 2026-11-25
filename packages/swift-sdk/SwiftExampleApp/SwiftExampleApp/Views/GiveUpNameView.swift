import SwiftUI
import SwiftData
import SwiftDashSDK

/// Give up a DPNS name the identity holds: delete it (the name becomes
/// free) or tombstone it (the name is retired for good).
///
/// Which ways are offered comes only from
/// `ManagedPlatformWallet.dpnsNameGiveUpChoices`: the protocol's rules (a
/// contested name can only be tombstoned, the feature needs protocol
/// version 15) are applied in Rust, and this view shows what it reports.
/// The name leaves the identity's list through persistence once the network
/// confirmed it gone; this view never removes or hides rows itself. The one
/// write it makes is clearing the identity's main-name pick after a
/// successful give-up, the same write `SelectMainNameView` makes.
struct GiveUpNameView: View {
    let identity: PersistentIdentity
    /// The display label of the name, as the identity's name list shows it.
    let name: String

    @EnvironmentObject private var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    @State private var phase: Phase = .loading
    @State private var submitting: DpnsNameGiveUpMethod?
    @State private var submitError: String?
    @State private var confirmingDelete = false
    @State private var confirmingTombstone = false

    private enum Phase {
        case loading
        /// The network does not serve the feature yet; the message is the
        /// SDK's own.
        case notAvailable(String)
        /// This app's SDK is pinned to an older protocol version; the
        /// message is the SDK's own.
        case sdkPinned(String)
        case failed(String)
        case loaded(DpnsNameGiveUpChoices)
        /// A give-up was sent and may have been delivered, but was not
        /// confirmed. No further give-up is offered for this name here.
        case unconfirmed(String)
    }

    private var wallet: ManagedPlatformWallet? {
        guard let walletId = identity.wallet?.walletId else { return nil }
        return walletManager.wallet(for: walletId)
    }

    private var isMainName: Bool {
        GiveUpNameUI.isMainName(name, mainDpnsName: identity.mainDpnsName)
    }

    var body: some View {
        NavigationStack {
            Form {
                Section("Name") {
                    LabeledContent("DPNS name", value: "\(name).dash")
                    LabeledContent(
                        "Identity",
                        value: DpnsMarketplaceUI.shortIdentifier(identity.identityId)
                    )
                }

                if isMainName {
                    Section {
                        Label(GiveUpNameUI.mainNameNotice, systemImage: "star.fill")
                            .font(.callout)
                            .accessibilityIdentifier("giveUpName.mainNameNotice")
                    }
                }

                phaseSections
            }
            .navigationTitle("Give Up Name")
            .navigationBarTitleDisplayMode(.inline)
            .interactiveDismissDisabled(submitting != nil)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                        .disabled(submitting != nil)
                        .accessibilityIdentifier("giveUpName.cancel")
                }
            }
            .task { await loadChoices() }
            .confirmationDialog(
                "Delete \(name).dash?",
                isPresented: $confirmingDelete,
                titleVisibility: .visible
            ) {
                Button("Delete Name", role: .destructive) {
                    submit(.delete)
                }
                .accessibilityIdentifier("giveUpName.confirmDelete")
                Button("Cancel", role: .cancel) {}
            } message: {
                Text(GiveUpNameUI.deleteConfirmation(name: name))
            }
            .alert(
                "Tombstone \(name).dash permanently?",
                isPresented: $confirmingTombstone
            ) {
                Button("Tombstone Permanently", role: .destructive) {
                    submit(.tombstone)
                }
                .accessibilityIdentifier("giveUpName.confirmTombstone")
                Button("Cancel", role: .cancel) {}
            } message: {
                Text(GiveUpNameUI.tombstoneConfirmation(name: name))
            }
        }
    }

    @ViewBuilder
    private var phaseSections: some View {
        switch phase {
        case .loading:
            Section {
                HStack {
                    ProgressView()
                    Text("Checking how this name can be given up…")
                        .foregroundColor(.secondary)
                }
                .accessibilityIdentifier("giveUpName.loading")
            }
        case .notAvailable(let message):
            Section("Not available on this network yet") {
                Text(GiveUpNameUI.notAvailableExplanation)
                Text(message)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .textSelection(.enabled)
            }
            .accessibilityIdentifier("giveUpName.notAvailable")
        case .sdkPinned(let message):
            Section("This app needs an update") {
                Text(GiveUpNameUI.sdkPinnedExplanation)
                Text(message)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .textSelection(.enabled)
            }
            .accessibilityIdentifier("giveUpName.sdkPinned")
        case .unconfirmed(let message):
            Section("Sent, not confirmed yet") {
                Text(GiveUpNameUI.unconfirmedExplanation)
                Text(message)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .textSelection(.enabled)
            }
            .accessibilityIdentifier("giveUpName.unconfirmed")
        case .failed(let message):
            Section("Could not check this name") {
                Text(message)
                    .textSelection(.enabled)
                    .accessibilityIdentifier("giveUpName.error")
                Button("Retry") {
                    Task { await loadChoices() }
                }
                .accessibilityIdentifier("giveUpName.retry")
            }
        case .loaded(let choices):
            loadedSections(choices)
        }
    }

    @ViewBuilder
    private func loadedSections(_ choices: DpnsNameGiveUpChoices) -> some View {
        if choices.canDelete {
            Section {
                Text(GiveUpNameUI.deleteExplanation)
                    .font(.callout)
                Button(role: .destructive) {
                    confirmingDelete = true
                } label: {
                    actionLabel("Delete Name", method: .delete)
                }
                .disabled(submitting != nil)
                .accessibilityIdentifier("giveUpName.delete")
            } header: {
                Text("Delete")
            }
        }

        if choices.canTombstone {
            Section {
                if !choices.canDelete {
                    Text(GiveUpNameUI.tombstoneOnlyNotice)
                        .font(.callout)
                        .accessibilityIdentifier("giveUpName.tombstoneOnlyNotice")
                }
                Text(GiveUpNameUI.tombstoneExplanation)
                    .font(.callout)
                Button(role: .destructive) {
                    confirmingTombstone = true
                } label: {
                    actionLabel("Tombstone Name", method: .tombstone)
                }
                .disabled(submitting != nil)
                .accessibilityIdentifier("giveUpName.tombstone")
            } header: {
                Text("Tombstone")
            }
        }

        if !choices.canDelete && !choices.canTombstone {
            Section {
                Text("The network accepts no way to give up this name.")
                    .foregroundColor(.secondary)
                    .accessibilityIdentifier("giveUpName.noAction")
            }
        }

        if let submitError {
            Section("Give-up failed") {
                Text(submitError)
                    .foregroundColor(.red)
                    .textSelection(.enabled)
                    .accessibilityIdentifier("giveUpName.submitError")
            }
        }

        Section {
            Text(
                "The app may ask you to unlock the identity signing key. The name leaves "
                    + "your list only after Platform confirms it is gone."
            )
            .font(.caption)
            .foregroundColor(.secondary)
        }
    }

    private func actionLabel(_ title: String, method: DpnsNameGiveUpMethod) -> some View {
        HStack {
            Spacer()
            if submitting == method {
                ProgressView().controlSize(.small)
                Text("Confirming…")
                    .accessibilityIdentifier("giveUpName.submitting")
            } else {
                Text(title)
            }
            Spacer()
        }
    }

    @MainActor
    private func loadChoices() async {
        guard let wallet else {
            phase = .failed("This identity's wallet is not loaded.")
            return
        }
        phase = .loading
        do {
            let choices = try await wallet.dpnsNameGiveUpChoices(
                ownerIdentityId: identity.identityId,
                name: name
            )
            phase = .loaded(choices)
        } catch {
            switch GiveUpNameUI.loadFailure(error) {
            case .notAvailable(let message):
                phase = .notAvailable(message)
            case .sdkPinned(let message):
                phase = .sdkPinned(message)
            case .unconfirmed(let message):
                phase = .unconfirmed(message)
            case .failed(let message):
                phase = .failed(message)
            }
        }
    }

    private func submit(_ method: DpnsNameGiveUpMethod) {
        guard submitting == nil, case .loaded(let choices) = phase else { return }
        guard let wallet else {
            submitError = "This identity's wallet is not loaded."
            return
        }
        submitting = method
        submitError = nil
        let signer = KeychainSigner(modelContainer: modelContext.container)
        let identityId = identity.identityId
        let label = name

        Task {
            do {
                try await wallet.giveUpDpnsName(
                    ownerIdentityId: identityId,
                    name: label,
                    expectedDocumentId: choices.documentId,
                    method: method,
                    signer: signer
                )
                await MainActor.run {
                    clearMainNamePickIfGivenUp()
                    submitting = nil
                    dismiss()
                }
            } catch {
                await MainActor.run {
                    switch GiveUpNameUI.submitFailure(error) {
                    case .unconfirmed(let message):
                        submitError = nil
                        phase = .unconfirmed(message)
                    case .failed(let message):
                        submitError = message
                    }
                    submitting = nil
                }
            }
        }
    }

    /// After a confirmed give-up, drop the main-name pick when it still
    /// names this name, with the same write `SelectMainNameView` makes.
    /// Kept, the pick would silently become the main name again if this
    /// identity ever registered the deleted name anew. Never called before
    /// success or on a failure.
    @MainActor
    private func clearMainNamePickIfGivenUp() {
        guard GiveUpNameUI.isMainName(name, mainDpnsName: identity.mainDpnsName) else { return }
        PersistentIdentity.updateMainDpnsName(
            in: modelContext,
            identityId: identity.identityId,
            mainDpnsName: nil
        )
        try? modelContext.save()
    }
}

/// The owned name a "Give Up Name…" sheet is presented for.
struct GiveUpNameRequest: Identifiable {
    let name: String
    var id: String { name }
}

/// Wording and error routing of `GiveUpNameView`, kept out of the view so
/// tests can pin them.
enum GiveUpNameUI {
    enum LoadFailure: Equatable {
        /// The network does not serve giving up names yet.
        case notAvailable(String)
        /// This app's SDK is pinned below the protocol version.
        case sdkPinned(String)
        /// An earlier give-up of the name was sent and is not confirmed yet.
        case unconfirmed(String)
        case failed(String)
    }

    enum SubmitFailure: Equatable {
        /// The give-up may have been delivered: offer nothing more.
        case unconfirmed(String)
        case failed(String)
    }

    static let mainNameNotice =
        "This is your main name. After you give it up, it is no longer shown as your main name."

    static let notAvailableExplanation =
        "Names can be deleted or tombstoned from Platform protocol version 15. "
            + "This network does not serve that yet, so this name can not be given up here."

    static let sdkPinnedExplanation =
        "This app's SDK is fixed to an older Platform protocol version, so it can not give "
            + "names up. Update the app to use this feature."

    static let unconfirmedExplanation =
        "The give-up was sent but Platform has not confirmed it, so it may still take "
            + "effect. To avoid a second paid or permanent change, nothing more is offered "
            + "here. To update which names you hold, refresh the Name Marketplace later: "
            + "its sync finds names that left. A refresh does not show that the give-up failed."

    static let deleteExplanation =
        "Deleting makes the name free again. Anybody, including someone else, can then "
            + "register it, and anything still pointing at this name would resolve to them."

    static let tombstoneExplanation =
        "Tombstoning retires the name for good. Nobody can register it again, you included, "
            + "and nobody can undo it."

    static let tombstoneOnlyNotice =
        "The network accepts only a tombstone for this name: it can not be deleted."

    static func deleteConfirmation(name: String) -> String {
        "\(name).dash becomes free and anybody can register it, including someone else. "
            + "Anything still pointing at it would then resolve to them."
    }

    static func tombstoneConfirmation(name: String) -> String {
        "This is permanent. Nobody can ever register \(name).dash again, you included, "
            + "and nobody can undo a tombstone."
    }

    /// Whether `name` is the identity's main-name pick. Compares the
    /// normalized forms, as the persistence layer does for the pick.
    static func isMainName(_ name: String, mainDpnsName: String?) -> Bool {
        guard let mainDpnsName, !mainDpnsName.isEmpty else { return false }
        return PersistentDPNSName.normalize(mainDpnsName) == PersistentDPNSName.normalize(name)
    }

    static func loadFailure(_ error: Error) -> LoadFailure {
        switch error as? PlatformWalletError {
        case .protocolVersionNotActive(let message)?:
            return .notAvailable(message)
        case .sdkPinnedBelowProtocolVersion(let message)?:
            return .sdkPinned(message)
        case .transactionBroadcastUnconfirmed(let message)?:
            return .unconfirmed(message)
        default:
            return .failed(DpnsMarketplaceUI.error(error))
        }
    }

    static func submitFailure(_ error: Error) -> SubmitFailure {
        switch error as? PlatformWalletError {
        case .transactionBroadcastUnconfirmed(let message)?:
            return .unconfirmed(message)
        case .protocolVersionNotActive?:
            return .failed("Not available on this network yet. " + notAvailableExplanation)
        case .sdkPinnedBelowProtocolVersion?:
            return .failed(sdkPinnedExplanation)
        default:
            return .failed(DpnsMarketplaceUI.error(error))
        }
    }
}
