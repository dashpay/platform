import Foundation
import SwiftDashSDK
import SwiftUI

/// ViewModel for the Withdraw Address Funds form.
@MainActor
final class WithdrawAddressFundsViewModel: BaseViewModel {

  @Published var inputAddressHex = ""
  @Published var inputAmount = ""
  @Published var inputPrivateKeyHex = ""
  @Published var coreAddress = ""
  @Published var changeAddressHex = ""
  @Published var useChangeAddress = false
  @Published var coreFeePerByte = "1"
  @Published var poolingStrategy: Addresses.PoolingStrategy = .never
  @Published var result: PlatformAddressInfosResult?

  private var validationResult: ValidationResult {
    WithdrawInputValidator.validate(
      inputAddressHex: inputAddressHex,
      inputPrivateKeyHex: inputPrivateKeyHex,
      coreAddress: coreAddress,
      inputAmount: inputAmount,
      useChangeAddress: useChangeAddress,
      changeAddressHex: changeAddressHex
    )
  }

  var isFormValid: Bool { validationResult.isValid }
  var validationErrors: [String] { validationResult.errors }

  override func reset() {
    super.reset()
    inputAddressHex = ""
    inputAmount = ""
    inputPrivateKeyHex = ""
    coreAddress = ""
    changeAddressHex = ""
    useChangeAddress = false
    coreFeePerByte = "1"
    poolingStrategy = .never
    result = nil
  }

  func executeWithdrawal(sdk: SDK) async {
    guard let input = TransferInputBuilder.createInput(
      addressHex: inputAddressHex,
      amount: inputAmount,
      nonce: 0,
      privateKeyHex: inputPrivateKeyHex
    )
    else {
      errorMessage = "Invalid input data"
      showResult = true
      return
    }

    let coreFee = NumberTransformer.parseFee(coreFeePerByte) ?? 0
    isLoading = true
    errorMessage = nil
    result = nil
    showResult = false

    do {
      let inputs = [input]
      let changeAddressData: Data? = useChangeAddress ? AddressTransformer.hexToData(changeAddressHex) : nil
      let withdrawalResult = try sdk.addresses.withdrawFunds(
        inputs: inputs,
        coreAddress: coreAddress,
        coreFeePerByte: coreFee,
        pooling: poolingStrategy,
        feeFromInputIndex: 0,
        changeAddress: changeAddressData
      )
      result = withdrawalResult
      showResult = true
    } catch {
      errorMessage = error.localizedDescription
      showResult = true
    }
    isLoading = false
  }
}
