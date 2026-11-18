import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

/// Source selection on the send screen: the automatic default follows the
/// offered sources as they change (a balance that gates a source can
/// arrive after the recipient is typed), an explicit choice is never
/// replaced by another source, nothing is sendable from a source that is
/// not offered, and Send runs only the route that was tapped. Pure
/// view-model tests; address parsing is offline.
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

    /// The privacy rule: an explicit Shielded choice for a Core recipient
    /// must never become a Core (transparent) payment. When it stops being
    /// offered it stays selected, Send is disabled, and the screen says so.
    func testShouldKeepAnExplicitChoiceThatIsNoLongerOfferedAndDisableSend() {
        let vm = makeViewModel()
        vm.reconcileSource(available: [.core, .shielded])
        vm.selectSource(.shielded)
        XCTAssertTrue(vm.canSend)
        vm.reconcileSource(available: [.core])
        XCTAssertEqual(vm.selectedSource, .shielded)
        XCTAssertTrue(vm.sourceChosenByUser)
        XCTAssertNil(vm.detectedFlow)
        XCTAssertFalse(vm.canSend)
        XCTAssertTrue(vm.chosenSourceUnavailable)
        XCTAssertNil(vm.flowToSend(tappedFlow: .shieldedToCore))
        XCTAssertNil(vm.flowToSend(tappedFlow: .coreToCore))
    }

    func testShouldResumeAnExplicitChoiceWhenItIsOfferedAgain() {
        let vm = makeViewModel()
        vm.reconcileSource(available: [.core, .shielded])
        vm.selectSource(.shielded)
        vm.reconcileSource(available: [.core])
        vm.reconcileSource(available: [.core, .shielded])
        XCTAssertEqual(vm.selectedSource, .shielded)
        XCTAssertEqual(vm.detectedFlow, .shieldedToCore)
        XCTAssertFalse(vm.chosenSourceUnavailable)
        XCTAssertTrue(vm.canSend)
    }

    func testShouldLetTheUserPickAnotherSourceAfterTheChoiceBecameUnavailable() {
        let vm = makeViewModel()
        vm.reconcileSource(available: [.core, .shielded])
        vm.selectSource(.shielded)
        vm.reconcileSource(available: [.core])
        vm.selectSource(.core)
        XCTAssertEqual(vm.selectedSource, .core)
        XCTAssertEqual(vm.detectedFlow, .coreToCore)
        XCTAssertFalse(vm.chosenSourceUnavailable)
        XCTAssertTrue(vm.canSend)
    }

    /// The asynchronous Send handler sends only the route that was shown
    /// when the user tapped Send.
    func testShouldSendOnlyTheRouteThatWasTapped() {
        let vm = makeViewModel()
        vm.reconcileSource(available: [.shielded, .core])
        XCTAssertEqual(vm.detectedFlow, .shieldedToCore)
        XCTAssertEqual(vm.flowToSend(tappedFlow: .shieldedToCore), .shieldedToCore)
        XCTAssertNil(vm.flowToSend(tappedFlow: nil))
        // The automatic default moves after the tap: the tapped route is
        // no longer the selected one, so nothing is sent.
        vm.reconcileSource(available: [.core])
        XCTAssertEqual(vm.detectedFlow, .coreToCore)
        XCTAssertNil(vm.flowToSend(tappedFlow: .shieldedToCore))
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

    private func orchardAddress(_ fill: UInt8) -> String {
        DashAddress.encodeOrchard(rawBytes: Data(repeating: fill, count: 43), network: .testnet)!
    }

    private func platformAddress(_ fill: UInt8) -> String {
        DashAddress.encodePlatform(rawBytes: Data([0xb0]) + Data(repeating: fill, count: 20), network: .testnet)!
    }

    /// The reviewer's sequence: an Orchard recipient, an explicit Core
    /// choice, a different Orchard recipient (offered list unchanged), and
    /// later a balance that changes the offered list. The choice is still
    /// offered throughout, so it stays.
    func testShouldKeepAnExplicitChoiceWhenTheRecipientChangesWithinItsKind() {
        let vm = SendViewModel(network: .testnet)
        vm.amountString = "0.001"
        vm.recipientAddress = orchardAddress(0x01)
        XCTAssertEqual(vm.detectedAddressType.kind, .orchard)
        vm.reconcileSource(available: [.platform, .core])
        vm.selectSource(.core)
        XCTAssertEqual(vm.detectedFlow, .coreToShielded)

        vm.recipientAddress = orchardAddress(0x02)
        XCTAssertTrue(vm.sourceChosenByUser)

        // The shielded balance arrives and its source now leads the list.
        vm.reconcileSource(available: [.shielded, .platform, .core])
        XCTAssertEqual(vm.selectedSource, .core)
        XCTAssertEqual(vm.detectedFlow, .coreToShielded)
    }

    /// Editing the address by hand passes through empty and unparsable
    /// text, which offers nothing; that is not a new kind of recipient.
    func testShouldKeepAnExplicitChoiceWhileTheRecipientIsBeingEdited() {
        let vm = SendViewModel(network: .testnet)
        vm.recipientAddress = orchardAddress(0x01)
        vm.reconcileSource(available: [.platform, .core])
        vm.selectSource(.core)

        vm.recipientAddress = ""
        vm.reconcileSource(available: [])
        vm.recipientAddress = "tdash1"
        vm.reconcileSource(available: [])
        vm.recipientAddress = orchardAddress(0x02)
        vm.reconcileSource(available: [.shielded, .platform, .core])

        XCTAssertTrue(vm.sourceChosenByUser)
        XCTAssertEqual(vm.selectedSource, .core)
    }

    /// A recipient of another kind offers its own default, even when the
    /// chosen source is offered there too.
    func testShouldDropAnExplicitChoiceWhenTheRecipientKindChanges() {
        let vm = makeViewModel()
        vm.reconcileSource(available: [.core, .shielded])
        vm.selectSource(.shielded)
        vm.recipientAddress = platformAddress(0x03)
        XCTAssertEqual(vm.detectedAddressType.kind, .platform)
        XCTAssertFalse(vm.sourceChosenByUser)
        vm.reconcileSource(available: [.platform, .shielded])
        XCTAssertEqual(vm.selectedSource, .platform)
    }

    func testShouldKeepAnExplicitChoiceWhenTheRecipientChangesToAnotherCoreAddress() {
        let vm = makeViewModel()
        vm.reconcileSource(available: [.core, .shielded])
        vm.selectSource(.shielded)
        vm.recipientAddress = otherCoreAddress
        XCTAssertTrue(vm.sourceChosenByUser)
        vm.reconcileSource(available: [.core, .shielded])
        XCTAssertEqual(vm.selectedSource, .shielded)
    }
}
