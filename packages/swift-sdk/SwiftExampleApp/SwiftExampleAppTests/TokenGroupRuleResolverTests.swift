import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

final class TokenGroupRuleResolverTests: XCTestCase {
    private func makeToken() -> PersistentToken {
        PersistentToken(
            contractId: Data(repeating: 0x11, count: 32),
            position: 0,
            name: "Test Token",
            baseSupply: "1000"
        )
    }

    func testMaxSupplyRuleContributesExplicitGroupPosition() {
        let token = makeToken()
        token.maxSupplyChangeRules = ChangeControlRules(
            authorizedToMakeChange: AuthorizedActionTakers.group(7)
        )

        XCTAssertEqual(TokenGroupRuleResolver.relevantGroupPositions(for: token), [7])
    }

    func testMaxSupplyMainGroupRuleUsesMainControlGroupPosition() {
        let token = makeToken()
        token.mainControlGroupPosition = 12
        token.maxSupplyChangeRules = ChangeControlRules(
            authorizedToMakeChange: AuthorizedActionTakers.mainGroup.rawValue
        )

        XCTAssertEqual(TokenGroupRuleResolver.relevantGroupPositions(for: token), [12])
    }

    func testMaxSupplyRuleDedupesPositionSharedWithAnotherAction() {
        let token = makeToken()
        let sharedRule = ChangeControlRules(
            authorizedToMakeChange: AuthorizedActionTakers.group(4)
        )
        token.maxSupplyChangeRules = sharedRule
        token.manualMintingRules = sharedRule

        XCTAssertEqual(TokenGroupRuleResolver.relevantGroupPositions(for: token), [4])
    }
}
