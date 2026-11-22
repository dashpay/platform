import Foundation
import SwiftData
import SwiftDashSDK

/// Shared persist/load helper for re-fetching an identity's balance and
/// public-key set from Platform and writing the result into SwiftData.
///
/// Factored out of `IdentityDetailView.refreshIdentityData()` so other
/// views that mutate an identity's keys (e.g. `KeyDetailView`'s
/// **Disable Key** action) can refresh the persisted rows (and the
/// `disabledAt` badge those rows drive) without a manual pull-to-
/// refresh and without duplicating the parse/persist logic.
///
/// Per `packages/swift-sdk/CLAUDE.md` this is strictly persist + load:
/// it calls the already-existing `SDK.identityGet` FFI query, marshals
/// the keys with `PersistentPublicKey.fromFetchedIdentity(_:identityId:)`
/// and merges them with `PersistentIdentity.mergeFetchedPublicKeys(_:)`.
/// No derivation, no policy decisions, no orchestration of multi-step
/// Rust pipelines.
enum IdentityKeyRefresher {
    /// Re-fetch `identity`'s balance + public keys from Platform and
    /// persist them.
    ///
    /// The keys go through `PersistentIdentity.mergeFetchedPublicKeys(_:)`:
    /// a key whose id, type and bytes are unchanged keeps its row, and with
    /// it the `privateKeyKeychainIdentifier`, `walletId` and
    /// `identityDerivationPath` that let it sign, and its stored contract
    /// bounds when the fetch does not carry them. A new or changed key gets
    /// a new row; a key no longer on Platform has its row deleted (its
    /// Keychain item is left alone).
    ///
    /// Mutates SwiftData on the `modelContext`'s actor; call from the
    /// main actor (the SwiftData container the views use is main-actor
    /// bound).
    @MainActor
    static func refreshBalanceAndKeys(
        identity: PersistentIdentity,
        sdk: SDK,
        modelContext: ModelContext
    ) async throws {
        let fetchedIdentity = try await sdk.identityGet(
            identityId: identity.identityIdBase58
        )

        // Balance: accept either the numeric or string-encoded form
        // the FFI may hand back depending on magnitude.
        if let balanceValue = fetchedIdentity["balance"] {
            if let balanceNum = balanceValue as? NSNumber {
                PersistentIdentity.updateBalance(
                    in: modelContext,
                    identityId: identity.identityId,
                    balance: balanceNum.uint64Value
                )
                try? modelContext.save()
            } else if let balanceString = balanceValue as? String,
                      let balanceUInt = UInt64(balanceString) {
                PersistentIdentity.updateBalance(
                    in: modelContext,
                    identityId: identity.identityId,
                    balance: balanceUInt
                )
                try? modelContext.save()
            }
        }

        // Public keys: marshal the fetched set and merge it into the
        // existing rows, so an unchanged key keeps its Keychain identifier,
        // wallet id, derivation path and (when the fetch does not say)
        // contract bounds.
        let fetchedKeys = PersistentPublicKey.fromFetchedIdentity(
            fetchedIdentity,
            identityId: identity.identityId
        )
        identity.mergeFetchedPublicKeys(fetchedKeys)
        try? modelContext.save()
    }
}
