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

    /// Alice's Core funds pay BOB's Platform address.
    ///
    /// The end-to-end shape of `fundFromAssetLockExternal`: two
    /// unrelated wallets, Alice builds the Core asset lock, Bob is
    /// credited an exact explicit amount, and Alice's own change
    /// address absorbs the remainder and the fee.
    ///
    /// What this pins beyond "it doesn't error":
    /// - Bob really is credited on Platform, observed from BOB's wallet
    ///   after a platform-address sync — not inferred from Alice's
    ///   changeset.
    /// - Alice's returned changeset carries ONLY her change address.
    ///   Bob's output is proven and credited, but it resolves to no
    ///   wallet-owned slot on Alice's side, so reconciliation skips it
    ///   (the `outcome.resolved == 0` / partial-resolve path). A row for
    ///   Bob appearing here would mean the wallet had mistaken a
    ///   third-party payee for one of its own addresses.
    /// - Alice's change row is persisted, which is what gates
    ///   `consume_asset_lock` on the Rust side.
    func testFundExternalRecipientFromCoreAssetLock() async throws {
        try await env.walletManager.startSpv(config: env.spvConfig)
        let alice = try await env.makeTestWallet(name: "c2p-external-alice")
        let bob = try await env.makeTestWallet(name: "c2p-external-bob")

        let coreAddress = try alice.getCoreWallet().nextReceiveAddress()
        _ = try await env.fund(address: coreAddress, dash: fundingDash)
        try await alice.waitForSpendable(exactly: fundingDuffs, timeout: 90)

        let aliceChange = try await firstPlatformAddress(
            walletId: alice.getPlatformWallet().walletId
        )
        let bobRecipient = try await firstPlatformAddress(
            walletId: bob.getPlatformWallet().walletId
        )
        XCTAssertNotEqual(
            aliceChange.hash, bobRecipient.hash,
            "test setup: the payee must not be one of Alice's own addresses"
        )

        let signer = KeychainSigner(
            modelContainer: env.modelContainer,
            network: .regtest
        )
        let aliceAddresses = try alice.getPlatformWallet().platformAddressWallet()
        let bobAddresses = try bob.getPlatformWallet().platformAddressWallet()

        XCTAssertEqual(
            try bobAddresses.totalCredits(), 0,
            "test setup: Bob starts with no platform credits"
        )

        // 0.1 DASH locked; 0.02 DASH of credits paid to Bob. The rest
        // (minus the ST fee) comes back to Alice's change address.
        let assetLockDuffs: UInt64 = 10_000_000
        // 2_000_000_000 credits == 2_000_000 duffs == 0.02 DASH
        // (`CREDITS_PER_DUFF` is 1000), comfortably above the
        // versioned `min_output_amount` of 500_000 credits.
        let payment: UInt64 = 2_000_000_000

        let updated = try await aliceAddresses.fundFromAssetLockExternal(
            amountDuffs: assetLockDuffs,
            fundingAccountIndex: 0,
            platformAccountIndex: 0,
            recipients: [
                .init(
                    addressType: bobRecipient.type,
                    hash: bobRecipient.hash,
                    credits: payment
                ),
                .init(addressType: aliceChange.type, hash: aliceChange.hash, credits: nil),
            ],
            signer: signer
        )

        // Only Alice's change output resolves to a wallet-owned slot.
        XCTAssertEqual(
            updated.count, 1,
            "the third party's output must not produce a local balance row"
        )
        XCTAssertEqual(updated.first?.hash, aliceChange.hash)
        XCTAssertGreaterThan(updated.first?.balance ?? 0, 0)
        XCTAssertFalse(
            updated.contains(where: { $0.hash == bobRecipient.hash }),
            "Alice's changeset must never carry Bob's address"
        )

        // Bob is credited on Platform — observed from Bob's own wallet.
        try await env.walletManager.syncPlatformAddressNow()
        try await Wait.until(
            "Bob's platform address reflects the external funding",
            timeout: 60,
            pollInterval: 0.5
        ) {
            try bobAddresses.totalCredits() == payment
        }
        XCTAssertEqual(
            try bobAddresses.addressesWithBalances()
                .first(where: { $0.hash == bobRecipient.hash })?.balance,
            payment,
            "Bob must be credited exactly the explicit amount — the fee comes out of "
                + "Alice's remainder, not out of the payee's output"
        )

        // Alice's change row reached disk (this is what gates
        // `consume_asset_lock`), and no row was written for Bob under
        // Alice's wallet id.
        let container = env.modelContainer
        let aliceWalletId = alice.getPlatformWallet().walletId
        let aliceHash = aliceChange.hash
        let bobHash = bobRecipient.hash
        try await Wait.until(
            "Alice's change address row carries the reconciled balance",
            timeout: 30,
            pollInterval: 0.1
        ) {
            try await MainActor.run {
                let ctx = ModelContext(container)
                let rows = try ctx.fetch(
                    FetchDescriptor<PersistentPlatformAddress>(
                        predicate: #Predicate<PersistentPlatformAddress> {
                            $0.walletId == aliceWalletId
                        }
                    )
                )
                XCTAssertFalse(
                    rows.contains(where: { $0.addressHash == bobHash }),
                    "Bob's address must never be persisted under Alice's wallet"
                )
                return rows.first(where: { $0.addressHash == aliceHash })
                    .map { $0.balance > 0 } ?? false
            }
        }
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
