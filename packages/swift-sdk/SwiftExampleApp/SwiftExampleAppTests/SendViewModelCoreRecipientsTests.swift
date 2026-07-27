import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

/// Behavioral tests for `SendViewModel`'s multi-recipient Core batch —
/// the pure view-model logic behind the `coreToCore` send screen
/// (`coreRecipients`, `coreSendTotalDuffs`, `canSend`, and the add/remove
/// of extra output rows).
///
/// These are pure view-model tests: no network, no FFI send. Address
/// validity runs through `DashAddress.parse` → `Address.validate` (Rust
/// FFI), which is pure/offline and links fine in the test bundle (other
/// suites already exercise the SDK). Everything uses `Network.testnet`.
///
/// `SendViewModel` is `@MainActor`, so the suite is too.
@MainActor
final class SendViewModelCoreRecipientsTests: XCTestCase {

    // Two verified testnet P2PKH (base58check) Core addresses.
    private let primaryAddress = "yX6nLg4fVSdkecziwJmkeLrVLqDUivHmnh"
    private let extraAddress = "yW2nW3XsoYRfcJV6QgcAXvdWTUorkjgsNQ"
    private let network: Network = .testnet

    /// Build a view model already routed onto the `coreToCore` flow with
    /// `primaryAddress` as the primary recipient. Mirrors how the app
    /// drives it: set the recipient (which runs `detectAddressType`) and
    /// the source, then `updateFlow()`.
    private func makeCoreToCoreViewModel(
        primaryAmount: String
    ) -> SendViewModel {
        let vm = SendViewModel(network: network)
        vm.recipientAddress = primaryAddress
        vm.amountString = primaryAmount
        vm.selectedSource = .core
        vm.updateFlow()
        return vm
    }

    // MARK: - Sanity: the fixtures really are Core addresses on testnet

    func test_fixtureAddresses_areTestnetCore() {
        if case .core = DashAddress.parse(primaryAddress, network: network).type {} else {
            XCTFail("primary fixture should be a testnet Core address")
        }
        if case .core = DashAddress.parse(extraAddress, network: network).type {} else {
            XCTFail("extra fixture should be a testnet Core address")
        }
    }

    func test_makeViewModel_routesToCoreToCore() {
        // If the flow isn't `.coreToCore`, every gating assertion below is
        // meaningless (the multi-recipient list is only consulted there).
        let vm = makeCoreToCoreViewModel(primaryAmount: "0.001")
        XCTAssertEqual(vm.detectedFlow, .coreToCore)
    }

    // MARK: - 1. Primary-only (no extra rows)

    func test_primaryOnly_singleOutputAndTotal() {
        let vm = makeCoreToCoreViewModel(primaryAmount: "0.001")

        let recipients = vm.coreRecipients
        XCTAssertEqual(recipients?.count, 1)
        XCTAssertEqual(recipients?.first?.address, primaryAddress)
        XCTAssertEqual(recipients?.first?.amountDuffs, 100_000) // 0.001 * 1e8

        // Total equals the single output; Send is enabled.
        XCTAssertEqual(vm.coreSendTotalDuffs, 100_000)
        XCTAssertTrue(vm.canSend)
    }

    // MARK: - 2. Primary + extras ordering

    func test_primaryPlusExtras_orderedOutputsAndSummedTotal() {
        let vm = makeCoreToCoreViewModel(primaryAmount: "0.001") // 100_000

        vm.addCoreRecipient()
        vm.additionalCoreRecipients[0].address = extraAddress
        vm.additionalCoreRecipients[0].amountString = "0.002" // 200_000

        vm.addCoreRecipient()
        vm.additionalCoreRecipients[1].address = primaryAddress
        vm.additionalCoreRecipients[1].amountString = "0.003" // 300_000

        let recipients = vm.coreRecipients
        XCTAssertEqual(recipients?.count, 3)
        // Display order: primary first, then extras in row order.
        XCTAssertEqual(recipients?[0].address, primaryAddress)
        XCTAssertEqual(recipients?[0].amountDuffs, 100_000)
        XCTAssertEqual(recipients?[1].address, extraAddress)
        XCTAssertEqual(recipients?[1].amountDuffs, 200_000)
        XCTAssertEqual(recipients?[2].address, primaryAddress)
        XCTAssertEqual(recipients?[2].amountDuffs, 300_000)

        XCTAssertEqual(vm.coreSendTotalDuffs, 600_000)
        XCTAssertTrue(vm.canSend)
    }

    // MARK: - 3. Invalid extra address → whole batch invalid

    func test_invalidExtraAddress_invalidatesBatch() {
        let vm = makeCoreToCoreViewModel(primaryAmount: "0.001")

        vm.addCoreRecipient()
        // Not a Core address (garbage / wrong type).
        vm.additionalCoreRecipients[0].address = "not-a-real-address"
        vm.additionalCoreRecipients[0].amountString = "0.002"

        XCTAssertNil(vm.coreRecipients)
        XCTAssertFalse(vm.canSend)
        // A nil batch shows no total, matching the disabled-Send state.
        XCTAssertEqual(vm.coreSendTotalDuffs, 0)
    }

