import XCTest
import SwiftData
@testable import SwiftDashSDK

final class CoreToPlatformIntegrationTests: IntegrationTestCase {
    private let fundingDash: Double = 0.5
    private var fundingDuffs: UInt64 { UInt64(fundingDash * 1e8) }

    /// Fund a Platform address from Core: build an asset lock from the
    /// Core wallet's UTXOs and credit one of the wallet's own platform
    /// addresses, then assert its credit balance reflects the funding.
    func testFundAddressReflectsInWallet() async throws {
        try await env.walletManager.startSpv(config: env.spvConfig)
        let alice = try await env.makeTestWallet(name: "c2p-fund-address")

        // Core UTXOs to source the asset lock from.
        let coreAddress = try alice.getCoreWallet().nextReceiveAddress()
        _ = try await env.fund(address: coreAddress, dash: fundingDash)
        try await alice.waitForSpendable(exactly: fundingDuffs, timeout: 90)

        let recipient = try await firstPlatformAddress(
            walletId: alice.getPlatformWallet().walletId
        )

        let signer = KeychainSigner(
            modelContainer: env.modelContainer,
            network: .regtest
        )
        let addressWallet = try alice.getPlatformWallet().platformAddressWallet()

        let assetLockDuffs: UInt64 = 10_000_000
        let updated = try await addressWallet.fundFromAssetLock(
            amountDuffs: assetLockDuffs,
            fundingAccountIndex: 0,
            platformAccountIndex: 0,
            recipients: [
                .init(addressType: recipient.type, hash: recipient.hash, credits: nil)
            ],
            signer: signer
        )

        XCTAssertEqual(updated.count, 1)
        XCTAssertGreaterThan(updated.first?.balance ?? 0, 0)
        XCTAssertGreaterThan(try addressWallet.totalCredits(), 0)
    }

    /// Lowest-index platform address for `walletId`, polled because the
    /// emit lands on the persister's background context.
    private func firstPlatformAddress(
        walletId: Data,
        timeout: TimeInterval = 10
    ) async throws -> (type: UInt8, hash: Data) {
        let container = env.modelContainer
        let deadline = Date().addingTimeInterval(timeout)

        while Date() < deadline {
            let found = try await MainActor.run { () -> (UInt8, Data)? in
                let ctx = ModelContext(container)

                let rows = try ctx.fetch(FetchDescriptor<PersistentPlatformAddress>(
                    predicate: #Predicate<PersistentPlatformAddress> { $0.walletId == walletId }
                ))

                guard let row = rows.min(by: { $0.addressIndex < $1.addressIndex }) else {
                    return nil
                }

                return (row.addressType, row.addressHash)
            }

            if let found { return (found.0, found.1) }

            try await Task.sleep(nanoseconds: 100_000_000)
        }

        XCTFail("no PersistentPlatformAddress row for wallet within \(timeout)s")
        throw SPVTestWaitError.timeout("no platform address emitted")
    }

    func testRegisterIdentityWithCoreFunding() async throws {
        try await env.walletManager.startSpv(config: env.spvConfig)
        let alice = try await env.makeTestWallet(name: "c2p-fund-identity")

        let address = try alice.getCoreWallet().nextReceiveAddress()
        _ = try await env.fund(address: address, dash: fundingDash)
        try await alice.waitForSpendable(exactly: fundingDuffs, timeout: 90)

        let wallet = alice.getPlatformWallet()
        let identityIndex: UInt32 = 0
        let keyCount: UInt32 = 3

        let pubkeys = try wallet.prePersistIdentityKeysForRegistration(
            identityIndex: identityIndex,
            keyCount: keyCount,
            network: .regtest
        )
        XCTAssertEqual(pubkeys.count, Int(keyCount))

        let signer = KeychainSigner(
            modelContainer: env.modelContainer,
            network: .regtest
        )

        // 0.1 DASH of credits — comfortably above the ~221.5k-duff
        // floor for three keys.
        let assetLockDuffs: UInt64 = 10_000_000
        let (identityId, identity) = try await wallet.registerIdentityWithFunding(
            amountDuffs: assetLockDuffs,
            accountIndex: 0,
            identityIndex: identityIndex,
            identityPubkeys: pubkeys,
            signer: signer
        )

        XCTAssertEqual(identityId.count, 32)

        // The registered identity carries the asset-lock credits
        // (minus the registration cost).
        let credits = try identity.getBalance()
        XCTAssertGreaterThan(credits, 0)

        // The identity persister writes the `PersistentIdentity` row on
        // a background context, so poll rather than read once.
        let container = env.modelContainer
        try await Wait.until(
            "PersistentIdentity row for the registered identity",
            timeout: 30,
            pollInterval: 0.1
        ) {
            try await MainActor.run {
                let ctx = ModelContext(container)
                let rows = try ctx.fetch(FetchDescriptor<PersistentIdentity>(
                    predicate: #Predicate<PersistentIdentity> { $0.identityId == identityId }
                ))
                guard let row = rows.first else { return false }
                return row.balance > 0
            }
        }
    }
}
