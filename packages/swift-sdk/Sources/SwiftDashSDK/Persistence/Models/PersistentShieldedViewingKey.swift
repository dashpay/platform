import Foundation
import SwiftData

/// SwiftData row for a per-subwallet Orchard viewing key.
///
/// Mirrors `platform_wallet::changeset::ShieldedChangeSet::viewing_keys`
/// from the Rust side. `fvkBytes` is the raw 96-byte Orchard
/// `FullViewingKey` encoding; IVK / OVK / the default payment address
/// are all pure functions of it, so this row alone lets Rust rebind
/// the shielded sub-wallet at launch without reading the mnemonic
/// from the Keychain. Viewing-grade only — the bytes can decrypt and
/// recognize notes but cannot authorize a spend, so they live in
/// SwiftData alongside the other derived-key batches (the same
/// placement as `PersistentAccount.derivedPlatformNodeKeys`), not in
/// the Keychain.
///
/// Written via the `on_persist_shielded_viewing_keys_fn` FFI callback
/// (fired once per seed-backed bind); streamed back to Rust on cold
/// start via `on_load_shielded_viewing_keys_fn`.
@Model
public final class PersistentShieldedViewingKey {
    /// Composite uniqueness on `(walletId, accountIndex)` — at most
    /// one viewing key per subwallet. The FVK for a subwallet never
    /// legitimately changes on a network, so re-persists are
    /// byte-identical upserts.
    #Unique<PersistentShieldedViewingKey>([\.walletId, \.accountIndex])
    #Index<PersistentShieldedViewingKey>([\.walletId])

    public var walletId: Data
    public var accountIndex: UInt32
    /// Raw 96-byte Orchard `FullViewingKey` encoding (`ak ‖ nk ‖ rivk`).
    public var fvkBytes: Data

    public var lastUpdated: Date

    public init(
        walletId: Data,
        accountIndex: UInt32,
        fvkBytes: Data
    ) {
        self.walletId = walletId
        self.accountIndex = accountIndex
        self.fvkBytes = fvkBytes
        self.lastUpdated = Date()
    }
}
