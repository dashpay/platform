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
                row.operatorPublicKey = mn.operatorPublicKey
                row.platformNodeId = mn.platformNodeId
                row.payoutAddress = mn.payoutAddress
                row.operatorPseudoAddress = mn.operatorPseudoAddress
                row.platformNodeAddress = mn.platformNodeAddress

                // Owner / voting key ownership: join each key's base58
                // address against the persisted `PersistentCoreAddress`
                // rows (address ⇒ account type + index). Durable source
                // that works for imported / restored wallets whose
                // in-memory pools aren't rehydrated — the same join the
                // account screen + address subtitle rely on.
                let owner = ownership(for: mn.ownerAddress, modelContext: modelContext)
                row.ownerInWallet = owner.inWallet
                row.ownerAccountType = owner.accountType
                row.ownerKeyIndex = owner.index
                let voting = ownership(for: mn.votingAddress, modelContext: modelContext)
                row.votingInWallet = voting.inWallet
                row.votingAccountType = voting.accountType
                row.votingKeyIndex = voting.index

                // Operator / platform key ownership comes from Rust's
                // pool-based match (these keys have no on-chain address to
                // join against). Operator keys derive from the account xpub
                // seedlessly, so operator ownership is always computable and
                // updates unconditionally.
                row.operatorInWallet = mn.operatorInWallet
                row.operatorAccountType = mn.operatorAccountType
                row.operatorKeyIndex = mn.operatorKeyIndex
                // Platform-node ownership can be a transient false negative:
                // on a seedless restore the platform pool is empty until the
                // persisted key batch has rehydrated it, and the FFI bool
                // can't distinguish "checked and absent" from "couldn't check
                // yet". So never DOWNGRADE an already-established platform
                // ownership to false on an existing row — mirror the
                // no-prune-on-empty rule. It still upgrades to true and sets
                // fresh true values (and new rows default to false, so they
                // take the fresh value regardless).
                if mn.platformInWallet || !row.platformInWallet {
                    row.platformInWallet = mn.platformInWallet
                    row.platformAccountType = mn.platformAccountType
                    row.platformKeyIndex = mn.platformKeyIndex
                }

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

    /// Resolve a provider key's wallet ownership by looking up its base58
    /// `address` in the persisted `PersistentCoreAddress` rows (address is
    /// `@Attribute(.unique)`, so at most one match). Returns the row's
    /// account-type tag + derivation index. Pure load + string join — no
    /// key material or decisions.
    private static func ownership(
        for address: String?,
        modelContext: ModelContext
    ) -> (inWallet: Bool, accountType: UInt8, index: UInt32) {
        guard let address, !address.isEmpty else { return (false, 0, 0) }
        let descriptor = FetchDescriptor<PersistentCoreAddress>(
            predicate: #Predicate { $0.address == address }
        )
        guard let row = try? modelContext.fetch(descriptor).first,
              let account = row.account
        else {
            return (false, 0, 0)
        }
        return (true, UInt8(truncatingIfNeeded: account.accountType), row.addressIndex)
    }
}
