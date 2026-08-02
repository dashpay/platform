import XCTest
@testable import SwiftDashSDK

/// Where the txMetadata wire-version decision lives, proven through
/// `ManagedPlatformWallet.createEncryptedDocument`.
///
/// The wrapper does not decide which version bytes are meaningful. Only the
/// wallet core knows which ones the legacy stack can decode, and it rejects an
/// unsupported one from the arguments alone — before the wallet handle is
/// resolved, before the key resolver runs, and before anything is sealed. A
/// guard in Swift would be a second place where that set is written down, free
/// to drift from the core and to reject a value a later core accepts.
///
/// These cases therefore assert the RUST behavior as it arrives through the
/// FFI: an unsupported byte reaches the export and comes back as a propagated
/// invalid-parameter result. That the rejection happens before the handle is
/// used is what lets a dummy, never-registered handle exercise it with no live
/// wallet.
final class EncryptedDocumentVersionValidationTests: XCTestCase {

    /// A dummy handle that is never registered in the FFI handle storage.
    ///
    /// The shared argument gate runs before the handle is resolved, so an
    /// unsupported version is refused without it ever being read. If the
    /// ordering regressed, these cases would surface a not-found failure
    /// instead of the invalid-parameter one asserted below — which is exactly
    /// what makes the ordering observable here.
    private func makeWallet() -> ManagedPlatformWallet {
        ManagedPlatformWallet(handle: 0, walletId: Data(count: 32))
    }

    private let id32 = Data(count: 32)
    private let payload = Data([0, 1, 2, 3])

    /// A signer is a required argument. Only its presence is checked before the
    /// version is rejected, so an in-memory-backed instance is enough.
    private func makeSigner() throws -> KeychainSigner {
        let container = try DashModelContainer.createInMemory()
        return KeychainSigner(modelContainer: container, network: .testnet)
    }

    private func create(version: UInt8) async throws -> (Identifier, String) {
        let wallet = makeWallet()
        let signer = try makeSigner()
        return try await wallet.createEncryptedDocument(
            ownerIdentityId: id32,
            contractId: id32,
            documentType: "txMetadata",
            version: version,
            payload: payload,
            signer: signer
        )
    }

    /// An unsupported wire version is refused by the Rust core and the typed
    /// failure propagates through the wrapper unchanged.
    ///
    /// The wrapper passes the byte through untouched, so what is asserted here
    /// is the core's decision arriving intact — not a Swift-side check.
    func testAnUnsupportedVersionIsRefusedByTheRustCore() async throws {
        for version: UInt8 in [2, 3, 127, 255] {
            do {
                _ = try await create(version: version)
                XCTFail("version=\(version) must be refused by the wallet core")
            } catch let error as PlatformWalletError {
                guard case let .invalidParameter(message) = error else {
                    XCTFail(
                        "version=\(version) must surface as .invalidParameter — a "
                            + "not-found failure would mean the wallet handle was "
                            + "resolved before the version was judged, got \(error)"
                    )
                    continue
                }
                XCTAssertFalse(
                    message.isEmpty,
                    "the core's typed explanation must reach the caller"
                )
            } catch {
                XCTFail("expected PlatformWalletError for version=\(version), got \(error)")
            }
        }
    }

    /// A supported version gets past the argument gate and on to the wallet
    /// lookup, which fails because this handle was never registered.
    ///
    /// This is what shows the rejections above are the version gate's doing and
    /// not an unconditional refusal of every call made with a dummy handle.
    func testASupportedVersionGetsPastTheArgumentGate() async throws {
        for version: UInt8 in [0, 1] {
            do {
                _ = try await create(version: version)
                XCTFail("version=\(version) cannot succeed against an unregistered handle")
            } catch let error as PlatformWalletError {
                if case .invalidParameter = error {
                    XCTFail(
                        "version=\(version) is supported and must not be refused as an "
                            + "invalid argument; got \(error)"
                    )
                }
            } catch {
                XCTFail("expected PlatformWalletError for version=\(version), got \(error)")
            }
        }
    }
}
