import Foundation
import SwiftDashSDK
import SwiftUI

/// ViewModel for the Get Address Info query view.
@MainActor
final class GetAddressInfoViewModel: BaseViewModel {

  @Published var addressInput = ""
  @Published var result: PlatformAddressInfo?

  static let testBech32m = "tdashevo1qqyfsqyzcn5hzu7echru54njypdq0v4d7gv8pkdf"
  static let testAddressHex = "00" + "1234567890abcdef1234567890abcdef12345678"

  var isFormValid: Bool { !addressInput.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }

  var detectedFormat: (icon: String, color: Color, description: String) {
    let trimmed = addressInput.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
    if trimmed.hasPrefix("dashevo1") || trimmed.hasPrefix("tdashevo1") {
      if Bech32m.isValidPlatformAddress(trimmed) {
        return ("checkmark.circle.fill", .green, "Valid bech32m address")
      } else {
        return ("xmark.circle.fill", .red, "Invalid bech32m address")
      }
    } else if trimmed.count == 42 && trimmed.allSatisfy({ $0.isHexDigit }) {
      return ("checkmark.circle.fill", .green, "Hex format (42 characters)")
    } else if !trimmed.isEmpty {
      return ("questionmark.circle", .orange, "Unknown format")
    }
    return ("circle", .gray, "")
  }

  override func reset() {
    super.reset()
    addressInput = ""
    result = nil
  }

  func fetchAddressInfo(sdk: SDK) async {
    isLoading = true
    errorMessage = nil
    result = nil
    showResult = false

    do {
      let info = try sdk.addresses.getInfo(address: addressInput)
      result = info
      showResult = true
    } catch {
      errorMessage = error.localizedDescription
      showResult = true
    }
    isLoading = false
  }
}
