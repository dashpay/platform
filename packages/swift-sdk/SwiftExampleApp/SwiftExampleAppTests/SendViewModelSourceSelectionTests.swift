import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

/// Source selection on the send screen: the automatic default follows the
/// offered sources as they change (a balance that gates a source can
/// arrive after the recipient is typed), an explicit choice that is still
/// offered stays, and nothing is sendable from a source that is not
/// offered. Pure view-model tests; address parsing is offline.
@MainActor
final class SendViewModelSourceSelectionTests: XCTestCase {

    private let coreAddress = "yX6nLg4fVSdkecziwJmkeLrVLqDUivHmnh"
    private let otherCoreAddress = "yW2nW3XsoYRfcJV6QgcAXvdWTUorkjgsNQ"

    private func makeViewModel() -> SendViewModel {
        let vm = SendViewModel(network: .testnet)
        vm.recipientAddress = coreAddress
        vm.amountString = "0.001"
        return vm
    }

    func testShouldMoveTheAutomaticSelectionWhenALaterSourceBecomesTheDefault() {
        let vm = makeViewModel()
        vm.reconcileSource(available: [.core])
        XCTAssertEqual(vm.selectedSource, .core)
        // The shielded balance arrives and its source now leads the list.
        vm.reconcileSource(available: [.shielded, .core])
        XCTAssertEqual(vm.selectedSource, .shielded)
        XCTAssertEqual(vm.detectedFlow, .shieldedToCore)
    }

    func testShouldKeepAnExplicitChoiceThatIsStillOffered() {
        let vm = makeViewModel()
        vm.reconcileSource(available: [.core, .shielded])
        vm.selectSource(.shielded)
        vm.reconcileSource(available: [.core, .shielded, .platform])
        XCTAssertEqual(vm.selectedSource, .shielded)
        XCTAssertTrue(vm.sourceChosenByUser)
    }

    func testShouldFallBackToTheDefaultWhenTheChosenSourceIsNoLongerOffered() {
        let vm = makeViewModel()
        vm.reconcileSource(available: [.core, .shielded])
        vm.selectSource(.shielded)
        vm.reconcileSource(available: [.core])
        XCTAssertEqual(vm.selectedSource, .core)
        XCTAssertFalse(vm.sourceChosenByUser)
        XCTAssertEqual(vm.detectedFlow, .coreToCore)
        XCTAssertTrue(vm.canSend)
    }

    func testShouldNotAllowSendingFromASourceThatIsNotOffered() {
        let vm = makeViewModel()
        vm.reconcileSource(available: [.core])
        XCTAssertTrue(vm.canSend)
        vm.reconcileSource(available: [])
        XCTAssertEqual(vm.selectedSource, .core)
        XCTAssertFalse(vm.canSend)
    }

    func testShouldIgnoreAChoiceOfASourceThatIsNotOffered() {
        let vm = makeViewModel()
        vm.reconcileSource(available: [.core])
        vm.selectSource(.shielded)
        XCTAssertEqual(vm.selectedSource, .core)
        XCTAssertFalse(vm.sourceChosenByUser)
    }

    func testShouldDropAnExplicitChoiceWhenTheRecipientChanges() {
        let vm = makeViewModel()
        vm.reconcileSource(available: [.core, .shielded])
        vm.selectSource(.shielded)
        vm.recipientAddress = otherCoreAddress
        XCTAssertFalse(vm.sourceChosenByUser)
        vm.reconcileSource(available: [.core, .shielded])
        XCTAssertEqual(vm.selectedSource, .core)
    }
}
