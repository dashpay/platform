import SwiftUI
import SwiftData
import SwiftDashSDK

// MARK: - Permission Result
//
// Three states per row:
// - `.allowed`           : the row is enabled and tappable.
// - `.denied(reason)`    : the row is rendered but disabled, with
//                          `reason` shown as a subtitle so the user
//                          understands *why* they can't do this.
// - `.hidden`             : the row is omitted from the list entirely.
//                          Used when the action genuinely doesn't exist
//                          for this token configuration (e.g. claim row
//                          on a token that doesn't allow choosing
//                          destination and the identity isn't the
//                          designated recipient).
enum TokenActionPermission: Equatable {
    case allowed
    case denied(reason: String)
    case hidden

    var isAllowed: Bool {
        if case .allowed = self { return true }
        return false
    }

    var isHidden: Bool {
        if case .hidden = self { return true }
        return false
    }

    var deniedReason: String? {
        if case .denied(let reason) = self { return reason }
        return nil
    }
}

// MARK: - Action Kind

/// Every token action this view knows how to render. Order in the
/// declaration drives the order they show up in the list (Transfer is
/// surfaced first via a dedicated insertion in the ordering helper —
/// the rest fall in declaration order).
enum TokenActionKind: String, CaseIterable, Identifiable {
    case transfer
    case mint
    case burn
    case freeze
    case unfreeze
    case destroyFrozenFunds
    case pause
    case resume
    case claim
    case directPurchase
    case setConventions
    case updateMaxSupply
    case changeDistribution

    var id: String { rawValue }

    var title: String {
        switch self {
        case .transfer: return "Transfer"
        case .mint: return "Mint"
        case .burn: return "Burn"
        case .freeze: return "Freeze"
        case .unfreeze: return "Unfreeze"
        case .destroyFrozenFunds: return "Destroy Frozen Funds"
        case .pause: return "Pause"
        case .resume: return "Resume"
        case .claim: return "Claim"
        case .directPurchase: return "Direct Purchase"
        case .setConventions: return "Set / Change Conventions"
        case .updateMaxSupply: return "Update Max Supply"
        case .changeDistribution: return "Change Distribution Rules"
        }
    }

    var subtitle: String {
        switch self {
        case .transfer: return "Move tokens to another identity"
        case .mint: return "Issue new tokens"
        case .burn: return "Destroy tokens you hold"
        case .freeze: return "Freeze a holder's balance"
        case .unfreeze: return "Unfreeze a holder's balance"
        case .destroyFrozenFunds: return "Permanently destroy frozen funds"
        case .pause: return "Halt all token operations"
        case .resume: return "Resume operations after a pause"
        case .claim: return "Claim distribution rewards"
        case .directPurchase: return "Buy tokens at the listed price"
        case .setConventions: return "Update token name / localizations"
        case .updateMaxSupply: return "Change the maximum supply"
        case .changeDistribution: return "Change perpetual / pre-programmed distribution"
        }
    }

    var icon: String {
        switch self {
        case .transfer: return "arrow.right.arrow.left"
        case .mint: return "plus.circle"
        case .burn: return "flame"
        case .freeze: return "snowflake"
        case .unfreeze: return "sun.max"
        case .destroyFrozenFunds: return "trash"
        case .pause: return "pause.circle"
        case .resume: return "play.circle"
        case .claim: return "gift"
        case .directPurchase: return "cart"
        case .setConventions: return "textformat"
        case .updateMaxSupply: return "chart.line.uptrend.xyaxis"
        case .changeDistribution: return "slider.horizontal.3"
        }
    }
}

// MARK: - Evaluator

/// Resolves "is this identity allowed to perform this action on this
/// token, given the contract's groups configuration?"
///
/// The evaluator only handles the easy cases of `authorizedToMakeChange`
/// for now — `noOne`, `contractOwner`, `identity:<id>`, `mainGroup`, and
/// `group:<position>`. The harder cases (nested groups, admin-action-takers
/// overrides) are surfaced as `.denied(reason: "TODO: ...")` so they're
/// visible in the UI rather than silently treated as "allowed". When the
/// permission can't be determined (e.g. the contract row isn't loaded),
/// the row falls back to "Group data unavailable" rather than crashing.
enum TokenActionEvaluator {
    /// Evaluate a `ChangeControlRules` against a candidate identity.
    /// `feature` is the human-readable name used in the denial reason
    /// (e.g. "Manual minting") so the row subtitle reads naturally.
    static func evaluate(
        rule: ChangeControlRules?,
        feature: String,
        identity: PersistentIdentity,
        contract: PersistentDataContract?,
        token: PersistentToken
    ) -> TokenActionPermission {
        guard let rule = rule else {
            return .denied(reason: "\(feature) is not configured for this token")
        }

        return resolveTaker(
            authorized: rule.authorizedToMakeChange,
            feature: feature,
            identity: identity,
            contract: contract,
            token: token
        )
    }

