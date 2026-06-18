import Foundation
import SwiftData

// MARK: - On-demand token balance refresh (fetch + persist)

extension SDK {
    /// Fetch fresh on-chain token balances for `token` across
    /// `identityIds` and upsert them into the local
    /// `PersistentTokenBalance` rows the UI observes via `@Query`.
    ///
    /// This is the synchronous backstop the periodic balance sync
    /// can't provide quickly: after a token state transition
    /// (transfer / mint / burn / freeze / …) the affected balances
    /// change on chain immediately, but the local SwiftData rows stay
    /// stale until the next sync round. Call this right after a token
    /// action succeeds so the balance surfaces refresh without waiting.
    ///
    /// Architecture (see `packages/swift-sdk/CLAUDE.md`): this is a
    /// fetch-then-persist bridge — it calls an existing FFI query
    /// (`getIdentityTokenBalances`) and writes the result into
    /// SwiftData. It makes no protocol decisions; the example app
    /// decides *which* identities to refresh (sender, and recipient
    /// when local).
    ///
    /// Row shape matches the periodic-sync persister
    /// (`PlatformWalletPersistenceHandler.persistTokenBalances`):
    /// rows are keyed by `(canonicalTokenId, identityId)` and stitched
    /// into the relationship graph (`identity` + `token`) so the
    /// view-side matchers that look up balances via
    /// `identity.tokenBalances.first { $0.token?.id == token.id }`
    /// resolve them.
    ///
    /// The canonical on-chain token id is derived from the token's
    /// `contractId` + `position` via `calculateTokenId` — `token.id`
    /// is a SwiftData uniqueness key (`contractId || u16_be(position)`),
    /// *not* the canonical id the balance query is keyed by.
    ///
    /// Callers pass plain values (not the `PersistentToken` model) so
    /// no SwiftData model is read across the network `await` /
    /// off-main: the caller derives these on the main actor.
    ///
    /// - Parameters:
    ///   - contractId: The 32-byte data-contract id that owns the token.
    ///   - tokenPosition: The token's position within the contract.
    ///   - tokenRelationshipKey: The local `PersistentToken.id`
    ///     uniqueness key (`contractId || u16_be(position)`), used only
    ///     to relink the `token` relationship on upserted rows.
    ///   - identityIds: 32-byte identity ids to refresh. Identities
    ///     not present locally still have their `(tokenId, identityId)`
    ///     row upserted, but the `identity` relationship is only linked
    ///     when a matching `PersistentIdentity` row exists.
    ///   - context: The SwiftData context backing the views' `@Query`s
    ///     (the app's main `ModelContext`).
    ///
    /// `@MainActor`-isolated because it writes the SwiftData
    /// `ModelContext`. The blocking network query is hopped off-main
    /// via `Task.detached` so the `await` suspends the main actor
    /// rather than freezing the UI; only the upsert/save runs on main.
    @MainActor
    public func refreshTokenBalances(
        contractId: Data,
        tokenPosition: UInt16,
        tokenRelationshipKey: Data,
        identityIds: [Data],
        in context: ModelContext
    ) async throws {
        guard !identityIds.isEmpty else { return }

        let contractIdString = contractId.toBase58String()
        let net = self.network

        let canonicalTokenId = try calculateTokenId(
            contractId: contractIdString,
            position: tokenPosition
        )

        // One fetch per identity. We deliberately use the
        // single-identity query (`getIdentityTokenBalances`) rather
        // than the multi-identity one: it parses `NSNumber` balances
        // correctly and matches the precedent in
        // `TokenActionPermissionsView`. At most two identities are
        // refreshed in practice (sender + local recipient).
        //
        // `getIdentityTokenBalances` blocks on a Rust runtime, so run
        // it off the main actor. `SDK` is `@unchecked Sendable`; only
        // `Sendable` values (the SDK, strings) cross into the task.
        let sdk = self
        let freshBalances: [Data: UInt64] = try await Task.detached(priority: .userInitiated) {
            var result: [Data: UInt64] = [:]
            for identityId in identityIds {
                let identityBase58 = identityId.toBase58String()
                let balances = try await sdk.getIdentityTokenBalances(
                    identityId: identityBase58,
                    tokenIds: [canonicalTokenId]
                )
                // The query omits tokens the identity has never held;
                // default those to 0 so a sender that drained to empty
                // still gets its row zeroed.
                result[identityId] = balances[canonicalTokenId] ?? 0
            }
            return result
        }.value

        try Self.persistTokenBalances(
            canonicalTokenId: canonicalTokenId,
            tokenRelationshipKey: tokenRelationshipKey,
            network: net,
            freshBalances: freshBalances,
            in: context
        )
    }

    /// Upsert + relationship-link a batch of `(identityId → balance)`
    /// rows for one token. Mirrors
    /// `PlatformWalletPersistenceHandler.persistTokenBalances` /
    /// `linkTokenBalanceRelations` so the rows land identically to the
    /// periodic-sync path.
    @MainActor
    private static func persistTokenBalances(
        canonicalTokenId: String,
        tokenRelationshipKey: Data,
        network: Network,
        freshBalances: [Data: UInt64],
        in context: ModelContext
    ) throws {
        let tokenDescriptor = FetchDescriptor<PersistentToken>(
            predicate: #Predicate { $0.id == tokenRelationshipKey }
        )
        let tokenRow = try? context.fetch(tokenDescriptor).first

        for (identityId, balance) in freshBalances {
            let descriptor = FetchDescriptor<PersistentTokenBalance>(
                predicate: #Predicate {
                    $0.tokenId == canonicalTokenId && $0.identityId == identityId
                }
            )

            let row: PersistentTokenBalance
            if let existing = try? context.fetch(descriptor).first {
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

            row.updateBalance(Int64(bitPattern: balance))
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
                if let parent = try? context.fetch(identityDescriptor).first {
                    row.identity = parent
                }
            }
        }

        try context.save()
    }
}
