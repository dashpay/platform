import Foundation

/// Durable marker for one Core-to-shielded "Max" transfer (wallet drain).
///
/// The SDK identifies a drain by `operationId` and requires the caller to
/// persist it before calling, so a restart can ask Rust what became of it.
/// Every amount, input and fee is Rust's; this record only names the
/// operation, the account index it swept and the recipient a resume pays.
struct WalletDrainOperationRecord: Codable, Equatable, Identifiable, Sendable {
    let operationId: UUID
    let walletId: Data
    /// `Network.rawValue`, the same value `PersistentWallet.networkRaw` holds.
    let networkRaw: UInt32
    let fundingAccountIndex: UInt32
    /// The wallet's own shielded address at submit time. A resume pays it:
    /// Rust refuses a recipient other than the one recorded on the lock.
    let recipientRaw43: Data
    let createdAt: Date

    var id: UUID { operationId }

    /// The 16 bytes Rust stores in `PersistentAssetLock.operationId`, read
    /// back as a UUID. `nil` for anything that is not 16 bytes.
    static func uuid(fromOperationIdBytes bytes: Data) -> UUID? {
        guard bytes.count == 16 else { return nil }
        let b = [UInt8](bytes)
        return UUID(uuid: (
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
            b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
        ))
    }
}

/// JSON file of unfinished wallet-drain operation ids, in Application Support.
///
/// A file written with `.atomic` rather than `UserDefaults`, because
/// `UserDefaults` writes are flushed asynchronously and the id must be on
/// disk before the FFI call starts. Not a SwiftData model, because an
/// app-only marker does not justify changing the shared SwiftData model
/// graph and its schema versions.
///
/// A corrupt file throws instead of reading as empty: treating it as empty
/// would hide an unfinished transfer.
///
/// `@unchecked Sendable`: the stored properties are a URL and an `NSLock`
/// that serializes every read-modify-write of the file.
final class WalletDrainOperationStore: @unchecked Sendable {
    static let shared = WalletDrainOperationStore(
        directory: URL.applicationSupportDirectory
            .appendingPathComponent("WalletDrainOperations", isDirectory: true)
    )

    let fileURL: URL
    private let directory: URL
    private let lock = NSLock()

    init(directory: URL) {
        self.directory = directory
        self.fileURL = directory.appendingPathComponent("wallet-drain-operations.json")
    }

    func records() throws -> [WalletDrainOperationRecord] {
        lock.lock()
        defer { lock.unlock() }
        return try readLocked()
    }

    /// Adds `record`, replacing one with the same operation id. Throws when
    /// the file cannot be written, so the caller does not start the drain.
    func insert(_ record: WalletDrainOperationRecord) throws {
        lock.lock()
        defer { lock.unlock() }
        var current = try readLocked().filter { $0.operationId != record.operationId }
        current.append(record)
        try writeLocked(current)
    }

    func remove(operationId: UUID) throws {
        lock.lock()
        defer { lock.unlock() }
        let current = try readLocked()
        let remaining = current.filter { $0.operationId != operationId }
        guard remaining.count != current.count else { return }
        try writeLocked(remaining)
    }

    /// Removes every record of `walletId`, on every network. Used when the
    /// wallet itself is deleted, so a re-imported wallet with the same id
    /// does not inherit markers of transfers the user chose to forget.
    func remove(walletId: Data) throws {
        lock.lock()
        defer { lock.unlock() }
        let current = try readLocked()
        let remaining = current.filter { $0.walletId != walletId }
        guard remaining.count != current.count else { return }
        try writeLocked(remaining)
    }

    private func readLocked() throws -> [WalletDrainOperationRecord] {
        let data: Data
        do {
            data = try Data(contentsOf: fileURL)
        } catch let error as CocoaError where error.code == .fileReadNoSuchFile {
            return []
        }
        return try JSONDecoder().decode([WalletDrainOperationRecord].self, from: data)
    }

    private func writeLocked(_ records: [WalletDrainOperationRecord]) throws {
        try FileManager.default.createDirectory(
            at: directory,
            withIntermediateDirectories: true
        )
        let data = try JSONEncoder().encode(records)
        try data.write(to: fileURL, options: [.atomic])
    }
}
