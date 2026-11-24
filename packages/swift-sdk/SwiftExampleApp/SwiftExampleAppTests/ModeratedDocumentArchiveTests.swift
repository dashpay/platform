import XCTest
@testable import SwiftExampleApp

/// Pins the on-device archive of deleted documents: a save says whether the
/// bytes are kept, they load back by contract, type and document id, a
/// restore's removal drops them, and only the newest files are kept.
final class ModeratedDocumentArchiveTests: XCTestCase {
    private var directory: URL!

    override func setUpWithError() throws {
        directory = FileManager.default.temporaryDirectory
            .appending(path: "ModeratedDocumentArchiveTests-\(UUID().uuidString)", directoryHint: .isDirectory)
    }

    override func tearDownWithError() throws {
        try? FileManager.default.removeItem(at: directory)
    }

    private func archive(capacity: Int = 10) -> ModeratedDocumentArchive {
        ModeratedDocumentArchive(directory: directory, capacity: capacity)
    }

    /// Ages the file kept for `documentId` to `date`, so eviction order does
    /// not hang on how quickly the saves ran.
    private func age(_ archive: ModeratedDocumentArchive, documentId: String, to date: Date) throws {
        let url = archive.fileURL(contractId: "C", documentTypeName: "post", documentId: documentId)
        try FileManager.default.setAttributes([.modificationDate: date], ofItemAtPath: url.path)
    }

    func testShouldKeepSavedBytesAndDropThemOnRemove() {
        let archive = archive()
        let bytes = Data([1, 2, 3])
        XCTAssertTrue(archive.save(bytes, contractId: "C", documentTypeName: "post", documentId: "D"))
        XCTAssertTrue(archive.contains(contractId: "C", documentTypeName: "post", documentId: "D"))
        XCTAssertEqual(archive.load(contractId: "C", documentTypeName: "post", documentId: "D"), bytes)
        // Another type or document is another entry
        XCTAssertNil(archive.load(contractId: "C", documentTypeName: "note", documentId: "D"))
        XCTAssertNil(archive.load(contractId: "C", documentTypeName: "post", documentId: "E"))

        archive.remove(contractId: "C", documentTypeName: "post", documentId: "D")
        XCTAssertFalse(archive.contains(contractId: "C", documentTypeName: "post", documentId: "D"))
        XCTAssertNil(archive.load(contractId: "C", documentTypeName: "post", documentId: "D"))
    }

    func testShouldNameEachEntryByAStableHashOfItsIds() {
        let archive = archive()
        let url = archive.fileURL(contractId: "C", documentTypeName: "post", documentId: "D")
        XCTAssertEqual(url, archive.fileURL(contractId: "C", documentTypeName: "post", documentId: "D"))
        XCTAssertEqual(url.deletingLastPathComponent().standardizedFileURL, directory.standardizedFileURL)
        // The separator keeps "C"+"post.D" and "C.post"+"D" apart
        XCTAssertNotEqual(
            archive.fileURL(contractId: "C", documentTypeName: "ab", documentId: "c"),
            archive.fileURL(contractId: "C", documentTypeName: "a", documentId: "bc")
        )
    }

    func testShouldEvictTheOldestFilesPastItsCapacity() throws {
        let archive = archive(capacity: 2)
        XCTAssertTrue(archive.save(Data([1]), contractId: "C", documentTypeName: "post", documentId: "old"))
        XCTAssertTrue(archive.save(Data([2]), contractId: "C", documentTypeName: "post", documentId: "mid"))
        try age(archive, documentId: "old", to: Date(timeIntervalSince1970: 1_000))
        try age(archive, documentId: "mid", to: Date(timeIntervalSince1970: 2_000))

        XCTAssertTrue(archive.save(Data([3]), contractId: "C", documentTypeName: "post", documentId: "new"))
        XCTAssertFalse(archive.contains(contractId: "C", documentTypeName: "post", documentId: "old"))
        XCTAssertTrue(archive.contains(contractId: "C", documentTypeName: "post", documentId: "mid"))
        XCTAssertTrue(archive.contains(contractId: "C", documentTypeName: "post", documentId: "new"))
    }

    func testShouldNeverEvictTheFileJustSavedEvenWhenItIsTheOldest() throws {
        let archive = archive(capacity: 1)
        XCTAssertTrue(archive.save(Data([1]), contractId: "C", documentTypeName: "post", documentId: "other"))
        // Dated ahead of the clock (a clock set back since), so the file
        // saved next is the oldest by date
        try age(archive, documentId: "other", to: Date().addingTimeInterval(86_400 * 365))
        XCTAssertTrue(archive.save(Data([2]), contractId: "C", documentTypeName: "post", documentId: "saved"))
        XCTAssertTrue(archive.contains(contractId: "C", documentTypeName: "post", documentId: "saved"))
        XCTAssertFalse(archive.contains(contractId: "C", documentTypeName: "post", documentId: "other"))
        XCTAssertEqual(archive.load(contractId: "C", documentTypeName: "post", documentId: "saved"), Data([2]))
    }

    func testShouldReportBytesItCouldNotKeep() throws {
        // A plain file where the folder should be: nothing can be written
        try Data().write(to: directory)
        let archive = archive()
        XCTAssertFalse(archive.save(Data([1]), contractId: "C", documentTypeName: "post", documentId: "D"))
        XCTAssertFalse(archive.contains(contractId: "C", documentTypeName: "post", documentId: "D"))
    }
}
