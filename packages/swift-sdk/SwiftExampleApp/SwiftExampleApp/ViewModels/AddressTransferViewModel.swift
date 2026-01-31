import Foundation
import SwiftUI
import SwiftDashSDK

/// ViewModel for the Transfer Address Funds form.
/// Holds form state, validation, and executes the transfer via SDK.
@MainActor
final class AddressTransferViewModel: BaseViewModel {

    // MARK: - Form inputs

    @Published var inputAddressHex = ""
    @Published var inputAmount = ""
    @Published var inputPrivateKeyHex = ""
    @Published var outputAddressHex = ""
    @Published var outputAmount = ""

    // MARK: - Result

    @Published var result: PlatformAddressInfosResult?

    // MARK: - Validation

    private var validationResult: ValidationResult {
        TransferInputValidator.validate(
            inputAddressHex: inputAddressHex,
            inputPrivateKeyHex: inputPrivateKeyHex,
            outputAddressHex: outputAddressHex,
            inputAmount: inputAmount,
            outputAmount: outputAmount
        )
    }

    var isFormValid: Bool {
        validationResult.isValid
    }

    var validationErrors: [String] {
        validationResult.errors
    }

    // MARK: - Actions

    override func reset() {
        super.reset()
        inputAddressHex = ""
        inputAmount = ""
        inputPrivateKeyHex = ""
        outputAddressHex = ""
        outputAmount = ""
        result = nil
    }

    /// Execute the transfer using the given SDK. Updates result or errorMessage on main actor.
    func executeTransfer(sdk: SDK) async {
        guard let inputAddressData = Data(hexString: inputAddressHex),
              let privateKeyData = Data(hexString: inputPrivateKeyHex),
              let outputAddressData = Data(hexString: outputAddressHex),
              let inputAmt = UInt64(inputAmount),
              let outputAmt = UInt64(outputAmount)
        else {
            errorMessage = "Invalid input data"
            showResult = true
            return
        }

        isLoading = true
        errorMessage = nil
        result = nil
        showResult = false

        do {
            let inputs = [
                Addresses.AddressTransferInput(
                    addressBytes: inputAddressData,
                    amount: inputAmt,
                    nonce: 0,
                    privateKey: privateKeyData
                )
            ]
            let outputs = [
                Addresses.AddressTransferOutput(
                    addressBytes: outputAddressData,
                    amount: outputAmt
                )
            ]
            let transferResult = try sdk.addresses.transferFunds(
                inputs: inputs,
                outputs: outputs,
                feeFromInputIndex: 0
            )
            result = transferResult
            showResult = true
        } catch {
            errorMessage = error.localizedDescription
            showResult = true
        }
        isLoading = false
    }
}
