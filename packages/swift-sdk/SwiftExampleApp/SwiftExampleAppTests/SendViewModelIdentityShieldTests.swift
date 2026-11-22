import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

/// Pure view-model tests for the identity to shielded (Type 21) send path:
/// source availability, flow detection, the fee display and the `canSend`
/// gate. No network and no FFI send; the fee floor and the identity balance
/// are pushed in exactly as `SendTransactionView` pushes them.
@MainActor
final class SendViewModelIdentityShieldTests: XCTestCase {

    private let network: Network = .testnet
    private let coreAddress = "yX6nLg4fVSdkecziwJmkeLrVLqDUivHmnh"
    private let identityId = Data(repeating: 0x11, count: 32)
    private let feeFloor: UInt64 = 5_000

    /// Any 43 raw bytes encode to a parseable testnet Orchard address.
    private var orchardAddress: String {
        DashAddress.encodeOrchard(rawBytes: Data(repeating: 0x42, count: 43), network: network)!
    }

    private func makeViewModel(recipient: String) -> SendViewModel {
        let vm = SendViewModel(network: network)
        vm.recipientAddress = recipient
        return vm
    }

    private func balances(
        core: UInt64 = 0,
        shielded: UInt64 = 0,
        platformAddresses: UInt64 = 0,
        largestIdentity: UInt64 = 0
    ) -> SendSourceBalances {
        SendSourceBalances(
            core: core,
            shielded: shielded,
            platformAddresses: platformAddresses,
            largestIdentity: largestIdentity
        )
    }

    /// Offer the sources the screen would offer for `balances`, as the view
    /// does on every change, then pick `source` as the user would.
    private func offerAndPick(
        _ source: FundSource,
        on vm: SendViewModel,
        balances: SendSourceBalances
    ) {
        vm.reconcileSource(available: vm.availableSources(balances))
        vm.selectSource(source)
    }

    /// A view model on the identity shield flow with the floor resolved and
    /// an identity of `balance` credits selected.
    private func makeIdentityShieldViewModel(
        amount: String,
        balance: UInt64
    ) -> SendViewModel {
        let vm = makeViewModel(recipient: orchardAddress)
        vm.shieldedFeeEstimates = [.shieldFromIdentity: feeFloor]
        vm.selectedIdentityId = identityId
        vm.selectedIdentityBalance = balance
        vm.amountString = amount
        offerAndPick(.identity, on: vm, balances: balances(largestIdentity: balance))
        return vm
    }

    // MARK: - Fixtures

    func test_orchardFixture_parsesAsOrchard() {
        if case .orchard = DashAddress.parse(orchardAddress, network: network).type {} else {
            XCTFail("fixture should be a testnet Orchard address")
        }
    }

    // MARK: - Source availability

    func test_orchardRecipient_offersIdentitySourceLast() {
        let vm = makeViewModel(recipient: orchardAddress)
        let sources = vm.availableSources(balances(
            core: 1, shielded: 1, platformAddresses: 1, largestIdentity: 1
        ))
        XCTAssertEqual(sources, [.shielded, .platform, .core, .identity])
        // Shielded to shielded stays the auto-selected default.
        vm.reconcileSource(available: sources)
        XCTAssertEqual(vm.selectedSource, .shielded)
    }

    func test_orchardRecipient_withOnlyIdentityFunds_offersIdentityOnly() {
        let vm = makeViewModel(recipient: orchardAddress)
        let sources = vm.availableSources(balances(largestIdentity: 1))
        XCTAssertEqual(sources, [.identity])
        // With nothing else offered it is the default.
        vm.reconcileSource(available: sources)
        XCTAssertEqual(vm.selectedSource, .identity)
        XCTAssertEqual(vm.detectedFlow, .identityToShielded)
    }

    func test_orchardRecipient_withoutFundedIdentity_hidesIdentitySource() {
        let vm = makeViewModel(recipient: orchardAddress)
        let sources = vm.availableSources(balances(
            core: 1, shielded: 1, platformAddresses: 1, largestIdentity: 0
        ))
        XCTAssertEqual(sources, [.shielded, .platform, .core])
    }

