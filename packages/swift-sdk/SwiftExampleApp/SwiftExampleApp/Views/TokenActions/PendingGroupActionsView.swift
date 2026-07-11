import SwiftUI
import SwiftData
import SwiftDashSDK

/// Lists pending group-action proposals for a `(token, identity)` pair.
///
/// Computes the set of group positions the token's rules reference
/// (deduped across `MainGroup` / `Group:<n>`) and queries each via the
/// platform-wallet FFI. Each row navigates to `CoSignProposalView`
/// where the user can review and sign without re-entering the
/// proposal's parameters.
struct PendingGroupActionsView: View {
    let token: PersistentToken
    let identity: PersistentIdentity

    @EnvironmentObject var walletManager: PlatformWalletManager

    @State private var proposals: [DiscoveredProposal] = []
    @State private var isLoading: Bool = false
    @State private var loadError: String?

    /// Pairs a proposal with the group position it was discovered
    /// under, so the co-sign view knows which group to query for
    /// signers and which position to dispatch under.
    struct DiscoveredProposal: Identifiable, Equatable {
        let groupContractPosition: UInt16
        let proposal: TokenGroupAction
        var id: Data { proposal.actionId }
    }

    var body: some View {
        List {
            if isLoading {
                Section {
                    HStack {
                        ProgressView().controlSize(.small)
                        Text("Loading pending group actions…")
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                    }
                }
            } else if let loadError {
                Section {
                    Label(loadError, systemImage: "exclamationmark.triangle")
                        .font(.subheadline)
                        .foregroundColor(.red)
                }
            } else if proposals.isEmpty {
                Section {
                    Text("No pending group actions for this token.")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                }
            } else {
                Section("Pending proposals") {
                    ForEach(proposals) { entry in
                        NavigationLink {
                            CoSignProposalView(
                                token: token,
                                identity: identity,
                                proposal: entry.proposal,
                                groupContractPosition: entry.groupContractPosition
                            )
                        } label: {
                            ProposalRow(
                                proposal: entry.proposal,
                                identity: identity,
                                isUserProposer: entry.proposal.proposer == identity.identityId
                            )
                        }
                    }
                }
            }
        }
        .navigationTitle("Pending Group Actions")
        .navigationBarTitleDisplayMode(.inline)
        .task {
            await reload()
        }
        .refreshable {
            await reload()
        }
    }

    private var managedWallet: ManagedPlatformWallet? {
        guard let walletId = identity.wallet?.walletId else { return nil }
        return walletManager.wallet(for: walletId)
    }

    @MainActor
    private func reload() async {
        guard let wallet = managedWallet else {
            loadError = "Wallet not loaded"
            return
        }
        isLoading = true
        loadError = nil
        defer { isLoading = false }

        let positions = TokenGroupRuleResolver.relevantGroupPositions(for: token)
        guard !positions.isEmpty else {
            proposals = []
            return
        }

        var collected: [DiscoveredProposal] = []
        for position in positions {
            // The resolver already filters out positions that don't
            // fit in UInt16; re-validate to keep the cast trap-free.
            guard let groupContractPosition = UInt16(exactly: position) else {
                continue
            }
            do {
                let page = try await wallet.tokenPendingGroupActions(
                    contractId: token.contractId,
                    groupContractPosition: groupContractPosition,
                    status: .pending,
                    startAtActionId: nil,
                    limit: 0
                )
                for proposal in page {
                    collected.append(DiscoveredProposal(
                        groupContractPosition: groupContractPosition,
                        proposal: proposal
                    ))
                }
            } catch {
                loadError = error.localizedDescription
                return
            }
        }
        proposals = TokenGroupRuleResolver.dedupeByActionId(collected)
    }
}

