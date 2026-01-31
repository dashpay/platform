import Foundation
import SwiftDashSDK
import SwiftUI

/// ViewModel for the Get Addresses Infos query view.
@MainActor
final class GetAddressesInfosViewModel: BaseViewModel {

  @Published var addressesText = ""
  @Published var result: PlatformAddressInfosResult?

  static let testBech32mAddresses = """
    tdashevo1qqyfsqyzcn5hzu7echru54njypdq0v4d7gv8pkdf
    tdashevo1qq0rs5w7e3xv6ls3f7s4hz82e44p29e38fqlmhs
    """

  static let testHexAddresses = """
    001234567890abcdef1234567890abcdef12345678
    00abcdef1234567890abcdef1234567890abcdef12
    """

  var isFormValid: Bool {
    !addressesText
      .components(separatedBy: .newlines)
      .map { $0.trimmingCharacters(in: .whitespaces) }
      .filter { !$0.isEmpty }
      .isEmpty
  }

  override func reset() {
    super.reset()
    addressesText = ""
    result = nil
  }

  func fetchAddressesInfos(sdk: SDK) async {
    isLoading = true
    errorMessage = nil
    result = nil
    showResult = false

    let addresses =
      addressesText
      .components(separatedBy: .newlines)
      .map { $0.trimmingCharacters(in: .whitespaces) }
      .filter { !$0.isEmpty }

    guard !addresses.isEmpty else {
      errorMessage = "No valid addresses entered"
      showResult = true
      isLoading = false
      return
    }

    do {
      let infosResult = try sdk.addresses.getInfos(addresses: addresses)
      result = infosResult
      showResult = true
    } catch {
      errorMessage = error.localizedDescription
      showResult = true
    }
    isLoading = false
  }
}
