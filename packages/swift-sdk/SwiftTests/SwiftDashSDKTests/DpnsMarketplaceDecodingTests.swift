import Foundation
import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

final class DpnsMarketplaceDecodingTests: XCTestCase {
    private let counterparty = Data(repeating: 0x44, count: 32)

    private func ffiRow(
        status: UInt8,
        counterparty: Data? = nil
    ) -> DpnsNameStateRowFFI {
        var row = DpnsNameStateRowFFI()
        row.status = status
        row.has_counterparty = counterparty != nil
        if let counterparty {
            _ = withUnsafeMutableBytes(of: &row.counterparty_id) { destination in
                counterparty.copyBytes(to: destination)
            }
        }
        return row
    }

    func testKnownConsistentStatusesDecode() throws {
        let owned = try XCTUnwrap(DpnsNameStateRow(ffi: ffiRow(status: 0)))
        XCTAssertEqual(owned.status, .owned)

        let sold = try XCTUnwrap(
            DpnsNameStateRow(ffi: ffiRow(status: 1, counterparty: counterparty))
        )
        XCTAssertEqual(sold.status, .sold(to: counterparty))

        let transferred = try XCTUnwrap(
            DpnsNameStateRow(ffi: ffiRow(status: 2, counterparty: counterparty))
        )
        XCTAssertEqual(transferred.status, .transferred(to: counterparty))
    }

    func testUnknownAndInconsistentStatusesFailClosed() {
        XCTAssertNil(DpnsNameStateRow(ffi: ffiRow(status: 3)))
        XCTAssertNil(DpnsNameStateRow(ffi: ffiRow(status: 3, counterparty: counterparty)))
        XCTAssertNil(DpnsNameStateRow(ffi: ffiRow(status: 0, counterparty: counterparty)))
        XCTAssertNil(DpnsNameStateRow(ffi: ffiRow(status: 1)))
        XCTAssertNil(DpnsNameStateRow(ffi: ffiRow(status: 2)))
    }

    func testZeroDocumentTimestampsDecodeAsUnknown() throws {
        var row = ffiRow(status: 0)
        row.created_at_ms = 0
        row.updated_at_ms = 12
        row.transferred_at_ms = 0
        row.last_synced_at_ms = 99

        let decoded = try XCTUnwrap(DpnsNameStateRow(ffi: row))
        XCTAssertNil(decoded.createdAtMs)
        XCTAssertEqual(decoded.updatedAtMs, 12)
        XCTAssertNil(decoded.transferredAtMs)
        XCTAssertEqual(decoded.lastSyncedAtMs, 99)
    }

    func testDetailedSyncRowsPreserveIdentifiersStatusesAndOptionalPrices() {
        let identityId = Data(repeating: 0x11, count: 32)
        let documentId = Data(repeating: 0x22, count: 32)

        var addedFFI = DpnsNameAddedFFI()
        _ = withUnsafeMutableBytes(of: &addedFFI.identity_id) { destination in
            identityId.copyBytes(to: destination)
        }
        let added = DpnsNameAdded(ffi: addedFFI)
        XCTAssertEqual(added.identityId, identityId)

        var departedFFI = DpnsNameDepartedFFI()
        _ = withUnsafeMutableBytes(of: &departedFFI.identity_id) { destination in
            identityId.copyBytes(to: destination)
        }
        departedFFI.has_document_id = true
        _ = withUnsafeMutableBytes(of: &departedFFI.document_id) { destination in
            documentId.copyBytes(to: destination)
        }
        departedFFI.has_status = true
        departedFFI.status = 1
        _ = withUnsafeMutableBytes(of: &departedFFI.counterparty_id) { destination in
            counterparty.copyBytes(to: destination)
        }
        let departed = DpnsNameDeparture(ffi: departedFFI)
        XCTAssertEqual(departed.identityId, identityId)
        XCTAssertEqual(departed.documentId, documentId)
        XCTAssertEqual(departed.status, .sold(to: counterparty))

        var priceFFI = DpnsPriceChangeFFI()
        _ = withUnsafeMutableBytes(of: &priceFFI.document_id) { destination in
            documentId.copyBytes(to: destination)
        }
        priceFFI.has_previous = false
        priceFFI.has_current = true
        priceFFI.current = 500
        let change = DpnsPriceChange(ffi: priceFFI)
        XCTAssertEqual(change.documentId, documentId)
        XCTAssertNil(change.previousPriceCredits)
        XCTAssertEqual(change.currentPriceCredits, 500)
    }

    func testMarketplaceJSONErrorsDecodeToTypedCases() {
        let price = PlatformWalletError(
            code: .errorDocumentPriceChanged,
            message: #"{"documentId":"doc","expected":10,"actual":12}"#
        )
        guard case .priceChanged(let documentId, let expected, let actual) = price else {
            return XCTFail("Expected priceChanged, got \(price)")
        }
        XCTAssertEqual(documentId, "doc")
        XCTAssertEqual(expected, 10)
        XCTAssertEqual(actual, 12)

        let credits = PlatformWalletError(
            code: .errorInsufficientIdentityCredits,
            message: #"{"identityId":"identity","required":101,"available":100}"#
        )
        guard case .insufficientIdentityCredits(
            let identityId,
            let required,
            let available
        ) = credits else {
            return XCTFail("Expected insufficientIdentityCredits, got \(credits)")
        }
        XCTAssertEqual(identityId, "identity")
        XCTAssertEqual(required, 101)
        XCTAssertEqual(available, 100)

        let contest = PlatformWalletError(
            code: .errorContestedNameNotTradable,
            message: #"{"label":"a11ce","endsAtMs":1234}"#
        )
        guard case .contestedNameNotTradable(let label, let endsAtMs) = contest else {
            return XCTFail("Expected contestedNameNotTradable, got \(contest)")
        }
        XCTAssertEqual(label, "a11ce")
        XCTAssertEqual(endsAtMs, 1_234)
    }

