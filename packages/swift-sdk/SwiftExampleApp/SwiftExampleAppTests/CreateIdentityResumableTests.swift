import XCTest
@testable import SwiftExampleApp

/// Tests the pure anti-join that powers the Identities-tab
/// "Resumable Registrations" section. This is the *only* surface
/// the user has post-app-restart to see an in-flight registration:
/// the in-memory `RegistrationCoordinator` map is wiped on relaunch,
/// so without the SwiftData-backed cross-wallet filter the user
/// would have no signal that an orphan asset lock is waiting.
///
/// The filter has four pieces of business logic that can silently
/// regress:
///
///   1. `statusRaw >= 1` floor — Built (0) is rejected (tight
///      crash window with no useful UX action), Broadcast (1) and
///      every later status are accepted. The row's *trailing
///      affordance* — spinner vs. Resume button — is staged on
///      `statusRaw >= 2` separately inside `ResumableRegistrationRow`,
///      not at the filter level.
///   2. Anti-join on `(walletId, identityIndex)` — a slot taken
///      by a `PersistentIdentity` row removes its lock from the
///      surface even if the lock's own status would otherwise
///      qualify. This is what makes the filter "orphan-only".
///   3. The anti-join is **per-wallet**: slot 0 used on wallet A
///      must not block slot 0 on wallet B. The `UsedSlot` value
///      key (walletId + slot) carries that scoping.
///   4. `identityIndexRaw` is stored as `Int32` but compared as
///      `UInt32` via `UInt32(bitPattern:)`. A future cast change
///      (e.g. `UInt32(lockIdentityIndexRaw)` — which would trap on
///      negative inputs) must fail loudly here, not in production.
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

    // MARK: - status floor

    func testRejectsBuiltLocks() {
        // Built (0) is the tight crash window between "build TX" and
        // "broadcast TX". There's no useful resume action, so it
        // shouldn't appear on the Identities tab at all.
        let locks = [
            FakeAssetLockRow(walletId: walletA, statusRaw: 0, identityIndexRaw: 0)
        ]
        let result = IdentitiesContentView.crossWalletResumableLocks(
            in: locks,
            usedSlots: []
        )
        XCTAssertTrue(result.isEmpty)
    }

    /// Broadcast (1) must surface even though it's not actionable —
    /// the row renders a spinner ("Waiting for InstantSendLock…")
    /// until SPV delivers the IS lock and the persister flips it
    /// to (2). Hiding (1) would create the UX asymmetry where a
    /// just-broadcast lock vanishes from the UI for ~10-30 seconds
    /// and then reappears at (2) — confusing rather than reassuring.
    func testAcceptsBroadcastForVisibility() {
        let lock = FakeAssetLockRow(walletId: walletA, statusRaw: 1, identityIndexRaw: 0)
        let result = IdentitiesContentView.crossWalletResumableLocks(
            in: [lock],
            usedSlots: []
        )
        XCTAssertEqual(result, [lock])
    }

    func testAcceptsInstantSendLockedAndChainLocked() {
        let locks = [
            FakeAssetLockRow(walletId: walletA, statusRaw: 2, identityIndexRaw: 0),
            FakeAssetLockRow(walletId: walletA, statusRaw: 3, identityIndexRaw: 1),
        ]
        let result = IdentitiesContentView.crossWalletResumableLocks(
            in: locks,
            usedSlots: []
        )
        XCTAssertEqual(result, locks)
    }

    /// Defensive: any unknown future status (e.g. 4) should still
    /// pass the `>= 1` floor. If we ever flip from "≥1 means
    /// surfaced" to a closed set, this test will need updating —
    /// that's intentional, the surprise is signal.
    func testForwardCompatibleStatusFloor() {
        let lock = FakeAssetLockRow(walletId: walletA, statusRaw: 4, identityIndexRaw: 0)
        let result = IdentitiesContentView.crossWalletResumableLocks(
            in: [lock],
            usedSlots: []
        )
        XCTAssertEqual(result, [lock])
    }

    // MARK: - anti-join

    func testFiltersOutLocksWhoseOwnWalletSlotIsAlreadyUsed() {
        // A registration that completed normally leaves both a
        // `PersistentAssetLock` and a `PersistentIdentity` row. The
        // lock is "consumed" even if it hasn't been purged yet, so
        // the surface must hide it.
        let lock = FakeAssetLockRow(walletId: walletA, statusRaw: 2, identityIndexRaw: 7)
        let usedSlots: Set<IdentitiesContentView.UsedSlot> = [
            IdentitiesContentView.UsedSlot(walletId: walletA, slot: 7)
        ]
        let result = IdentitiesContentView.crossWalletResumableLocks(
            in: [lock],
            usedSlots: usedSlots
        )
        XCTAssertTrue(result.isEmpty)
    }

    func testDoesNotBleedUsedSlotsAcrossWallets() {
        // Same numerical slot (0) on both wallets, but only
        // wallet-A's slot is taken by an existing identity. The
        // lock on wallet B at slot 0 must stay surfaced — the
        // user's identity on A has no bearing on B's slot pool.
        let locks = [
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
        XCTAssertEqual(result, [locks[1]])
    }

    // MARK: - Int32 -> UInt32 bridge

    /// `identityIndexRaw` is `Int32` (the storage row type) but the
    /// `UsedSlot.slot` field is `UInt32` (the FFI / wallet-side
    /// type). The filter bridges via `UInt32(bitPattern:)`. A
    /// negative `Int32` maps to a high `UInt32`. This test pins
    /// the conversion so a future change to `UInt32(...)` —
    /// which would trap on negative inputs — fails loudly here
    /// instead of crashing in production.
    func testNegativeIdentityIndexBridgesViaBitPattern() {
        let negativeIndex: Int32 = -1
        let bridged = UInt32(bitPattern: negativeIndex) // 0xFFFF_FFFF
        let lock = FakeAssetLockRow(
            walletId: walletA,
            statusRaw: 2,
            identityIndexRaw: negativeIndex
        )

        // With the bridged slot in `usedSlots`, the lock is filtered.
        let blocked = IdentitiesContentView.crossWalletResumableLocks(
            in: [lock],
            usedSlots: [IdentitiesContentView.UsedSlot(walletId: walletA, slot: bridged)]
        )
        XCTAssertTrue(blocked.isEmpty)

        // Without it, the lock stays.
        let kept = IdentitiesContentView.crossWalletResumableLocks(
            in: [lock],
            usedSlots: []
        )
        XCTAssertEqual(kept, [lock])
    }

    // MARK: - empty inputs

    func testEmptyLocksReturnsEmpty() {
        let result = IdentitiesContentView.crossWalletResumableLocks(
            in: [FakeAssetLockRow](),
            usedSlots: [
                IdentitiesContentView.UsedSlot(walletId: walletA, slot: 0)
            ]
        )
        XCTAssertTrue(result.isEmpty)
    }

    // MARK: - in-flight controller exclusion

    /// Regression for the double-counting bug: during a normal
    /// in-session registration, the asset lock reaches `Broadcast`
    /// (or higher) **before** the persister writes a
    /// `PersistentIdentity` row. Without including in-flight
    /// controller slots in `usedSlots`, the same lock would
    /// appear in BOTH the in-memory Pending Registrations list
    /// and the SwiftData-backed Resumable Registrations section
    /// — and a second tap on Resume would race a duplicate FFI
    /// call against the original. The fix is structural: the
    /// `ResumableRegistrationsList` view unions identity-claimed
    /// slots with `controller.phase.isActive` slots before
    /// calling `crossWalletResumableLocks`. The unioned set is
    /// what gets passed here; this test pins that the filter
    /// honors active-slot occupancy the same way it honors
    /// identity-row occupancy.
    func testInFlightSlotIsExcludedFromResumableSurface() {
        let lock = FakeAssetLockRow(walletId: walletA, statusRaw: 2, identityIndexRaw: 3)
        // No `PersistentIdentity` exists at (walletA, 3) yet, but a
        // controller in `.inFlight` does. The union of identity
        // slots (empty) and active slots (one) must hide the lock.
        let activeSlots: Set<IdentitiesContentView.UsedSlot> = [
            IdentitiesContentView.UsedSlot(walletId: walletA, slot: 3)
        ]
        let result = IdentitiesContentView.crossWalletResumableLocks(
            in: [lock],
            usedSlots: activeSlots
        )
        XCTAssertTrue(result.isEmpty)
    }

    /// Exhaustive predicate test for which controller phases hold
    /// the slot. The Resumable surface uses this to decide which
    /// in-memory controllers block their lock from re-appearing
    /// in the section. Changing this predicate without thinking
    /// about both the Pending and Resumable surfaces is how
    /// double-tap-Resume bugs come back.
    func testControllerPhaseIsActivePredicate() {
        XCTAssertFalse(IdentityRegistrationController.Phase.idle.isActive,
                       ".idle: pre-submit — no slot occupancy yet")
        XCTAssertTrue(IdentityRegistrationController.Phase.preparingKeys.isActive,
                      ".preparingKeys: keys derived; FFI imminent — slot held")
        XCTAssertTrue(IdentityRegistrationController.Phase.inFlight.isActive,
                      ".inFlight: FFI mid-call — slot held")
        XCTAssertFalse(
            IdentityRegistrationController.Phase
                .completed(identityId: Data(repeating: 0xCC, count: 32))
                .isActive,
            ".completed: PersistentIdentity row covers the slot via the identity anti-join — don't double-block"
        )
        XCTAssertFalse(IdentityRegistrationController.Phase.failed("nope").isActive,
                       ".failed: user is expected to retry — let the lock resurface")
    }
}
