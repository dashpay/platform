import DashSDKFFI
import Foundation
import XCTest
@testable import SwiftDashSDK

/// `topUpExternalIdentityFromAddresses` is a bridge to
/// `platform_wallet_top_up_external_identity_from_addresses_with_signer`:
/// it rejects malformed arguments before the FFI, hands the rest to the
/// export and maps the export's result code. Nothing here reaches Platform.
final class ExternalIdentityTopUpTests: XCTestCase {

    private let identityId = Data((0..<32).map { UInt8($0) })
    private let input = ManagedPlatformWallet.IdentityAddressInput(
        addressType: 0, hash: Data(repeating: 0xAB, count: 20), credits: 1_000
    )

    private func wallet() -> ManagedPlatformWallet {
        ManagedPlatformWallet(handle: NULL_HANDLE, walletId: Data())
    }

    private func signer() throws -> KeychainSigner {
        KeychainSigner(modelContainer: try DashModelContainer.createInMemory(), network: .testnet)
    }

    private func assertInvalidParameter(
        identityId: Data,
        inputs: [ManagedPlatformWallet.IdentityAddressInput],
        contains expected: String,
        file: StaticString = #filePath,
        line: UInt = #line
    ) async throws {
        do {
            _ = try await wallet().topUpExternalIdentityFromAddresses(
                identityId: identityId, inputs: inputs, addressSigner: try signer()
            )
            XCTFail("expected invalidParameter", file: file, line: line)
        } catch PlatformWalletError.invalidParameter(let message, _) {
            XCTAssertTrue(message.contains(expected), "got \(message)", file: file, line: line)
        } catch {
            XCTFail("expected invalidParameter, got \(error)", file: file, line: line)
        }
    }

    func testRejectsIdentityIdThatIsNot32BytesBeforeTheFFI() async throws {
        try await assertInvalidParameter(
            identityId: Data(repeating: 1, count: 31), inputs: [input],
            contains: "identityId must be 32 bytes, got 31"
        )
    }

    /// An empty input list is not refused in Swift: it goes to the export as
    /// `(NULL, 0)`, and the export's invalid-parameter code comes back typed.
    func testHandsEmptyInputsToTheExportAsNullAndZero() async throws {
        let recorder = Recorder()
        do {
            _ = try await wallet()
                .topUpFromAddresses(
                    identityId: identityId, inputs: [], addressSigner: try signer()
                ) { _, _, inputsPtr, count, _, _ in
                    recorder.inputsWereNull = inputsPtr == nil
                    recorder.count = count
                    return PlatformWalletFFIResult(
                        code: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_INVALID_PARAMETER,
                        message: nil
                    )
                }
            XCTFail("the export's invalid-parameter code must throw")
        } catch PlatformWalletError.invalidParameter {
        } catch {
            XCTFail("expected invalidParameter, got \(error)")
        }
        XCTAssertTrue(recorder.inputsWereNull)
        XCTAssertEqual(recorder.count, 0)
    }

    func testRejectsMalformedAddressHashBeforeTheFFI() async throws {
        let short = ManagedPlatformWallet.IdentityAddressInput(
            addressType: 0, hash: Data(repeating: 0xAB, count: 19), credits: 1
        )
        try await assertInvalidParameter(
            identityId: identityId, inputs: [input, short],
            contains: "input hash must be 20 bytes, got 19"
        )
    }

    /// Well-formed arguments reach the real export, which resolves the wallet
    /// handle first: an unknown handle comes back as the registry's `notFound`.
    func testWellFormedCallReachesTheExportAndMapsItsCode() async throws {
        do {
            _ = try await wallet().topUpExternalIdentityFromAddresses(
                identityId: identityId, inputs: [input], addressSigner: try signer()
            )
            XCTFail("a null wallet handle must not report a balance")
        } catch PlatformWalletError.notFound {
        } catch {
            XCTFail("expected notFound from the export, got \(error)")
        }
    }

    // MARK: - Export selection

