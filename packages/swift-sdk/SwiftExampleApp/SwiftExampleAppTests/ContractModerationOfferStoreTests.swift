import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

/// Pins how the moderation hub's offer store merges the wallets' answers and
/// reports the wallets it could not ask: no wallet asked is not "nobody
/// moderates", a wallet that is not loaded is counted while the others'
/// moderators are kept, and a failing wallet's error is shown.
///
/// Every wallet is a closure returning a decoded ``ContractModerationOffer``,
/// so no device or network is needed.
@MainActor
final class ContractModerationOfferStoreTests: XCTestCase {
    private let contractId = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"
    private let walletA = Data(repeating: 0x0a, count: 32)
    private let walletB = Data(repeating: 0x0b, count: 32)
    private let identityA = "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF"
    private let identityB = "8eTDkBhpQjHeqgbVeriwLeZr1tCa6yBGw76SckvD1cwc"

    private struct Refused: LocalizedError {
        var errorDescription: String? { "refused by the node" }
    }

    /// An offer naming `identityIds` as appointed moderators that may ban
    /// and warn, decoded from the JSON the wallet writes.
    private func offer(
        naming identityIds: [String],
        team: String = "declared",
        documentType: String? = nil
    ) throws -> ContractModerationOffer {
        let documentTypes = documentType.map {
            """
            [{"documentTypeName":"\($0)","delete":true,"restore":false,"changeableFields":[]}]
            """
        } ?? "[]"
        let identities = identityIds.map {
            """
            {"identityId":"\($0)","role":"appointedModerator","userActions":["ban","warn"],\
            "documentTypes":\(documentTypes),"approveTeamActions":false}
            """
        }.joined(separator: ",")
        let json = """
        {"contractId":"\(contractId)","moderatorsTeam":"\(team)","identities":[\(identities)]}
        """
        return try ContractModerationOffer.decode(fromJSON: json)
    }

    /// A wallet lookup over `wallets`: a wallet id not in it has no loaded
    /// wallet.
    private func lookup(
        _ wallets: [Data: ContractModerationOfferStore.AskWallet]
    ) -> (Data) -> ContractModerationOfferStore.AskWallet? {
        { wallets[$0] }
    }

    /// No wallet linked to an identity on the network is found: no wallet
    /// is asked, and the store says so instead of saying nobody moderates.
    func testShouldSayNoWalletWasAskedWhenNoLinkedWalletIsFound() async {
        let store = ContractModerationOfferStore()
        var lookups = 0

        await store.reload(contractId: contractId, walletIds: []) { _ in
            lookups += 1
            return nil
        }

        XCTAssertEqual(lookups, 0)
        XCTAssertTrue(store.loadedOnce)
        XCTAssertFalse(store.isLoading)
        XCTAssertTrue(store.noLinkedWallet)
        XCTAssertFalse(store.nobodyModerates)
        XCTAssertEqual(store.askedWalletCount, 0)
        XCTAssertEqual(store.unloadedWalletCount, 0)
        XCTAssertNil(store.errorMessage)
        XCTAssertTrue(store.moderators.isEmpty)
    }

    /// A wallet holding identities that is not loaded is counted, and the
    /// moderators another wallet named are kept.
    func testShouldReportAWalletThatIsNotLoadedAndKeepTheOthersModerators() async throws {
        let store = ContractModerationOfferStore()
        let answer = try offer(naming: [identityB])

        await store.reload(
            contractId: contractId,
            walletIds: [walletA, walletB],
            askWallet: lookup([walletB: { _ in answer }])
        )

        XCTAssertEqual(store.unloadedWalletCount, 1)
        XCTAssertEqual(store.askedWalletCount, 1)
        XCTAssertFalse(store.noLinkedWallet)
        XCTAssertFalse(store.nobodyModerates)
        XCTAssertNil(store.errorMessage)
        XCTAssertEqual(store.moderators.map(\.identityIdBase58), [identityB])
        XCTAssertEqual(store.moderators.map(\.walletId), [walletB])
        XCTAssertEqual(store.moderatorsTeam, .declared)
    }

    /// When none of the wallets holding identities is loaded, nobody was
    /// asked: the store counts them and does not say nobody moderates.
    func testShouldNotSayNobodyModeratesWhenNoWalletIsLoaded() async {
        let store = ContractModerationOfferStore()

        await store.reload(
            contractId: contractId,
            walletIds: [walletA, walletB],
            askWallet: lookup([:])
        )

        XCTAssertEqual(store.unloadedWalletCount, 2)
        XCTAssertEqual(store.askedWalletCount, 0)
        XCTAssertFalse(store.noLinkedWallet)
        XCTAssertFalse(store.nobodyModerates)
        XCTAssertNil(store.errorMessage)
        XCTAssertTrue(store.moderators.isEmpty)
    }

