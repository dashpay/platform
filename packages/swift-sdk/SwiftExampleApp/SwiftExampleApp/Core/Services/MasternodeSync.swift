import Foundation
import SwiftData
import SwiftDashSDK

/// Refreshes persisted `PersistentMasternode` rows from the Rust
/// masternode aggregation (`PlatformWalletManager.masternodes(for:)`).
///
/// This is pure persist/marshalling per `packages/swift-sdk/CLAUDE.md` —
/// all grouping/decoding happened in Rust; here we only upsert the flat
/// result by `(walletId, proTxHash)` and delete rows the aggregation no
/// longer returns. Triggered on the Identities tab appearing (and safe to
/// call again after a Core sync pass).
@MainActor
enum MasternodeSync {
    static func refresh(
        walletManager: PlatformWalletManager,
        walletIds: Set<Data>,
        modelContext: ModelContext
    ) {
        var changed = false
        for walletId in walletIds {
            let fresh = walletManager.masternodes(for: walletId)

            let wid = walletId
            let existing = (try? modelContext.fetch(
                FetchDescriptor<PersistentMasternode>(
                    predicate: #Predicate { $0.walletId == wid }
                )
            )) ?? []
            var byProTx: [Data: PersistentMasternode] = [:]
            for row in existing { byProTx[row.proTxHash] = row }

            // An EMPTY aggregation is indistinguishable from "wallet not
            // rehydrated yet / SPV not started" — the provider records the
            // aggregation reads live in memory and repopulate lazily after
            // launch (via restore staging / rescan). Treating empty as
            // ground truth would delete every persisted masternode on every
            // cold start, so we only ever prune against a NON-empty result.
            let mayPrune = !fresh.isEmpty

            var seen = Set<Data>()
            for mn in fresh {
                seen.insert(mn.proTxHash)
                let row: PersistentMasternode
                if let found = byProTx[mn.proTxHash] {
                    row = found
                } else {
                    row = PersistentMasternode(
                        walletId: walletId,
                        proTxHash: mn.proTxHash,
                        registrationTxid: mn.proTxHash
                    )
                    modelContext.insert(row)
                    changed = true
                }
                row.registrationTxid = mn.proTxHash
                row.serviceAddress = mn.serviceAddress
                row.isEvonode = mn.isEvonode
                row.ownerKeyHash = mn.ownerKeyHash
                row.votingKeyHash = mn.votingKeyHash
                row.ownerAddress = mn.ownerAddress
                row.votingAddress = mn.votingAddress
                row.collateralTxid = mn.collateralTxid
                row.collateralVout = mn.collateralVout
                row.revoked = mn.revoked
                row.revocationReason = mn.revocationReason
                // Status: skip on Unknown (3) so a not-yet-synced DML
                // doesn't clobber a previously resolved Active/Inactive/
                // Retired. Rust returns Unknown only when the list is
                // unavailable.
                if mn.status != 3 {
                    row.statusRaw = mn.status
                }
                row.registrationHeight = mn.registrationHeight
                row.hasRegistration = mn.hasRegistration
                row.txCount = mn.txCount
                row.orderIndex = mn.orderIndex
                row.typeIndex = mn.typeIndex
                row.lastUpdated = Date()
                changed = true
            }

            // Drop rows the aggregation no longer returns — but ONLY when
            // the aggregation was non-empty (see `mayPrune`). An empty
            // result during early startup must not wipe persisted rows.
            if mayPrune {
                for row in existing where !seen.contains(row.proTxHash) {
                    modelContext.delete(row)
                    changed = true
                }
            }
        }
        if changed {
            try? modelContext.save()
        }
    }
}
