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
    @Query(sort: \PersistentIdentity.identityIndex)
    private var identities: [PersistentIdentity]
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

    var body: some View {
        List {
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
        }
        .refreshable {
            await platformBalanceSyncService.performSync()
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
    /// - `1` Broadcast: spinner + "Waiting for InstantSendLock…"
    ///   — the lock can't fund a Platform identity until the
    ///   masternodes sign an IS lock. SPV is running; the
    ///   persister will flip the row to (2) when the event
    ///   arrives, and SwiftData `@Query` re-renders the row
    ///   into the actionable state without any extra wiring.
    /// - `2` / `3` InstantSendLocked / ChainLocked: Resume
    ///   button. Tapping opens `CreateIdentityView` pre-configured
    ///   for the `.unusedAssetLock` funding path with this lock
    ///   pinned.
    ///
    /// Empty when there are no orphan locks; collapses to nothing
    /// in that case so the rest of the screen isn't pushed down by
    /// an empty header.
    @ViewBuilder
    private var resumableRegistrationsSection: some View {
        let locks = resumableAssetLocks
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

    /// Cross-wallet variant of the per-wallet resume picker filter
    /// implemented at `CreateIdentityView.resumableLocks(...)`. Same
    /// anti-join logic, but the per-wallet `usedIndices` set is
    /// generalized to a per-`(walletId, identityIndex)` set so we
    /// can filter all wallets in one pass.
    ///
    /// Independent of `RegistrationCoordinator` — this is purely a
    /// SwiftData read. Survives app restarts because the underlying
    /// `PersistentAssetLock` and `PersistentIdentity` rows are
    /// disk-persisted.
    private var resumableAssetLocks: [PersistentAssetLock] {
        let usedSlots: Set<UsedSlot> = Set(
            identities.compactMap { identity -> UsedSlot? in
                guard let walletId = identity.wallet?.walletId else {
                    return nil
                }
                return UsedSlot(walletId: walletId, slot: identity.identityIndex)
            }
        )
        return Self.crossWalletResumableLocks(
            in: allAssetLocks,
            usedSlots: usedSlots
        )
    }

    /// Wallet display label for the Resume row's sub-line. Prefers
    /// the wallet's stored name (via `PersistentWallet.label`'s
    /// short-hex fallback), so the row shows "MyWallet" / "Wallet
    /// a1b2c3d4…" rather than a raw 32-byte id dump.
    private func walletDisplayLabel(for walletId: Data) -> String {
        if let wallet = allWallets.first(where: { $0.walletId == walletId }) {
            return wallet.label
        }
        // Defensive — should never hit this branch in practice
        // because every asset lock is owned by a wallet that's
        // in `allWallets`. Mirrors the same fallback shape that
        // `PersistentWallet.label` uses so the cosmetic doesn't
        // diverge.
        let hex = walletId.prefix(4)
            .map { String(format: "%02x", $0) }
            .joined()
        return hex.isEmpty ? "Wallet" : "Wallet \(hex)…"
    }

    /// Pure anti-join across all wallets. A lock is *visible* on the
    /// Resumable Registrations surface iff
    /// - `statusRaw >= 1` (Broadcast or higher), AND
    /// - no `(walletId, identityIndex)` slot is already claimed by
    ///   a `PersistentIdentity` row.
    ///
    /// The floor here (`>= 1`, Broadcast) is intentionally **lower
    /// than the per-wallet picker's floor** in
    /// `CreateIdentityView.resumableLocks(...)` (which uses `>= 2`,
    /// InstantSendLocked). Reason: the picker only surfaces locks
    /// that can fund a Platform identity *right now* — only IS-
    /// or chain-locked locks have a usable proof — so its row is
    /// always immediately actionable. This section, by contrast,
    /// is the user's only signal that *any* registration is
    /// mid-flight after an app restart. A lock at Broadcast (1) is
    /// in mid-handoff — SPV will deliver the InstantSendLock
    /// shortly and the persister will flip it to (2), at which
    /// point the row's trailing affordance flips from a spinner to
    /// a Resume button automatically (SwiftData `@Query` is
    /// reactive). Hiding (1) entirely would create the UX
    /// asymmetry where users see their just-broadcast lock at
    /// (1) vanish from the UI, then reappear seconds later at
    /// (2) — confusing rather than reassuring.
    ///
    /// `statusRaw == 0` (Built but never broadcast) is still
    /// filtered out: it's a tight crash window between TX build
    /// and broadcast, and there's no useful UX action to take.
    /// A re-broadcast would have to come from a different surface.
    ///
    /// Generic over `AssetLockResumeRow` (the same protocol the
    /// per-wallet helper uses) so the pure filter is unit-testable
    /// without spinning up a SwiftData container.
    nonisolated static func crossWalletResumableLocks<R: AssetLockResumeRow>(
        in locks: [R],
        usedSlots: Set<UsedSlot>
    ) -> [R] {
        locks.filter { lock in
            guard lock.statusRaw >= 1 else { return false }
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
                Text("Asset Lock \(shortOutPoint(lock.outPointHex))")
                    .font(.body)
                    .lineLimit(1)
                HStack(spacing: 6) {
                    Text(formatDuffs(lock.amountDuffs))
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text("·")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text(statusLabel(lock.statusRaw))
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

    /// Trailing view that depends on the lock's stage. At
    /// `Broadcast` (1) the lock isn't usable yet — SPV is waiting
    /// on the masternodes to sign an InstantSendLock — so we show
    /// a spinner instead of a button. SwiftData `@Query` is
    /// reactive, so when the persister flips the row to
    /// `InstantSendLocked` (2) this view re-renders into the
    /// Resume button without any extra plumbing.
    @ViewBuilder
    private var trailingAffordance: some View {
        if lock.statusRaw >= 2 {
            Button(action: onResume) {
                Label("Resume", systemImage: "arrow.clockwise")
                    .labelStyle(.titleAndIcon)
                    .font(.callout)
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.small)
        } else {
            HStack(spacing: 6) {
                ProgressView()
                    .controlSize(.small)
                Text("Waiting for InstantSendLock…")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
    }

    /// Short txid prefix (first 8 hex chars) from the canonical
    /// `<txid>:<vout>` outpoint encoding. Matches the row format
    /// used by `AssetLockStorageListView`.
    private func shortOutPoint(_ outPointHex: String) -> String {
        let parts = outPointHex.split(
            separator: ":",
            maxSplits: 1,
            omittingEmptySubsequences: false
        )
        guard parts.count == 2 else { return outPointHex }
        let txidPrefix = parts[0].prefix(8)
        return "\(txidPrefix):\(parts[1])"
    }

    private func formatDuffs(_ amountDuffs: Int64) -> String {
        let dash = Double(amountDuffs) / 1e8
        return String(format: "%g DASH", dash)
    }

    private func statusLabel(_ raw: Int) -> String {
        switch raw {
        case 0: return "Built"
        case 1: return "Broadcast"
        case 2: return "InstantSendLocked"
        case 3: return "ChainLocked"
        default: return "Unknown(\(raw))"
        }
    }
}