    /// The offers of two wallets merge, each moderator with the wallet that
    /// named it; the team is the first wallet's answer.
    func testShouldMergeTheOffersOfTwoWallets() async throws {
        let store = ContractModerationOfferStore()
        let first = try offer(naming: [identityA], team: "electedSeated", documentType: "post")
        let second = try offer(naming: [identityB], team: "electedSeated", documentType: "reply")

        await store.reload(
            contractId: contractId,
            walletIds: [walletA, walletB],
            askWallet: lookup([walletA: { _ in first }, walletB: { _ in second }])
        )

        XCTAssertEqual(store.askedWalletCount, 2)
        XCTAssertEqual(store.unloadedWalletCount, 0)
        XCTAssertNil(store.errorMessage)
        XCTAssertEqual(store.moderators.map(\.identityIdBase58), [identityA, identityB])
        XCTAssertEqual(store.moderators.map(\.walletId), [walletA, walletB])
        XCTAssertEqual(store.moderatorsTeam, .electedSeated)
        XCTAssertEqual(store.documentTypeNames, ["post", "reply"])
        XCTAssertEqual(store.userModerators.count, 2)
    }

    /// A wallet whose answer fails shows its error, and the moderators the
    /// other wallet named stand.
    func testShouldShowTheErrorOfAFailingWalletAndKeepTheOthers() async throws {
        let store = ContractModerationOfferStore()
        let answer = try offer(naming: [identityB])

        await store.reload(
            contractId: contractId,
            walletIds: [walletA, walletB],
            askWallet: lookup([
                walletA: { _ in throw Refused() },
                walletB: { _ in answer },
            ])
        )

        XCTAssertEqual(store.errorMessage, "refused by the node")
        XCTAssertEqual(store.askedWalletCount, 2)
        XCTAssertFalse(store.nobodyModerates)
        XCTAssertEqual(store.moderators.map(\.identityIdBase58), [identityB])
    }

    /// Asked wallets that answer and name nobody: the store says nobody
    /// moderates.
    func testShouldSayNobodyModeratesWhenTheAskedWalletsNameNoIdentity() async throws {
        let store = ContractModerationOfferStore()
        let answer = try offer(naming: [], team: "noModeration")

        await store.reload(
            contractId: contractId,
            walletIds: [walletA],
            askWallet: lookup([walletA: { _ in answer }])
        )

        XCTAssertTrue(store.nobodyModerates)
        XCTAssertFalse(store.noLinkedWallet)
        XCTAssertEqual(store.unloadedWalletCount, 0)
        XCTAssertEqual(store.moderatorsTeam, .noModeration)
    }

    /// Each wallet is asked with the contract's 32-byte id.
    func testShouldAskEachWalletWithTheContractIdBytes() async throws {
        let store = ContractModerationOfferStore()
        let answer = try offer(naming: [identityA])
        let expected = try XCTUnwrap(Data.identifier(fromBase58: contractId))
        XCTAssertEqual(expected.count, 32)

        await store.reload(contractId: contractId, walletIds: [walletA]) { _ in
            { bytes in
                guard bytes == expected else { throw Refused() }
                return answer
            }
        }

        XCTAssertNil(store.errorMessage)
        XCTAssertEqual(store.moderators.map(\.walletId), [walletA])
    }

    /// A base58 id that is not 32 bytes is refused before any wallet is
    /// looked up, and is not reported as no linked wallet.
    func testShouldRefuseAContractIdThatIsNot32Bytes() async {
        let store = ContractModerationOfferStore()
        var lookups = 0

        await store.reload(contractId: "1", walletIds: []) { _ in
            lookups += 1
            return nil
        }

        XCTAssertEqual(lookups, 0)
        XCTAssertEqual(store.errorMessage, "The contract id must be a 32-byte base58 identifier.")
        XCTAssertFalse(store.noLinkedWallet)
        XCTAssertFalse(store.nobodyModerates)
        XCTAssertEqual(store.askedWalletCount, 0)
        XCTAssertTrue(store.loadedOnce)
        XCTAssertFalse(store.isLoading)

        // With a wallet id listed, still no wallet is looked up
        await store.reload(contractId: "1", walletIds: [walletA]) { _ in
            lookups += 1
            return nil
        }
        XCTAssertEqual(lookups, 0)
        XCTAssertEqual(store.unloadedWalletCount, 0)
    }

    /// Clearing drops the counts with the answer, so a later screen does
    /// not show a stale note.
    func testShouldDropTheCountsOnClear() async {
        let store = ContractModerationOfferStore()
        await store.reload(contractId: contractId, walletIds: [walletA], askWallet: lookup([:]))
        XCTAssertEqual(store.unloadedWalletCount, 1)

        store.clear()

        XCTAssertEqual(store.unloadedWalletCount, 0)
        XCTAssertEqual(store.askedWalletCount, 0)
        XCTAssertFalse(store.noLinkedWallet)
        XCTAssertFalse(store.loadedOnce)
    }
}
