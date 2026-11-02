import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

/// The invitee's read-only view of a `dashpay://invite` link: the offline
/// preview carries the inviter metadata the claim UI shows, and the networked
/// claim-status call refuses a malformed link definitively rather than
/// reporting it as an unclaimed invitation.
final class InvitationPreviewTests: XCTestCase {

    /// Testnet WIF for the secret `0x11 × 32` (compressed).
    private static let testnetWif = "cN9spWsvaxA8taS7DFMxnk1yJD2gaF2PX1npuTpy3vuZFJdwavaw"
    private static let txid = String(repeating: "ab", count: 32)

    private func wallet() -> ManagedPlatformWallet {
        // The preview never touches the handle; the status call rejects a
        // malformed link before any wallet lookup.
        ManagedPlatformWallet(handle: NULL_HANDLE, walletId: Data())
    }

    func testPreviewSurfacesInviterDisplayNameAndAvatar() throws {
        let uri = "dashpay://invite?du=alice&assetlocktx=\(Self.txid)&pk=\(Self.testnetWif)"
            + "&islock=null&display-name=Alice%20B&avatar-url=https%3A%2F%2Fexample.org%2Fa.png"
        let preview = try wallet().parseInvitation(uri: uri)
        XCTAssertTrue(preview.structurallyValid)
        XCTAssertEqual(preview.inviterUsername, "alice")
        XCTAssertEqual(preview.inviterDisplayName, "Alice B")
        XCTAssertEqual(preview.inviterAvatarURL, "https://example.org/a.png")
        XCTAssertEqual(preview.amountDuffs, 0, "the amount is a network fact, not in the link")
    }

    func testPreviewWithoutMetadataHasNilDisplayNameAndAvatar() throws {
        let uri = "dashpay://invite?du=alice&assetlocktx=\(Self.txid)&pk=\(Self.testnetWif)&islock=null"
        let preview = try wallet().parseInvitation(uri: uri)
        XCTAssertTrue(preview.structurallyValid)
        XCTAssertEqual(preview.inviterUsername, "alice")
        XCTAssertNil(preview.inviterDisplayName)
        XCTAssertNil(preview.inviterAvatarURL)
    }

    func testMalformedPreviewIsInvalidNotThrown() throws {
        let preview = try wallet().parseInvitation(uri: "https://not-an-invite")
        XCTAssertFalse(preview.structurallyValid)
        XCTAssertNil(preview.inviterDisplayName)
        XCTAssertNil(preview.inviterAvatarURL)
    }

    func testClaimStatusRejectsMalformedLinkAsInvalidParameter() async {
        do {
            _ = try await wallet().invitationClaimStatus(uri: "https://not-an-invite")
            XCTFail("a malformed link must throw, never report an unclaimed invitation")
        } catch PlatformWalletError.invalidParameter {
            // Definitive: there is no invitation to claim.
        } catch {
            XCTFail("expected invalidParameter, got \(error)")
        }
    }
}
