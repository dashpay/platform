import Foundation
import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

/// A wallet imported into a RUNNING SPV client must end up holding *every* one
/// of its transactions — the ones that predate it and the ones that arrive
/// after it — and not merely the right aggregate balance.
///
/// Companion to `SpvLateWalletBackfillIntegrationTests`, which imports a
/// single-transaction wallet and stops at the backfill. Here the funding is
/// many transactions across many addresses, and it straddles the registration
/// so the two recovery paths are each exercised by a half that the *other*
/// path provably cannot reach:
///
/// - the **pre-registration** half is mined and fully scanned while no wallet
///   exists. Script matching runs against the registered wallets' scripts, so
///   nothing could have matched it at the time; it can only come back through
///   the genesis rescan that registering a `birthHeight: 0` wallet starts.
/// - the **post-registration** half is mined *after* `createWallet` returns, so
///   it is above the ceiling that rescan swept and can only be found by live
///   filter matching against the now-registered wallet.
///
/// Neither half depends on timing — the test waits for the client to reach the
/// tip before it registers, and registers before it funds again — so a
/// regression in either path fails the test rather than racing past it.
///
/// Both the balance and the persisted per-transaction history are asserted.
/// They are separate write paths — the balance atomic on one side,
/// `PersistentTransaction` rows written by `PlatformWalletPersistenceHandler`
/// on the other — so a regression that restores the right total while dropping
/// individual transaction records would otherwise pass unnoticed.
final class SpvManyTxWalletImportHistoryIntegrationTests: IntegrationTestCase {
    /// Funding transactions per half — one per distinct external address, one
    /// per block. `2 * halfTxCount` has to stay inside the default BIP-44 gap
    /// limit so the imported wallet derives every funded address by itself.
    private let halfTxCount = 5

    /// Funding amount per address, held in duffs so the DASH figure handed to
    /// `sendtoaddress` is derived from it and not the other way round:
    /// `UInt64(dash * 1e8)` truncates for values such as 0.0001.
    private let amountEachDuffs: UInt64 = 100_000
    private var amountEachDash: Double { Double(amountEachDuffs) / 1e8 }

    func testImportedWalletRecoversHistoryEitherSideOfRegistration() async throws {
        let mnemonic = try Mnemonic.generate(wordCount: 24)

        // 1. Derive the receive addresses from a throwaway KeyWallet manager
        //    (regtest-encoded — the manager's network drives encoding), so the
        //    SPV-driving manager stays empty and the client knows nothing about
        //    this wallet until step 4.
        let km = try WalletManager(network: .regtest)
        let walletId = try km.addWallet(mnemonic: mnemonic)
        let addresses = try Self.deriveExternalAddresses(
            manager: km, walletId: walletId, count: halfTxCount * 2
        )

        // 2. Fund the pre-registration half, one transaction per block.
        var preTxids: [Data] = []
        for address in addresses.prefix(halfTxCount) {
            preTxids.append(try await fund(address))
        }

        // 3. Start the SPV client EMPTY and let it reach the tip. It matches
        //    nothing — there is no wallet to match against — so this half is
        //    now provably swept with no wallet having seen it.
        try await env.walletManager.startSpv(config: env.spvConfig)
        try await env.walletManager.waitUntilUpToDate(
            height: try await env.coreRPC.getBlockCount(),
            timeout: 180
        )

        // 4. Import the wallet into the running client with birthHeight 0 — per
        //    the `createWallet` contract that means "scan history from
        //    genesis". Only that rescan can recover step 2's funding.
        let imported = try await env.walletManager.createWallet(
            mnemonic: mnemonic,
            network: .regtest,
            name: "many-tx",
            createDefaultAccounts: true,
            birthHeight: 0
        )

        let preTotal = UInt64(halfTxCount) * amountEachDuffs
        _ = try? await Wait.until(
            "rescan backfills \(halfTxCount) pre-registration transactions",
            timeout: 120,
            pollInterval: 0.5
        ) {
            try imported.balance().total == preTotal
        }
        try await assertHistory(
            imported, expectedTotal: preTotal, expectedTxids: preTxids,
            phase: "after the register-time rescan"
        )

        // 5. Fund the post-registration half against the running client. These
        //    blocks are above everything the rescan swept, so they can only be
        //    picked up by live filter matching against the registered wallet.
        var postTxids: [Data] = []
        for address in addresses.suffix(halfTxCount) {
            postTxids.append(try await fund(address))
        }

        try await env.walletManager.waitUntilUpToDate(
            height: try await env.coreRPC.getBlockCount(),
            timeout: 180
        )

        // 6. Everything from both halves must now be present. Don't fail on the
        //    poll — `assertHistory` reports the clean assertions.
        let allTxids = preTxids + postTxids
        let expectedTotal = UInt64(allTxids.count) * amountEachDuffs
        _ = try? await Wait.until(
            "wallet reaches \(expectedTotal) duffs across \(allTxids.count) transactions",
            timeout: 120,
            pollInterval: 0.5
        ) {
            guard try imported.balance().total == expectedTotal else { return false }
            return try await self.readTxids().isSuperset(of: allTxids)
        }
        try await assertHistory(
            imported, expectedTotal: expectedTotal, expectedTxids: allTxids,
            phase: "after live matching of post-registration blocks"
        )
    }

