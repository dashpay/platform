import XCTest
@testable import SwiftExampleApp

/// Tests the pure anti-join that powers the "Fund from unused Asset
/// Lock" resume picker (iter 5). The filter has three pieces of
/// business logic that can silently regress:
///
///   1. `walletId` match — never surface a lock from a different
///      wallet (would otherwise sign with the wrong key path).
///   2. `statusRaw >= 2` floor — only InstantSendLocked (2) or
///      ChainLocked (3) locks are resumable; Built (0) and
///      Broadcast (1) aren't final and the Platform side rejects
///      them.
///   3. Anti-join on `(walletId, identityIndexRaw)` against the
///      set of in-use identity slots — never offer a lock whose
///      slot already has a `PersistentIdentity` row, otherwise the
///      resume submit would collide with an already-registered
///      identity.
///
/// We exercise these with a lightweight `FakeAssetLockRow` so the
/// test stays a pure function call — no SwiftData container needed.
final class CreateIdentityResumableTests: XCTestCase {

    private struct FakeAssetLockRow: AssetLockResumeRow, Equatable {
        let walletId: Data
        let statusRaw: Int
        let identityIndexRaw: Int32
    }

    private let walletA = Data(repeating: 0xA1, count: 8)
    private let walletB = Data(repeating: 0xB2, count: 8)

    // MARK: - walletId match

    func testFiltersOutLocksFromOtherWallets() {
        let locks: [FakeAssetLockRow] = [
            FakeAssetLockRow(walletId: walletA, statusRaw: 2, identityIndexRaw: 0),
            FakeAssetLockRow(walletId: walletB, statusRaw: 2, identityIndexRaw: 0),
        ]
        let result = CreateIdentityView.resumableLocks(
            in: locks,
            usedIndices: [],
            walletId: walletA
        )
        XCTAssertEqual(result, [locks[0]])
    }

    // MARK: - statusRaw floor

    func testRejectsBuiltAndBroadcastStatuses() {
        let locks: [FakeAssetLockRow] = [
            FakeAssetLockRow(walletId: walletA, statusRaw: 0, identityIndexRaw: 0), // Built
            FakeAssetLockRow(walletId: walletA, statusRaw: 1, identityIndexRaw: 1), // Broadcast
        ]
        let result = CreateIdentityView.resumableLocks(
            in: locks,
            usedIndices: [],
            walletId: walletA
        )
        XCTAssertTrue(result.isEmpty)
    }

    func testAcceptsInstantSendLockedAndChainLocked() {
        let locks: [FakeAssetLockRow] = [
            FakeAssetLockRow(walletId: walletA, statusRaw: 2, identityIndexRaw: 0), // ISLock
            FakeAssetLockRow(walletId: walletA, statusRaw: 3, identityIndexRaw: 1), // CLock
        ]
        let result = CreateIdentityView.resumableLocks(
            in: locks,
            usedIndices: [],
            walletId: walletA
        )
        XCTAssertEqual(result.count, 2)
    }

    /// Defensive: any unknown future status (e.g. 4) should still
    /// pass the `>= 2` floor. If we ever flip from "≥2 means final"
    /// to a closed set, this test will need updating — that's
    /// intentional, the surprise is signal.
    func testForwardCompatibleStatusFloor() {
        let locks = [
            FakeAssetLockRow(walletId: walletA, statusRaw: 4, identityIndexRaw: 0)
        ]
        let result = CreateIdentityView.resumableLocks(
            in: locks,
            usedIndices: [],
            walletId: walletA
        )
        XCTAssertEqual(result.count, 1)
    }

    // MARK: - anti-join on used slots

    func testFiltersOutLocksWhoseSlotIsAlreadyUsed() {
        let locks: [FakeAssetLockRow] = [
            FakeAssetLockRow(walletId: walletA, statusRaw: 2, identityIndexRaw: 0),
            FakeAssetLockRow(walletId: walletA, statusRaw: 3, identityIndexRaw: 1),
            FakeAssetLockRow(walletId: walletA, statusRaw: 2, identityIndexRaw: 2),
        ]
        let result = CreateIdentityView.resumableLocks(
            in: locks,
            usedIndices: [0, 2],
            walletId: walletA
        )
        XCTAssertEqual(result, [locks[1]]) // only slot 1 is unused
    }

    /// The `usedIndices` set is per-wallet by construction (the
    /// view derives it from `usedIdentityIndices(for: walletId)`),
    /// so a slot used on wallet B must not bleed into wallet A's
    /// pickability check. We model that by simply not including
    /// wallet-B's slots in `usedIndices` when filtering for A.
    func testUsedSlotsScopedPerWallet() {
        let locks: [FakeAssetLockRow] = [
            FakeAssetLockRow(walletId: walletA, statusRaw: 2, identityIndexRaw: 0),
        ]
        // Pretend wallet B has slot 0 used, but the caller — which
        // is filtering for wallet A — never tells us that. Lock on
        // wallet A at slot 0 must stay resumable.
        let result = CreateIdentityView.resumableLocks(
            in: locks,
            usedIndices: [],
            walletId: walletA
        )
        XCTAssertEqual(result, locks)
    }