    /// Each public wrapper hands its arguments to its own export: neither
    /// the top-up of any identity nor the external top-up reaches the
    /// managed one, which would refuse an identity the wallet does not
    /// manage.
    func testEachWrapperDispatchesToItsOwnExport() async throws {
        let wallet = self.wallet()
        wallet.addressTopUpExports = .init(
            any: { _, _, _, _, _, outBalance in
                outBalance!.pointee = 3
                return PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
            },
            managed: { _, _, _, _, _, outBalance in
                outBalance!.pointee = 1
                return PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
            },
            external: { _, _, _, _, _, outBalance in
                outBalance!.pointee = 2
                return PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
            }
        )

        let external = try await wallet.topUpExternalIdentityFromAddresses(
            identityId: identityId, inputs: [input], addressSigner: try signer()
        )
        let managed = try await wallet.topUpFromAddresses(
            identityId: identityId, inputs: [input], addressSigner: try signer()
        )

        let any = try await wallet.topUpIdentityFromAddresses(
            identityId: identityId, inputs: [input], addressSigner: try signer()
        )

        XCTAssertEqual(any, 3, "the top-up of any identity must call its own export")
        XCTAssertEqual(external, 2, "the external wrapper must call the external export")
        XCTAssertEqual(managed, 1, "the managed wrapper must call the managed export")
    }

    // MARK: - Marshalling through the shared helper

    func testMarshalsArgumentsAndReturnsTheBalanceTheExportWrites() async throws {
        let signer = try signer()
        let recorder = Recorder()
        let inputs = [
            input,
            .init(addressType: 1, hash: Data(repeating: 0xCD, count: 20), credits: 7)
        ]
        let balance = try await wallet()
            .topUpFromAddresses(
                identityId: identityId, inputs: inputs, addressSigner: signer
            ) { handle, idPtr, inputsPtr, count, signerHandle, outBalance in
                recorder.record(
                    handle: handle,
                    id: withUnsafeBytes(of: idPtr!.pointee) { Data($0) },
                    rows: Array(UnsafeBufferPointer(start: inputsPtr, count: Int(count))),
                    signer: signerHandle
                )
                outBalance!.pointee = 123_456
                return PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
            }

        XCTAssertEqual(balance, 123_456)
        XCTAssertEqual(recorder.handle, NULL_HANDLE)
        XCTAssertEqual(recorder.id, identityId)
        XCTAssertEqual(recorder.signer, signer.handle)
        XCTAssertEqual(recorder.rows.map(\.address_type), [0, 1])
        XCTAssertEqual(recorder.rows.map(\.credits), [1_000, 7])
        XCTAssertEqual(
            recorder.rows.map { row in withUnsafeBytes(of: row.hash) { Data($0) } },
            [Data(repeating: 0xAB, count: 20), Data(repeating: 0xCD, count: 20)]
        )
    }

    func testIdentityNotFoundCodeMapsToIdentityNotFound() async throws {
        XCTAssertEqual(PlatformWalletResultCode.errorIdentityNotFound.rawValue, 7)
        do {
            _ = try await wallet().topUpFromAddresses(
                identityId: identityId, inputs: [input], addressSigner: try signer()
            ) { _, _, _, _, _, _ in
                PlatformWalletFFIResult(
                    code: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_IDENTITY_NOT_FOUND,
                    message: nil
                )
            }
            XCTFail("code 7 must throw")
        } catch PlatformWalletError.identityNotFound {
        } catch {
            XCTFail("expected identityNotFound, got \(error)")
        }
    }

    private final class Recorder: @unchecked Sendable {
        var handle: Handle?
        var id = Data()
        var rows: [IdentityFundingInputFFI] = []
        var signer: OpaquePointer?
        var inputsWereNull = false
        var count: UInt = .max

        func record(handle: Handle, id: Data, rows: [IdentityFundingInputFFI], signer: OpaquePointer?) {
            self.handle = handle
            self.id = id
            self.rows = rows
            self.signer = signer
        }
    }
}
