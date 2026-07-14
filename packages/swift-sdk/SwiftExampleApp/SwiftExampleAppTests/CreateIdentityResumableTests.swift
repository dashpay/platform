import XCTest
import SwiftDashSDK
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
        /// Defaults to 0 (IdentityRegistration) so the pre-existing cases
        /// exercise the non-invitation path unchanged; the invitation
        /// exclusion tests pass 3 (IdentityInvitation) explicitly.
        var fundingTypeRaw: Int = 0
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

    /// `statusRaw == 4` (Consumed) is the terminal state for a lock
    /// that already funded an identity. It must NOT surface on the
    /// Resumable Registrations list: in the happy path the anti-join
    /// against `PersistentIdentity` hides it, but the local-only
    /// delete-identity action removes the identity row WITHOUT the
    /// asset-lock row, which frees the slot in the anti-join. Without
    /// the upper bound (`<= 3`) on the filter, a Consumed lock would
    /// re-surface as a perpetual-spinner row that can't be advanced
    /// (`resume_asset_lock` rejects Consumed entries with
    /// "already Consumed — nothing to resume").
    func testConsumedLocksAreHiddenFromResumableList() {
        let lock = FakeAssetLockRow(walletId: walletA, statusRaw: 4, identityIndexRaw: 0)
        let result = IdentitiesContentView.crossWalletResumableLocks(
            in: [lock],
            usedSlots: []
        )
        XCTAssertEqual(result, [])
    }

    // MARK: - invitation-voucher exclusion

    /// An `IdentityInvitation` (fundingTypeRaw 3) lock is a shared bearer
    /// voucher — it must NEVER surface as a resumable registration, even at
    /// a fully actionable status with its nominal slot 0 unused (the exact
    /// state of every unclaimed voucher). Resuming it would consume the
    /// voucher into an unrelated local identity and kill the invitee's claim;
    /// vouchers are recovered only via the Invitations screen's reclaim flow.
    func testInvitationVouchersAreExcludedFromResumableList() {
        for status in 1...3 {
            let voucher = FakeAssetLockRow(
                walletId: walletA,
                statusRaw: status,
                identityIndexRaw: 0,
                fundingTypeRaw: 3
            )
            let result = IdentitiesContentView.crossWalletResumableLocks(
                in: [voucher],
                usedSlots: []
            )
            XCTAssertEqual(
                result, [],
                "unclaimed invitation voucher at status \(status) must not be resumable"
            )
        }
    }

    /// The exclusion is invitation-specific: the other identity funding
    /// types (0 registration / 1 top-up / 2 top-up-not-bound) keep
    /// surfacing exactly as before.
    func testNonInvitationFundingTypesStillSurface() {
        let locks = (0...2).map { fundingType in
            FakeAssetLockRow(
                walletId: walletA,
                statusRaw: 2,
                identityIndexRaw: Int32(fundingType), // distinct free slots
                fundingTypeRaw: fundingType
            )
        }
        let result = IdentitiesContentView.crossWalletResumableLocks(
            in: locks,
            usedSlots: []
        )
        XCTAssertEqual(result, locks)
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

    // MARK: - outpoint hex round-trip

    /// `PersistentAssetLock.encodeOutPoint` (rawBytes → display
    /// hex) and `CreateIdentityView.parseOutPointHex` (display hex
    /// → rawBytes) are the two halves of the outpoint round-trip
    /// the resume path depends on: the persister writes the
    /// display-order hex, the submit path reads it and hands the
    /// raw wire-order bytes to the FFI. If either side flips
    /// endianness, the resume FFI silently addresses a different
    /// outpoint and the Platform proof-verification failure that
    /// follows is opaque. Pin the round-trip explicitly.
    func testOutPointHexRoundTripPreservesRawBytes() {
        // Deliberately asymmetric bytes so any endian flip is
        // visible. txid wire-order = little-endian.
        let originalTxid = Data([
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10,
            0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
            0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F, 0x20,
        ])
        let originalVout: UInt32 = 7

        // The persister writes the 36-byte (txid_le || vout_le)
        // blob — assemble it the same way and encode.
        var raw = Data(originalTxid)
        withUnsafeBytes(of: originalVout.littleEndian) { raw.append(contentsOf: $0) }
        XCTAssertEqual(raw.count, 36)
        let hex = PersistentAssetLock.encodeOutPoint(rawBytes: raw)

        // Decode via the submit-path parser.
        guard let (rtTxid, rtVout) = CreateIdentityView.parseOutPointHex(hex) else {
            XCTFail("parseOutPointHex returned nil for canonical encoding \(hex)")
            return
        }
        XCTAssertEqual(rtTxid, originalTxid,
                       "txid bytes round-tripped through display-hex must match")
        XCTAssertEqual(rtVout, originalVout)
    }

    /// Defensive: parseOutPointHex must reject malformed inputs
    /// instead of silently producing zeros. Same risk class as the
    /// endianness flip — a "valid-looking" but wrong outpoint
    /// would fail at the Platform layer with confusing errors.
    func testParseOutPointHexRejectsMalformedInputs() {
        XCTAssertNil(CreateIdentityView.parseOutPointHex(""))
        XCTAssertNil(CreateIdentityView.parseOutPointHex("nope"))
        XCTAssertNil(CreateIdentityView.parseOutPointHex("abc:1"))  // txid too short
        XCTAssertNil(CreateIdentityView.parseOutPointHex(
            String(repeating: "a", count: 64) + ":bogus"))            // vout NaN
        XCTAssertNil(CreateIdentityView.parseOutPointHex(
            String(repeating: "g", count: 64) + ":0"))                // non-hex chars
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
        XCTAssertTrue(
            IdentityRegistrationController.Phase
                .unconfirmed(
                    identityId: Data(repeating: 0xDD, count: 32),
                    message: "pending"
                )
                .isActive,
            ".unconfirmed: identity is probably live on chain — keep the slot held so a re-submission can't burn funds against the registered-key-hash check"
        )
    }

    // MARK: - network-switch gate

    /// The network picker's `.disabled(_:)` gate reads
    /// `RegistrationCoordinator.hasInFlightRegistrations`. That predicate
    /// must hold for an `.unconfirmed` slot (the same reason the dismissal
    /// gate does): switching networks tears down the
    /// `PlatformWalletManager` and with it the coordinator + the Rust-side
    /// note reservation, so releasing the gate for an `.unconfirmed`
    /// controller lets the same HD slot be re-selected and burn funds
    /// against the registered-key-hash check.
    ///
    /// The gate is implemented as `controller.phase.isActive` so it
    /// cannot list a different set of phases than the slot-occupancy
    /// model. Reaching `.unconfirmed` directly requires throwing the
    /// SDK's `ShieldedIdentityCreateUnconfirmedError`, whose initializer
    /// is `internal` to `SwiftDashSDK` and not constructible here — so
    /// the `.unconfirmed → gate held` leg is pinned transitively: the
    /// exhaustive `testControllerPhaseIsActivePredicate` already asserts
    /// `.unconfirmed.isActive == true`, and this test pins that the gate
    /// is exactly `isActive` over the map (true iff some controller is
    /// active, false when all are terminal-non-active).
    @MainActor
    func testNetworkSwitchGateMatchesPhaseIsActive() async throws {
        let coordinator = RegistrationCoordinator()
        let walletId = Data(repeating: 0xAB, count: 32)

        XCTAssertFalse(
            coordinator.hasInFlightRegistrations,
            "empty coordinator holds nothing in flight"
        )

        // A controller that fails terminally is `.failed` (isActive ==
        // false): the gate must release once it's the only entry.
        let failing = coordinator.startRegistration(
            walletId: walletId,
            identityIndex: 1,
            fundingKind: .shieldedPool
        ) {
            throw PlatformWalletError.shieldedBroadcastFailed("rejected on merits")
        }
        for _ in 0..<200 {
            if case .failed = failing.phase { break }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        guard case .failed = failing.phase else {
            return XCTFail("controller did not reach .failed in time")
        }
        XCTAssertFalse(failing.phase.isActive)
        XCTAssertFalse(
            coordinator.hasInFlightRegistrations,
            "a lone .failed controller must not hold the network-switch gate (it's not isActive)"
        )

        // A controller stuck `.inFlight` (isActive == true) — the gate
        // must hold. Use a never-returning body so the phase stays
        // `.inFlight` for the duration of the assertion.
        let inFlight = coordinator.startRegistration(
            walletId: walletId,
            identityIndex: 2,
            fundingKind: .shieldedPool
        ) {
            try await Task.sleep(nanoseconds: 60_000_000_000)
            return Data()
        }
        XCTAssertTrue(inFlight.phase.isActive)
        XCTAssertTrue(
            coordinator.hasInFlightRegistrations,
            "an active (.inFlight) controller must hold the network-switch gate; the gate is exactly phase.isActive, which also covers .unconfirmed"
        )
    }
}