    /// `identityIndexRaw` is `Int32` (the storage row type) but the
    /// in-use index set is `Set<UInt32>` (the FFI / wallet-side
    /// type). The filter bridges via `UInt32(bitPattern:)`. A
    /// negative `Int32` therefore maps to a high `UInt32`. This
    /// test pins that conversion so a future cast change (e.g.
    /// `UInt32(lockIdentityIndexRaw)` which would trap on negative)
    /// fails loudly here instead of crashing in production.
    func testNegativeIdentityIndexBridgesViaBitPattern() {
        let negativeIndex: Int32 = -1
        let bridged = UInt32(bitPattern: negativeIndex) // 0xFFFF_FFFF

        // Lock with slot -1 / 0xFFFF_FFFF — when that exact bridged
        // value is in `usedIndices`, the lock must be filtered out.
        let locks: [FakeAssetLockRow] = [
            FakeAssetLockRow(walletId: walletA, statusRaw: 2, identityIndexRaw: negativeIndex)
        ]
        let blocked = CreateIdentityView.resumableLocks(
            in: locks,
            usedIndices: [bridged],
            walletId: walletA
        )
        XCTAssertTrue(blocked.isEmpty)

        // ...and when it's NOT in `usedIndices`, the lock stays.
        let kept = CreateIdentityView.resumableLocks(
            in: locks,
            usedIndices: [],
            walletId: walletA
        )
        XCTAssertEqual(kept, locks)
    }

    // MARK: - empty inputs

    func testEmptyLocksListReturnsEmpty() {
        let result = CreateIdentityView.resumableLocks(
            in: [FakeAssetLockRow](),
            usedIndices: [0, 1, 2],
            walletId: walletA
        )
        XCTAssertTrue(result.isEmpty)
    }

    // MARK: - cross-wallet anti-join (Identities-tab surface)

    /// `IdentitiesContentView.resumableRegistrationsSection` is the
    /// surface that catches orphan locks after an app crash — the
    /// in-memory `RegistrationCoordinator` map is wiped on restart,
    /// so this section is the user's only signal that an asset
    /// lock at `InstantSendLocked` / `ChainLocked` is waiting to be
    /// resumed. It uses `crossWalletResumableLocks(in:usedSlots:)`
    /// instead of the per-wallet helper because the section spans
    /// every wallet in one pass.
    ///
    /// The pair of tests below pin the two pieces of business
    /// logic that differ from the per-wallet form:
    ///
    ///   1. The `usedSlots` set is keyed by
    ///      `(walletId, identityIndex)` — a slot used on wallet A
    ///      must not block the same numerical slot on wallet B.
    ///   2. The cross-wallet pass still enforces the `statusRaw >=
    ///      2` floor — Built / Broadcast locks aren't resumable
    ///      regardless of which wallet they live on.
    func testCrossWalletFilterDoesNotBleedSlotsAcrossWallets() {
        // Same numerical slot (0) on both wallets, but only
        // wallet-A's slot is taken by an existing identity. The
        // lock on wallet B at slot 0 must stay resumable — the
        // user's identity on A has no bearing on B's slot pool.
        let locks: [FakeAssetLockRow] = [
            FakeAssetLockRow(walletId: walletA, statusRaw: 2, identityIndexRaw: 0),
            FakeAssetLockRow(walletId: walletB, statusRaw: 2, identityIndexRaw: 0),
        ]
        let usedSlots: Set<IdentitiesContentView.UsedSlot> = [
            IdentitiesContentView.UsedSlot(walletId: walletA, slot: 0)
        ]
        let result = IdentitiesContentView.crossWalletResumableLocks(
            in: locks,
            usedSlots: usedSlots
        )
        // Only wallet B's lock survives.
        XCTAssertEqual(result, [locks[1]])
    }

    func testCrossWalletFilterEnforcesStatusFloor() {
        // Two locks on two different wallets, both at slot 0, but
        // wallet A's is `Broadcast` (statusRaw 1, pre-final) and
        // wallet B's is `ChainLocked` (statusRaw 3, final). Only
        // the ChainLocked lock should pass the filter — Broadcast
        // can't fund a Platform identity yet.
        let locks: [FakeAssetLockRow] = [
            FakeAssetLockRow(walletId: walletA, statusRaw: 1, identityIndexRaw: 0),
            FakeAssetLockRow(walletId: walletB, statusRaw: 3, identityIndexRaw: 0),
        ]
        let result = IdentitiesContentView.crossWalletResumableLocks(
            in: locks,
            usedSlots: []
        )
        XCTAssertEqual(result, [locks[1]])
    }

    /// Edge case: a lock at a slot that's marked used on its OWN
    /// wallet must still be filtered out. This is the orphan-
    /// recovery semantics in reverse — if the registration has
    /// completed (a `PersistentIdentity` row exists at the same
    /// `(walletId, identityIndex)`), the lock is no longer
    /// "resumable", it's consumed. Even if its
    /// `PersistentAssetLock` row hasn't been purged yet.
    func testCrossWalletFilterFiltersOutOwnWalletUsedSlot() {
        let locks: [FakeAssetLockRow] = [
            FakeAssetLockRow(walletId: walletA, statusRaw: 2, identityIndexRaw: 7)
        ]
        let usedSlots: Set<IdentitiesContentView.UsedSlot> = [
            IdentitiesContentView.UsedSlot(walletId: walletA, slot: 7)
        ]
        let result = IdentitiesContentView.crossWalletResumableLocks(
            in: locks,
            usedSlots: usedSlots
        )
        XCTAssertTrue(result.isEmpty)
    }
}