    /// Resolve a single `authorizedToMakeChange` string into a
    /// permission for this identity. The string format mirrors what
    /// `DataContractParser` writes via `AuthorizedActionTakers`:
    /// - "NoOne"
    /// - "ContractOwner"
    /// - "MainGroup"
    /// - "Identity:<base58>"
    /// - "Group:<position>"
    /// Anything else lands in the TODO bucket.
    static func resolveTaker(
        authorized: String,
        feature: String,
        identity: PersistentIdentity,
        contract: PersistentDataContract?,
        token: PersistentToken
    ) -> TokenActionPermission {
        // Easy cases first.
        if authorized == AuthorizedActionTakers.noOne.rawValue {
            return .denied(reason: "\(feature): no one is authorized")
        }

        if authorized == AuthorizedActionTakers.contractOwner.rawValue {
            guard let ownerId = contract?.ownerId else {
                return .denied(reason: "\(feature): contract owner unknown locally")
            }
            return ownerId == identity.identityId
                ? .allowed
                : .denied(reason: "\(feature): only the contract owner")
        }

        if authorized == AuthorizedActionTakers.mainGroup.rawValue {
            return resolveMainGroup(
                feature: feature,
                identity: identity,
                contract: contract,
                token: token
            )
        }

        if let prefix = "Identity:".asPrefixOf(authorized) {
            let idBase58 = prefix
            guard let target = Data.identifier(fromBase58: idBase58) else {
                return .denied(reason: "\(feature): authorized identity unparseable")
            }
            return target == identity.identityId
                ? .allowed
                : .denied(reason: "\(feature): only identity \(short(idBase58))")
        }

        if let posStr = "Group:".asPrefixOf(authorized), let position = Int(posStr) {
            return resolveGroup(
                position: position,
                feature: feature,
                identity: identity,
                contract: contract
            )
        }

        // TODO: nested groups and admin-action-takers overrides — the
        // shape is documented in rs-dpp's TokenConfigurationConvention
        // but we don't have a parsed Swift model for it yet. Surface
        // the gap rather than silently green-lighting the action.
        return .denied(reason: "\(feature): authorization rule '\(authorized)' not yet evaluated")
    }

    private static func resolveMainGroup(
        feature: String,
        identity: PersistentIdentity,
        contract: PersistentDataContract?,
        token: PersistentToken
    ) -> TokenActionPermission {
        guard let position = token.mainControlGroupPosition else {
            return .denied(reason: "\(feature): main control group not configured")
        }
        return resolveGroup(
            position: position,
            feature: feature,
            identity: identity,
            contract: contract
        )
    }

    private static func resolveGroup(
        position: Int,
        feature: String,
        identity: PersistentIdentity,
        contract: PersistentDataContract?
    ) -> TokenActionPermission {
        guard let contract = contract else {
            return .denied(reason: "\(feature): contract not loaded")
        }
        guard let groups = contract.groups else {
            return .denied(reason: "\(feature): group data unavailable")
        }
        // groups is `[String: [String: Any]]` keyed by position string;
        // each entry has `members: [base58Id: power]`. See parser at
        // packages/swift-sdk/Sources/SwiftDashSDK/Core/Utils/DataContractParser.swift.
        guard let group = groups[String(position)] as? [String: Any] else {
            return .denied(reason: "\(feature): group \(position) not found")
        }
        guard let members = group["members"] as? [String: Any] else {
            return .denied(reason: "\(feature): group \(position) has no members")
        }
        let identityBase58 = identity.identityIdBase58
        return members[identityBase58] != nil
            ? .allowed
            : .denied(reason: "\(feature): not in group \(position)")
    }

    private static func short(_ s: String) -> String {
        guard s.count > 12 else { return s }
        return "\(s.prefix(6))..\(s.suffix(4))"
    }
}

// Local `String` helper so the prefix-strip reads cleanly above.
private extension String {
    /// If `self` is a prefix of `value`, return the rest of `value`
    /// (i.e. the part after the prefix). Otherwise nil.
    func asPrefixOf(_ value: String) -> String? {
        guard value.hasPrefix(self) else { return nil }
        return String(value.dropFirst(self.count))
    }
}

