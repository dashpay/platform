import CryptoKit
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
///
/// Rust names identities by role (owner, appointed, interim, seated team),
/// not by key: a named identity whose Keychain holds no usable CRITICAL key
/// still gets a control, and its moderation fails when sent with that error,
/// which the screen shows inline.
@MainActor
final class ContractModerationOfferStore: ObservableObject {
    /// One identity Rust named, with the wallet that holds it. Two wallets
    /// can hold the same identity, so the wallet is part of the id.
    struct Moderator: Identifiable, Hashable {
        let walletId: Data
        let offer: ContractIdentityModerationOffer
        var id: String { "\(offer.identityId).\(walletId.map { String(format: "%02x", $0) }.joined())" }
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
    /// The contract the answer shown is for.
    private var answeredContractId: String?

    /// Ask each wallet of `walletIds` what its identities may do on
    /// `contractId` (base58), and keep the merged answer. A wallet whose
    /// answer fails is skipped and its error shown; the others' answers
    /// stand. When every wallet fails nothing is offered.
    func reload(contractId: String, walletIds: [Data], walletManager: PlatformWalletManager) async {
        generation += 1
        let mine = generation
        isLoading = true
        errorMessage = nil
        // Another contract: the answer shown is not for it
        if answeredContractId != contractId {
            moderators = []
            moderatorsTeam = nil
            loadedOnce = false
        }
        answeredContractId = contractId
        guard let contractIdBytes = Data.identifier(fromBase58: contractId) else {
            moderators = []
            moderatorsTeam = nil
            isLoading = false
            loadedOnce = true
            return
        }
        var named: [Moderator] = []
        var team: ContractModeratorsPotTeam?
        var failures: [String] = []
        for walletId in walletIds {
            guard let wallet = walletManager.wallet(for: walletId) else { continue }
            do {
                let offer = try await wallet.contractModerationOffer(contractId: contractIdBytes)
                team = team ?? offer.moderatorsTeam
                named += offer.identities.map { Moderator(walletId: walletId, offer: $0) }
            } catch {
                failures.append(error.localizedDescription)
            }
        }
        guard mine == generation else { return }
        moderators = named
        moderatorsTeam = team
        if !failures.isEmpty {
            errorMessage = Array(Set(failures)).sorted().joined(separator: "; ")
        }
        isLoading = false
        loadedOnce = true
    }

