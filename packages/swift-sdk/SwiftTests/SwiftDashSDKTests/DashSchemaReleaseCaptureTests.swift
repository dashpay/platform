import Foundation
import SwiftData
import XCTest

@testable import SwiftDashSDK

/// Run on an iOS simulator built from the same pinned sources as the archive.
/// The workflow exports these attachments before uploading that archive.
final class DashSchemaReleaseCaptureTests: XCTestCase {
    @MainActor
    func testCaptureReleaseSchema() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let store = directory.appendingPathComponent("fixture.store")
        try DashSchemaFixtureSupport.writeLiveStore(at: store)
        let description = try DashSchemaFixtureSupport.describeStore(
            at: store, version: DashModelContainer.schema.version)
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys, .prettyPrinted]
        let metadata = directory.appendingPathComponent("schema.json")
        try encoder.encode(description).write(to: metadata)
        for url in [metadata, store] {
            let attachment = XCTAttachment(contentsOfFile: url)
            attachment.name = url.lastPathComponent
            attachment.lifetime = .keepAlways
            add(attachment)
        }
    }
}