// MARK: - Resolved Action

/// A single computed row, ready to render.
struct ResolvedTokenAction: Identifiable {
    let kind: TokenActionKind
    let permission: TokenActionPermission

    var id: String { kind.id }
}

// MARK: - Action Resolution

/// Pulls together the full list of actions for a `(token, identity)`
/// pair. The result is already in display order; the caller just
/// filters out `.hidden` rows and renders.
enum TokenActionResolver {
    static func resolve(
        token: PersistentToken,
        identity: PersistentIdentity
    ) -> [ResolvedTokenAction] {
        let contract = token.dataContract

        // Transfer always shows; gated on paused state and the
        // (currently unmodelled) "transfer-restricted-to-X" rule.
        // PersistentToken doesn't carry a transferRules field today,
        // so the only signal we can act on is `isPaused`. Surface a
        // TODO subtitle when we get into the unmodelled territory.
        let transfer: TokenActionPermission = {
            if token.isPaused {
                return .denied(reason: "Token is paused")
            }
            // TODO: token transfer-restriction rules (per-identity / per-group)
            // aren't on PersistentToken yet. When they land, evaluate them here.
            return .allowed
        }()

        var rows: [ResolvedTokenAction] = []
        rows.append(ResolvedTokenAction(kind: .transfer, permission: transfer))

        // Mint
        if token.canManuallyMint {
            let perm = TokenActionEvaluator.evaluate(
                rule: token.manualMintingRules,
                feature: "Manual minting",
                identity: identity,
                contract: contract,
                token: token
            )
            let final = applyPausedGuard(perm, token: token, allowWhenPaused: false)
            rows.append(ResolvedTokenAction(kind: .mint, permission: final))
        } else {
            rows.append(ResolvedTokenAction(
                kind: .mint,
                permission: .denied(reason: "Manual minting not enabled for this token")
            ))
        }

        // Burn
        if token.canManuallyBurn {
            let perm = TokenActionEvaluator.evaluate(
                rule: token.manualBurningRules,
                feature: "Manual burning",
                identity: identity,
                contract: contract,
                token: token
            )
            let final = applyPausedGuard(perm, token: token, allowWhenPaused: false)
            rows.append(ResolvedTokenAction(kind: .burn, permission: final))
        } else {
            rows.append(ResolvedTokenAction(
                kind: .burn,
                permission: .denied(reason: "Manual burning not enabled for this token")
            ))
        }

        // Freeze
        if token.canFreeze {
            let perm = TokenActionEvaluator.evaluate(
                rule: token.freezeRules,
                feature: "Freezing",
                identity: identity,
                contract: contract,
                token: token
            )
            rows.append(ResolvedTokenAction(kind: .freeze, permission: perm))
        } else {
            rows.append(ResolvedTokenAction(
                kind: .freeze,
                permission: .denied(reason: "Freezing not enabled for this token")
            ))
        }

        // Unfreeze
        if token.canUnfreeze {
            let perm = TokenActionEvaluator.evaluate(
                rule: token.unfreezeRules,
                feature: "Unfreezing",
                identity: identity,
                contract: contract,
                token: token
            )
            rows.append(ResolvedTokenAction(kind: .unfreeze, permission: perm))
        } else {
            rows.append(ResolvedTokenAction(
                kind: .unfreeze,
                permission: .denied(reason: "Unfreezing not enabled for this token")
            ))
        }

        // Destroy frozen funds
        if token.canDestroyFrozenFunds {
            let perm = TokenActionEvaluator.evaluate(
                rule: token.destroyFrozenFundsRules,
                feature: "Destroy frozen funds",
                identity: identity,
                contract: contract,
                token: token
            )
            rows.append(ResolvedTokenAction(kind: .destroyFrozenFunds, permission: perm))
        } else {
            rows.append(ResolvedTokenAction(
                kind: .destroyFrozenFunds,
                permission: .denied(reason: "Destroying frozen funds not enabled")
            ))
        }

        // Emergency action -> Pause + Resume
        if token.hasEmergencyActions {
            let basePerm = TokenActionEvaluator.evaluate(
                rule: token.emergencyActionRules,
                feature: "Emergency action",
                identity: identity,
                contract: contract,
                token: token
            )
            // Pause is unavailable while already paused.
            let pausePerm: TokenActionPermission = {
                if case .allowed = basePerm, token.isPaused {
                    return .denied(reason: "Token is already paused")
                }
                return basePerm
            }()
            // Resume is unavailable while not paused.
            let resumePerm: TokenActionPermission = {
                if case .allowed = basePerm, !token.isPaused {
                    return .denied(reason: "Token is not paused")
                }
                return basePerm
            }()
            rows.append(ResolvedTokenAction(kind: .pause, permission: pausePerm))
            rows.append(ResolvedTokenAction(kind: .resume, permission: resumePerm))
        } else {
            rows.append(ResolvedTokenAction(
                kind: .pause,
                permission: .denied(reason: "Emergency actions not enabled")
            ))
            rows.append(ResolvedTokenAction(
                kind: .resume,
                permission: .denied(reason: "Emergency actions not enabled")
            ))
        }

        // Claim — perpetual / pre-programmed distribution.
        // Visibility rule from the prompt:
        //   * If `mintingAllowChoosingDestination == false` AND identity is
        //     not the designated `newTokensDestinationIdentity` -> hide.
        //   * Otherwise show, enabled when this identity is the
        //     designated recipient (we don't have per-slot
        //     "next-in-line" data yet, so leave that as TODO).
        let claim: TokenActionPermission = resolveClaim(token: token, identity: identity)
        rows.append(ResolvedTokenAction(kind: .claim, permission: claim))

        // Direct purchase. PersistentToken doesn't carry a current
        // direct-purchase price field — only the rules that govern who
        // can *change* the pricing. Treat "rules configured" as a weak
        // proxy for "direct pricing exists" and surface the gap as a
        // TODO. If neither rule is set, hide the row entirely.
        let directPurchase: TokenActionPermission = resolveDirectPurchase(
            token: token,
            identity: identity,
            contract: contract
        )
        rows.append(ResolvedTokenAction(kind: .directPurchase, permission: directPurchase))

        // Conventions
        rows.append(ResolvedTokenAction(
            kind: .setConventions,
            permission: TokenActionEvaluator.evaluate(
                rule: token.conventionsChangeRules,
                feature: "Conventions change",
                identity: identity,
                contract: contract,
                token: token
            )
        ))

        // Max supply
        rows.append(ResolvedTokenAction(
            kind: .updateMaxSupply,
            permission: TokenActionEvaluator.evaluate(
                rule: token.maxSupplyChangeRules,
                feature: "Max supply change",
                identity: identity,
                contract: contract,
                token: token
            )
        ))

        // Change distribution rules — `TokenDistributionChangeRules`
        // bundles four sub-rules. We pick perpetualDistributionRules
        // as the headline rule for the row; if it's nil, fall back to
        // newTokensDestinationIdentityRules. Either rule resolving
        // allowed lets the user into the distribution-edit flow; once
        // we have separate edit views per sub-rule we can break this
        // out into multiple rows.
        let distributionRule = token.distributionChangeRules?.perpetualDistributionRules
            ?? token.distributionChangeRules?.newTokensDestinationIdentityRules
            ?? token.distributionChangeRules?.mintingAllowChoosingDestinationRules
            ?? token.distributionChangeRules?.changeDirectPurchasePricingRules
        rows.append(ResolvedTokenAction(
            kind: .changeDistribution,
            permission: TokenActionEvaluator.evaluate(
                rule: distributionRule,
                feature: "Distribution change",
                identity: identity,
                contract: contract,
                token: token
            )
        ))

        return rows
    }