    /// Drop the answer: it was read through a connection that was replaced.
    func clear() {
        generation += 1
        answeredContractId = nil
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
///
/// One file per document in its own folder under Application Support, named
/// by the SHA-256 of its contract, type and document id, and dropped once the
/// document is restored. Only the newest ``capacity`` files are kept: saving
/// one more evicts the oldest. That bound is storage housekeeping for this
/// app, not a rule about restores, which Platform alone decides.
struct ModeratedDocumentArchive {
    /// The folder the files live in.
    let directory: URL
    /// How many documents are kept at most.
    let capacity: Int

    /// The archive the app uses: `ModeratedDocuments` under Application
    /// Support, the newest 200 documents.
    static let standard = ModeratedDocumentArchive(
        directory: URL.applicationSupportDirectory.appending(path: "ModeratedDocuments", directoryHint: .isDirectory),
        capacity: 200
    )

    /// The file the document named by these ids is kept in.
    func fileURL(contractId: String, documentTypeName: String, documentId: String) -> URL {
        // NUL cannot appear in a base58 id or a document type name, so the
        // joined string names exactly one triple
        let name = [contractId, documentTypeName, documentId].joined(separator: "\u{0}")
        let digest = SHA256.hash(data: Data(name.utf8))
        let hex = digest.map { String(format: "%02x", $0) }.joined()
        return directory.appending(path: "\(hex).document", directoryHint: .notDirectory)
    }

    /// Keep `data` for the document these ids name, written atomically and
    /// read back. Returns whether it is now kept; `false` when it could not
    /// be written or did not read back the same, in which case no file is
    /// left for it.
    @discardableResult
    func save(_ data: Data, contractId: String, documentTypeName: String, documentId: String) -> Bool {
        let url = fileURL(contractId: contractId, documentTypeName: documentTypeName, documentId: documentId)
        do {
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            try data.write(to: url, options: .atomic)
            guard try Data(contentsOf: url) == data else {
                try? FileManager.default.removeItem(at: url)
                return false
            }
        } catch {
            try? FileManager.default.removeItem(at: url)
            return false
        }
        evictOldest(keeping: url)
        return true
    }

    /// The document these ids name as it was kept, `nil` when none is.
    func load(contractId: String, documentTypeName: String, documentId: String) -> Data? {
        try? Data(contentsOf: fileURL(contractId: contractId, documentTypeName: documentTypeName, documentId: documentId))
    }

    /// Whether a document is kept for these ids, without reading it.
    func contains(contractId: String, documentTypeName: String, documentId: String) -> Bool {
        FileManager.default.fileExists(
            atPath: fileURL(contractId: contractId, documentTypeName: documentTypeName, documentId: documentId).path
        )
    }

    /// Drop the document these ids name.
    func remove(contractId: String, documentTypeName: String, documentId: String) {
        try? FileManager.default.removeItem(
            at: fileURL(contractId: contractId, documentTypeName: documentTypeName, documentId: documentId)
        )
    }

    /// Remove the oldest files past ``capacity``, never `kept`, the file
    /// just saved. Oldest by modification date, then by name.
    private func evictOldest(keeping kept: URL) {
        let keys: [URLResourceKey] = [.contentModificationDateKey]
        guard let files = try? FileManager.default.contentsOfDirectory(
            at: directory,
            includingPropertiesForKeys: keys,
            options: [.skipsHiddenFiles]
        ), files.count > capacity else { return }
        let dated = files.map { url -> (url: URL, date: Date) in
            let date = (try? url.resourceValues(forKeys: Set(keys)).contentModificationDate) ?? .distantPast
            return (url, date)
        }
        let oldestFirst = dated.sorted {
            $0.date != $1.date ? $0.date < $1.date : $0.url.lastPathComponent < $1.url.lastPathComponent
        }
        var excess = files.count - capacity
        for file in oldestFirst where excess > 0 {
            guard file.url.standardizedFileURL != kept.standardizedFileURL else { continue }
            try? FileManager.default.removeItem(at: file.url)
            excess -= 1
        }
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
                    Text("\(label(moderator.identityIdBase58)) (\(moderator.offer.role.title))\(walletSuffix(moderator))")
                        .tag(moderator.id)
                }
            }
            .accessibleFormPicker(identifier)
        } else if let only = moderators.first {
            ModerationFieldRow(label: "Act as", value: "\(label(only.identityIdBase58)) (\(only.offer.role.title))")
                .accessibilityIdentifier(identifier)
        }
    }

    /// The wallet a moderator signs through, when another wallet also holds
    /// that identity; empty otherwise.
    private func walletSuffix(_ moderator: ContractModerationOfferStore.Moderator) -> String {
        let heldTwice = moderators.contains {
            $0.identityIdBase58 == moderator.identityIdBase58 && $0.walletId != moderator.walletId
        }
        guard heldTwice else { return "" }
        return " (wallet \(moderator.walletId.prefix(4).map { String(format: "%02x", $0) }.joined()))"
    }

    /// The picked moderator (its ``ContractModerationOfferStore/Moderator/id``),
    /// or the first one Rust named.
    static func chosen(
        _ selection: String,
        among moderators: [ContractModerationOfferStore.Moderator]
    ) -> ContractModerationOfferStore.Moderator? {
        moderators.first { $0.id == selection } ?? moderators.first
    }
}

/// An identity a moderation is about, for `sheet(item:)`.
struct ModerationTarget: Identifiable, Hashable {
    let identityId: String
    var id: String { identityId }
}
