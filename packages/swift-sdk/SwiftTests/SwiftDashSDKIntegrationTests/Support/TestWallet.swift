import Foundation
import SwiftDashSDK

/// Wraps a `ManagedPlatformWallet` and a `ManagedCoreWallet`
final class TestWalletWrapper {
    private let core: ManagedCoreWallet
    private let wallet: ManagedPlatformWallet

    init(wallet: ManagedPlatformWallet, core: ManagedCoreWallet) {
        self.wallet = wallet
        self.core = core
    }

    func getCoreWallet() -> ManagedCoreWallet {
        core
    }

    func getPlatformWallet() -> ManagedPlatformWallet {
        wallet
    }

    func waitForSpendable(exactly duffs: UInt64, timeout: TimeInterval = 60) async throws {
        try await Wait.until(
            "wallet spendable == \(duffs) duffs",
            timeout: timeout,
            pollInterval: 0.01
        ) {
            try wallet.balance().spendable == duffs
        }
    }

    /// Build, sign, and broadcast a single-recipient send from BIP44 account 0,
    /// mirroring `SendViewModel`'s `.coreToCore` flow (the SDK no longer exposes
    /// a one-shot `sendToAddresses`). Returns the signed transaction so callers
    /// can read its consensus-serialized `data` or derive the broadcast txid.
    @discardableResult
    func send(toAddress address: String, amountDuffs: UInt64) throws -> CoreTransaction {
        let builder = try CoreTransactionBuilder(network: core.network())
        try builder.addOutput(address: address, amountDuffs: amountDuffs)
        try builder.setFunding(wallet: wallet, accountType: .bip44, accountIndex: 0)
        let signedTx = try builder.buildSigned(
            wallet: wallet,
            accountType: .bip44,
            accountIndex: 0
        )
        _ = try core.broadcastTransaction(signedTx)
        return signedTx
    }
}

enum Wait {
    struct TimeoutError: Error, CustomStringConvertible {
        let description: String
    }

    static func until(
    _ message: @autoclosure () -> String,
    timeout: TimeInterval = 60,
    pollInterval: TimeInterval = 0.5,
    _ condition: () async throws -> Bool
    ) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if try await condition() { return }
            try await Task.sleep(nanoseconds: UInt64(pollInterval * 1_000_000_000))
        }
        throw TimeoutError(description: "Timed out after \(timeout)s waiting for: \(message())")
    }
}

@MainActor
extension PlatformWalletManager {
    func waitUntilUpToDate(height: Int, timeout: TimeInterval = 10) async throws {
        let target = UInt32(max(0, height))
        let deadline = Date().addingTimeInterval(timeout)
        var lastHeaders: UInt32 = 0
        var lastFilters: UInt32 = 0

        while Date() < deadline {
            let progress = try syncProgress()
            lastHeaders = progress.headers?.currentHeight ?? 0
            lastFilters = progress.filters?.currentHeight ?? 0

            if lastHeaders >= target && lastFilters >= target { return }

            try await Task.sleep(nanoseconds: 100_000_000)
        }

        throw SPVTestWaitError.timeout(
            "SPV did not reach height \(target) within \(timeout)s — " +
            "headers=\(lastHeaders), filters=\(lastFilters)"
        )
    }
}

enum SPVTestWaitError: LocalizedError {
    case timeout(String)

    var errorDescription: String? {
        switch self {
        case .timeout(let msg): return msg
        }
    }
}