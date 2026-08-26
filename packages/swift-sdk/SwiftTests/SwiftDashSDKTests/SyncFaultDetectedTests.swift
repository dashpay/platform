// SyncFaultDetectedTests.swift
// SwiftDashSDKTests
//
// Unit tests for the `PlatformWalletManager.syncFaultDetected()` FFI bridge.

import XCTest
@testable import SwiftDashSDK

/// Coverage for the Swift side of the watermark-freeze fault latch
/// (dashpay/platform#4069), exported over the C ABI by PR #4314 as
/// `platform_wallet_manager_sync_fault_detected`.
///
/// `syncFaultDetected()` is a pure bridge: guard the handle, call the export,
/// return the out-param. The latch's actual semantics — the persisted
/// `syncedHeight` being deliberately held behind the chain tip after the
/// wallet-event adapter drops record-bearing events or a persistence `store()`
/// is rejected, and the flag latching `true` for the process lifetime — live in
/// Rust and are covered by the shared tests in
/// `packages/rs-platform-wallet/src/changeset/core_bridge.rs`. What is Swift's
/// to get right is the unconfigured-handle guard, which must throw rather than
/// hand `NULL_HANDLE` across the FFI boundary.
///
/// `PlatformWalletManager` is main-actor isolated, so the suite runs on the
/// main actor.
@MainActor
final class SyncFaultDetectedTests: XCTestCase {

    /// A bare `PlatformWalletManager()` has never been configured: `handle` is
    /// `NULL_HANDLE` and `isConfigured` is false, so the wrapper must throw
    /// `PlatformWalletError.invalidHandle` instead of calling the export with
    /// a null handle.
    func testSyncFaultDetectedRequiresConfiguredManager() {
        let manager = PlatformWalletManager()
        XCTAssertFalse(manager.isConfigured, "a bare manager is unconfigured")

        XCTAssertThrowsError(try manager.syncFaultDetected()) { error in
            XCTAssertTrue(error is PlatformWalletError)
            if case PlatformWalletError.invalidHandle = error {
                // Expected error
            } else {
                XCTFail("Expected invalidHandle error, got \(error)")
            }
        }
    }
}
