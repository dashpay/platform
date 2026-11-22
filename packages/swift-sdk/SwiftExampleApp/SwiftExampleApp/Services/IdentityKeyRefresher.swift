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
    /// When `wallet` holds the identity, the refresh is the wallet's own
    /// proved one (`refreshIdentity`): Rust folds the fetch into its copy
    /// and upserts the changed key rows through the persister, keeping
    /// each row's wallet id, derivation path and Keychain reference, and
    /// never letting a lagging node roll back a raise or a disable. Swift
    /// does not write those rows itself then, so the wallet stays their
    /// only writer.
    ///
    /// Without a wallet (an identity only looked up, not held), the
    /// identity is fetched and its keys go through
    /// `PersistentIdentity.mergeFetchedPublicKeys(_:)`: a key whose id,
    /// type and bytes are unchanged keeps its row, and with it the
    /// `privateKeyKeychainIdentifier`, `walletId` and
    /// `identityDerivationPath` that let it sign. A new key gets a new row.
    /// A stored key the response lacks keeps its row, since Platform never
    /// removes a key and such a response predates it. Keychain items are
    /// left alone.
    ///
    /// The keys are read before anything is written: when the response's
    /// key list cannot be read completely, this throws and changes nothing,
    /// the balance included.
    ///
    /// Mutates SwiftData on the `modelContext`'s actor; call from the
    /// main actor (the SwiftData container the views use is main-actor
    /// bound).
    @MainActor
    static func refreshBalanceAndKeys(
        identity: PersistentIdentity,
        wallet: ManagedPlatformWallet?,
        sdk: SDK,
        modelContext: ModelContext
    ) async throws {
        if let wallet {
            try await wallet.refreshIdentity(identityId: identity.identityId)
            return
        }
        // Held by a wallet that is not loaded right now: its rows carry the
        // wallet id and derivation path the signer needs, which a rebuild
        // from a plain fetch would drop. Leave them to the wallet.
        if identity.wallet != nil {
            throw SDKError.invalidState("The wallet holding this identity is not loaded; open the wallet and refresh again.")
        }

        let fetchedIdentity = try await sdk.identityGet(
            identityId: identity.identityIdBase58
        )
        // Wallet discovery may have claimed the identity, and written its
        // rows with their signing metadata, while the fetch was out. This
        // check narrows that window but does not close it: the wallet's
        // persister commits from its own context off the main actor, so a
        // claim can still land before the save below. Tracked in #6074.
        if identity.wallet != nil {
            throw SDKError.invalidState("A wallet took over this identity during the refresh; refresh it again from the wallet.")
        }

        // Public keys: read the whole fetched set before writing anything.
        // A key read only in part would be written over its stored row, so
        // a list that cannot be read completely is not applied at all.
        guard let fetchedKeys = PersistentPublicKey.fromFetchedIdentity(
            fetchedIdentity,
            identityId: identity.identityId
        ) else {
            throw SDKError.serializationError("This identity's keys could not be read from the response, so nothing was changed. Refresh again.")
        }

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

        // Merge the fetched keys into the existing rows, so an unchanged key
        // keeps its Keychain identifier, wallet id and derivation path.
        identity.mergeFetchedPublicKeys(fetchedKeys)
        try? modelContext.save()
    }
}