    func testMalformedMarketplaceJSONFailsClosed() {
        let malformed = PlatformWalletError(
            code: .errorDocumentPriceChanged,
            message: #"{"documentId":"doc","expected":10}"#
        )
        guard case .unknown(let detail) = malformed else {
            return XCTFail("Expected unknown, got \(malformed)")
        }
        XCTAssertEqual(detail, #"{"documentId":"doc","expected":10}"#)
    }

    func testManagerSyncWalletResultCopiesCallbackStorage() {
        let walletId = Data(repeating: 0x55, count: 32)
        var ffi = DpnsSyncWalletResultFFI()
        _ = withUnsafeMutableBytes(of: &ffi.wallet_id) { destination in
            walletId.copyBytes(to: destination)
        }
        ffi.success = false
        ffi.names_tracked = 1
        ffi.names_added = 2
        ffi.names_departed = 3
        ffi.prices_changed = 4

        let decoded = "native callback error".withCString { pointer in
            ffi.error_message = pointer
            return DpnsWalletSyncResult(ffi: ffi)
        }

        XCTAssertEqual(decoded.walletId, walletId)
        XCTAssertFalse(decoded.success)
        XCTAssertEqual(decoded.namesTracked, 1)
        XCTAssertEqual(decoded.namesAdded, 2)
        XCTAssertEqual(decoded.namesDeparted, 3)
        XCTAssertEqual(decoded.pricesChanged, 4)
        XCTAssertEqual(decoded.errorMessage, "native callback error")
    }
}

@MainActor
final class DpnsMarketplaceManagerWrapperTests: XCTestCase {
    private func assertInvalidHandle(
        file: StaticString = #filePath,
        line: UInt = #line,
        _ operation: () throws -> Void
    ) {
        XCTAssertThrowsError(try operation(), file: file, line: line) { error in
            guard case PlatformWalletError.invalidHandle = error else {
                return XCTFail("Expected invalidHandle, got \(error)", file: file, line: line)
            }
        }
    }

    func testSynchronousDpnsSyncWrappersRejectUnconfiguredManager() {
        let manager = PlatformWalletManager()
        XCTAssertFalse(manager.isConfigured)

        assertInvalidHandle { try manager.startDpnsSync() }
        assertInvalidHandle { try manager.stopDpnsSync() }
        assertInvalidHandle { _ = try manager.isDpnsSyncRunning() }
        assertInvalidHandle { _ = try manager.isDpnsSyncing() }
        assertInvalidHandle { _ = try manager.dpnsLastSyncUnixSeconds() }
        assertInvalidHandle { try manager.setDpnsSyncInterval(seconds: 5) }
    }

    func testDpnsSyncNowRejectsUnconfiguredManager() async {
        let manager = PlatformWalletManager()
        do {
            _ = try await manager.dpnsSyncNow()
            XCTFail("Expected invalidHandle")
        } catch PlatformWalletError.invalidHandle {
            // Expected.
        } catch {
            XCTFail("Expected invalidHandle, got \(error)")
        }
    }

    func testEventExtensionPublishesOwnedWalletResults() async {
        let manager = PlatformWalletManager()
        let handler = PlatformWalletEventHandler(manager: manager)
        let eventExtension = handler.makeCallbacksExtension()
        XCTAssertEqual(
            eventExtension.struct_size,
            UInt(MemoryLayout<EventHandlerCallbacksExtension>.size)
        )
        XCTAssertEqual(
            eventExtension.version,
            UInt32(PLATFORM_WALLET_EVENT_CALLBACKS_EXTENSION_VERSION)
        )
        XCTAssertNotNil(eventExtension.on_dpns_marketplace_sync_completed_fn)

        let walletId = Data(repeating: 0x66, count: 32)
        var ffi = DpnsSyncWalletResultFFI()
        _ = withUnsafeMutableBytes(of: &ffi.wallet_id) { destination in
            walletId.copyBytes(to: destination)
        }
        ffi.success = false

        let context = Unmanaged.passUnretained(handler).toOpaque()
        "ephemeral error".withCString { errorPointer in
            ffi.error_message = errorPointer
            withUnsafePointer(to: &ffi) { resultPointer in
                dpnsMarketplaceSyncCompletedCallback(
                    context: context,
                    resultsPtr: resultPointer,
                    count: 1,
                    syncUnixSeconds: 123
                )
            }
        }

        for _ in 0..<10 where manager.lastDpnsSyncEvent == nil {
            await Task.yield()
        }

        let event = manager.lastDpnsSyncEvent
        XCTAssertEqual(event?.syncUnixSeconds, 123)
        XCTAssertEqual(event?.result(for: walletId)?.errorMessage, "ephemeral error")
    }
}
