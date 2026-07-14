// IdentitiesContentView.swift
// SwiftExampleApp
//
// Identities-only tab content. Split off from the old combined
// Wallets+Identities screen so each concern gets its own root tab.

import SwiftUI
import SwiftDashSDK
import SwiftData

struct IdentitiesContentView: View {
    @EnvironmentObject var platformState: AppState
    @EnvironmentObject var platformBalanceSyncService: PlatformBalanceSyncService
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    /// Active network the parent threads in. Scopes the identities
    /// query below so rows from another network don't leak into the
    /// list after a network switch — same pattern as
    /// `ContractsTabView`.
    private let network: Network
    @Query private var identities: [PersistentIdentity]
    /// All tracked asset locks across wallets. Filtered into
    /// "resumable" rows (status >= `InstantSendLocked` AND no
    /// `PersistentIdentity` at the same `(walletId, identityIndex)`
    /// slot) by `resumableAssetLocks` so the orphan-lock-after-crash
    /// case surfaces as a tappable Resume row. Sorted newest-first
    /// by `updatedAt` so the most recent unfinished registration
    /// sits at the top of the section.
    @Query(sort: [SortDescriptor(\PersistentAssetLock.updatedAt, order: .reverse)])
    private var allAssetLocks: [PersistentAssetLock]
    /// All wallets, used purely for the "wallet name" lookup on the
    /// Resume row label. Cheap query — wallet rows are few and
    /// the matching is on the in-memory array.
    @Query private var allWallets: [PersistentWallet]
    @State private var showingLoadIdentity = false
    @State private var showingCreateIdentity = false
    @State private var showingSearchWallets = false
    /// Identity targeted by a pending Remove swipe. Non-nil presents
    /// the confirmation dialog below. Stored as a reference rather than
    /// an id so the dialog can show the display name / truncated id
    /// without a second fetch.
    @State private var identityPendingRemoval: PersistentIdentity?
    /// Asset lock the user tapped Resume on. Drives the `.sheet(item:)`
    /// presentation of a pre-configured `CreateIdentityView`. Cleared
    /// when the sheet dismisses (SwiftUI nils the binding for us).
    @State private var resumingAssetLock: PersistentAssetLock?
    /// Identities vs. Masternodes segment. The segmented control is only
    /// shown when the active network has at least one aggregated
    /// masternode; otherwise the screen is unchanged (Identities only).
    @State private var identitiesSegment: IdentitiesSegment = .identities
    /// Whether Retired (not-in-DML) masternodes are shown. Default off —
    /// only Active / Inactive / Unknown appear until the user opts in.
    @State private var showRetired = false
    /// All persisted masternodes, filtered to the active network via
    /// `walletIdsOnNetwork` (masternodes carry no `networkRaw` column —
    /// the wallet-id pivot the platform-address / shielded views use).
    @Query private var allMasternodes: [PersistentMasternode]

    enum IdentitiesSegment: Hashable {
        case identities
        case masternodes
    }

    init(network: Network) {
        self.network = network
        _identities = Query(
            filter: PersistentIdentity.predicate(network: network),
            sort: \PersistentIdentity.identityIndex,
            order: .forward
        )
    }

