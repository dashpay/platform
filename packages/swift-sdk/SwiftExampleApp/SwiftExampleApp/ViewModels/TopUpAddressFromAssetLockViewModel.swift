import Foundation
import SwiftDashSDK
import SwiftUI

/// ViewModel for the Top Up Address From Asset Lock form.
@MainActor
final class TopUpAddressFromAssetLockViewModel: BaseViewModel {

  @Published var proofType: Addresses.AssetLockProofType = .instant
  @Published var outputAddressHex = ""
  @Published var outputAmount = ""
  @Published var assetLockPrivateKeyHex = ""
  @Published var instantLockHex = ""
  @Published var transactionHex = ""
  @Published var outputIndex = "0"
  @Published var coreChainLockedHeight = ""
  @Published var outPointHex = ""
  @Published var result: PlatformAddressInfosResult?
  /// The last top-up's failure. While it holds its slot (the asset lock
  /// may already be consumed, or no class) no new top-up runs from this
  /// form; `reset()` keeps it. It clears on a success, a later `.notSent`
  /// failure, or `releasePaymentHold()`.
  @Published var paymentFailure: PaymentFailure?

  /// Whether the form may start a top-up (`PaymentFailure.permitsFreshPayment`).
  var permitsTopUp: Bool { PaymentFailure.permitsFreshPayment(after: paymentFailure) }

  private var validationResult: ValidationResult {
    let proof: AssetLockProofTypeForValidation = proofType == .instant ? .instant : .chain
    return TopUpAddressFromAssetLockValidator.validate(
      outputAddressHex: outputAddressHex,
      assetLockPrivateKeyHex: assetLockPrivateKeyHex,
      proofType: proof,
      instantLockHex: instantLockHex,
      transactionHex: transactionHex,
      outputIndex: outputIndex,
      coreChainLockedHeight: coreChainLockedHeight,
      outPointHex: outPointHex
    )
  }

  var isFormValid: Bool { validationResult.isValid }
  var validationErrors: [String] { validationResult.errors }

  override func reset() {
    super.reset()
    outputAddressHex = ""
    outputAmount = ""
    assetLockPrivateKeyHex = ""
    instantLockHex = ""
    transactionHex = ""
    outputIndex = "0"
    coreChainLockedHeight = ""
    outPointHex = ""
    result = nil
  }

  /// The user confirmed the held top-up in the history: release the form.
  func releasePaymentHold() {
    paymentFailure = nil
    errorMessage = nil
    showResult = false
  }

  func executeTopUp(sdk: SDK) async {
    guard permitsTopUp else { return }
    guard let outputAddressData = AddressTransformer.hexToData(outputAddressHex),
      let privateKeyData = AddressTransformer.hexToData(assetLockPrivateKeyHex)
    else {
      errorMessage = "Invalid input data"
      showResult = true
      return
    }

    let outputAmountValue = NumberTransformer.parseUInt64(outputAmount) ?? 0
    isLoading = true
    errorMessage = nil
    result = nil
    showResult = false

    do {
      let outputs = [
        Addresses.AddressTransferOutput(
          addressBytes: outputAddressData,
          amount: outputAmountValue
        )
      ]

      let topUpResult: PlatformAddressInfosResult
      if proofType == .instant {
        guard let instantLockData = AddressTransformer.hexToData(instantLockHex),
          let transactionData = AddressTransformer.hexToData(transactionHex),
          let outputIdx = NumberTransformer.parseUInt32(outputIndex)
        else {
          errorMessage = "Invalid instant lock data"
          showResult = true
          isLoading = false
          return
        }
        topUpResult = try sdk.addresses.topUpAddressFromAssetLock(
          proofType: .instant,
          instantLockData: instantLockData,
          transactionData: transactionData,
          outputIndex: outputIdx,
          coreChainLockedHeight: 0,
          outPoint: nil,
          assetLockPrivateKey: privateKeyData,
          outputs: outputs
        )
      } else {
        guard let outPointData = AddressTransformer.hexToData(outPointHex),
          let height = NumberTransformer.parseUInt32(coreChainLockedHeight)
        else {
          errorMessage = "Invalid chain lock data"
          showResult = true
          isLoading = false
          return
        }
        topUpResult = try sdk.addresses.topUpAddressFromAssetLock(
          proofType: .chain,
          instantLockData: nil,
          transactionData: nil,
          outputIndex: 0,
          coreChainLockedHeight: height,
          outPoint: outPointData,
          assetLockPrivateKey: privateKeyData,
          outputs: outputs
        )
      }
      paymentFailure = nil
      result = topUpResult
      showResult = true
    } catch {
      applyTopUpFailure(error)
    }
    isLoading = false
  }

  /// The asset lock may already be consumed: present the failure by its
  /// payment class (an SDK error with no class reads as unconfirmed, no
  /// retry) and keep it, so a holding one refuses the next top-up; an error
  /// that is not an SDK error shows as its own message and keeps any
  /// earlier hold.
  func applyTopUpFailure(_ error: Error) {
    paymentFailure = PaymentFailure.keptAfter(paymentFailure, error: error)
    errorMessage = PaymentFailure.ofPayment(error)?.message ?? error.localizedDescription
    showResult = true
  }
}
