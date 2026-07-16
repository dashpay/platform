import XCTest
import SwiftData
@testable import SwiftDashSDK

@MainActor
final class TokenBalanceUInt64PersistenceTests: XCTestCase {
    private let identityId = Data(repeating: 0xC3, count: 32)

    func testFullUInt64DomainRoundTripsAndSortsUnsigned() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        let dataManager = DataManager(modelContext: context, currentNetwork: .testnet)
        let boundaries: [(String, UInt64)] = [
            ("zero", 0),
            ("signed-max", UInt64(Int64.max)),
            ("sign-bit", UInt64(1) << 63),
            ("unsigned-max", UInt64.max),
        ]

        for (tokenId, value) in boundaries {
            try dataManager.saveTokenBalance(
                tokenId: tokenId,
                identityId: identityId,
                balance: value
            )
        }

        // DAO output is unsigned-descending and lossless across the sign bit.
        let fetched = try dataManager.fetchTokenBalances(identityId: identityId)
        XCTAssertEqual(fetched.map(\.balance), boundaries.map { $0.1 }.sorted(by: >))

        // Every non-zero raw bit pattern remains queryable even when SQLite's
        // signed carrier is negative above the sign bit.
        let nonZero = try context.fetch(
            FetchDescriptor<PersistentTokenBalance>(
                predicate: PersistentTokenBalance.nonZeroBalancesPredicate
            )
        )
        XCTAssertEqual(Set(nonZero.map(\.unsignedBalance)), Set(boundaries.dropFirst().map { $0.1 }))
        XCTAssertEqual(
            nonZero.first(where: { $0.tokenId == "unsigned-max" })?.balance,
            Int64(bitPattern: UInt64.max)
        )

        // A fresh context proves the values were not merely retained in the
        // original model objects.
        let reloaded = try DataManager(
            modelContext: ModelContext(container),
            currentNetwork: .testnet
        ).fetchTokenBalances(identityId: identityId)
        XCTAssertEqual(reloaded.map(\.balance), fetched.map(\.balance))
    }

    func testOriginalSignedColumnPreservesUnsignedRawBits() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        let row = PersistentTokenBalance(
            tokenId: "raw-bits",
            identityId: identityId,
            balance: UInt64(1) << 63,
            network: .testnet
        )
        context.insert(row)
        try context.save()

        let manager = DataManager(modelContext: context, currentNetwork: .testnet)
        XCTAssertEqual(
            try manager.fetchTokenBalances(identityId: identityId).first?.balance,
            UInt64(1) << 63
        )
        XCTAssertEqual(row.balance, Int64.min)
        XCTAssertEqual(row.unsignedBalance, UInt64(1) << 63)
    }

    func testFormattingNeverConvertsProtocolAmountsThroughDouble() {
        let maximum = PersistentTokenBalance(
            tokenId: "max",
            identityId: identityId,
            balance: UInt64.max,
            tokenDecimals: 8,
            network: .testnet
        )
        XCTAssertEqual(maximum.formattedBalance, "184467440737.09551615")

        let signBit = PersistentTokenBalance(
            tokenId: "sign-bit",
            identityId: identityId,
            balance: UInt64(1) << 63,
            network: .testnet
        )
        XCTAssertEqual(signBit.formattedBalance, "9223372036854775808")

        XCTAssertEqual(
            PersistentToken.formatSupply(String(UInt64.max), decimals: 0),
            "18,446,744,073,709,551,615"
        )
        XCTAssertEqual(
            PersistentToken.formatSupply(String(UInt64.max), decimals: 8),
            "184,467,440,737.09551615"
        )
    }

    func testBalanceFormattingFallsBackToRelatedTokenMetadata() {
        let token = PersistentToken(
            contractId: Data(repeating: 0x11, count: 32),
            position: 0,
            name: "TKN",
            baseSupply: "0",
            decimals: 8
        )
        let balance = PersistentTokenBalance(
            tokenId: "token",
            identityId: identityId,
            balance: UInt64.max,
            network: .testnet
        )
        balance.token = token
        XCTAssertNil(balance.tokenDecimals)
        XCTAssertNil(balance.tokenSymbol)
        XCTAssertEqual(balance.displayBalance, "184467440737.09551615 TKN")
    }
}