    func test_coreRecipient_neverOffersIdentitySource() {
        let vm = makeViewModel(recipient: coreAddress)
        let sources = vm.availableSources(balances(
            core: 1, shielded: 1, platformAddresses: 1, largestIdentity: 1
        ))
        XCTAssertFalse(sources.contains(.identity))
    }

    func test_unknownShieldedBalancePassedAsZero_doesNotOfferShielded() {
        // The screen passes a not-yet-known pushed balance as 0.
        let vm = makeViewModel(recipient: orchardAddress)
        XCTAssertEqual(vm.availableSources(balances(largestIdentity: 1)), [.identity])
    }

    func test_identityShield_doesNotSpendShieldedNotes() {
        XCTAssertFalse(SendFlow.identityToShielded.spendsShieldedNotes)
        // So an unknown or zero account-0 balance never blocks its dispatch.
        XCTAssertTrue(SendViewModel.mayDispatch(.identityToShielded, accountZeroShieldedBalance: nil))
        XCTAssertTrue(SendViewModel.mayDispatch(.identityToShielded, accountZeroShieldedBalance: 0))
    }

    func test_pickedIdentitySource_isKeptWhenItStopsBeingOffered() {
        let vm = makeIdentityShieldViewModel(amount: "0.001", balance: 1_000_000_000)
        XCTAssertTrue(vm.canSend)
        // The identity balance goes away: the choice stays, Send is disabled.
        vm.reconcileSource(available: vm.availableSources(balances(shielded: 1)))
        XCTAssertEqual(vm.selectedSource, .identity)
        XCTAssertTrue(vm.chosenSourceUnavailable)
        XCTAssertFalse(vm.canSend)
    }

    func test_tappedIdentityShield_isTheFlowToSend() {
        let vm = makeIdentityShieldViewModel(amount: "0.001", balance: 1_000_000_000)
        XCTAssertEqual(
            vm.flowToSend(tappedFlow: .identityToShielded, tappedRecipient: orchardAddress),
            .identityToShielded
        )
    }

    // MARK: - Flow detection

    func test_orchardRecipientWithIdentitySource_detectsIdentityToShielded() {
        let vm = makeViewModel(recipient: orchardAddress)
        offerAndPick(.identity, on: vm, balances: balances(shielded: 1, largestIdentity: 1))
        XCTAssertEqual(vm.detectedFlow, .identityToShielded)
        XCTAssertEqual(vm.detectedFlow?.displayName, "Shield from Identity")
    }

    func test_coreRecipient_cannotPickTheIdentitySource() {
        let vm = makeViewModel(recipient: coreAddress)
        offerAndPick(.identity, on: vm, balances: balances(core: 1, largestIdentity: 1))
        // Not offered for a Core recipient, so the pick is ignored.
        XCTAssertNotEqual(vm.selectedSource, .identity)
        XCTAssertNotEqual(vm.detectedFlow, .identityToShielded)
    }

    // MARK: - Fee display

    func test_unresolvedFeeFloor_showsNoEstimatedFee() {
        let vm = makeViewModel(recipient: orchardAddress)
        offerAndPick(.identity, on: vm, balances: balances(largestIdentity: 1))
        XCTAssertEqual(vm.detectedFlow, .identityToShielded)
        XCTAssertNil(vm.estimatedFee)
    }

    func test_resolvedFeeFloor_isTheEstimatedFee() {
        let vm = makeIdentityShieldViewModel(amount: "0.001", balance: 1_000_000_000)
        XCTAssertEqual(vm.estimatedFee, feeFloor)
    }

    // MARK: - Send gate

    func test_canSend_withAffordableAmount_isTrue() {
        let vm = makeIdentityShieldViewModel(amount: "0.001", balance: 1_000_000_000)
        XCTAssertTrue(vm.canSend)
    }

    func test_canSend_withoutSelectedIdentity_isFalse() {
        let vm = makeIdentityShieldViewModel(amount: "0.001", balance: 1_000_000_000)
        vm.selectedIdentityId = nil
        XCTAssertFalse(vm.canSend)
    }

    func test_canSend_withUnresolvedFeeFloor_isFalse() {
        let vm = makeIdentityShieldViewModel(amount: "0.001", balance: 1_000_000_000)
        vm.shieldedFeeEstimates = [:]
        XCTAssertFalse(vm.canSend)
    }