/// One row in the proposal list — type label + summary + proposer +
/// "you signed?" hint.
private struct ProposalRow: View {
    let proposal: TokenGroupAction
    let identity: PersistentIdentity
    let isUserProposer: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Image(systemName: iconName)
                    .foregroundColor(.accentColor)
                Text(proposal.type.displayLabel)
                    .font(.headline)
                Spacer()
                if isUserProposer {
                    Text("Proposed by you")
                        .font(.caption2)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(Color.accentColor.opacity(0.15))
                        .clipShape(Capsule())
                }
            }
            Text(summary)
                .font(.subheadline)
                .foregroundColor(.primary)
            HStack(spacing: 6) {
                Text("Proposer")
                    .font(.caption2)
                    .foregroundColor(.secondary)
                Text(short(proposal.proposer.toBase58String()))
                    .font(.caption2.monospaced())
                    .foregroundColor(.secondary)
            }
            HStack(spacing: 6) {
                Text("Action")
                    .font(.caption2)
                    .foregroundColor(.secondary)
                Text(short(proposal.actionId.toBase58String()))
                    .font(.caption2.monospaced())
                    .foregroundColor(.secondary)
            }
        }
        .padding(.vertical, 4)
    }

    private var iconName: String {
        switch proposal.type {
        case .mint: return "plus.circle"
        case .burn: return "flame"
        case .freeze: return "snowflake"
        case .unfreeze: return "sun.max"
        case .destroyFrozenFunds: return "trash"
        case .pause: return "pause.circle"
        case .resume: return "play.circle"
        case .setPrice: return "tag"
        case .directPurchase: return "cart"
        case .other: return "questionmark.circle"
        }
    }

    private var summary: String {
        switch proposal.params {
        case .mint(let amount, let recipient, _):
            return "Mint \(amount) to \(short(recipient.toBase58String()))"
        case .burn(let amount, let burnFrom, _):
            return "Burn \(amount) from \(short(burnFrom.toBase58String()))"
        case .freeze(let target, _):
            return "Freeze \(short(target.toBase58String()))"
        case .unfreeze(let target, _):
            return "Unfreeze \(short(target.toBase58String()))"
        case .destroyFrozenFunds(let target, let amount, _):
            return "Destroy \(amount) frozen from \(short(target.toBase58String()))"
        case .pause:
            return "Pause all token operations"
        case .resume:
            return "Resume all token operations"
        case .setPrice(let price, _):
            if let price {
                return "Set direct purchase price to \(price) credits"
            }
            return "Disable direct purchase"
        case .directPurchase(let amount, let totalCost):
            return "Purchase \(amount) for \(totalCost) credits"
        case .other:
            return proposal.type.displayLabel
        }
    }

    private func short(_ s: String) -> String {
        guard s.count > 12 else { return s }
        return "\(s.prefix(6))..\(s.suffix(4))"
    }
}

// MARK: - Group rule resolution

/// Helpers for computing which group positions to query for a given
/// token. Mirrors the resolver logic in `TokenActionEvaluator`'s
/// `resolveTaker`/`resolveMainGroup` — we collect positions from every
/// group-capable rule on the token, dedupe, and ask Platform per
/// position.
enum TokenGroupRuleResolver {
    static func relevantGroupPositions(for token: PersistentToken) -> [Int] {
        var seen = Set<Int>()
        var ordered: [Int] = []
        for rule in groupCapableRules(token) {
            guard let position = position(of: rule, token: token) else { continue }
            if seen.insert(position).inserted {
                ordered.append(position)
            }
        }
        return ordered
    }

    /// Drop duplicate proposals by `actionId` — when multiple rules
    /// reference the same group, the same proposal can come back from
    /// independent queries. First occurrence wins; ordering matches
    /// the first hit.
    static func dedupeByActionId(
        _ proposals: [PendingGroupActionsView.DiscoveredProposal]
    ) -> [PendingGroupActionsView.DiscoveredProposal] {
        var seen = Set<Data>()
        var out: [PendingGroupActionsView.DiscoveredProposal] = []
        for entry in proposals {
            if seen.insert(entry.proposal.actionId).inserted {
                out.append(entry)
            }
        }
        return out
    }

    private static func groupCapableRules(_ token: PersistentToken) -> [ChangeControlRules] {
        // The rules below mirror the actions Waves 1-5 ship with
        // `GroupActionMode.propose` — only those can have pending
        // group proposals worth listing here.
        return [
            token.manualMintingRules,
            token.manualBurningRules,
            token.freezeRules,
            token.unfreezeRules,
            token.destroyFrozenFundsRules,
            token.emergencyActionRules,
            token.distributionChangeRules?.changeDirectPurchasePricingRules,
        ].compactMap { $0 }
    }

    private static func position(of rule: ChangeControlRules, token: PersistentToken) -> Int? {
        let authorized = rule.authorizedToMakeChange
        if authorized == AuthorizedActionTakers.mainGroup.rawValue {
            // Group positions are u16 over FFI; drop entries that don't
            // fit so callers can downcast safely.
            guard let main = token.mainControlGroupPosition,
                  UInt16(exactly: main) != nil
            else { return nil }
            return main
        }
        if authorized.hasPrefix("Group:"),
           let pos = Int(authorized.dropFirst("Group:".count)),
           UInt16(exactly: pos) != nil {
            return pos
        }
        return nil
    }
}
