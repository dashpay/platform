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
    guard let outputAddressData = Data(hexString: outputAddressHex),
      let privateKeyData = Data(hexString: assetLockPrivateKeyHex)
    else {
      errorMessage = "Invalid input data"
      showResult = true
      return
    }

    let outputAmountValue = UInt64(outputAmount) ?? 0
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
        guard let instantLockData = Data(hexString: instantLockHex),
          let transactionData = Data(hexString: transactionHex),
          let outputIdx = UInt32(outputIndex)
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
        guard let outPointData = Data(hexString: outPointHex),
          let height = UInt32(coreChainLockedHeight)
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
