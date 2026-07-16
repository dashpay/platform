import Foundation
import SwiftData

// MARK: - Persist proof-verified token balances

extension SDK {
    /// Upsert a batch of proof-verified `(identityId → balance)` rows
    /// for one token into the local `PersistentTokenBalance` rows the UI
    /// observes via `@Query`.
    ///
    /// This is the persist half of the post-action balance refresh: a
    /// token state transition (transfer / burn) already returns the
    /// affected identities' proof-verified post-action balances in its
    /// broadcast result (`ManagedPlatformWallet.tokenTransfer` /
    /// `tokenBurn` surface them as `[Data: UInt64]`). The caller passes
    /// that map straight here — no follow-up balance query is needed,
    /// so the refresh is single-round-trip, race-free, and includes the
    /// recipient for free.
    ///
    /// Architecture (see `packages/swift-sdk/CLAUDE.md`): this is a
    /// pure persist bridge — it writes SwiftData rows from values the
    /// FFI already returned. It makes no protocol decisions; the example
    /// app decides nothing beyond which `(contract, position)` token the
    /// balances belong to.
    ///
    /// Row shape matches the periodic-sync persister
    /// (`PlatformWalletPersistenceHandler.persistTokenBalances`): rows
    /// are keyed by `(canonicalTokenId, identityId)` and stitched into
    /// the relationship graph (`identity` + `token`) so the view-side
    /// matchers that look up balances via
    /// `identity.tokenBalances.first { $0.token?.id == token.id }`
    /// resolve them.
    ///
    /// The canonical on-chain token id used for the
    /// `PersistentTokenBalance.tokenId` field is derived from the
    /// token's `contractId` + `position` via `calculateTokenId`.
    /// `tokenRelationshipKey` is the *separate* local
    /// `PersistentToken.id` uniqueness key (a `contractId`-plus-position
    /// composite, treated opaquely here) used only to relink the
    /// `token` relationship — it is **not** the canonical id the balance
    /// rows are keyed by.
    ///
    /// - Parameters:
    ///   - contractId: The 32-byte data-contract id that owns the token.
    ///   - tokenPosition: The token's position within the contract.
    ///   - tokenRelationshipKey: The local `PersistentToken.id`
    ///     uniqueness key (a `contractId`-plus-position composite,
    ///     treated opaquely), used only to relink the `token`
    ///     relationship on upserted rows.
    ///   - balances: Proof-verified `identityId (raw 32 bytes) → balance`
    ///     map as returned by the transfer / burn FFI. An empty map is a
    ///     no-op (e.g. a group-action proposal burned nothing yet).
    ///     Identities not present locally still have their
    ///     `(tokenId, identityId)` row upserted, but the `identity`
    ///     relationship is only linked when a matching
    ///     `PersistentIdentity` row exists.
    ///   - context: The SwiftData context backing the views' `@Query`s
    ///     (the app's main `ModelContext`).
    ///
    /// `@MainActor`-isolated because it writes the SwiftData
    /// `ModelContext`. No network round-trip happens here — the balances
    /// were already verified during the broadcast — so this is a cheap
    /// synchronous upsert + save.
    @MainActor
    public func persistProvenTokenBalances(
        contractId: Data,
        tokenPosition: UInt16,
        tokenRelationshipKey: Data,
        balances: [Data: UInt64],
        in context: ModelContext
    ) throws {
        guard !balances.isEmpty else { return }

        let canonicalTokenId = try calculateTokenId(
            contractId: contractId.toBase58String(),
            position: tokenPosition
        )

        try Self.persistTokenBalances(
            canonicalTokenId: canonicalTokenId,
            tokenRelationshipKey: tokenRelationshipKey,
            network: self.network,
            balances: balances,
            in: context
        )
    }

    /// Upsert + relationship-link a batch of `(identityId → balance)`
    /// rows for one token. Mirrors
    /// `PlatformWalletPersistenceHandler.persistTokenBalances` /
    /// `linkTokenBalanceRelations` so the rows land identically to the
    /// periodic-sync path.
    @MainActor
    static func persistTokenBalances(
        canonicalTokenId: String,
        tokenRelationshipKey: Data,
        network: Network,
        balances: [Data: UInt64],
        in context: ModelContext
    ) throws {
        let tokenDescriptor = FetchDescriptor<PersistentToken>(
            predicate: #Predicate { $0.id == tokenRelationshipKey }
        )
        // `try ... .first` (not `try?`): a genuine SwiftData fetch
        // failure must propagate, not masquerade as "token row absent".
        let tokenRow = try context.fetch(tokenDescriptor).first

        for (identityId, balance) in balances {
            let descriptor = FetchDescriptor<PersistentTokenBalance>(
                predicate: #Predicate {
                    $0.tokenId == canonicalTokenId && $0.identityId == identityId
                }
            )

            // `try ... .first` (not `try?`): `PersistentTokenBalance`
            // has no unique constraint on `(tokenId, identityId)`, so a
            // swallowed fetch error would fall through to the insert
            // branch and create a *duplicate* row. Let the error
            // propagate to the view's best-effort catch instead.
            let row: PersistentTokenBalance
            if let existing = try context.fetch(descriptor).first {
                row = existing
            } else {
                row = PersistentTokenBalance(
                    tokenId: canonicalTokenId,
                    identityId: identityId,
                    balance: 0,
                    network: network
                )
                context.insert(row)
            }

            row.updateBalance(balance)
            row.markAsSynced()

            // Re-link on every upsert (cheap) so a row inserted before
            // its identity / token row landed gets stitched in here too.
            if row.token == nil, let tokenRow = tokenRow {
                row.token = tokenRow
            }
            if row.identity == nil {
                let identityDescriptor = FetchDescriptor<PersistentIdentity>(
                    predicate: #Predicate { $0.identityId == identityId }
                )
                // `try ... .first` (not `try?`): propagate real fetch
                // errors; `.first` still yields nil for a legit miss
                // (identity not present locally).
                if let parent = try context.fetch(identityDescriptor).first {
                    row.identity = parent
                }
            }
        }

        try context.save()
    }
}
