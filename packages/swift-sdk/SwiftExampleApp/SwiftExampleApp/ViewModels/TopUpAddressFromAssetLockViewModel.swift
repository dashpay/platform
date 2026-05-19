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

  func executeTopUp(sdk: SDK) async {
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
      result = topUpResult
      showResult = true
    } catch {
      errorMessage = error.localizedDescription
      showResult = true
    }
    isLoading = false
  }
}
