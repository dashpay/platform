import Foundation
import SwiftData

/// SwiftData row for one decrypted shielded (Orchard) note owned by
/// a specific subwallet.
///
/// Mirrors `platform_wallet::changeset::ShieldedChangeSet::notes_saved`
/// from the Rust side. The persister callback writes one row per
/// `(walletId, accountIndex, position)` and re-saves with the same
/// nullifier overwrite the existing row in place — Orchard
/// nullifiers are globally unique, so repeated discovery of the
/// same note (e.g. after a re-sync) shouldn't double-count.
///
/// On cold start the matching `loadShieldedNotes` callback streams
/// every row back to Rust so `ShieldedWallet::restore_from_snapshot`
/// can rehydrate `SubwalletState.notes` before the first sync runs.
@Model
public final class PersistentShieldedNote {
    /// Index `(walletId, accountIndex)` so per-subwallet balance
    /// scans hit an index instead of the full table.
    #Index<PersistentShieldedNote>([\.walletId, \.accountIndex])

    /// 32-byte wallet identifier (matches `PersistentWallet.walletId`).
    public var walletId: Data
    /// ZIP-32 account index inside the wallet.
    public var accountIndex: UInt32
    /// Global commitment-tree position.
    public var position: UInt64
    /// Note commitment (32 bytes).
    public var cmx: Data
    /// Spending nullifier (32 bytes). Unique across the table —
    /// Orchard nullifiers are globally unique, so making this the
    /// upsert key prevents double-counts on re-sync.
    @Attribute(.unique) public var nullifier: Data
    /// Block height this note was first observed at.
    public var blockHeight: UInt64
    /// Whether the nullifier has been observed as spent on-chain.
    public var isSpent: Bool
    /// Note value in credits.
    public var value: UInt64
    /// Serialized `orchard::Note` bytes (115 bytes:
    /// `recipient(43) || value(8 LE) || rho(32) || rseed(32)`).
    public var noteData: Data

    /// Insertion timestamps.
    public var createdAt: Date
    public var lastUpdated: Date

    public init(
        walletId: Data,
        accountIndex: UInt32,
        position: UInt64,
        cmx: Data,
        nullifier: Data,
        blockHeight: UInt64,
        isSpent: Bool,
        value: UInt64,
        noteData: Data
    ) {
        self.walletId = walletId
        self.accountIndex = accountIndex
        self.position = position
        self.cmx = cmx
        self.nullifier = nullifier
        self.blockHeight = blockHeight
        self.isSpent = isSpent
        self.value = value
        self.noteData = noteData
        let now = Date()
        self.createdAt = now
        self.lastUpdated = now
    }
}

public extension PersistentShieldedNote {
    /// Predicate for one wallet's unspent notes in every account,
    /// identity accounts included. Their `value` sum is therefore not
    /// the wallet's shielded balance: pass the fetched rows to
    /// [`ordinaryBalance(_:)`] for that. Single home for the filter so
    /// every balance surface agrees on what "unspent" means.
    static func unspentPredicate(walletId: Data) -> Predicate<PersistentShieldedNote> {
        #Predicate<PersistentShieldedNote> {
            $0.walletId == walletId && $0.isSpent == false
        }
    }

    /// Predicate for ALL wallets' unspent notes, in every account;
    /// callers scope by walletId in memory (multi-wallet surfaces like
    /// the identity funding picker).
    static var unspentPredicate: Predicate<PersistentShieldedNote> {
        #Predicate<PersistentShieldedNote> { $0.isSpent == false }
    }

    /// The ordinary shielded balance of `notes`: the sum of the values
    /// of the notes whose account is not an identity account, by the
    /// wallet's rule (`PlatformWalletManager.isShieldedIdentityAccount`).
    /// This is the number the sync result reports as a wallet's
    /// `balance`. Pass one wallet's unspent notes, for example those
    /// [`unspentPredicate(walletId:)`] fetches. A SwiftData predicate
    /// cannot call the native rule, so the filter runs here, after the
    /// fetch. A sum past `UInt64.max` stays at `UInt64.max` rather than
    /// trapping.
    static func ordinaryBalance(_ notes: some Sequence<PersistentShieldedNote>) -> UInt64 {
        notes.reduce(UInt64(0)) { total, note in
            guard !PlatformWalletManager.isShieldedIdentityAccount(note.accountIndex) else {
                return total
            }
            let (sum, overflow) = total.addingReportingOverflow(note.value)
            return overflow ? UInt64.max : sum
        }
    }
}