    // MARK: - Assertions

    /// Assert the wallet's balance *and* its persisted transaction rows, so a
    /// regression in either write path is reported on its own terms.
    private func assertHistory(
        _ wallet: ManagedPlatformWallet,
        expectedTotal: UInt64,
        expectedTxids: [Data],
        phase: String,
        file: StaticString = #filePath,
        line: UInt = #line
    ) async throws {
        let total = try wallet.balance().total
        XCTAssertEqual(
            total, expectedTotal,
            "balance \(phase): total=\(total) expected=\(expectedTotal)",
            file: file, line: line
        )

        let persisted = try await readTxids()
        let missing = expectedTxids.filter { !persisted.contains($0) }
        XCTAssertTrue(
            missing.isEmpty,
            "history \(phase): \(missing.count)/\(expectedTxids.count) transactions missing — " +
            missing.map { Data($0.reversed()).toHexString() }.joined(separator: ", "),
            file: file, line: line
        )
    }

    // MARK: - Helpers

    struct SetupError: Error, CustomStringConvertible {
        let description: String
    }

    /// Fund `address` with `amountEachDuffs` and mine it into its own block.
    ///
    /// Goes through `env.fund`, which broadcasts to the masternodes and waits
    /// for the InstantSend lock before mining — without that the mine can race
    /// the propagation, which this test, funding many addresses back to back,
    /// is the most likely in the suite to hit.
    ///
    /// Returns the txid in the wire (little-endian) orientation
    /// `PersistentTransaction.txid` stores, not the display order Core's RPC
    /// hands back.
    private func fund(_ address: String) async throws -> Data {
        let displayTxid = try await env.fund(address: address, dash: amountEachDash)
        guard let displayBytes = Data(hexString: displayTxid),
              displayBytes.count == 32
        else {
            throw SetupError(description: "fund returned an unparseable txid: \(displayTxid)")
        }
        return Data(displayBytes.reversed())
    }

    // MARK: - KeyWallet address enumeration

    /// Generate and return BIP-44 external (receive) addresses in
    /// `[0, count)` from a KeyWallet `WalletManager` handle.
    ///
    /// `managed_wallet_get_bip_44_external_address_range` is a single Rust-side
    /// call that generates addresses lazily if they don't yet exist, so it can
    /// hand back the full 0..<count range regardless of the default gap limit —
    /// no Swift-side derivation or gap-limit walking required.
    private static func deriveExternalAddresses(
        manager: WalletManager, walletId: Data, count: Int
    ) throws -> [String] {
        var error = FFIError()

        guard let managedInfo = walletId.withUnsafeBytes({ raw in
            wallet_manager_get_managed_wallet_info(
                manager.ffiHandle,
                raw.bindMemory(to: UInt8.self).baseAddress,
                &error
            )
        }) else {
            throw SetupError(description: "wallet_manager_get_managed_wallet_info failed: \(Self.take(&error))")
        }
        defer { managed_wallet_info_free(managedInfo) }

        // Owning: the manager clones the wallet out of its map and boxes it
        // fresh on every call, so this pointer is ours to free.
        guard let wallet = walletId.withUnsafeBytes({ raw in
            wallet_manager_get_wallet(
                manager.ffiHandle,
                raw.bindMemory(to: UInt8.self).baseAddress,
                &error
            )
        }) else {
            throw SetupError(description: "wallet_manager_get_wallet failed: \(Self.take(&error))")
        }
        defer { wallet_free_const(wallet) }

        var addressesOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>? = nil
        var outCount = 0
        let ok = managed_wallet_get_bip_44_external_address_range(
            managedInfo,
            wallet,
            /* account_index */ 0,
            /* start_index   */ 0,
            /* end_index     */ UInt32(count),
            &addressesOut,
            &outCount,
            &error
        )
        guard ok, let arr = addressesOut else {
            throw SetupError(description: "managed_wallet_get_bip_44_external_address_range failed: \(Self.take(&error))")
        }
        defer { address_array_free(arr, outCount) }

        var result: [String] = []
        result.reserveCapacity(outCount)
        for i in 0..<outCount {
            if let cstr = arr[i] {
                result.append(String(cString: cstr))
            }
        }

        // Null entries are skipped above, so a short result would otherwise
        // surface downstream as an out-of-range crash in the funding loop.
        guard result.count == count else {
            throw SetupError(
                description: "expected \(count) external addresses, got \(result.count)"
            )
        }
        return result
    }

    /// Read (and free) an `FFIError` message, returning a display string.
    private static func take(_ error: inout FFIError) -> String {
        guard let msg = error.message else { return "code \(error.code.rawValue)" }
        defer {
            error_message_free(msg)
            error.message = nil
        }
        return String(cString: msg)
    }
}
