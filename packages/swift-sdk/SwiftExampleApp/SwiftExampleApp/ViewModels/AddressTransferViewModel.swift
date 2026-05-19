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
        guard let input = TransferInputBuilder.createInput(
            addressHex: inputAddressHex,
            amount: inputAmount,
            nonce: 0,
            privateKeyHex: inputPrivateKeyHex
        ),
        let output = TransferInputBuilder.createOutput(
            addressHex: outputAddressHex,
            amount: outputAmount
        )
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
            let inputs = [input]
            let outputs = [output]
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
