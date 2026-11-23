import SwiftUI
import SwiftDashSDK

/// What the loaded wallets may do on one contract as its moderators: Rust's
/// answer (`ManagedPlatformWallet.contractModerationOffer`), asked of each
/// loaded wallet on the current network and merged, each identity with the
/// wallet that holds it and signs for it.
///
/// The screens show a moderation control only for an identity this names,
/// and only the actions Rust listed for it. Whether the target, document or
/// team action admits the moderation is Platform's to answer when it is sent.
@MainActor
final class ContractModerationOfferStore: ObservableObject {
    /// One identity Rust named, with the wallet that holds it.
    struct Moderator: Identifiable, Hashable {
        let walletId: Data
        let offer: ContractIdentityModerationOffer
        var id: String { offer.identityId }
        var identityIdBase58: String { offer.identityId }
    }

    @Published private(set) var moderators: [Moderator] = []
    /// Who moderates the contract, as the first wallet answered.
    @Published private(set) var moderatorsTeam: ContractModeratorsPotTeam?
    @Published private(set) var errorMessage: String?
    @Published private(set) var isLoading = false
    @Published private(set) var loadedOnce = false

    /// Bumped on every reload and clear, so an answer to a superseded one is
    /// dropped.
    private var generation = 0

    /// Ask each wallet of `walletIds` what its identities may do on
    /// `contractId` (base58), and keep the merged answer.
    func reload(contractId: String, walletIds: [Data], walletManager: PlatformWalletManager) async {
        generation += 1
        let mine = generation
        isLoading = true
        errorMessage = nil
        guard let contractIdBytes = Data.identifier(fromBase58: contractId) else {
            moderators = []
            moderatorsTeam = nil
            isLoading = false
            loadedOnce = true
            return
        }
        var named: [Moderator] = []
        var team: ContractModeratorsPotTeam?
        do {
            for walletId in walletIds {
                guard let wallet = walletManager.wallet(for: walletId) else { continue }
                let offer = try await wallet.contractModerationOffer(contractId: contractIdBytes)
                team = team ?? offer.moderatorsTeam
                named += offer.identities.map { Moderator(walletId: walletId, offer: $0) }
            }
            guard mine == generation else { return }
            moderators = named
            moderatorsTeam = team
        } catch {
            guard mine == generation else { return }
            moderators = []
            moderatorsTeam = nil
            errorMessage = error.localizedDescription
        }
        isLoading = false
        loadedOnce = true
    }

    /// Drop the answer: it was read through a connection that was replaced.
    func clear() {
        generation += 1
        moderators = []
        moderatorsTeam = nil
        errorMessage = nil
        isLoading = false
        loadedOnce = false
    }

    /// The identities Rust lets moderate an identity on the contract's lists.
    var userModerators: [Moderator] {
        moderators.filter { !$0.offer.userActions.isEmpty }
    }

    /// The identities Rust lets act on documents of `documentTypeName`.
    func documentModerators(of documentTypeName: String) -> [Moderator] {
        moderators.filter { $0.offer.documentType(named: documentTypeName) != nil }
    }

    /// The identities Rust lets restore documents of `documentTypeName`.
    func restorers(of documentTypeName: String) -> [Moderator] {
        moderators.filter { $0.offer.documentType(named: documentTypeName)?.restore == true }
    }

    /// The identities Rust lets approve team actions.
    var approvers: [Moderator] {
        moderators.filter(\.offer.approveTeamActions)
    }

    /// The document types some identity may act on, by name.
    var documentTypeNames: [String] {
        Array(Set(moderators.flatMap { $0.offer.documentTypes.map(\.documentTypeName) })).sorted()
    }
}

/// The documents this app deleted as a moderator, as they were just before
/// (`ContractDocumentDeletion.documentData`), kept on this device so a
/// moderator can restore one: Platform keeps only the document's hash.
/// Stored in `UserDefaults` by contract, type and document id, and dropped
/// once the document is restored.
enum ModeratedDocumentArchive {
    private static func key(contractId: String, documentTypeName: String, documentId: String) -> String {
        "contractModeration.deletedDocument.\(contractId).\(documentTypeName).\(documentId)"
    }

