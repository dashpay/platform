import SwiftUI
import SwiftData
import SwiftDashSDK

/// The two fee pots of a contract: the balance of each and its last explicit
/// claim, and, for an identity of a loaded wallet that may claim a pot, a
/// button that pays it out (state transition type 25, protocol version 14).
///
/// Who may claim which pot is Rust's answer
/// (`ManagedPlatformWallet.contractFeeClaimEligibility`), asked of each loaded
/// wallet on the current network; this view shows a claim button for exactly
/// the identities it names. The claim is signed on the Rust side through the
/// `KeychainSigner`. Whether the pot was already paid out this epoch and how
/// it is split are Platform's: a refusal is shown with its consensus code,
/// and a claim that went through shows the proved result Rust returns.
struct ContractFeePotsSection: View {
    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Query private var identities: [PersistentIdentity]
    /// The contract and the connection it was opened on: this screen reads
    /// only through that connection, and the hub hides it once replaced.
    let scope: ModerationScope
    private var contractId: String { scope.contractId }

    @StateObject private var loader = ModerationSingleLoader<ContractFeePots?>()
    private var load: ModerationLoad<ContractFeePots?> { loader.load }

    /// What the loaded wallets answered, merged: every identity some wallet
    /// named for a pot, with the wallet that named it.
    @State private var claimants: [FeePotClaimant] = []
    /// Who the moderators pot belongs to, as the first wallet answered.
    @State private var moderatorsPotTeam: ContractModeratorsPotTeam?
    @State private var eligibilityError: String?
    /// The identity (base58) picked to claim each pot, when several may.
    @State private var selectedClaimant: [ContractFeePot: String] = [:]
    @State private var claimingPot: ContractFeePot?
    @State private var claimResult: ClaimedContractFees?
    @State private var claimError: String?

    var body: some View {
        Section {
            switch load {
            case .idle, .loading:
                ModerationMessageRow(
                    kind: .loading,
                    text: "Loading fee pots…",
                    identifier: "contractModeration.feePots.loading"
                )
            case .failed(let message):
                ModerationMessageRow(
                    kind: .error,
                    text: message,
                    identifier: "contractModeration.feePots.error"
                )
            case .loaded(nil):
                ModerationMessageRow(
                    kind: .empty,
                    text: "No answer for this contract.",
                    identifier: "contractModeration.feePots.empty"
                )
            case .loaded(let pots?):
                FeePotRow(title: "Owner Pot", pot: pots.owner)
                    .accessibilityIdentifier("contractModeration.feePots.owner")
                claimControls(for: .owner)
                FeePotRow(title: "Moderators Pot", pot: pots.moderators)
                    .accessibilityIdentifier("contractModeration.feePots.moderators")
                if let moderatorsPotTeam {
                    ModerationFieldRow(label: "Moderators pot team", value: moderatorsPotTeam.title)
                        .accessibilityIdentifier("contractModeration.feePots.moderatorsTeam")
                }
                claimControls(for: .moderators)
            }
            if let eligibilityError {
                ModerationMessageRow(
                    kind: .error,
                    text: "Could not ask who may claim: \(eligibilityError)",
                    identifier: "contractModeration.feePots.eligibilityError"
                )
            }
            if let claimError {
                ModerationMessageRow(
                    kind: .error,
                    text: claimError,
                    identifier: "contractModeration.feePots.claimError"
                )
            }
            if let claimResult {
                FeeClaimResultView(result: claimResult, label: label(forIdentity:))
                    .accessibilityIdentifier("contractModeration.feePots.claimResult")
            }
            Button("Reload") {
                Task { await reloadAll() }
            }
            .disabled(isLoading || claimingPot != nil)
            .accessibilityIdentifier("contractModeration.feePots.reload")
        } header: {
            Text("Fee Pots")
        } footer: {
            Text("Each pot's balance from the contract's document action fees. The last claim is the last explicit claim, at most one per epoch; a membership change of an elected team can also pay out the moderators pot without changing it. A claim button shows for each identity of a loaded wallet that may claim the pot; Platform refuses a pot already paid out this epoch or too small to pay everyone a credit.")
        }
        .task(id: contractId) {
            clearClaim()
            await reloadAll()
        }
        // A replaced SDK: drop what was read through the old one and read
        // nothing through the new one; the hub's gate shows the notice instead
        .onChange(of: appState.sdkGeneration) { _, _ in
            loader.clear()
            claimants = []
            moderatorsPotTeam = nil
            eligibilityError = nil
            clearClaim()
        }
    }

    // MARK: - Claim controls