    func test_canSend_withOtherKindsResolvedButNotTheFloor_isFalse() {
        let vm = makeIdentityShieldViewModel(amount: "0.001", balance: 1_000_000_000)
        vm.shieldedFeeEstimates = [.transfer: 1, .unshield: 1, .withdrawal: 1]
        XCTAssertFalse(vm.canSend)
    }

    func test_canSend_withZeroAmount_isFalse() {
        let vm = makeIdentityShieldViewModel(amount: "0", balance: 1_000_000_000)
        XCTAssertFalse(vm.canSend)
    }

    func test_canSend_withAmountPlusFeeAboveBalance_isFalse() {
        // 0.001 DASH = 100_000_000 credits; one credit short of amount + fee.
        let vm = makeIdentityShieldViewModel(amount: "0.001", balance: 100_000_000 + feeFloor - 1)
        XCTAssertFalse(vm.canSend)
    }

    func test_canSend_withAmountAloneAboveBalanceMinusFee_isFalse() {
        // The balance covers the amount but not the fee on top of it.
        let vm = makeIdentityShieldViewModel(amount: "0.001", balance: 100_000_000)
        XCTAssertFalse(vm.canSend)
    }

    func test_canSend_withAmountPlusFeeExactlyAtBalance_isTrue() {
        let vm = makeIdentityShieldViewModel(amount: "0.001", balance: 100_000_000 + feeFloor)
        XCTAssertTrue(vm.canSend)
    }

    func test_canSend_whenAmountPlusFeeOverflows_isFalseWithoutTrapping() {
        let vm = makeIdentityShieldViewModel(amount: "1", balance: UInt64.max)
        vm.shieldedFeeEstimates = [.shieldFromIdentity: UInt64.max]
        XCTAssertFalse(vm.canSend)
    }

    func test_canSend_whileSending_isFalse() {
        let vm = makeIdentityShieldViewModel(amount: "0.001", balance: 1_000_000_000)
        vm.isSending = true
        XCTAssertFalse(vm.canSend)
    }

    func test_identityGate_doesNotAffectOtherShieldedFlows() {
        // Platform to shielded keeps its own `amount > 0` gate.
        let vm = makeViewModel(recipient: orchardAddress)
        vm.amountString = "0.001"
        offerAndPick(.platform, on: vm, balances: balances(platformAddresses: 1, largestIdentity: 1))
        XCTAssertEqual(vm.detectedFlow, .platformToShielded)
        XCTAssertTrue(vm.canSend)
    }

    // MARK: - Platform source counts address funds only

    func test_identityCreditsWithoutAddressFunds_offerIdentityNotPlatform() {
        // Before the first address sync the screen's Platform balance row
        // falls back to identity credits; the sources must not.
        let vm = makeViewModel(recipient: orchardAddress)
        let sources = vm.availableSources(balances(largestIdentity: 1_000))
        XCTAssertEqual(sources, [.identity])
    }

    func test_addressFunds_offerPlatform() {
        let vm = makeViewModel(recipient: orchardAddress)
        let sources = vm.availableSources(balances(platformAddresses: 1_000))
        XCTAssertEqual(sources, [.platform])
    }

    // MARK: - Error routing

    func test_unconfirmedErrors_mayHaveGoneThrough() {
        XCTAssertTrue(SendViewModel.mayHaveGoneThrough(.shieldedSpendUnconfirmed("x")))
        XCTAssertTrue(SendViewModel.mayHaveGoneThrough(.transactionBroadcastUnconfirmed("x")))
        // A pending earlier debit means nothing was sent: not "may have gone through".
        XCTAssertFalse(SendViewModel.mayHaveGoneThrough(.shieldedIdentityDebitPending("x")))
    }

    // MARK: - Ok path of the identity shield

    func test_identityShieldOkPath_isSubmittedNotSuccess() {
        let vm = makeIdentityShieldViewModel(amount: "0.001", balance: 1_000_000_000)
        vm.successMessage = "stale"
        vm.unconfirmedMessage = "stale"
        vm.error = "stale"
        vm.applyIdentityShieldSubmitted()
        XCTAssertEqual(vm.submittedMessage, SendViewModel.identityShieldSubmittedMessage)
        XCTAssertNil(vm.successMessage)
        XCTAssertNil(vm.unconfirmedMessage)
        XCTAssertNil(vm.error)
    }

