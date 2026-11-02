import Foundation
import SwiftData
import SwiftDashSDK

/// Shared persist/load helper for re-fetching an identity's balance and
/// public-key set from Platform and writing the result into SwiftData.
///
/// Factored out of `IdentityDetailView.refreshIdentityData()` so other
/// views that mutate an identity's keys (e.g. `KeyDetailView`'s
/// **Disable Key** action) can refresh the persisted rows — and the
/// `disabledAt` badge those rows drive — without a manual pull-to-
/// refresh and without duplicating the parse/persist logic.
///
/// Per `packages/swift-sdk/CLAUDE.md` this is strictly persist + load:
/// it calls the already-existing `SDK.identityGet` FFI query, marshals
/// the response into `IdentityPublicKey` values, and replaces the
/// `PersistentPublicKey` rows. No derivation, no policy decisions, no
/// orchestration of multi-step Rust pipelines.
enum IdentityKeyRefresher {
    /// Re-fetch `identity`'s balance + public keys from Platform and
    /// persist them, replacing the existing `PersistentPublicKey` rows.
    ///
    /// Carries over any `privateKeyKeychainIdentifier` we already knew
    /// for a given keyId so a refresh doesn't orphan the matching
    /// Keychain reference.
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

        // Balance — accept either the numeric or string-encoded form
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

        // Public keys — parse the freshly-fetched set.
        var parsedPublicKeys: [IdentityPublicKey] = []
        if let publicKeysArray = fetchedIdentity["publicKeys"] as? [[String: Any]] {
            parsedPublicKeys = publicKeysArray.compactMap { keyData -> IdentityPublicKey? in
                guard let id = keyData["id"] as? Int,
                      let purpose = keyData["purpose"] as? Int,
                      let securityLevel = keyData["securityLevel"] as? Int,
                      let keyType = keyData["type"] as? Int,
                      let dataStr = keyData["data"] as? String,
                      let data = Data(base64Encoded: dataStr) else {
                    return nil
                }

                let readOnly = keyData["readOnly"] as? Bool ?? false
                // `disabledAt`, `totalBudget` and `expiresAt` are all protocol
                // `u64`s, which DPP writes as a JSON number up to 2^53 - 1 and
                // as a decimal string above it. `UInt64(jsonValue:)` reads both
                // shapes; a plain `as? UInt64` would drop the large ones.
                let disabledAt = UInt64(jsonValue: keyData["disabledAt"])
                // Usage limits (protocol version 14). Present only on a
                // version 1 key: `totalBudget` is credits for the key's
                // whole lifetime, `expiresAt` is block time in
                // milliseconds. Dropping them here would persist a
                // limited key as unlimited.
                let totalBudget = UInt64(jsonValue: keyData["totalBudget"])
                let expiresAt = UInt64(jsonValue: keyData["expiresAt"])

                return IdentityPublicKey(
                    id: UInt32(id),
                    purpose: KeyPurpose(rawValue: UInt8(purpose)) ?? .authentication,
                    securityLevel: SecurityLevel(rawValue: UInt8(securityLevel)) ?? .high,
                    contractBounds: nil,
                    keyType: KeyType(rawValue: UInt8(keyType)) ?? .ecdsaSecp256k1,
                    readOnly: readOnly,
                    data: data,
                    disabledAt: disabledAt,
                    totalBudget: totalBudget,
                    expiresAt: expiresAt
                )
            }
        }

        // Replace the PersistentIdentity's public key rows with the
        // freshly-fetched set, carrying over the keychain identifier
        // for any key we already knew about so we don't lose the
        // matching private key reference after a refresh.
        let identifierByKeyId: [Int32: String] = Dictionary(
            uniqueKeysWithValues: identity.publicKeys.compactMap { key in
                guard let identifier = key.privateKeyKeychainIdentifier else { return nil }
                return (key.keyId, identifier)
            }
        )
        identity.publicKeys.removeAll()
        let identityHex = identity.identityIdBase58
        for publicKey in parsedPublicKeys {
            guard let persistentKey = PersistentPublicKey.from(publicKey, identityId: identityHex) else {
                continue
            }
            if let identifier = identifierByKeyId[persistentKey.keyId] {
                persistentKey.privateKeyKeychainIdentifier = identifier
            }
            identity.addPublicKey(persistentKey)
        }
        try? modelContext.save()
    }
}