    @ViewBuilder
    private func claimControls(for pot: ContractFeePot) -> some View {
        let potClaimants = claimants.filter { $0.pot == pot }
        if !potClaimants.isEmpty {
            if potClaimants.count > 1 {
                Picker("Claim as", selection: claimantBinding(for: pot, among: potClaimants)) {
                    ForEach(potClaimants) { claimant in
                        Text(claimantLabel(claimant, among: potClaimants))
                            .tag(claimant.id)
                    }
                }
                .accessibleFormPicker("contractModeration.feePots.claimant.\(pot.rawValue)")
                .disabled(claimingPot != nil)
            }
            Button {
                if let claimant = chosenClaimant(for: pot, among: potClaimants) {
                    claim(pot, as: claimant)
                }
            } label: {
                HStack {
                    if claimingPot == pot {
                        ProgressView()
                            .controlSize(.small)
                        Text("Claiming…")
                    } else if potClaimants.count == 1, let only = potClaimants.first {
                        Text("Claim \(pot.title) as \(label(forIdentity: only.identityIdBase58))")
                    } else {
                        Text("Claim \(pot.title)")
                    }
                }
                .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .disabled(claimingPot != nil)
            .accessibilityIdentifier("contractModeration.feePots.claim.\(pot.rawValue)")
        }
    }

    private func claimantBinding(
        for pot: ContractFeePot,
        among potClaimants: [FeePotClaimant]
    ) -> Binding<String> {
        Binding(
            get: { chosenClaimant(for: pot, among: potClaimants)?.id ?? "" },
            set: { selectedClaimant[pot] = $0 }
        )
    }

    /// The picked claimant, or the first one Rust named.
    private func chosenClaimant(
        for pot: ContractFeePot,
        among potClaimants: [FeePotClaimant]
    ) -> FeePotClaimant? {
        if let picked = selectedClaimant[pot],
           let claimant = potClaimants.first(where: { $0.id == picked }) {
            return claimant
        }
        return potClaimants.first
    }

    /// A claimant's label in the picker: the identity, and the wallet it is
    /// claimed through when another wallet also holds that identity.
    private func claimantLabel(_ claimant: FeePotClaimant, among potClaimants: [FeePotClaimant]) -> String {
        let name = label(forIdentity: claimant.identityIdBase58)
        let heldTwice = potClaimants.contains {
            $0.identityIdBase58 == claimant.identityIdBase58 && $0.walletId != claimant.walletId
        }
        guard heldTwice else { return name }
        return "\(name) (wallet \(claimant.walletId.prefix(4).map { String(format: "%02x", $0) }.joined()))"
    }

    /// Forget the claim shown, its error and the picked claimants: they
    /// belong to the contract and connection they were made on.
    private func clearClaim() {
        claimResult = nil
        claimError = nil
        selectedClaimant = [:]
    }

    /// The alias of a local identity, or its base58 id.
    private func label(forIdentity base58: String) -> String {
        identities.first { $0.identityIdBase58 == base58 }?.alias ?? base58
    }

    private var isLoading: Bool {
        if case .loading = load { return true }
        return false
    }

    // MARK: - Reads

    private func reloadAll() async {
        await reload()
        await reloadEligibility()
    }

    private func reload() async {
        guard let connection = appState.moderationConnection(in: scope) else {
            loader.fail("SDK not initialized")
            return
        }
        let sdk = connection.sdk
        let appState = appState
        let contractId = contractId
        await loader.reload(
            { try await sdk.contractFeePots(contractId: contractId) },
            connectionIsCurrent: { appState.isCurrent(connection) }
        )
    }

    /// Ask every loaded wallet on the current network which of its
    /// identities may claim which pot, and keep the identities named. A
    /// wallet whose answer fails is skipped and its error shown; the others'
    /// answers stand. When every wallet fails nothing is offered.
    private func reloadEligibility() async {
        eligibilityError = nil
        guard appState.moderationConnection(in: scope) != nil,
              let contractIdBytes = Data.identifier(fromBase58: contractId) else {
            claimants = []
            moderatorsPotTeam = nil
            return
        }
        let network = appState.currentNetwork
        let walletIds = Set(
            identities
                .filter { $0.network == network }
                .compactMap { $0.wallet?.walletId }
        )
        var named: [FeePotClaimant] = []
        var team: ContractModeratorsPotTeam?
        var failures: [String] = []
        for walletId in walletIds.sorted(by: { $0.lexicographicallyPrecedes($1) }) {
            guard let wallet = walletManager.wallet(for: walletId) else { continue }
            do {
                let eligibility = try await wallet.contractFeeClaimEligibility(contractId: contractIdBytes)
                team = team ?? eligibility.moderatorsPotTeam
                for pot in ContractFeePot.allCases {
                    named += eligibility.claimants(of: pot).map {
                        FeePotClaimant(pot: pot, identityIdBase58: $0, walletId: walletId)
                    }
                }
            } catch {
                failures.append(error.localizedDescription)
            }
        }
        claimants = named
        moderatorsPotTeam = team
        if !failures.isEmpty {
            eligibilityError = Array(Set(failures)).sorted().joined(separator: "; ")
        }
    }

    // MARK: - Claim

    private func claim(_ pot: ContractFeePot, as claimant: FeePotClaimant) {
        guard let wallet = walletManager.wallet(for: claimant.walletId),
              let claimantId = Data.identifier(fromBase58: claimant.identityIdBase58),
              let contractIdBytes = Data.identifier(fromBase58: contractId) else {
            claimError = DocumentActionError.noWallet.localizedDescription
            return
        }
        // Fresh `KeychainSigner` per claim, as `CreateDocumentView` does.
        let signer = KeychainSigner(modelContainer: modelContext.container)
        claimingPot = pot
        claimError = nil
        claimResult = nil
        Task {
            do {
                claimResult = try await wallet.claimContractFees(
                    claimantIdentityId: claimantId,
                    contractId: contractIdBytes,
                    pot: pot,
                    signer: signer
                )
            } catch let error as PlatformWalletError {
                if let consensus = error.consensusError {
                    claimError = "Platform refused the claim (consensus code \(consensus.code)): \(error.localizedDescription)"
                } else {
                    claimError = error.localizedDescription
                }
            } catch {
                claimError = error.localizedDescription
            }
            claimingPot = nil
            await reload()
        }
    }
}

/// An identity some loaded wallet's Rust answer named for a pot, with that
/// wallet, which signs the claim. Two wallets can hold the same identity, so
/// the wallet is part of the id.
private struct FeePotClaimant: Identifiable, Hashable {
    let pot: ContractFeePot
    let identityIdBase58: String
    let walletId: Data
    var id: String {
        "\(pot.rawValue).\(identityIdBase58).\(walletId.map { String(format: "%02x", $0) }.joined())"
    }
}

private extension ContractFeePot {
    var title: String {
        switch self {
        case .owner: return "Owner Pot"
        case .moderators: return "Moderators Pot"
        }
    }
}

private extension ContractModeratorsPotTeam {
    var title: String {
        switch self {
        case .noModeration: return "None (no moderation declared)"
        case .declared: return "The declared team"
        case .electedInterim: return "The interim team (no charter seated)"
        case .electedSeated: return "The seated team (leader and active members)"
        }
    }
}

/// One pot: its balance and, once it was claimed, the last explicit claim.
private struct FeePotRow: View {
    let title: String
    let pot: ContractFeePotState

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title)
                .font(.headline)
            ModerationFieldRow(label: "Balance (credits)", value: "\(pot.credits)")
            if let epoch = pot.lastClaimEpoch {
                ModerationFieldRow(label: "Last claim epoch", value: "\(epoch)")
                if let timeMs = pot.lastClaimTimeMs {
                    ModerationFieldRow(label: "Last claim time", value: moderationDate(ms: timeMs))
                }
                if let claimant = pot.lastClaimantId {
                    ModerationFieldRow(label: "Last claimant", value: claimant, monospaced: true)
                }
            } else {
                Text("Never claimed")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
        .padding(.vertical, 2)
        .accessibilityElement(children: .combine)
    }
}

/// A claim as its proof showed it: the pot it left and the balances it paid.
private struct FeeClaimResultView: View {
    let result: ClaimedContractFees
    let label: (String) -> String

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Label("\(result.pot.title) claimed", systemImage: "checkmark.seal.fill")
                .foregroundColor(.green)
                .font(.headline)
            ModerationFieldRow(label: "Left in the pot (credits)", value: "\(result.potAfterClaim.credits)")
            if let epoch = result.potAfterClaim.lastClaimEpoch {
                ModerationFieldRow(label: "Last claim epoch", value: "\(epoch)")
            }
            if let claimant = result.potAfterClaim.lastClaimantId {
                ModerationFieldRow(label: "Last claimant", value: label(claimant), monospaced: true)
            }
            ForEach(result.balances, id: \.identityId) { entry in
                ModerationFieldRow(
                    label: "Balance of \(label(entry.identityId))",
                    value: "\(entry.balance)",
                    monospaced: true
                )
            }
        }
        .padding(.vertical, 2)
        .accessibilityElement(children: .combine)
    }
}
