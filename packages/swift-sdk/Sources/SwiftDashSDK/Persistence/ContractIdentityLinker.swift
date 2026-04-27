import Foundation
import SwiftData

/// Back-fills the `PersistentDataContract.ownerIdentity` ↔
/// `PersistentIdentity.ownedDataContracts` link when one side of the
/// pair shows up in the local store.
///
/// The link is purely a convenience for the UI — the canonical owner
/// id lives on `PersistentDataContract.ownerId` (the raw 32 bytes
/// from the on-chain contract) and stays valid even when the owner
/// identity isn't held locally. These helpers are idempotent and
/// don't save the model context themselves; the caller saves once at
/// the end of whatever bracket they're in.
///
/// Both helpers are intentionally **not** `@MainActor`. They mutate
/// only the SwiftData rows on the `ModelContext` they're handed, so
/// they run safely on whichever actor owns that context — the main
/// actor for view-driven inserts, the persister handler's background
/// context for Rust-side changesets.
public enum ContractIdentityLinker {
    /// Called right after a `PersistentDataContract` row is inserted
    /// or its `ownerId` is updated. If the contract names an owner
    /// id and a matching `PersistentIdentity` already lives in the
    /// store, sets `contract.ownerIdentity`. No-op when:
    ///  - `contract.ownerId` is nil (system contracts, contracts
    ///    persisted before owner parsing),
    ///  - the link is already established and points at the right
    ///    identity (skipping the assignment avoids bumping
    ///    `lastUpdated` for nothing),
    ///  - no matching identity exists locally (the common case for
    ///    contracts authored by other users).
    ///
    /// The caller is responsible for saving the `ModelContext` after
    /// this returns.
    public static func linkContractToOwner(
        contract: PersistentDataContract,
        modelContext: ModelContext
    ) {
        guard let ownerId = contract.ownerId else { return }
        // Idempotency guard. Prefer comparing against the canonical
        // raw bytes rather than identity object identity — the same
        // identity row can be re-fetched as a different in-memory
        // instance after a context reset.
        if contract.ownerIdentity?.identityId == ownerId {
            return
        }

        let descriptor = FetchDescriptor<PersistentIdentity>(
            predicate: #Predicate<PersistentIdentity> { identity in
                identity.identityId == ownerId
            }
        )
        guard let owner = try? modelContext.fetch(descriptor).first else {
            return
        }

        contract.ownerIdentity = owner
    }

    /// Called right after a `PersistentIdentity` row is inserted (or
    /// re-inserted via the BLAST persister). Walks every
    /// `PersistentDataContract` whose `ownerId` matches this
    /// identity's id and back-fills `ownerIdentity` on each one.
    ///
    /// Cheap in practice: the `FetchDescriptor` filters on the
    /// `ownerId` column and returns only the contracts that
    /// actually need patching. (The column isn't explicitly
    /// `@Attribute(.indexed)` and SwiftData doesn't auto-index
    /// optional `Data`, so this is a linear scan over the saved
    /// contracts on this device — the row count is tiny in the
    /// example app, and adding the index attribute would be a
    /// schema migration.) Idempotent — the per-contract `==`
    /// check below skips rows already linked to this identity.
    ///
    /// The caller is responsible for saving the `ModelContext`.
    public static func linkIdentityToOwnedContracts(
        identity: PersistentIdentity,
        modelContext: ModelContext
    ) {
        let identityId = identity.identityId
        let descriptor = FetchDescriptor<PersistentDataContract>(
            predicate: #Predicate<PersistentDataContract> { contract in
                contract.ownerId == identityId
            }
        )
        guard let contracts = try? modelContext.fetch(descriptor) else {
            return
        }
        for contract in contracts {
            // Skip writes for rows that already point at this exact
            // identity row to keep the pass idempotent.
            if contract.ownerIdentity?.identityId == identityId {
                continue
            }
            contract.ownerIdentity = identity
        }
    }
}
