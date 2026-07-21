import XCTest
@testable import SwiftDashSDK

/// Version-byte validation for
/// `ManagedPlatformWallet.createEncryptedDocument` (dashpay/platform#4091).
/// Only `0` (CBOR) and `1` (protobuf) are wire-meaningful — `seal_tx_metadata`
/// writes the byte verbatim and the legacy dashj `decryptTxMetadata` switches
/// on exactly those two values, so an out-of-range byte would silently seal a
/// document the legacy stack can't decode.
///
/// The `guard` runs before any FFI call (no `platform_wallet_*` symbol is
/// dereferenced and the wallet `handle` is never used), so the REJECTION paths
/// are exercised with a dummy handle and no live wallet — the Swift mirror of
/// the Kotlin `DocumentTransactionsVersionValidationTest`
/// (`walletHandle = 0L`). The accepted values `0` / `1` would proceed into
/// native and can't be unit-tested here.
final class EncryptedDocumentVersionValidationTests: XCTestCase {

    /// A dummy, never-dispatched wallet handle (matches Kotlin's `0L`). The
    /// version guard throws before the handle is read, so no FFI dispatch
    /// occurs on the rejection paths under test.
    private func makeWallet() -> ManagedPlatformWallet {
        ManagedPlatformWallet(handle: 0, walletId: Data(count: 32))
    }

    private let id32 = Data(count: 32)
    private let payload = Data([0, 1, 2, 3])

    /// A signer is a required argument, but the version guard throws before it
    /// is ever dereferenced — an in-memory-backed instance is enough to
    /// satisfy the type. Built per-test.
    private func makeSigner() throws -> KeychainSigner {
        let container = try DashModelContainer.createInMemory()
        return KeychainSigner(modelContainer: container, network: .testnet)
    }

    /// Bytes `2...255` (every value the legacy `0..=255` range once accepted
    /// beyond the two wire-meaningful ones) are rejected with a message that
    /// names them.
    func testRejectsVersionBytesTheLegacyStackCannotDecode() async throws {
        let wallet = makeWallet()
        let signer = try makeSigner()
        for version: UInt8 in [2, 3, 127, 255] {
            do {
                _ = try await wallet.createEncryptedDocument(
                    ownerIdentityId: id32,
                    contractId: id32,
                    documentType: "txMetadata",
                    version: version,
                    payload: payload,
                    signer: signer
                )
                XCTFail("version=\(version) must be rejected")
            } catch let error as PlatformWalletError {
                guard case let .invalidParameter(message) = error else {
                    XCTFail("expected .invalidParameter for version=\(version), got \(error)")
                    continue
                }
                XCTAssertTrue(
                    message.contains("0 (CBOR) or 1 (protobuf)"),
                    "message should name the wire-meaningful versions, got: \(message)"
                )
            } catch {
                XCTFail("expected PlatformWalletError for version=\(version), got \(error)")
            }
        }
    }
}