    // MARK: - 4. Sub-unit / zero amount in a row → batch invalid

    func test_subUnitExtraAmount_invalidatesBatch() {
        let vm = makeCoreToCoreViewModel(primaryAmount: "0.001")

        vm.addCoreRecipient()
        vm.additionalCoreRecipients[0].address = extraAddress
        // 0.000000001 DASH * 1e8 = 0.1 → truncates to 0 duffs.
        vm.additionalCoreRecipients[0].amountString = "0.000000001"

        XCTAssertNil(vm.coreRecipients)
        XCTAssertFalse(vm.canSend)
        XCTAssertEqual(vm.coreSendTotalDuffs, 0)
    }

    func test_emptyExtraAmount_invalidatesBatch() {
        let vm = makeCoreToCoreViewModel(primaryAmount: "0.001")

        vm.addCoreRecipient()
        vm.additionalCoreRecipients[0].address = extraAddress
        vm.additionalCoreRecipients[0].amountString = "" // unparseable → nil duffs

        XCTAssertNil(vm.coreRecipients)
        XCTAssertFalse(vm.canSend)
        XCTAssertEqual(vm.coreSendTotalDuffs, 0)
    }

    // MARK: - 5. Aggregate overflow → batch invalid (never traps)

    func test_aggregateOverflow_invalidatesBatchWithoutTrapping() {
        // Each row scales to 1e11 DASH * 1e8 = 1e19 duffs, which is below
        // UInt64.max (~1.8446744e19) so each parses to a valid UInt64 on
        // its own. Two of them sum to 2e19, which overflows UInt64 — the
        // batch must be rejected (nil), and the total must be 0 rather
        // than trapping/wrapping when the summary renders.
        let huge = "100000000000" // 1e11 DASH

        // Pre-flight: a single huge row is individually valid (it does NOT
        // already overflow on its own), so the rejection below is genuinely
        // the *aggregate* overflow path, not a per-row range failure.
        let single = makeCoreToCoreViewModel(primaryAmount: huge)
        XCTAssertEqual(single.coreRecipients?.count, 1)
        XCTAssertEqual(single.coreSendTotalDuffs, 10_000_000_000_000_000_000) // 1e19

        let vm = makeCoreToCoreViewModel(primaryAmount: huge)
        vm.addCoreRecipient()
        vm.additionalCoreRecipients[0].address = extraAddress
        vm.additionalCoreRecipients[0].amountString = huge

        XCTAssertNil(vm.coreRecipients)
        XCTAssertEqual(vm.coreSendTotalDuffs, 0)
        XCTAssertFalse(vm.canSend)
    }

    // MARK: - 6. Removal returns to the primary-only list

    func test_removeExtra_returnsToPrimaryOnly() {
        let vm = makeCoreToCoreViewModel(primaryAmount: "0.001")

        vm.addCoreRecipient()
        vm.additionalCoreRecipients[0].address = extraAddress
        vm.additionalCoreRecipients[0].amountString = "0.002"
        XCTAssertEqual(vm.coreRecipients?.count, 2)
        XCTAssertEqual(vm.coreSendTotalDuffs, 300_000)

        let id = vm.additionalCoreRecipients[0].id
        vm.removeCoreRecipient(id)

        XCTAssertTrue(vm.additionalCoreRecipients.isEmpty)
        XCTAssertEqual(vm.coreRecipients?.count, 1)
        XCTAssertEqual(vm.coreRecipients?.first?.address, primaryAddress)
        XCTAssertEqual(vm.coreSendTotalDuffs, 100_000)
        XCTAssertTrue(vm.canSend)
    }

    // MARK: - Authoritative Core broadcast outcome

    func test_acceptedBroadcastOutcome_showsSuccess() {
        let vm = makeCoreToCoreViewModel(primaryAmount: "0.001")

        vm.applyCoreBroadcastOutcome(.accepted(txid: "abc"), recipientCount: 1)

        XCTAssertEqual(vm.successMessage, "Payment sent")
        XCTAssertNil(vm.error)
    }

    func test_rejectedBroadcastOutcome_doesNotShowSuccess() {
        let vm = makeCoreToCoreViewModel(primaryAmount: "0.001")

        vm.applyCoreBroadcastOutcome(
            .rejected(txid: "abc", reason: "mempool policy"),
            recipientCount: 1
        )

        XCTAssertNil(vm.successMessage)
        XCTAssertEqual(vm.error, "Payment rejected by Dash Core: mempool policy")
    }

    func test_unknownBroadcastOutcome_warnsAgainstRetry() {
        let vm = makeCoreToCoreViewModel(primaryAmount: "0.001")

        vm.applyCoreBroadcastOutcome(
            .unknown(txid: "abc", reason: "request timed out"),
            recipientCount: 1
        )

        XCTAssertNil(vm.successMessage)
        XCTAssertTrue(vm.error?.contains("do not retry") == true)
        XCTAssertTrue(vm.error?.contains("abc") == true)
    }
}