    func test_identityShieldSubmittedMessage_saysTheRowStaysPending() {
        XCTAssertTrue(SendViewModel.identityShieldSubmittedMessage.contains("pending"))
    }

    func test_failureAfterASubmittedOutcome_clearsIt() {
        let vm = makeIdentityShieldViewModel(amount: "0.001", balance: 1_000_000_000)
        vm.applyIdentityShieldSubmitted()
        vm.applySendFailure(PlatformWalletError.walletOperation("x"), flow: .identityToShielded)
        XCTAssertNil(vm.submittedMessage)
        XCTAssertNotNil(vm.error)
    }

    func test_unconfirmedIdentityShield_setsUnconfirmedNotSuccess() {
        let vm = makeIdentityShieldViewModel(amount: "0.001", balance: 1_000_000_000)
        vm.applySendFailure(
            PlatformWalletError.shieldedSpendUnconfirmed("x"),
            flow: .identityToShielded
        )
        XCTAssertEqual(vm.unconfirmedMessage, SendViewModel.identityShieldMayHaveGoneThroughMessage)
        XCTAssertNil(vm.successMessage)
        XCTAssertNil(vm.error)
    }

    func test_unconfirmedOtherShieldedSend_setsUnconfirmedNotSuccess() {
        let vm = makeViewModel(recipient: orchardAddress)
        vm.applySendFailure(
            PlatformWalletError.transactionBroadcastUnconfirmed("x"),
            flow: .shieldedToShielded
        )
        XCTAssertTrue(vm.unconfirmedMessage?.contains("Do not retry") == true)
        XCTAssertNil(vm.successMessage)
        XCTAssertNil(vm.error)
    }

    func test_identityDebitPending_isAnErrorNotUnconfirmed() {
        let vm = makeIdentityShieldViewModel(amount: "0.001", balance: 1_000_000_000)
        vm.applySendFailure(
            PlatformWalletError.shieldedIdentityDebitPending("x"),
            flow: .identityToShielded
        )
        XCTAssertEqual(vm.error, SendViewModel.identityDebitPendingMessage)
        XCTAssertNil(vm.unconfirmedMessage)
    }

    // MARK: - Which identity-shield outcomes may have saved a debit

    func test_identityShieldReturnedNormally_mayHaveSavedADebit() {
        // The submitted outcome: the wallet saved the record before broadcasting.
        XCTAssertTrue(SendViewModel.identityShieldMayHaveSavedDebit(after: nil))
    }

    func test_identityShieldUnconfirmed_mayHaveSavedADebit() {
        XCTAssertTrue(SendViewModel.identityShieldMayHaveSavedDebit(
            after: PlatformWalletError.shieldedSpendUnconfirmed("x")))
        XCTAssertTrue(SendViewModel.identityShieldMayHaveSavedDebit(
            after: PlatformWalletError.transactionBroadcastUnconfirmed("x")))
    }

    func test_identityShieldOtherErrors_mayHaveSavedADebit() {
        XCTAssertTrue(SendViewModel.identityShieldMayHaveSavedDebit(
            after: PlatformWalletError.walletOperation("x")))
        XCTAssertTrue(SendViewModel.identityShieldMayHaveSavedDebit(
            after: PlatformWalletError.signingKeyUnavailable("x")))
        XCTAssertTrue(SendViewModel.identityShieldMayHaveSavedDebit(
            after: NSError(domain: "test", code: 1)))
    }

    func test_identityShieldPreSaveRefusals_saveNoDebit() {
        let refusals: [PlatformWalletError] = [
            .shieldedIdentityDebitPending("x"),
            .insufficientIdentityCredits(identityId: "id", required: 2, available: 1),
            .identityBalanceUnavailable("x"),
            .protocolVersionNotActive("x"),
            .sdkPinnedBelowProtocolVersion("x"),
            .invalidParameter("x"),
        ]
        for refusal in refusals {
            XCTAssertFalse(
                SendViewModel.identityShieldMayHaveSavedDebit(after: refusal),
                "\(refusal)"
            )
        }
    }