    static func save(_ data: Data, contractId: String, documentTypeName: String, documentId: String) {
        UserDefaults.standard.set(
            data,
            forKey: key(contractId: contractId, documentTypeName: documentTypeName, documentId: documentId)
        )
    }

    static func load(contractId: String, documentTypeName: String, documentId: String) -> Data? {
        UserDefaults.standard.data(
            forKey: key(contractId: contractId, documentTypeName: documentTypeName, documentId: documentId)
        )
    }

    static func remove(contractId: String, documentTypeName: String, documentId: String) {
        UserDefaults.standard.removeObject(
            forKey: key(contractId: contractId, documentTypeName: documentTypeName, documentId: documentId)
        )
    }
}

/// The text a failed moderation shows: Platform's refusal with its consensus
/// code, or the error as it is.
func moderationWriteErrorText(_ error: Error) -> String {
    if let error = error as? PlatformWalletError, let consensus = error.consensusError {
        return "Platform refused (consensus code \(consensus.code)): \(error.localizedDescription)"
    }
    return error.localizedDescription
}

extension ContractModeratorRole {
    var title: String {
        switch self {
        case .owner: return "Contract owner"
        case .appointedModerator: return "Appointed moderator"
        case .interimModerator: return "Interim moderator"
        case .teamLeader: return "Team leader"
        case .teamMember: return "Team member"
        }
    }
}

extension ContractUserModerationAction {
    var title: String {
        switch self {
        case .ban: return "Ban"
        case .unban: return "Unban"
        case .suspend: return "Suspend"
        case .unsuspend: return "Unsuspend"
        case .warn: return "Warn"
        case .clearWarnings: return "Clear Warnings"
        }
    }
}

/// The reason a moderator gives: free text and an optional `reason`
/// document of the moderation charters contract (base58), which a seated
/// team names from its proposal's list.
struct ModerationReasonFields: View {
    @Binding var text: String
    @Binding var reasonDocumentId: String
    let identifierPrefix: String

    var body: some View {
        TextField("Reason", text: $text, axis: .vertical)
            .accessibilityIdentifier("\(identifierPrefix).reasonText")
        TextField("Reason document ID (base58, optional)", text: $reasonDocumentId)
            .textInputAutocapitalization(.never)
            .autocorrectionDisabled()
            .font(.system(.body, design: .monospaced))
            .accessibilityIdentifier("\(identifierPrefix).reasonDocumentId")
    }

    /// The reason as the wallet takes it; `nil` when the document id typed
    /// is not a base58 identifier.
    static func input(text: String, reasonDocumentId: String) -> ContractModerationReasonInput? {
        let trimmed = reasonDocumentId.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty {
            return ContractModerationReasonInput(text: text)
        }
        guard let id = Data.identifier(fromBase58: trimmed) else { return nil }
        return ContractModerationReasonInput(text: text, reasonDocumentId: id)
    }
}

/// A picker of the identities Rust named for an action, shown only when
/// there is more than one.
struct ModeratorPicker: View {
    let moderators: [ContractModerationOfferStore.Moderator]
    @Binding var selection: String
    let label: (String) -> String
    let identifier: String

    var body: some View {
        if moderators.count > 1 {
            Picker("Act as", selection: $selection) {
                ForEach(moderators) { moderator in
                    Text("\(label(moderator.identityIdBase58)) (\(moderator.offer.role.title))")
                        .tag(moderator.identityIdBase58)
                }
            }
            .accessibleFormPicker(identifier)
        } else if let only = moderators.first {
            ModerationFieldRow(label: "Act as", value: "\(label(only.identityIdBase58)) (\(only.offer.role.title))")
                .accessibilityIdentifier(identifier)
        }
    }

    /// The picked moderator, or the first one Rust named.
    static func chosen(
        _ selection: String,
        among moderators: [ContractModerationOfferStore.Moderator]
    ) -> ContractModerationOfferStore.Moderator? {
        moderators.first { $0.identityIdBase58 == selection } ?? moderators.first
    }
}

/// An identity a moderation is about, for `sheet(item:)`.
struct ModerationTarget: Identifiable, Hashable {
    let identityId: String
    var id: String { identityId }
}