    var body: some View {
        List {
            if showMasternodeSegments {
                Section {
                    Picker("View", selection: $identitiesSegment) {
                        Text("Identities").tag(IdentitiesSegment.identities)
                        Text("Masternodes").tag(IdentitiesSegment.masternodes)
                    }
                    .pickerStyle(.segmented)
                    .accessibilityIdentifier("identities.segment")
                }
            }

            if showMasternodeSegments, identitiesSegment == .masternodes {
                masternodesSection
            } else {
            pendingRegistrationsSection
            resumableRegistrationsSection
            if identities.isEmpty {
                Section {
                    VStack(spacing: 12) {
                        Image(systemName: "person.crop.circle.badge.plus")
                            .font(.system(size: 40))
                            .foregroundColor(.gray)

                        Text("No Identities")
                            .font(.headline)

                        Text("Load an identity to interact with Dash Platform")
                            .font(.caption)
                            .foregroundColor(.secondary)

                        HStack(spacing: 12) {
                            Button {
                                showingCreateIdentity = true
                            } label: {
                                Label("Create Identity", systemImage: "person.crop.circle.badge.plus")
                                    .padding(.horizontal, 16)
                                    .padding(.vertical, 8)
                            }
                            .buttonStyle(.borderedProminent)

                            Button {
                                showingLoadIdentity = true
                            } label: {
                                Label("Load Identity", systemImage: "square.and.arrow.down")
                                    .padding(.horizontal, 16)
                                    .padding(.vertical, 8)
                            }
                            .buttonStyle(.bordered)
                        }
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 20)
                }
            } else {
                Section("Identities") {
                    ForEach(identities) { identity in
                        IdentityRow(identity: identity)
                            .environmentObject(platformState)
                            // `allowsFullSwipe: false` so the user
                            // can't accidentally bypass the
                            // confirmation with a long swipe. We
                            // deliberately avoid `role: .destructive`
                            // on the swipe button: SwiftUI animates
                            // destructive swipe buttons as if the row
                            // is already gone the moment they're
                            // tapped, and when the confirmation
                            // dialog is dismissed the row "pops
                            // back" because the underlying @Query
                            // still yields it. Painting red via
                            // `.tint` keeps the look without that
                            // pre-commit animation.
                            .swipeActions(edge: .trailing, allowsFullSwipe: false) {
                                Button {
                                    identityPendingRemoval = identity
                                } label: {
                                    Label("Remove", systemImage: "trash")
                                }
                                .tint(.red)
                            }
                    }
                }
            }
            }
        }
        .confirmationDialog(
            removalDialogTitle,
            isPresented: Binding(
                get: { identityPendingRemoval != nil },
                set: { newValue in
                    if !newValue { identityPendingRemoval = nil }
                }
            ),
            titleVisibility: .visible,
            presenting: identityPendingRemoval
        ) { identity in
            Button("Remove from Device", role: .destructive) {
                removeIdentityLocally(identity)
                identityPendingRemoval = nil
            }
            Button("Keep on Device", role: .cancel) {
                identityPendingRemoval = nil
            }
        } message: { _ in
            Text(
                "This only deletes the local copy. The identity remains on the Dash Platform network. You can reload it later if you still have the keys."
            )
        }
        .navigationTitle("Identities")
        .navigationDestination(for: PersistentMasternode.self) { masternode in
            MasternodeDetailView(masternode: masternode)
        }
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Menu {
                    Button {
                        showingCreateIdentity = true
                    } label: {
                        Label("Create Identity", systemImage: "person.crop.circle.badge.plus")
                    }

                    Button {
                        showingLoadIdentity = true
                    } label: {
                        Label("Load Identity", systemImage: "square.and.arrow.down")
                    }

                    Button {
                        showingSearchWallets = true
                    } label: {
                        Label("Re-scan for Identities", systemImage: "magnifyingglass")
                    }
                } label: {
                    Image(systemName: "plus")
                }
            }
        }
        .sheet(isPresented: $showingLoadIdentity) {
            LoadIdentityView()
                .environmentObject(platformState)
        }
        .sheet(isPresented: $showingCreateIdentity) {
            CreateIdentityView()
                .environmentObject(platformState)
        }
        .sheet(item: $resumingAssetLock) { lock in
            // Pre-configured resume of an orphan asset lock. Same
            // view, same code path — the constructor seeds the
            // four selection `@State`s as initial values so the
            // form opens already on the "Fund from unused Asset
            // Lock" step with this specific lock highlighted, and
            // the user only has to tap "Create Identity".
            CreateIdentityView(preselectedAssetLock: lock)
                .environmentObject(platformState)
        }
        .sheet(isPresented: $showingSearchWallets) {
            SearchWalletsForIdentitiesView()
                .environmentObject(platformState)
        }
        .refreshable {
            await platformBalanceSyncService.performSync()
        }
        // Refresh the persisted masternode aggregation from Rust when the
        // tab appears. Pure persist step — Rust owns the aggregation
        // (`MasternodeSync.refresh`).
        .onAppear {
            MasternodeSync.refresh(
                walletManager: walletManager,
                walletIds: walletIdsOnNetwork,
                modelContext: modelContext
            )
        }
    }

    /// Wallet ids on the active network — the scoping pivot for
    /// `allMasternodes` (which has no `networkRaw` column of its own).
    private var walletIdsOnNetwork: Set<Data> {
        let raw = network.rawValue
        return Set(allWallets.lazy
            .filter { $0.networkRaw == raw }
            .map(\.walletId))
    }

    /// Active-network masternodes in stable registration order.
    private var masternodesOnNetwork: [PersistentMasternode] {
        let ids = walletIdsOnNetwork
        return allMasternodes
            .filter { ids.contains($0.walletId) }
            .sorted { $0.orderIndex < $1.orderIndex }
    }

    /// Show the Identities/Masternodes toggle only when this network has
    /// at least one masternode; otherwise the screen stays Identities-only.
    private var showMasternodeSegments: Bool {
        !masternodesOnNetwork.isEmpty
    }

    /// Active-network masternodes after the "Show retired" filter. Retired
    /// (status == .retired) rows are hidden unless `showRetired`; every
    /// other status — including Unknown — stays visible.
    private var visibleMasternodes: [PersistentMasternode] {
        masternodesOnNetwork.filter { showRetired || $0.status != .retired }
    }

    /// Count of retired masternodes currently hidden by the filter.
    private var hiddenRetiredCount: Int {
        showRetired ? 0 : masternodesOnNetwork.filter { $0.status == .retired }.count
    }

    /// One row per aggregated masternode, in registration order, with the
    /// "Show retired" toggle.
    @ViewBuilder
    private var masternodesSection: some View {
        Section("Masternodes") {
            Toggle("Show retired", isOn: $showRetired)
                .accessibilityIdentifier("masternodes.showRetiredToggle")

            ForEach(visibleMasternodes) { masternode in
                // Value-based navigation (destination hoisted onto the
                // List below) so a `@Query` re-render can't dismiss an
                // open detail page mid-scroll — see the nav-churn note.
                NavigationLink(value: masternode) {
                    MasternodeRow(masternode: masternode)
                }
            }

            // Guard against a blank-looking list when every masternode is
            // retired and the toggle is off.
            if visibleMasternodes.isEmpty && hiddenRetiredCount > 0 {
                Text("\(hiddenRetiredCount) retired hidden")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
    }

    /// "Pending registrations" row group. Surfaces every controller
    /// the `RegistrationCoordinator` is tracking — both in-flight
    /// flows (the user dismissed `CreateIdentityView` but the
    /// registration is still running) and terminal-but-undismissed
    /// flows (`.completed` rows linger ~30s, `.failed` rows linger
    /// indefinitely until the user manually dismisses). Empty when
    /// the coordinator's map is empty, in which case the section
    /// collapses to nothing so the rest of the screen isn't pushed
    /// down by an "empty" header.
    ///
    /// Observation: wrap the coordinator in a dedicated
    /// `PendingRegistrationsList` so its `@ObservedObject` reads
    /// the coordinator directly. Reading
    /// `walletManager.registrationCoordinator` inside this view's
    /// body would not subscribe — `walletManager` is the
    /// `EnvironmentObject` we observe, not the coordinator hung off
    /// it — so map mutations wouldn't trigger a redraw.
    @ViewBuilder
    private var pendingRegistrationsSection: some View {
        PendingRegistrationsList(
            coordinator: walletManager.registrationCoordinator
        )
    }

    /// "Resumable Registrations" row group. Surfaces orphan
    /// `PersistentAssetLock` rows — those at status >= `Broadcast`
    /// (`statusRaw >= 1`) with no `PersistentIdentity` at the same
    /// `(walletId, identityIndex)` slot — that the in-memory
    /// `RegistrationCoordinator` can't know about because its map
    /// is wiped on app restart. Without this section, an app kill
    /// mid-registration leaves the user with no surface signal
    /// that there's an in-flight lock: the lock still lives in
    /// SwiftData, but the "Pending Registrations" section above
    /// only reflects the in-memory coordinator state.
    ///
    /// Each row's trailing affordance is staged on the lock's
    /// `statusRaw`:
    /// - `1` Broadcast: spinner + "Waiting for InstantSendLock or
    ///   ChainLock…" — the lock can't fund a Platform identity
    ///   until the masternodes sign either lock type. For
    ///   freshly broadcast funding txs the IS lock typically
    ///   arrives within seconds; for aged stuck locks (catch-up
    ///   case) the IS quorum has rotated so only a ChainLock
    ///   can resolve them. Either path lands as
    ///   `statusRaw = 2` or `3`. SPV is running; the persister
    ///   will flip the row when the event arrives, and
    ///   SwiftData `@Query` re-renders the row into the
    ///   actionable state without any extra wiring.
    /// - `2` / `3` InstantSendLocked / ChainLocked: Resume
    ///   button. Tapping opens `CreateIdentityView` pre-configured
    ///   for the `.unusedAssetLock` funding path with this lock
    ///   pinned.
    ///
    /// Empty when there are no orphan locks; collapses to nothing
    /// in that case so the rest of the screen isn't pushed down by
    /// an empty header.
    /// Wraps the Resumable Registrations section in a dedicated view
    /// that observes `RegistrationCoordinator` directly. Necessary
    /// because the section's filter needs to consult **both** the
    /// SwiftData @Query results AND the coordinator's in-memory
    /// controller map: an in-flight registration owns its slot
    /// transiently between `asset-lock-broadcast` and `PersistentIdentity-
    /// written`, and during that window the orphan-lock anti-join
    /// alone would let the same slot surface as both "Pending" and
    /// "Resumable" → user could tap Resume and race a duplicate
    /// FFI against the original.
    @ViewBuilder
    private var resumableRegistrationsSection: some View {
        ResumableRegistrationsList(
            coordinator: walletManager.registrationCoordinator,
            network: network,
            allAssetLocks: allAssetLocks,
            allWallets: allWallets,
            allIdentities: identities,
            resumingAssetLock: $resumingAssetLock
        )
    }

    /// Pure anti-join across all wallets. A lock is *visible* on the
    /// Resumable Registrations surface iff
    /// - `statusRaw` is in `1...3` (Broadcast through ChainLocked), AND
    /// - no `(walletId, identityIndex)` slot is in `usedSlots`.
    ///
    /// `usedSlots` is the **union** of two sources of slot
    /// occupancy (the caller assembles it):
    /// 1. Persisted `PersistentIdentity` rows — registrations that
    ///    have completed and written an identity to SwiftData.
    /// 2. In-memory `RegistrationCoordinator` controllers in
    ///    `.preparingKeys` or `.inFlight` — registrations whose
    ///    asset lock has been broadcast but whose identity row
    ///    hasn't landed yet. Including these prevents a transient
    ///    window where the same lock would surface here *and* in
    ///    the in-memory Pending Registrations list, letting the
    ///    user race a duplicate Resume tap against the original.
    ///
    /// The status floor (`>= 1`, Broadcast) is intentionally low:
    /// a lock at Broadcast (1) is in mid-handoff — SPV will
    /// deliver the InstantSendLock shortly and the persister will
    /// flip it to (2), at which point the row's trailing affordance
    /// flips from a spinner to a Resume button automatically
    /// (SwiftData `@Query` is reactive). Hiding (1) entirely would
    /// create a UX asymmetry where the just-broadcast lock vanishes
    /// from the UI then reappears seconds later at (2).
    ///
    /// `statusRaw == 0` (Built but never broadcast) is filtered
    /// out: a tight crash window between TX build and broadcast
    /// with no useful UX action to take.
    ///
    /// The upper bound (`<= 3`, ChainLocked) excludes `statusRaw == 4`
    /// (Consumed) — the terminal state set when a lock has already
    /// funded an identity. In the happy path the anti-join against
    /// `PersistentIdentity` would hide a Consumed lock anyway
    /// (every successful registration writes both rows at the same
    /// slot), but the local-only delete-identity action removes
    /// the identity row WITHOUT the asset-lock row, which frees the
    /// slot in the anti-join. Without this upper bound a Consumed
    /// lock would re-surface as a perpetual-spinner row whose
    /// "Resume" path can't advance — `resume_asset_lock` rejects
    /// Consumed entries with "already Consumed — nothing to resume".
    ///
    /// Generic over `AssetLockResumeRow` so the pure filter is
    /// unit-testable without a SwiftData container.
    nonisolated static func crossWalletResumableLocks<R: AssetLockResumeRow>(
        in locks: [R],
        usedSlots: Set<UsedSlot>
    ) -> [R] {
        locks.filter { lock in
            guard lock.statusRaw >= 1 && lock.statusRaw <= 3 else { return false }
            // Invitation vouchers (fundingTypeRaw 3, IdentityInvitation) are
            // bearer locks whose key is shared in the invitation link — they
            // are reclaimed via the Invitations screen, never resumed as a
            // local registration (which would consume the voucher into an
            // unrelated identity and kill the invitee's claim). Their nominal
            // identity slot is 0, so without this exclusion an unclaimed
            // voucher surfaces here whenever slot 0 is free. The Rust funding
            // resolver refuses them too; this keeps the row from rendering.
            guard lock.fundingTypeRaw != 3 else { return false }
            let slot = UInt32(bitPattern: lock.identityIndexRaw)
            return !usedSlots.contains(
                UsedSlot(walletId: lock.walletId, slot: slot)
            )
        }
    }

    /// Composite key for the per-`(walletId, identityIndex)`
    /// anti-join. Public visibility so unit tests in the same
    /// module can build the set directly.
    struct UsedSlot: Hashable {
        let walletId: Data
        let slot: UInt32
    }

    /// Short title for the removal confirmation dialog. Uses
    /// `displayName` so the user sees the DPNS name / alias when
    /// available, and a truncated-hex id otherwise — matches the
    /// label shown in the row itself.
    private var removalDialogTitle: String {
        guard let identity = identityPendingRemoval else {
            return "Remove identity from this device?"
        }
        return "Remove \(identity.displayName) from this device?"
    }

    /// Delete a single identity locally. This does **not** touch the
    /// Dash Platform network — the identity on-chain is untouched and
    /// can be reloaded later if the keys survive (either in the
    /// originating wallet mnemonic or via an import flow).
    ///
    /// Keychain cleanup policy — we walk the identity's public keys
    /// and delete each entry by the stored
    /// `privateKeyKeychainIdentifier` account string. That covers
    /// both write formats the app uses (see `KeychainManager`):
    ///   * `privkey_<identityHex>_<keyIndex>` — legacy direct storage
    ///     via `storePrivateKey`.
    ///   * `identity_privkey.<derivationPath>` — wallet-derived
    ///     entries written by the Rust persister callback.
    /// Because the cascade delete on `PersistentPublicKey` rows runs
    /// *after* we've harvested these strings, the keychain entries
    /// would otherwise orphan (the legacy `deleteAllPrivateKeys(for:)`
    /// filter only matches the `privkey_…_` prefix and would miss
    /// derivation-path entries). We also clear the three special-key
    /// identifiers stored directly on the identity
    /// (`votingPrivateKeyIdentifier`, `ownerPrivateKeyIdentifier`,
    /// `payoutPrivateKeyIdentifier`) since those are likewise keyed
    /// by string identifier.
    ///
    /// After keychain cleanup the SwiftData cascade does the rest:
    /// `@Relationship(.cascade)` on `publicKeys` / `documents` drops
    /// those child rows, `@Relationship(.nullify)` on `tokenBalances`
    /// severs the link, and the `PersistentWallet.identities` inverse
    /// (declared on the wallet side, also `.nullify`) naturally drops
    /// this identity from its owning wallet — no manual detachment
    /// required.
    private func removeIdentityLocally(_ identity: PersistentIdentity) {
        let keychain = KeychainManager.shared

        for publicKey in identity.publicKeys {
            if let identifier = publicKey.privateKeyKeychainIdentifier {
                _ = keychain.deleteKeyData(identifier: identifier)
            }
        }

        if let votingIdentifier = identity.votingPrivateKeyIdentifier {
            _ = keychain.deleteKeyData(identifier: votingIdentifier)
        }
        if let ownerIdentifier = identity.ownerPrivateKeyIdentifier {
            _ = keychain.deleteKeyData(identifier: ownerIdentifier)
        }
        if let payoutIdentifier = identity.payoutPrivateKeyIdentifier {
            _ = keychain.deleteKeyData(identifier: payoutIdentifier)
        }

        modelContext.delete(identity)
        do {
            try modelContext.save()
        } catch {
            platformState.showError(
                message: "Failed to remove identity from device: \(error.localizedDescription)"
            )
        }
    }
}

/// Section view backing the Identities tab's "Resumable Registrations"
/// surface. Observes `RegistrationCoordinator` (`@ObservedObject`) so
/// the in-memory controller map is part of the filter input, not just
/// the SwiftData-backed identity rows: a controller in `.preparingKeys`
/// or `.inFlight` owns its slot transiently before a `PersistentIdentity`
/// row exists for it, and during that window the orphan-lock anti-join
/// would otherwise surface the same slot as both "Pending" and
/// "Resumable" → a duplicate Resume tap could race a second FFI for
/// the same outpoint.
///
/// Phase-change reactivity within an existing controller is best-
/// effort. The coordinator's `@Published controllers` dict re-emits
/// on add/remove but not on phase transitions inside an existing
/// entry. The two transitions that need re-render are
/// `.inFlight → .completed` (handled — the persister writes a
/// `PersistentIdentity` row, the @Query re-fires) and
/// `.inFlight → .failed` (not handled directly; the row re-surfaces
/// once the user dismisses the failed Pending entry, which mutates
/// the dict). The latter delay is acceptable given the explicit
/// dismiss UX on `.failed` rows.
private struct ResumableRegistrationsList: View {
    @ObservedObject var coordinator: RegistrationCoordinator
    /// Active network. Scopes `allAssetLocks` to wallets on this
    /// network before the anti-join so locks from another network
    /// don't leak into the list after a network switch.
    let network: Network
    let allAssetLocks: [PersistentAssetLock]
    let allWallets: [PersistentWallet]
    let allIdentities: [PersistentIdentity]
    @Binding var resumingAssetLock: PersistentAssetLock?

    var body: some View {
        let locks = resumableLocks
        if !locks.isEmpty {
            Section("Resumable Registrations (\(locks.count))") {
                ForEach(locks) { lock in
                    ResumableRegistrationRow(
                        lock: lock,
                        walletLabel: walletDisplayLabel(for: lock.walletId),
                        onResume: {
                            resumingAssetLock = lock
                        }
                    )
                }
            }
        }
    }

    /// Filter is `anti-join(identity-claimed slots ∪ in-flight slots)`
    /// over `crossWalletResumableLocks`. The union is the key
    /// fix: identity rows alone leave a window where the asset
    /// lock is already broadcast but the registration hasn't
    /// persisted an identity yet, and during that window the
    /// row would double-count in both Pending and Resumable.
    private var resumableLocks: [PersistentAssetLock] {
        let identitySlots: Set<IdentitiesContentView.UsedSlot> = Set(
            allIdentities.compactMap { identity in
                guard let walletId = identity.wallet?.walletId else {
                    return nil
                }
                return IdentitiesContentView.UsedSlot(
                    walletId: walletId,
                    slot: identity.identityIndex
                )
            }
        )
        let activeSlots: Set<IdentitiesContentView.UsedSlot> = Set(
            coordinator.controllers
                .filter { _, controller in controller.phase.isActive }
                .map { (key, _) in
                    IdentitiesContentView.UsedSlot(
                        walletId: key.walletId,
                        slot: key.identityIndex
                    )
                }
        )
        return IdentitiesContentView.crossWalletResumableLocks(
            in: networkScopedAssetLocks,
            usedSlots: identitySlots.union(activeSlots)
        )
    }

    /// `allAssetLocks` restricted to wallets on the active network.
    /// `PersistentAssetLock` carries no `networkRaw` column itself;
    /// the canonical join is through `walletId` to the parent
    /// `PersistentWallet.networkRaw` — same pivot `CoreContentView`
    /// uses. Without this, locks from another network would surface
    /// as resumable rows after a network switch.
    private var networkScopedAssetLocks: [PersistentAssetLock] {
        let raw = network.rawValue
        let walletIdsOnNetwork = Set(
            allWallets.lazy
                .filter { $0.networkRaw == raw }
                .map(\.walletId)
        )
        return allAssetLocks.filter { walletIdsOnNetwork.contains($0.walletId) }
    }

    private func walletDisplayLabel(for walletId: Data) -> String {
        if let wallet = allWallets.first(where: { $0.walletId == walletId }) {
            return wallet.label
        }
        let hex = walletId.prefix(4)
            .map { String(format: "%02x", $0) }
            .joined()
        return hex.isEmpty ? "Wallet" : "Wallet \(hex)…"
    }
}

/// Single row in the "Resumable Registrations" section. Renders the
/// lock summary (txid prefix, amount, status, owning wallet, slot)
/// plus a compact Resume button that fires the caller-supplied
/// `onResume` closure. Visually matches the row density of
/// `IdentityRow` and the storage-explorer's `AssetLockStorageListView`
/// row at `StorageModelListViews.swift:1636`.
private struct ResumableRegistrationRow: View {
    let lock: PersistentAssetLock
    let walletLabel: String
    let onResume: () -> Void

    var body: some View {
        HStack(alignment: .top) {
            VStack(alignment: .leading, spacing: 4) {
                Text("Asset Lock \(lock.shortOutPointDisplay)")
                    .font(.body)
                    .lineLimit(1)
                HStack(spacing: 6) {
                    Text(formatDuffs(lock.amountDuffs))
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text("·")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text(lock.statusLabel)
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text("·")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text("#\(UInt32(bitPattern: lock.identityIndexRaw))")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .monospacedDigit()
                }
                Text(walletLabel)
                    .font(.caption2)
                    .foregroundColor(.secondary)
                    .lineLimit(1)
            }
            Spacer(minLength: 8)
            trailingAffordance
        }
        .padding(.vertical, 2)
    }

    /// Trailing Resume button. Every lock reaching this row is
    /// `statusRaw` 1...3 (per `crossWalletResumableLocks`) AND — by the
    /// list's `(walletId, identityIndex)` slot anti-join, which unions
    /// active-controller slots — is NOT owned by an in-flight
    /// registration. So every row is a genuinely-orphaned lock with no
    /// driver, and offering Resume is always safe:
    /// - InstantSendLocked / ChainLocked (`canFundIdentity`) submits the
    ///   IdentityCreate as soon as it's tapped.
    /// - Broadcast (statusRaw 1) has no proof yet; resuming re-enters the
    ///   finality wait via `resume_asset_lock` (which re-broadcasts the
    ///   tx, then waits for the ChainLock — guaranteed finality, however
    ///   long it takes). This previously rendered a permanent spinner,
    ///   stranding a broadcast-but-interrupted registration lock (app
    ///   killed mid-wait, or an aged lock whose IS quorum rotated) with
    ///   no in-app recovery path.
    ///
    /// Unlike the address-funding sibling, no `hasActiveFunding` gate is
    /// needed here: identity locks carry their `identityIndex`, so the
    /// anti-join excludes the in-flight lock precisely, where the
    /// address flow (whose lock doesn't know its recipient) cannot.
    ///
    /// SwiftData `@Query` re-renders this row as the persister advances
    /// the lock's status.
    private var trailingAffordance: some View {
        Button(action: onResume) {
            Label("Resume", systemImage: "arrow.clockwise")
                .labelStyle(.titleAndIcon)
                .font(.callout)
        }
        .buttonStyle(.borderedProminent)
        .controlSize(.small)
    }

    private func formatDuffs(_ amountDuffs: Int64) -> String {
        let dash = Double(amountDuffs) / 1e8
        return String(format: "%g DASH", dash)
    }
}

/// One masternode row: type badge (Evonode / Masternode), service IP,
/// short proTxHash, and Active / Revoked status. All fields come
/// pre-aggregated from Rust via `PersistentMasternode`.
private struct MasternodeRow: View {
    let masternode: PersistentMasternode

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(masternode.displayTitle)
                    .font(.subheadline)
                    .fontWeight(.semibold)
                Spacer()
                Text(masternode.typeName)
                    .font(.caption2)
                    .fontWeight(.medium)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(
                        (masternode.isEvonode ? Color.purple : Color.blue).opacity(0.15)
                    )
                    .foregroundColor(masternode.isEvonode ? .purple : .blue)
                    .cornerRadius(4)
            }

            HStack(spacing: 6) {
                Image(systemName: "network")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Text(masternode.serviceAddress ?? "—")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            HStack(spacing: 6) {
                Text(masternode.proTxHashShort)
                    .font(.system(.caption2, design: .monospaced))
                    .foregroundColor(.secondary)
                Spacer()
                Text(masternode.statusName)
                    .font(.caption2)
                    .fontWeight(.medium)
                    .foregroundColor(statusColor)
            }
        }
        .padding(.vertical, 2)
        .accessibilityIdentifier("masternodes.row.\(masternode.proTxHashShort)")
    }

    /// Active = green, Inactive (PoSe-banned) = orange, Retired = red,
    /// Unknown (DML not synced) = secondary.
    private var statusColor: Color {
        switch masternode.status {
        case .active: return .green
        case .inactive: return .orange
        case .retired: return .red
        case .unknown: return .secondary
        }
    }
}