    func test_identityDebitFlag_startsUnset() {
        let vm = makeIdentityShieldViewModel(amount: "0.001", balance: 1_000_000_000)
        XCTAssertFalse(vm.identityDebitMayHaveBeenSaved)
    }

    func test_identityDebitPendingMessage_isWalletScoped() {
        // An unreadable record blocks every identity and the error names the
        // requested identity, so the message must not blame it.
        let message = SendViewModel.identityDebitPendingMessage
        XCTAssertFalse(message.contains("from this identity"))
        XCTAssertTrue(message.contains("an identity in this wallet"))
        XCTAssertTrue(message.contains("could not be read"))
        XCTAssertTrue(message.contains("In-Progress Identity Shields"))
        XCTAssertEqual(
            SendViewModel.identityShieldRefusalMessage(.shieldedIdentityDebitPending("x")),
            message
        )
    }

    func test_insufficientIdentityCredits_showsRequiredAndAvailableInDash() {
        let vm = makeIdentityShieldViewModel(amount: "0.001", balance: 1_000_000_000)
        vm.applySendFailure(
            PlatformWalletError.insufficientIdentityCredits(
                identityId: "id",
                required: 150_000_000_000,
                available: 50_000_000_000
            ),
            flow: .identityToShielded
        )
        XCTAssertTrue(vm.error?.contains(formatCreditsAsDash(150_000_000_000)) == true)
        XCTAssertTrue(vm.error?.contains(formatCreditsAsDash(50_000_000_000)) == true)
        XCTAssertTrue(vm.error?.contains("Nothing was sent") == true)
        XCTAssertNil(vm.unconfirmedMessage)
    }

    func test_insufficientIdentityCredits_oneCreditShortfall_showsDifferentAmounts() {
        let required: UInt64 = 100_000_000_001
        let available: UInt64 = 100_000_000_000
        let message = SendViewModel.identityShieldRefusalMessage(
            .insufficientIdentityCredits(identityId: "id", required: required, available: available)
        )
        XCTAssertNotEqual(formatCreditsAsDash(required), formatCreditsAsDash(available))
        XCTAssertTrue(message?.contains(formatCreditsAsDash(required)) == true)
        XCTAssertTrue(message?.contains(formatCreditsAsDash(available)) == true)
    }

    func test_insufficientIdentityCredits_oneCreditAvailable_isNotShownAsZero() {
        let message = SendViewModel.identityShieldRefusalMessage(
            .insufficientIdentityCredits(identityId: "id", required: 2, available: 1)
        )
        XCTAssertTrue(message?.contains("has \(formatCreditsAsDash(1)).") == true)
        XCTAssertFalse(message?.contains("has \(formatCreditsAsDash(0)).") == true)
    }

    func test_protocolVersionNotActive_isReadable() {
        let message = SendViewModel.identityShieldRefusalMessage(.protocolVersionNotActive("x"))
        XCTAssertEqual(
            message,
            "This network has not activated shielding from an identity yet. Nothing was sent."
        )
    }

    func test_sdkPinnedBelowProtocolVersion_isReadable() {
        let message = SendViewModel.identityShieldRefusalMessage(.sdkPinnedBelowProtocolVersion("x"))
        XCTAssertTrue(message?.contains("pinned below") == true)
        XCTAssertTrue(message?.contains("Nothing was sent") == true)
    }

    func test_identityBalanceUnavailable_saysNothingWasSent() {
        let vm = makeIdentityShieldViewModel(amount: "0.001", balance: 1_000_000_000)
        vm.applySendFailure(
            PlatformWalletError.identityBalanceUnavailable("x"),
            flow: .identityToShielded
        )
        XCTAssertTrue(vm.error?.contains("could not be read") == true)
        XCTAssertTrue(vm.error?.contains("Nothing was sent") == true)
    }

    func test_otherWalletError_showsItsOwnDescription() {
        let vm = makeIdentityShieldViewModel(amount: "0.001", balance: 1_000_000_000)
        let failure = PlatformWalletError.walletOperation("refused below protocol version 14")
        vm.applySendFailure(failure, flow: .identityToShielded)
        XCTAssertEqual(vm.error, failure.localizedDescription)
        XCTAssertNil(vm.unconfirmedMessage)
    }
}
