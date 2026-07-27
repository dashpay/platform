import Foundation
import SwiftDashSDK
import SwiftUI

/// ViewModel for the Get Addresses Infos query view.
@MainActor
final class GetAddressesInfosViewModel: BaseViewModel {

  @Published var addressesText = ""
  @Published var result: PlatformAddressInfosResult?

  static let testBech32mAddresses = """
    tdash1kzdl4c3apkekqevkqrzctgagv2v2ng5hysegt5x4
    tdash1kz4ummcjx3t83y9tehh3ydzk0zg2hn00zgc6jtwv
    """

  // Hex field feeds the FFI directly, which expects the storage-form type byte
  // (0x00 = P2PKH), not the user-facing bech32m byte (0xb0).
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
