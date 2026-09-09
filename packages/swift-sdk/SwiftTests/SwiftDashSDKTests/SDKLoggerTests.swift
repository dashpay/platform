import Foundation
import XCTest
@testable import SwiftDashSDK

final class SDKLoggerTests: XCTestCase {
    private func temporaryDirectory() throws -> URL {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("SDKLoggerTests-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        addTeardownBlock { try? FileManager.default.removeItem(at: directory) }
        return directory
    }

    func testConcurrentWritesProduceCompleteNonInterleavedLines() throws {
        let session = try temporaryDirectory()
        let sink = try SDKLogFileSink(sessionDirectory: session)
        let count = 500

        DispatchQueue.concurrentPerform(iterations: count) { index in
            let line = SDKLogFormatter.format(
                event: "parallel_write",
                category: .persistence,
                severity: .info,
                fields: ["index": .integer(Int64(index))]
            )
            sink.write(line)
        }
        sink.flush()

        let contents = try String(contentsOf: sink.fileURL, encoding: .utf8)
        let lines = contents.split(separator: "\n", omittingEmptySubsequences: true).map(String.init)

        XCTAssertEqual(lines.count, count)
        XCTAssertEqual(Set(lines).count, count)
        for line in lines {
            XCTAssertNotNil(
                line.range(
                    of: #"^\S+ INFO swift\.persistence event=parallel_write index=\d+$"#,
                    options: .regularExpression
                ),
                "Malformed or interleaved line: \(line)"
            )
        }
    }

    func testFormattingHasStableFieldOrderEscapingAndLevels() {
        let timestamp = Date(timeIntervalSince1970: 1_700_000_000)
        let fields: [String: SDKLogValue] = [
            "zeta": .boolean(true),
            "alpha": .publicText("first\nsecond\u{0085}third\u{2028}fourth\u{2029}fifth"),
            "middle": .integer(-2),
        ]

        let info = SDKLogFormatter.format(
            timestamp: timestamp,
            event: "format_test",
            category: .lifecycle,
            severity: .info,
            fields: fields
        )

        XCTAssertTrue(info.hasPrefix("2023-11-14T22:13:20.000Z INFO swift.lifecycle event=format_test "))
        XCTAssertTrue(
            info.hasSuffix(
                #"alpha="first\nsecond\u0085third\u2028fourth\u2029fifth" middle=-2 zeta=true"#
            )
        )
        XCTAssertFalse(info.contains("first\nsecond"))
        XCTAssertFalse(info.contains { $0.isNewline })

        for severity in [SDKLogSeverity.debug, .warning, .error] {
            let line = SDKLogFormatter.format(
                timestamp: timestamp,
                event: "level",
                category: .lifecycle,
                severity: severity
            )
            XCTAssertTrue(line.contains(" \(severity.rawValue) swift.lifecycle "))
        }
    }

    func testErrorIsStructuredRedactedAndLimited() {
        let sensitive = "identity-fixture-that-must-not-leak"
        let longDescription = sensitive + " /private/user/wallet.sqlite " + String(repeating: "x", count: 2_000)
        let error = NSError(
            domain: "wallet.persistence",
            code: 41,
            userInfo: [NSLocalizedDescriptionKey: longDescription]
        )

        let line = SDKLogFormatter.format(
            event: "save_failed",
            category: .persistence,
            severity: .error,
            error: error,
            redacting: [sensitive]
        )

        XCTAssertTrue(line.contains("error_code=41"))
        XCTAssertTrue(line.contains(#"error_domain="wallet.persistence""#))
        XCTAssertTrue(line.contains("error_type="))
        XCTAssertTrue(line.contains("<redacted>"))
        XCTAssertFalse(line.contains(sensitive))
        XCTAssertFalse(line.contains("wallet.sqlite"))
        XCTAssertFalse(line.contains("\n"))

        let marker = #"error_message=""#
        let start = try! XCTUnwrap(line.range(of: marker)?.upperBound)
        let remainder = line[start...]
        let end = try! XCTUnwrap(remainder.firstIndex(of: "\""))
        XCTAssertLessThanOrEqual(remainder[..<end].count, SDKLogFormatter.maximumErrorDescriptionLength)
    }

    func testShortSensitiveValueRedactsOnlyAWholeToken() {
        let error = NSError(
            domain: "SwiftDashSDK.PlatformWalletError",
            code: 19,
            userInfo: [NSLocalizedDescriptionKey: "invalid mnemonic: m"]
        )

        let line = SDKLogFormatter.format(
            event: "wallet_create_failed",
            category: .lifecycle,
            severity: .error,
            error: error,
            redacting: ["m"]
        )

        XCTAssertTrue(line.contains("error_domain=\"SwiftDashSDK.PlatformWalletError\""))
        XCTAssertTrue(line.contains("error_message=\"invalid mnemonic: <redacted>\""))
    }

    func testReferenceIsDeterministicAndRawValueNeverReachesLine() throws {
        let fixture = "wallet-identity-fixture-123456789"
        let first = SDKLogFormatter.reference(fixture)
        let second = SDKLogFormatter.reference(fixture)

        XCTAssertEqual(first, "40e17fb042f5")
        XCTAssertEqual(first, second)
        XCTAssertEqual(first.count, 12)

        let session = try temporaryDirectory()
        let sink = try SDKLogFileSink(sessionDirectory: session)
        sink.write(
            SDKLogFormatter.format(
                event: "reference_test",
                category: .lifecycle,
                severity: .info,
                fields: [
                    "data_reference": .reference(Data(fixture.utf8)),
                    "string_reference": .referenceString(fixture),
                ]
            )
        )
        sink.flush()

        let contents = try String(contentsOf: sink.fileURL, encoding: .utf8)
        XCTAssertFalse(contents.contains(fixture))
        XCTAssertEqual(contents.components(separatedBy: first).count - 1, 2)
    }

    func testFailedSinkCreationDoesNotAffectIndependentWorkingSink() throws {
        let root = try temporaryDirectory()
        let goodSession = root.appendingPathComponent("good", isDirectory: true)
        let goodSink = try SDKLogFileSink(sessionDirectory: goodSession)

        let invalidSession = root.appendingPathComponent("not-a-directory")
        XCTAssertTrue(FileManager.default.createFile(atPath: invalidSession.path, contents: Data()))
        XCTAssertThrowsError(try SDKLogFileSink(sessionDirectory: invalidSession))

        goodSink.write("still-active")
        goodSink.flush()
        XCTAssertEqual(
            try String(contentsOf: goodSink.fileURL, encoding: .utf8),
            "still-active\n"
        )
    }
}
