import Foundation
import SwiftData
import DashSDKFFI

/// Bridges FFI persistence callbacks to SwiftData storage.
///
/// Allocated as a class so its pointer can be passed as the opaque `context`
/// to the Rust persistence callbacks. Must be retained for the lifetime of
/// the `PlatformWalletManager`.
// All mutable state (`backgroundContext`, caches) is confined to `serialQueue`
// — the handler's de-facto actor — so it is safe to hand to a `@Sendable`
// closure (e.g. the off-main `serialQueue.async` backfill dispatch).
//
// Known coverage deviation: the deferred contact-crypto queue
// (`PlatformWalletChangeSet.pending_contact_crypto_added/_cleared`) has no
// persister vtable slot, so it is NOT durable on this host — a restart
// before a signer-backed drain relies on the recurring sweep to re-enqueue.
public final class PlatformWalletPersistenceHandler: @unchecked Sendable {
    static func shouldRestoreProviderSpecialTransaction(
        walletId: Data,
        involvedAccounts: [(walletId: Data, accountType: UInt32)]
    ) -> Bool {
        involvedAccounts.contains { account in
            account.walletId == walletId
                && account.accountType >= 8
                && account.accountType <= 11
        }
    }

    /// Wallet a TXO belongs to, resolved the way `loadWalletList` already
    /// resolves it.
    ///
    /// `PersistentTxo.walletId` is a denormalized convenience field and is
    /// **empty on rows written before it existed**. Comparing it raw makes
    /// every legacy TXO look like it belongs to no wallet — which, for
    /// sent-payment reconstruction, silently reclassifies a real spend as
    /// "not ours" and drops the payment. Fall back to the owning account's
    /// wallet for those rows.
    ///
    /// `account.wallet` is non-optional on the model but is a fault-loaded
    /// relationship, so it is read through an Optional cast: a
    /// relationship-store inconsistency would otherwise crash here.
    static func resolvedWalletId(of txo: PersistentTxo) -> Data? {
        if !txo.walletId.isEmpty {
            return txo.walletId
        }
        let account: PersistentAccount? = txo.account
        guard let account else { return nil }
        let wallet: PersistentWallet? = account.wallet
        return wallet?.walletId
    }

    static func walletOwnsTransaction(
        walletId: Data,
        transaction: PersistentTransaction
    ) -> Bool {
        // A globally-swept row is never "owned" for restore purposes, even
        // though `involvedAccounts` below can still name this wallet — that
        // membership was recorded before the transaction lost the sweep and
        // `applySweptTransaction` does not (and should not) rewrite history
        // by removing it. Excluding here, at the single call site every
        // restore-to-Rust enumeration goes through (`walletCoreTxids`), is
        // what keeps a row `isGloballySwept` has already proven dead from
        // being handed back as this wallet's transaction after a restart.
        guard !transaction.isGloballySwept else { return false }
        if transaction.involvedAccounts.contains(where: {
            let wallet: PersistentWallet? = $0.wallet
            return wallet?.walletId == walletId
        }) {
            return true
        }
        if transaction.outputs.contains(where: { resolvedWalletId(of: $0) == walletId }) {
            return true
        }
        if transaction.inputs.contains(where: { resolvedWalletId(of: $0) == walletId }) {
            return true
        }
        // `PersistentPendingInput` carries no account relationship, so its
        // denormalized `walletId` is the only thing to compare — it is also a
        // newer row type, written only by the current send path.
        return transaction.pendingInputs.contains(where: { $0.walletId == walletId })
    }

    let modelContainer: ModelContainer

    /// Network this handler's owning `PlatformWalletManager` is bound
    /// to. When set, `loadWalletList` filters out persisted wallets
    /// from other networks so a per-network manager only restores its
    /// own wallets. `nil` keeps the legacy "load every wallet"
    /// behavior for callers that don't yet thread network through —
    /// once the example app's `WalletManagerStore` is the only
    /// caller, the `nil` path can be retired.
    let network: Network?

    /// Background context for writing from callback threads.
    ///
    /// `ModelContext` is not thread-safe — touching it from the
    /// Tokio worker threads that drive the Rust persistence
    /// callbacks corrupts SwiftData's internal state and crashes
    /// inside `fetch`/`save`. The context is therefore confined to
    /// `serialQueue`: every public entry point wraps its body in
    /// `onQueue { … }`, and internal helpers (`upsertTransaction`,
    /// `markUtxoSpent`, …) assume they are already on the queue.
    private let backgroundContext: ModelContext

    /// Serial queue that owns `backgroundContext` and any other
    /// non-Sendable handler state (`loadAllocations`). All public
    /// entry points — both the FFI callback shims and the
    /// app-facing accessors — funnel through `onQueue` so the
    /// context is only ever touched on this queue.
    private let serialQueue = DispatchQueue(
        label: "org.dash.platform-wallet.persistence",
        qos: .userInitiated
    )

    /// True while inside a begin/end changeset bracket. When set,
    /// per-kind helpers skip their own `backgroundContext.save()` and
    /// let `endChangeset` commit (or rollback) the whole round
    /// atomically.
    private var inChangeset = false

    /// In-memory index over the rows the open changeset round has
    /// inserted into `backgroundContext` but not yet saved, keyed by the
    /// same columns the hot-path fetches filter on.
    ///
    /// Why it exists: a `FetchDescriptor` with the default
    /// `includePendingChanges == true` evaluates its predicate IN MEMORY
    /// against every unsaved insert of the target entity —
    /// `Predicate.evaluate` walks the key path per row, with a dynamic
    /// cast per step. The `#Index`/`.unique` declarations on the models
    /// only accelerate the SQL half of the fetch; the pending-changes
    /// half is always a linear scan. Because the whole round defers its
    /// `save()` to `endChangeset` (the `inChangeset` contract above), a
    /// large wallet's initial scan accumulates thousands of unsaved
    /// inserts in one round, and every subsequent fetch paid O(inserts
    /// so far) — quadratic over the round, and measured as ~99% of CPU
    /// on `serialQueue` minutes after the SPV scan itself finished.
    ///
    /// How it is used: while the index is non-nil, the lookup helpers
    /// (`fetchTransactionRow`, `fetchTxoRow`, `pendingInputRows`,
    /// `coreAddressRow`) consult it first and run their store fetch with
    /// `includePendingChanges = false`, so SQLite answers from its
    /// indexes and never triggers the in-memory scan. The single-object
    /// maps are READ-THROUGH: they hold both this round's unsaved
    /// inserts (registered at the insert site) and every row a store
    /// fetch has already resolved this round (registered by the helper).
    /// Caching store hits is not an optimization — it is load-bearing
    /// for correctness: a store-only fetch that matches an
    /// already-registered object REFRESHES that object to its store
    /// values, silently discarding the round's unsaved attribute
    /// mutations (unlike the default pending-changes fetch, which
    /// returns the object with its in-memory state; staged deletions do
    /// survive the refresh). Registering every resolution means each
    /// key touches the store at most once per round — at first touch,
    /// before the round can have mutated the object — so the refresh
    /// never has anything to discard. Both sources stay disjoint
    /// because `beginChangeset` builds the index only over a clean
    /// context. Rows deleted mid-round are filtered by `isDeleted` on
    /// both sources (index entries are deliberately never
    /// unregistered — `isDeleted` already answers the question, and it
    /// also covers deletes on paths that don't know about the index,
    /// e.g. wallet removal).
    ///
    /// Lifecycle: built by `beginChangeset`, discarded in
    /// `endChangeset`'s `defer` on both the commit and rollback paths —
    /// after a commit the cached rows are ordinary saved rows the store
    /// fetch finds on its own, and on rollback the context un-inserts /
    /// reverts every one of them, so the index dies with the round
    /// either way and never leaks state across rounds. `nil` outside a
    /// round (and inside a round that began on a dirty context — see
    /// `beginChangeset`), in which case the lookup helpers run the
    /// exact pre-index fetch, pending changes included.
    private struct ChangesetRoundIndex {
        var transactionsByTxid: [Data: PersistentTransaction] = [:]
        var txosByOutpoint: [Data: PersistentTxo] = [:]
        /// `PersistentPendingInput.outpoint` is deliberately not unique
        /// (re-org / double-spend can stack rows on one outpoint — see
        /// the model), so this holds only the round's staged inserts
        /// per key; saved rows come from the store fetch each time.
        /// Pending rows need no read-through registration because
        /// nothing mutates their attributes before the sweep pass, and
        /// sweeps run last in the round (see `pendingInputRows`).
        var pendingInputsByOutpoint: [Data: [PersistentPendingInput]] = [:]
        var coreAddressesByAddress: [String: PersistentCoreAddress] = [:]
    }
    private var roundIndex: ChangesetRoundIndex?

    /// Breadcrumb backfills that arrived on the serial queue while a
    /// changeset round was open. The backfill both mutates
    /// `backgroundContext` and saves it, so running it mid-round would
    /// commit the round's staged (uncommitted) writes early and break the
    /// "each Rust `store()` is one atomic transaction" invariant. Instead
    /// the request is parked here and drained by `endChangeset` once the
    /// round has committed/rolled back and `inChangeset` is clear — the
    /// backfill still completes, just cleanly outside any open round.
    /// Confined to `serialQueue` like all other mutable handler state.
    private var deferredBackfills: [(walletId: Data, items: [KeychainManager.IdentityPrivateKeyMetadata])] = []

    /// DashPay payment rows the persister callback could not stage
    /// because the owner `PersistentIdentity` row wasn't resolvable
    /// mid-round. Normally the owner is visible — the identities
    /// callback fires before the payments callback in the same Rust
    /// `store()` round, and `FetchDescriptor` sees the round's pending
    /// inserts — so this only holds rows whose owner is in neither the
    /// current round (yet) nor the store. Drained by `endChangeset`
    /// BEFORE the round's single `save()`, so parked rows commit
    /// atomically with everything else; a group whose owner is still
    /// unresolvable at that point fails the whole round (rollback +
    /// failure reported to Rust) rather than committing a lossy
    /// persist. Cleared without staging on a failed round: the Rust
    /// side rolled its in-memory entries back too, so persisting them
    /// later would fabricate history. Never survives a round either
    /// way, so it cannot grow across rounds. Confined to `serialQueue`
    /// like all other mutable handler state.
    private var deferredPaymentUpserts: [(ownerIdentityId: Data, payments: [DashPayPayment])] = []

    public init(modelContainer: ModelContainer, network: Network? = nil) {
        self.modelContainer = modelContainer
        self.network = network
        self.backgroundContext = ModelContext(modelContainer)
        // Autosave off: this context is the transaction buffer for the
        // begin → changeset → sweeps → end sequence, and autosave can commit
        // its pending mutations between those callbacks. Since sweeps moved
        // to their own callback the round spans two calls, so an autosave
        // landing in between would make the watermark and the additive rows
        // durable while the removal is still unstaged — and `rollback()`
        // cannot take back a save that already happened. The handler
        // attests `ATOMIC_CHANGESETS`, which is what Rust now relies on to
        // trust the split transport, so that guarantee has to be real.
        //
        // Nothing depends on the implicit commits: every path either runs
        // inside a round, which `endChangeset` commits with its single
        // `save()`, or saves itself when `inChangeset` is clear.
        self.backgroundContext.autosaveEnabled = false
    }

    /// Synchronously run `body` on `serialQueue`.
    ///
    /// All public methods that read or write `backgroundContext`
    /// (or `loadAllocations`) must call through this helper.
    /// `sync` matches the synchronous FFI contract — the C shims
    /// need a return value before yielding back to Rust — and
    /// turns the queue into the handler's de-facto actor: only
    /// one thread runs SwiftData operations at a time.
    ///
    /// Do not call `onQueue` from another method that already
    /// runs on the queue; `DispatchQueue.sync` will deadlock on
    /// recursive entry. The internal helpers in this file all
    /// assume they are already on the queue and call
    /// `backgroundContext` directly.
    private func onQueue<T>(_ body: () throws -> T) rethrows -> T {
        try serialQueue.sync(execute: body)
    }

    // MARK: - Platform Address Balances

    /// Apply an incremental BLAST balance changeset to SwiftData.
    ///
    /// BLAST sync identifies each address by its 20-byte
    /// `addressHash`. `PersistentPlatformAddress` rows are seeded by
    /// the address-emit path (`persistAccountAddresses` for
    /// PlatformPayment accounts), which knows the full DIP-0018
    /// bech32m form plus derivation metadata. This callback only
    /// refreshes the volatile fields (balance, nonce, `isUsed`). If
    /// BLAST reports balances for an address we never emitted (e.g.
    /// cache wipe between runs), we skip it — the next
    /// address-emit pass will bring the row back and the next sync
    /// will fill in the balance.
    ///
    /// The entry also carries `accountIndex` / `addressIndex`, but this
    /// callback deliberately does NOT write them: the derivation index is
    /// authoritative from the address-emit path (the row's index is fixed
    /// the moment its address is derived and never changes). A reconcile
    /// *removal* can arrive here carrying a pool-resolved `addressIndex`
    /// that conflicts with another address's true index (the Rust provider
    /// still emits the zero so the balance can't resurrect — see
    /// `commit_reconciliation`'s index-conflict removal path). Overwriting
    /// the row's index with that value would make two durable rows claim
    /// one index; on the next restore the bijection rebuild
    /// (`insert_persisted_entry`) would then drop the funded pairing and
    /// orphan its balance. So the balance path owns balance/nonce/`isUsed`
    /// only; derivation metadata stays as the address-emit path set it.
    func persistAddressBalances(
        walletId: Data,
        entries: [(UInt8, Data, UInt64, UInt32, UInt32, UInt32, UInt64)]
    ) {
        onQueue {
            // `accountIndex` / `addressIndex` (tuple slots 5 and 6) are
            // intentionally ignored — see the note above.
            for (_, addressHash, balance, nonce, _, _, asOfHeight) in entries {
                // Scope by walletId + hash: a hash-only predicate can match
                // another wallet's row in a multi-wallet store (same seed
                // imported on coin-type-sharing networks, watch-only
                // duplicates) — the same fix the view-level upserts carry.
                let descriptor = FetchDescriptor<PersistentPlatformAddress>(
                    predicate: #Predicate {
                        $0.walletId == walletId && $0.addressHash == addressHash
                    }
                )
                guard let existing = try? backgroundContext.fetch(descriptor).first else {
                    continue
                }
                existing.balance = balance
                existing.nonce = nonce
                // Balance height pin — persisted verbatim so the load
                // path can hand it back to Rust (delta-replay gating).
                existing.lastSeenHeight = asOfHeight
                if balance > 0 || nonce > 0 {
                    existing.isUsed = true
                }
                existing.lastUpdated = Date()
            }

            // No save() here — this handler runs inside the Rust-side
            // changeset round, which is bracketed by changesetBegin /
            // changesetEnd; the atomic save fires in endChangeset.
        }
    }

    // MARK: - Asset locks

    /// Apply an `AssetLockChangeSet` projection to SwiftData.
    ///
    /// The Rust-side asset-lock manager emits a changeset on every
    /// status transition (`Built → Broadcast → InstantSendLocked →
    /// ChainLocked`) and on consumption (the registration flow drops
    /// the row once the IdentityCreate state transition lands). Each
    /// `upsert` maps onto a `PersistentAssetLock` row keyed by
    /// `outPointHex` (the 36-byte outpoint encoded as
    /// `<txid_display_hex>:<vout>`); each `removed` entry deletes the
    /// matching row. `RegistrationProgressView` watches these rows
    /// via `@Query` to drive the stage progress bar.
    ///
    /// No `save()` here — bracketed by `beginChangeset` /
    /// `endChangeset` from the Rust `store()` round.
    func persistAssetLocks(
        walletId: Data,
        upserts: [AssetLockEntrySnapshot],
        removed: [Data]
    ) {
        onQueue {
            for entry in upserts {
                let outPointHex = entry.outPointHex
                let descriptor = FetchDescriptor<PersistentAssetLock>(
                    predicate: #Predicate { $0.outPointHex == outPointHex }
                )
                if let existing = try? backgroundContext.fetch(descriptor).first {
                    // Consumed (4) is the terminal lifecycle state — never
                    // let a non-Consumed snapshot regress it. Writers race:
                    // the wallet-event adapter's batched drain can deliver a
                    // stale reconstruction/enrichment snapshot AFTER the
                    // live flow's synchronous consumption write, and this
                    // upsert is otherwise last-write-wins. Mirrors the same
                    // guard in `AssetLockChangeSet::merge` and the
                    // rs-platform-wallet-storage sqlite upsert; all other
                    // transitions stay last-write-wins because non-terminal
                    // statuses legitimately move both ways.
                    if existing.statusRaw == 4 && entry.statusRaw != 4 {
                        continue
                    }
                    existing.walletId = walletId
                    existing.transactionBytes = entry.transactionBytes
                    existing.fundingTypeRaw = entry.fundingTypeRaw
                    existing.identityIndexRaw = entry.identityIndexRaw
                    existing.accountIndexRaw = entry.accountIndexRaw
                    existing.amountDuffs = entry.amountDuffs
                    existing.statusRaw = entry.statusRaw
                    existing.proofBytes = entry.proofBytes
                    existing.updatedAt = Date()
                } else {
                    let record = PersistentAssetLock(
                        outPointHex: outPointHex,
                        walletId: walletId,
                        transactionBytes: entry.transactionBytes,
                        fundingTypeRaw: entry.fundingTypeRaw,
                        identityIndexRaw: entry.identityIndexRaw,
                        accountIndexRaw: entry.accountIndexRaw,
                        amountDuffs: entry.amountDuffs,
                        statusRaw: entry.statusRaw,
                        proofBytes: entry.proofBytes
                    )
                    backgroundContext.insert(record)
                }
            }

            for outPointHex in removed {
                let hex = PersistentAssetLock.encodeOutPoint(rawBytes: outPointHex)
                let descriptor = FetchDescriptor<PersistentAssetLock>(
                    predicate: #Predicate { $0.outPointHex == hex }
                )
                if let existing = try? backgroundContext.fetch(descriptor).first {
                    // Same terminal rule as the upsert guard above: a
                    // Consumed (4) row is deliberately retained for
                    // historical lookup and the only removal emitter
                    // (`untrack_asset_lock`) targets rejected Built
                    // rows — a removal reaching a consumed row is by
                    // construction a stale write.
                    if existing.statusRaw == 4 {
                        continue
                    }
                    backgroundContext.delete(existing)
                }
            }
        }
    }

    /// Persist created/updated invitations (Sent-invitations bridge). Mirrors
    /// `persistAssetLocks`, simpler — POD entries, no owned buffers. Runs
    /// entirely on `onQueue`, body **inline** (never re-enter `onQueue` — a
    /// recursive `serialQueue.sync` deadlocks); **no `save()` here**
    /// (`endChangeset` commits the round). Sets `walletId` on BOTH the insert
    /// and update branch (the view's `@Query` filters on it). The removal path
    /// keys via the same `encodeOutPoint` display form the upsert stores (the
    /// T1 seam), so an upsert and a later removal of the same outpoint match.
    /// Returns `true` iff every upsert/removal was applied. A `false` return
    /// drives the callback to signal `store()` failure so the Rust caller
    /// (`create_invitation`) surfaces a funded-but-unrecorded voucher instead of
    /// reporting success — SwiftData is the sole UI source (no Rust→Swift
    /// rehydrate), so a silently skipped upsert would make a funded invitation
    /// vanish from the Sent list with no trace.
    func persistInvitations(
        walletId: Data,
        upserts: [InvitationEntrySnapshot],
        removed: [Data]
    ) -> Bool {
        onQueue {
            // A fetch failure on any row drops that mutation; report it so the
            // round rolls back rather than half-committing. The commit itself is
            // the shared changeset `save()` in `endChangeset`; the file-wide
            // `try?`-on-save convention (asset locks, identities, txs, …) is
            // intentionally left unchanged here — repo-wide persistence-error
            // telemetry is a separate follow-up.
            var allPersisted = true
            for entry in upserts {
                let outPointHex = entry.outPointHex
                let descriptor = FetchDescriptor<PersistentInvitation>(
                    predicate: #Predicate { $0.outPointHex == outPointHex }
                )
                let existing: PersistentInvitation?
                do {
                    existing = try backgroundContext.fetch(descriptor).first
                } catch {
                    print("⚠️ persistInvitations: fetch failed for outpoint \(outPointHex) — skipping upsert; this invitation may be missing from the Sent list: \(error)")
                    allPersisted = false
                    continue
                }
                if let existing {
                    existing.walletId = walletId
                    existing.rawOutPoint = entry.rawOutPoint
                    existing.fundingIndexRaw = entry.fundingIndexRaw
                    existing.amountDuffs = entry.amountDuffs
                    existing.expiryUnix = entry.expiryUnix
                    existing.createdAtSecs = entry.createdAtSecs
                    existing.hasInviter = entry.hasInviter
                    existing.statusRaw = entry.statusRaw
                    existing.updatedAt = Date()
                } else {
                    let record = PersistentInvitation(
                        outPointHex: outPointHex,
                        rawOutPoint: entry.rawOutPoint,
                        walletId: walletId,
                        fundingIndexRaw: entry.fundingIndexRaw,
                        amountDuffs: entry.amountDuffs,
                        expiryUnix: entry.expiryUnix,
                        createdAtSecs: entry.createdAtSecs,
                        hasInviter: entry.hasInviter,
                        statusRaw: entry.statusRaw
                    )
                    backgroundContext.insert(record)
                }
            }

            for rawOutPoint in removed {
                let hex = PersistentAssetLock.encodeOutPoint(rawBytes: rawOutPoint)
                let descriptor = FetchDescriptor<PersistentInvitation>(
                    predicate: #Predicate { $0.outPointHex == hex }
                )
                do {
                    if let existing = try backgroundContext.fetch(descriptor).first {
                        backgroundContext.delete(existing)
                    }
                } catch {
                    print("⚠️ persistInvitations: fetch failed for removal of outpoint \(hex) — stale invitation may linger in the Sent list: \(error)")
                    allPersisted = false
                }
            }
            return allPersisted
        }
    }

    /// Mirror the DPNS username-marketplace rows onto `PersistentDPNSName`.
    ///
    /// Rows are keyed the same way `upsertDPNSNames` keys the label cache
    /// — `(networkRaw, normalizedParentDomainName, normalizedLabel)`,
    /// mirroring the DPNS contract's `parentNameAndLabel` unique index —
    /// so a marketplace row and the identity label snapshot converge on
    /// ONE row per name rather than two competing ones. The difference is
    /// which columns each owns: the identity snapshot owns
    /// `label`/`acquiredAt`, this owns the marketplace section.
    ///
    /// Unlike the label cache this carries the real parent domain (the
    /// FFI forwards it), so no `"dash"` default is stamped here.
    /// Marketplace rows own only the marketplace columns; `isOwned` remains
    /// exclusively controlled by the canonical identity snapshot. A row first
    /// observed here is initialized fail-closed as not owned until such a
    /// snapshot includes it.
    ///
    /// A row whose owning identity isn't in the store yet is logged and
    /// skipped rather than failing the round: the relationship is
    /// non-optional, the Rust side treats a failed marketplace store as
    /// self-healing (the next sync pass re-emits the same rows), and
    /// rolling the round back would also discard the identity insert that
    /// makes the next pass succeed. A genuine SwiftData fetch failure DOES
    /// fail the round, matching `persistInvitations`.
    ///
    /// `removed` document ids clear the marketplace section (and only it)
    /// — the label cache belongs to the identity snapshot, so dropping the
    /// whole row here would destroy state this callback does not own. A
    /// cleared row reads as "not tracked" via `documentIdBase58 == nil`,
    /// which is what `PersistentDPNSName.saleStatus` gates on.
    ///
    /// Runs entirely on `onQueue`, body inline (never re-enter `onQueue`);
    /// no `save()` here — `endChangeset` commits the round.
    func persistDpnsNameStates(
        walletId: Data,
        upserts: [DpnsNameStateSnapshot],
        removed: [String]
    ) -> Bool {
        onQueue {
            var allPersisted = true

            for entry in upserts {
                let identityId = entry.walletIdentityId
                let identityDescriptor = FetchDescriptor<PersistentIdentity>(
                    predicate: #Predicate { $0.identityId == identityId }
                )
                let identityRow: PersistentIdentity?
                do {
                    identityRow = try backgroundContext.fetch(identityDescriptor).first
                } catch {
                    print("⚠️ persistDpnsNameStates: identity fetch failed for \(identityId.toBase58String()) — skipping marketplace row for \"\(entry.label)\": \(error)")
                    allPersisted = false
                    continue
                }
                guard let identityRow else {
                    // Not an error: the identity row simply isn't staged
                    // yet. The next marketplace sync pass re-emits this row.
                    print("ℹ️ persistDpnsNameStates: no identity row for \(identityId.toBase58String()) yet — marketplace state for \"\(entry.label)\" will land on the next sync pass")
                    continue
                }

                let networkRaw = identityRow.networkRaw
                let normalizedLabel = entry.normalizedLabel
                let normalizedParent = entry.normalizedParentDomainName
                let descriptor = FetchDescriptor<PersistentDPNSName>(
                    predicate: #Predicate {
                        $0.networkRaw == networkRaw
                            && $0.normalizedParentDomainName == normalizedParent
                            && $0.normalizedLabel == normalizedLabel
                    }
                )
                let existing: PersistentDPNSName?
                do {
                    existing = try backgroundContext.fetch(descriptor).first
                } catch {
                    print("⚠️ persistDpnsNameStates: fetch failed for \"\(normalizedLabel)\" — its price/sale state may be stale in the UI: \(error)")
                    allPersisted = false
                    continue
                }

                let row: PersistentDPNSName
                if let existing {
                    row = existing
                    // Rebind to the identity Rust tracks for this one
                    // per-document row. For a name that left the wallet this
                    // is the previous owner, preserving departed history. For
                    // a same-wallet transfer Rust deliberately emits the
                    // current owner's `Owned` row, which wins over the old
                    // owner's departure because this schema has one unique row
                    // per name.
                    if row.identity !== identityRow {
                        row.identity = identityRow
                    }
                } else {
                    // No label-cache row yet (a name observed by the
                    // marketplace sweep before the identity snapshot
                    // carried it). The FFI forwards the NORMALIZED parent
                    // domain only, so it seeds both the display and the
                    // normalized column — identical for "dash", DPNS's
                    // only top-level domain today, and the init re-runs
                    // the (idempotent) normalization for the index column.
                    row = PersistentDPNSName(
                        identity: identityRow,
                        label: entry.label,
                        parentDomainName: entry.normalizedParentDomainName,
                        isOwned: false
                    )
                    backgroundContext.insert(row)
                }

                // The display label can gain corrected casing between
                // flushes for the same normalized form; the normalized
                // index columns don't move, so the unique constraint holds.
                if row.label != entry.label {
                    row.label = entry.label
                }
                row.documentIdBase58 = entry.documentIdBase58
                row.priceCredits = entry.priceCredits.map { Int64(bitPattern: $0) }
                row.saleStatusRaw = entry.statusRaw
                row.counterpartyIdBase58 = entry.counterpartyIdBase58
                row.documentCreatedAtMs = entry.createdAtMs
                row.documentUpdatedAtMs = entry.updatedAtMs
                row.documentTransferredAtMs = entry.transferredAtMs
                row.marketplaceUpdatedAt = entry.lastSyncedAtMs
                row.lastUpdated = Date()
            }

            for documentId in removed {
                let descriptor = FetchDescriptor<PersistentDPNSName>(
                    predicate: #Predicate { $0.documentIdBase58 == documentId }
                )
                do {
                    for row in try backgroundContext.fetch(descriptor) {
                        // Clear only the marketplace section — the label
                        // cache is the identity snapshot's to own.
                        row.documentIdBase58 = nil
                        row.priceCredits = nil
                        row.saleStatusRaw = 0
                        row.counterpartyIdBase58 = nil
                        row.documentCreatedAtMs = nil
                        row.documentUpdatedAtMs = nil
                        row.documentTransferredAtMs = nil
                        row.marketplaceUpdatedAt = 0
                        row.lastUpdated = Date()
                    }
                } catch {
                    print("⚠️ persistDpnsNameStates: fetch failed for removal of document \(documentId) — stale price/sale state may linger: \(error)")
                    allPersisted = false
                }
            }

            return allPersisted
        }
    }

    /// Load all persisted tracked asset locks for a wallet — used by
    /// the wallet load path to rebuild `unused_asset_locks` on the
    /// Rust side so an in-flight registration that was interrupted by
    /// an app kill can resume from the latest status without
    /// rebroadcasting the asset-lock transaction.
    public func loadCachedAssetLocks(walletId: Data) -> [AssetLockEntrySnapshot] {
        onQueue { loadCachedAssetLocksOnQueue(walletId: walletId) }
    }

    /// On-queue implementation reused by the load-wallet-list path
    /// without re-entering `onQueue`.
    func loadCachedAssetLocksOnQueue(walletId: Data) -> [AssetLockEntrySnapshot] {
        let descriptor = FetchDescriptor<PersistentAssetLock>(
            predicate: PersistentAssetLock.predicate(walletId: walletId)
        )
        guard let records = try? backgroundContext.fetch(descriptor) else {
            return []
        }
        return records.map { record in
            AssetLockEntrySnapshot(
                outPointHex: record.outPointHex,
                transactionBytes: record.transactionBytes,
                fundingTypeRaw: record.fundingTypeRaw,
                identityIndexRaw: record.identityIndexRaw,
                accountIndexRaw: record.accountIndexRaw,
                amountDuffs: record.amountDuffs,
                statusRaw: record.statusRaw,
                proofBytes: record.proofBytes
            )
        }
    }

    /// Owned snapshot of an `AssetLockEntryFFI` row. Same lifetime
    /// rationale as `IdentityEntrySnapshot` — the callback copies
    /// every byte buffer into owned `Data` before invoking the
    /// handler, so the handler runs against pure-Swift values
    /// regardless of when the Rust-side allocation gets reclaimed.
    public struct AssetLockEntrySnapshot {
        public let outPointHex: String
        public let transactionBytes: Data
        public let fundingTypeRaw: Int
        public let identityIndexRaw: Int32
        public let accountIndexRaw: Int32
        public let amountDuffs: Int64
        public let statusRaw: Int
        public let proofBytes: Data?
    }

    /// Owned snapshot of an `InvitationEntryFFI` row. All-POD — the callback
    /// copies the outpoint bytes into owned `Data` (`rawOutPoint`) and
    /// precomputes the display-form key (`outPointHex`) before invoking the
    /// handler, so the handler runs against pure-Swift values regardless of when
    /// the Rust-side buffer is reclaimed.
    public struct InvitationEntrySnapshot {
        public let outPointHex: String
        public let rawOutPoint: Data
        public let fundingIndexRaw: Int
        public let amountDuffs: Int64
        public let expiryUnix: Int
        public let createdAtSecs: Int
        public let hasInviter: Bool
        public let statusRaw: Int
    }

    /// Owned snapshot of one `DpnsNameStateFFI` — the DPNS
    /// username-marketplace state of a name tracked for a wallet
    /// identity. Decouples the three C strings from the FFI heap so the
    /// callback can return immediately and Rust can run its free-loop.
    ///
    /// Optionals mirror the FFI's `has_*` flags: `priceCredits == nil`
    /// means "not listed for sale" (never a 0-credit listing), and a nil
    /// timestamp means the document didn't carry one (never the epoch).
    public struct DpnsNameStateSnapshot {
        /// The DPNS `domain` document id, base58 — this row's key.
        public let documentIdBase58: String
        /// The wallet identity this row is tracked for.
        public let walletIdentityId: Data
        /// Display label, e.g. "Alice".
        public let label: String
        /// Homograph-normalized label, e.g. "a11ce".
        public let normalizedLabel: String
        /// Normalized parent domain — part of the row's uniqueness key.
        public let normalizedParentDomainName: String
        /// Listed price in credits, or nil when not for sale.
        public let priceCredits: UInt64?
        /// 0 = owned, 1 = sold, 2 = transferred.
        public let statusRaw: Int16
        /// Buyer / recipient of a departed name, base58. Nil while owned
        /// or when the counterparty could not be resolved.
        public let counterpartyIdBase58: String?
        /// Domain document `$createdAt` in Unix ms, or nil when absent.
        public let createdAtMs: UInt64?
        /// Domain document `$updatedAt` in Unix ms, or nil when absent.
        public let updatedAtMs: UInt64?
        /// Domain document `$transferredAt` in Unix ms, or nil when absent.
        public let transferredAtMs: UInt64?
        /// Unix ms of the pass that wrote this row.
        public let lastSyncedAtMs: UInt64
    }

    /// Load all cached platform-address balances for a wallet. Tuple
    /// shape matches the Rust-side `AddressBalanceEntryFFI` layout so
    /// the load-wallet-list path can re-seed the provider on startup
    /// without a full rescan.
    public func loadCachedBalances(walletId: Data) -> [(UInt8, [UInt8], UInt64, UInt32, UInt32, UInt32, UInt64)] {
        onQueue { loadCachedBalancesOnQueue(walletId: walletId) }
    }

    /// Implementation for `loadCachedBalances` that assumes it is
    /// already running on `serialQueue`. Lets internal on-queue
    /// callers (`loadWalletList`) reuse the body without recursing
    /// through `onQueue`, which would deadlock.
    private func loadCachedBalancesOnQueue(walletId: Data) -> [(UInt8, [UInt8], UInt64, UInt32, UInt32, UInt32, UInt64)] {
        let descriptor = FetchDescriptor<PersistentPlatformAddress>(
            predicate: PersistentPlatformAddress.predicate(walletId: walletId)
        )

        guard let records = try? backgroundContext.fetch(descriptor) else {
            return []
        }

        return records.map { record in
            (
                record.addressType,
                Array(record.addressHash),
                record.balance,
                record.nonce,
                record.accountIndex,
                record.addressIndex,
                record.lastSeenHeight
            )
        }
    }

    // MARK: - Sync State

    /// Upsert sync state into SwiftData.
    ///
    /// The BLAST watermark is network-scoped, not wallet-scoped: every
    /// wallet on the same network shares one merged checkpoint.
    func persistSyncState(
        walletId: Data,
        syncHeight: UInt64,
        syncTimestamp: UInt64,
        lastKnownRecentBlock: UInt64
    ) {
        onQueue {
            guard let network = walletNetwork(walletId: walletId) else {
                return
            }
            let scopeId = syncStateScopeId(for: network)
            let descriptor = FetchDescriptor<PersistentPlatformAddressesSyncState>(
                predicate: #Predicate { $0.walletId == scopeId }
            )

            if let existing = try? backgroundContext.fetch(descriptor).first {
                existing.network = network
                existing.syncHeight = syncHeight
                existing.syncTimestamp = syncTimestamp
                existing.lastKnownRecentBlock = lastKnownRecentBlock
                existing.lastUpdated = Date()
            } else {
                let record = PersistentPlatformAddressesSyncState(
                    walletId: scopeId,
                    network: network,
                    syncHeight: syncHeight,
                    syncTimestamp: syncTimestamp,
                    lastKnownRecentBlock: lastKnownRecentBlock
                )
                backgroundContext.insert(record)
            }
            // No save() — bracketed by changesetBegin/End from the
            // Rust store() round.
        }
    }

    /// Load cached sync state for a wallet's network.
    public func loadCachedSyncState(walletId: Data) -> (syncHeight: UInt64, syncTimestamp: UInt64, lastKnownRecentBlock: UInt64)? {
        onQueue {
            guard let network = walletNetwork(walletId: walletId) else {
                return nil
            }
            return loadCachedSyncStateOnQueue(network: network)
        }
    }

    /// Load cached sync state for a specific network.
    public func loadCachedSyncState(network: Network) -> (syncHeight: UInt64, syncTimestamp: UInt64, lastKnownRecentBlock: UInt64)? {
        onQueue { loadCachedSyncStateOnQueue(network: network) }
    }

    /// Implementation for `loadCachedSyncState` that assumes it is
    /// already running on `serialQueue`. Both public overloads
    /// route through this so the `(walletId:)` variant can resolve
    /// the network and read the row in a single queue hop without
    /// recursing into `onQueue`, which would deadlock.
    private func loadCachedSyncStateOnQueue(network: Network) -> (syncHeight: UInt64, syncTimestamp: UInt64, lastKnownRecentBlock: UInt64)? {
        let scopeId = syncStateScopeId(for: network)
        let descriptor = FetchDescriptor<PersistentPlatformAddressesSyncState>(
            predicate: #Predicate { $0.walletId == scopeId }
        )

        guard let record = try? backgroundContext.fetch(descriptor).first else {
            return nil
        }

        return (record.syncHeight, record.syncTimestamp, record.lastKnownRecentBlock)
    }

    // MARK: - Wallet Changeset (transactions, utxos, accounts, balance, chain)

    /// Apply a full `WalletChangeSetFFI` to SwiftData.
    ///
    /// Called from the Rust persister when an SPV round produces core-
    /// wallet state changes. Upserts PersistentAccount / Transaction /
    /// Utxo records so views observing via `@Query` update automatically.
    ///
    /// Returns `false` when the round could not be applied, which the C shim
    /// forwards to Rust so `store()` rolls the round back instead of treating
    /// it as durable. Everything this method itself applies is additive, so
    /// only a failed wallet lookup reports it here; the round's subtractive
    /// part arrives through `persistWalletChangesetSweeps` below, with its
    /// own failure path.
    @discardableResult
    func persistWalletChangeset(
        walletId: Data,
        changeset: UnsafePointer<WalletChangeSetFFI>
    ) -> Bool {
        onQueue {
            // A stale post-deletion callback is not a failure — there is
            // simply nothing left to write to. A fetch that *throws* is a
            // different matter: reporting success would let Rust discard the
            // round's sweep, and a later callback could then persist a height
            // beyond a removal that never landed.
            let wallet: PersistentWallet?
            do {
                wallet = try fetchWalletRecord(walletId: walletId)
            } catch {
                print(
                    "⚠️ persistWalletChangeset: wallet lookup failed: "
                        + "\(error.localizedDescription); failing the round"
                )
                return false
            }
            guard let wallet else { return true }
            let cs = changeset.pointee

            // Chain update.
            if cs.has_chain {
                if cs.chain.has_synced_height {
                    wallet.syncedHeight = cs.chain.synced_height
                }
                wallet.lastUpdated = Date()
            }

            // Persisted `last_applied_chain_lock` — bincode bytes
            // from the FFI carry the wallet's
            // `WalletMetadata::last_applied_chain_lock` snapshot for
            // restart roundtrip. Stored as opaque `Data` (decoded on
            // the Rust load side); SPV persists its own
            // `best_chainlock` independently so this column is the
            // wallet-side mirror, not a duplicate of SPV state.
            // Pre-feature rows / wallets that have never observed a
            // ChainLock carry `null` from Rust and stay `nil` here.
            if cs.last_applied_chain_lock_bytes_len > 0,
               let clPtr = cs.last_applied_chain_lock_bytes {
                let bytes = Data(
                    bytes: clPtr,
                    count: Int(cs.last_applied_chain_lock_bytes_len)
                )
                wallet.lastAppliedChainLockBytes = bytes
                wallet.lastUpdated = Date()
            }

            // Balance delta — Rust still emits per-round deltas, but the
            // PersistentWallet `balance*` fields they used to update were
            // removed (canonical source is now the in-memory account
            // totals via `walletManager.accountBalances(for:)`). Bump the
            // updated timestamp so the row reflects the persistence round
            // and discard the payload itself.
            if cs.has_balance {
                wallet.lastUpdated = Date()
            }

            // Per-account: transactions, UTXOs, pool state.
            if cs.accounts_count > 0, let accountsPtr = cs.accounts {
                for i in 0..<Int(cs.accounts_count) {
                    let acc = accountsPtr[i]
                    applyAccountChangeset(walletRecord: wallet, acc: acc)
                }
            }

            // Swept transactions no longer ride this struct: they arrive
            // through `persistWalletChangesetSweeps(walletId:sweeps:count:)`
            // below, fired by Rust immediately after this callback in the
            // same round. The struct crosses the C ABI by bare pointer, so a
            // field appended to it cannot be proven present to a consumer
            // built after a producer — the extension callback's negotiated
            // `struct_size` is what carries that proof instead.

            // No save() — bracketed by changesetBegin/End.
            return true
        }
    }

    /// Apply a round's sweep batches — the one subtractive part of the
    /// changeset path, delivered through the size-negotiated
    /// `PersistenceCallbacksExtension` slot rather than as a field on
    /// `WalletChangeSetFFI` (see `persistWalletChangeset` for why). Rust
    /// fires this right after that callback within the same
    /// begin/end round, so a wallet-relevant winner riding in the round has
    /// its claim on the shared inputs already recorded when the removal here
    /// decides which links are left pointing at a dead transaction.
    ///
    /// Returns `false` to fail the round, same contract as
    /// `persistWalletChangeset`: a deletion that silently didn't happen
    /// would have Rust clear the sweep while the dead row survives to be
    /// replayed at the next load.
    @discardableResult
    func persistWalletChangesetSweeps(
        walletId: Data,
        sweeps: UnsafePointer<SweepBatchFFI>?,
        count: UInt
    ) -> Bool {
        onQueue {
            // Same wallet gate as `persistWalletChangeset`: a stale
            // post-deletion callback has nothing left to write to, but a
            // lookup that throws must fail the round rather than let Rust
            // discard a sweep that never landed.
            let wallet: PersistentWallet?
            do {
                wallet = try fetchWalletRecord(walletId: walletId)
            } catch {
                print(
                    "⚠️ persistWalletChangesetSweeps: wallet lookup failed: "
                        + "\(error.localizedDescription); failing the round"
                )
                return false
            }
            guard wallet != nil else { return true }
            guard count > 0, let sweepsPtr = sweeps else { return true }

            // One batch at a time, in order. A later sweep can keep a
            // coin spent that an earlier one freed — each batch is only
            // true of the wallet it saw — so folding them together lets
            // the first answer outlive the last one that still holds.
            for batchIndex in 0..<Int(count) {
                let batch = sweepsPtr[batchIndex]

                // The coins this batch freed, as the 36-byte keys the
                // TXO rows are stored under.
                var released = Set<Data>()
                if batch.released_outpoints_count > 0,
                   let releasedPtr = batch.released_outpoints {
                    for i in 0..<Int(batch.released_outpoints_count) {
                        let outpoint = releasedPtr[i]
                        let txid = Swift.withUnsafeBytes(of: outpoint.txid) { Data($0) }
                        released.insert(
                            PersistentTxo.makeOutpoint(txid: txid, vout: outpoint.vout)
                        )
                    }
                }

                let supersededBy = Swift.withUnsafeBytes(of: batch.superseded_by) { Data($0) }

                guard batch.txids_count > 0, let txidsPtr = batch.txids else { continue }
                for i in 0..<Int(batch.txids_count) {
                    let txid = Swift.withUnsafeBytes(of: txidsPtr[i]) { Data($0) }
                    do {
                        try applySweptTransaction(
                            walletId: walletId,
                            txid: txid,
                            supersededBy: supersededBy,
                            released: released
                        )
                    } catch {
                        // Fail the round rather than report a deletion
                        // that did not happen: Rust would clear the sweep
                        // and the dead row would be replayed at the next
                        // load.
                        print(
                            "⚠️ persistWalletChangesetSweeps: sweep of "
                                + "\(txid.prefix(8).toHexString())… failed: "
                                + "\(error.localizedDescription); failing the round"
                        )
                        return false
                    }
                }
            }

            // No save() — bracketed by changesetBegin/End.
            return true
        }
    }

    /// Delete the mirror of a transaction the wallet swept.
    ///
    /// A swept transaction was a recorded spend that `supersededBy` provably
    /// beat to one of its inputs, so it can never confirm; Rust has already
    /// dropped it. Keeping the row would hand it back at the next load and
    /// re-create a balance the wallet has already corrected — this is the
    /// only removal the changeset path performs.
    ///
    /// `isGloballySwept` is upstream's word as of this callback, not a
    /// permanent verdict — the wallet's sweep state can itself be swept in
    /// turn (IS-lock precedence: a chainlocked return beats the IS-locked
    /// conflict that swept it originally), and `upsertTransaction` clears
    /// this flag when a later record reinstates the txid. See that
    /// method's doc comment for what reinstatement can and cannot undo.
    ///
    /// `commit_batch` calls `store()` once per wallet, and each of those
    /// commits independently — there is no single transaction spanning every
    /// wallet this sweep touches. That splits what has to be durable in
    /// *this* callback from what can wait for a later one: the outputs this
    /// row created are phantom money for every wallet, not just the one
    /// running right now, and once Rust has proven the row dead no
    /// restore/enumeration path may serve it to anyone — waiting for the
    /// last wallet's callback to confirm that would leave it acknowledged-but-
    /// resurrectable for however long the other wallets take to run, or
    /// forever if one of them crashes first or never arrives. So the outputs
    /// are deleted and `isGloballySwept` is set in EVERY callback that
    /// reaches this function, idempotently, before anything wallet-scoped is
    /// touched below. Physically removing `row` itself is different: that is
    /// safe to defer, because `isGloballySwept` already makes the row inert
    /// the moment the first callback sets it — see the ownership check near
    /// the bottom for why the row is still worth reclaiming once nothing
    /// points at it, now purely as housekeeping.
    ///
    /// The coins it claimed to *spend* split in two, and
    /// `released` is the authority on which is which:
    ///
    /// - an input named there came free — no surviving transaction spends it;
    /// - every other input it claimed was taken by the transaction that beat
    ///   it, and is gone.
    ///
    /// That distinction cannot be made here. Upstream only ever sweeps
    /// *unconfirmed* records, and this store flips `isSpent` only for a
    /// spender that reached a block, so a swept loser holds its inputs by
    /// link alone with `isSpent == false`; deleting the row nils the link and
    /// every one of those coins would fall back into the restore set,
    /// including the consumed one. Nor can the winner's own row be consulted:
    /// it need not be wallet-relevant at all, and even when it is, the sweep
    /// can be committed in a round that arrives before the winner's record.
    /// So upstream computes the split and names the freed coins, and this
    /// applies it verbatim — the rest are held spent with no spender named,
    /// which keeps them out of the restore set.
    ///
    /// A held input can also have no `PersistentTxo` at all yet — the loser
    /// was persisted before its own funding TXO was, so
    /// `resolveInputOutpoint` parked the claim as a `PersistentPendingInput`
    /// instead. `PersistentTransaction.pendingInputs` cascades on delete just
    /// like `outputs`, so left alone that claim would vanish with `row`
    /// below, and the funding TXO's own later `upsertUtxo` — even after a
    /// restart — would have nothing to tell it the coin isn't really free.
    /// A held pending input is therefore detached from `row` (so the cascade
    /// no longer reaches it) and repointed at `supersededBy` before the
    /// delete, flagged `isSweptTombstone` so `upsertUtxo` knows to keep the
    /// coin spent — durably, via `PersistentTxo.supersededByTxid` — once the
    /// funding TXO materializes rather than treating it as an ordinary
    /// in-flight spend. A released pending input needs none of this: it is
    /// left for the cascade, the same as a released materialized input needs
    /// no special handling beyond the loop above.
    ///
    /// A tombstoned row can itself need to move again: `supersededBy` is
    /// only this round's winner, and nothing stops it from losing a later
    /// round to a further winner while its own funding TXO is still
    /// unresolved. `row.pendingInputs` above cannot see that earlier
    /// tombstone — it already detached from `spendingTransaction` (and
    /// therefore from `row`) the moment it was first written — so it is
    /// looked up the only other way it is still findable, by the scalar
    /// `spendingTxid` it was repointed to, and carried the rest of the
    /// chain below: deleted if this round finally frees its outpoint,
    /// repointed at the new winner if not.
    ///
    /// `PersistentTransaction` is shared across wallets by design, but
    /// `released` is not: upstream computes it per wallet
    /// (`per_wallet_released_outpoints`), so this wallet's set says nothing
    /// about an input a *different* wallet's coin claims on the same row.
    /// The input decisions below are scoped to the inputs this wallet
    /// actually owns; the physical row delete at the bottom is housekeeping
    /// only now (see above) and runs once no other wallet's claim is still
    /// attached to it. See the ownership check below for how "no other
    /// wallet" is decided without an explicit cross-wallet coordination
    /// point.
    ///
    /// Throws if SwiftData cannot answer the lookup. The caller fails the
    /// round on that: a deletion silently skipped would let Rust clear the
    /// sweep while the dead row survives.
    private func applySweptTransaction(
        walletId: Data,
        txid: Data,
        supersededBy: Data,
        released: Set<Data>
    ) throws {
        var descriptor = FetchDescriptor<PersistentTransaction>(
            predicate: #Predicate { $0.txid == txid }
        )
        descriptor.fetchLimit = 1
        descriptor.relationshipKeyPathsForPrefetching = [\.outputs, \.inputs, \.pendingInputs]
        // A successful fetch that finds nothing skips only the row-scoped
        // work below, NOT the whole function. Sweeps are idempotent and can
        // name a transaction this store never had — but they can also name
        // one this store DID have and another wallet's callback already
        // deleted. The row is shared; the detached tombstones this wallet
        // wrote against it are not, and they are exactly the state that is
        // still findable — by scalar `spendingTxid` — after the row is gone.
        // Returning here would strand them: this wallet's release decision
        // would never reach a tombstone that then marks its coin spent by a
        // transaction that no longer exists, and a held one could never
        // follow the chain to a further winner. So the wallet-scoped
        // tombstone reconciliation at the bottom runs either way.
        let row = try backgroundContext.fetch(descriptor).first

        if let row {
            // The global half, done every time this function runs regardless
            // of which wallet's callback it is or whether this row has been
            // seen by a sweep before: delete the outputs this row created
            // (they are nobody's coin, ever — a swept transaction cannot have
            // funded anything) and mark the row excluded from restoration.
            // Both are idempotent, so re-processing an already-flagged row (a
            // second wallet's callback, or a re-emitted sweep) is a harmless
            // no-op.
            for output in row.outputs {
                backgroundContext.delete(output)
            }
            row.isGloballySwept = true

            // `released` is only ever true of the wallet that computed it, so
            // an input this wallet does not own must be left exactly as it is
            // — that wallet's own callback (delivered earlier, arriving
            // later, or never coming at all) is the only thing allowed to
            // decide it. Resolved through `resolvedWalletId(of:)` rather than
            // a raw `walletId` compare, same reasoning as `loadWalletList`:
            // the denormalized column reads empty on a row migrated before it
            // existed, and comparing it raw would make every such coin look
            // unowned and leave it untouched forever.
            for txo in row.inputs where Self.resolvedWalletId(of: txo) == walletId {
                txo.isSpent = !released.contains(txo.outpoint)
                txo.spendingTransaction = nil
                txo.lastUpdated = Date()
            }
            for pending in row.pendingInputs where pending.walletId == walletId {
                guard !released.contains(pending.outpoint) else {
                    // Deleted now rather than left for the row's cascade.
                    // Still attached it reads as this wallet's claim in the
                    // ownership check below, so a shared loser holding one
                    // released input per wallet deadlocks: each callback
                    // sees the other's row and declines the delete, and
                    // replaying either reaches the same stalemate. The
                    // global marker keeps the dead transaction from
                    // contributing funds regardless, but the row and both
                    // pending entries would otherwise be stored forever.
                    backgroundContext.delete(pending)
                    continue
                }
                pending.spendingTransaction = nil
                pending.spendingTxid = supersededBy
                pending.isSweptTombstone = true
            }

            // Whatever is still attached to `row` after the scoping above
            // belongs to a different wallet that has not weighed in yet —
            // this wallet's own rows are all resolved by now, held ones
            // detached and released ones deleted. Whichever callback finds nothing
            // left over is the last one to run and performs the delete, so
            // order stops mattering. A wallet whose callback never arrives at
            // all just leaves the row behind with every other wallet's inputs
            // already correctly decided — a leaked dead row, not a
            // wrongly-spent coin, and a re-emitted sweep cleans it up.
            //
            // Nothing below is load-bearing for correctness anymore: `row`
            // has no outputs and reads as `isGloballySwept` as of the block
            // above, in every callback that reaches this point, regardless of
            // whether this delete ever fires. This is reclaiming the
            // now-inert row's storage, not finishing the sweep. Detached
            // tombstones deliberately do not count as claims here — they no
            // longer need the row (the scalar reconciliation below never
            // touches it), so holding the delete for them would leak the row
            // for nothing. Nor do this wallet's released pending inputs:
            // they were deleted outright above precisely so they cannot
            // stalemate another wallet's callback.
            let otherWalletStillClaims = row.inputs.contains { txo in
                txo.spendingTransaction != nil && Self.resolvedWalletId(of: txo) != walletId
            } || row.pendingInputs.contains { pending in
                pending.spendingTransaction != nil && pending.walletId != walletId
            }
            if !otherWalletStillClaims {
                backgroundContext.delete(row)
            }
        }

        // Chained-sweep continuation: a pending row an EARLIER sweep already
        // tombstoned to `txid` (this transaction, itself a sweep's winner
        // until now) is no longer reachable through `row.pendingInputs` —
        // see the doc comment above. Find it by the scalar `spendingTxid`
        // it carries instead, scoped to this wallet for the same reason the
        // live pending inputs above were: the tombstone names one specific
        // wallet's coin, and only that wallet's own released set is the
        // right authority to re-decide it.
        //
        // Deliberately outside the `if let row` above. A tombstone's very
        // existence means `resolveInputOutpoint` declined to re-attach a
        // pending row when the winner's own record arrived (the duplicate
        // guard matches on `(outpoint, spendingTxid)` and a tombstone
        // occupies that key), so a wallet-relevant winner can carry no
        // attached claim of this wallet's at all — and another wallet's
        // callback, seeing nothing attached, legitimately deletes the shared
        // row before this wallet's callback ever runs. The tombstones are
        // this wallet's private state; the row's fate says nothing about
        // whether they still need their release applied or their chain
        // continued.
        var tombstoneDescriptor = FetchDescriptor<PersistentPendingInput>(
            predicate: #Predicate {
                $0.spendingTxid == txid && $0.isSweptTombstone == true && $0.walletId == walletId
            }
        )
        tombstoneDescriptor.includePendingChanges = true
        let priorTombstones = try backgroundContext.fetch(tombstoneDescriptor)
        for pending in priorTombstones {
            if released.contains(pending.outpoint) {
                backgroundContext.delete(pending)
            } else {
                pending.spendingTxid = supersededBy
            }
        }
    }

    /// Find or create the `PersistentWallet` row for `walletId`.
    /// Used only by `persistWalletMetadata`; every other write path
    /// fetches via `findWalletRecord` and drops on missing so that
    /// stale post-deletion callbacks can't resurrect a wiped wallet.
    private func ensureWalletRecord(walletId: Data) -> PersistentWallet {
        let descriptor = FetchDescriptor<PersistentWallet>(
            predicate: walletRecordPredicate(walletId: walletId)
        )
        if let existing = try? backgroundContext.fetch(descriptor).first {
            return existing
        }
        let record = PersistentWallet(walletId: walletId, network: self.network)
        backgroundContext.insert(record)
        return record
    }

    /// Find the `PersistentWallet` row for `walletId`. Returns `nil`
    /// when no row exists.
    private func findWalletRecord(walletId: Data) -> PersistentWallet? {
        try? fetchWalletRecord(walletId: walletId)
    }

    /// Throwing form of `findWalletRecord`, for callers that must tell a
    /// successful "no such wallet" apart from a failed lookup — anything
    /// carrying a subtractive change, where swallowing the failure would
    /// report a removal durable that never happened.
    private func fetchWalletRecord(walletId: Data) throws -> PersistentWallet? {
        let descriptor = FetchDescriptor<PersistentWallet>(
            predicate: walletRecordPredicate(walletId: walletId)
        )
        return try backgroundContext.fetch(descriptor).first
    }

    /// Predicate matching the `PersistentWallet` row owned by THIS
    /// handler. A handler is constructed per-network, so when
    /// `self.network` is set we scope to `(walletId, networkRaw)` —
    /// otherwise the mainnet handler would find and overwrite the
    /// devnet row (and vice versa) now that the same `walletId` can
    /// have one row per network. When `self.network` is `nil` (the
    /// advanced `configure(sdkPointer:network:nil)` path) we fall
    /// back to walletId-only matching to preserve that behaviour.
    private func walletRecordPredicate(walletId: Data) -> Predicate<PersistentWallet> {
        if let network = self.network {
            let networkRaw = network.rawValue
            return #Predicate { $0.walletId == walletId && $0.networkRaw == networkRaw }
        }
        return #Predicate { $0.walletId == walletId }
    }

    /// Look up a `PersistentWallet` to hang on
    /// `PersistentIdentity.wallet`. Non-creating — returns `nil` if
    /// no row exists (an identity may arrive before its owning
    /// wallet row under weird restore orderings) or if the caller
    /// passed `nil`. Kept separate from `ensureWalletRecord` so a
    /// stray identity upsert never creates a placeholder wallet.
    private func fetchWalletForLink(walletId: Data?) -> PersistentWallet? {
        guard let walletId else { return nil }
        let descriptor = FetchDescriptor<PersistentWallet>(
            predicate: walletRecordPredicate(walletId: walletId)
        )
        return try? backgroundContext.fetch(descriptor).first
    }

    /// Apply a single account changeset to SwiftData.
    private func applyAccountChangeset(
        walletRecord: PersistentWallet,
        acc: AccountChangeSetFFI
    ) {
        let accountIndex = acc.account_index
        // Stable account-type discriminants from the FFI. Used as the
        // upsert key so a load-path emit and a sync-path emit for the
        // same account collapse onto a single row — the legacy
        // `account_type_name` string was Rust's `Debug` output, which
        // differs from the canonical name the load path emits ("BIP44
        // Account" vs "Standard { index: 0, … }") and made the
        // string-keyed predicate produce duplicate rows.
        // `AccountTypeTagFFI` / `StandardAccountTypeTagFFI` come over
        // as plain `UInt8` aliases (cbindgen flat-enum projection).
        let typeTag = UInt32(acc.type_tag)
        let standardTag = UInt8(acc.standard_tag)
        let registrationIndex = acc.registration_index
        let keyClass = acc.key_class
        let userIdentityId = withUnsafeBytes(of: acc.user_identity_id) { Data($0) }
        let friendIdentityId = withUnsafeBytes(of: acc.friend_identity_id) { Data($0) }
        let typeName = accountTypeName(for: acc.type_tag, standardTag: acc.standard_tag)

        // Upsert keyed by the full account identity. We can't easily
        // express the identity tuple in a #Predicate with local `Data`
        // captures, so fetch by (walletId, accountType, accountIndex)
        // and verify the richer fields in Swift — same pattern the
        // load path uses for `applyAccountSpec`.
        let walletId = walletRecord.walletId
        let accountDescriptor = FetchDescriptor<PersistentAccount>(
            predicate: #Predicate {
                $0.wallet.walletId == walletId
                    && $0.accountType == typeTag
                    && $0.accountIndex == accountIndex
            }
        )
        let existing = (try? backgroundContext.fetch(accountDescriptor)) ?? []
        let match = existing.first { row in
            row.standardTag == standardTag
                && row.registrationIndex == registrationIndex
                && row.keyClass == keyClass
                && row.userIdentityId == userIdentityId
                && row.friendIdentityId == friendIdentityId
        }
        let account: PersistentAccount
        if let match = match {
            account = match
            account.lastUpdated = Date()
        } else {
            account = PersistentAccount(
                wallet: walletRecord,
                accountType: typeTag,
                accountIndex: accountIndex,
                accountTypeName: typeName
            )
            backgroundContext.insert(account)
        }
        // Refresh the variant-specific fields so the row stays in
        // sync with the latest emit (matches the load-path apply).
        account.standardTag = standardTag
        account.registrationIndex = registrationIndex
        account.keyClass = keyClass
        account.userIdentityId = userIdentityId
        account.friendIdentityId = friendIdentityId

        // Highest-used address pool indices.
        if acc.has_external_highest_used {
            account.externalHighestUsed = acc.external_highest_used
        }
        if acc.has_internal_highest_used {
            account.internalHighestUsed = acc.internal_highest_used
        }

        // Transactions.
        if acc.transactions_count > 0, let txsPtr = acc.transactions {
            for i in 0..<Int(acc.transactions_count) {
                upsertTransaction(account: account, tx: txsPtr[i])
            }
        }

        // UTXOs added.
        if acc.utxos_added_count > 0, let utxosPtr = acc.utxos_added {
            for i in 0..<Int(acc.utxos_added_count) {
                upsertUtxo(account: account, utxo: utxosPtr[i])
            }
        }

        // UTXOs spent — mark them spent (keep for history).
        if acc.utxos_spent_count > 0, let spentPtr = acc.utxos_spent {
            for i in 0..<Int(acc.utxos_spent_count) {
                markUtxoSpent(spentPtr[i])
            }
        }

        // UTXOs became InstantSend-locked — update flag.
        if acc.utxos_instant_locked_count > 0, let ilPtr = acc.utxos_instant_locked {
            for i in 0..<Int(acc.utxos_instant_locked_count) {
                markUtxoInstantLocked(ilPtr[i])
            }
        }
    }

    // MARK: - Round-indexed lookups
    //
    // The helpers below are the only way the changeset hot path
    // (`upsertTransaction`, `upsertUtxo`, `resolveInputOutpoint`,
    // `markUtxoSpent`, `markUtxoInstantLocked`, `removePendingInputs`,
    // `persistAccountAddresses`) resolves rows by key. Each one reads
    // `roundIndex` first, and on a miss — only while the index is
    // active — fetches with `includePendingChanges = false` so the store
    // lookup stays on SQLite's indexes instead of scanning the round's
    // pending inserts in memory (see `roundIndex`); a store hit is
    // registered in the index so the same key never fetches twice in one
    // round (the store-only refetch would refresh the object and discard
    // the round's unsaved mutations — see `roundIndex`). A miss on both
    // sources may re-fetch on a later call, which is safe: there is no
    // registered object for the refresh to clobber. With no active index
    // the helpers degrade to the plain default fetch. Predicates only
    // name immutable key columns (`txid`, `outpoint`, `address` are
    // fixed at insert), so matching on store values instead of in-memory
    // values cannot miss an in-round mutation; mutable-column filters
    // (`spendingTxid` on pending rows) stay in Swift at the call sites,
    // on live values. `isDeleted` is filtered on both sources because a
    // store-only fetch still returns rows whose delete is staged but
    // unsaved.
    //
    // `applySweptTransaction` stays on plain pending-changes fetches: its
    // row fetch needs relationship prefetching, and its tombstone fetch
    // keys on columns that MUTATE mid-round (`spendingTxid`,
    // `isSweptTombstone`), which neither the index nor a store-only
    // fetch can answer. Sweeps only target unconfirmed conflicts, so
    // that path stays off the initial-scan hot loop. It also mutates
    // TXO / pending rows through `row.inputs` / `row.pendingInputs`
    // without any keyed lookup the index could observe — which is safe
    // only because sweeps are applied LAST in `persistWalletChangeset`,
    // so no store-only first-touch fetch can follow those mutations
    // within the round and refresh them away.

    /// Resolve a `PersistentTransaction` by its unique `txid`.
    private func fetchTransactionRow(txid: Data) -> PersistentTransaction? {
        if let known = roundIndex?.transactionsByTxid[txid] {
            return known.isDeleted ? nil : known
        }
        var descriptor = FetchDescriptor<PersistentTransaction>(
            predicate: #Predicate { $0.txid == txid }
        )
        descriptor.fetchLimit = 1
        if roundIndex != nil { descriptor.includePendingChanges = false }
        guard let row = (try? backgroundContext.fetch(descriptor))?.first,
              !row.isDeleted else { return nil }
        roundIndex?.transactionsByTxid[txid] = row
        return row
    }

    /// Resolve a `PersistentTxo` by its unique 36-byte `outpoint`.
    private func fetchTxoRow(outpoint: Data) -> PersistentTxo? {
        if let known = roundIndex?.txosByOutpoint[outpoint] {
            return known.isDeleted ? nil : known
        }
        var descriptor = FetchDescriptor<PersistentTxo>(
            predicate: #Predicate { $0.outpoint == outpoint }
        )
        descriptor.fetchLimit = 1
        if roundIndex != nil { descriptor.includePendingChanges = false }
        guard let row = (try? backgroundContext.fetch(descriptor))?.first,
              !row.isDeleted else { return nil }
        roundIndex?.txosByOutpoint[outpoint] = row
        return row
    }

    /// Every live `PersistentPendingInput` row keyed on `outpoint` —
    /// saved rows plus this round's staged inserts. Non-unique key, so
    /// this returns the full set; callers filter further (by
    /// `spendingTxid`, `createdAt`) on the live objects. Saved rows are
    /// re-fetched store-only on every call rather than registered: no
    /// path mutates a pending row's attributes before the sweep pass,
    /// and sweeps run last (see the MARK comment), so the refetch
    /// refresh never has unsaved changes to discard — deletions, the
    /// one staged state these rows do accumulate mid-round, survive it.
    /// De-duped by object identity as insurance against a save landing
    /// mid-round (which would make a staged row visible to the store
    /// fetch too).
    private func pendingInputRows(outpoint: Data) -> [PersistentPendingInput] {
        var descriptor = FetchDescriptor<PersistentPendingInput>(
            predicate: #Predicate { $0.outpoint == outpoint }
        )
        if roundIndex != nil { descriptor.includePendingChanges = false }
        var rows = (try? backgroundContext.fetch(descriptor)) ?? []
        if let staged = roundIndex?.pendingInputsByOutpoint[outpoint] {
            let seen = Set(rows.map { ObjectIdentifier($0) })
            rows.append(contentsOf: staged.filter { !seen.contains(ObjectIdentifier($0)) })
        }
        return rows.filter { !$0.isDeleted }
    }

    /// Resolve a `PersistentCoreAddress` by its unique `address`.
    private func coreAddressRow(address: String) -> PersistentCoreAddress? {
        if let known = roundIndex?.coreAddressesByAddress[address] {
            return known.isDeleted ? nil : known
        }
        var descriptor = FetchDescriptor<PersistentCoreAddress>(
            predicate: #Predicate { $0.address == address }
        )
        descriptor.fetchLimit = 1
        if roundIndex != nil { descriptor.includePendingChanges = false }
        guard let row = (try? backgroundContext.fetch(descriptor))?.first,
              !row.isDeleted else { return nil }
        roundIndex?.coreAddressesByAddress[address] = row
        return row
    }

    private func upsertTransaction(account: PersistentAccount, tx: TransactionRecordFFI) {
        // The `account` parameter scopes the wallet-id used for the
        // input-reconciliation pass at the bottom of this method, and
        // records this account's participation in the tx via the
        // `involvedAccounts` join appended below.
        //
        // The transaction row's *funds* stay account-agnostic — a
        // single tx can land in multiple accounts (or wallets), and
        // per-wallet fund membership is recovered through the TXO
        // graph (`outputs` / `inputs`) rather than a denormalized
        // column. But this handler is invoked once per matched account
        // (the Rust changeset buckets `cs.records` by
        // `record.account_type`), including for payload-only matches —
        // a special-tx payload matching this account's provider owner /
        // voting key address with no TXO in the account. The TXO join
        // is blind to those, so we append `account` to
        // `record.involvedAccounts` to keep the involvement
        // representable at all.
        //
        let resolvedWalletId: Data = account.wallet.walletId
        let txidData = hashData(tx.txid)

        // The FFI projection always serializes the transaction body
        // (`dashcore::consensus::encode::serialize` upstream), so
        // `tx.tx_data` is non-null and `tx.tx_data_len > 0` in
        // practice. Fall back to empty `Data()` only as a defensive
        // guard against a future projection change — the
        // persister-fallback read path treats empty bytes as miss
        // (the Rust side can't decode an empty consensus buffer).
        let transactionData: Data = {
            guard let dataPtr = tx.tx_data, tx.tx_data_len > 0 else { return Data() }
            return Data(bytes: dataPtr, count: Int(tx.tx_data_len))
        }()

        // The FFI only carries a real `first_seen` once the tx is in a
        // block (it surfaces the block timestamp); mempool / instant-send
        // sightings arrive with 0 and delegate the observation stamp to
        // this side. Stamp the wall clock for those so a fresh send
        // doesn't sit at the epoch until it's mined.
        let firstSeen: UInt64 =
            tx.first_seen != 0 ? tx.first_seen : UInt64(Date().timeIntervalSince1970)

        let existing = fetchTransactionRow(txid: txidData)
        // A sweep is upstream's word at the moment it fired, but the
        // wallet's sweep state is not monotonic: `CoreChangeSet::merge`
        // documents the exact reachable sequence — an unconfirmed
        // transaction swept by an IS-locked conflict can return
        // chainlocked and sweep that conflict in turn, per key-wallet's
        // own IS-lock precedence rules. When both events land in the same
        // changeset the merge already strips the sweep before it gets
        // here. Across separate rounds it can't: the earlier sweep is
        // already durable (row tombstoned, possibly still physically
        // present because another wallet's claim held the delete back —
        // see `applySweptTransaction`), and this later record is the only
        // signal this callback ever sees that the wallet reversed itself.
        // Upstream never re-emits a live record for a txid it still
        // considers dead, so a record naming an `isGloballySwept` txid is
        // authoritative reinstatement, not a stale replay — treat it as
        // upstream's newer word and let it win: clear the tombstone and
        // fall through to the ordinary upsert below.
        //
        // What this does and does not restore: `context`/`blockHeight`,
        // `involvedAccounts` membership, and this record's own input
        // reconciliation all rebuild normally from here since they're
        // driven straight off `tx` and `account`. The outputs
        // `applySweptTransaction` physically deleted are a different
        // story — they come back only if this round (or the one
        // `upsertUtxo` processes moments later, before any other sweep
        // callback can re-tombstone this row) also carries fresh
        // `utxos_added` entries for them, the same way any transaction's
        // outputs ordinarily arrive alongside its record. That is not
        // this method's call to make: if Rust doesn't re-emit them, they
        // cannot be reconstructed here from nothing.
        if let existing, existing.isGloballySwept {
            existing.isGloballySwept = false
        }

        let record: PersistentTransaction
        if let existing {
            record = existing
        } else {
            record = PersistentTransaction(
                txid: txidData,
                transactionData: transactionData,
                context: tx.context,
                blockHeight: tx.block_height,
                direction: tx.direction,
                transactionType: tx.transaction_type.map { String(cString: $0) } ?? "Standard",
                netAmount: tx.net_amount,
                firstSeen: firstSeen
            )
            backgroundContext.insert(record)
            roundIndex?.transactionsByTxid[txidData] = record
        }

        record.context = tx.context
        record.blockHeight = tx.block_height
        record.blockTimestamp = tx.block_timestamp
        record.blockPosition = tx.block_position
        record.hasBlockPosition = tx.has_block_position
        let blockHashBytes = hashData(tx.block_hash)
        record.blockHash = blockHashBytes.allSatisfy { $0 == 0 } ? nil : blockHashBytes
        record.direction = tx.direction
        if let typeName = tx.transaction_type {
            record.transactionType = String(cString: typeName)
        }
        record.transactionTypeKind = tx.transaction_type_kind
        // Provider (masternode) payload — parsed on the Rust side from
        // the DIP-3 special-tx body; marshal the flat fields straight
        // onto the row (null string / `has_* == false` ⇒ nil).
        record.providerServiceAddress = tx.provider_service_address.map { String(cString: $0) }
        record.providerProTxHash = tx.has_provider_pro_tx_hash
            ? withUnsafeBytes(of: tx.provider_pro_tx_hash) { Data($0) }
            : nil
        record.providerCollateralTxid = tx.has_provider_collateral
            ? withUnsafeBytes(of: tx.provider_collateral_txid) { Data($0) }
            : nil
        record.providerCollateralVout = tx.has_provider_collateral ? tx.provider_collateral_vout : 0
        record.providerOwnerKeyHash = tx.has_provider_owner_key_hash
            ? withUnsafeBytes(of: tx.provider_owner_key_hash) { Data($0) }
            : nil
        record.providerVotingKeyHash = tx.has_provider_voting_key_hash
            ? withUnsafeBytes(of: tx.provider_voting_key_hash) { Data($0) }
            : nil
        record.netAmount = tx.net_amount
        record.fee = tx.has_fee ? tx.fee : nil
        if let labelPtr = tx.label {
            record.label = String(cString: labelPtr)
        }
        // Once mined, `tx.first_seen` carries the block timestamp —
        // adopt it. While still unconfirmed it stays 0; keep whatever
        // stamp the row already has rather than zeroing it, and stamp
        // rows that never got one (placeholder rows from `upsertUtxo`,
        // rows persisted before insert-time stamping existed).
        if tx.first_seen != 0 || record.firstSeen == 0 {
            record.firstSeen = firstSeen
        }
        record.transactionData = transactionData
        record.lastUpdated = Date()

        // Record this account's participation in the tx. Idempotent:
        // SPV re-upserts the same (account, tx) pair on every touch, so
        // append only when the account isn't already linked. Compare by
        // `persistentModelID` — object identity isn't stable across
        // fetches within a context, but the model id is. This is the
        // sole carrier of payload-only involvement (no TXO in the
        // account); for ordinary funded txs it harmlessly duplicates
        // the TXO-derived membership, which the per-account union
        // de-dups.
        if !record.involvedAccounts.contains(where: {
            $0.persistentModelID == account.persistentModelID
        }) {
            record.involvedAccounts.append(account)
        }

        // Walk every input in this transaction and reconcile it
        // against the `PersistentTxo` table. The FFI populates
        // `input_outpoints` from `tx.input.iter()` directly, so the
        // list survives even when the wallet's in-memory `self.utxos`
        // didn't classify the input as ours at processing time —
        // that gap was the silent-drop path that left
        // `PersistentTxo.isSpent` stuck at false on out-of-order
        // arrival. For each input outpoint:
        //   1. Look up the matching `PersistentTxo`.
        //   2. If found → set `isSpent` and link `spendingTransaction`.
        //   3. If not found → write a `PersistentPendingInput` row;
        //      the matching `upsertUtxo` will pick it up later and
        //      delete the pending row in the same pass.
        if let inPtr = tx.input_outpoints, tx.input_outpoints_count > 0 {
            for i in 0..<Int(tx.input_outpoints_count) {
                let entry = inPtr[i]
                let prevTxid = withUnsafeBytes(of: entry.txid) { Data($0) }
                let outpoint = PersistentTxo.makeOutpoint(txid: prevTxid, vout: entry.vout)
                resolveInputOutpoint(
                    outpoint: outpoint,
                    inputIndex: UInt32(i),
                    spendingTransaction: record,
                    spendingTxid: txidData,
                    walletId: resolvedWalletId
                )
            }
        }
    }

    /// `true` if the spending tx has reached a confirmed context. Used
    /// to gate `isSpent` writes so a mempool-sighting alone — which is
    /// reversible by RBF or mempool eviction — doesn't permanently flip
    /// the input TXO out of the unspent set. The TXO becomes truly spent
    /// only when the spending tx lands in a block; until then the
    /// persisted state reflects "still spendable from this row's POV",
    /// and the catch-up classifier on the next launch reloads the
    /// row and recognises it as ours when the block arrives.
    private static func spendIsInBlock(_ tx: PersistentTransaction) -> Bool {
        tx.context >= TransactionContextType.inBlock.rawValue
    }

    /// Mark the `PersistentTxo` whose 36-byte `outpoint` matches the
    /// given input as spent and link it to `spendingTransaction`.
    /// If no matching TXO exists yet (in-Swift out-of-order, or
    /// load_from_persistor missed it), write a
    /// `PersistentPendingInput` row so the next `upsertUtxo` for
    /// that outpoint can resolve the linkage.
    private func resolveInputOutpoint(
        outpoint: Data,
        inputIndex: UInt32,
        spendingTransaction: PersistentTransaction,
        spendingTxid: Data,
        walletId: Data
    ) {
        if let txo = fetchTxoRow(outpoint: outpoint) {
            // `isSpent` only flips once the spending tx is in a block
            // (see `spendIsInBlock`'s doc) — a mempool sighting
            // alone links the spending relationship but keeps the
            // row in the unspent set so a `restartWalletManager()`
            // load can hand the TXO back to Rust for the post-restart
            // catch-up classifier to recognise as ours. The next
            // upsert of this same tx with a confirmed context flips
            // `isSpent` then.
            let expectedIsSpent = Self.spendIsInBlock(spendingTransaction)
            let linkageChanged =
                txo.isSpent != expectedIsSpent
                || txo.spendingTransaction?.txid != spendingTxid
                || txo.spendingInputIndex != inputIndex
            if linkageChanged {
                txo.isSpent = expectedIsSpent
                if txo.spendingTransaction?.txid != spendingTxid {
                    txo.spendingTransaction = spendingTransaction
                }
                // Capture the canonical vin index so the detail
                // view can render inputs in serialized order.
                txo.spendingInputIndex = inputIndex
                txo.lastUpdated = Date()
            }
            // A pending entry from an earlier write is now stale —
            // resolved by this fetch. Drop it.
            removePendingInputs(for: outpoint)
        } else {
            // Defer: record a pending row so a future `upsertUtxo`
            // can complete the link. Writing one row per input is
            // cheap; the cascade-delete relationship + the resolve
            // path in `upsertUtxo` keep the table from growing
            // unbounded.
            //
            // Skip the write if a pending row for this exact
            // (outpoint, spending-tx) pair already exists — re-upserts
            // of the same transaction would otherwise produce
            // duplicate pending rows that all resolve to the same
            // TXO, wasting fetch work on the resolve side. The
            // `spendingTxid` half of the pair is compared in Swift on
            // the live rows (it is mutable — `applySweptTransaction`
            // rewrites it on tombstones — so it can't be a store-side
            // predicate under the round index's store-only fetch).
            let alreadyPending = pendingInputRows(outpoint: outpoint)
                .contains { $0.spendingTxid == spendingTxid }
            if !alreadyPending {
                let pending = PersistentPendingInput(
                    outpoint: outpoint,
                    inputIndex: inputIndex,
                    spendingTxid: spendingTxid,
                    spendingTransaction: spendingTransaction,
                    walletId: walletId
                )
                backgroundContext.insert(pending)
                roundIndex?.pendingInputsByOutpoint[outpoint, default: []].append(pending)
            }
        }
    }

    /// Drop every `PersistentPendingInput` row keyed on `outpoint`.
    /// Called after a successful `PersistentTxo` mark-spent so the
    /// pending entries don't linger as orphans, and from
    /// `upsertUtxo`'s resolve path so a freshly-arrived TXO doesn't
    /// keep its corresponding pending row alive.
    private func removePendingInputs(for outpoint: Data) {
        // Deletes are not unregistered from `roundIndex` — the stale
        // entry answers `isDeleted == true` and every lookup filters on
        // that (see the index's doc).
        for row in pendingInputRows(outpoint: outpoint) {
            backgroundContext.delete(row)
        }
    }

    private func upsertUtxo(account: PersistentAccount, utxo: UtxoEntryFFI) {
        // Pull the per-account wallet id once. Used both for the new
        // `PersistentTxo.walletId` denorm (so per-wallet predicates
        // can hit a single column) and for stub-tx routing below.
        let resolvedWalletId: Data = account.wallet.walletId

        let txidData = hashData(utxo.outpoint.txid)
        let outpoint = PersistentTxo.makeOutpoint(txid: txidData, vout: utxo.outpoint.vout)
        let record: PersistentTxo
        if let existing = fetchTxoRow(outpoint: outpoint) {
            record = existing
            // Backfill if the account or wallet linkage is missing —
            // the per-wallet query path filters on TXO.walletId, so
            // an empty value would silently hide the row.
            if record.account == nil { record.account = account }
            if record.walletId.isEmpty, !resolvedWalletId.isEmpty {
                record.walletId = resolvedWalletId
            }
        } else {
            // Look up the containing transaction. Upstream sends the
            // transaction record before its TXOs in the same flush,
            // so it should already be in the context. If not, create
            // a stub keyed by txid so the cascade-delete invariant
            // (TXO cannot exist without its creating transaction)
            // holds; the real record will overwrite the stub when it
            // arrives. Note we no longer set `parentTx.account` —
            // transactions don't carry account linkage anymore (they
            // can span multiple accounts).
            let parentTx: PersistentTransaction
            if let existingTx = fetchTransactionRow(txid: txidData) {
                // A globally-swept parent is a transaction Rust has already
                // proven can never confirm — a fresh UTXO entry naming its
                // txid would (re-)create exactly the phantom output
                // `applySweptTransaction` deletes on every callback that
                // observes the sweep. Bail rather than attach a new
                // `PersistentTxo` to a row still excluded from restoration.
                //
                // This does not fight `upsertTransaction`'s reinstatement
                // path — it relies on it running first. `applyAccountChangeset`
                // processes an account's `tx.transactions` before its
                // `utxos_added`, so a reinstating record for this same txid
                // in this same round has already cleared the tombstone by
                // the time this guard reads it here; only a UTXO entry with
                // no accompanying record this round (or in a stray one that
                // arrives out of order relative to it) still finds the flag
                // set. That is genuinely a stale/out-of-order signal — Rust
                // does not otherwise re-emit a swept loser's own outputs —
                // and staying defensive here is correct: there is no record
                // in flight to attribute a resurrected output to.
                guard !existingTx.isGloballySwept else { return }
                parentTx = existingTx
            } else {
                // Stub row — `transactionData` is left as empty
                // `Data()` on purpose. The real upsert (which has the
                // tx bytes) overwrites every field including
                // `transactionData` when it arrives. An orphaned
                // stub (real upsert never lands) reads back as empty
                // bytes, which the persister-fallback decode path
                // treats as miss.
                parentTx = PersistentTransaction(txid: txidData, transactionData: Data())
                backgroundContext.insert(parentTx)
                roundIndex?.transactionsByTxid[txidData] = parentTx
            }

            let script: Data = {
                guard let p = utxo.script_pubkey, utxo.script_pubkey_len > 0 else { return Data() }
                return Data(bytes: p, count: Int(utxo.script_pubkey_len))
            }()
            let addressStr = utxo.address.map { String(cString: $0) } ?? ""
            record = PersistentTxo(
                transaction: parentTx,
                vout: utxo.outpoint.vout,
                amount: utxo.amount,
                address: addressStr,
                scriptPubKey: script,
                height: utxo.height
            )
            record.account = account
            record.walletId = resolvedWalletId
            backgroundContext.insert(record)
            roundIndex?.txosByOutpoint[outpoint] = record
        }

        record.amount = utxo.amount
        record.height = utxo.height
        record.isCoinbase = utxo.is_coinbase
        record.isConfirmed = utxo.is_confirmed
        record.isInstantLocked = utxo.is_instantlocked
        record.isLocked = utxo.is_locked
        record.lastUpdated = Date()

        // The wallet is handing this outpoint over as a UTXO, so it holds it
        // unspent — authoritative, and the only thing that can lift a mark
        // left with no spender on record. `applySweptTransaction` parks the
        // inputs of a sweep it cannot resolve in exactly that state; a
        // rescan re-delivering the coin lands here and frees it. A row whose
        // spend is still on record is left alone: the pending-input resolve
        // below owns that transition. `supersededByTxid` is a different
        // kind of "no spender" — a sweep's winner is known but its row
        // never materialized here — and must not be lifted the same way,
        // or a tombstone the pending-resolve below just wrote would be
        // undone by the very next sync round that re-delivers this outpoint.
        if record.isSpent, record.spendingTransaction == nil, record.supersededByTxid == nil {
            record.isSpent = false
        }

        // Attach the `PersistentCoreAddress` row, if we have one. The
        // address-emit pass typically runs ahead of the SPV-utxo pass
        // within a flush, so the row should exist; if it doesn't (TXO
        // paid to an address outside our pool, or out-of-order flush),
        // leave the relationship nil — `record.address` stays as the
        // authoritative identifier.
        if record.coreAddress == nil, !record.address.isEmpty {
            if let coreAddr = coreAddressRow(address: record.address) {
                record.coreAddress = coreAddr
            }
        }

        // Resolve any deferred spend signal that landed before this
        // TXO existed. `upsertTransaction` writes a
        // `PersistentPendingInput` row for every input outpoint
        // whose previous-output isn't in SwiftData yet; the matching
        // upsert here drains those rows and stamps `isSpent` on the
        // TXO. Symmetric with the resolve path in
        // `upsertTransaction`, so the spend signal is order-
        // independent at this layer regardless of which side arrives
        // first.
        let pendingRows = pendingInputRows(outpoint: record.outpoint)
        if !pendingRows.isEmpty {
            // Pick the freshest pending entry — under normal sync
            // there's only one, but a chain reorg or double-spend
            // observation could leave multiple. Newest wins so the
            // visible spendingTransaction matches the most recent
            // observation; the rest are dropped.
            //
            // A tombstone outranks every ordinary row regardless of age.
            // Newest-wins arbitrates between competing *observations*, but a
            // tombstone is not an observation — it is the sweep's settled
            // verdict that its winner consumed this coin. The two coexist in
            // exactly one way: records precede sweeps within a round, so the
            // winner's own record can stage an ordinary pending row for this
            // outpoint moments before the sweep repoints the loser's row —
            // which keeps its original, older `createdAt`. Letting the
            // younger ordinary row win there would take the gated branch
            // below (`isSpent` false until the winner confirms), never stamp
            // `supersededByTxid`, and then delete every row including the
            // tombstone — the durable hold evaporates and the consumed coin
            // re-enters the restore set.
            let chosen = pendingRows.filter(\.isSweptTombstone)
                .max(by: { $0.createdAt < $1.createdAt })
                ?? pendingRows.max(by: { $0.createdAt < $1.createdAt })
                ?? pendingRows[0]

            // Resolve the spending tx (prefer the relationship; fall
            // back to a txid lookup if the row wasn't faulted in).
            // We need its `context` to gate `isSpent` — same rule as
            // `resolveInputOutpoint`: mempool sighting links the
            // spendingTransaction but doesn't flip `isSpent` until
            // the spending tx is in a block.
            let resolvedSpending: PersistentTransaction?
            if let spending = chosen.spendingTransaction {
                resolvedSpending = spending
                // Resolved through the relationship, not the index —
                // register it so a later `fetchTransactionRow` for this
                // txid returns this same object instead of running a
                // first-touch store fetch that would refresh away any
                // staged writes it carries (see `roundIndex`).
                roundIndex?.transactionsByTxid[spending.txid] = spending
            } else {
                resolvedSpending = fetchTransactionRow(txid: chosen.spendingTxid)
            }

            // Carry the vin index forward so the spending tx's
            // detail view can render its inputs in the canonical
            // serialized order. Same source as the linkage write
            // in `resolveInputOutpoint` — the only path that creates
            // pending rows captures the index from FFI's
            // `input_outpoints` slice, which mirrors `tx.input.iter()`.
            record.spendingInputIndex = chosen.inputIndex
            if let spending = resolvedSpending,
               record.spendingTransaction?.txid != spending.txid {
                record.spendingTransaction = spending
            }
            if chosen.isSweptTombstone {
                // `applySweptTransaction` repointed this row at the sweep's
                // winner because the loser it originally recorded is gone.
                // A sweep's winner is already final — there is no mempool
                // state to wait out — so `isSpent` does not gate on
                // `resolvedSpending` the way an ordinary pending spend does;
                // that lookup only succeeds when the winner happens to have
                // its own materialized row, which is not guaranteed.
                // `supersededByTxid` is what makes the mark durable either
                // way — it is what the recovery clear above checks so this
                // coin isn't handed back as spendable on a later sync.
                record.isSpent = true
                record.supersededByTxid = chosen.spendingTxid
            } else if let spending = resolvedSpending {
                record.isSpent = Self.spendIsInBlock(spending)
            }
            record.lastUpdated = Date()
            for row in pendingRows {
                backgroundContext.delete(row)
            }
        }
    }

    private func markUtxoSpent(_ entry: SpentOutPointFFI) {
        let outpoint = PersistentTxo.makeOutpoint(
            txid: hashData(entry.outpoint.txid),
            vout: entry.outpoint.vout
        )
        guard let txo = fetchTxoRow(outpoint: outpoint) else {
            return
        }
        // Link the spending transaction. The FFI now carries
        // `spending_txid` alongside the outpoint (the txid of the
        // `TransactionRecord` whose inputs included this outpoint),
        // so we can resolve the parent and set the relationship.
        // If the spending tx hasn't landed in SwiftData yet (rare
        // — same-flush ordering normally upserts the tx before
        // its spent-outpoint emit) leave the relationship nil; the
        // next flush carrying that tx triggers another upsert
        // round and eventually catches up.
        let spendingTxid = hashData(entry.spending_txid)
        var spendingTx: PersistentTransaction? = nil
        if !spendingTxid.isEmpty, !spendingTxid.allSatisfy({ $0 == 0 }) {
            if txo.spendingTransaction?.txid == spendingTxid {
                spendingTx = txo.spendingTransaction
            } else {
                spendingTx = fetchTransactionRow(txid: spendingTxid)
                if let spending = spendingTx {
                    txo.spendingTransaction = spending
                }
            }
        }
        // Gate the `isSpent` flip on the spending tx being in a
        // block — same rule as `resolveInputOutpoint`. When the
        // spending tx isn't resolved this flush, leave `isSpent`
        // alone instead of writing `false`: the next upsert round
        // carrying the spending tx will run `resolveInputOutpoint`
        // and set it then. Writing `false` here would flap a
        // previously-true `isSpent` on every reordered emit.
        if let spending = spendingTx {
            txo.isSpent = Self.spendIsInBlock(spending)
        }
        txo.lastUpdated = Date()
        // The spend signal landed both via the legacy
        // `utxos_spent` slice (this path) and — assuming the
        // spending tx's record was emitted in the same flush —
        // through `upsertTransaction`'s reconciliation pass. Both
        // resolve to the same TXO row, but the latter may have
        // written a `PersistentPendingInput` row when the TXO
        // didn't yet exist. Drain any leftover pending rows for
        // this outpoint so they don't linger as orphans.
        removePendingInputs(for: outpoint)
    }

    private func markUtxoInstantLocked(_ op: OutPointFFI) {
        let outpoint = PersistentTxo.makeOutpoint(txid: hashData(op.txid), vout: op.vout)
        if let txo = fetchTxoRow(outpoint: outpoint) {
            txo.isInstantLocked = true
            txo.lastUpdated = Date()
        }
    }

    private func addDelta(_ base: UInt64, _ delta: Int64) -> UInt64 {
        if delta >= 0 {
            return base.addingReportingOverflow(UInt64(delta)).0
        }
        let sub = UInt64(-delta)
        return base >= sub ? base - sub : 0
    }

    // MARK: - Callbacks

    /// Explicit semantic capability declaration passed alongside (not inside)
    /// the established callback vtable by the additive manager-create API.
    func makePersistenceCapabilities() -> PersistenceCapabilitiesFFI {
        PersistenceCapabilitiesFFI(
            version: PlatformWalletPersistenceCapabilities.version1,
            reserved: 0,
            bits: PlatformWalletPersistenceCapabilities.atomicChangesets
                | PlatformWalletPersistenceCapabilities.invitations
                | PlatformWalletPersistenceCapabilities.assetLockFundingIndices
                | PlatformWalletPersistenceCapabilities.shieldedViewingKeys
                | PlatformWalletPersistenceCapabilities.providerTransactions
                | PlatformWalletPersistenceCapabilities.unsignedTokenStorage
                | PlatformWalletPersistenceCapabilities.walletRestore
                | PlatformWalletPersistenceCapabilities.dpnsNameStates
                | PlatformWalletPersistenceCapabilities.trackedAssetLocks
                | PlatformWalletPersistenceCapabilities.coreSweepRemoval
        )
    }

    /// Additive callbacks live in their own size/version-tagged structure so
    /// Rust never reads beyond the established unsized callback vtable used by
    /// older hosts.
    func makePersistenceCallbacksExtension() -> PersistenceCallbacksExtension {
        var extensionCallbacks = PersistenceCallbacksExtension()
        extensionCallbacks.struct_size = UInt(MemoryLayout<PersistenceCallbacksExtension>.size)
        extensionCallbacks.version = UInt32(PLATFORM_WALLET_PERSISTENCE_CALLBACKS_EXTENSION_VERSION)
        extensionCallbacks.reserved = 0
        extensionCallbacks.on_persist_dpns_name_states_fn = persistDpnsNameStatesCallback
        // Sweeps negotiate through this size-tagged structure rather than
        // riding `WalletChangeSetFFI` because that struct crosses by bare
        // pointer: `struct_size` above is what proves to an older native
        // library that this slot exists, and proves to this build that an
        // older library will simply never call it — rather than either side
        // reading memory the other never allocated.
        extensionCallbacks.on_persist_wallet_changeset_sweeps_fn =
            persistWalletChangesetSweepsCallback
        return extensionCallbacks
    }

    /// Build `PersistenceCallbacks` that point to this handler.
    ///
    /// **Transfers ownership of a strong reference to Rust**: the context
    /// is `passRetained`, and `release_fn` balances that retain exactly
    /// once — when the Rust manager and every background worker holding
    /// its persister have dropped their references (possibly on a Rust
    /// thread, possibly after `destroy` returns if a worker straggles).
    /// ARC therefore cannot free this handler while any Rust worker can
    /// still call back into it, no matter how teardown went.
    ///
    /// If manager creation fails, Rust never took the reference — the
    /// caller must balance the retain itself (see `configure`).
    func makeCallbacks() -> PersistenceCallbacks {
        let contextPtr = Unmanaged.passRetained(self).toOpaque()
        var cb = PersistenceCallbacks()
        cb.context = contextPtr
        cb.release_fn = { context in
            guard let context else { return }
            Unmanaged<PlatformWalletPersistenceHandler>.fromOpaque(context).release()
        }
        cb.on_changeset_begin_fn = changesetBeginCallback
        cb.on_changeset_end_fn = changesetEndCallback
        cb.on_persist_address_balances_fn = persistAddressBalancesCallback
        cb.on_persist_wallet_changeset_fn = persistWalletChangesetCallback
        cb.on_persist_sync_state_fn = persistSyncStateCallback
        // `on_persist_wallet_root_xpub_fn` intentionally unassigned.
        // Root xpub is redundant with `wallet_id` for identity /
        // verification; Rust-side will stop requiring it once the
        // upstream rust-dashcore PR lands.
        cb.on_persist_account_registrations_fn = persistAccountRegistrationsCallback
        cb.on_load_wallet_list_fn = loadWalletListCallback
        cb.on_load_wallet_list_free_fn = loadWalletListFreeCallback
        cb.on_persist_wallet_metadata_fn = persistWalletMetadataCallback
        cb.on_persist_account_address_pools_fn = persistAccountAddressPoolsCallback
        cb.on_persist_identities_fn = persistIdentitiesCallback
        cb.on_persist_identity_keys_fn = persistIdentityKeysCallback
        cb.on_persist_token_balances_fn = persistTokenBalancesCallback
        cb.on_persist_contacts_fn = persistContactsCallback
        cb.on_persist_shielded_notes_fn = persistShieldedNotesCallback
        cb.on_persist_shielded_nullifiers_spent_fn = persistShieldedNullifiersSpentCallback
        cb.on_persist_shielded_outgoing_notes_fn = persistShieldedOutgoingNotesCallback
        cb.on_persist_shielded_synced_indices_fn = persistShieldedSyncedIndicesCallback
        cb.on_persist_shielded_activity_fn = persistShieldedActivityCallback
        cb.on_load_shielded_notes_fn = loadShieldedNotesCallback
        cb.on_load_shielded_notes_free_fn = loadShieldedNotesFreeCallback
        cb.on_load_shielded_outgoing_notes_fn = loadShieldedOutgoingNotesCallback
        cb.on_load_shielded_outgoing_notes_free_fn = loadShieldedOutgoingNotesFreeCallback
        cb.on_load_shielded_sync_states_fn = loadShieldedSyncStatesCallback
        cb.on_load_shielded_sync_states_free_fn = loadShieldedSyncStatesFreeCallback
        cb.on_load_shielded_activity_fn = loadShieldedActivityCallback
        cb.on_load_shielded_activity_free_fn = loadShieldedActivityFreeCallback
        cb.on_persist_shielded_viewing_keys_fn = persistShieldedViewingKeysCallback
        cb.on_load_shielded_viewing_keys_fn = loadShieldedViewingKeysCallback
        cb.on_load_shielded_viewing_keys_free_fn = loadShieldedViewingKeysFreeCallback
        cb.on_persist_asset_locks_fn = persistAssetLocksCallback
        cb.on_persist_invitations_fn = persistInvitationsCallback
        cb.on_get_core_tx_record_fn = getCoreTxRecordCallback
        cb.on_get_core_tx_record_free_fn = getCoreTxRecordFreeCallback
        cb.on_list_wallet_core_txids_fn = listWalletCoreTxidsCallback
        cb.on_list_wallet_core_txids_free_fn = listWalletCoreTxidsFreeCallback
        cb.on_persist_dashpay_payments_fn = persistDashpayPaymentsCallback
        return cb
    }

    // MARK: - Changeset atomicity

    /// Opens a persistence round. Paired with
    /// [`endChangeset(walletId:success:)`]. Every per-kind handler
    /// (`persistIdentities`, `persistIdentityKeys`,
    /// `persistAccountChangeset`, …) fires between begin and end and
    /// only mutates `backgroundContext`; `save()` happens at the end.
    ///
    /// Beyond the tag, this builds the round's insert index (see
    /// `roundIndex`) — `ModelContext`'s pending-change buffer already
    /// gives us the batching we need.
    func beginChangeset(walletId: Data) {
        onQueue {
            _ = walletId
            self.inChangeset = true
            // The index's O(1) lookups are only equivalent to the plain
            // pending-changes fetch when the index and the store
            // partition the rows between them: index = this round's
            // inserts, store = everything saved. A context that is
            // already dirty here (an out-of-round writer whose `save()`
            // threw and left its staged rows behind) breaks that
            // partition — such a row is in neither source — so the
            // round runs unindexed and the lookup helpers fall back to
            // the exact pre-index fetch, pending changes included.
            self.roundIndex = backgroundContext.hasChanges ? nil : ChangesetRoundIndex()
        }
    }

    /// Closes a persistence round. Commits all per-kind writes
    /// accumulated in `backgroundContext` since the matching
    /// `beginChangeset` in one `save()` (success path), or discards
    /// them via `rollback()` (failure path — any per-kind callback
    /// returned non-zero).
    ///
    /// One fsync per Rust `store()` round instead of one per per-
    /// kind callback, and the whole round is atomic from SwiftData's
    /// perspective: a crash between callbacks leaves the store in
    /// its pre-round state rather than half-applied.
    /// Returns `true` iff the round's staged writes were durably committed
    /// (`success && save()` succeeded). A `false` return — a per-kind failure,
    /// or a `save()` that threw and was rolled back — is forwarded to Rust via
    /// the C shim so `store()` reports a persistence failure instead of
    /// silently advancing its in-memory state (pending queues, cleared drain
    /// entries, ignored-sender deltas) against writes that never reached disk.
    @discardableResult
    func endChangeset(walletId: Data, success: Bool) -> Bool {
        onQueue {
            _ = walletId
            // Clear the flag before draining deferred backfills so each one's
            // save() lands cleanly outside the round; `drainDeferredBackfills`
            // is guarded on `!inChangeset`, so the ordering inside this `defer`
            // (clear, then drain) is load-bearing. The round index dies here
            // on both paths — after the commit its entries are ordinary saved
            // rows the store fetch finds on its own, and after a rollback the
            // context has un-inserted every one of them.
            defer {
                self.roundIndex = nil
                self.inChangeset = false
                self.drainDeferredBackfills()
            }
            if success {
                // Stage payment groups parked on a mid-round missing owner
                // BEFORE the round's single save — by now every identity
                // insert the round staged is visible to the fetch. A group
                // whose owner is STILL unresolvable fails the whole round:
                // committing the rest while dropping payment rows would
                // report success for a lossy persist, and Rust would keep
                // in-memory payment state that never reached disk — the
                // exact invariant `record_dashpay_payment`'s rollback
                // protects. No second save after the commit: the round
                // stays one atomic transaction.
                let parked = deferredPaymentUpserts
                deferredPaymentUpserts.removeAll()
                for entry in parked where !stageDashpayPaymentUpserts(
                    ownerIdentityId: entry.ownerIdentityId,
                    payments: entry.payments
                ) {
                    print(
                        "⚠️ endChangeset: no PersistentIdentity for owner "
                            + "\(entry.ownerIdentityId.prefix(8).toHexString())… after the "
                            + "round's identity applies; failing the round so Rust rolls "
                            + "back \(entry.payments.count) payment row(s) instead of "
                            + "losing them"
                    )
                    backgroundContext.rollback()
                    return false
                }
                do {
                    try backgroundContext.save()
                    return true
                } catch {
                    // The context still has the pending changes on
                    // its dirty list after a failed save; drop them so
                    // the next round starts clean. SQLite's WAL will
                    // only have committed data prior to this save, so
                    // the user-visible store is consistent — but the
                    // round did NOT commit, so report failure upward.
                    print("⚠️ endChangeset: save failed: \(error.localizedDescription)")
                    backgroundContext.rollback()
                    return false
                }
            } else {
                backgroundContext.rollback()
                // Parked rows from the failed round die with it: the Rust
                // side rolled its in-memory entries back too, so persisting
                // them later would fabricate history.
                deferredPaymentUpserts.removeAll()
                return false
            }
        }
    }

    /// Run any breadcrumb backfills that were parked while a changeset
    /// round was open. Must be called on `serialQueue` with `inChangeset`
    /// already cleared so each `backfillCore` mutates + saves cleanly on
    /// its own. Draining after the round's own `save()`/`rollback()` keeps
    /// the backfill's writes out of the round's transaction.
    private func drainDeferredBackfills() {
        guard !inChangeset, !deferredBackfills.isEmpty else { return }
        let pending = deferredBackfills
        deferredBackfills.removeAll()
        for request in pending {
            _ = backfillCore(walletId: request.walletId, items: request.items)
        }
    }

    // MARK: - Identity scalar persistence

    /// Upsert / remove rows from `PersistentIdentity` in response to
    /// an `IdentityChangeSet` forwarded by the Rust side.
    ///
    /// Mapping:
    /// - Each `upsert.identity_id` gets an upsert on
    ///   `PersistentIdentity` keyed by that unique column.
    /// - Each `removed` id drops the matching row.
    ///
    /// Public keys are written by `persistIdentityKeys` on a paired
    /// callback; this path only touches the identity row itself.
    /// Both callbacks run under the same Rust-side wallet lock so
    /// the two-step apply is atomic from Swift's perspective.
    ///
    /// Primary-identity selection and the gap-limit scan watermark
    /// were dropped from the Rust side — the former moved to the UI
    /// layer, the latter is now derived from
    /// `IdentityManager.highestRegistrationIndex(...)` at read time.
    func persistIdentities(
        walletId: Data,
        upserts: [IdentityEntrySnapshot],
        removed: [Data]
    ) {
        onQueue {
        for entry in upserts {
            let identityId = entry.identityId
            let descriptor = FetchDescriptor<PersistentIdentity>(
                predicate: #Predicate { $0.identityId == identityId }
            )
            let row: PersistentIdentity
            if let existing = try? backgroundContext.fetch(descriptor).first {
                row = existing
            } else {
                // Resolve the network from the owning wallet row.
                // `persistWalletMetadata` always fires before the
                // first `persistIdentities` call for a new wallet, so
                // the row's network is populated by now; if for some
                // reason the lookup comes up empty, fall back to
                // `.testnet` so we never block the write path on a
                // missing network column (the CreateIdentity flow
                // restamps the network on return anyway).
                let networkWalletId = entry.walletId ?? walletId
                let network = walletNetwork(walletId: networkWalletId) ?? .testnet
                // `isLocal` = "this identity is yours or tracked
                // here": wallet-derived identities are ALWAYS local
                // (promoted below once the wallet linkage attaches)
                // and manual adds (LoadIdentityView et al.) mark
                // their own rows local. Only incidental rows —
                // observed foreign identities materialized by sync —
                // stay `false`. Seed `false` at creation; the
                // wallet-attach below promotes wallet-owned rows,
                // and NOTHING ever demotes (sync must not erase a
                // user's manual mark, and losing a wallet link
                // doesn't un-track an identity).
                row = PersistentIdentity(
                    identityId: entry.identityId,
                    balance: Int64(bitPattern: entry.balance),
                    revision: Int64(bitPattern: entry.revision),
                    isLocal: false,
                    network: network
                )
                backgroundContext.insert(row)
                // Back-fill any contracts in the local store that
                // already name this identity as their owner. Runs on
                // the background context — `ContractIdentityLinker`
                // is context-agnostic and isolation-free for exactly
                // this reason. The atomic save at `endChangeset`
                // persists the relationship.
                ContractIdentityLinker.linkIdentityToOwnedContracts(
                    identity: row,
                    modelContext: backgroundContext
                )
            }
            // Scalars that ride every upsert — Rust guarantees
            // monotonic revision + paired balance/revision updates
            // by the merge gate in `IdentityChangeSet::merge`, so
            // overwriting unconditionally here is safe.
            row.balance = Int64(bitPattern: entry.balance)
            row.revision = Int64(bitPattern: entry.revision)
            // Only write the index when the snapshot actually carries
            // one (wallet-owned identities). Out-of-wallet entries
            // arrive with `nil` — leave the existing column untouched
            // rather than overwriting with the placeholder `0`, which
            // would collide with a real wallet-owned identity at
            // index 0 if the row were ever rebound.
            if let idx = entry.identityIndex {
                row.identityIndex = idx
            }
            if let label = entry.label {
                row.alias = label
            }
            row.lastUpdated = Date()

            // Reconcile the DPNS-label cache against Rust's canonical
            // last-write-wins identity snapshot. A missing label is no longer
            // owned. Untracked cache-only rows can be deleted; marketplace
            // rows are retained with `isOwned == false` so their sale/transfer
            // history remains available unless another wallet identity's
            // canonical snapshot rebinds that single unique-name row.
            upsertDPNSNames(
                identityRow: row,
                names: entry.dpnsNames
            )

            // Upsert the DashPay profile cache for this identity.
            //
            // Gated on `entry.dashpayProfile != nil` — a `nil`
            // snapshot mirrors the FFI's
            // `dashpay_profile_present == false`, which the Rust
            // `IdentityChangeSet::merge` policy treats as "no
            // update" (NOT delete). DashPay doesn't expose a
            // user-driven "delete profile" today; if it ever does,
            // the removal must arrive via a separate signal so we
            // know it's intentional. Match the dpns-name handling
            // shape: a missing snapshot leaves any existing row
            // intact.
            if let profile = entry.dashpayProfile {
                upsertDashpayProfile(identityRow: row, profile: profile)
            }

            // Upsert the cached contact-profile rows for this identity.
            //
            // One row per contact (keyed by `(owner, contact)`), distinct
            // from the own-profile upsert above. Rust emits a row only for
            // contacts it (re)fetched this sweep — present ones upsert,
            // confirmed-absent ones (`is_present == false`) delete. A
            // contact simply MISSING from this flush is "no update" (not a
            // delete). An empty array leaves any existing rows intact.
            if !entry.contactProfiles.isEmpty {
                upsertDashpayContactProfiles(
                    identityRow: row,
                    profiles: entry.contactProfiles
                )
            }

            // Attach the identity to its owning `PersistentWallet`
            // via the relationship — the sole wallet-side
            // association on the row (`deleteRule: .nullify` on the
            // inverse nulls it if the wallet row is removed).
            //
            // Owner resolution: prefer the per-entry `walletId`;
            // an entry with no `walletId` but a real
            // `identityIndex` is wallet-derived and falls back to
            // the scope wallet (the "create new identity" corner
            // case). An entry with NEITHER is an out-of-wallet
            // (observed) identity — `add_out_of_wallet_identity`
            // emits that shape — and must NOT inherit the scope
            // wallet: the old unconditional fallback mislinked
            // observed identities to whatever wallet's changeset
            // carried them.
            let ownerWalletId: Data? =
                entry.walletId ?? (entry.identityIndex != nil ? walletId : nil)
            if let ownerWallet = fetchWalletForLink(walletId: ownerWalletId) {
                row.wallet = ownerWallet
                // Things from the wallet are always local — promote.
                // One-way: no path ever writes `false` over a `true`.
                row.isLocal = true
            } else if let declaredOwnerId = ownerWalletId {
                // Declared owner didn't resolve (e.g. its wallet row
                // is absent on this handler's network scope). Keep
                // the existing link only when it already points at
                // that declared owner; a link to any OTHER wallet
                // contradicts the entry's declared ownership and is
                // cleared.
                if row.wallet?.walletId != declaredOwnerId {
                    row.wallet = nil
                }
            } else if row.wallet?.walletId == walletId {
                // A genuinely out-of-wallet entry unlinks ONLY a
                // relationship to this changeset's scope wallet —
                // the one the old fallback could have fabricated.
                // "Out-of-wallet" is relative to the emitting Rust
                // manager: wallet A resolving wallet B's identity
                // via `load_identity_by_dpns_name` emits the
                // nil/nil shape from A's manager, and the row is
                // globally keyed by identityId, so wallet B's valid
                // relationship must survive.
                row.wallet = nil
            }
        }

        for identityId in removed {
            let descriptor = FetchDescriptor<PersistentIdentity>(
                predicate: #Predicate { $0.identityId == identityId }
            )
            if let existing = try? backgroundContext.fetch(descriptor).first {
                backgroundContext.delete(existing)
            }
        }

        // No save() — bracketed by changesetBegin/End.
        }  // onQueue
    }

    /// Upsert a `PersistentDPNSName` row for every label the FFI
    /// identity entry carried. Rows are keyed on
    /// `(networkRaw, normalizedParentDomainName, normalizedLabel)`,
    /// matching `PersistentDPNSName`'s
    /// `#Unique<…>([\.networkRaw, \.normalizedParentDomainName,
    /// \.normalizedLabel])` declaration — which itself mirrors the
    /// DPNS contract's `parentNameAndLabel` unique index. If a label
    /// transferred between identities on the same network the
    /// existing row's `identity` is rebound to the current owner.
    ///
    /// The FFI `IdentityEntryFFI.dpns_names` array carries only the
    /// display label today; the parent domain defaults to `"dash"`
    /// (the only top-level DPNS domain on Dash Platform), and the
    /// normalized forms are derived via
    /// `PersistentDPNSName.normalize(_:)` on insert. If/when the FFI
    /// is extended to carry the parent domain, this site's defaults
    /// become the fallback path.
    ///
    /// The carried list is canonical for current ownership. Missing
    /// cache-only rows are pruned. Missing marketplace-tracked rows survive as
    /// history but are marked `isOwned == false` so owned-name queries and UI
    /// selection cannot surface them. If the name moved to another identity in
    /// this wallet, that identity's canonical snapshot subsequently rebinds the
    /// same unique row and marks it owned there.
    ///
    /// Assumes it's already running on `serialQueue` — only called
    /// from inside `persistIdentities`'s `onQueue` body.
    private func upsertDPNSNames(
        identityRow: PersistentIdentity,
        names: [(label: String, acquiredAt: UInt64)]
    ) {
        let networkRaw = identityRow.networkRaw
        // DPNS today exposes only the "dash" top-level domain. If the
        // FFI ever forwards a different parent, the model carries it
        // through verbatim — for now we stamp the universal default.
        let parentDomainName = "dash"
        let normalizedParentDomainName = PersistentDPNSName.normalize(parentDomainName)
        let canonicalLabels = Set(names.map { PersistentDPNSName.normalize($0.label) })
        let identityId = identityRow.identityId
        let ownedRowsDescriptor = FetchDescriptor<PersistentDPNSName>(
            predicate: #Predicate { $0.identity.identityId == identityId }
        )
        let previouslyAssociatedRows =
            (try? backgroundContext.fetch(ownedRowsDescriptor)) ?? Array(identityRow.dpnsNames)

        for row in previouslyAssociatedRows
        where !canonicalLabels.contains(row.normalizedLabel) {
            row.isOwned = false
            row.lastUpdated = Date()
            if row.documentIdBase58 == nil {
                // No marketplace history is attached, so this is only a stale
                // label-cache row and can be removed entirely.
                backgroundContext.delete(row)
            }
        }

        for entry in names {
            let normalizedLabel = PersistentDPNSName.normalize(entry.label)
            let descriptor = FetchDescriptor<PersistentDPNSName>(
                predicate: #Predicate {
                    $0.networkRaw == networkRaw
                        && $0.normalizedParentDomainName == normalizedParentDomainName
                        && $0.normalizedLabel == normalizedLabel
                }
            )
            if let existing = try? backgroundContext.fetch(descriptor).first {
                existing.isOwned = true
                // Refresh the timestamp if the FFI now carries a
                // non-zero value. Don't clobber a real timestamp
                // with a `0` placeholder — `acquired_at` is sticky
                // once set.
                if entry.acquiredAt != 0 && existing.acquiredAt != entry.acquiredAt {
                    existing.acquiredAt = entry.acquiredAt
                    existing.lastUpdated = Date()
                }
                // Refresh the display label too — a later flush may
                // carry a corrected casing for the same normalized
                // form (e.g. originally synced as "alice" then
                // re-synced as "Alice"). The normalized index column
                // doesn't change, so the unique constraint holds.
                if existing.label != entry.label {
                    existing.label = entry.label
                    existing.lastUpdated = Date()
                }
                // Rebind to the current owner if the label transferred
                // between identities on this network. DPNS supports
                // transfers, and the unique constraint is per-network,
                // so the row stays but the owner pointer moves.
                if existing.identity !== identityRow {
                    existing.identity = identityRow
                    existing.lastUpdated = Date()
                }
            } else {
                let row = PersistentDPNSName(
                    identity: identityRow,
                    label: entry.label,
                    parentDomainName: parentDomainName,
                    acquiredAt: entry.acquiredAt
                )
                backgroundContext.insert(row)
            }
        }

        let fallbackLabel = names.first?.label
        if let selected = identityRow.mainDpnsName,
           !canonicalLabels.contains(PersistentDPNSName.normalize(selected)) {
            identityRow.mainDpnsName = fallbackLabel
        }
        if let displayed = identityRow.dpnsName,
           !canonicalLabels.contains(PersistentDPNSName.normalize(displayed)) {
            identityRow.dpnsName = fallbackLabel
        }
    }

    /// Upsert the at-most-one `PersistentDashpayProfile` row for an
    /// identity. Idempotent on repeated flushes: an existing row is
    /// refreshed in place rather than replaced, so SwiftUI views
    /// observing it via `@Query` see field-level updates rather than
    /// row-replacement churn.
    ///
    /// The DashPay contract guarantees one `profile` document per
    /// `ownerId`, so we never have to disambiguate multiple rows for
    /// the same identity — `identityRow.dashpayProfile` is either
    /// already present (refresh) or absent (insert).
    ///
    /// Runs on `serialQueue` — only called from inside
    /// `persistIdentities`'s `onQueue` body.
    private func upsertDashpayProfile(
        identityRow: PersistentIdentity,
        profile: DashpayProfileSnapshot
    ) {
        if let existing = identityRow.dashpayProfile {
            // Field-level refresh. Every column is overwritten on
            // every flush — the FFI snapshot is authoritative for
            // the profile document's contents (the underlying
            // `IdentityEntry::dashpay_profile` is a whole-document
            // `Some(_)` payload, not a partial diff). Fields the
            // sender omitted come through as `nil` here too, so
            // setting them to nil mirrors the on-Platform state.
            existing.displayName = profile.displayName
            existing.bio = profile.bio
            existing.publicMessage = profile.publicMessage
            existing.avatarUrl = profile.avatarUrl
            existing.avatarHash = profile.avatarHash
            existing.avatarFingerprint = profile.avatarFingerprint
            existing.lastUpdated = Date()
        } else {
            let row = PersistentDashpayProfile(
                identity: identityRow,
                displayName: profile.displayName,
                publicMessage: profile.publicMessage,
                bio: profile.bio,
                avatarUrl: profile.avatarUrl,
                avatarHash: profile.avatarHash,
                avatarFingerprint: profile.avatarFingerprint
            )
            backgroundContext.insert(row)
            // SwiftData populates the inverse `dashpayProfile`
            // pointer from the `inverse:` declaration on
            // `PersistentIdentity.dashpayProfile`, so we don't need
            // to assign `identityRow.dashpayProfile = row` here.
        }
    }

    /// Upsert one `PersistentDashpayContactProfile` row per cached
    /// **contact** profile snapshot — keyed by `(networkRaw,
    /// ownerIdentityId, contactIdentityId)`. Idempotent on repeated
    /// flushes: an existing row is refreshed in place so SwiftUI views
    /// observing it via `@Query` see field-level updates rather than
    /// row-replacement churn.
    ///
    /// Full-REPLACE per contact, mirroring the Rust cache-write
    /// semantics (§4.7): each fetched profile is the authoritative
    /// *complete* state for that contact, so every column is overwritten
    /// — a contact who *removes* their `avatarUrl` must not keep showing
    /// a stale avatar. This is the same field-level overwrite the
    /// own-profile `upsertDashpayProfile` does, just per contact.
    ///
    /// A contact NOT in this flush keeps its existing row (Rust emits a row
    /// only for contacts it (re)fetched this sweep, so a missing snapshot is
    /// "no update"). A contact present in the flush as a `isPresent == false`
    /// tombstone is DELETED — that's a contact who removed their on-chain
    /// profile, and the stale name/avatar must not survive. The cache cannot
    /// grow duplicate rows for the same contact because of the `#Unique`
    /// compound key.
    ///
    /// Runs on `serialQueue` — only called from inside
    /// `persistIdentities`'s `onQueue` body.
    private func upsertDashpayContactProfiles(
        identityRow: PersistentIdentity,
        profiles: [ContactProfileSnapshot]
    ) {
        let ownerIdentityId = identityRow.identityId
        for profile in profiles {
            let contactIdentityId = profile.contactIdentityId
            let descriptor = FetchDescriptor<PersistentDashpayContactProfile>(
                predicate: PersistentDashpayContactProfile.predicate(
                    ownerIdentityId: ownerIdentityId,
                    contactIdentityId: contactIdentityId
                )
            )
            guard profile.isPresent else {
                // Confirmed-absent: delete the stale row if one exists; a
                // never-persisted contact is a no-op.
                if let existing = try? backgroundContext.fetch(descriptor).first {
                    backgroundContext.delete(existing)
                }
                continue
            }
            if let existing = try? backgroundContext.fetch(descriptor).first {
                existing.displayName = profile.displayName
                existing.bio = profile.bio
                existing.publicMessage = profile.publicMessage
                existing.avatarUrl = profile.avatarUrl
                existing.avatarHash = profile.avatarHash
                existing.avatarFingerprint = profile.avatarFingerprint
                existing.checkedAtMs = profile.checkedAtMs
                existing.lastUpdated = Date()
            } else {
                let row = PersistentDashpayContactProfile(
                    owner: identityRow,
                    contactIdentityId: contactIdentityId,
                    checkedAtMs: profile.checkedAtMs,
                    displayName: profile.displayName,
                    publicMessage: profile.publicMessage,
                    bio: profile.bio,
                    avatarUrl: profile.avatarUrl,
                    avatarHash: profile.avatarHash,
                    avatarFingerprint: profile.avatarFingerprint
                )
                backgroundContext.insert(row)
                // SwiftData populates the inverse `owner.contactProfiles`
                // collection from the `inverse:` declaration on
                // `PersistentIdentity.contactProfiles`.
            }
        }
    }

    // MARK: - Identity keys persistence

    /// Upsert / remove rows from `PersistentPublicKey` in response to
    /// an `IdentityKeysChangeSet` forwarded by the Rust side.
    ///
    /// Mapping:
    /// - Each `upsert` is keyed by `(identity_id, key_id)` — the
    ///   same composite the Rust side uses for `BTreeMap` uniqueness.
    /// - Each `removed` pair deletes the matching row.
    ///
    /// Private-key handling: no secret crosses the FFI. Each wallet-derivable
    /// key persists its `(walletId, identityDerivationPath)` breadcrumb so the
    /// signer derives it on demand from the Keychain seed (derive-sign-destroy).
    /// A key already materialized by another path keeps / adopts its existing
    /// `privateKeyKeychainIdentifier`; a genuinely watch-only key has neither.
    func persistIdentityKeys(
        walletId: Data,
        upserts: [IdentityKeyEntrySnapshot],
        removed: [(identityId: Data, keyId: UInt32)]
    ) {
        onQueue {
        for entry in upserts {
            // PersistentPublicKey is keyed on (identity, keyId) via
            // its parent relationship; fetch by keyId + identityId
            // (stored as base58 string on the row).
            let targetKeyId = Int32(bitPattern: entry.keyId)
            let identityHex = entry.identityId.toBase58String()
            let descriptor = FetchDescriptor<PersistentPublicKey>(
                predicate: #Predicate {
                    $0.keyId == targetKeyId && $0.identityId == identityHex
                }
            )
            // Project the snapshot's ContractBounds enum into the
            // pair of columns `PersistentPublicKey` uses:
            //   * `contractBoundsIds` — `[contractId]` (or nil)
            //   * `contractBoundsDocumentTypeName` — non-nil iff the
            //     bound was `.singleContractDocumentType`
            // Keeping both lets the SwiftData row round-trip both
            // variants verbatim; legacy stores without the
            // doc-type column just see `nil` for the second field
            // and reconstruct as `.singleContract`.
            let snapshotBoundsIds: [Data]?
            let snapshotBoundsDocType: String?
            switch entry.contractBounds {
            case .some(.singleContract(let id)):
                snapshotBoundsIds = [id]
                snapshotBoundsDocType = nil
            case .some(.singleContractDocumentType(let id, let name)):
                snapshotBoundsIds = [id]
                snapshotBoundsDocType = name
            case .none:
                snapshotBoundsIds = nil
                snapshotBoundsDocType = nil
            }

            let row: PersistentPublicKey
            if let existing = try? backgroundContext.fetch(descriptor).first {
                row = existing
            } else {
                let purposeEnum = KeyPurpose(rawValue: entry.purpose) ?? .authentication
                let levelEnum = SecurityLevel(rawValue: entry.securityLevel) ?? .high
                let keyTypeEnum = KeyType(rawValue: entry.keyType) ?? .ecdsaSecp256k1
                row = PersistentPublicKey(
                    keyId: targetKeyId,
                    purpose: purposeEnum,
                    securityLevel: levelEnum,
                    keyType: keyTypeEnum,
                    publicKeyData: entry.publicKeyData,
                    readOnly: entry.readOnly,
                    disabledAt: entry.disabledAt.map { Int64(bitPattern: $0) },
                    contractBounds: snapshotBoundsIds,
                    contractBoundsDocumentTypeName: snapshotBoundsDocType,
                    identityId: identityHex
                )
                backgroundContext.insert(row)
                // Link to the owning identity if we already have the
                // row. (We don't insert a missing parent here —
                // Rust-side ordering guarantees identities apply
                // before keys within the same changeset.)
                let identityIdData = entry.identityId
                let parentDescriptor = FetchDescriptor<PersistentIdentity>(
                    predicate: #Predicate { $0.identityId == identityIdData }
                )
                if let parent = try? backgroundContext.fetch(parentDescriptor).first {
                    row.identity = parent
                    parent.addPublicKey(row)
                }
            }
            // Refresh mutable fields every upsert.
            row.publicKeyData = entry.publicKeyData
            row.readOnly = entry.readOnly
            row.disabledAt = entry.disabledAt.map { Int64(bitPattern: $0) }
            // Mirror the contract-bounds projection onto an
            // existing row too — Rust is the source of truth on
            // each callback, so the snapshot's bounds (which can
            // change if Drive ever re-emits a key with a different
            // scope) must overwrite any stale value here.
            row.contractBounds = snapshotBoundsIds
            row.contractBoundsDocumentTypeName = snapshotBoundsDocType

            // Private-key handling: no secret crosses the FFI. A
            // wallet-derivable key whose private bytes were materialized by
            // another path (e.g. identity registration writes its keychain
            // items directly) adopts that existing keychain account by a
            // public-key-hex lookup — no derivation, no secret loaded — so the
            // legacy fast-path signer lookup and the `hasPrivateKey` marker
            // still work for already-materialized keys. A genuinely watch-only
            // key finds nothing and stays so. Every wallet-derivable key also
            // gets its breadcrumb persisted below, so a freshly discovered key
            // (no keychain item yet) signs by deriving on demand from the seed.
            if entry.derivationIndices != nil,
                row.privateKeyKeychainIdentifier == nil
            {
                if let account = KeychainManager.shared.identityPrivateKeyAccount(
                    publicKeyHex: entry.publicKeyData.toHexString()
                ) {
                    row.privateKeyKeychainIdentifier = account
                }
            }

            // Persist the derivation breadcrumb so the signer can derive this
            // key on demand from the Keychain seed (derive-sign-destroy),
            // independent of whether a scalar was carried this callback. Always
            // overwrite when the key is wallet-derivable so a backfilled value
            // and a freshly-persisted one stay byte-identical.
            if let indices = entry.derivationIndices {
                let resolvedWalletId = entry.walletId ?? walletId
                row.walletId = resolvedWalletId
                if let path = identityAuthPath(walletId: resolvedWalletId, indices: indices) {
                    row.identityDerivationPath = path
                }
            }

            row.lastAccessed = Date()
        }

        for (identityIdBytes, keyId) in removed {
            let targetKeyId = Int32(bitPattern: keyId)
            let identityHex = identityIdBytes.toBase58String()
            let descriptor = FetchDescriptor<PersistentPublicKey>(
                predicate: #Predicate {
                    $0.keyId == targetKeyId && $0.identityId == identityHex
                }
            )
            if let existing = try? backgroundContext.fetch(descriptor).first {
                backgroundContext.delete(existing)
            }
        }

        // `walletId` is consumed as the scope fallback when resolving the
        // owning wallet for a carried key, so it's not a dead parameter.
        // No save() — bracketed by changesetBegin/End.
        }  // onQueue
    }

    // MARK: - Token balance persistence

    /// Apply a `TokenBalanceChangeSet` upsert/removal pair to
    /// `PersistentTokenBalance` rows.
    ///
    /// Mapping:
    /// - Each upsert is keyed by `(tokenId, identityId)` — the same
    ///   composite the Rust side uses on its `BTreeMap`. The 32-byte
    ///   token id from Rust is rendered as base58 to match
    ///   `PersistentTokenBalance.tokenId` (string column, the same
    ///   shape the rest of the app uses for token id strings).
    /// - Each removal deletes the matching row.
    ///
    /// Token metadata (name, symbol, decimals) is owned by
    /// `PersistentToken` and joined at read time — we don't replicate
    /// it here. The `PersistentTokenBalance.token` relationship is
    /// linked when the matching `PersistentToken` row exists; rows
    /// inserted before the contract has been parsed locally simply
    /// link later when SwiftUI re-queries.
    func persistTokenBalances(
        walletId: Data,
        upserts: [TokenBalanceUpsertSnapshot],
        removals: [TokenBalanceRemovalSnapshot]
    ) {
        onQueue {
        let network = walletNetwork(walletId: walletId) ?? .testnet

        for entry in upserts {
            let tokenIdBase58 = entry.tokenId.toBase58String()
            let identityIdData = entry.identityId
            let descriptor = FetchDescriptor<PersistentTokenBalance>(
                predicate: #Predicate {
                    $0.tokenId == tokenIdBase58 && $0.identityId == identityIdData
                }
            )
            let row: PersistentTokenBalance
            if let existing = try? backgroundContext.fetch(descriptor).first {
                row = existing
            } else {
                row = PersistentTokenBalance(
                    tokenId: tokenIdBase58,
                    identityId: entry.identityId,
                    unsignedBalance: 0,
                    network: network
                )
                backgroundContext.insert(row)
                linkTokenBalanceRelations(
                    row: row,
                    identityId: entry.identityId,
                    tokenIdData: entry.tokenId
                )
            }
            row.updateUnsignedBalance(entry.balance)
            row.markAsSynced()
            // Re-link on every upsert too so a balance row that
            // pre-existed before its parent identity / token row
            // landed gets stitched into the relationship graph on the
            // next sync round.
            if row.identity == nil || row.token == nil {
                linkTokenBalanceRelations(
                    row: row,
                    identityId: entry.identityId,
                    tokenIdData: entry.tokenId
                )
            }
        }

        for entry in removals {
            let tokenIdBase58 = entry.tokenId.toBase58String()
            let identityIdData = entry.identityId
            let descriptor = FetchDescriptor<PersistentTokenBalance>(
                predicate: #Predicate {
                    $0.tokenId == tokenIdBase58 && $0.identityId == identityIdData
                }
            )
            if let existing = try? backgroundContext.fetch(descriptor).first {
                backgroundContext.delete(existing)
            }
        }

        // No save() — bracketed by changesetBegin/End from the Rust
        // store() round.
        }  // onQueue
    }

    /// Stitch a freshly-inserted `PersistentTokenBalance` into the
    /// relationship graph: link the owning `PersistentIdentity` (when
    /// present locally) and the matching `PersistentToken` (looked up
    /// by its 32-byte canonical id, which `PersistentToken.id`
    /// stores). Either side may legitimately be missing if the row
    /// is being inserted before the contract has been parsed locally
    /// — the next sync round re-links via the upsert-path nil-check.
    private func linkTokenBalanceRelations(
        row: PersistentTokenBalance,
        identityId: Data,
        tokenIdData: Data
    ) {
        let identityDescriptor = FetchDescriptor<PersistentIdentity>(
            predicate: #Predicate { $0.identityId == identityId }
        )
        if let parent = try? backgroundContext.fetch(identityDescriptor).first {
            row.identity = parent
        }
        let tokenDescriptor = FetchDescriptor<PersistentToken>(
            predicate: #Predicate { $0.id == tokenIdData }
        )
        if let token = try? backgroundContext.fetch(tokenDescriptor).first {
            row.token = token
        }
    }

    /// Owned snapshot of a `TokenBalanceUpsertFFI` row. Same
    /// rationale as `IdentityEntrySnapshot`: callbacks copy out the
    /// raw FFI struct fields before the trampoline returns, so the
    /// handler runs against pure-Swift values regardless of when the
    /// Rust-side allocation gets reclaimed.
    struct TokenBalanceUpsertSnapshot {
        let identityId: Data
        let tokenId: Data
        let balance: UInt64
    }

    /// Owned snapshot of a `TokenBalanceRemovalFFI` row.
    struct TokenBalanceRemovalSnapshot {
        let identityId: Data
        let tokenId: Data
    }

    // MARK: - DashPay contact-request persistence

    /// Apply a DashPay `ContactChangeSet` projection to SwiftData.
    ///
    /// Mapping:
    /// - Each `upsert.ContactRequestFFI` becomes one row keyed by
    ///   `(networkRaw, ownerIdentityId, contactIdentityId, isOutgoing)`
    ///   on `PersistentDashpayContactRequest`. The Rust side projects
    ///   `ContactChangeSet::sent_requests` / `incoming_requests` /
    ///   `established` into this flat array (with `is_outgoing`
    ///   stamped per row), so the upsert path is direction-agnostic.
    /// - Each `removedSent` row drops the matching outgoing row.
    /// - Each `removedIncoming` row drops the matching incoming row.
    /// - Each `ignored` entry (`isIgnored == true`) drops **every**
    ///   incoming row from that sender — ignore is per-sender, so a
    ///   rotated (bumped-`accountReference`) request is suppressed too
    ///   (unlike the old per-`accountReference` reject) — and upserts
    ///   the `PersistentDashpayIgnoredSender` row. An `unignored` entry
    ///   (`isIgnored == false`) deletes that ignored-sender row. The
    ///   Rust side owns ignore suppression across re-syncs (an ignored
    ///   sender never re-enters `upserts`); SwiftData only stops showing
    ///   them and persists the ignored set for the Ignored screen.
    ///
    /// The owner identity is required to exist in SwiftData before
    /// the row is inserted — the relationship is non-optional and
    /// `networkRaw` is read off it. If a flush carries a contact
    /// upsert for an owner identity Swift hasn't seen yet (race with
    /// a first-time identity flush), the row is skipped; the next
    /// flush will replay it after the identity row lands. In
    /// practice the changeset is one round, so this only matters
    /// for the very first identity registration where the contact
    /// changeset and identity changeset arrive in the same store()
    /// call — within a round, identities apply before contacts (see
    /// the ordering in `FFIPersister::store`), so the lookup here
    /// will normally succeed.
    func persistContacts(
        walletId: Data,
        upserts: [ContactRequestSnapshot],
        removedSent: [ContactRequestRemovalSnapshot],
        removedIncoming: [ContactRequestRemovalSnapshot],
        ignored: [ContactIgnoredSenderSnapshot]
    ) {
        onQueue {
            for entry in upserts {
                let ownerId = entry.ownerIdentityId
                let ownerDescriptor = FetchDescriptor<PersistentIdentity>(
                    predicate: #Predicate { $0.identityId == ownerId }
                )
                guard let owner = try? backgroundContext.fetch(ownerDescriptor).first else {
                    // Owner identity hasn't landed yet. Within a
                    // single round identities apply before contacts,
                    // so we'd only hit this if the FFI changeset
                    // surfaces a contact for an identity that isn't
                    // managed by any wallet locally — there's no
                    // identity row to hang it off, and the contract's
                    // `ownerId` invariant means the row would be
                    // orphaned anyway. The recurring sweep replays it
                    // once the owner row exists; log so a contact that
                    // is somehow dropped permanently (e.g. an
                    // out-of-wallet owner with no PersistentIdentity)
                    // is at least observable rather than vanishing
                    // silently.
                    print("⚠️ persistContacts: skipped contact upsert — no PersistentIdentity for owner \(entry.ownerIdentityId.prefix(8).toHexString())…; will retry next sync round")
                    continue
                }

                let networkRaw = owner.networkRaw
                let contactId = entry.contactIdentityId
                let isOutgoing = entry.isOutgoing
                let descriptor = FetchDescriptor<PersistentDashpayContactRequest>(
                    predicate: #Predicate {
                        $0.networkRaw == networkRaw
                            && $0.ownerIdentityId == ownerId
                            && $0.contactIdentityId == contactId
                            && $0.isOutgoing == isOutgoing
                    }
                )
                if let existing = try? backgroundContext.fetch(descriptor).first {
                    // Refresh in place — every column is overwritten
                    // because the FFI snapshot is authoritative for
                    // the underlying `ContactRequest` document. This
                    // is also the path `established` rows take to
                    // promote a previously-pending row in place over
                    // its prior `sent_requests` / `incoming_requests`
                    // entry; the unique key is identical because the
                    // promotion doesn't change `(owner, contact,
                    // direction)`.
                    existing.senderKeyIndex = entry.senderKeyIndex
                    existing.recipientKeyIndex = entry.recipientKeyIndex
                    existing.accountReference = entry.accountReference
                    existing.encryptedPublicKey = entry.encryptedPublicKey
                    existing.encryptedAccountLabel = entry.encryptedAccountLabel
                    existing.autoAcceptProof = entry.autoAcceptProof
                    existing.coreHeightCreatedAt = entry.coreHeightCreatedAt
                    existing.createdAtMillis = entry.createdAtMillis
                    existing.paymentChannelBroken = entry.paymentChannelBroken
                    existing.contactAlias = entry.contactAlias
                    existing.contactNote = entry.contactNote
                    existing.contactHidden = entry.contactHidden
                    existing.contactAccountLabel = entry.contactAccountLabel
                    existing.contactAcceptedAccounts = entry.contactAcceptedAccounts
                    if existing.owner !== owner {
                        existing.owner = owner
                    }
                    existing.lastUpdated = Date()
                } else {
                    let row = PersistentDashpayContactRequest(
                        owner: owner,
                        contactIdentityId: entry.contactIdentityId,
                        isOutgoing: entry.isOutgoing,
                        senderKeyIndex: entry.senderKeyIndex,
                        recipientKeyIndex: entry.recipientKeyIndex,
                        accountReference: entry.accountReference,
                        encryptedPublicKey: entry.encryptedPublicKey,
                        encryptedAccountLabel: entry.encryptedAccountLabel,
                        autoAcceptProof: entry.autoAcceptProof,
                        coreHeightCreatedAt: entry.coreHeightCreatedAt,
                        createdAtMillis: entry.createdAtMillis,
                        paymentChannelBroken: entry.paymentChannelBroken
                    )
                    row.contactAlias = entry.contactAlias
                    row.contactNote = entry.contactNote
                    row.contactHidden = entry.contactHidden
                    row.contactAccountLabel = entry.contactAccountLabel
                    row.contactAcceptedAccounts = entry.contactAcceptedAccounts
                    backgroundContext.insert(row)
                }
            }

            for tomb in removedSent {
                deleteContactRow(
                    ownerId: tomb.ownerIdentityId,
                    contactId: tomb.contactIdentityId,
                    isOutgoing: true
                )
            }
            for tomb in removedIncoming {
                deleteContactRow(
                    ownerId: tomb.ownerIdentityId,
                    contactId: tomb.contactIdentityId,
                    isOutgoing: false
                )
            }
            for row in ignored {
                if row.isIgnored {
                    // Ignore: (1) drop the sender's incoming row so the
                    // request stops showing in the pending UI, and (2)
                    // persist a durable ignored-sender row so the Rust
                    // `ignored_senders` set can be restored at load —
                    // without (2) the ignored sender resurfaces on the
                    // next post-relaunch sweep. Per-sender (no
                    // accountReference): ALL the sender's incoming rows go.
                    deleteIgnoredSenderIncomingRows(
                        ownerId: row.ownerIdentityId,
                        senderId: row.senderIdentityId
                    )
                    upsertIgnoredSender(row)
                } else {
                    // Un-ignore: delete the ignored-sender row so the
                    // sender's requests resurface on the next sweep (the
                    // Rust side rewinds the cursor to re-fetch them).
                    deleteIgnoredSender(
                        ownerId: row.ownerIdentityId,
                        senderId: row.senderIdentityId
                    )
                }
            }
            // No save() — bracketed by changesetBegin/End from the
            // Rust store() round.
            _ = walletId  // reserved for future wallet-scope batching
        }
    }

    /// Delete the single `PersistentDashpayContactRequest` row matching
    /// `(ownerIdentityId, contactIdentityId, isOutgoing)`. The fourth
    /// uniqueness column (`networkRaw`) is implied by the owner — an
    /// identity belongs to exactly one network — so we don't have to
    /// fan out the predicate across networks. Silent on miss (no
    /// existing row): the FFI changeset replays tombstones, and an
    /// already-removed row is the success state.
    ///
    /// Assumes it's already running on `serialQueue`.
    private func deleteContactRow(ownerId: Data, contactId: Data, isOutgoing: Bool) {
        let direction = isOutgoing
        let descriptor = FetchDescriptor<PersistentDashpayContactRequest>(
            predicate: #Predicate {
                $0.ownerIdentityId == ownerId
                    && $0.contactIdentityId == contactId
                    && $0.isOutgoing == direction
            }
        )
        if let existing = try? backgroundContext.fetch(descriptor).first {
            backgroundContext.delete(existing)
        }
    }

    /// Drop every incoming-request row from an ignored sender so their
    /// requests stop lingering in the UI store. Per-sender (no
    /// `accountReference` gate): unlike the old reject, ignore suppresses
    /// ALL of the sender's requests, including rotated ones. Silent on
    /// miss: an already-removed row is the success state.
    ///
    /// Assumes it's already running on `serialQueue`.
    private func deleteIgnoredSenderIncomingRows(ownerId: Data, senderId: Data) {
        let descriptor = FetchDescriptor<PersistentDashpayContactRequest>(
            predicate: #Predicate {
                $0.ownerIdentityId == ownerId
                    && $0.contactIdentityId == senderId
                    && $0.isOutgoing == false
            }
        )
        if let rows = try? backgroundContext.fetch(descriptor) {
            for row in rows {
                backgroundContext.delete(row)
            }
        }
    }

    /// Persist one ignored sender as a durable
    /// `PersistentDashpayIgnoredSender` row so the Rust `ignored_senders`
    /// set can be rebuilt at load. Without this the in-memory set starts
    /// empty after relaunch and the still-on-platform immutable
    /// `contactRequest`s re-ingest on the next sweep, resurfacing the
    /// ignored sender.
    ///
    /// Upsert keyed `(networkRaw, ownerIdentityId, ignoredSenderId)` — the
    /// Rust per-sender suppression key. Idempotent: a replay of the same
    /// ignore is a no-op. Requires the owner `PersistentIdentity` to exist
    /// (the row hangs off it); skipped + logged if it hasn't landed yet —
    /// the next sync round replays it.
    ///
    /// Assumes it's already running on `serialQueue`.
    private func upsertIgnoredSender(_ row: ContactIgnoredSenderSnapshot) {
        let ownerId = row.ownerIdentityId
        let ownerDescriptor = FetchDescriptor<PersistentIdentity>(
            predicate: #Predicate { $0.identityId == ownerId }
        )
        guard let owner = try? backgroundContext.fetch(ownerDescriptor).first else {
            print("⚠️ persistContacts: skipped ignored-sender — no PersistentIdentity for owner \(row.ownerIdentityId.prefix(8).toHexString())…; will retry next sync round")
            return
        }

        let networkRaw = owner.networkRaw
        let senderId = row.senderIdentityId
        let descriptor = FetchDescriptor<PersistentDashpayIgnoredSender>(
            predicate: #Predicate {
                $0.networkRaw == networkRaw
                    && $0.ownerIdentityId == ownerId
                    && $0.ignoredSenderId == senderId
            }
        )
        if (try? backgroundContext.fetch(descriptor).first) == nil {
            backgroundContext.insert(
                PersistentDashpayIgnoredSender(
                    owner: owner,
                    ignoredSenderId: row.senderIdentityId
                )
            )
        }
    }

    /// Delete the ignored-sender row matching `(ownerId, senderId)` — the
    /// un-ignore path. Silent on miss: an already-removed row is the
    /// success state.
    ///
    /// Assumes it's already running on `serialQueue`.
    private func deleteIgnoredSender(ownerId: Data, senderId: Data) {
        let descriptor = FetchDescriptor<PersistentDashpayIgnoredSender>(
            predicate: #Predicate {
                $0.ownerIdentityId == ownerId
                    && $0.ignoredSenderId == senderId
            }
        )
        if let existing = try? backgroundContext.fetch(descriptor).first {
            backgroundContext.delete(existing)
        }
    }

    /// Owned snapshot of a `ContactRequestFFI` row. Decouples the
    /// lifetime of the encrypted-key buffers from the Rust-side
    /// allocation: the callback copies them into Swift `Data` before
    /// returning, so `free_contact_requests_ffi` runs cleanly.
    struct ContactRequestSnapshot {
        let ownerIdentityId: Data
        let contactIdentityId: Data
        let isOutgoing: Bool
        let senderKeyIndex: UInt32
        let recipientKeyIndex: UInt32
        let accountReference: UInt32
        let encryptedPublicKey: Data
        let encryptedAccountLabel: Data?
        let autoAcceptProof: Data?
        let coreHeightCreatedAt: UInt32
        let createdAtMillis: UInt64
        let paymentChannelBroken: Bool
        /// Owner-private alias (contactInfo-backed, M3). Established
        /// rows only — nil for pending rows.
        let contactAlias: String?
        /// Owner-private note — same conventions as `contactAlias`.
        let contactNote: String?
        /// `contactInfo.displayHidden`.
        let contactHidden: Bool
        /// The contact's decrypted account label — system-derived,
        /// incoming-row only (nil on outgoing / pending rows).
        let contactAccountLabel: String?
        /// `EstablishedContact::accepted_accounts` — DIP-15 rotated-account
        /// acceptances. Established rows only (replicated onto both
        /// directions); empty for pending rows.
        let contactAcceptedAccounts: [UInt32]
    }

    /// Owned snapshot of a `ContactRequestRemovalFFI` row. Carries
    /// just the `(owner, contact)` pair — the direction is implied
    /// by which array (`removed_sent` vs `removed_incoming`) the
    /// removal came from on the FFI side.
    struct ContactRequestRemovalSnapshot {
        let ownerIdentityId: Data
        let contactIdentityId: Data
    }

    /// Owned snapshot of a `ContactIgnoredSenderFFI` row. The per-sender
    /// suppression key is `(owner, sender)` — no `accountReference`, so an
    /// ignored sender's requests are ALL suppressed (rotations included).
    /// `isIgnored` is the insert/remove bit: `true` ⇒ persist the
    /// ignored-sender row (an ignore); `false` ⇒ delete it (an un-ignore).
    struct ContactIgnoredSenderSnapshot {
        let ownerIdentityId: Data
        let senderIdentityId: Data
        let isIgnored: Bool
    }

    // MARK: - DashPay payment-history persistence

    /// Upsert DashPay payment-history rows for one owner identity —
    /// the reconciler half of the payment durability loop. Called by
    /// `PlatformWalletManager.refreshDashPayPayments` after reading
    /// the `managed_identity_get_dashpay_payments` getter, so the UI
    /// can `@Query` `PersistentDashpayPayment` rows reactively. The
    /// authoritative event-driven half is the
    /// `on_persist_dashpay_payments_fn` persister callback
    /// (`persistDashpayPayments(walletId:entriesByOwner:)` below);
    /// this refresh path reconciles anything the callback era predates
    /// or a parked-row drop lost.
    ///
    /// Skips silently when the owner identity row doesn't exist yet —
    /// the next refresh after the identity flush replays it.
    ///
    /// Saves immediately when no changeset round is open — same
    /// convention as the other app-facing writers (`setWalletName`):
    /// mid-round calls leave the commit/rollback to `endChangeset`.
    public func persistDashpayPayments(
        ownerIdentityId: Data,
        payments: [DashPayPayment]
    ) {
        onQueue {
            guard stageDashpayPaymentUpserts(
                ownerIdentityId: ownerIdentityId,
                payments: payments
            ) else { return }
            // Same guard as the other app-facing writers
            // (`setWalletName`, …): a refresh landing while a Rust
            // persister round is open must ride that round's
            // endChangeset commit/rollback instead of flushing the
            // half-applied round early.
            //
            // Surface (don't swallow) a save failure: a dropped payment
            // upsert silently loses Sent history + memos, the exact H1
            // symptom this path exists to prevent, so a failure must at
            // least be observable rather than vanishing behind `try?`.
            if !self.inChangeset {
                do {
                    try backgroundContext.save()
                } catch {
                    print("⚠️ persistDashpayPayments: SwiftData save failed — payment history may be incomplete: \(error)")
                }
            }
        }
    }

    /// Apply one `on_persist_dashpay_payments_fn` persister-callback
    /// batch — payment rows flattened out of a Rust `store()` round,
    /// grouped by owner identity. This is the event-driven write half
    /// of the payment durability loop: it fires on every round whose
    /// changeset carries payment rows (a live `send_payment`, a
    /// pending→confirmed sweep flip, a reconstruction upsert), so
    /// Sent entries + memos are durable without any UI surface ever
    /// appearing.
    ///
    /// Runs mid-round: rows are staged on `backgroundContext` and ride
    /// the round's `endChangeset` commit/rollback. A group whose owner
    /// `PersistentIdentity` row isn't resolvable — not even as a
    /// pending insert staged by this round's identities callback,
    /// which fires first — is parked on `deferredPaymentUpserts`;
    /// `endChangeset` stages parked groups before the round's single
    /// save and fails the round if an owner is still unresolvable (see
    /// that field's doc).
    func persistDashpayPayments(
        walletId: Data,
        entriesByOwner: [Data: [DashPayPayment]]
    ) {
        onQueue {
            _ = walletId
            for (ownerId, payments) in entriesByOwner {
                if !stageDashpayPaymentUpserts(ownerIdentityId: ownerId, payments: payments) {
                    deferredPaymentUpserts.append((ownerId, payments))
                }
            }
            // No save here even outside a round: the Rust store() round
            // that invoked this callback brackets it with begin/end, so
            // `inChangeset` is set in practice; if a host ever fires it
            // without a bracket, autosave/next round flushes the stage.
        }
    }

    /// Stage upserts for one owner's payment rows on
    /// `backgroundContext` — shared core of the persister callback,
    /// the post-commit replay, and the refresh reconciler. No
    /// `save()`; each caller owns its own commit point. Returns
    /// `false` (nothing staged) when the owner `PersistentIdentity`
    /// row doesn't exist — not even as a pending insert in the open
    /// round. Must run on `serialQueue`.
    ///
    /// Upsert-only: the Rust `dashpay_payments` map is append-only
    /// history (keyed by txid), so this never has to delete rows;
    /// cascade from the owner identity handles wallet wipes. Rows are
    /// keyed `(networkRaw, ownerIdentityId, txid)`.
    private func stageDashpayPaymentUpserts(
        ownerIdentityId: Data,
        payments: [DashPayPayment]
    ) -> Bool {
        let ownerId = ownerIdentityId
        let ownerDescriptor = FetchDescriptor<PersistentIdentity>(
            predicate: #Predicate { $0.identityId == ownerId }
        )
        guard let owner = try? backgroundContext.fetch(ownerDescriptor).first else {
            return false
        }
        let networkRaw = owner.networkRaw

        for payment in payments {
            guard !payment.txid.isEmpty else { continue }
            let txid = payment.txid
            let descriptor = FetchDescriptor<PersistentDashpayPayment>(
                predicate: #Predicate {
                    $0.networkRaw == networkRaw
                        && $0.ownerIdentityId == ownerId
                        && $0.txid == txid
                }
            )
            if let existing = try? backgroundContext.fetch(descriptor).first {
                // Refresh in place only when a field actually changed.
                // The FFI snapshot is authoritative, and `status` is the
                // field that moves (Pending → Confirmed / Failed). A
                // no-op rewrite would still dirty the row and re-fire
                // every `@Query` observer on each refresh pass — and the
                // recurring DashPay-sync falling edge calls this even on
                // a quiescent channel, so skipping unchanged rows keeps
                // an open payment list from re-rendering every sync.
                let changed = existing.counterpartyIdentityId != payment.counterpartyId
                    || existing.amountDuffs != payment.amountDuffs
                    || existing.directionRaw != payment.direction.rawValue
                    || existing.statusRaw != payment.status.rawValue
                    || existing.memo != payment.memo
                    || existing.owner !== owner
                if changed {
                    existing.counterpartyIdentityId = payment.counterpartyId
                    existing.amountDuffs = payment.amountDuffs
                    existing.directionRaw = payment.direction.rawValue
                    existing.statusRaw = payment.status.rawValue
                    existing.memo = payment.memo
                    if existing.owner !== owner {
                        existing.owner = owner
                    }
                    existing.lastUpdated = Date()
                }
            } else {
                let row = PersistentDashpayPayment(
                    owner: owner,
                    counterpartyIdentityId: payment.counterpartyId,
                    amountDuffs: payment.amountDuffs,
                    direction: payment.direction,
                    status: payment.status,
                    txid: payment.txid,
                    memo: payment.memo
                )
                backgroundContext.insert(row)
            }
        }
        return true
    }

    // MARK: - Identity key derivation-path helpers

    /// Resolve the wallet's network and format the DIP-9 identity-auth path
    /// for `(identityIndex, keyIndex)`. Pure string formatting (the FFI
    /// formatter takes only network + indices, no mnemonic — not key
    /// derivation). `nil` if the wallet row is missing or the path build
    /// fails. Scoped to THIS handler's network via `walletRecordPredicate`,
    /// since the same `walletId` can have a row per network.
    private func identityAuthPath(
        walletId: Data,
        indices: (identityIndex: UInt32, keyIndex: UInt32)
    ) -> String? {
        let walletDescriptor = FetchDescriptor<PersistentWallet>(
            predicate: walletRecordPredicate(walletId: walletId)
        )
        guard let persistentWallet = try? backgroundContext.fetch(walletDescriptor).first else {
            return nil
        }
        let network: Network = persistentWallet.network ?? .testnet
        return try? KeyDerivation.getIdentityAuthenticationPath(
            network: network,
            identityIndex: indices.identityIndex,
            keyIndex: indices.keyIndex
        )
    }

    /// One-time, Keychain-driven, self-verifying backfill of the derivation
    /// breadcrumb columns for `walletId`'s identity keys that were materialized
    /// before those columns existed. For each `identity_privkey.*` item owned
    /// by the wallet it matches the `PersistentPublicKey` row by public key and
    /// — when the stored path is the canonical DIP-9 path for its indices (a
    /// seedless self-check) — writes `(walletId, identityDerivationPath)` so the
    /// key signs via the resolver instead of the stored scalar.
    ///
    /// Idempotent: rows that already carry a path are skipped. Keychain-sourced,
    /// so it heals even after a SwiftData store rebuild. The sign-time pubkey
    /// binding is the ultimate guard; this only rejects an obviously-corrupt
    /// path up front. A non-zero `failed` count means some materialized key
    /// could not be migrated — a signal the scalar-deletion gate must not be
    /// crossed yet.
    /// Fire-and-forget production entry: scans the Keychain and runs the
    /// backfill on the serial queue, OFF the calling (main) thread — the
    /// `@MainActor` unlock path must not block on the Keychain scan + the
    /// serial-queue SwiftData work. Safe to run lazily: an un-migrated key
    /// still signs via the stored-scalar fallback until this heals it.
    func scheduleBackfillIdentityKeyBreadcrumbs(walletId: Data) {
        let walletIdHex = walletId.toHexString()
        serialQueue.async { [weak self] in
            guard let self else { return }
            let items = KeychainManager.shared.allIdentityPrivateKeyMetadata()
                .filter { $0.walletId.caseInsensitiveCompare(walletIdHex) == .orderedSame }
            _ = self.backfillCore(walletId: walletId, items: items)
        }
    }

    /// Testable entry point: the caller supplies the metadata items (so a unit
    /// test can inject them without the real Keychain). Runs the SwiftData work
    /// synchronously on the serial queue.
    @discardableResult
    func backfillIdentityKeyBreadcrumbs(
        walletId: Data,
        items: [KeychainManager.IdentityPrivateKeyMetadata]
    ) -> (written: Int, skipped: Int, failed: Int) {
        onQueue { backfillCore(walletId: walletId, items: items) }
    }

    /// Backfill body. **Assumes it is already running on `serialQueue`** (it
    /// touches `backgroundContext` directly — do not wrap in `onQueue`).
    private func backfillCore(
        walletId: Data,
        items: [KeychainManager.IdentityPrivateKeyMetadata]
    ) -> (written: Int, skipped: Int, failed: Int) {
        guard !items.isEmpty else { return (0, 0, 0) }

        // A backfill that lands mid-round must NOT touch `backgroundContext`:
        // its save() would flush the round's staged writes early, and even a
        // save-less mutation would ride the round's own `save()`/`rollback()`.
        // Park the request and let `endChangeset` replay it once the round has
        // settled. Nothing is written on this call — the caller (unlock path)
        // is fire-and-forget, and the deletion-gate `failed` count is only read
        // from the synchronous, out-of-round entry point.
        if inChangeset {
            deferredBackfills.append((walletId: walletId, items: items))
            return (0, 0, 0)
        }

        var written = 0
        var skipped = 0
        var failed = 0

        let walletDescriptor = FetchDescriptor<PersistentWallet>(
            predicate: walletRecordPredicate(walletId: walletId)
        )
        let network: Network =
            (try? backgroundContext.fetch(walletDescriptor).first)?.network ?? .testnet

        for meta in items {
            guard let pubKeyData = Data(hexString: meta.publicKey) else {
                failed += 1
                continue
            }
            let descriptor = FetchDescriptor<PersistentPublicKey>(
                predicate: #Predicate<PersistentPublicKey> { $0.publicKeyData == pubKeyData }
            )
            guard let row = try? backgroundContext.fetch(descriptor).first else {
                // No row yet (e.g. store rebuilt before discovery re-ran).
                // Discovery re-materializes and writes the column itself;
                // nothing for the backfill to heal here.
                continue
            }
            if row.identityDerivationPath != nil {
                skipped += 1
                continue
            }
            guard
                let expectedPath = try? KeyDerivation.getIdentityAuthenticationPath(
                    network: network,
                    identityIndex: meta.identityIndex,
                    keyIndex: meta.keyIndex
                ),
                expectedPath == meta.derivationPath
            else {
                print("⚠️ backfill: path self-check failed for \(meta.publicKey.prefix(8))… — left unmigrated")
                failed += 1
                continue
            }
            row.walletId = walletId
            row.identityDerivationPath = meta.derivationPath
            written += 1
        }

        // Save only when a row actually changed; `failed`/`skipped` paths never
        // mutate the context.
        if written > 0 {
            try? backgroundContext.save()
        }
        if written > 0 || failed > 0 {
            print("ℹ️ backfill(\(walletId.toHexString().prefix(8))…): wrote \(written), skipped \(skipped), failed \(failed)")
        }
        return (written, skipped, failed)
    }

    // MARK: - Identity snapshot structs

    /// Swift-side snapshot of the Rust `IdentityEntryFFI` with C
    /// strings + byte tuples already converted. The callback copies
    /// these out of the raw FFI struct before handing control to
    /// `persistIdentities` so the Rust-side free-loop can run
    /// immediately after the callback returns.
    struct IdentityEntrySnapshot {
        let identityId: Data
        let balance: UInt64
        let revision: UInt64
        /// `nil` for out-of-wallet (observed) identities — they have
        /// no derivation context. `Some(_)` mirrors the BIP-9 HD
        /// identity index used during registration.
        let identityIndex: UInt32?
        let label: String?
        let status: UInt8
        let walletId: Data?
        /// Confirmed DPNS labels owned by this identity, paired with
        /// their `acquired_at` Unix-millis timestamp (`0` when the
        /// source `Option<u64>` was `None`). Mirrors the parallel
        /// `dpns_names` / `dpns_names_acquired_at` arrays on
        /// `IdentityEntryFFI`. Empty when the identity has no settled
        /// labels.
        let dpnsNames: [(label: String, acquiredAt: UInt64)]
        /// DashPay profile snapshot — populated iff
        /// `IdentityEntryFFI.dashpay_profile_present == true`. `nil`
        /// means "no update for this flush", which mirrors the
        /// changeset's `dashpay_profile: None` semantics on the Rust
        /// side (NOT a delete signal). Inner fields are individually
        /// optional because every DashPay profile field but the
        /// implicit `$ownerId` is optional in the contract schema.
        let dashpayProfile: DashpayProfileSnapshot?
        /// Cached **contact** profiles for this identity — one per
        /// (re)fetched entry of the Rust `contact_profiles` map
        /// (`IdentityEntryFFI.contact_profiles`). Distinct from
        /// `dashpayProfile` (the owner's own profile): these are
        /// contacts' public profiles, keyed by the contact's identity
        /// id. Empty when no contact profile rode this flush. Each row is
        /// applied independently: a present one upserts, an
        /// `isPresent == false` tombstone deletes, and a contact simply
        /// missing from the array is left intact (no update).
        let contactProfiles: [ContactProfileSnapshot]
    }

    /// Owned snapshot of one `ContactProfileRowFFI` — the contact's
    /// identity id, the five public profile fields, and the
    /// `checked_at_ms` self-heal timestamp. Decouples every contained
    /// `String` / `Data` from the FFI heap so the callback can return
    /// immediately and the Rust side can run its free-loop. Same
    /// `*_present`-gated decode as `DashpayProfileSnapshot` plus the
    /// leading `contactIdentityId` key and trailing `checkedAtMs`.
    struct ContactProfileSnapshot {
        let contactIdentityId: Data
        /// `false` for a confirmed-absent contact (a tombstone row): the
        /// persist side emits one so the upsert can DELETE the stale row for
        /// a contact who removed their profile. `true` for a present profile
        /// (all fields below are authoritative).
        let isPresent: Bool
        let displayName: String?
        let bio: String?
        let publicMessage: String?
        let avatarUrl: String?
        /// 32-byte SHA-256 of the avatar binary. `nil` when the source
        /// `avatar_hash_present == false`.
        let avatarHash: Data?
        /// 8-byte DHash perceptual fingerprint. `nil` when the source
        /// `avatar_fingerprint_present == false`.
        let avatarFingerprint: Data?
        /// Wall-clock ms of the last fetch attempt on the Rust side
        /// (`ContactProfileEntry.checked_at_ms`).
        let checkedAtMs: UInt64
    }

    /// Owned snapshot of the `dashpay_profile_*` fields on
    /// `IdentityEntryFFI`. Decouples the lifetime of every contained
    /// `String` / `Data` from the FFI heap so the callback can
    /// return immediately and the Rust side can run its free-loop.
    struct DashpayProfileSnapshot {
        let displayName: String?
        let bio: String?
        let publicMessage: String?
        let avatarUrl: String?
        /// 32-byte SHA-256 of the avatar binary (DIP-15 `avatarHash`).
        /// `nil` when the source `avatar_hash_present == false` —
        /// disambiguates "no hash" from "all-zero hash" since the
        /// underlying byte array is zero-initialized either way.
        let avatarHash: Data?
        /// 8-byte DHash perceptual fingerprint (DIP-15
        /// `avatarFingerprint`). `nil` when the source
        /// `avatar_fingerprint_present == false`.
        let avatarFingerprint: Data?
    }

    /// Swift-side snapshot of `IdentityKeyEntryFFI` — public-key
    /// payload copied to owned `Data`, derivation breadcrumb +
    /// precomputed pubkey hash captured as scalars. Same rationale
    /// as `IdentityEntrySnapshot`: decouple lifetime from the
    /// callback window.
    struct IdentityKeyEntrySnapshot {
        let identityId: Data
        let keyId: UInt32
        let purpose: UInt8
        let securityLevel: UInt8
        let keyType: UInt8
        let readOnly: Bool
        let disabledAt: UInt64?
        let publicKeyData: Data
        let publicKeyHash: Data
        /// Owning wallet if this key is derivable from one we control.
        let walletId: Data?
        /// DIP-9 `(identity_index, key_index)` pair. Present iff the key is
        /// wallet-derivable; the client derives it on demand from the Keychain
        /// seed at this path when it needs to sign (no secret crosses the FFI).
        let derivationIndices: (identityIndex: UInt32, keyIndex: UInt32)?
        /// Full ContractBounds projection mirrored from Rust:
        /// `nil` when the key has no bounds; `.singleContract` for
        /// kind=1; `.singleContractDocumentType` for kind=2. Carried
        /// so the SwiftData row preserves the doc-type name on
        /// round-trip (would otherwise be silently downgraded to
        /// `.singleContract` and break local DPP projections that
        /// read `identity.identityPublicKeys`).
        let contractBounds: ManagedPlatformWallet.ContractBounds?
    }

    // MARK: - Watch-only Restore: Account Addresses

    /// Upsert `PersistentCoreAddress` rows for one account's address
    /// pool. Fires on wallet create (initial gap-limit fill), pool
    /// extension, and when SPV flips an address's `used` flag.
    ///
    /// Addresses are identified by their base58check string (the
    /// `@Attribute(.unique)` on `PersistentCoreAddress.address`).
    /// Parent linkage uses the same lookup key as
    /// `persistAccount(walletId:spec:)` so the row reliably maps to
    /// the right `PersistentAccount`.
    /// Returns `true` iff the addresses were durably staged (or the account is a
    /// non-security-critical type whose transient misses stay tolerant). Returns
    /// `false` ONLY when an `IdentityInvitation` (type tag 5) account is missing —
    /// that account is registered at wallet-setup and its funding-index pool is the
    /// only durable record of the one-time voucher key's derivation index, so a
    /// miss is a genuine anomaly. Signaling it drives `store() -> Err`, which makes
    /// the pre-broadcast gate abort (preventing voucher-key reuse). For every other
    /// account type the tolerant skip is kept: a transient miss during ordinary
    /// address sync (e.g. a same-round, not-yet-committed first registration) must
    /// NOT roll back and wedge the whole persistence round.
    func persistAccountAddresses(
        walletId: Data,
        accountKey: AccountLookupKey,
        entries: [CoreAddressEntrySnapshot]
    ) -> Bool {
        onQueue {
        guard let account = fetchAccount(walletId: walletId, key: accountKey) else {
            return accountKey.typeTag != Self.identityInvitationTypeTag
        }

        // DIP-17 PlatformPayment pool addresses land in
        // `PersistentPlatformAddress` so they don't share a model with
        // Core-chain (base58check) addresses.
        let isPlatformPayment = accountKey.typeTag == 14
        if isPlatformPayment {
            persistPlatformPaymentAddresses(
                account: account,
                walletId: walletId,
                entries: entries
            )
            return true
        }

        for entry in entries {
            let address = entry.address
            let row: PersistentCoreAddress
            if let existing = coreAddressRow(address: address) {
                row = existing
            } else {
                row = PersistentCoreAddress(
                    address: entry.address,
                    publicKey: entry.publicKey,
                    keyType: entry.keyType,
                    poolTypeTag: entry.poolTypeTag,
                    addressIndex: entry.addressIndex,
                    derivationPath: entry.derivationPath,
                    isUsed: entry.isUsed,
                    balance: entry.balance
                )
                backgroundContext.insert(row)
                roundIndex?.coreAddressesByAddress[address] = row
            }
            // Mutation path for both insert + update.
            row.publicKey = entry.publicKey
            row.keyType = entry.keyType
            row.poolTypeTag = entry.poolTypeTag
            row.addressIndex = entry.addressIndex
            row.derivationPath = entry.derivationPath
            row.isUsed = entry.isUsed
            row.balance = entry.balance
            row.account = account
            row.lastUpdated = Date()

            // Backfill the `coreAddress` link on any TXOs that were
            // persisted before this address row existed. The SPV
            // pass can emit UTXOs for an address whose pool row
            // hasn't landed yet; in that case `upsertUtxo` skipped
            // the relationship and `record.coreAddress` stayed nil.
            // Without this sweep the storage-explorer's "Address
            // Row" field renders as "—" forever even though the
            // address row now exists. Avoid the SwiftData
            // optional-relationship-in-predicate gotcha by
            // filtering nil-coreAddress in Swift after the fetch.
            //
            // Deliberately NOT a round-indexed store-only lookup: this
            // joins TXOs by `address`, and the rows it returns are the
            // same objects the outpoint-keyed hot path mutates — a
            // store-only fetch here would refresh those objects and
            // discard the round's unsaved writes (see `roundIndex`).
            // The pending-changes scan this keeps is bounded by the
            // round's TXO inserts per emitted address entry; the
            // outpoint-keyed quadratic hot path stays indexed.
            let txoBackfillDescriptor = FetchDescriptor<PersistentTxo>(
                predicate: #Predicate { $0.address == address }
            )
            if let txosAtAddress = try? backgroundContext.fetch(txoBackfillDescriptor) {
                for txo in txosAtAddress where txo.coreAddress == nil {
                    txo.coreAddress = row
                    // This write happened outside any keyed lookup, so
                    // register the row: a later first-touch
                    // `fetchTxoRow` for this outpoint would otherwise
                    // run a store-only fetch and refresh the link away
                    // (see `roundIndex`).
                    roundIndex?.txosByOutpoint[txo.outpoint] = txo
                }
            }
        }

        if !self.inChangeset { try? backgroundContext.save() }
        return true
        }  // onQueue
    }

    /// The `AccountTypeTagFFI::IdentityInvitation` discriminant. The invitation
    /// funding pool is the only durable record of the one-time voucher key's
    /// derivation index, so its persistence is treated as a hard gate (see
    /// `persistAccountAddresses`), unlike every other account type.
    private static let identityInvitationTypeTag: UInt32 = 5

    /// Upsert PlatformPayment entries into `PersistentPlatformAddress`.
    /// Called only when the address-emit target account is a DIP-17
    /// PlatformPayment account (type tag 14). The Rust side emits the
    /// DIP-0018 bech32m form in `entry.address`; we derive the
    /// 20-byte hash + address type here so BLAST balance updates
    /// (which arrive with `addressHash` only) can upsert the same row.
    private func persistPlatformPaymentAddresses(
        account: PersistentAccount,
        walletId: Data,
        entries: [CoreAddressEntrySnapshot]
    ) {
        for entry in entries {
            guard let (addressType, addressHash) =
                platformAddressComponents(fromBech32m: entry.address)
            else {
                continue
            }
            let address = entry.address
            let descriptor = FetchDescriptor<PersistentPlatformAddress>(
                predicate: #Predicate { $0.address == address }
            )
            let row: PersistentPlatformAddress
            if let existing = try? backgroundContext.fetch(descriptor).first {
                row = existing
                row.addressType = addressType
                row.addressHash = addressHash
            } else {
                row = PersistentPlatformAddress(
                    address: entry.address,
                    addressType: addressType,
                    addressHash: addressHash,
                    publicKey: entry.publicKey,
                    accountIndex: account.accountIndex,
                    addressIndex: entry.addressIndex,
                    derivationPath: entry.derivationPath,
                    isUsed: entry.isUsed,
                    balance: entry.balance,
                    nonce: 0,
                    walletId: walletId
                )
                backgroundContext.insert(row)
            }
            // Address-emit is authoritative for derivation metadata
            // and the used flag on first creation; we preserve any
            // later BLAST-driven balance/nonce updates by only
            // lowering `isUsed` if the emit says so explicitly and
            // we don't already have funds showing.
            row.publicKey = entry.publicKey
            row.accountIndex = account.accountIndex
            row.addressIndex = entry.addressIndex
            row.derivationPath = entry.derivationPath
            if entry.isUsed {
                row.isUsed = true
            } else if row.balance == 0 && row.nonce == 0 {
                row.isUsed = false
            }
            if row.balance == 0 && entry.balance != 0 {
                row.balance = entry.balance
            }
            row.account = account
            row.walletId = walletId
            row.lastUpdated = Date()
        }

        if !self.inChangeset { try? backgroundContext.save() }
    }

    /// Split a DIP-0018 bech32m platform address back into
    /// `(addressType, 20-byte hash)`. Returns nil on any decode
    /// failure or unexpected type byte. Type bytes follow
    /// DIP-0018: `0xb0` → P2PKH (stored as 0), `0x80` → P2SH
    /// (stored as 1).
    private func platformAddressComponents(
        fromBech32m address: String
    ) -> (addressType: UInt8, hash: Data)? {
        guard let decoded = Bech32m.decode(address.lowercased()),
              decoded.hrp == "dash" || decoded.hrp == "tdash",
              decoded.data.count == 21
        else {
            return nil
        }
        let typeByte = decoded.data[0]
        let hash = decoded.data.subdata(in: 1..<21)
        switch typeByte {
        case 0xb0: return (0, hash)
        case 0x80: return (1, hash)
        default: return nil
        }
    }

    /// Lookup key mirroring the identifying subset of
    /// `AccountSpecFFI` so the handler can locate the
    /// `PersistentAccount` row for address linkage. `standardTag` is
    /// included because a wallet can have both BIP44 (tag 0) and
    /// BIP32 (tag 1) Standard accounts at the same index — without
    /// disambiguating on `standardTag`, BIP32 addresses would be
    /// routed to the BIP44 row.
    struct AccountLookupKey {
        let typeTag: UInt32
        let index: UInt32
        let standardTag: UInt8
        let registrationIndex: UInt32
        let keyClass: UInt32
        let userIdentityId: Data
        let friendIdentityId: Data
    }

    /// Snapshot of a `CoreAddressEntryFFI` with the C strings copied
    /// into Swift Strings so the callback can return before we touch
    /// the data.
    struct CoreAddressEntrySnapshot {
        let address: String
        let publicKey: Data
        /// `KeyTypeTagFFI` raw value (0 ECDSA / 1 BLS / 2 EdDSA);
        /// meaningful only when `publicKey` is non-empty.
        let keyType: UInt8
        let poolTypeTag: UInt8
        let addressIndex: UInt32
        let isUsed: Bool
        let balance: UInt64
        let derivationPath: String
    }

    private func fetchAccount(
        walletId: Data,
        key: AccountLookupKey
    ) -> PersistentAccount? {
        let typeTag = key.typeTag
        let index = key.index
        let descriptor = FetchDescriptor<PersistentAccount>(
            predicate: #Predicate {
                $0.wallet.walletId == walletId
                    && $0.accountType == typeTag
                    && $0.accountIndex == index
            }
        )
        let matches = (try? backgroundContext.fetch(descriptor)) ?? []
        return matches.first { acc in
            acc.standardTag == key.standardTag
                && acc.registrationIndex == key.registrationIndex
                && acc.keyClass == key.keyClass
                && acc.userIdentityId == key.userIdentityId
                && acc.friendIdentityId == key.friendIdentityId
        }
    }

    // MARK: - Watch-only Restore: Wallet Metadata

    // MARK: - Shielded persistence (Orchard)

    /// One incoming shielded-note row from
    /// `ShieldedChangeSet::notes_saved`. Decoupled from
    /// `ShieldedNoteFFI` so the trampoline can copy bytes out
    /// before this method runs on `onQueue`.
    struct ShieldedNoteSnapshot {
        let walletId: Data
        let accountIndex: UInt32
        let position: UInt64
        let cmx: Data
        let nullifier: Data
        let blockHeight: UInt64
        let isSpent: Bool
        let value: UInt64
        let noteData: Data
    }

    /// Upsert a batch of decrypted shielded notes by `nullifier`.
    /// Re-saves with the same nullifier overwrite the existing
    /// row in place — Orchard nullifiers are globally unique.
    func persistShieldedNotes(walletId: Data, snapshots: [ShieldedNoteSnapshot]) {
        onQueue {
            for snap in snapshots {
                let nf = snap.nullifier
                let predicate = #Predicate<PersistentShieldedNote> { $0.nullifier == nf }
                var descriptor = FetchDescriptor<PersistentShieldedNote>(predicate: predicate)
                descriptor.fetchLimit = 1
                if let existing = try? backgroundContext.fetch(descriptor).first {
                    existing.walletId = snap.walletId
                    existing.accountIndex = snap.accountIndex
                    existing.position = snap.position
                    existing.cmx = snap.cmx
                    existing.blockHeight = snap.blockHeight
                    existing.isSpent = snap.isSpent
                    existing.value = snap.value
                    existing.noteData = snap.noteData
                    existing.lastUpdated = Date()
                } else {
                    let row = PersistentShieldedNote(
                        walletId: snap.walletId,
                        accountIndex: snap.accountIndex,
                        position: snap.position,
                        cmx: snap.cmx,
                        nullifier: snap.nullifier,
                        blockHeight: snap.blockHeight,
                        isSpent: snap.isSpent,
                        value: snap.value,
                        noteData: snap.noteData
                    )
                    backgroundContext.insert(row)
                }
            }
            if !self.inChangeset { try? backgroundContext.save() }
        }
    }

    /// One outgoing (sent) shielded-note row from
    /// `ShieldedChangeSet::outgoing_notes`. Decoupled from
    /// `ShieldedOutgoingNoteFFI` so the trampoline can copy the
    /// `recipient` / `memo` bytes out before this method runs on
    /// `onQueue` (the Rust pointers are only valid for the callback
    /// window).
    struct ShieldedOutgoingNoteSnapshot {
        let walletId: Data
        let accountIndex: UInt32
        let cmx: Data
        let recipient: Data
        let value: UInt64
        let memo: Data
        let blockHeight: UInt64
    }

    /// Upsert a batch of OVK-recovered outgoing (sent) notes by
    /// `(walletId, accountIndex, cmx)`. Append-only send history with
    /// no spend / nullifier state; re-persisting the same `cmx`
    /// (a re-scan) overwrites the existing row in place.
    func persistShieldedOutgoingNotes(walletId: Data, snapshots: [ShieldedOutgoingNoteSnapshot]) {
        onQueue {
            for snap in snapshots {
                let wid = snap.walletId
                let acct = snap.accountIndex
                let cmx = snap.cmx
                let predicate = #Predicate<PersistentShieldedOutgoingNote> {
                    $0.walletId == wid && $0.accountIndex == acct && $0.cmx == cmx
                }
                var descriptor = FetchDescriptor<PersistentShieldedOutgoingNote>(
                    predicate: predicate
                )
                descriptor.fetchLimit = 1
                if let existing = try? backgroundContext.fetch(descriptor).first {
                    existing.recipient = snap.recipient
                    existing.value = snap.value
                    existing.memo = snap.memo
                    existing.blockHeight = snap.blockHeight
                    existing.lastUpdated = Date()
                } else {
                    let row = PersistentShieldedOutgoingNote(
                        walletId: snap.walletId,
                        accountIndex: snap.accountIndex,
                        cmx: snap.cmx,
                        recipient: snap.recipient,
                        value: snap.value,
                        memo: snap.memo,
                        blockHeight: snap.blockHeight
                    )
                    backgroundContext.insert(row)
                }
            }
            if !self.inChangeset { try? backgroundContext.save() }
        }
    }

    /// One derived activity-log row from
    /// `ShieldedChangeSet::activity_entries`. Decoupled from
    /// `ShieldedActivityFFI` so the trampoline copies the pointer-backed
    /// fields out before this runs on `onQueue` (the Rust pointers are
    /// only valid for the callback window).
    struct ShieldedActivitySnapshot {
        let walletId: Data
        let accountIndex: UInt32
        let entryId: Data
        let kindTag: Int
        let direction: Int
        let status: Int
        let amount: UInt64
        let fee: UInt64
        let hasFee: Bool
        let blockHeight: UInt64
        let hasBlockHeight: Bool
        let createdAtMs: UInt64
        /// Chain-order key (commitment-tree position) when
        /// `hasMinNotePosition` — orders scan-derived restored entries
        /// whose date/height are unknown. See `PersistentShieldedActivity`.
        let minNotePosition: UInt64
        let hasMinNotePosition: Bool
        let identityId: Data
        let counterparty: Data
        let memo: Data
        let noteCmxs: Data
        let spentNullifiers: Data
    }

    /// Upsert a batch of derived activity entries by
    /// `(walletId, accountIndex, entryId)`. Re-persisting the same
    /// `entryId` refines the existing row in place: a `Pending` row flips
    /// to `Confirmed`/`Failed`, and a scan-derived `ShieldedSpend` can be
    /// upgraded to a richer kind (the Rust side re-emits the same id).
    func persistShieldedActivity(walletId: Data, snapshots: [ShieldedActivitySnapshot]) {
        onQueue {
            for snap in snapshots {
                let wid = snap.walletId
                let acct = snap.accountIndex
                let eid = snap.entryId
                let predicate = #Predicate<PersistentShieldedActivity> {
                    $0.walletId == wid && $0.accountIndex == acct && $0.entryId == eid
                }
                var descriptor = FetchDescriptor<PersistentShieldedActivity>(predicate: predicate)
                descriptor.fetchLimit = 1
                if let existing = try? backgroundContext.fetch(descriptor).first {
                    existing.kindTag = snap.kindTag
                    existing.direction = snap.direction
                    existing.status = snap.status
                    existing.amount = snap.amount
                    existing.fee = snap.fee
                    existing.hasFee = snap.hasFee
                    existing.blockHeight = snap.blockHeight
                    existing.hasBlockHeight = snap.hasBlockHeight
                    existing.createdAtMs = snap.createdAtMs
                    existing.minNotePosition = snap.minNotePosition
                    existing.hasMinNotePosition = snap.hasMinNotePosition
                    existing.identityId = snap.identityId
                    existing.counterparty = snap.counterparty
                    existing.memo = snap.memo
                    existing.noteCmxs = snap.noteCmxs
                    existing.spentNullifiers = snap.spentNullifiers
                    existing.lastUpdated = Date()
                } else {
                    let row = PersistentShieldedActivity(
                        walletId: snap.walletId,
                        accountIndex: snap.accountIndex,
                        entryId: snap.entryId,
                        kindTag: snap.kindTag,
                        direction: snap.direction,
                        status: snap.status,
                        amount: snap.amount,
                        fee: snap.fee,
                        hasFee: snap.hasFee,
                        blockHeight: snap.blockHeight,
                        hasBlockHeight: snap.hasBlockHeight,
                        createdAtMs: snap.createdAtMs,
                        minNotePosition: snap.minNotePosition,
                        hasMinNotePosition: snap.hasMinNotePosition,
                        identityId: snap.identityId,
                        counterparty: snap.counterparty,
                        memo: snap.memo,
                        noteCmxs: snap.noteCmxs,
                        spentNullifiers: snap.spentNullifiers
                    )
                    backgroundContext.insert(row)
                }
            }
            if !self.inChangeset { try? backgroundContext.save() }
        }
    }

    /// Mark notes as spent by nullifier.
    func persistShieldedNullifiersSpent(
        walletId: Data,
        entries: [(walletId: Data, accountIndex: UInt32, nullifier: Data)]
    ) {
        onQueue {
            for entry in entries {
                let nf = entry.nullifier
                let predicate = #Predicate<PersistentShieldedNote> { $0.nullifier == nf }
                var descriptor = FetchDescriptor<PersistentShieldedNote>(predicate: predicate)
                descriptor.fetchLimit = 1
                if let row = try? backgroundContext.fetch(descriptor).first {
                    if !row.isSpent {
                        row.isSpent = true
                        row.lastUpdated = Date()
                    }
                }
            }
            if !self.inChangeset { try? backgroundContext.save() }
        }
    }

    /// Upsert per-subwallet sync watermarks.
    func persistShieldedSyncedIndices(
        walletId: Data,
        entries: [(walletId: Data, accountIndex: UInt32, lastSyncedIndex: UInt64)]
    ) {
        onQueue {
            for entry in entries {
                let row = ensureShieldedSyncStateRow(
                    walletId: entry.walletId,
                    accountIndex: entry.accountIndex
                )
                if entry.lastSyncedIndex > row.lastSyncedIndex {
                    row.lastSyncedIndex = entry.lastSyncedIndex
                }
                row.lastUpdated = Date()
            }
            if !self.inChangeset { try? backgroundContext.save() }
        }
    }

    /// Upsert per-subwallet Orchard viewing keys (raw 96-byte FVK
    /// encodings). Fired once per seed-backed bind; the FVK for a
    /// subwallet never changes on a network, so re-persists are
    /// byte-identical upserts.
    func persistShieldedViewingKeys(
        walletId: Data,
        entries: [(walletId: Data, accountIndex: UInt32, fvkBytes: Data)]
    ) {
        onQueue {
            for entry in entries {
                guard entry.fvkBytes.count == 96 else { continue }
                let rowWalletId = entry.walletId
                let rowAccountIndex = entry.accountIndex
                let predicate = #Predicate<PersistentShieldedViewingKey> { row in
                    row.walletId == rowWalletId && row.accountIndex == rowAccountIndex
                }
                var descriptor = FetchDescriptor<PersistentShieldedViewingKey>(
                    predicate: predicate
                )
                descriptor.fetchLimit = 1
                if let row = try? backgroundContext.fetch(descriptor).first {
                    if row.fvkBytes != entry.fvkBytes {
                        row.fvkBytes = entry.fvkBytes
                        row.lastUpdated = Date()
                    }
                } else {
                    backgroundContext.insert(
                        PersistentShieldedViewingKey(
                            walletId: rowWalletId,
                            accountIndex: rowAccountIndex,
                            fvkBytes: entry.fvkBytes
                        )
                    )
                }
            }
            if !self.inChangeset { try? backgroundContext.save() }
        }
    }

    /// Fetch-or-create a `PersistentShieldedSyncState` row for
    /// `(walletId, accountIndex)`. Caller must be on `onQueue`.
    private func ensureShieldedSyncStateRow(
        walletId: Data,
        accountIndex: UInt32
    ) -> PersistentShieldedSyncState {
        let predicate = #Predicate<PersistentShieldedSyncState> { row in
            row.walletId == walletId && row.accountIndex == accountIndex
        }
        var descriptor = FetchDescriptor<PersistentShieldedSyncState>(predicate: predicate)
        descriptor.fetchLimit = 1
        if let row = try? backgroundContext.fetch(descriptor).first {
            return row
        }
        let row = PersistentShieldedSyncState(
            walletId: walletId,
            accountIndex: accountIndex
        )
        backgroundContext.insert(row)
        return row
    }

    /// Wallet ids belonging to the handler's bound network, used to
    /// scope the shielded loaders the same way `loadWalletList()` scopes
    /// its wallet fetch. Returns `nil` when the handler has no bound
    /// network (legacy callers that haven't threaded `network` through),
    /// signalling "don't filter" so those paths keep their pre-refactor
    /// cross-network behavior. Caller must be on `onQueue`.
    private func inNetworkWalletIds() -> Set<Data>? {
        guard let network = self.network else { return nil }
        let raw = network.rawValue
        let descriptor = FetchDescriptor<PersistentWallet>(
            predicate: #Predicate { $0.networkRaw == raw }
        )
        let wallets = (try? backgroundContext.fetch(descriptor)) ?? []
        return Set(wallets.map { $0.walletId })
    }

    /// Build the host-allocated `ShieldedNoteRestoreFFI` array Rust
    /// reads at boot. The allocation is tracked in
    /// `shieldedLoadAllocations` and freed by
    /// `loadShieldedNotesFree` once Rust hands the pointer back.
    func loadShieldedNotes() -> (
        entries: UnsafePointer<ShieldedNoteRestoreFFI>?,
        count: Int,
        errored: Bool
    ) {
        var resultEntries: UnsafePointer<ShieldedNoteRestoreFFI>?
        var resultCount: Int = 0
        var resultErrored = false
        onQueue {
            let descriptor = FetchDescriptor<PersistentShieldedNote>()
            var rows: [PersistentShieldedNote]
            do {
                rows = try backgroundContext.fetch(descriptor)
            } catch {
                resultErrored = true
                return
            }
            // Scope to the handler's bound network so a per-network
            // manager never rehydrates another network's shielded notes
            // (the commitment tree DB is network-scoped). `nil` ids =>
            // no in-network wallets => nothing to restore.
            if let inNetworkIds = self.inNetworkWalletIds() {
                rows = rows.filter { inNetworkIds.contains($0.walletId) }
            }
            if rows.isEmpty {
                return
            }
            let allocation = ShieldedLoadAllocation()
            // Allocate the entries buffer up front; populate slots
            // one by one and track `entriesInitialized` so a
            // mid-loop bail-out can deinit only the populated
            // slots. (Today nothing fails in this loop, but
            // matching the existing `LoadAllocation` pattern keeps
            // future field additions safe.)
            let buf = UnsafeMutablePointer<ShieldedNoteRestoreFFI>.allocate(capacity: rows.count)
            allocation.entries = buf
            allocation.entriesCount = rows.count
            // `written` is the next free slot in `buf`; we increment it
            // only after a row's struct is fully populated, so the
            // returned prefix `[0..written)` is contiguous initialized
            // memory regardless of how many rows are skipped by the
            // length guards below. Indexing by `rows.enumerated()`'s
            // `idx` here would leave gaps when an early row is skipped
            // and Rust would read uninitialized bytes off the
            // `slice::from_raw_parts(ptr, count)` it builds in
            // `FFIPersister::load`.
            var written = 0
            for row in rows {
                guard row.walletId.count == 32 else { continue }
                guard row.cmx.count == 32 else { continue }
                guard row.nullifier.count == 32 else { continue }
                let noteDataBuf = UnsafeMutablePointer<UInt8>.allocate(capacity: row.noteData.count)
                row.noteData.copyBytes(to: noteDataBuf, count: row.noteData.count)
                allocation.scalarBuffers.append((noteDataBuf, row.noteData.count))

                var walletIdTuple: FFIByteTuple32 = (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
                row.walletId.withUnsafeBytes { src in
                    Swift.withUnsafeMutableBytes(of: &walletIdTuple) { dst in
                        dst.copyMemory(from: src)
                    }
                }
                var cmxTuple: FFIByteTuple32 = (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
                row.cmx.withUnsafeBytes { src in
                    Swift.withUnsafeMutableBytes(of: &cmxTuple) { dst in
                        dst.copyMemory(from: src)
                    }
                }
                var nullifierTuple: FFIByteTuple32 = (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
                row.nullifier.withUnsafeBytes { src in
                    Swift.withUnsafeMutableBytes(of: &nullifierTuple) { dst in
                        dst.copyMemory(from: src)
                    }
                }
                buf[written] = ShieldedNoteRestoreFFI(
                    wallet_id: walletIdTuple,
                    account_index: row.accountIndex,
                    position: row.position,
                    cmx: cmxTuple,
                    nullifier: nullifierTuple,
                    block_height: row.blockHeight,
                    is_spent: row.isSpent ? 1 : 0,
                    value: row.value,
                    note_data_ptr: UnsafePointer(noteDataBuf),
                    note_data_len: UInt(row.noteData.count)
                )
                written += 1
                allocation.entriesInitialized = written
            }
            let entriesPtr = UnsafePointer(buf)
            shieldedLoadAllocations[UnsafeRawPointer(entriesPtr)] = allocation
            resultEntries = entriesPtr
            resultCount = written
        }
        return (resultEntries, resultCount, resultErrored)
    }

    func loadShieldedNotesFree(entries: UnsafeRawPointer?) {
        onQueue {
            guard let entries = entries,
                  let allocation = shieldedLoadAllocations.removeValue(forKey: entries) else {
                return
            }
            allocation.release()
        }
    }

    /// Build the host-allocated `ShieldedOutgoingNoteRestoreFFI`
    /// array Rust reads at boot. Same allocation pattern as
    /// `loadShieldedNotes` — the entries buffer plus a per-row
    /// heap `memo` byte buffer each entry's `memo_ptr` references.
    /// Tracked in `shieldedOutgoingNoteLoadAllocations` and freed by
    /// `loadShieldedOutgoingNotesFree` once Rust hands the pointer
    /// back.
    func loadShieldedOutgoingNotes() -> (
        entries: UnsafePointer<ShieldedOutgoingNoteRestoreFFI>?,
        count: Int,
        errored: Bool
    ) {
        var resultEntries: UnsafePointer<ShieldedOutgoingNoteRestoreFFI>?
        var resultCount: Int = 0
        var resultErrored = false
        onQueue {
            let descriptor = FetchDescriptor<PersistentShieldedOutgoingNote>()
            var rows: [PersistentShieldedOutgoingNote]
            do {
                rows = try backgroundContext.fetch(descriptor)
            } catch {
                resultErrored = true
                return
            }
            // Scope to the handler's bound network so a per-network
            // manager never rehydrates another network's send history
            // (the commitment tree DB is network-scoped). `nil` ids =>
            // no in-network wallets => nothing to restore.
            if let inNetworkIds = self.inNetworkWalletIds() {
                rows = rows.filter { inNetworkIds.contains($0.walletId) }
            }
            if rows.isEmpty {
                return
            }
            let allocation = ShieldedOutgoingNoteLoadAllocation()
            let buf = UnsafeMutablePointer<ShieldedOutgoingNoteRestoreFFI>.allocate(
                capacity: rows.count
            )
            allocation.entries = buf
            allocation.entriesCount = rows.count
            // Same `written`-counter discipline as `loadShieldedNotes`:
            // increment only after a slot is fully populated so the
            // returned prefix `[0..written)` is contiguous initialized
            // memory even when malformed rows are skipped.
            var written = 0
            for row in rows {
                guard row.walletId.count == 32 else { continue }
                guard row.cmx.count == 32 else { continue }
                // `recipient` is a fixed 43-byte raw Orchard address.
                // A wrong-length blob is a corrupt row — skip it (with
                // a log) rather than zero-padding it into a wrong
                // address. Mirrors the Rust persist side, which rejects
                // non-43-byte recipients before they reach SwiftData.
                guard row.recipient.count == 43 else {
                    print("⚠️ loadShieldedOutgoingNotes: skipping row with malformed recipient length \(row.recipient.count) (expected 43)")
                    continue
                }
                let memoBuf = UnsafeMutablePointer<UInt8>.allocate(capacity: row.memo.count)
                if row.memo.count > 0 {
                    row.memo.copyBytes(to: memoBuf, count: row.memo.count)
                }
                allocation.scalarBuffers.append((memoBuf, row.memo.count))

                var walletIdTuple: FFIByteTuple32 = (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
                copyBytes(row.walletId, into: &walletIdTuple)
                var cmxTuple: FFIByteTuple32 = (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
                copyBytes(row.cmx, into: &cmxTuple)
                // `recipient` is a 43-byte raw Orchard address; the C
                // field imports as a 43-element tuple. The length guard
                // above guarantees exactly 43 bytes, so this is a full
                // copy via the shared fixed-tuple writer.
                var recipientTuple: FFIByteTuple43 = ffiByteTuple43Zero
                copyBytes(row.recipient, into: &recipientTuple)
                buf[written] = ShieldedOutgoingNoteRestoreFFI(
                    wallet_id: walletIdTuple,
                    account_index: row.accountIndex,
                    cmx: cmxTuple,
                    recipient: recipientTuple,
                    value: row.value,
                    block_height: row.blockHeight,
                    memo_ptr: UnsafePointer(memoBuf),
                    memo_len: UInt(row.memo.count)
                )
                written += 1
                allocation.entriesInitialized = written
            }
            let entriesPtr = UnsafePointer(buf)
            shieldedOutgoingNoteLoadAllocations[UnsafeRawPointer(entriesPtr)] = allocation
            resultEntries = entriesPtr
            resultCount = written
        }
        return (resultEntries, resultCount, resultErrored)
    }

    func loadShieldedOutgoingNotesFree(entries: UnsafeRawPointer?) {
        onQueue {
            guard let entries = entries,
                  let allocation = shieldedOutgoingNoteLoadAllocations.removeValue(forKey: entries)
            else {
                return
            }
            allocation.release()
        }
    }

    /// Build the host-allocated `ShieldedActivityRestoreFFI` array Rust
    /// reads at boot so the scan deriver's dedupe set includes every
    /// persisted entry (a rich live entry is never clobbered by a
    /// re-derivation). Same allocation / `written`-counter discipline as
    /// `loadShieldedOutgoingNotes`; each row's four pointer-backed fields
    /// (counterparty / memo / cmx + nullifier arrays) reference per-row
    /// byte buffers tracked in the allocation.
    func loadShieldedActivity() -> (
        entries: UnsafePointer<ShieldedActivityRestoreFFI>?,
        count: Int,
        errored: Bool
    ) {
        var resultEntries: UnsafePointer<ShieldedActivityRestoreFFI>?
        var resultCount: Int = 0
        var resultErrored = false
        onQueue {
            let descriptor = FetchDescriptor<PersistentShieldedActivity>()
            var rows: [PersistentShieldedActivity]
            do {
                rows = try backgroundContext.fetch(descriptor)
            } catch {
                resultErrored = true
                return
            }
            if let inNetworkIds = self.inNetworkWalletIds() {
                rows = rows.filter { inNetworkIds.contains($0.walletId) }
            }
            if rows.isEmpty {
                return
            }
            let allocation = ShieldedActivityLoadAllocation()
            let buf = UnsafeMutablePointer<ShieldedActivityRestoreFFI>.allocate(
                capacity: rows.count
            )
            allocation.entries = buf
            allocation.entriesCount = rows.count
            var written = 0
            for row in rows {
                guard row.walletId.count == 32, row.entryId.count == 32 else { continue }
                // Exact conversion, not truncating: a stored tag outside
                // u8 range (corruption / unmigrated future tag) must NOT
                // wrap into a different valid-looking discriminant (256 →
                // 0 = Shield/In/Pending) — that would bypass Rust's
                // unknown-tag fallback. Drop the row instead.
                guard let kindTagU8 = UInt8(exactly: row.kindTag),
                      let directionU8 = UInt8(exactly: row.direction),
                      let statusU8 = UInt8(exactly: row.status) else {
                    continue
                }

                // Per-row variable-length buffers: counterparty, memo,
                // noteCmxs, spentNullifiers. Allocate each and retain a
                // pointer in `scalarBuffers` so `release()` frees them.
                func makeBuffer(_ data: Data) -> (UnsafeMutablePointer<UInt8>, Int) {
                    let n = data.count
                    let p = UnsafeMutablePointer<UInt8>.allocate(capacity: max(n, 1))
                    if n > 0 { data.copyBytes(to: p, count: n) }
                    allocation.scalarBuffers.append((p, n))
                    return (p, n)
                }
                let (cpPtr, cpLen) = makeBuffer(row.counterparty)
                let (memoPtr, memoLen) = makeBuffer(row.memo)
                let (cmxPtr, cmxLen) = makeBuffer(row.noteCmxs)
                let (nfPtr, nfLen) = makeBuffer(row.spentNullifiers)

                var walletIdTuple: FFIByteTuple32 = (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
                copyBytes(row.walletId, into: &walletIdTuple)
                var entryIdTuple: FFIByteTuple32 = (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
                copyBytes(row.entryId, into: &entryIdTuple)
                var identityTuple: FFIByteTuple32 = (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
                if row.identityId.count == 32 {
                    copyBytes(row.identityId, into: &identityTuple)
                }
                buf[written] = ShieldedActivityRestoreFFI(
                    wallet_id: walletIdTuple,
                    account_index: row.accountIndex,
                    entry_id: entryIdTuple,
                    kind_tag: kindTagU8,
                    direction: directionU8,
                    status: statusU8,
                    amount: row.amount,
                    fee: row.fee,
                    has_fee: row.hasFee ? 1 : 0,
                    block_height: row.blockHeight,
                    has_block_height: row.hasBlockHeight ? 1 : 0,
                    created_at_ms: row.createdAtMs,
                    min_note_position: row.minNotePosition,
                    has_min_note_position: row.hasMinNotePosition ? 1 : 0,
                    identity_id: identityTuple,
                    has_identity_id: row.identityId.count == 32 ? 1 : 0,
                    counterparty_ptr: cpLen > 0 ? UnsafePointer(cpPtr) : nil,
                    counterparty_len: UInt(cpLen),
                    memo_ptr: memoLen > 0 ? UnsafePointer(memoPtr) : nil,
                    memo_len: UInt(memoLen),
                    // A persisted blob that isn't a whole number of
                    // 32-byte elements is corrupt — drop the linkage
                    // (count 0, null ptr) rather than silently truncating
                    // trailing bytes into a wrong-but-plausible array.
                    note_cmxs_ptr: cmxLen > 0 && cmxLen % 32 == 0 ? UnsafePointer(cmxPtr) : nil,
                    note_cmxs_count: cmxLen % 32 == 0 ? UInt(cmxLen / 32) : 0,
                    spent_nullifiers_ptr: nfLen > 0 && nfLen % 32 == 0 ? UnsafePointer(nfPtr) : nil,
                    spent_nullifiers_count: nfLen % 32 == 0 ? UInt(nfLen / 32) : 0
                )
                written += 1
                allocation.entriesInitialized = written
            }
            let entriesPtr = UnsafePointer(buf)
            shieldedActivityLoadAllocations[UnsafeRawPointer(entriesPtr)] = allocation
            resultEntries = entriesPtr
            resultCount = written
        }
        return (resultEntries, resultCount, resultErrored)
    }

    func loadShieldedActivityFree(entries: UnsafeRawPointer?) {
        onQueue {
            guard let entries = entries,
                  let allocation = shieldedActivityLoadAllocations.removeValue(forKey: entries)
            else {
                return
            }
            allocation.release()
        }
    }

    /// Build the host-allocated `ShieldedSubwalletSyncStateFFI`
    /// array Rust reads at boot. Same allocation pattern as
    /// `loadShieldedNotes`.
    func loadShieldedSyncStates() -> (
        entries: UnsafePointer<ShieldedSubwalletSyncStateFFI>?,
        count: Int,
        errored: Bool
    ) {
        var resultEntries: UnsafePointer<ShieldedSubwalletSyncStateFFI>?
        var resultCount: Int = 0
        var resultErrored = false
        onQueue {
            let descriptor = FetchDescriptor<PersistentShieldedSyncState>()
            var rows: [PersistentShieldedSyncState]
            do {
                rows = try backgroundContext.fetch(descriptor)
            } catch {
                resultErrored = true
                return
            }
            // Same network scoping as `loadShieldedNotes` — keep both
            // loaders consistent so a per-network manager doesn't restore
            // foreign-network sync watermarks.
            if let inNetworkIds = self.inNetworkWalletIds() {
                rows = rows.filter { inNetworkIds.contains($0.walletId) }
            }
            if rows.isEmpty {
                return
            }
            let allocation = ShieldedSyncStateLoadAllocation()
            let buf = UnsafeMutablePointer<ShieldedSubwalletSyncStateFFI>.allocate(
                capacity: rows.count
            )
            allocation.entries = buf
            allocation.entriesCount = rows.count
            // Same `written`-counter pattern as `loadShieldedNotes`:
            // skip malformed rows without leaving holes in the
            // contiguous prefix Rust will read.
            var written = 0
            for row in rows {
                guard row.walletId.count == 32 else { continue }
                var walletIdTuple: FFIByteTuple32 = (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
                row.walletId.withUnsafeBytes { src in
                    Swift.withUnsafeMutableBytes(of: &walletIdTuple) { dst in
                        dst.copyMemory(from: src)
                    }
                }
                buf[written] = ShieldedSubwalletSyncStateFFI(
                    wallet_id: walletIdTuple,
                    account_index: row.accountIndex,
                    last_synced_index: row.lastSyncedIndex
                )
                written += 1
                allocation.entriesInitialized = written
            }
            let entriesPtr = UnsafePointer(buf)
            shieldedSyncStateLoadAllocations[UnsafeRawPointer(entriesPtr)] = allocation
            resultEntries = entriesPtr
            resultCount = written
        }
        return (resultEntries, resultCount, resultErrored)
    }

    func loadShieldedSyncStatesFree(entries: UnsafeRawPointer?) {
        onQueue {
            guard let entries = entries,
                  let allocation = shieldedSyncStateLoadAllocations.removeValue(forKey: entries)
            else {
                return
            }
            allocation.release()
        }
    }

    /// Build the host-allocated `ShieldedViewingKeyRestoreFFI` array
    /// Rust reads at boot so `bind_shielded_from_persisted` can rebind
    /// without a mnemonic resolve. Same allocation pattern (and
    /// network scoping) as `loadShieldedSyncStates`.
    func loadShieldedViewingKeys() -> (
        entries: UnsafePointer<ShieldedViewingKeyRestoreFFI>?,
        count: Int,
        errored: Bool
    ) {
        var resultEntries: UnsafePointer<ShieldedViewingKeyRestoreFFI>?
        var resultCount: Int = 0
        var resultErrored = false
        onQueue {
            let descriptor = FetchDescriptor<PersistentShieldedViewingKey>()
            var rows: [PersistentShieldedViewingKey]
            do {
                rows = try backgroundContext.fetch(descriptor)
            } catch {
                resultErrored = true
                return
            }
            // Same network scoping as the other shielded loaders — the
            // FVK embeds the coin type, so serving another network's
            // rows would fail the Rust-side bind, not corrupt it, but
            // the loaders stay consistent regardless.
            if let inNetworkIds = self.inNetworkWalletIds() {
                rows = rows.filter { inNetworkIds.contains($0.walletId) }
            }
            if rows.isEmpty {
                return
            }
            // Fail closed on a present-but-malformed row, BEFORE any
            // allocation: silently skipping it (the sync-state
            // loader's pattern) would make Rust see the account as
            // "no persisted key" and fall back to a mnemonic resolve,
            // masking persistence corruption — the exact opposite of
            // `bind_shielded_from_persisted`'s documented contract,
            // which surfaces a malformed row as an error.
            if let bad = rows.first(where: {
                $0.walletId.count != 32 || $0.fvkBytes.count != 96
            }) {
                SDKLogger.error(
                    "loadShieldedViewingKeys: corrupt row "
                        + "(walletId \(bad.walletId.count)B, fvk \(bad.fvkBytes.count)B) — "
                        + "failing the load rather than masking it as a missing key"
                )
                resultErrored = true
                return
            }
            let allocation = ShieldedViewingKeyLoadAllocation()
            let buf = UnsafeMutablePointer<ShieldedViewingKeyRestoreFFI>.allocate(
                capacity: rows.count
            )
            allocation.entries = buf
            allocation.entriesCount = rows.count
            var written = 0
            for row in rows {
                var entry = ShieldedViewingKeyRestoreFFI()
                Swift.withUnsafeMutableBytes(of: &entry.wallet_id) { dst in
                    row.walletId.withUnsafeBytes { dst.copyMemory(from: $0) }
                }
                entry.account_index = row.accountIndex
                Swift.withUnsafeMutableBytes(of: &entry.fvk_bytes) { dst in
                    row.fvkBytes.withUnsafeBytes { dst.copyMemory(from: $0) }
                }
                buf[written] = entry
                written += 1
                allocation.entriesInitialized = written
            }
            let entriesPtr = UnsafePointer(buf)
            shieldedViewingKeyLoadAllocations[UnsafeRawPointer(entriesPtr)] = allocation
            resultEntries = entriesPtr
            resultCount = written
        }
        return (resultEntries, resultCount, resultErrored)
    }

    func loadShieldedViewingKeysFree(entries: UnsafeRawPointer?) {
        onQueue {
            guard let entries = entries,
                  let allocation = shieldedViewingKeyLoadAllocations.removeValue(forKey: entries)
            else {
                return
            }
            allocation.release()
        }
    }

    /// Outstanding shielded-load allocations keyed by the entries
    /// pointer we handed Rust. Drained by `loadShieldedNotesFree`.
    private var shieldedLoadAllocations: [UnsafeRawPointer: ShieldedLoadAllocation] = [:]
    private var shieldedOutgoingNoteLoadAllocations:
        [UnsafeRawPointer: ShieldedOutgoingNoteLoadAllocation] = [:]
    private var shieldedSyncStateLoadAllocations:
        [UnsafeRawPointer: ShieldedSyncStateLoadAllocation] = [:]
    private var shieldedActivityLoadAllocations:
        [UnsafeRawPointer: ShieldedActivityLoadAllocation] = [:]
    private var shieldedViewingKeyLoadAllocations:
        [UnsafeRawPointer: ShieldedViewingKeyLoadAllocation] = [:]

    /// Set network, group id + birth height on the `PersistentWallet`
    /// row. Fires once at wallet registration with values the Rust side
    /// can contribute but Swift can't easily recompute (network is on
    /// the manager's SDK; the group id is the network-independent digest
    /// Rust derives from the root key; birth height is SPV's confirmed
    /// tip at creation). `walletGroupId` ties this row to its
    /// sibling-network rows for the same seed; it is left empty only if
    /// Rust handed back no bytes.
    func persistWalletMetadata(
        walletId: Data,
        network: Network,
        walletGroupId: Data,
        birthHeight: UInt32
    ) {
        onQueue {
            let wallet = ensureWalletRecord(walletId: walletId)
            wallet.network = network
            if !walletGroupId.isEmpty {
                wallet.walletGroupId = walletGroupId
            }
            wallet.birthHeight = birthHeight
            wallet.lastUpdated = Date()
            if !self.inChangeset { try? backgroundContext.save() }
        }
    }

    /// Set the user-facing name on the `PersistentWallet` row.
    /// Called from `PlatformWalletManager.createWallet` after the FFI
    /// returns a wallet id; only Swift knows the name, so it doesn't
    /// travel through a Rust-side callback. Silently skips if the row
    /// is missing (wallet wasn't successfully registered).
    public func setWalletName(walletId: Data, name: String) {
        onQueue {
            guard let wallet = findWalletRecord(walletId: walletId) else { return }
            wallet.name = name
            wallet.lastUpdated = Date()
            try? backgroundContext.save()
        }
    }

    /// Load the persisted seed-binding marker for `walletId`, or `nil`
    /// if none was ever written (first launch, pre-column row). The
    /// marker is opaque to Swift — it round-trips into
    /// `platform_wallet_verify_seed_binds_to_wallet_cached`, where Rust
    /// decides whether it still proves the binding.
    public func seedBindingMarker(walletId: Data) -> String? {
        onQueue {
            findWalletRecord(walletId: walletId)?.seedBindingVerifiedMarker
        }
    }

    /// Persist the seed-binding marker the cached verify FFI handed
    /// back (it returns one only when a full verification ran and
    /// bound). Silently skips if the row is missing, mirroring
    /// `setWalletName`.
    public func setSeedBindingMarker(walletId: Data, marker: String) {
        onQueue {
            guard let wallet = findWalletRecord(walletId: walletId) else { return }
            wallet.seedBindingVerifiedMarker = marker
            wallet.lastUpdated = Date()
            try? backgroundContext.save()
        }
    }

    /// Count `PersistentWallet` rows for `walletId` across ALL
    /// networks (deliberately ignores `self.network`). The mnemonic /
    /// metadata in the Keychain are shared by every network's row, so
    /// `deleteWallet` consults this after wiping its own network's row
    /// to decide whether the shared Keychain material can be purged.
    public func walletRowCountAcrossNetworks(walletId: Data) throws -> Int {
        try onQueue {
            let descriptor = FetchDescriptor<PersistentWallet>(
                predicate: PersistentWallet.predicate(walletId: walletId)
            )
            return try backgroundContext.fetchCount(descriptor)
        }
    }

    public func identityIdsForWallet(walletId: Data) throws -> [Data] {
        try onQueue {
            let descriptor = FetchDescriptor<PersistentWallet>(
                predicate: walletRecordPredicate(walletId: walletId)
            )
            guard let walletRow = try backgroundContext.fetch(descriptor).first else {
                return []
            }
            return walletRow.identities.map { $0.identityId }
        }
    }

    /// Wipe a wallet's SwiftData footprint.
    public func deleteWalletData(walletId: Data) throws {
        try onQueue {
            do {
                let walletDescriptor = FetchDescriptor<PersistentWallet>(
                    predicate: walletRecordPredicate(walletId: walletId)
                )
                let walletRow = try backgroundContext.fetch(walletDescriptor).first
                let walletNetwork = walletRow?.network

                if let walletRow = walletRow {
                    // Wallet → identities is `.nullify`; this delete
                    // path cascades them explicitly.
                    let identitiesToDelete = Array(walletRow.identities)
                    let identityIds = identitiesToDelete.map { $0.identityId }

                    for identityId in identityIds {
                        let balanceDescriptor = FetchDescriptor<PersistentTokenBalance>(
                            predicate: PersistentTokenBalance.predicate(identityId: identityId)
                        )
                        for row in try backgroundContext.fetch(balanceDescriptor) {
                            backgroundContext.delete(row)
                        }
                    }

                    // SwiftData fatals during save() whenever it has
                    // to null out a non-optional inverse on a child
                    // being processed in the same save batch (the
                    // canonical wording is
                    //   `Cannot remove PersistentX from relationship
                    //    Y on PersistentZ because an appropriate
                    //    default value is not configured`).
                    // Marking children for delete in the SAME batch
                    // doesn't help — SwiftData still walks their
                    // inverses during the merge phase.
                    //
                    // The workaround is to delete each layer in its
                    // own `save()`, parent last, so by the time the
                    // parent's delete runs its relationship
                    // collections are empty and SwiftData has no
                    // inverse to clean up. Costs us atomicity (four
                    // saves) — acceptable for a user-initiated wipe.
                    //
                    // PHASE 1: delete every identity's cascade-children
                    // whose inverse to identity is non-optional (DPNS
                    // names, DashPay profile, DashPay contact profiles,
                    // DashPay contact requests, DashPay payments, DashPay
                    // ignored senders). PublicKey, Document, and
                    // TokenBalance inverses to identity are already
                    // Optional and don't need pre-deletion.
                    //
                    // Every one of these rows has a non-optional
                    // `owner: PersistentIdentity`, so omitting any of them
                    // makes PHASE 2's identity delete hit the SwiftData
                    // fatal PHASE 1 exists to avoid — aborting the wipe and
                    // leaving sender-controlled DashPay strings (contact
                    // profile display name / public message / avatar URL),
                    // plaintext counterparty/memo/amount/txid (payments),
                    // and privacy-relevant ignored-sender ids on disk after
                    // a user-initiated wallet wipe.
                    for identity in identitiesToDelete {
                        for name in Array(identity.dpnsNames) {
                            backgroundContext.delete(name)
                        }
                        if let profile = identity.dashpayProfile {
                            backgroundContext.delete(profile)
                        }
                        for contactProfile in Array(identity.contactProfiles) {
                            backgroundContext.delete(contactProfile)
                        }
                        for cr in Array(identity.contactRequests) {
                            backgroundContext.delete(cr)
                        }
                        for payment in Array(identity.dashpayPayments) {
                            backgroundContext.delete(payment)
                        }
                        for ignored in Array(identity.dashpayIgnoredSenders) {
                            backgroundContext.delete(ignored)
                        }
                    }
                    try backgroundContext.save()

                    // PHASE 2: delete the identities themselves now
                    // that their problematic cascade children are
                    // gone from the store.
                    for identity in identitiesToDelete {
                        backgroundContext.delete(identity)
                    }
                    try backgroundContext.save()

                    // PHASE 3: delete the wallet's accounts. Same
                    // reasoning — `PersistentAccount.wallet` is
                    // non-optional; deleting accounts in their own
                    // save() pass leaves the wallet's `accounts`
                    // collection empty when the wallet itself is
                    // deleted.
                    let accountsToDelete = Array(walletRow.accounts)
                    for account in accountsToDelete {
                        backgroundContext.delete(account)
                    }
                    try backgroundContext.save()
                }

                // The txo / pending-input / asset-lock tables are keyed
                // by raw `walletId` with no relationship to
                // `PersistentWallet`, so the wallet-row delete below
                // does not cascade them — purge them explicitly.
                // `walletId` is network-scoped (key-wallet folds a
                // domain tag + network discriminant into the digest),
                // so every row under this id belongs to this wallet on
                // this network alone and the purge can't touch a
                // sibling network's cached state; a mnemonic's rows on
                // other networks live under different walletIds, tied
                // together only by `walletGroupId`.
                let txoDescriptor = FetchDescriptor<PersistentTxo>(
                    predicate: #Predicate<PersistentTxo> { $0.walletId == walletId }
                )
                for row in try backgroundContext.fetch(txoDescriptor) {
                    backgroundContext.delete(row)
                }

                let pendingDescriptor = FetchDescriptor<PersistentPendingInput>(
                    predicate: #Predicate<PersistentPendingInput> { $0.walletId == walletId }
                )
                for row in try backgroundContext.fetch(pendingDescriptor) {
                    backgroundContext.delete(row)
                }

                // `loadCachedAssetLocksOnQueue` rehydrates these rows on
                // the wallet-load path back into the Rust-side
                // `unused_asset_locks` map so an in-flight registration
                // can resume across an app kill. Without this cleanup,
                // delete-then-reimport of the same wallet would
                // resurrect stale Pending / Resumable asset-lock state
                // that the user thought they had wiped.
                let assetLockDescriptor = FetchDescriptor<PersistentAssetLock>(
                    predicate: #Predicate<PersistentAssetLock> { $0.walletId == walletId }
                )
                for row in try backgroundContext.fetch(assetLockDescriptor) {
                    backgroundContext.delete(row)
                }

                // Shielded (Orchard) per-wallet state. These four
                // tables are keyed by raw `walletId` (no relationship
                // to `PersistentWallet`), so the wallet-row delete
                // below does not cascade them — purge them explicitly
                // or they leak after a wipe and could resurface /
                // mis-attribute if the same `walletId` is reimported
                // (activity rows rehydrate into Rust via the
                // `on_load_shielded_activity_fn` callback as ghost
                // history and suppress fresh scan-derived entries).
                let shieldedNoteDescriptor = FetchDescriptor<PersistentShieldedNote>(
                    predicate: #Predicate<PersistentShieldedNote> { $0.walletId == walletId }
                )
                for row in try backgroundContext.fetch(shieldedNoteDescriptor) {
                    backgroundContext.delete(row)
                }

                let shieldedOutgoingNoteDescriptor = FetchDescriptor<PersistentShieldedOutgoingNote>(
                    predicate: #Predicate<PersistentShieldedOutgoingNote> { $0.walletId == walletId }
                )
                for row in try backgroundContext.fetch(shieldedOutgoingNoteDescriptor) {
                    backgroundContext.delete(row)
                }

                let shieldedSyncStateDescriptor = FetchDescriptor<PersistentShieldedSyncState>(
                    predicate: #Predicate<PersistentShieldedSyncState> { $0.walletId == walletId }
                )
                for row in try backgroundContext.fetch(shieldedSyncStateDescriptor) {
                    backgroundContext.delete(row)
                }

                let shieldedActivityDescriptor = FetchDescriptor<PersistentShieldedActivity>(
                    predicate: #Predicate<PersistentShieldedActivity> { $0.walletId == walletId }
                )
                for row in try backgroundContext.fetch(shieldedActivityDescriptor) {
                    backgroundContext.delete(row)
                }

                let shieldedViewingKeyDescriptor = FetchDescriptor<PersistentShieldedViewingKey>(
                    predicate: #Predicate<PersistentShieldedViewingKey> { $0.walletId == walletId }
                )
                for row in try backgroundContext.fetch(shieldedViewingKeyDescriptor) {
                    backgroundContext.delete(row)
                }

                // Masternode aggregation rows, keyed by raw `walletId` (no
                // relationship to `PersistentWallet`), so the wallet-row
                // delete below does not cascade them. `MasternodeSync` never
                // prunes on an empty aggregation (its no-prune-on-empty
                // rule), so without an explicit purge a delete-then-reimport
                // of the same wallet resurrects stale masternode rows
                // indefinitely.
                let masternodeDescriptor = FetchDescriptor<PersistentMasternode>(
                    predicate: #Predicate<PersistentMasternode> { $0.walletId == walletId }
                )
                for row in try backgroundContext.fetch(masternodeDescriptor) {
                    backgroundContext.delete(row)
                }

                if let walletRow = walletRow {
                    backgroundContext.delete(walletRow)
                }

                try backgroundContext.save()

                // Orphan sweep: drop tx rows no longer referenced by any
                // wallet. A row is referenced through the TXO graph
                // (outputs / inputs / pendingInputs) OR through the
                // `involvedAccounts` join — payload-only special txs
                // (e.g. a ProRegTx matching a provider owner key) have
                // no TXOs anywhere yet legitimately belong to a live
                // account, so sweeping on the TXO relations alone would
                // erase another wallet's payload-only history. The
                // deleted wallet's own payload-only rows still qualify:
                // its accounts were deleted (and their join links
                // nullified) in the earlier save above.
                let txRows = try backgroundContext.fetch(FetchDescriptor<PersistentTransaction>())
                for tx in txRows where tx.outputs.isEmpty &&
                    tx.inputs.isEmpty &&
                    tx.pendingInputs.isEmpty &&
                    tx.involvedAccounts.isEmpty {
                    backgroundContext.delete(tx)
                }

                if let walletNetwork = walletNetwork {
                    let networkRaw = walletNetwork.rawValue
                    let siblingDescriptor = FetchDescriptor<PersistentWallet>(
                        predicate: #Predicate<PersistentWallet> { $0.networkRaw == networkRaw }
                    )
                    let remaining = try backgroundContext.fetch(siblingDescriptor)
                        .filter { $0.walletId != walletId }
                    if remaining.isEmpty {
                        let scopeId = syncStateScopeId(for: walletNetwork)
                        let syncDescriptor = FetchDescriptor<PersistentPlatformAddressesSyncState>(
                            predicate: #Predicate { $0.walletId == scopeId }
                        )
                        if let syncRow = try backgroundContext.fetch(syncDescriptor).first {
                            backgroundContext.delete(syncRow)
                        }
                    }
                }

                try backgroundContext.save()
            } catch {
                backgroundContext.rollback()
                throw error
            }
        }
    }

    // MARK: - Watch-only Restore: Account xpub

    /// Upsert a `PersistentAccount` row with the full `AccountSpecFFI`
    /// payload. Key is `(walletId, type_tag, index, registration_index,
    /// key_class, user_identity_id, friend_identity_id)` — everything
    /// that uniquely identifies an account across variants.
    func persistAccount(walletId: Data, spec: AccountSpecFFI) {
        onQueue {
            guard let wallet = findWalletRecord(walletId: walletId) else { return }
            let typeTag = UInt32(spec.type_tag)
            let index = spec.index
            let registrationIndex = spec.registration_index
            let keyClass = spec.key_class
            var userIdentityId = Data(count: 32)
            withUnsafeBytes(of: spec.user_identity_id) { src in
                userIdentityId.withUnsafeMutableBytes { dst in
                    dst.copyMemory(from: src)
                }
            }
            var friendIdentityId = Data(count: 32)
            withUnsafeBytes(of: spec.friend_identity_id) { src in
                friendIdentityId.withUnsafeMutableBytes { dst in
                    dst.copyMemory(from: src)
                }
            }
            let xpubBytes: Data
            if let xpubPtr = spec.account_xpub_bytes, spec.account_xpub_bytes_len > 0 {
                xpubBytes = Data(bytes: xpubPtr, count: Int(spec.account_xpub_bytes_len))
            } else {
                xpubBytes = Data()
            }

            // Upsert keyed by the full account identity. We can't easily
            // express the identity tuple in a #Predicate with local `Data`
            // captures, so fetch by (walletId, accountType, accountIndex)
            // and verify the richer fields in Swift.
            let descriptor = FetchDescriptor<PersistentAccount>(
                predicate: #Predicate {
                    $0.wallet.walletId == walletId
                        && $0.accountType == typeTag
                        && $0.accountIndex == index
                }
            )
            let existing = (try? backgroundContext.fetch(descriptor)) ?? []
            let match = existing.first { acc in
                // `standardTag` splits Standard accounts into BIP44 (0)
                // and BIP32 (1) variants. Without it, the second emit
                // (whichever the Rust side serializes last) silently
                // aliases onto the first row and the BIP32 account is
                // never persisted as its own record.
                acc.standardTag == spec.standard_tag
                    && acc.registrationIndex == registrationIndex
                    && acc.keyClass == keyClass
                    && acc.userIdentityId == userIdentityId
                    && acc.friendIdentityId == friendIdentityId
            }
            let account: PersistentAccount
            if let match = match {
                account = match
            } else {
                account = PersistentAccount(
                    wallet: wallet,
                    accountType: typeTag,
                    accountIndex: index,
                    accountTypeName: accountTypeName(
                        for: spec.type_tag,
                        standardTag: spec.standard_tag
                    )
                )
                backgroundContext.insert(account)
            }
            account.standardTag = spec.standard_tag
            account.registrationIndex = registrationIndex
            account.keyClass = keyClass
            account.userIdentityId = userIdentityId
            account.friendIdentityId = friendIdentityId
            account.accountExtendedPubKeyBytes = xpubBytes
            account.lastUpdated = Date()
            if !self.inChangeset { try? backgroundContext.save() }
        }
    }

    // MARK: - Watch-only Restore: Load

    /// Enumerate persisted wallets into heap-allocated `WalletRestoreEntryFFI[]`.
    ///
    /// Ownership: Swift owns every allocation returned and retains them
    /// on `self.loadAllocations` keyed by the entries pointer. Rust
    /// calls `loadWalletListFree` exactly once after it's done reading,
    /// at which point we release the allocations.
    ///
    /// A wallet is "restorable" when it has at least one
    /// `PersistentAccount` row with non-empty
    /// `accountExtendedPubKeyBytes`. The Rust side reconstructs the
    /// watch-only `Wallet` via `Wallet::new_watch_only(network,
    /// wallet_id, accounts)`; accounts come directly from the spec
    /// array, wallet id from the top-level struct.
    ///
    /// One-shot upgrade heal: promote `isLocal` on wallet-linked rows
    /// still carrying `false` — the persister used to write a
    /// constant `false`, so a wallet's own identities (which are
    /// always local) were mis-marked on stores from that era.
    /// Promote-only and idempotent; a `true` on an unlinked row
    /// (manual add) is never touched. Runs here because load is the
    /// one guaranteed per-launch pass over the store, outside any
    /// changeset round.
    private func healIdentityIsLocalFlags() {
        guard !inChangeset else { return }
        guard let rows = try? backgroundContext.fetch(
            FetchDescriptor<PersistentIdentity>()
        ) else { return }
        var healed = 0
        for row in rows where row.wallet != nil && !row.isLocal {
            row.isLocal = true
            healed += 1
        }
        guard healed > 0 else { return }
        do {
            try backgroundContext.save()
            NSLog(
                "[persistor-load:swift] healed isLocal on %d identity row(s)",
                healed
            )
        } catch {
            // Non-fatal: the next launch retries. Roll back so the
            // failed heal can't bleed into the restore fetches below.
            backgroundContext.rollback()
            NSLog(
                "[persistor-load:swift] isLocal heal save failed: %@",
                String(describing: error)
            )
        }
    }

    /// Returns `(nil, 0)` if nothing is restorable.
    func loadWalletList() -> (entries: UnsafePointer<WalletRestoreEntryFFI>?, count: Int, errored: Bool) {
        onQueue {
        healIdentityIsLocalFlags()
        // Scope the fetch to the handler's bound network so a
        // per-network manager only sees its own wallets. If
        // `network` is `nil` (legacy callers that haven't threaded
        // network through yet) we fall back to the cross-network
        // fetch — those callers were already fragile against
        // cross-network data and the new path keeps them on the
        // pre-refactor behavior until they migrate.
        let walletDescriptor: FetchDescriptor<PersistentWallet>
        if let network = self.network {
            let raw = network.rawValue
            walletDescriptor = FetchDescriptor<PersistentWallet>(
                predicate: #Predicate { $0.networkRaw == raw }
            )
        } else {
            walletDescriptor = FetchDescriptor<PersistentWallet>()
        }
        let wallets: [PersistentWallet]
        do {
            wallets = try backgroundContext.fetch(walletDescriptor)
        } catch {
            // Surfacing the SwiftData failure to Rust is critical —
            // returning success-with-empty here would let restore
            // appear to "succeed" with zero wallets, hiding a real
            // database fault from the user. The callback returns
            // non-zero on `errored == true`.
            NSLog(
                "[persistor-load:swift] PersistentWallet fetch failed: %@",
                String(describing: error)
            )
            return (nil, 0, true)
        }
        let restorable = wallets.filter { wallet in
            wallet.accounts.contains { ($0.accountExtendedPubKeyBytes?.isEmpty == false) }
        }
        if restorable.isEmpty {
            return (nil, 0, false)
        }

        // Single bucketed fetch of every unspent `PersistentTxo` so
        // each wallet's per-iteration buffer build is a dictionary
        // lookup instead of a fresh database round-trip. Prefetches
        // `account.wallet` to keep the legacy-walletId routing path
        // (rows whose `walletId` field defaults to `Data()` because
        // they predate the denorm) from triggering one SwiftData
        // fault per row when we resolve the parent wallet.
        //
        // The fetch happens BEFORE we allocate `entriesPtr` /
        // `LoadAllocation` so an early fetch failure doesn't leak
        // the entries buffer (`LoadAllocation.release` is only
        // called on the path through `loadAllocations` after the
        // pointer hand-off to Rust succeeds).
        var unspentBuckets: [Data: [PersistentTxo]] = [:]
        do {
            var unspentDescriptor = FetchDescriptor<PersistentTxo>(
                predicate: #Predicate { $0.isSpent == false }
            )
            unspentDescriptor.relationshipKeyPathsForPrefetching = [\.account]
            // Bail with `errored = true` on a SwiftData failure rather
            // than degrading to an empty bucket map. Without this, Rust
            // would see `entry.utxos_count == 0` for every wallet,
            // skip `wallet_info.update_balance()`, and the restore
            // would silently report zero core-chain funds — exactly
            // the failure mode this code path was added to eliminate.
            let unspent: [PersistentTxo]
            do {
                unspent = try backgroundContext.fetch(unspentDescriptor)
            } catch {
                NSLog(
                    "[persistor-load:swift] PersistentTxo unspent fetch failed: %@",
                    String(describing: error)
                )
                return (nil, 0, true)
            }
            unspentBuckets.reserveCapacity(restorable.count)
            for row in unspent {
                guard row.account != nil else { continue }
                let key: Data
                if !row.walletId.isEmpty {
                    key = row.walletId
                } else if let account = row.account {
                    // `account.wallet` is non-optional on the
                    // model but is a fault-loaded relationship;
                    // a relationship-store inconsistency would
                    // crash here, so guard via Optional cast.
                    let wallet: PersistentWallet? = account.wallet
                    guard let resolved = wallet else { continue }
                    key = resolved.walletId
                } else {
                    continue
                }
                unspentBuckets[key, default: []].append(row)
            }
        }

        // Allocate `entriesPtr` and the `LoadAllocation` here — past
        // the fallible SwiftData fetch above — so an early-error path
        // doesn't leak the entries buffer (LoadAllocation only gets
        // released through the `loadAllocations` map after the
        // successful pointer hand-off at the bottom of this fn).
        let allocation = LoadAllocation()
        let entriesPtr = UnsafeMutablePointer<WalletRestoreEntryFFI>.allocate(
            capacity: restorable.count
        )
        allocation.entries = entriesPtr
        allocation.entriesCount = restorable.count

        for (i, w) in restorable.enumerated() {
            let sortedAccounts = w.accounts
                .filter { ($0.accountExtendedPubKeyBytes?.isEmpty == false) }
                .sorted {
                    ($0.accountType, $0.accountIndex, $0.registrationIndex, $0.keyClass)
                        < ($1.accountType, $1.accountIndex, $1.registrationIndex, $1.keyClass)
                }
            let accountsBuffer: UnsafeMutablePointer<AccountSpecFFI>?
            let accountsWritten: Int
            if sortedAccounts.isEmpty {
                accountsBuffer = nil
                accountsWritten = 0
            } else {
                let buf = UnsafeMutablePointer<AccountSpecFFI>.allocate(capacity: sortedAccounts.count)
                var written = 0
                for acc in sortedAccounts {
                    // Filter above guarantees non-nil + non-empty.
                    let xpub = acc.accountExtendedPubKeyBytes ?? Data()
                    // Reject rows whose `accountType` (UInt32) doesn't
                    // fit in `u8`. `truncatingIfNeeded` would silently
                    // wrap a corrupt 0x100+ value into a potentially-
                    // valid tag in the 0–255 range, defeating Rust's
                    // `AccountTypeTagFFI::try_from_u8` validation.
                    //
                    // A `continue` here would silently drop a
                    // funds-bearing account from the snapshot and
                    // still report a successful restore — so abort
                    // the whole load callback instead. The Rust
                    // loader treats `errored = true` as a hard fail
                    // and won't construct a half-loaded manager.
                    guard let typeTagByte = UInt8(exactly: acc.accountType) else {
                        NSLog(
                            "[persistor-load:swift] aborting load: account row has accountType %u out of UInt8 range — refusing to silently drop it",
                            acc.accountType
                        )
                        buf.deallocate()
                        allocation.release()
                        return (nil, 0, true)
                    }
                    let xpubBuffer = UnsafeMutablePointer<UInt8>.allocate(capacity: xpub.count)
                    xpub.copyBytes(to: xpubBuffer, count: xpub.count)
                    allocation.scalarBuffers.append((xpubBuffer, xpub.count))

                    var spec = AccountSpecFFI()
                    spec.type_tag = typeTagByte
                    spec.standard_tag = acc.standardTag
                    spec.index = acc.accountIndex
                    spec.registration_index = acc.registrationIndex
                    spec.key_class = acc.keyClass
                    copyBytes(acc.userIdentityId, into: &spec.user_identity_id)
                    copyBytes(acc.friendIdentityId, into: &spec.friend_identity_id)
                    spec.account_xpub_bytes = UnsafePointer(xpubBuffer)
                    spec.account_xpub_bytes_len = UInt(xpub.count)
                    // The platform-node (Ed25519) pool now rehydrates from
                    // this account's persisted typed core-address rows like
                    // every other pool — no dedicated batch on the spec.
                    buf[written] = spec
                    written += 1
                }
                if written == 0 {
                    buf.deallocate()
                    accountsBuffer = nil
                    accountsWritten = 0
                } else {
                    accountsBuffer = buf
                    accountsWritten = written
                    allocation.accountArrays.append((buf, written))
                }
            }

            let cachedBalances = loadCachedBalancesOnQueue(walletId: w.walletId)
            // Compact-write into the buffer with a `written` counter so
            // a malformed row (`hash.count != 20`) doesn't leave an
            // uninitialized slot in the published slice. Rust reads
            // exactly `entry.platform_address_balances_count` entries
            // from the pointer; any uninit slot would be undefined
            // behaviour. Same pattern the UTXO loader below uses.
            let addressBalancesBuffer: UnsafeMutablePointer<AddressBalanceEntryFFI>?
            let addressBalancesWritten: Int
            if cachedBalances.isEmpty {
                addressBalancesBuffer = nil
                addressBalancesWritten = 0
            } else {
                let buf = UnsafeMutablePointer<AddressBalanceEntryFFI>.allocate(
                    capacity: cachedBalances.count
                )
                var written = 0
                for cached in cachedBalances {
                    let (addressType, hash, balance, nonce, accountIndex, addressIndex, asOfHeight) =
                        cached
                    guard hash.count == 20 else { continue }

                    var hashTuple:
                        (
                            UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                            UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
                        ) = (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
                    withUnsafeMutableBytes(of: &hashTuple) { raw in
                        raw.copyBytes(from: hash)
                    }

                    buf[written] = AddressBalanceEntryFFI(
                        address: PlatformAddressFFI(address_type: addressType, hash: hashTuple),
                        balance: balance,
                        nonce: nonce,
                        account_index: accountIndex,
                        address_index: addressIndex,
                        as_of_height: asOfHeight
                    )
                    written += 1
                }
                if written == 0 {
                    buf.deallocate()
                    addressBalancesBuffer = nil
                    addressBalancesWritten = 0
                } else {
                    addressBalancesBuffer = buf
                    addressBalancesWritten = written
                    allocation.addressBalanceArrays.append((buf, written))
                }
            }

            let syncState = w.network.flatMap { loadCachedSyncStateOnQueue(network: $0) }

            // Identity slice. Sorted by `identityIndex` then
            // `identityId` so the rehydrated `IndexMap` order is
            // deterministic across launches; the explorer paginates by
            // index, so the order matters for stable rendering.
            let sortedIdentities = w.identities.sorted {
                if $0.identityIndex != $1.identityIndex {
                    return $0.identityIndex < $1.identityIndex
                }
                return $0.identityId.lexicographicallyPrecedes($1.identityId)
            }
            let identitiesBuffer = buildIdentityRestoreBuffer(
                identities: sortedIdentities,
                allocation: allocation
            )

            var entry = WalletRestoreEntryFFI()
            copyBytes(w.walletId, into: &entry.wallet_id)
            entry.network = (w.network ?? .testnet).ffiValue
            entry.accounts = accountsBuffer.map { UnsafePointer($0) }
            entry.accounts_count = UInt(accountsWritten)
            entry.platform_address_balances = addressBalancesBuffer.map { UnsafePointer($0) }
            entry.platform_address_balances_count = UInt(addressBalancesWritten)
            entry.platform_sync_height = syncState?.syncHeight ?? 0
            entry.platform_sync_timestamp = syncState?.syncTimestamp ?? 0
            entry.platform_last_known_recent_block = syncState?.lastKnownRecentBlock ?? 0
            entry.identities = identitiesBuffer.map { UnsafePointer($0) }
            entry.identities_count = UInt(sortedIdentities.count)
            // Core-chain sync metadata. `PersistentWallet` doesn't
            // carry a separate `lastProcessedHeight` column today;
            // for non-pruning SPV wallets the two heights advance in
            // lockstep at runtime, so re-using `syncedHeight` keeps
            // the restored wallet aligned with the runtime invariant.
            // Sending `0` here would leave `metadata.last_processed_height`
            // at `birth_height - 1` after restore, which mis-buckets
            // matured coinbase outputs as immature in
            // `update_balance` until SPV next advances. The proper
            // fix is a dedicated column on `PersistentWallet` —
            // tracked separately.
            entry.birth_height = w.birthHeight
            entry.synced_height = w.syncedHeight
            entry.last_processed_height = w.syncedHeight
            entry.last_synced = w.lastSynced

            // Persisted `last_applied_chain_lock` bincode bytes from
            // the previous session. Rust's `build_wallet_start_state`
            // decodes these and stamps `wallet_info.metadata.
            // last_applied_chain_lock`, so the asset-lock-resume
            // CL-from-metadata fallback in `proof.rs` can fire on
            // catch-up tasks at app launch without waiting for SPV
            // to re-apply a fresh chainlock. Wallets that have
            // never observed a chainlock (fresh creations,
            // pre-feature rows) carry `nil` here and the FFI fields
            // stay null / zero — Rust load falls back to leaving
            // `metadata.last_applied_chain_lock = None`.
            if let clBytes = w.lastAppliedChainLockBytes, !clBytes.isEmpty {
                let buffer = UnsafeMutablePointer<UInt8>.allocate(
                    capacity: clBytes.count
                )
                clBytes.copyBytes(to: buffer, count: clBytes.count)
                allocation.scalarBuffers.append((buffer, clBytes.count))
                entry.last_applied_chain_lock_bytes = UnsafePointer(buffer)
                entry.last_applied_chain_lock_bytes_len = UInt(clBytes.count)
            } else {
                entry.last_applied_chain_lock_bytes = nil
                entry.last_applied_chain_lock_bytes_len = 0
            }

            // Persisted unspent UTXOs for this wallet. The SPV inbound
            // path writes `PersistentTxo` rows and flips `isSpent`
            // (rather than deleting) on spend, so the unspent set is
            // exactly `isSpent == false`. Rust routes each row into
            // the matching funds-bearing account by tag; rows whose
            // account isn't a funds variant get silently skipped on
            // the receiving side.
            let (utxoBuf, utxoCount, utxoErrored) = buildUtxoRestoreBuffer(
                rows: unspentBuckets[w.walletId] ?? [],
                allocation: allocation
            )
            // `buildUtxoRestoreBuffer` already deallocated its own
            // buffer on the errored path; release everything else
            // we've accumulated and abort the load callback so Rust
            // doesn't see a partial / dropped-row snapshot.
            if utxoErrored {
                allocation.release()
                return (nil, 0, true)
            }
            entry.utxos = utxoBuf.map { UnsafePointer($0) }
            entry.utxos_count = UInt(utxoCount)

            let (poolBuf, poolCount, poolErrored) = buildCoreAddressPoolBuffer(
                accounts: sortedAccounts,
                allocation: allocation
            )
            if poolErrored {
                allocation.release()
                return (nil, 0, true)
            }
            entry.core_address_pools = poolBuf.map { UnsafePointer($0) }
            entry.core_address_pools_count = UInt(poolCount)

            // Tracked asset-lock rows. The Rust side rehydrates these
            // into `unused_asset_locks` so an in-flight registration
            // that was killed mid-flight can resume from the latest
            // status without rebroadcasting. Empty / null when the
            // wallet has no persisted locks.
            let assetLockRows = loadCachedAssetLocksOnQueue(walletId: w.walletId)
            let (assetLockBuf, assetLockCount) = buildAssetLockRestoreBuffer(
                rows: assetLockRows,
                allocation: allocation
            )
            entry.tracked_asset_locks = assetLockBuf.map { UnsafePointer($0) }
            entry.tracked_asset_locks_count = UInt(assetLockCount)

            // Funding tx records for asset locks at `statusRaw < 2`
            // (Built / Broadcast). The Rust load path re-inserts each
            // entry into the matching `standard_bip44_accounts[
            // account_index].transactions_mut()` bucket so the next
            // incoming chain-lock event can cascade-promote them.
            // Without this, the in-memory transactions map starts
            // empty after every restart, `apply_chain_lock` finds
            // nothing to promote at that height, and any asset lock
            // whose funding block has already been chain-locked
            // stays stuck at `Broadcast` indefinitely.
            //
            // Rows are filtered to `statusRaw < 2` so already-IS-
            // locked / already-chain-locked locks (which already
            // carry their proof on the `PersistentAssetLock` row and
            // don't need cascade-promotion) don't take up FFI
            // bandwidth. Empty / null when the wallet has no
            // unresolved locks.
            let (unresolvedBuf, unresolvedCount) =
                buildUnresolvedAssetLockTxRecordBuffer(
                    walletId: w.walletId,
                    allocation: allocation
                )
            entry.unresolved_asset_lock_tx_records = unresolvedBuf.map { UnsafePointer($0) }
            entry.unresolved_asset_lock_tx_records_count = UInt(unresolvedCount)

            // Provider special transactions (ProRegTx / ProUpServTx /
            // ProUpRegTx / ProUpRevTx) re-staged onto the provider-key
            // accounts so #876 retention keeps them and the masternode
            // list survives a restart (mirrors the asset-lock records above).
            let (providerTxBuf, providerTxCount) =
                buildProviderSpecialTxRestoreBuffer(
                    walletId: w.walletId,
                    allocation: allocation
                )
            entry.provider_special_txs = providerTxBuf.map { UnsafePointer($0) }
            entry.provider_special_txs_count = UInt(providerTxCount)

            // Primary-identity selection + gap-limit scan watermark
            // were dropped from the FFI shape — both moved off the
            // Rust manager (UI owns selection now, scan resume is
            // derived from the highest already-registered slot).
            entriesPtr[i] = entry
            // Bump the initialized-count so a later abort path's
            // `release()` only deinitializes slots that were
            // actually written (see `entriesInitialized`'s
            // doc-comment for why we can't reuse `entriesCount`).
            allocation.entriesInitialized = i + 1
        }

        let typed = UnsafePointer(entriesPtr)
        loadAllocations[UnsafeRawPointer(typed)] = allocation
        return (typed, restorable.count, false)
        }  // onQueue
    }

    /// Allocate a contiguous `[IdentityRestoreEntryFFI]` buffer for
    /// one wallet's identities and stash every nested allocation on
    /// `allocation` so the matching free callback can release them.
    ///
    /// Returns `nil` for an empty input — Rust treats `null` +
    /// `count == 0` as "no identities for this wallet".
    ///
    /// Every entry on a wallet's list is wallet-owned by definition
    /// (the per-identity `is_watched` flag was dropped along with the
    /// underlying `WatchedIdentity` type). The Rust side files each
    /// entry into `wallet_identities[wallet_id][identity_index]`.
    ///
    /// `dpns_names` / `contested_dpns_names` / `alias` aren't
    /// reflected here today — the sync path drops them on the floor.
    /// Both arrays come back as zero length and a `null` outer
    /// pointer. They're wired up so a future SwiftData column for
    /// either list is one-line work. The user-facing `alias` lives
    /// on `PersistentIdentity` and is read directly by the UI; it
    /// no longer roundtrips through Rust now that `ManagedIdentity`
    /// dropped its `label` field.
    /// Build a contiguous `[UtxoRestoreEntryFFI]` buffer for one
    /// wallet's unspent UTXOs. Walks `PersistentTxo` rows scoped to
    /// `walletId` and `isSpent == false`, copies the account-tag
    /// fields off the parent `PersistentAccount`, and emits one row
    /// per UTXO. Returns `(nil, 0)` for empty input — Rust treats
    /// `null` + `count == 0` as "no UTXOs to restore".
    ///
    /// Per-row script_pubkey buffers and the outer array are tracked
    /// on `allocation` so `loadWalletListFree` can release them.
    /// Rows whose `outpoint` payload isn't 32 bytes are skipped — the
    /// model stores it as `Data` (`outpoint: Data`) and bad data
    /// shouldn't crash the FFI handoff.
    /// Build the per-wallet UTXO restore buffer from a list of
    /// `PersistentTxo` rows already bucketed for this wallet by the
    /// caller. The bucketing pass in `loadWalletList` does the
    /// SwiftData fetch once for the whole batch (legacy empty-walletId
    /// rows route via `account.wallet.walletId`), so this function is
    /// pure marshalling.
    private func buildUtxoRestoreBuffer(
        rows: [PersistentTxo],
        allocation: LoadAllocation
    ) -> (UnsafeMutablePointer<UtxoRestoreEntryFFI>?, Int, Bool) {
        if rows.isEmpty {
            return (nil, 0, false)
        }
        let buf = UnsafeMutablePointer<UtxoRestoreEntryFFI>.allocate(capacity: rows.count)
        var written = 0
        for record in rows {
            guard let account = record.account else { continue }
            // `outpoint` on `PersistentTxo` is 36 bytes (32-byte txid
            // followed by LE u32 vout) — composed via
            // `makeOutpoint(txid:vout:)`. Use the dedicated `txid`
            // accessor, which prefers `transaction.txid` and falls
            // back to `outpoint.prefix(32)` so storage-explorer rows
            // and the FFI handoff agree on the same 32-byte identity.
            //
            // A row whose `txid` doesn't measure 32 bytes is corrupt
            // by construction (the model guarantees the prefix on
            // every write). Treat it the same way as the corrupt
            // `accountType` case below — abort the whole load so the
            // caller can surface the error rather than silently
            // under-restoring the funds set. Symmetric handling
            // keeps the restore contract uniform.
            let txid = record.txid
            guard txid.count == 32 else {
                NSLog(
                    "[persistor-load:swift] aborting load: UTXO has txid of %d bytes (expected 32) — refusing to silently drop it",
                    txid.count
                )
                buf.deallocate()
                return (nil, 0, true)
            }
            // Reject UTXOs whose parent `accountType` (UInt32) doesn't
            // fit in `u8`. Truncating would silently wrap a corrupt
            // 0x100+ value into a potentially-valid tag in 0–255 and
            // bypass Rust's `try_from_u8` validation. Drop-and-continue
            // would also silently under-restore the funds set, so we
            // signal `errored = true` and let `loadWalletList` fail
            // the whole callback — the persisted snapshot is corrupt.
            guard let typeTagByte = UInt8(exactly: account.accountType) else {
                NSLog(
                    "[persistor-load:swift] aborting load: UTXO has parent accountType %u out of UInt8 range — refusing to silently drop it",
                    account.accountType
                )
                buf.deallocate()
                return (nil, 0, true)
            }

            // Allocate + copy the script_pubkey bytes. Empty scripts
            // pass through with a null pointer + zero len.
            let scriptBytes = record.scriptPubKey
            let scriptPtr: UnsafePointer<UInt8>?
            let scriptLen = scriptBytes.count
            if scriptLen > 0 {
                let buffer = UnsafeMutablePointer<UInt8>.allocate(capacity: scriptLen)
                scriptBytes.copyBytes(to: buffer, count: scriptLen)
                allocation.scalarBuffers.append((buffer, scriptLen))
                scriptPtr = UnsafePointer(buffer)
            } else {
                scriptPtr = nil
            }

            var utxo = UtxoRestoreEntryFFI()
            // Tag fields are FFI-typed `u8` and validated via
            // `try_from_u8` on the Rust side; pass the exact byte
            // we just guarded above.
            utxo.type_tag = typeTagByte
            utxo.standard_tag = account.standardTag
            utxo.account_index = account.accountIndex
            utxo.registration_index = account.registrationIndex
            utxo.key_class = account.keyClass
            copyBytes(account.userIdentityId, into: &utxo.user_identity_id)
            copyBytes(account.friendIdentityId, into: &utxo.friend_identity_id)
            copyBytes(txid, into: &utxo.prev_txid)
            utxo.vout = record.vout
            utxo.value_duffs = record.amount
            utxo.script_pubkey = scriptPtr
            utxo.script_pubkey_len = UInt(scriptLen)
            utxo.height = record.height
            utxo.is_coinbase = record.isCoinbase
            utxo.is_confirmed = record.isConfirmed
            utxo.is_instantlocked = record.isInstantLocked
            utxo.is_locked = record.isLocked
            buf[written] = utxo
            written += 1
        }
        if written == 0 {
            buf.deallocate()
            return (nil, 0, false)
        }
        allocation.utxoArrays.append((buf, written))
        return (buf, written, false)
    }

    /// Build a contiguous `[AccountAddressPoolFFI]` buffer for one
    /// wallet's persisted core address pools
    private func buildCoreAddressPoolBuffer(
        accounts: [PersistentAccount],
        allocation: LoadAllocation
    ) -> (UnsafeMutablePointer<AccountAddressPoolFFI>?, Int, Bool) {
        var groups: [(account: PersistentAccount, poolTypeTag: UInt8, rows: [PersistentCoreAddress])] = []
        for account in accounts {
            if account.coreAddresses.isEmpty { continue }
            var byPool: [UInt8: [PersistentCoreAddress]] = [:]
            for addr in account.coreAddresses {
                byPool[addr.poolTypeTag, default: []].append(addr)
            }
            for (tag, rows) in byPool.sorted(by: { $0.key < $1.key }) {
                groups.append((account, tag, rows))
            }
        }
        if groups.isEmpty {
            return (nil, 0, false)
        }

        let buf = UnsafeMutablePointer<AccountAddressPoolFFI>.allocate(capacity: groups.count)
        var written = 0
        for group in groups {
            let account = group.account
            guard let typeTagByte = UInt8(exactly: account.accountType) else {
                NSLog(
                    "[persistor-load:swift] aborting load: address-pool account row has accountType %u out of UInt8 range",
                    account.accountType
                )
                buf.deallocate()
                return (nil, 0, true)
            }

            // Inner CoreAddressEntryFFI array — one row per address.
            let rowBuf = UnsafeMutablePointer<CoreAddressEntryFFI>.allocate(
                capacity: group.rows.count
            )
            for (j, row) in group.rows.enumerated() {
                var e = CoreAddressEntryFFI()
                // Copy the typed key bytes (<= 48) into the fixed slot and
                // record their length + curve tag. A row whose stored key
                // somehow exceeds the slot is emitted with no key rather
                // than truncated. Pure marshalling — the Rust side decides.
                if row.publicKey.count <= MemoryLayout.size(ofValue: e.public_key) {
                    copyBytes(row.publicKey, into: &e.public_key)
                    e.public_key_len = UInt8(row.publicKey.count)
                    e.key_type_tag = row.keyType
                } else {
                    e.public_key_len = 0
                    e.key_type_tag = 0
                }
                e.pool_type_tag = group.poolTypeTag
                e.address_index = row.addressIndex
                e.is_used = row.isUsed
                e.balance = row.balance
                e.address_base58 = UnsafePointer(
                    duplicateCString(row.address, allocation: allocation)
                )
                e.derivation_path = UnsafePointer(
                    duplicateCString(row.derivationPath, allocation: allocation)
                )
                rowBuf[j] = e
            }
            allocation.coreAddressEntryArrays.append((rowBuf, group.rows.count))

            var spec = AccountSpecFFI()
            spec.type_tag = typeTagByte
            spec.standard_tag = account.standardTag
            spec.index = account.accountIndex
            spec.registration_index = account.registrationIndex
            spec.key_class = account.keyClass
            copyBytes(account.userIdentityId, into: &spec.user_identity_id)
            copyBytes(account.friendIdentityId, into: &spec.friend_identity_id)
            spec.account_xpub_bytes = nil
            spec.account_xpub_bytes_len = 0

            var pool = AccountAddressPoolFFI()
            pool.account = spec
            pool.pool_type_tag = group.poolTypeTag
            pool.addresses_ptr = UnsafePointer(rowBuf)
            pool.addresses_count = UInt(group.rows.count)
            buf[written] = pool
            written += 1
        }
        allocation.coreAddressPoolArrays.append((buf, written))
        return (buf, written, false)
    }

    /// Build a contiguous `[AssetLockEntryFFI]` buffer for one wallet's
    /// tracked asset locks. Walks `PersistentAssetLock` rows scoped to
    /// `walletId`, copies the consensus-encoded transaction + optional
    /// bincode-encoded proof into Swift-owned heap buffers, and emits
    /// one row per lock. Returns `(nil, 0)` for empty input — Rust
    /// treats `null` + `count == 0` as "no tracked locks to restore".
    ///
    /// Per-row transaction/proof buffers and the outer array are
    /// tracked on `allocation` so `loadWalletListFree` releases them.
    /// Rows whose `outPointHex` doesn't parse back to 36 bytes are
    /// skipped — the model writes them in a known shape, so a
    /// mismatch indicates corruption that would crash Rust's decoder
    /// anyway.
    private func buildAssetLockRestoreBuffer(
        rows: [AssetLockEntrySnapshot],
        allocation: LoadAllocation
    ) -> (UnsafeMutablePointer<AssetLockEntryFFI>?, Int) {
        if rows.isEmpty {
            return (nil, 0)
        }
        let buf = UnsafeMutablePointer<AssetLockEntryFFI>.allocate(capacity: rows.count)
        var written = 0
        for record in rows {
            // Parse `<txid_hex>:<vout>` back into the 36-byte raw form
            // the Rust side expects. Any parse failure drops the row
            // — we can't manufacture a valid outpoint and a malformed
            // row indicates an old / corrupt snapshot.
            guard let outPoint = decodeOutPointHex(record.outPointHex) else {
                NSLog(
                    "[persistor-load:swift] dropping asset-lock row with malformed outPointHex: %@",
                    record.outPointHex
                )
                continue
            }

            // Allocate + copy the transaction bytes (Rust-owned for
            // the callback window via the allocation).
            let txBytes = record.transactionBytes
            let txPtr: UnsafePointer<UInt8>?
            let txLen = txBytes.count
            if txLen > 0 {
                let buffer = UnsafeMutablePointer<UInt8>.allocate(capacity: txLen)
                txBytes.copyBytes(to: buffer, count: txLen)
                allocation.scalarBuffers.append((buffer, txLen))
                txPtr = UnsafePointer(buffer)
            } else {
                // A row with no transaction bytes is broken — Rust's
                // load path will reject it; drop here.
                NSLog(
                    "[persistor-load:swift] dropping asset-lock row with empty transactionBytes: %@",
                    record.outPointHex
                )
                continue
            }

            // Optional proof bytes.
            let proofPtr: UnsafePointer<UInt8>?
            let proofLen: Int
            if let bytes = record.proofBytes, !bytes.isEmpty {
                let buffer = UnsafeMutablePointer<UInt8>.allocate(capacity: bytes.count)
                bytes.copyBytes(to: buffer, count: bytes.count)
                allocation.scalarBuffers.append((buffer, bytes.count))
                proofPtr = UnsafePointer(buffer)
                proofLen = bytes.count
            } else {
                proofPtr = nil
                proofLen = 0
            }

            var entry = AssetLockEntryFFI()
            copyBytes(outPoint, into: &entry.out_point)
            entry.transaction_bytes = txPtr
            entry.transaction_bytes_len = UInt(txLen)
            // BIP44 account the funding tx was built from, captured
            // on every upsert. The Rust load path uses this value to
            // route the unresolved record back into the matching
            // `standard_bip44_accounts[account_index]` bucket — a
            // wrong value silently drops the record, which broke
            // restore for any wallet that funded an asset lock from
            // a non-zero account index. Pre-feature rows default to
            // 0 (matches the previous behavior; the only realistic
            // common case).
            entry.account_index = UInt32(bitPattern: record.accountIndexRaw)
            // Exact (not clamping) conversion: a corrupt persisted row
            // with `fundingTypeRaw` or `statusRaw` outside `0...255`
            // would be silently coerced to a valid-looking enum value
            // by `UInt8(clamping:)` (negative → 0 = Built / IdentityRegistration,
            // >255 → 255 = sentinel). Either drops or rewrites the
            // asset-lock's effective state. Skip the row instead,
            // logged loudly so an operator can see and fix the bad row.
            guard let fundingType = UInt8(exactly: record.fundingTypeRaw) else {
                NSLog(
                    "[persistor-load] dropping asset-lock row %@ — fundingTypeRaw out of u8 range: %d",
                    record.outPointHex,
                    record.fundingTypeRaw
                )
                continue
            }
            guard let status = UInt8(exactly: record.statusRaw) else {
                NSLog(
                    "[persistor-load] dropping asset-lock row %@ — statusRaw out of u8 range: %d",
                    record.outPointHex,
                    record.statusRaw
                )
                continue
            }
            entry.funding_type = fundingType
            entry.identity_index = UInt32(bitPattern: record.identityIndexRaw)
            entry.amount_duffs = UInt64(bitPattern: record.amountDuffs)
            entry.status = status
            entry.proof_bytes = proofPtr
            entry.proof_bytes_len = UInt(proofLen)
            buf[written] = entry
            written += 1
        }
        if written == 0 {
            buf.deallocate()
            return (nil, 0)
        }
        allocation.assetLockArrays.append((buf, written))
        return (buf, written)
    }

    /// Build the per-wallet `UnresolvedAssetLockTxRecordFFI` array
    /// for the load callback. One entry per `PersistentAssetLock` row
    /// at `statusRaw < 2` (Built / Broadcast) whose funding tx has a
    /// matching `PersistentTransaction` row. Returns `(nil, 0)` when
    /// there are no eligible rows.
    ///
    /// The Rust side reads each row and re-inserts the decoded
    /// transaction into the matching BIP44 account's in-memory
    /// `transactions()` map so the next chain-lock event can promote
    /// it via `apply_chain_lock`. See
    /// `restore_unresolved_asset_lock_tx_records` for the Rust-side
    /// contract.
    ///
    /// Rows with no matching `PersistentTransaction` (e.g. an
    /// orphaned asset-lock row whose tx never made it into the
    /// transaction table) are skipped — the Rust side has no way to
    /// reconstruct the funding tx without its consensus bytes, so
    /// projecting an empty row would just bloat the FFI surface.
    private func buildUnresolvedAssetLockTxRecordBuffer(
        walletId: Data,
        allocation: LoadAllocation
    ) -> (UnsafeMutablePointer<UnresolvedAssetLockTxRecordFFI>?, Int) {
        // Filter to `statusRaw < 2` so already-IS-locked /
        // already-chain-locked rows don't end up in the array —
        // those locks have their proof bytes persisted on the
        // `PersistentAssetLock` row and the Rust side doesn't need
        // the funding tx in the in-memory map to use them.
        let descriptor = FetchDescriptor<PersistentAssetLock>(
            predicate: #Predicate { entry in
                entry.walletId == walletId && entry.statusRaw < 2
            }
        )
        guard let locks = try? backgroundContext.fetch(descriptor), !locks.isEmpty else {
            return (nil, 0)
        }

        // Pre-query the matching `PersistentTransaction` rows.
        // `PersistentAssetLock.outPointHex` carries the txid in
        // display order; `PersistentTransaction.txid` is wire order
        // — the same flip `decodeOutPointHex` already performs.
        let buf = UnsafeMutablePointer<UnresolvedAssetLockTxRecordFFI>.allocate(
            capacity: locks.count
        )
        var written = 0
        for lock in locks {
            guard let outpoint = decodeOutPointHex(lock.outPointHex) else {
                continue
            }
            let txid = outpoint.prefix(32)
            let txidData = Data(txid)
            let txDescriptor = FetchDescriptor<PersistentTransaction>(
                predicate: #Predicate { $0.txid == txidData }
            )
            guard let txRow = try? backgroundContext.fetch(txDescriptor).first else {
                // No matching tx — Rust can't reconstruct the
                // funding body without its consensus bytes. Skip.
                continue
            }
            // A globally-swept funding tx lost a double-spend on one of its
            // own inputs — it never confirms, so there is no unresolved
            // asset lock left to restore it into. Skip rather than hand
            // Rust a dead transaction to re-track.
            guard !txRow.isGloballySwept else { continue }
            let txBytes = txRow.transactionData
            guard !txBytes.isEmpty else {
                // A stub row whose real upsert never arrived;
                // skip rather than emit an undecodable buffer.
                continue
            }

            // Allocate the consensus-bytes buffer. Lifetime is
            // owned by `allocation.scalarBuffers`, freed by
            // `LoadAllocation.release()` after Rust returns.
            let txBuf = UnsafeMutablePointer<UInt8>.allocate(capacity: txBytes.count)
            txBytes.copyBytes(to: txBuf, count: txBytes.count)
            allocation.scalarBuffers.append((txBuf, txBytes.count))

            var entry = UnresolvedAssetLockTxRecordFFI()
            // Use the row's persisted `accountIndexRaw` — the Rust
            // side looks up `standard_bip44_accounts.get(&account_index)`
            // and silently drops the restore if the account doesn't
            // exist, so passing the actual funding account is
            // load-bearing for any wallet that funded an asset lock
            // from a non-zero BIP44 account index.
            entry.account_index = UInt32(bitPattern: lock.accountIndexRaw)
            entry.tx_bytes = txBuf
            entry.tx_bytes_len = UInt(txBytes.count)
            entry.context_raw = txRow.context
            entry.block_height = txRow.blockHeight
            if let hash = txRow.blockHash, hash.count == 32 {
                withUnsafeMutableBytes(of: &entry.block_hash) { raw in
                    raw.copyBytes(from: hash)
                }
            }
            entry.block_timestamp = UInt64(txRow.blockTimestamp)
            entry.first_seen = txRow.firstSeen
            buf[written] = entry
            written += 1
        }
        if written == 0 {
            buf.deallocate()
            return (nil, 0)
        }
        allocation.unresolvedAssetLockTxRecordArrays.append((buf, written))
        return (buf, written)
    }

    /// Stage this wallet's persisted provider special transactions
    /// (ProRegTx / ProUpServTx / ProUpRegTx / ProUpRevTx — `transactionTypeKind`
    /// 2...5) so the Rust load path re-inserts them onto the provider-key
    /// accounts and rust-dashcore #876 retention keeps them resident.
    /// Without this the masternode-list aggregation is empty after a
    /// restart until a rescan re-processes the blocks.
    ///
    /// Scoped to the wallet through `involvedAccounts` (provider txs create
    /// no TXOs, so they're payload-only matches carried by that
    /// many-to-many). Mirrors `buildUnresolvedAssetLockTxRecordBuffer`; the
    /// `tx_bytes` buffers live in `allocation.scalarBuffers` and the array
    /// in `allocation.providerSpecialTxRecordArrays`, both freed by
    /// `release()`.
    private func buildProviderSpecialTxRestoreBuffer(
        walletId: Data,
        allocation: LoadAllocation
    ) -> (UnsafeMutablePointer<ProviderSpecialTxRestoreEntryFFI>?, Int) {
        // Provider special-tx kinds are the contiguous discriminant range
        // 2...5 (ProviderRegistration=2 … ProviderUpdateRevocation=5).
        // `!isGloballySwept` excludes a provider tx that itself lost a
        // double-spend on one of its inputs — an edge case (most losers are
        // ordinary spends), but a swept row is never restorable regardless
        // of kind.
        let descriptor = FetchDescriptor<PersistentTransaction>(
            predicate: #Predicate { tx in
                tx.transactionTypeKind >= 2 && tx.transactionTypeKind <= 5
                    && tx.isGloballySwept == false
            }
        )
        guard let providerTxs = try? backgroundContext.fetch(descriptor),
              !providerTxs.isEmpty
        else {
            return (nil, 0)
        }

        // Scope through an explicitly involved provider-key account. Merely
        // sharing a wallet is insufficient: a provider-kind record observed
        // on a Standard account must not leak into unrelated provider state.
        // AccountTypeTagFFI 8...11 are Voting / Owner / Operator / Platform.
        let scoped = providerTxs.filter { tx in
            Self.shouldRestoreProviderSpecialTransaction(
                walletId: walletId,
                involvedAccounts: tx.involvedAccounts.map {
                    (walletId: $0.wallet.walletId, accountType: $0.accountType)
                }
            )
        }.sorted { lhs, rhs in
            if lhs.blockHeight != rhs.blockHeight {
                return lhs.blockHeight < rhs.blockHeight
            }
            if lhs.hasBlockPosition != rhs.hasBlockPosition {
                return lhs.hasBlockPosition && !rhs.hasBlockPosition
            }
            if lhs.blockPosition != rhs.blockPosition {
                return lhs.blockPosition < rhs.blockPosition
            }
            return lhs.firstSeen < rhs.firstSeen
        }
        guard !scoped.isEmpty else { return (nil, 0) }

        let buf = UnsafeMutablePointer<ProviderSpecialTxRestoreEntryFFI>.allocate(
            capacity: scoped.count
        )
        var written = 0
        for txRow in scoped {
            let txBytes = txRow.transactionData
            guard !txBytes.isEmpty else {
                // Stub row whose real upsert never landed — skip rather
                // than emit an undecodable buffer.
                continue
            }

            let txBuf = UnsafeMutablePointer<UInt8>.allocate(capacity: txBytes.count)
            txBytes.copyBytes(to: txBuf, count: txBytes.count)
            allocation.scalarBuffers.append((txBuf, txBytes.count))

            var entry = ProviderSpecialTxRestoreEntryFFI()
            entry.tx_bytes = txBuf
            entry.tx_bytes_len = UInt(txBytes.count)
            entry.context_raw = txRow.context
            entry.block_height = txRow.blockHeight
            if let hash = txRow.blockHash, hash.count == 32 {
                withUnsafeMutableBytes(of: &entry.block_hash) { raw in
                    raw.copyBytes(from: hash)
                }
            }
            entry.block_timestamp = UInt64(txRow.blockTimestamp)
            entry.block_position = txRow.blockPosition
            entry.has_block_position = txRow.hasBlockPosition
            entry.first_seen = txRow.firstSeen
            buf[written] = entry
            written += 1
        }
        if written == 0 {
            buf.deallocate()
            return (nil, 0)
        }
        allocation.providerSpecialTxRecordArrays.append((buf, written))
        return (buf, written)
    }

    /// Parse `<txid_hex (display order)>:<vout>` back into the 36-byte
    /// raw outpoint Rust expects (32-byte raw txid + 4-byte
    /// little-endian vout). Mirror of
    /// `PersistentAssetLock.encodeOutPoint`. Returns `nil` for any
    /// parse failure.
    private func decodeOutPointHex(_ hex: String) -> Data? {
        let parts = hex.split(separator: ":", maxSplits: 1, omittingEmptySubsequences: false)
        guard parts.count == 2 else { return nil }
        let txidHex = String(parts[0])
        guard let vout = UInt32(parts[1]) else { return nil }
        guard txidHex.count == 64 else { return nil }
        var txid = Data(capacity: 32)
        var idx = txidHex.startIndex
        for _ in 0..<32 {
            let end = txidHex.index(idx, offsetBy: 2)
            guard let byte = UInt8(txidHex[idx..<end], radix: 16) else { return nil }
            txid.append(byte)
            idx = end
        }
        // Reverse from display-order back to raw byte order.
        let raw = Data(txid.reversed())
        var out = Data(raw)
        out.append(contentsOf: withUnsafeBytes(of: vout.littleEndian) { Data($0) })
        return out
    }

    private func buildIdentityRestoreBuffer(
        identities: [PersistentIdentity],
        allocation: LoadAllocation
    ) -> UnsafeMutablePointer<IdentityRestoreEntryFFI>? {
        if identities.isEmpty {
            return nil
        }
        let buf = UnsafeMutablePointer<IdentityRestoreEntryFFI>.allocate(
            capacity: identities.count
        )
        for (j, identity) in identities.enumerated() {
            var entry = IdentityRestoreEntryFFI()
            copyBytes(identity.identityId, into: &entry.identity_id)
            // `PersistentIdentity` stores balance / revision as Int64
            // bit-pattern (matches how `persistIdentities` writes them).
            // Round-trip them as the same UInt64 bit-pattern.
            entry.balance = UInt64(bitPattern: identity.balance)
            entry.revision = UInt64(bitPattern: identity.revision)
            entry.identity_index = identity.identityIndex
            // Status isn't persisted today (no `status` column on
            // `PersistentIdentity`); fall back to `Unknown` (0). The
            // next identity sync round will re-stamp it via the
            // identity changeset path.
            entry.status = 0

            // DPNS names — currently empty. Wiring is here so a
            // future query against `PersistentDpnsName` rows (or a
            // dedicated array column on the identity) drops in
            // without touching the FFI plumbing.
            entry.dpns_names = nil
            entry.dpns_names_count = 0
            entry.contested_dpns_names = nil
            entry.contested_dpns_names_count = 0

            // Public keys — read the per-identity `PersistentPublicKey`
            // rows (relationship navigated directly; the rows are
            // fetched lazily by SwiftData but live in the same
            // background context as the identity row so the access is
            // synchronous). Sort by `keyId` so the BTreeMap that gets
            // built on the Rust side keeps a deterministic order.
            let sortedKeys = identity.publicKeys.sorted { $0.keyId < $1.keyId }
            if sortedKeys.isEmpty {
                entry.keys = nil
                entry.keys_count = 0
            } else {
                let keyBuf = UnsafeMutablePointer<IdentityKeyRestoreFFI>.allocate(
                    capacity: sortedKeys.count
                )
                for (k, pk) in sortedKeys.enumerated() {
                    var row = IdentityKeyRestoreFFI()
                    row.key_id = UInt32(bitPattern: pk.keyId)
                    // PersistentPublicKey stores the discriminants as
                    // `String(rawValue)` of the original `UInt8` —
                    // same shape as the `purposeEnum` /
                    // `securityLevelEnum` / `keyTypeEnum` accessors on
                    // the model. Decode back to `UInt8`; fall back to
                    // `UInt8.max` (an out-of-range sentinel) on parse
                    // failure so Rust's
                    // `KeyType::try_from(u8)` /
                    // `Purpose::try_from(u8)` /
                    // `SecurityLevel::try_from(u8)` rejects the row
                    // and `build_identity_public_keys` drops it. The
                    // prior fallback (`?? 0`) silently coerced
                    // corrupt rows into ECDSA_SECP256K1 / AUTHENTICATION
                    // / MASTER — a far worse outcome than a clean
                    // skip-and-continue.
                    row.key_type = UInt8(pk.keyType) ?? UInt8.max
                    row.purpose = UInt8(pk.purpose) ?? UInt8.max
                    row.security_level = UInt8(pk.securityLevel) ?? UInt8.max
                    row.read_only = pk.readOnly

                    // Allocate a dedicated byte buffer for the public
                    // key data. Same lifetime convention as xpub
                    // bytes — released by `LoadAllocation.release`
                    // via the `scalarBuffers` list.
                    let len = pk.publicKeyData.count
                    if len > 0 {
                        let dataBuf = UnsafeMutablePointer<UInt8>.allocate(capacity: len)
                        pk.publicKeyData.copyBytes(to: dataBuf, count: len)
                        row.data = UnsafePointer(dataBuf)
                        row.data_len = UInt(len)
                        allocation.scalarBuffers.append((dataBuf, len))
                    } else {
                        row.data = nil
                        row.data_len = 0
                    }

                    // Mirror the contract-bounds projection into
                    // the restore row so scoped keys (DashPay's
                    // SingleContractDocumentType, in particular)
                    // come back with their full variant on cold
                    // restart instead of silently degrading to
                    // unbounded. Encoding matches
                    // `IdentityKeyEntryFFI` on the persist side:
                    //   * kind=0 → no bounds; id zeroed, doc-type null
                    //   * kind=1 → SingleContract; id meaningful
                    //   * kind=2 → SingleContractDocumentType; id +
                    //     doc-type both meaningful
                    // Length-validated by `pk.publicKeyData.count
                    // == 32` (matches the gating in
                    // `toIdentityPublicKey()`); a row with a
                    // wrong-length id falls back to "no bounds"
                    // rather than crashing FFI marshalling on the
                    // Rust side.
                    if let id = pk.contractBounds?.first, id.count == 32 {
                        withUnsafeMutableBytes(of: &row.contract_bounds_id) { dst in
                            id.copyBytes(to: dst.bindMemory(to: UInt8.self).baseAddress!, count: 32)
                        }
                        if let docType = pk.contractBoundsDocumentTypeName, !docType.isEmpty {
                            row.contract_bounds_kind = 2
                            row.contract_bounds_document_type = UnsafePointer(
                                duplicateCString(docType, allocation: allocation)
                            )
                        } else {
                            row.contract_bounds_kind = 1
                            row.contract_bounds_document_type = nil
                        }
                    } else {
                        row.contract_bounds_kind = 0
                        row.contract_bounds_document_type = nil
                    }

                    keyBuf[k] = row
                }
                entry.keys = UnsafePointer(keyBuf)
                entry.keys_count = UInt(sortedKeys.count)
                allocation.identityKeyArrays.append((keyBuf, sortedKeys.count))
            }

            // DashPay contact rows — restores pending + established
            // contacts (with their contactInfo metadata) into the
            // Rust state at load. Without this, contacts re-derive
            // from chain on the first sweep and the re-establish
            // round wipes alias/note/hidden during the DIP-15
            // deferred-publish window (M3 relaunch-durability gap).
            let contactRows = identity.contactRequests
            if contactRows.isEmpty {
                entry.contacts = nil
                entry.contacts_count = 0
            } else {
                let contactBuf = UnsafeMutablePointer<ContactRequestFFI>.allocate(
                    capacity: contactRows.count
                )
                for (c, contact) in contactRows.enumerated() {
                    var row = ContactRequestFFI()
                    copyBytes(contact.ownerIdentityId, into: &row.owner_id)
                    copyBytes(contact.contactIdentityId, into: &row.contact_id)
                    row.is_outgoing = contact.isOutgoing
                    row.sender_key_index = contact.senderKeyIndex
                    row.recipient_key_index = contact.recipientKeyIndex
                    row.account_reference = contact.accountReference
                    row.core_height_created_at = contact.coreHeightCreatedAt
                    row.created_at = contact.createdAtMillis
                    row.payment_channel_broken = contact.paymentChannelBroken
                    row.is_hidden = contact.contactHidden

                    let payloads: [(Data?, WritableKeyPath<ContactRequestFFI, UnsafePointer<UInt8>?>, WritableKeyPath<ContactRequestFFI, UInt>)] = [
                        (contact.encryptedPublicKey, \.encrypted_public_key, \.encrypted_public_key_len),
                        (contact.encryptedAccountLabel, \.encrypted_account_label, \.encrypted_account_label_len),
                        (contact.autoAcceptProof, \.auto_accept_proof, \.auto_accept_proof_len),
                    ]
                    for (data, ptrPath, lenPath) in payloads {
                        if let data, !data.isEmpty {
                            let buf = UnsafeMutablePointer<UInt8>.allocate(capacity: data.count)
                            data.copyBytes(to: buf, count: data.count)
                            row[keyPath: ptrPath] = UnsafePointer(buf)
                            row[keyPath: lenPath] = UInt(data.count)
                            allocation.scalarBuffers.append((buf, data.count))
                        }
                    }

                    if let alias = contact.contactAlias, !alias.isEmpty {
                        row.alias = UnsafePointer(duplicateCString(alias, allocation: allocation))
                    }
                    if let note = contact.contactNote, !note.isEmpty {
                        row.note = UnsafePointer(duplicateCString(note, allocation: allocation))
                    }
                    // Direction-specific: only the incoming row stored the
                    // contact's label, so this is null on outgoing rows.
                    if let label = contact.contactAccountLabel, !label.isEmpty {
                        row.contact_account_label = UnsafePointer(duplicateCString(label, allocation: allocation))
                    }

                    // Relationship-wide (both directions carry it): feed the
                    // DIP-15 accepted-account acceptances back so the FFI row
                    // rebuild restores them instead of resetting to empty.
                    let accepted = contact.contactAcceptedAccounts
                    if !accepted.isEmpty {
                        let buf = UnsafeMutablePointer<UInt32>.allocate(capacity: accepted.count)
                        accepted.withUnsafeBufferPointer { src in
                            buf.initialize(from: src.baseAddress!, count: accepted.count)
                        }
                        row.accepted_accounts = UnsafePointer(buf)
                        row.accepted_accounts_len = UInt(accepted.count)
                        allocation.u32Buffers.append((buf, accepted.count))
                    }

                    contactBuf[c] = row
                }
                entry.contacts = UnsafePointer(contactBuf)
                entry.contacts_count = UInt(contactRows.count)
                allocation.contactArrays.append((contactBuf, contactRows.count))
            }

            // DashPay payment history — restores the dashpay_payments map
            // at load. Without this the in-memory map starts empty and only
            // Received entries are re-derived from UTXOs, so Sent entries +
            // memos silently vanish on every relaunch (H1).
            let paymentRows = identity.dashpayPayments
            if paymentRows.isEmpty {
                entry.payments = nil
                entry.payments_count = 0
            } else {
                let paymentBuf = UnsafeMutablePointer<PaymentRestoreEntryFFI>.allocate(
                    capacity: paymentRows.count
                )
                for (c, payment) in paymentRows.enumerated() {
                    var row = PaymentRestoreEntryFFI()
                    row.txid = UnsafePointer(duplicateCString(payment.txid, allocation: allocation))
                    copyBytes(payment.counterpartyIdentityId, into: &row.counterparty_id)
                    row.amount_duffs = payment.amountDuffs
                    row.direction_raw = payment.directionRaw
                    row.status_raw = payment.statusRaw
                    if let memo = payment.memo, !memo.isEmpty {
                        row.memo = UnsafePointer(duplicateCString(memo, allocation: allocation))
                    }
                    paymentBuf[c] = row
                }
                entry.payments = UnsafePointer(paymentBuf)
                entry.payments_count = UInt(paymentRows.count)
                allocation.paymentArrays.append((paymentBuf, paymentRows.count))
            }

            // DashPay ignored senders (per-sender mute, local-only) —
            // restores the ignored_senders set at load. Without this the
            // set starts empty on relaunch and a previously-ignored
            // sender's still-on-platform immutable contactRequests re-ingest
            // on the next sweep, resurfacing the ignored sender. Each entry
            // is a bare 32-byte sender id — a flat `[u8; 32]` array, no
            // owned pointers; Swift allocates + frees the buffer (via
            // `allocation.ignoredSenderArrays`), Rust only reads + copies.
            // Drop any row with a wrong-length id BEFORE allocating (same
            // abort-on-corrupt convention as the contact-profile array).
            let ignoredRows = identity.dashpayIgnoredSenders.filter {
                $0.ignoredSenderId.count == 32
            }
            if ignoredRows.isEmpty {
                entry.ignored_senders = nil
                entry.ignored_senders_count = 0
            } else {
                let ignoredBuf = UnsafeMutablePointer<FFIByteTuple32>.allocate(
                    capacity: ignoredRows.count
                )
                for (c, row) in ignoredRows.enumerated() {
                    var idTuple: FFIByteTuple32 =
                        (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
                    copyBytes(row.ignoredSenderId, into: &idTuple)
                    ignoredBuf[c] = idTuple
                }
                entry.ignored_senders = UnsafePointer(ignoredBuf)
                entry.ignored_senders_count = UInt(ignoredRows.count)
                allocation.ignoredSenderArrays.append((ignoredBuf, ignoredRows.count))
            }

            // Cached contact profiles — restores the contact_profiles map
            // (present entries only) at load. Without this the cache
            // starts empty on relaunch and the requests/contacts UI shows
            // raw identity ids until the next profile sweep re-fetches
            // every contact. Same ownership convention as the payments
            // array above: Swift allocates + frees (via
            // `allocation.contactProfileArrays` in `LoadAllocation.release`);
            // Rust only reads + copies out, never frees.
            // Drop any row with a wrong-length contact id BEFORE allocating —
            // `copyBytes` would otherwise zero-pad it and restore the profile
            // under a wrong key (matching the abort-on-corrupt convention the
            // UTXO restore uses). Filtering up front also keeps the fixed-
            // capacity buffer fully initialized so the count stays exact.
            let contactProfileRows = identity.contactProfiles.filter {
                $0.contactIdentityId.count == 32
            }
            if contactProfileRows.isEmpty {
                entry.contact_profiles = nil
                entry.contact_profiles_count = 0
            } else {
                let cpBuf = UnsafeMutablePointer<ContactProfileRestoreEntryFFI>.allocate(
                    capacity: contactProfileRows.count
                )
                for (c, profile) in contactProfileRows.enumerated() {
                    var row = ContactProfileRestoreEntryFFI()
                    copyBytes(profile.contactIdentityId, into: &row.contact_id)
                    if let displayName = profile.displayName, !displayName.isEmpty {
                        row.display_name = UnsafePointer(
                            duplicateCString(displayName, allocation: allocation))
                    }
                    if let bio = profile.bio, !bio.isEmpty {
                        row.bio = UnsafePointer(
                            duplicateCString(bio, allocation: allocation))
                    }
                    if let avatarUrl = profile.avatarUrl, !avatarUrl.isEmpty {
                        row.avatar_url = UnsafePointer(
                            duplicateCString(avatarUrl, allocation: allocation))
                    }
                    if let publicMessage = profile.publicMessage, !publicMessage.isEmpty {
                        row.public_message = UnsafePointer(
                            duplicateCString(publicMessage, allocation: allocation))
                    }
                    // Gate the byte arrays on presence — an absent hash /
                    // fingerprint must round-trip as `_present == false`,
                    // not as an all-zero value (which Rust would otherwise
                    // restore as a real `Some([0u8; N])`).
                    if let avatarHash = profile.avatarHash, avatarHash.count == 32 {
                        copyBytes(avatarHash, into: &row.avatar_hash)
                        row.avatar_hash_present = true
                    } else {
                        row.avatar_hash_present = false
                    }
                    if let avatarFingerprint = profile.avatarFingerprint,
                       avatarFingerprint.count == 8 {
                        copyBytes(avatarFingerprint, into: &row.avatar_fingerprint)
                        row.avatar_fingerprint_present = true
                    } else {
                        row.avatar_fingerprint_present = false
                    }
                    row.checked_at_ms = profile.checkedAtMs
                    cpBuf[c] = row
                }
                entry.contact_profiles = UnsafePointer(cpBuf)
                entry.contact_profiles_count = UInt(contactProfileRows.count)
                allocation.contactProfileArrays.append((cpBuf, contactProfileRows.count))
            }

            buf[j] = entry
        }
        allocation.identityArrays.append((buf, identities.count))
        return buf
    }

    /// Allocate a NUL-terminated UTF-8 c-string copy of `s` and stash
    /// it on `allocation` for release after the load callback returns.
    /// Empty strings are still allocated (round-trip with a
    /// `\0`-only buffer) — callers gate emission on `!isEmpty` before
    /// asking for one.
    private func duplicateCString(
        _ s: String,
        allocation: LoadAllocation
    ) -> UnsafeMutablePointer<CChar> {
        // `Array(s.utf8) + [0]` builds the byte sequence with the
        // trailing NUL Rust's `CStr::from_ptr` requires. We allocate
        // the pointer ourselves (rather than `strdup`) so the free
        // path can use `deallocate()` without coupling to libc.
        let utf8 = Array(s.utf8) + [0]
        let buf = UnsafeMutablePointer<CChar>.allocate(capacity: utf8.count)
        for (i, byte) in utf8.enumerated() {
            buf[i] = CChar(bitPattern: byte)
        }
        allocation.cStringBuffers.append((buf, utf8.count))
        return buf
    }

    /// Return the list of wallet ids that could be restored from
    /// SwiftData (i.e. have ≥1 account with a non-empty xpub). Used by
    /// `PlatformWalletManager.loadFromPersistor` after the FFI call
    /// succeeds so it can fetch a Swift-side handle for each wallet
    /// Rust just reconstructed.
    ///
    /// Network-scoped to match `loadWalletList()`: when `network` is
    /// non-nil the fetch is filtered to the handler's bound network so
    /// `loadFromPersistor` only requests handles for the wallets the FFI
    /// just reconstructed on this network — not sibling-network rows,
    /// whose handle lookups would miss and pollute `lastError`. When
    /// `network` is nil (legacy callers) we fall back to the unfiltered
    /// cross-network fetch, matching `loadWalletList()`.
    public func restorableWalletIds() -> [Data] {
        onQueue {
            let descriptor: FetchDescriptor<PersistentWallet>
            if let network = self.network {
                let raw = network.rawValue
                descriptor = FetchDescriptor<PersistentWallet>(
                    predicate: #Predicate { $0.networkRaw == raw }
                )
            } else {
                descriptor = FetchDescriptor<PersistentWallet>()
            }
            guard let wallets = try? backgroundContext.fetch(descriptor) else {
                return []
            }
            return wallets
                .filter { w in
                    w.accounts.contains { ($0.accountExtendedPubKeyBytes?.isEmpty == false) }
                }
                .map { $0.walletId }
        }
    }

    /// Release all allocations for a given load-callback result.
    func loadWalletListFree(entries: UnsafeRawPointer?) {
        onQueue {
            guard let entries = entries,
                  let allocation = loadAllocations.removeValue(forKey: entries) else {
                return
            }
            allocation.release()
        }
    }

    /// Outstanding load-call allocations keyed by the entries pointer
    /// we handed to Rust. Drained by `loadWalletListFree`.
    private var loadAllocations: [UnsafeRawPointer: LoadAllocation] = [:]

    /// Human-readable name for a persisted account, mirroring the
    /// top-level `AccountTypeTagFFI` discriminant plus — for tag 0
    /// (Standard) — the `StandardAccountTypeTagFFI` sub-discriminant.
    /// BIP44 vs BIP32 gets folded into the name so the UI can
    /// distinguish them without reading `standardTag` separately.
    private func accountTypeName(for tag: UInt8, standardTag: UInt8) -> String {
        switch tag {
        case 0:
            switch standardTag {
            case 0: return "BIP44 Account"
            case 1: return "BIP32 Account"
            default: return "Standard Account(\(standardTag))"
            }
        case 1: return "CoinJoin"
        case 2: return "Identity Registration"
        case 3: return "Identity Top-Up"
        case 4: return "Identity Top-Up (Unbound)"
        case 5: return "Identity Invitation"
        case 6: return "Asset Lock Address Top-Up"
        case 7: return "Asset Lock Shielded Address Top-Up"
        case 8: return "Provider Voting Keys"
        case 9: return "Provider Owner Keys"
        case 10: return "Provider Operator Keys"
        case 11: return "Provider Platform Node Keys"
        case 12: return "DashPay Receiving Funds"
        case 13: return "DashPay External Account"
        case 14: return "Platform Payment"
        case 15: return "Identity Auth (ECDSA)"
        case 16: return "Identity Auth (BLS)"
        default: return "Unknown(\(tag))"
        }
    }

    /// Build the 32-byte synthetic walletId used as the uniqueness
    /// key for the per-network `PersistentPlatformAddressesSyncState` row. The content
    /// is "platform-sync:<networkName>" zero-padded to 32 bytes.
    private func syncStateScopeId(for network: Network) -> Data {
        let scopeString = "platform-sync:\(network.networkName)"
        var data = Data(scopeString.utf8.prefix(32))
        if data.count < 32 {
            data.append(Data(repeating: 0, count: 32 - data.count))
        }
        return data
    }

    /// Look up a transaction record for the asset-lock proof flow's
    /// persister fallback (Rust trait method
    /// `PlatformWalletPersistence::get_core_tx_record`).
    ///
    /// The Rust-side asset-lock proof flow needs the chain-lock
    /// height + block hash + timestamp to construct a
    /// `ChainAssetLockProof`. With upstream's
    /// `keep-finalized-transactions` Cargo feature OFF (the default),
    /// chain-locked records are evicted from the in-memory
    /// `transactions()` map, so the chain-lock metadata is no longer
    /// reachable through the wallet-info API. The persister received
    /// the record on the chain-lock-transition `store` call before
    /// eviction; this lookup walks the corresponding
    /// `PersistentTransaction` row.
    ///
    /// Returns the row's actual `context` discriminant alongside the
    /// block info (when applicable). The Rust side faithfully
    /// reconstructs the matching `TransactionContext` variant — no
    /// chain-lock filter here, so a row in any state may be
    /// returned. `blockHash` / `blockHeight` / `blockTimestamp` are
    /// only meaningful for `context` 2 (InBlock) and 3
    /// (InChainLockedBlock); the Rust side ignores those fields for
    /// 0 (Mempool) and 1 (InstantSend).
    ///
    /// Returns `nil` when no `PersistentTransaction` row exists for
    /// `txid`, when an in-block / chain-locked row is missing its
    /// `blockHash` (treated as miss rather than fabricating a zero
    /// hash that would round-trip back to Rust as a real block id),
    /// or when the row has no `transactionData` (the FFI write path
    /// always populates it, so a missing one signals a corrupt row
    /// the Rust side can't decode anyway).
    ///
    /// The wallet-id is currently unused (`txid` is globally
    /// unique), but is accepted to match the Rust trait signature
    /// and to leave room for a wallet-scoped variant.
    func coreTxRecord(
        walletId: Data,
        txid: Data
    ) -> (context: UInt32, blockHeight: UInt32, blockHash: Data, blockTimestamp: UInt32, transactionData: Data)? {
        _ = walletId
        return onQueue {
            let descriptor = FetchDescriptor<PersistentTransaction>(
                predicate: #Predicate { $0.txid == txid }
            )
            guard let row = try? backgroundContext.fetch(descriptor).first else {
                return nil
            }
            // A globally-swept row can still physically exist (another
            // wallet's claim may not have cleared yet), but Rust has already
            // proven it dead — treat it the same as "no such transaction"
            // rather than handing back a body sent-payment reconciliation or
            // the asset-lock proof flow would read as live.
            guard !row.isGloballySwept else {
                return nil
            }
            // The Rust side decodes `transactionData` into a
            // `dashcore::Transaction`; an empty buffer (left over
            // from an orphaned stub row in the UTXO upsert path
            // whose real upsert never arrived) won't decode, so
            // treat it as miss.
            guard !row.transactionData.isEmpty else {
                return nil
            }
            let transactionData = row.transactionData
            switch row.context {
            case 0, 1:
                // Mempool / InstantSend — block fields not meaningful;
                // the Rust side ignores them. Hand back zeroed
                // placeholders so the caller's tuple shape stays
                // uniform.
                return (
                    context: row.context,
                    blockHeight: 0,
                    blockHash: Data(count: 32),
                    blockTimestamp: 0,
                    transactionData: transactionData
                )
            default:
                // InBlock / InChainLockedBlock — `blockHash` MUST be
                // present and 32 bytes for the row to round-trip
                // correctly to Rust as a `BlockHash`.
                guard let blockHash = row.blockHash, blockHash.count == 32 else {
                    return nil
                }
                return (
                    context: row.context,
                    blockHeight: row.blockHeight,
                    blockHash: blockHash,
                    blockTimestamp: row.blockTimestamp,
                    transactionData: transactionData
                )
            }
        }
    }

    /// `AccountTypeTagFFI` discriminant for a watch-only DashPay external
    /// (contact) account. TXOs tracked under it are the *contact's* coins,
    /// mirrored locally so sends to the contact can be detected — they are
    /// not spendable by this wallet.
    static let dashpayExternalAccountTypeTag: UInt32 = 13

    /// `true` when `transaction` spends at least one input funded by one of
    /// this wallet's own spendable accounts.
    ///
    /// Pure row data: each entry in `transaction.inputs` is a `PersistentTxo`
    /// this transaction spent, carrying the owning wallet denorm and the
    /// account it was tracked under. A TXO tracked only by the watch-only
    /// DashPay external account does NOT count — those are the contact's
    /// coins, and counting them would tag a third party's transaction (the
    /// contact spending their own money) as wallet-funded. A TXO whose
    /// account link faulted to `nil` counts as owned: spendable-account rows
    /// always carry the link, so `nil` is a relationship-store anomaly and
    /// under-reporting would silently erase real sent history.
    /// `pendingInputs` are deliberately ignored: a spend of our own coins
    /// always has its funding TXO persisted (the wallet had to know the
    /// output to spend it), while a pending row proves nothing about
    /// ownership.
    static func walletFundedTransaction(
        walletId: Data,
        transaction: PersistentTransaction
    ) -> Bool {
        transaction.inputs.contains { txo in
            // Resolved, not raw: a legacy TXO with an empty denormalized
            // `walletId` is still our coin, and reading it as "not ours" turns
            // a real spend into an unfunded transaction — the sweep then skips
            // it and can still stamp the contact, losing the payment for the
            // process lifetime.
            Self.resolvedWalletId(of: txo) == walletId
                && txo.account.map { $0.accountType != dashpayExternalAccountTypeTag } ?? true
        }
    }

    /// Enumerate the persisted txids scoped to `walletId`, each paired with
    /// whether this wallet funded the transaction (see
    /// [`walletFundedTransaction`]).
    ///
    /// Scope is the union of wallet-owned TXOs (`outputs`, `inputs`,
    /// `pendingInputs`) and payload-only account involvement
    /// (`involvedAccounts`).
    /// Returns `errored: true` when the fetch itself failed, so the shim can
    /// report a non-zero status. Collapsing a database fault to an empty list
    /// would be indistinguishable from a wallet with no transactions, and the
    /// Rust side treats those two very differently.
    func walletCoreTxids(
        walletId: Data
    ) -> (txids: [(txid: Data, spendsWalletInput: Bool)], errored: Bool) {
        onQueue {
            let descriptor = FetchDescriptor<PersistentTransaction>()
            let rows: [PersistentTransaction]
            do {
                rows = try backgroundContext.fetch(descriptor)
            } catch {
                NSLog(
                    "[persistor-txids:swift] PersistentTransaction fetch failed: %@",
                    String(describing: error)
                )
                return ([], true)
            }
            let txids = rows.compactMap { tx -> (txid: Data, spendsWalletInput: Bool)? in
                guard Self.walletOwnsTransaction(walletId: walletId, transaction: tx) else {
                    return nil
                }
                return (
                    txid: tx.txid,
                    spendsWalletInput: Self.walletFundedTransaction(
                        walletId: walletId,
                        transaction: tx
                    )
                )
            }
            return (txids, false)
        }
    }

    /// Look up the network for a wallet id by reading the owning
    /// `PersistentWallet` row. Returns `nil` if the wallet row
    /// doesn't exist or its network hasn't been resolved yet.
    private func walletNetwork(walletId: Data) -> Network? {
        // Scope to this handler's network when one is set so a mnemonic
        // that lives on multiple networks resolves to the row for THIS
        // manager's network — not an arbitrary sibling row that would
        // mis-stamp persisted sync state / identity / token writes and
        // feed the wrong coin type into key derivation. Falls back to
        // walletId-only when no network is set (legacy / no-container).
        let descriptor = FetchDescriptor<PersistentWallet>(
            predicate: walletRecordPredicate(walletId: walletId)
        )
        guard let wallet = try? backgroundContext.fetch(descriptor).first else {
            return nil
        }
        return wallet.network
    }
}

/// Retains all heap allocations produced by a single
/// `loadWalletList` call. Released wholesale by `loadWalletListFree`.
private final class LoadAllocation {
    var entries: UnsafeMutablePointer<WalletRestoreEntryFFI>?
    /// Allocated capacity — equal to `restorable.count`. Used for
    /// `deallocate()` (which only requires "the original allocation
    /// size") and as the upper bound on `entriesInitialized`.
    var entriesCount: Int = 0
    /// How many of the `entriesCount` slots have actually been
    /// written via `entriesPtr[i] = entry`. Tracked separately from
    /// `entriesCount` because early-abort paths (account-tag
    /// overflow, UTXO marshalling failure) call `release()` after
    /// only `0..<i` slots have been initialized; calling
    /// `deinitialize(count: entriesCount)` over the full capacity
    /// would deinitialize uninitialized memory, which is UB by
    /// `UnsafeMutablePointer`'s contract. The fact that
    /// `WalletRestoreEntryFFI` and its siblings happen to import as
    /// trivial C structs means the no-op deinit doesn't crash today,
    /// but any future field that imports as a non-trivial Swift
    /// type would turn this into real UB.
    var entriesInitialized: Int = 0
    /// `AccountSpecFFI` arrays per wallet.
    var accountArrays: [(UnsafeMutablePointer<AccountSpecFFI>, Int)] = []
    /// `AddressBalanceEntryFFI` arrays per wallet.
    var addressBalanceArrays: [(UnsafeMutablePointer<AddressBalanceEntryFFI>, Int)] = []
    /// `IdentityRestoreEntryFFI` arrays per wallet.
    var identityArrays: [(UnsafeMutablePointer<IdentityRestoreEntryFFI>, Int)] = []
    /// Per-identity `IdentityKeyRestoreFFI` arrays. One entry per
    /// identity that has at least one persisted public key. The byte
    /// buffers each row's `data` pointer references live in
    /// `scalarBuffers` (same `UnsafeMutablePointer<UInt8>.allocate`
    /// shape as xpub bytes).
    var identityKeyArrays: [(UnsafeMutablePointer<IdentityKeyRestoreFFI>, Int)] = []
    /// Per-identity `ContactRequestFFI` arrays (DashPay contact
    /// restore — M3). Byte payloads live in `scalarBuffers`; the
    /// alias/note strings live in `cStringBuffers`. NOTE: these rows
    /// are load-allocation-owned — Rust's `free_contact_requests_ffi`
    /// must never run on them (it owns only persist-side rows).
    var contactArrays: [(UnsafeMutablePointer<ContactRequestFFI>, Int)] = []
    /// Per-identity `PaymentRestoreEntryFFI` arrays (DashPay payment
    /// restore — H1). The txid/memo strings live in `cStringBuffers`.
    var paymentArrays: [(UnsafeMutablePointer<PaymentRestoreEntryFFI>, Int)] = []
    /// Per-identity ignored-sender arrays (DashPay ignored-sender
    /// restore). Each row is a bare 32-byte sender id (`FFIByteTuple32`) —
    /// flat POD, no owned pointers, so nothing extra rides
    /// `scalarBuffers`/`cStringBuffers`.
    var ignoredSenderArrays: [(UnsafeMutablePointer<FFIByteTuple32>, Int)] = []
    /// Per-identity `ContactProfileRestoreEntryFFI` arrays (cached
    /// contact-profile restore). The four optional profile strings each
    /// row references live in `cStringBuffers`. NOTE: these rows are
    /// load-allocation-owned — Rust only reads them; it must never run a
    /// free over them.
    var contactProfileArrays:
        [(UnsafeMutablePointer<ContactProfileRestoreEntryFFI>, Int)] = []
    /// Byte buffers backing `root_xpub_bytes` and `account_xpub_bytes`.
    var scalarBuffers: [(UnsafeMutablePointer<UInt8>, Int)] = []
    /// `u32` buffers backing `ContactRequestFFI::accepted_accounts` (the
    /// DIP-15 rotated-account acceptances). Separate from `scalarBuffers`
    /// because the element type differs; freed by `deallocate()`.
    var u32Buffers: [(UnsafeMutablePointer<UInt32>, Int)] = []
    /// NUL-terminated c-string buffers carried by identity entries
    /// (`label`, dpns name labels, etc.). Allocated via plain
    /// `UnsafeMutablePointer<CChar>.allocate`, freed by `deallocate()`.
    var cStringBuffers: [(UnsafeMutablePointer<CChar>, Int)] = []
    /// `*const c_char` arrays referenced by `dpns_names` /
    /// `contested_dpns_names`. Each inner pointer points into
    /// `cStringBuffers`; releasing this array doesn't touch the
    /// underlying strings.
    var cStringPointerArrays: [(UnsafeMutablePointer<UnsafePointer<CChar>?>, Int)] = []
    /// Per-wallet `UtxoRestoreEntryFFI` arrays. The script bytes each
    /// row references live in `scalarBuffers`.
    var utxoArrays: [(UnsafeMutablePointer<UtxoRestoreEntryFFI>, Int)] = []
    /// Per-wallet `AssetLockEntryFFI` arrays. The transaction-bytes
    /// and proof-bytes buffers each row references live in
    /// `scalarBuffers`.
    var assetLockArrays: [(UnsafeMutablePointer<AssetLockEntryFFI>, Int)] = []
    /// Per-wallet `UnresolvedAssetLockTxRecordFFI` arrays — the funding
    /// tx records for asset locks at `statusRaw < 2` that the Rust
    /// load path re-inserts into the in-memory `transactions()` map
    /// so the next chain-lock event can cascade-promote them. The
    /// `tx_bytes` buffer each row references lives in `scalarBuffers`.
    var unresolvedAssetLockTxRecordArrays: [(UnsafeMutablePointer<UnresolvedAssetLockTxRecordFFI>, Int)] = []
    /// Per-wallet `ProviderSpecialTxRestoreEntryFFI` arrays — provider
    /// special txs re-staged so #876 retention keeps them resident after a
    /// restart. The `tx_bytes` buffer each row references lives in
    /// `scalarBuffers`.
    var providerSpecialTxRecordArrays: [(UnsafeMutablePointer<ProviderSpecialTxRestoreEntryFFI>, Int)] = []
    /// Per-wallet `AccountAddressPoolFFI` arrays, the persisted core
    /// address pools
    var coreAddressPoolArrays: [(UnsafeMutablePointer<AccountAddressPoolFFI>, Int)] = []
    /// Inner `CoreAddressEntryFFI` arrays, one per pool entry above.
    var coreAddressEntryArrays: [(UnsafeMutablePointer<CoreAddressEntryFFI>, Int)] = []

    func release() {
        if let entries = entries {
            // Deinitialize ONLY the slots that were actually written
            // (`entriesInitialized`), then deallocate the full
            // capacity (`entriesCount`). Per Swift's pointer
            // contract, `deinitialize(count:)` requires the region
            // to be initialized; `deallocate()` only requires the
            // pointer to match the original allocation.
            if entriesInitialized > 0 {
                entries.deinitialize(count: entriesInitialized)
            }
            entries.deallocate()
        }
        for (ptr, count) in accountArrays {
            ptr.deinitialize(count: count)
            ptr.deallocate()
        }
        for (ptr, count) in addressBalanceArrays {
            ptr.deinitialize(count: count)
            ptr.deallocate()
        }
        for (ptr, count) in identityArrays {
            ptr.deinitialize(count: count)
            ptr.deallocate()
        }
        for (ptr, count) in identityKeyArrays {
            ptr.deinitialize(count: count)
            ptr.deallocate()
        }
        for (ptr, count) in contactArrays {
            ptr.deinitialize(count: count)
            ptr.deallocate()
        }
        for (ptr, count) in paymentArrays {
            ptr.deinitialize(count: count)
            ptr.deallocate()
        }
        for (ptr, count) in ignoredSenderArrays {
            ptr.deinitialize(count: count)
            ptr.deallocate()
        }
        for (ptr, count) in contactProfileArrays {
            ptr.deinitialize(count: count)
            ptr.deallocate()
        }
        for (ptr, _) in scalarBuffers {
            ptr.deallocate()
        }
        for (ptr, _) in u32Buffers {
            ptr.deallocate()
        }
        for (ptr, _) in cStringBuffers {
            ptr.deallocate()
        }
        for (ptr, _) in cStringPointerArrays {
            ptr.deallocate()
        }
        for (ptr, count) in utxoArrays {
            ptr.deinitialize(count: count)
            ptr.deallocate()
        }
        for (ptr, count) in assetLockArrays {
            ptr.deinitialize(count: count)
            ptr.deallocate()
        }
        for (ptr, count) in unresolvedAssetLockTxRecordArrays {
            ptr.deinitialize(count: count)
            ptr.deallocate()
        }
        for (ptr, count) in providerSpecialTxRecordArrays {
            ptr.deinitialize(count: count)
            ptr.deallocate()
        }
        for (ptr, count) in coreAddressEntryArrays {
            ptr.deinitialize(count: count)
            ptr.deallocate()
        }
        for (ptr, count) in coreAddressPoolArrays {
            ptr.deinitialize(count: count)
            ptr.deallocate()
        }
    }
}

/// Allocation tracker for `loadShieldedNotes` — the entries
/// buffer plus per-row `note_data` byte buffers.
private final class ShieldedLoadAllocation {
    var entries: UnsafeMutablePointer<ShieldedNoteRestoreFFI>?
    var entriesCount: Int = 0
    var entriesInitialized: Int = 0
    /// Per-row `note_data` byte buffers; each entry's
    /// `note_data_ptr` references one of these.
    var scalarBuffers: [(UnsafeMutablePointer<UInt8>, Int)] = []

    func release() {
        if let entries = entries {
            if entriesInitialized > 0 {
                entries.deinitialize(count: entriesInitialized)
            }
            entries.deallocate()
        }
        for (ptr, _) in scalarBuffers {
            ptr.deallocate()
        }
    }
}

/// Allocation tracker for `loadShieldedOutgoingNotes` — the entries
/// buffer plus per-row `memo` byte buffers. Same shape as
/// `ShieldedLoadAllocation`; each entry's `memo_ptr` references one
/// of the `scalarBuffers`.
private final class ShieldedOutgoingNoteLoadAllocation {
    var entries: UnsafeMutablePointer<ShieldedOutgoingNoteRestoreFFI>?
    var entriesCount: Int = 0
    var entriesInitialized: Int = 0
    /// Per-row `memo` byte buffers; each entry's `memo_ptr`
    /// references one of these.
    var scalarBuffers: [(UnsafeMutablePointer<UInt8>, Int)] = []

    func release() {
        if let entries = entries {
            if entriesInitialized > 0 {
                entries.deinitialize(count: entriesInitialized)
            }
            entries.deallocate()
        }
        for (ptr, _) in scalarBuffers {
            ptr.deallocate()
        }
    }
}

/// Allocation tracker for `loadShieldedSyncStates`. No nested
/// buffers — every field is plain-data — so this is just the
/// entries buffer.
private final class ShieldedSyncStateLoadAllocation {
    var entries: UnsafeMutablePointer<ShieldedSubwalletSyncStateFFI>?
    var entriesCount: Int = 0
    var entriesInitialized: Int = 0

    func release() {
        if let entries = entries {
            if entriesInitialized > 0 {
                entries.deinitialize(count: entriesInitialized)
            }
            entries.deallocate()
        }
    }
}

/// Allocation tracker for `loadShieldedViewingKeys` — a flat entries
/// buffer with no per-row pointer fields (the FVK is a fixed 96-byte
/// inline array), so the same shape as
/// `ShieldedSyncStateLoadAllocation`.
private final class ShieldedViewingKeyLoadAllocation {
    var entries: UnsafeMutablePointer<ShieldedViewingKeyRestoreFFI>?
    var entriesCount: Int = 0
    var entriesInitialized: Int = 0

    func release() {
        if let entries = entries {
            if entriesInitialized > 0 {
                entries.deinitialize(count: entriesInitialized)
            }
            entries.deallocate()
        }
    }
}

/// Allocation tracker for `loadShieldedActivity` — the entries buffer
/// plus per-row byte buffers for the four pointer-backed fields
/// (counterparty / memo / note-cmx array / spent-nullifier array). Each
/// entry's `*_ptr` references one of `scalarBuffers`.
private final class ShieldedActivityLoadAllocation {
    var entries: UnsafeMutablePointer<ShieldedActivityRestoreFFI>?
    var entriesCount: Int = 0
    var entriesInitialized: Int = 0
    var scalarBuffers: [(UnsafeMutablePointer<UInt8>, Int)] = []

    func release() {
        if let entries = entries {
            if entriesInitialized > 0 {
                entries.deinitialize(count: entriesInitialized)
            }
            entries.deallocate()
        }
        for (ptr, _) in scalarBuffers {
            ptr.deallocate()
        }
    }
}

/// Copy bytes from `src` into a fixed-size C-tuple field. Swift
/// imports `u8[N]` as an N-tuple — identical memory layout, so
/// `withUnsafeMutableBytes` gives us a contiguous write window of
/// exactly N bytes.
@inline(__always)
private func copyBytes<T>(_ src: Data, into dst: inout T) {
    withUnsafeMutableBytes(of: &dst) { raw in
        let bytes = raw.bindMemory(to: UInt8.self)
        let len = min(src.count, raw.count)
        src.copyBytes(to: bytes, count: len)
    }
}

// MARK: - C Callbacks

private func persistAddressBalancesCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    entriesPtr: UnsafePointer<AddressBalanceEntryFFI>?,
    count: UInt
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr,
          let entriesPtr = entriesPtr,
          count > 0 else {
        return 0
    }

    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()

    let walletId = Data(bytes: walletIdPtr, count: 32)

    var entries: [(UInt8, Data, UInt64, UInt32, UInt32, UInt32, UInt64)] = []
    entries.reserveCapacity(Int(count))

    for i in 0..<Int(count) {
        let entry = entriesPtr[i]
        let hashData = withUnsafeBytes(of: entry.address.hash) { Data($0) }
        entries.append((
            entry.address.address_type,
            hashData,
            entry.balance,
            entry.nonce,
            entry.account_index,
            entry.address_index,
            entry.as_of_height
        ))
    }

    handler.persistAddressBalances(walletId: walletId, entries: entries)
    return 0
}

private func persistWalletChangesetCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    changesetPtr: UnsafePointer<WalletChangeSetFFI>?
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr,
          let changesetPtr = changesetPtr else {
        return 0
    }

    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()

    let walletId = Data(bytes: walletIdPtr, count: 32)
    // Non-zero fails the round: `endChangeset(success: false)` rolls the
    // staged writes back and Rust keeps its in-memory state instead of
    // treating a partly-applied changeset as durable.
    return handler.persistWalletChangeset(walletId: walletId, changeset: changesetPtr) ? 0 : 1
}

/// C shim for the extension's `on_persist_wallet_changeset_sweeps_fn` —
/// the round's sweep batches, fired right after the changeset callback
/// above within the same begin/end bracket. Same non-zero-fails-the-round
/// contract: a removal Rust believes durable but that never landed would
/// replay the dead row at the next load.
private func persistWalletChangesetSweepsCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    sweepsPtr: UnsafePointer<SweepBatchFFI>?,
    sweepsCount: UInt
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }

    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()

    let walletId = Data(bytes: walletIdPtr, count: 32)
    return handler.persistWalletChangesetSweeps(
        walletId: walletId,
        sweeps: sweepsPtr,
        count: sweepsCount
    ) ? 0 : 1
}

/// C shim for `on_changeset_begin_fn`. Forwards to
/// `PlatformWalletPersistenceHandler.beginChangeset` so the handler
/// can prep any wallet-scope batching it needs for the round.
private func changesetBeginCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)
    handler.beginChangeset(walletId: walletId)
    return 0
}

/// C shim for `on_changeset_end_fn`. Forwards to
/// `PlatformWalletPersistenceHandler.endChangeset(walletId:success:)`,
/// which does the single `save()` (or `rollback()`) that commits all
/// per-kind writes accumulated during the round.
private func changesetEndCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    success: Bool
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)
    // Forward the commit outcome: a failed/rolled-back save returns non-zero so
    // Rust's `store()` reports a persistence failure (it would otherwise treat
    // the round as durably committed and clear its pending state).
    let committed = handler.endChangeset(walletId: walletId, success: success)
    return committed ? 0 : 1
}

private func persistSyncStateCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    syncHeight: UInt64,
    syncTimestamp: UInt64,
    lastKnownRecentBlock: UInt64
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }

    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()

    let walletId = Data(bytes: walletIdPtr, count: 32)
    handler.persistSyncState(
        walletId: walletId,
        syncHeight: syncHeight,
        syncTimestamp: syncTimestamp,
        lastKnownRecentBlock: lastKnownRecentBlock
    )
    return 0
}

/// C shim for `on_persist_account_registrations_fn`. Walks the
/// Rust-owned `[AccountSpecFFI]` slice and writes one
/// `PersistentAccount` row per entry. Replaces the legacy
/// per-entry `on_persist_account_fn` — same shape per row, but
/// the round arrives as a single batched callback so the whole
/// registration round flushes through one `store(...)` cycle on
/// the Rust side.
private func persistAccountRegistrationsCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    specsPtr: UnsafePointer<AccountSpecFFI>?,
    count: UInt
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)
    if count > 0, let specsPtr = specsPtr {
        for i in 0..<Int(count) {
            handler.persistAccount(walletId: walletId, spec: specsPtr[i])
        }
    }
    return 0
}

private func loadWalletListCallback(
    context: UnsafeMutableRawPointer?,
    outEntries: UnsafeMutablePointer<UnsafePointer<WalletRestoreEntryFFI>?>?,
    outCount: UnsafeMutablePointer<UInt>?
) -> Int32 {
    guard let context = context,
          let outEntries = outEntries,
          let outCount = outCount else {
        return 1
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let (entries, count, errored) = handler.loadWalletList()
    outEntries.pointee = entries
    outCount.pointee = UInt(count)
    // Surface SwiftData fetch failures as a non-zero callback return so
    // the Rust loader aborts instead of silently degrading to an empty
    // restore (which previously masked database faults as
    // "successful 0-balance restore").
    return errored ? 1 : 0
}

private func loadWalletListFreeCallback(
    context: UnsafeMutableRawPointer?,
    entries: UnsafePointer<WalletRestoreEntryFFI>?,
    _ count: UInt
) {
    guard let context = context else { return }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    handler.loadWalletListFree(entries: entries.map(UnsafeRawPointer.init))
}

/// C shim for `on_persist_account_address_pools_fn`. Walks the
/// Rust-owned `[AccountAddressPoolFFI]` slice and dispatches one
/// `persistAccountAddresses` call per pool. Replaces the legacy
/// per-pool `on_persist_account_addresses_fn` — same row shape
/// but batched into a single round so the whole registration
/// flushes through one Rust-side `store(...)` cycle.
private func persistAccountAddressPoolsCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    poolsPtr: UnsafePointer<AccountAddressPoolFFI>?,
    count: UInt
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)
    guard count > 0, let poolsPtr = poolsPtr else {
        return 0
    }

    var allOk = true
    for i in 0..<Int(count) {
        let pool = poolsPtr[i]
        let spec = pool.account
        var userIdentityId = Data(count: 32)
        withUnsafeBytes(of: spec.user_identity_id) { src in
            userIdentityId.withUnsafeMutableBytes { dst in dst.copyMemory(from: src) }
        }
        var friendIdentityId = Data(count: 32)
        withUnsafeBytes(of: spec.friend_identity_id) { src in
            friendIdentityId.withUnsafeMutableBytes { dst in dst.copyMemory(from: src) }
        }
        let key = PlatformWalletPersistenceHandler.AccountLookupKey(
            typeTag: UInt32(spec.type_tag),
            index: spec.index,
            standardTag: spec.standard_tag,
            registrationIndex: spec.registration_index,
            keyClass: spec.key_class,
            userIdentityId: userIdentityId,
            friendIdentityId: friendIdentityId
        )

        // Copy every C-string into a Swift String before leaving the
        // callback — Rust owns the underlying storage only for this window.
        var snapshots: [PlatformWalletPersistenceHandler.CoreAddressEntrySnapshot] = []
        snapshots.reserveCapacity(Int(pool.addresses_count))
        if pool.addresses_count > 0, let addressesPtr = pool.addresses_ptr {
            for j in 0..<Int(pool.addresses_count) {
                let entry = addressesPtr[j]
                let address = entry.address_base58.map { String(cString: $0) } ?? ""
                let derivationPath = entry.derivation_path.map { String(cString: $0) } ?? ""
                // Copy exactly `public_key_len` leading bytes out of the
                // 48-byte slot; `key_type_tag` records the curve. Pure
                // marshalling — the Rust side already validated the pair.
                let keyLen = Int(entry.public_key_len)
                let publicKey = keyLen > 0
                    ? withUnsafeBytes(of: entry.public_key) { Data($0.prefix(keyLen)) }
                    : Data()
                if address.isEmpty { continue }
                snapshots.append(.init(
                    address: address,
                    publicKey: publicKey,
                    keyType: entry.key_type_tag,
                    poolTypeTag: entry.pool_type_tag,
                    addressIndex: entry.address_index,
                    isUsed: entry.is_used,
                    balance: entry.balance,
                    derivationPath: derivationPath
                ))
            }
        }

        // Accumulate across ALL pools (do not early-return) so every pool is
        // persisted; then signal failure iff any pool's persist failed. Only an
        // `IdentityInvitation` pool ever returns false (see persistAccountAddresses),
        // so ordinary address-sync pools never wedge the round.
        if !handler.persistAccountAddresses(walletId: walletId, accountKey: key, entries: snapshots) {
            allOk = false
        }
    }

    return allOk ? 0 : 1
}

/// C shim for `on_persist_identities_fn`. Copies every
/// `IdentityEntryFFI` into an owned `IdentityEntrySnapshot` before
/// invoking the handler so the Rust-side free-loop can release
/// heap allocations the moment this closure returns.
///
/// Typed pointers arrive as `UnsafeRawPointer?` because
/// `@convention(c)` can't carry non-`@objc`-bridgeable typed Swift
/// pointers — we cast to the real layout via `assumingMemoryBound`
/// here on the Swift side. The Rust `#[repr(C)]` definitions match
/// the Swift struct layout byte-for-byte so the cast is sound.
private func persistIdentitiesCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    upsertsPtr: UnsafePointer<IdentityEntryFFI>?,
    upsertsCount: UInt,
    removedPtr: UnsafePointer<FFIByteTuple32>?,
    removedCount: UInt
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)

    var upserts: [PlatformWalletPersistenceHandler.IdentityEntrySnapshot] = []
    if upsertsCount > 0, let upsertsPtr = upsertsPtr {
        upserts.reserveCapacity(Int(upsertsCount))
        for i in 0..<Int(upsertsCount) {
            let e = upsertsPtr[i]
            let identityId = dataFromTuple32(e.identity_id)
            let walletIdField: Data? = e.wallet_id_is_some ? dataFromTuple32(e.wallet_id) : nil
            let identityIndexField: UInt32? = e.identity_index_is_some ? e.identity_index : nil

            // Walk the parallel DPNS arrays into Swift-owned
            // `(label, acquired_at)` tuples. Inner null pointers
            // (interior NUL labels — unreachable in practice) are
            // skipped without dropping the timestamp slot count, so
            // we keep iteration index-aligned. The Rust-side
            // `free_identity_entry_ffi` releases the C strings + the
            // outer arrays after this callback returns.
            var dpnsNames: [(label: String, acquiredAt: UInt64)] = []
            let dpnsCount = Int(e.dpns_names_count)
            if dpnsCount > 0,
               let labelsPtr = e.dpns_names,
               let acquiredPtr = e.dpns_names_acquired_at {
                dpnsNames.reserveCapacity(dpnsCount)
                for j in 0..<dpnsCount {
                    let labelPtr = labelsPtr[j]
                    let acquiredAt = acquiredPtr[j]
                    if let labelPtr = labelPtr {
                        let label = String(cString: labelPtr)
                        dpnsNames.append((label: label, acquiredAt: acquiredAt))
                    }
                }
            }

            // Walk the optional DashPay profile block. The
            // `dashpay_profile_present` bit is the single source of
            // truth: `false` means "no update for this flush"
            // (changeset-`None` semantics, NOT a delete signal), so
            // we carry `dashpayProfile == nil` through to the
            // handler. When the bit is set, every `*_present`
            // sub-flag is checked individually because zero-valued
            // payloads (empty strings, all-zero hashes /
            // fingerprints) are valid contract values and the FFI
            // would otherwise alias them to "absent".
            let dashpayProfile: PlatformWalletPersistenceHandler.DashpayProfileSnapshot?
            if e.dashpay_profile_present {
                let avatarHash: Data? = e.dashpay_profile_avatar_hash_present
                    ? hashData(e.dashpay_profile_avatar_hash)
                    : nil
                let avatarFingerprint: Data? = e.dashpay_profile_avatar_fingerprint_present
                    ? Swift.withUnsafeBytes(of: e.dashpay_profile_avatar_fingerprint) {
                        Data($0)
                    }
                    : nil
                dashpayProfile = PlatformWalletPersistenceHandler.DashpayProfileSnapshot(
                    displayName: e.dashpay_profile_display_name.map { String(cString: $0) },
                    bio: e.dashpay_profile_bio.map { String(cString: $0) },
                    publicMessage: e.dashpay_profile_public_message.map { String(cString: $0) },
                    avatarUrl: e.dashpay_profile_avatar_url.map { String(cString: $0) },
                    avatarHash: avatarHash,
                    avatarFingerprint: avatarFingerprint
                )
            } else {
                dashpayProfile = nil
            }

            // Walk the cached contact-profile rows into owned snapshots.
            // Rust projects a row per (re)fetched contact — present
            // profiles and `is_present == false` tombstones for
            // confirmed-absent ones. Each `*_present` sub-flag is checked
            // individually because zero-valued payloads (empty strings,
            // all-zero hashes / fingerprints) are valid contract values.
            // The Rust-side `free_identity_entry_ffi` releases the row
            // array + every C string after this callback returns.
            var contactProfiles:
                [PlatformWalletPersistenceHandler.ContactProfileSnapshot] = []
            let contactProfilesCount = Int(e.contact_profiles_count)
            if contactProfilesCount > 0, let rowsPtr = e.contact_profiles {
                contactProfiles.reserveCapacity(contactProfilesCount)
                for j in 0..<contactProfilesCount {
                    let row = rowsPtr[j]
                    let avatarHash: Data? = row.avatar_hash_present
                        ? hashData(row.avatar_hash)
                        : nil
                    let avatarFingerprint: Data? = row.avatar_fingerprint_present
                        ? Swift.withUnsafeBytes(of: row.avatar_fingerprint) { Data($0) }
                        : nil
                    contactProfiles.append(
                        .init(
                            contactIdentityId: dataFromTuple32(row.contact_id),
                            isPresent: row.is_present,
                            displayName: row.display_name.map { String(cString: $0) },
                            bio: row.bio.map { String(cString: $0) },
                            publicMessage: row.public_message.map { String(cString: $0) },
                            avatarUrl: row.avatar_url.map { String(cString: $0) },
                            avatarHash: avatarHash,
                            avatarFingerprint: avatarFingerprint,
                            checkedAtMs: row.checked_at_ms
                        )
                    )
                }
            }

            upserts.append(.init(
                identityId: identityId,
                balance: e.balance,
                revision: e.revision,
                identityIndex: identityIndexField,
                // Label is no longer carried over the FFI — Swift
                // owns `PersistentIdentity.alias` directly.
                label: nil,
                status: e.status,
                walletId: walletIdField,
                dpnsNames: dpnsNames,
                dashpayProfile: dashpayProfile,
                contactProfiles: contactProfiles
            ))
        }
    }

    var removed: [Data] = []
    if removedCount > 0, let removedPtr = removedPtr {
        removed.reserveCapacity(Int(removedCount))
        for i in 0..<Int(removedCount) {
            removed.append(dataFromTuple32(removedPtr[i]))
        }
    }

    handler.persistIdentities(
        walletId: walletId,
        upserts: upserts,
        removed: removed
    )
    return 0
}

/// C shim for `on_persist_identity_keys_fn`. Same snapshot + cast
/// pattern as the identities callback.
private func persistIdentityKeysCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    upsertsPtr: UnsafePointer<IdentityKeyEntryFFI>?,
    upsertsCount: UInt,
    removedPtr: UnsafePointer<IdentityKeyRemovalFFI>?,
    removedCount: UInt
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)

    var upserts: [PlatformWalletPersistenceHandler.IdentityKeyEntrySnapshot] = []
    if upsertsCount > 0, let upsertsPtr = upsertsPtr {
        upserts.reserveCapacity(Int(upsertsCount))
        for i in 0..<Int(upsertsCount) {
            let e = upsertsPtr[i]
            let identityId = dataFromTuple32(e.identity_id)
            let pubKey: Data
            if let ptr = e.public_key_data_ptr, e.public_key_data_len > 0 {
                pubKey = Data(bytes: ptr, count: Int(e.public_key_data_len))
            } else {
                pubKey = Data()
            }
            let walletId: Data? = e.wallet_id_is_some ? dataFromTuple32(e.wallet_id) : nil
            let indices: (identityIndex: UInt32, keyIndex: UInt32)? =
                e.derivation_indices_is_some
                    ? (e.identity_index, e.key_index)
                    : nil

            // Project the contract-bounds trio (kind / id / doc-
            // type C-string) into the Swift enum. Mirrors the Rust
            // `IdentityKeyEntryFFI::from_entry` encoding — kinds:
            //   0 → no bounds
            //   1 → SingleContract { id }
            //   2 → SingleContractDocumentType { id, doc_type_name }
            // The doc-type C-string for kind=2 is owned by Rust and
            // freed via `free_identity_key_entry_ffi` after this
            // callback returns, so we copy it into a Swift String
            // here. A null doc-type pointer with kind=2 is a
            // serialization edge case (interior NUL); treat it as
            // no-bounds rather than constructing a partial variant.
            let bounds: ManagedPlatformWallet.ContractBounds?
            switch e.contract_bounds_kind {
            case 1:
                bounds = .singleContract(id: dataFromTuple32(e.contract_bounds_id))
            case 2:
                if let docPtr = e.contract_bounds_document_type {
                    bounds = .singleContractDocumentType(
                        id: dataFromTuple32(e.contract_bounds_id),
                        documentTypeName: String(cString: docPtr)
                    )
                } else {
                    bounds = nil
                }
            default:
                bounds = nil
            }

            upserts.append(.init(
                identityId: identityId,
                keyId: e.key_id,
                purpose: e.purpose,
                securityLevel: e.security_level,
                keyType: e.key_type,
                readOnly: e.read_only,
                disabledAt: e.disabled_at_is_some ? e.disabled_at : nil,
                publicKeyData: pubKey,
                publicKeyHash: dataFromTuple20(e.public_key_hash),
                walletId: walletId,
                derivationIndices: indices,
                contractBounds: bounds
            ))
        }
    }

    var removed: [(identityId: Data, keyId: UInt32)] = []
    if removedCount > 0, let removedPtr = removedPtr {
        removed.reserveCapacity(Int(removedCount))
        for i in 0..<Int(removedCount) {
            let r = removedPtr[i]
            removed.append((identityId: dataFromTuple32(r.identity_id), keyId: r.key_id))
        }
    }

    handler.persistIdentityKeys(walletId: walletId, upserts: upserts, removed: removed)
    return 0
}

/// C shim for `on_persist_token_balances_fn`. Same snapshot + cast
/// pattern as the identities callback — copies every
/// `TokenBalanceUpsertFFI` / `TokenBalanceRemovalFFI` into an owned
/// Swift snapshot before invoking the handler so the callback can
/// return immediately even if the receiver dispatches asynchronously.
private func persistTokenBalancesCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    upsertsPtr: UnsafePointer<TokenBalanceUpsertFFI>?,
    upsertsCount: UInt,
    removedPtr: UnsafePointer<TokenBalanceRemovalFFI>?,
    removedCount: UInt
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)

    var upserts: [PlatformWalletPersistenceHandler.TokenBalanceUpsertSnapshot] = []
    if upsertsCount > 0, let upsertsPtr = upsertsPtr {
        upserts.reserveCapacity(Int(upsertsCount))
        for i in 0..<Int(upsertsCount) {
            let e = upsertsPtr[i]
            upserts.append(.init(
                identityId: dataFromTuple32(e.identity_id),
                tokenId: dataFromTuple32(e.token_id),
                balance: e.balance
            ))
        }
    }

    var removals: [PlatformWalletPersistenceHandler.TokenBalanceRemovalSnapshot] = []
    if removedCount > 0, let removedPtr = removedPtr {
        removals.reserveCapacity(Int(removedCount))
        for i in 0..<Int(removedCount) {
            let r = removedPtr[i]
            removals.append(.init(
                identityId: dataFromTuple32(r.identity_id),
                tokenId: dataFromTuple32(r.token_id)
            ))
        }
    }

    handler.persistTokenBalances(walletId: walletId, upserts: upserts, removals: removals)
    return 0
}

/// C shim for `on_persist_invitations_fn`. Deep-copies every all-POD
/// `InvitationEntryFFI` row into an owned `InvitationEntrySnapshot` (precomputing
/// `rawOutPoint` + the `encodeOutPoint` display key) and every removed-outpoint
/// tuple into owned `Data` before invoking the handler, so the Rust side can
/// reclaim its buffers the moment we return. Mirrors `persistAssetLocksCallback`.
/// Returns 0 when every invitation mutation was staged successfully. Returns
/// nonzero when any write was skipped, which fails the Rust persistence round
/// and rolls back the changeset — safe here because the invitation round is
/// invitation-only, so no unrelated writes are discarded — and lets
/// `create_invitation` surface the failure instead of reporting a voucher that
/// never reached SwiftData.
private func persistInvitationsCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    upsertsPtr: UnsafePointer<InvitationEntryFFI>?,
    upsertsCount: UInt,
    removedPtr: UnsafePointer<FFIByteTuple36>?,
    removedCount: UInt
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)

    var upserts: [PlatformWalletPersistenceHandler.InvitationEntrySnapshot] = []
    if upsertsCount > 0, let upsertsPtr = upsertsPtr {
        upserts.reserveCapacity(Int(upsertsCount))
        for i in 0..<Int(upsertsCount) {
            let e = upsertsPtr[i]
            // Outpoint tuple → 36-byte raw Data → display-order hex key.
            let outPointRaw = Swift.withUnsafeBytes(of: e.out_point) { Data($0) }
            let outPointHex = PersistentAssetLock.encodeOutPoint(rawBytes: outPointRaw)
            upserts.append(.init(
                outPointHex: outPointHex,
                rawOutPoint: outPointRaw,
                fundingIndexRaw: Int(e.funding_index),
                amountDuffs: Int64(bitPattern: e.amount_duffs),
                expiryUnix: Int(e.expiry_unix),
                createdAtSecs: Int(e.created_at_secs),
                hasInviter: e.has_inviter != 0,
                statusRaw: Int(e.status)
            ))
        }
    }

    var removed: [Data] = []
    if removedCount > 0, let removedPtr = removedPtr {
        removed.reserveCapacity(Int(removedCount))
        for i in 0..<Int(removedCount) {
            var tuple = removedPtr[i]
            let bytes = Swift.withUnsafeBytes(of: &tuple) { Data($0) }
            removed.append(bytes)
        }
    }

    // Signal failure (nonzero) so the Rust `store()` returns Err and
    // `create_invitation` surfaces a funded-but-unrecorded voucher instead of
    // reporting success.
    return handler.persistInvitations(walletId: walletId, upserts: upserts, removed: removed) ? 0 : 1
}

/// C shim for `on_persist_dpns_name_states_fn`. Deep-copies every
/// `DpnsNameStateFFI` row — including its three Rust-owned C strings —
/// and every removed document-id tuple into owned Swift values before
/// invoking the handler, so Rust can run its string free-loop the moment
/// we return.
///
/// Returns 0 when every marketplace mutation was staged. A nonzero return
/// fails the Rust persistence round and rolls the changeset back, which is
/// reserved for genuine SwiftData fetch failures — a row whose identity
/// isn't staged yet is skipped and re-emitted by the next sync pass rather
/// than discarding the round's other writes.
private func persistDpnsNameStatesCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    rowsPtr: UnsafePointer<DpnsNameStateFFI>?,
    rowsCount: UInt,
    removedPtr: UnsafePointer<FFIByteTuple32>?,
    removedCount: UInt
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)

    var upserts: [PlatformWalletPersistenceHandler.DpnsNameStateSnapshot] = []
    if rowsCount > 0, let rowsPtr = rowsPtr {
        upserts.reserveCapacity(Int(rowsCount))
        for i in 0..<Int(rowsCount) {
            let r = rowsPtr[i]
            let documentId = dataFromTuple32(r.document_id)
            let counterparty: String? = r.has_counterparty
                ? dataFromTuple32(r.counterparty_id).toBase58String()
                : nil
            upserts.append(.init(
                documentIdBase58: documentId.toBase58String(),
                walletIdentityId: dataFromTuple32(r.wallet_identity_id),
                label: r.label.map { String(cString: $0) } ?? "",
                normalizedLabel: r.normalized_label.map { String(cString: $0) } ?? "",
                normalizedParentDomainName: r.normalized_parent_domain_name
                    .map { String(cString: $0) } ?? "",
                priceCredits: r.has_price ? r.price : nil,
                statusRaw: Int16(r.status),
                counterpartyIdBase58: counterparty,
                createdAtMs: r.created_at_ms == 0 ? nil : r.created_at_ms,
                updatedAtMs: r.updated_at_ms == 0 ? nil : r.updated_at_ms,
                transferredAtMs: r.transferred_at_ms == 0 ? nil : r.transferred_at_ms,
                lastSyncedAtMs: r.last_synced_at_ms
            ))
        }
    }

    var removed: [String] = []
    if removedCount > 0, let removedPtr = removedPtr {
        removed.reserveCapacity(Int(removedCount))
        for i in 0..<Int(removedCount) {
            removed.append(dataFromTuple32(removedPtr[i]).toBase58String())
        }
    }

    // A row whose normalized label didn't survive the C-string copy has
    // no usable uniqueness key, so it would upsert onto the wrong row.
    // Drop it here rather than corrupting the cache.
    let usable = upserts.filter { !$0.normalizedLabel.isEmpty }
    if usable.count != upserts.count {
        print("⚠️ persistDpnsNameStates: dropped \(upserts.count - usable.count) marketplace row(s) with an unreadable normalized label")
    }
    if usable.isEmpty && removed.isEmpty {
        return 0
    }
    return handler.persistDpnsNameStates(walletId: walletId, upserts: usable, removed: removed)
        ? 0 : 1
}

/// C shim for `on_persist_asset_locks_fn`. Copies every
/// `AssetLockEntryFFI` row + every removed-outpoint tuple into
/// Swift-owned `Data` snapshots before invoking the handler so the
/// Rust-side `_storage` Vec can release the byte buffers as soon as
/// this trampoline returns.
private func persistAssetLocksCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    upsertsPtr: UnsafePointer<AssetLockEntryFFI>?,
    upsertsCount: UInt,
    removedPtr: UnsafePointer<FFIByteTuple36>?,
    removedCount: UInt
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)

    var upserts: [PlatformWalletPersistenceHandler.AssetLockEntrySnapshot] = []
    if upsertsCount > 0, let upsertsPtr = upsertsPtr {
        upserts.reserveCapacity(Int(upsertsCount))
        for i in 0..<Int(upsertsCount) {
            let e = upsertsPtr[i]
            // Outpoint tuple → 36-byte raw → display-order hex string.
            let outPointRaw = Swift.withUnsafeBytes(of: e.out_point) { Data($0) }
            let outPointHex = PersistentAssetLock.encodeOutPoint(rawBytes: outPointRaw)
            // Consensus-encoded transaction bytes.
            let txBytes: Data
            if let ptr = e.transaction_bytes, e.transaction_bytes_len > 0 {
                txBytes = Data(bytes: ptr, count: Int(e.transaction_bytes_len))
            } else {
                txBytes = Data()
            }
            // Optional bincode-encoded proof.
            let proofBytes: Data?
            if let ptr = e.proof_bytes, e.proof_bytes_len > 0 {
                proofBytes = Data(bytes: ptr, count: Int(e.proof_bytes_len))
            } else {
                proofBytes = nil
            }
            upserts.append(.init(
                outPointHex: outPointHex,
                transactionBytes: txBytes,
                fundingTypeRaw: Int(e.funding_type),
                identityIndexRaw: Int32(bitPattern: e.identity_index),
                accountIndexRaw: Int32(bitPattern: e.account_index),
                amountDuffs: Int64(bitPattern: e.amount_duffs),
                statusRaw: Int(e.status),
                proofBytes: proofBytes
            ))
        }
    }

    var removed: [Data] = []
    if removedCount > 0, let removedPtr = removedPtr {
        removed.reserveCapacity(Int(removedCount))
        for i in 0..<Int(removedCount) {
            var tuple = removedPtr[i]
            let bytes = Swift.withUnsafeBytes(of: &tuple) { Data($0) }
            removed.append(bytes)
        }
    }

    handler.persistAssetLocks(walletId: walletId, upserts: upserts, removed: removed)
    return 0
}

/// C shim for `on_persist_contacts_fn`. Same snapshot + cast pattern
/// as the identities callback — copies every `ContactRequestFFI` /
/// `ContactRequestRemovalFFI` row into Swift-owned tuples before
/// invoking the handler so the matching `free_contact_requests_ffi`
/// pass on the Rust side runs cleanly the moment we return.
///
/// The `removed_sent` and `removed_incoming` arrays come in as two
/// parallel `*const ContactRequestRemovalFFI` slots; we keep them
/// separate through the snapshot too because the handler uses the
/// arrival bucket to decide which `is_outgoing` row to delete.
///
/// The trailing `ignored` array carries the per-sender ignore deltas —
/// POD rows (no heap payloads), copied into snapshots like everything
/// else. Each row's `is_ignored` bit says persist (ignore) vs delete
/// (un-ignore).
private func persistContactsCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    upsertsPtr: UnsafePointer<ContactRequestFFI>?,
    upsertsCount: UInt,
    removedSentPtr: UnsafePointer<ContactRequestRemovalFFI>?,
    removedSentCount: UInt,
    removedIncomingPtr: UnsafePointer<ContactRequestRemovalFFI>?,
    removedIncomingCount: UInt,
    ignoredPtr: UnsafePointer<ContactIgnoredSenderFFI>?,
    ignoredCount: UInt
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)

    var upserts: [PlatformWalletPersistenceHandler.ContactRequestSnapshot] = []
    if upsertsCount > 0, let upsertsPtr = upsertsPtr {
        upserts.reserveCapacity(Int(upsertsCount))
        for i in 0..<Int(upsertsCount) {
            let e = upsertsPtr[i]
            // `encrypted_public_key` is always populated on a
            // well-formed `ContactRequest` document — Rust's
            // `allocate_byte_buffer` only returns `(null, 0)` for an
            // empty slice, which is invalid for this field. Treat a
            // null pointer here defensively as an empty `Data` rather
            // than panicking; the unique constraint still upserts the
            // row, the row just won't decrypt.
            let encryptedPublicKey: Data
            if let pkPtr = e.encrypted_public_key, e.encrypted_public_key_len > 0 {
                encryptedPublicKey = Data(bytes: pkPtr, count: Int(e.encrypted_public_key_len))
            } else {
                encryptedPublicKey = Data()
            }
            let encryptedAccountLabel: Data?
            if let labelPtr = e.encrypted_account_label, e.encrypted_account_label_len > 0 {
                encryptedAccountLabel = Data(
                    bytes: labelPtr,
                    count: Int(e.encrypted_account_label_len)
                )
            } else {
                encryptedAccountLabel = nil
            }
            let autoAcceptProof: Data?
            if let proofPtr = e.auto_accept_proof, e.auto_accept_proof_len > 0 {
                autoAcceptProof = Data(bytes: proofPtr, count: Int(e.auto_accept_proof_len))
            } else {
                autoAcceptProof = nil
            }
            let acceptedAccounts: [UInt32]
            if let acceptedPtr = e.accepted_accounts, e.accepted_accounts_len > 0 {
                acceptedAccounts = Array(
                    UnsafeBufferPointer(start: acceptedPtr, count: Int(e.accepted_accounts_len))
                )
            } else {
                acceptedAccounts = []
            }

            upserts.append(.init(
                ownerIdentityId: dataFromTuple32(e.owner_id),
                contactIdentityId: dataFromTuple32(e.contact_id),
                isOutgoing: e.is_outgoing,
                senderKeyIndex: e.sender_key_index,
                recipientKeyIndex: e.recipient_key_index,
                accountReference: e.account_reference,
                encryptedPublicKey: encryptedPublicKey,
                encryptedAccountLabel: encryptedAccountLabel,
                autoAcceptProof: autoAcceptProof,
                coreHeightCreatedAt: e.core_height_created_at,
                createdAtMillis: e.created_at,
                paymentChannelBroken: e.payment_channel_broken,
                contactAlias: e.alias.map { String(cString: $0) },
                contactNote: e.note.map { String(cString: $0) },
                contactHidden: e.is_hidden,
                contactAccountLabel: e.contact_account_label.map { String(cString: $0) },
                contactAcceptedAccounts: acceptedAccounts
            ))
        }
    }

    var removedSent: [PlatformWalletPersistenceHandler.ContactRequestRemovalSnapshot] = []
    if removedSentCount > 0, let removedSentPtr = removedSentPtr {
        removedSent.reserveCapacity(Int(removedSentCount))
        for i in 0..<Int(removedSentCount) {
            let r = removedSentPtr[i]
            removedSent.append(.init(
                ownerIdentityId: dataFromTuple32(r.owner_id),
                contactIdentityId: dataFromTuple32(r.contact_id)
            ))
        }
    }

    var removedIncoming: [PlatformWalletPersistenceHandler.ContactRequestRemovalSnapshot] = []
    if removedIncomingCount > 0, let removedIncomingPtr = removedIncomingPtr {
        removedIncoming.reserveCapacity(Int(removedIncomingCount))
        for i in 0..<Int(removedIncomingCount) {
            let r = removedIncomingPtr[i]
            removedIncoming.append(.init(
                ownerIdentityId: dataFromTuple32(r.owner_id),
                contactIdentityId: dataFromTuple32(r.contact_id)
            ))
        }
    }

    var ignored: [PlatformWalletPersistenceHandler.ContactIgnoredSenderSnapshot] = []
    if ignoredCount > 0, let ignoredPtr = ignoredPtr {
        ignored.reserveCapacity(Int(ignoredCount))
        for i in 0..<Int(ignoredCount) {
            let r = ignoredPtr[i]
            ignored.append(.init(
                ownerIdentityId: dataFromTuple32(r.owner_id),
                senderIdentityId: dataFromTuple32(r.sender_id),
                isIgnored: r.is_ignored
            ))
        }
    }

    handler.persistContacts(
        walletId: walletId,
        upserts: upserts,
        removedSent: removedSent,
        removedIncoming: removedIncoming,
        ignored: ignored
    )
    return 0
}

/// Copy a fixed 32-byte C tuple into an owned `Data`. Used by the
/// identity-persistence callbacks where Rust hands over `[u8; 32]`
/// fields as `(UInt8, UInt8, ...)` tuples.
@inline(__always)
private func dataFromTuple32(_ tuple: FFIByteTuple32) -> Data {
    var value = tuple
    return Swift.withUnsafeBytes(of: &value) { Data($0) }
}

/// Copy a fixed 20-byte C tuple into an owned `Data`. Identical
/// idiom to `dataFromTuple32`, just for RIPEMD160(SHA256) pubkey
/// hashes on identity-key entries.
@inline(__always)
private func dataFromTuple20(_ tuple: FFIByteTuple20) -> Data {
    var value = tuple
    return Swift.withUnsafeBytes(of: &value) { Data($0) }
}

private func persistWalletMetadataCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    network: FFINetwork,
    walletGroupIdPtr: UnsafePointer<UInt8>?,
    birthHeight: UInt32
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)
    let walletGroupId = walletGroupIdPtr.map { Data(bytes: $0, count: 32) } ?? Data()
    handler.persistWalletMetadata(
        walletId: walletId,
        network: Network(ffiNetwork: network),
        walletGroupId: walletGroupId,
        birthHeight: birthHeight
    )
    return 0
}

// MARK: - Shielded persistence (Orchard)
//
// Mirror of the four `on_persist_shielded_*_fn` callbacks declared
// in `rs-platform-wallet-ffi/src/persistence.rs` plus the matching
// load callbacks used at boot to rehydrate `SubwalletState`s.

private func persistShieldedNotesCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    entriesPtr: UnsafePointer<ShieldedNoteFFI>?,
    count: UInt
) -> Int32 {
    guard let context = context, let walletIdPtr = walletIdPtr else { return 0 }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)

    var snapshots: [PlatformWalletPersistenceHandler.ShieldedNoteSnapshot] = []
    if count > 0, let entriesPtr = entriesPtr {
        snapshots.reserveCapacity(Int(count))
        for i in 0..<Int(count) {
            let e = entriesPtr[i]
            let noteData: Data
            if let dataPtr = e.note_data_ptr, e.note_data_len > 0 {
                noteData = Data(bytes: dataPtr, count: Int(e.note_data_len))
            } else {
                noteData = Data()
            }
            snapshots.append(.init(
                walletId: dataFromTuple32(e.wallet_id),
                accountIndex: e.account_index,
                position: e.position,
                cmx: dataFromTuple32(e.cmx),
                nullifier: dataFromTuple32(e.nullifier),
                blockHeight: e.block_height,
                isSpent: e.is_spent != 0,
                value: e.value,
                noteData: noteData
            ))
        }
    }
    handler.persistShieldedNotes(walletId: walletId, snapshots: snapshots)
    return 0
}

private func persistShieldedNullifiersSpentCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    entriesPtr: UnsafePointer<ShieldedNullifierSpentFFI>?,
    count: UInt
) -> Int32 {
    guard let context = context, let walletIdPtr = walletIdPtr else { return 0 }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)

    var entries: [(walletId: Data, accountIndex: UInt32, nullifier: Data)] = []
    if count > 0, let entriesPtr = entriesPtr {
        entries.reserveCapacity(Int(count))
        for i in 0..<Int(count) {
            let e = entriesPtr[i]
            entries.append((
                walletId: dataFromTuple32(e.wallet_id),
                accountIndex: e.account_index,
                nullifier: dataFromTuple32(e.nullifier)
            ))
        }
    }
    handler.persistShieldedNullifiersSpent(walletId: walletId, entries: entries)
    return 0
}

private func persistShieldedOutgoingNotesCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    entriesPtr: UnsafePointer<ShieldedOutgoingNoteFFI>?,
    count: UInt
) -> Int32 {
    guard let context = context, let walletIdPtr = walletIdPtr else { return 0 }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)

    var snapshots: [PlatformWalletPersistenceHandler.ShieldedOutgoingNoteSnapshot] = []
    if count > 0, let entriesPtr = entriesPtr {
        snapshots.reserveCapacity(Int(count))
        for i in 0..<Int(count) {
            let e = entriesPtr[i]
            // Copy the `recipient` (43-byte fixed tuple) and `memo`
            // (Rust-owned heap buffer) out now — both pointers are
            // only valid for this callback window.
            let recipient = Swift.withUnsafeBytes(of: e.recipient) { Data($0) }
            let memo: Data
            if let memoPtr = e.memo_ptr, e.memo_len > 0 {
                memo = Data(bytes: memoPtr, count: Int(e.memo_len))
            } else {
                memo = Data()
            }
            snapshots.append(.init(
                walletId: dataFromTuple32(e.wallet_id),
                accountIndex: e.account_index,
                cmx: dataFromTuple32(e.cmx),
                recipient: recipient,
                value: e.value,
                memo: memo,
                blockHeight: e.block_height
            ))
        }
    }
    handler.persistShieldedOutgoingNotes(walletId: walletId, snapshots: snapshots)
    return 0
}

private func persistShieldedActivityCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    entriesPtr: UnsafePointer<ShieldedActivityFFI>?,
    count: UInt
) -> Int32 {
    guard let context = context, let walletIdPtr = walletIdPtr else { return 0 }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)

    var snapshots: [PlatformWalletPersistenceHandler.ShieldedActivitySnapshot] = []
    if count > 0, let entriesPtr = entriesPtr {
        snapshots.reserveCapacity(Int(count))
        for i in 0..<Int(count) {
            let e = entriesPtr[i]
            // Copy every pointer-backed field out now — the Rust
            // pointers are only valid for this callback window.
            func data(_ ptr: UnsafePointer<UInt8>?, _ len: UInt) -> Data {
                if let ptr = ptr, len > 0 { return Data(bytes: ptr, count: Int(len)) }
                return Data()
            }
            let counterparty = data(e.counterparty_ptr, UInt(e.counterparty_len))
            let memo = data(e.memo_ptr, UInt(e.memo_len))
            // The counts are Rust-supplied: use checked multiplication so
            // a corrupt row degrades to an empty linkage instead of a
            // trapped overflow crashing the callback path.
            func byteLen(_ count: UInt) -> UInt {
                let (value, overflow) = count.multipliedReportingOverflow(by: 32)
                return overflow ? 0 : value
            }
            let noteCmxs = data(e.note_cmxs_ptr, byteLen(UInt(e.note_cmxs_count)))
            let spentNullifiers = data(e.spent_nullifiers_ptr, byteLen(UInt(e.spent_nullifiers_count)))
            let identityId = e.has_identity_id != 0 ? dataFromTuple32(e.identity_id) : Data()
            snapshots.append(.init(
                walletId: dataFromTuple32(e.wallet_id),
                accountIndex: e.account_index,
                entryId: dataFromTuple32(e.entry_id),
                kindTag: Int(e.kind_tag),
                direction: Int(e.direction),
                status: Int(e.status),
                amount: e.amount,
                fee: e.fee,
                hasFee: e.has_fee != 0,
                blockHeight: e.block_height,
                hasBlockHeight: e.has_block_height != 0,
                createdAtMs: e.created_at_ms,
                minNotePosition: e.min_note_position,
                hasMinNotePosition: e.has_min_note_position != 0,
                identityId: identityId,
                counterparty: counterparty,
                memo: memo,
                noteCmxs: noteCmxs,
                spentNullifiers: spentNullifiers
            ))
        }
    }
    handler.persistShieldedActivity(walletId: walletId, snapshots: snapshots)
    return 0
}

private func persistShieldedSyncedIndicesCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    entriesPtr: UnsafePointer<ShieldedSyncedIndexFFI>?,
    count: UInt
) -> Int32 {
    guard let context = context, let walletIdPtr = walletIdPtr else { return 0 }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)

    var entries: [(walletId: Data, accountIndex: UInt32, lastSyncedIndex: UInt64)] = []
    if count > 0, let entriesPtr = entriesPtr {
        entries.reserveCapacity(Int(count))
        for i in 0..<Int(count) {
            let e = entriesPtr[i]
            entries.append((
                walletId: dataFromTuple32(e.wallet_id),
                accountIndex: e.account_index,
                lastSyncedIndex: e.last_synced_index
            ))
        }
    }
    handler.persistShieldedSyncedIndices(walletId: walletId, entries: entries)
    return 0
}

private func loadShieldedNotesCallback(
    context: UnsafeMutableRawPointer?,
    outEntries: UnsafeMutablePointer<UnsafePointer<ShieldedNoteRestoreFFI>?>?,
    outCount: UnsafeMutablePointer<UInt>?
) -> Int32 {
    guard let context = context, let outEntries = outEntries, let outCount = outCount else {
        return 1
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let (entries, count, errored) = handler.loadShieldedNotes()
    outEntries.pointee = entries
    outCount.pointee = UInt(count)
    return errored ? 1 : 0
}

private func loadShieldedNotesFreeCallback(
    context: UnsafeMutableRawPointer?,
    entries: UnsafePointer<ShieldedNoteRestoreFFI>?,
    _ count: UInt
) {
    guard let context = context else { return }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    handler.loadShieldedNotesFree(entries: entries.map(UnsafeRawPointer.init))
}

private func loadShieldedOutgoingNotesCallback(
    context: UnsafeMutableRawPointer?,
    outEntries: UnsafeMutablePointer<UnsafePointer<ShieldedOutgoingNoteRestoreFFI>?>?,
    outCount: UnsafeMutablePointer<UInt>?
) -> Int32 {
    guard let context = context, let outEntries = outEntries, let outCount = outCount else {
        return 1
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let (entries, count, errored) = handler.loadShieldedOutgoingNotes()
    outEntries.pointee = entries
    outCount.pointee = UInt(count)
    return errored ? 1 : 0
}

private func loadShieldedOutgoingNotesFreeCallback(
    context: UnsafeMutableRawPointer?,
    entries: UnsafePointer<ShieldedOutgoingNoteRestoreFFI>?,
    _ count: UInt
) {
    guard let context = context else { return }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    handler.loadShieldedOutgoingNotesFree(entries: entries.map(UnsafeRawPointer.init))
}

private func loadShieldedActivityCallback(
    context: UnsafeMutableRawPointer?,
    outEntries: UnsafeMutablePointer<UnsafePointer<ShieldedActivityRestoreFFI>?>?,
    outCount: UnsafeMutablePointer<UInt>?
) -> Int32 {
    guard let context = context, let outEntries = outEntries, let outCount = outCount else {
        return 1
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let (entries, count, errored) = handler.loadShieldedActivity()
    outEntries.pointee = entries
    outCount.pointee = UInt(count)
    return errored ? 1 : 0
}

private func loadShieldedActivityFreeCallback(
    context: UnsafeMutableRawPointer?,
    entries: UnsafePointer<ShieldedActivityRestoreFFI>?,
    _ count: UInt
) {
    guard let context = context else { return }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    handler.loadShieldedActivityFree(entries: entries.map(UnsafeRawPointer.init))
}

private func loadShieldedSyncStatesCallback(
    context: UnsafeMutableRawPointer?,
    outEntries: UnsafeMutablePointer<UnsafePointer<ShieldedSubwalletSyncStateFFI>?>?,
    outCount: UnsafeMutablePointer<UInt>?
) -> Int32 {
    guard let context = context, let outEntries = outEntries, let outCount = outCount else {
        return 1
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let (entries, count, errored) = handler.loadShieldedSyncStates()
    outEntries.pointee = entries
    outCount.pointee = UInt(count)
    return errored ? 1 : 0
}

private func loadShieldedSyncStatesFreeCallback(
    context: UnsafeMutableRawPointer?,
    entries: UnsafePointer<ShieldedSubwalletSyncStateFFI>?,
    _ count: UInt
) {
    guard let context = context else { return }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    handler.loadShieldedSyncStatesFree(entries: entries.map(UnsafeRawPointer.init))
}

private func persistShieldedViewingKeysCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    entriesPtr: UnsafePointer<ShieldedViewingKeyFFI>?,
    count: UInt
) -> Int32 {
    guard let context = context, let walletIdPtr = walletIdPtr else { return 0 }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)

    var entries: [(walletId: Data, accountIndex: UInt32, fvkBytes: Data)] = []
    if count > 0, let entriesPtr = entriesPtr {
        entries.reserveCapacity(Int(count))
        for i in 0..<Int(count) {
            let e = entriesPtr[i]
            let fvkBytes = Swift.withUnsafeBytes(of: e.fvk_bytes) { Data($0) }
            entries.append((
                walletId: dataFromTuple32(e.wallet_id),
                accountIndex: e.account_index,
                fvkBytes: fvkBytes
            ))
        }
    }
    handler.persistShieldedViewingKeys(walletId: walletId, entries: entries)
    return 0
}

private func loadShieldedViewingKeysCallback(
    context: UnsafeMutableRawPointer?,
    outEntries: UnsafeMutablePointer<UnsafePointer<ShieldedViewingKeyRestoreFFI>?>?,
    outCount: UnsafeMutablePointer<UInt>?
) -> Int32 {
    guard let context = context, let outEntries = outEntries, let outCount = outCount else {
        return 1
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let (entries, count, errored) = handler.loadShieldedViewingKeys()
    outEntries.pointee = entries
    outCount.pointee = UInt(count)
    return errored ? 1 : 0
}

private func loadShieldedViewingKeysFreeCallback(
    context: UnsafeMutableRawPointer?,
    entries: UnsafePointer<ShieldedViewingKeyRestoreFFI>?,
    _ count: UInt
) {
    guard let context = context else { return }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    handler.loadShieldedViewingKeysFree(entries: entries.map(UnsafeRawPointer.init))
}

// MARK: - Core tx-record persister fallback

/// C shim for `on_get_core_tx_record_fn`. Calls
/// `PlatformWalletPersistenceHandler.coreTxRecord(...)` and writes
/// the row's actual context kind, block info (when applicable), and
/// raw transaction bytes to the Rust-owned output pointers.
///
/// The transaction bytes are allocated here via
/// `UnsafeMutablePointer<UInt8>.allocate(capacity:)` and the
/// allocation is owned by the Rust side until it invokes
/// `getCoreTxRecordFreeCallback` below — Rust calls free exactly
/// once per hit.
///
/// Output contract:
/// - Sets `*outFound = true` and populates `outContextKind` (and
///   the three block fields when context is 2 or 3, plus the tx
///   bytes pointer + length) on a hit; returns `0`.
/// - Sets `*outFound = false` on a miss; returns `0`.
/// - Returns `0` even on Swift-side errors (treated as miss); the
///   Rust side's `record_or_persister` helper logs and falls
///   through to the caller's existing not-found / poll path.
private func getCoreTxRecordCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    txidPtr: UnsafePointer<UInt8>?,
    outContextKind: UnsafeMutablePointer<UInt8>?,
    outBlockHeight: UnsafeMutablePointer<UInt32>?,
    outBlockHash: UnsafeMutablePointer<UInt8>?,
    outBlockTimestamp: UnsafeMutablePointer<UInt32>?,
    outTxBytes: UnsafeMutablePointer<UnsafePointer<UInt8>?>?,
    outTxBytesLen: UnsafeMutablePointer<UInt>?,
    outFound: UnsafeMutablePointer<Bool>?
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr,
          let txidPtr = txidPtr,
          let outFound = outFound else {
        return 0
    }
    outFound.pointee = false
    outTxBytes?.pointee = nil
    outTxBytesLen?.pointee = 0

    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)
    let txid = Data(bytes: txidPtr, count: 32)

    guard let row = handler.coreTxRecord(walletId: walletId, txid: txid) else {
        // Miss — outFound already set to false above.
        return 0
    }

    outContextKind?.pointee = UInt8(row.context)
    outBlockHeight?.pointee = row.blockHeight
    outBlockTimestamp?.pointee = row.blockTimestamp
    if let outBlockHash = outBlockHash {
        // `coreTxRecord` returns a 32-byte `blockHash` (real for
        // in-block / chain-locked rows, zeroed placeholder for
        // mempool / IS rows that the Rust side will ignore), so
        // this copy is bounded.
        row.blockHash.copyBytes(
            to: UnsafeMutableBufferPointer(start: outBlockHash, count: 32),
            count: 32
        )
    }

    // Hand the tx bytes to Rust. The buffer outlives this callback
    // — Rust calls `getCoreTxRecordFreeCallback` to release it.
    let len = row.transactionData.count
    let buffer = UnsafeMutablePointer<UInt8>.allocate(capacity: len)
    row.transactionData.copyBytes(
        to: UnsafeMutableBufferPointer(start: buffer, count: len),
        count: len
    )
    outTxBytes?.pointee = UnsafePointer(buffer)
    outTxBytesLen?.pointee = UInt(len)

    outFound.pointee = true
    return 0
}

/// Paired free callback for `on_get_core_tx_record_free_fn`.
/// Releases the buffer `getCoreTxRecordCallback` allocated above.
/// `UInt8` is trivial so no `deinitialize(count:)` is required —
/// `deallocate()` alone matches the `allocate(capacity:)`.
private func getCoreTxRecordFreeCallback(
    context: UnsafeMutableRawPointer?,
    txBytes: UnsafePointer<UInt8>?,
    _ txBytesLen: UInt
) {
    guard let txBytes = txBytes else { return }
    UnsafeMutablePointer(mutating: txBytes).deallocate()
    _ = context
    _ = txBytesLen
}

/// C shim for `on_list_wallet_core_txids_fn`. Returns a contiguous
/// `count * 32` byte buffer of raw txids in wire order plus a parallel
/// `count`-byte flags buffer (bit `0x01` = the wallet funded the
/// transaction).
private func listWalletCoreTxidsCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    outTxids: UnsafeMutablePointer<UnsafePointer<UInt8>?>?,
    outFlags: UnsafeMutablePointer<UnsafePointer<UInt8>?>?,
    outCount: UnsafeMutablePointer<UInt>?
) -> Int32 {
    // Non-zero on a missing argument: reporting success here would hand Rust
    // an empty enumeration that it cannot tell apart from a wallet with no
    // transactions.
    guard let context = context,
          let walletIdPtr = walletIdPtr,
          let outTxids = outTxids,
          let outFlags = outFlags,
          let outCount = outCount else {
        return -1
    }

    outTxids.pointee = nil
    outFlags.pointee = nil
    outCount.pointee = 0

    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)
    let (txids, errored) = handler.walletCoreTxids(walletId: walletId)
    guard !errored else {
        return -1
    }
    guard !txids.isEmpty else {
        return 0
    }

    let buffer = UnsafeMutablePointer<UInt8>.allocate(capacity: txids.count * 32)
    let flags = UnsafeMutablePointer<UInt8>.allocate(capacity: txids.count)
    // Pack only well-formed txids and report how many were packed. Skipping a
    // malformed one while still reporting `txids.count` would leave its slot
    // uninitialized and hand Rust 32 bytes of garbage as a txid.
    var packed = 0
    for row in txids where row.txid.count == 32 {
        row.txid.copyBytes(to: buffer.advanced(by: packed * 32), count: 32)
        flags.advanced(by: packed).pointee = row.spendsWalletInput ? 0x01 : 0x00
        packed += 1
    }
    guard packed > 0 else {
        buffer.deallocate()
        flags.deallocate()
        return 0
    }
    outTxids.pointee = UnsafePointer(buffer)
    outFlags.pointee = UnsafePointer(flags)
    outCount.pointee = UInt(packed)
    return 0
}

/// Paired free callback for `on_list_wallet_core_txids_free_fn`.
private func listWalletCoreTxidsFreeCallback(
    context: UnsafeMutableRawPointer?,
    txids: UnsafePointer<UInt8>?,
    flags: UnsafePointer<UInt8>?,
    _ count: UInt
) {
    if let txids = txids {
        UnsafeMutablePointer(mutating: txids).deallocate()
    }
    if let flags = flags {
        UnsafeMutablePointer(mutating: flags).deallocate()
    }
    _ = context
}

/// C shim for `on_persist_dashpay_payments_fn`. Copies every
/// `DashpayPaymentPersistEntryFFI` row into a Swift-owned
/// `DashPayPayment` (grouped by owner identity) before invoking the
/// handler, so the Rust side can drop its backing strings the moment
/// we return. Rows without a txid pointer are skipped defensively —
/// the Rust builder documents `txid` as always non-null.
///
/// Always returns 0: a missing owner identity parks the group on
/// `deferredPaymentUpserts` — staged before the round's single save,
/// with a still-unresolvable owner failing the round — and a commit
/// failure is reported through the round's `on_changeset_end_fn`
/// return, so per-batch failure signaling here would be redundant.
private func persistDashpayPaymentsCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    entriesPtr: UnsafePointer<DashpayPaymentPersistEntryFFI>?,
    count: UInt
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)

    var entriesByOwner: [Data: [DashPayPayment]] = [:]
    if count > 0, let entriesPtr = entriesPtr {
        for i in 0..<Int(count) {
            let e = entriesPtr[i]
            guard let txidPtr = e.txid else { continue }
            var ownerRaw = e.owner_identity_id
            let ownerId = Swift.withUnsafeBytes(of: &ownerRaw) { Data($0) }
            var counterpartyRaw = e.counterparty_id
            let counterpartyId = Swift.withUnsafeBytes(of: &counterpartyRaw) { Data($0) }
            // Unknown discriminants fall back to `.sent` / `.pending`
            // rather than dropping the row — same forward-compat
            // posture as `DashPayPayment.init(ffi:)`.
            let payment = DashPayPayment(
                counterpartyId: counterpartyId,
                amountDuffs: e.amount_duffs,
                direction: DashPayPaymentDirection(rawValue: e.direction_raw) ?? .sent,
                status: DashPayPaymentStatus(rawValue: e.status_raw) ?? .pending,
                txid: String(cString: txidPtr),
                memo: e.memo.map { String(cString: $0) }
            )
            entriesByOwner[ownerId, default: []].append(payment)
        }
    }
    guard !entriesByOwner.isEmpty else { return 0 }

    handler.persistDashpayPayments(walletId: walletId, entriesByOwner: entriesByOwner)
    return 0
}