    /// If the action would otherwise be allowed but the token is
    /// paused, return a denied permission instead. Used for actions
    /// like minting/burning where a paused token should block the
    /// action regardless of the caller's role.
    private static func applyPausedGuard(
        _ permission: TokenActionPermission,
        token: PersistentToken,
        allowWhenPaused: Bool
    ) -> TokenActionPermission {
        if !allowWhenPaused, case .allowed = permission, token.isPaused {
            return .denied(reason: "Token is paused")
        }
        return permission
    }

    private static func resolveClaim(
        token: PersistentToken,
        identity: PersistentIdentity
    ) -> TokenActionPermission {
        let isDesignated = token.newTokensDestinationIdentity == identity.identityId
        let allowsChoosing = token.mintingAllowChoosingDestination
        let hasPerpetual = token.perpetualDistribution != nil
        let hasPreProgrammed = token.preProgrammedDistribution != nil

        // Match the rest of the screen's pattern: rows stay visible
        // and surface a denial reason instead of disappearing —
        // freeze / unfreeze / destroy-frozen-funds all do this when
        // "no one is authorized." Hiding Claim outright made it look
        // like the screen was missing an action entirely.
        //
        // Drive's claim mechanic only fires against an actual
        // distribution schedule, so without one the token has
        // nothing to claim — surface that as the disabled-state
        // reason rather than offering a tappable action that
        // would round-trip into a "no distribution" error.
        if !hasPerpetual && !hasPreProgrammed {
            return .denied(reason: "Token has no distribution schedule")
        }

        // The contract doesn't let identities choose where new
        // mints go AND this identity isn't the pinned recipient,
        // so they'd never receive anything to claim.
        if !allowsChoosing && !isDesignated {
            return .denied(reason: "Not the designated distribution recipient")
        }

        if isDesignated {
            return .allowed
        }

        // Allowed-to-choose-destination but not yet computed who's
        // next-in-line for a perpetual slot. Surface the limitation
        // rather than blocking outright.
        // TODO: query the next-in-line distribution slot for this identity.
        return .denied(reason: "Distribution eligibility not yet evaluated")
    }

    private static func resolveDirectPurchase(
        token: PersistentToken,
        identity: PersistentIdentity,
        contract: PersistentDataContract?
    ) -> TokenActionPermission {
        // No direct-purchase pricing infrastructure on the token row -> hide.
        let hasPricingRules = token.distributionChangeRules?.changeDirectPurchasePricingRules != nil
        if !hasPricingRules {
            return .hidden
        }
        // We don't model a current "for sale" price on PersistentToken,
        // so we can't decide whether the token is *currently* for sale.
        // Leave the row visible with a TODO so the gap is obvious.
        // TODO: surface direct-purchase price on PersistentToken and gate this row on it.
        if token.isPaused {
            return .denied(reason: "Token is paused")
        }
        _ = identity
        _ = contract
        return .denied(reason: "Direct purchase pricing not yet surfaced")
    }
}

// MARK: - View

/// "What can this identity do with this token, right now?"
///
/// Render a list of every supported token action, with each row showing
/// whether the current identity is permitted and — when not — why. Tap
/// targets currently route to a placeholder; the real state-transition
/// flows are out of scope for this PR.
struct TokenActionPermissionsView: View {
    let token: PersistentToken
    /// Optional caller-supplied identity. When nil, the view shows a
    /// Picker over the local identities that share the token's network
    /// so the user can pick one to evaluate against.
    @State private var pickedIdentity: PersistentIdentity?
    private let initialIdentity: PersistentIdentity?

    @Query private var localIdentities: [PersistentIdentity]

    init(token: PersistentToken, identity: PersistentIdentity? = nil) {
        self.token = token
        self.initialIdentity = identity
        self._pickedIdentity = State(initialValue: identity)
        // Filter to local identities on the same network as this
        // token's parent contract; falls back to "all local" if the
        // contract isn't loaded.
        //
        // Routes through the SDK-side
        // `PersistentIdentity.localIdentitiesPredicate(network:)`
        // helper rather than a hand-rolled `#Predicate` block. The
        // bespoke predicate that lived here referenced a captured
        // local `networkRaw: Int` whose name shadowed the model's
        // `identity.networkRaw` property, and SwiftData's
        // predicate translator chokes on the resulting ambiguity
        // mid-fetch — surfaces as a `_swift_runtime_on_report` /
        // `_assertionFailure` crash from `ModelContext.fetch`
        // (the `@Query` getter is the visible frame). The helper
        // captures the raw Int by a unique name so the
        // translator stays unambiguous.
        let resolvedNetwork: AppNetwork? = token.dataContract
            .flatMap { AppNetwork(rawValue: $0.networkRaw) }
        if let resolvedNetwork {
            self._localIdentities = Query(
                filter: PersistentIdentity.localIdentitiesPredicate(network: resolvedNetwork),
                sort: [SortDescriptor(\.identityIndex), SortDescriptor(\.identityIdString)]
            )
        } else {
            self._localIdentities = Query(
                filter: PersistentIdentity.localIdentitiesPredicate,
                sort: [SortDescriptor(\.identityIndex), SortDescriptor(\.identityIdString)]
            )
        }
    }

    private var resolvedIdentity: PersistentIdentity? {
        pickedIdentity ?? localIdentities.first
    }

    var body: some View {
        List {
            // Identity selector — only when the caller didn't pin
            // one. Mirrors the GroupDetailView "live @Query, render
            // as a Picker" pattern so the list updates if a new
            // identity is loaded while this view is on screen.
            if initialIdentity == nil {
                Section("Acting as") {
                    if localIdentities.isEmpty {
                        Text("No local identities on this network")
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                    } else {
                        Picker("Identity", selection: identityPickerBinding) {
                            ForEach(localIdentities, id: \.identityId) { identity in
                                Text(identity.displayName)
                                    .tag(Optional(identity.identityId))
                            }
                        }
                    }
                }
            } else if let identity = pickedIdentity {
                Section("Acting as") {
                    HStack {
                        Text(identity.displayName)
                            .font(.subheadline)
                        Spacer()
                        Text(short(identity.identityIdBase58))
                            .font(.caption.monospaced())
                            .foregroundColor(.secondary)
                    }
                }
            }

            // Status banner (paused / read-only signal)
            if token.isPaused {
                Section {
                    HStack {
                        Image(systemName: "pause.circle.fill")
                            .foregroundColor(.orange)
                        Text("This token is currently paused. Most actions are unavailable.")
                            .font(.subheadline)
                    }
                }
            }

            // Action rows
            if let identity = resolvedIdentity {
                Section("Actions") {
                    ForEach(visibleActions(for: identity)) { row in
                        actionRow(row)
                    }
                }
            } else {
                Section("Actions") {
                    Text("Pick an identity to see available actions")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle("Token Actions")
        .navigationBarTitleDisplayMode(.inline)
        .onAppear {
            if pickedIdentity == nil {
                pickedIdentity = localIdentities.first
            }
        }
    }

    private var identityPickerBinding: Binding<Data?> {
        Binding(
            get: { pickedIdentity?.identityId },
            set: { newId in
                guard let newId = newId else {
                    pickedIdentity = nil
                    return
                }
                pickedIdentity = localIdentities.first(where: { $0.identityId == newId })
            }
        )
    }

    private func visibleActions(for identity: PersistentIdentity) -> [ResolvedTokenAction] {
        TokenActionResolver.resolve(token: token, identity: identity)
            .filter { !$0.permission.isHidden }
    }

    @ViewBuilder
    private func actionRow(_ row: ResolvedTokenAction) -> some View {
        // Each enabled row navigates to a placeholder destination;
        // the actual state-transition flows are deliberately out of
        // scope for this PR. See TODOs in TokenActionResolver.
        if row.permission.isAllowed {
            NavigationLink(destination: TokenActionPlaceholderView(kind: row.kind)) {
                actionRowContent(row)
            }
        } else {
            actionRowContent(row)
                .opacity(0.55)
        }
    }

    @ViewBuilder
    private func actionRowContent(_ row: ResolvedTokenAction) -> some View {
        HStack(spacing: 12) {
            Image(systemName: row.kind.icon)
                .font(.title3)
                .frame(width: 28)
                .foregroundColor(row.permission.isAllowed ? .accentColor : .secondary)

            VStack(alignment: .leading, spacing: 2) {
                Text(row.kind.title)
                    .font(.body)
                    .foregroundColor(row.permission.isAllowed ? .primary : .secondary)

                if let reason = row.permission.deniedReason {
                    Text(reason)
                        .font(.caption)
                        .foregroundColor(.secondary)
                } else {
                    Text(row.kind.subtitle)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }

            Spacer()

            // Allowed rows are wrapped in a `NavigationLink` (see
            // `actionRow`) which already draws its own trailing
            // chevron; drawing one here too produced the doubled
            // `> >` users were seeing. Only render the lock for
            // denied rows; the system chevron handles the rest.
            if !row.permission.isAllowed {
                Image(systemName: "lock.fill")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
        .padding(.vertical, 4)
    }

    private func short(_ s: String) -> String {
        guard s.count > 12 else { return s }
        return "\(s.prefix(6))..\(s.suffix(4))"
    }
}

// MARK: - Placeholder Destination

/// Stub destination for tappable rows. The actual mint/burn/transfer
/// flows live behind their own state-transition entry points; this
/// view intentionally just confirms which action was selected so the
/// navigation wiring can be tested without dragging in the full
/// transition pipeline.
private struct TokenActionPlaceholderView: View {
    let kind: TokenActionKind

    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: kind.icon)
                .font(.system(size: 56))
                .foregroundColor(.accentColor)
            Text(kind.title)
                .font(.title2)
                .fontWeight(.semibold)
            Text("Action coming soon")
                .font(.subheadline)
                .foregroundColor(.secondary)
            // TODO: route to the real state-transition flow for `kind`.
        }
        .padding()
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .navigationTitle(kind.title)
        .navigationBarTitleDisplayMode(.inline)
    }
}
