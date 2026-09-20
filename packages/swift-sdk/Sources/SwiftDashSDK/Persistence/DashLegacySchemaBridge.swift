import CoreData
import Darwin
import Foundation
import SwiftData

/// One-time, local-store bridge into the fixed V2 schema. The ordinary migration
/// plan owns recognized versions. This path accepts only an older 1.0.0 graph
/// whose complete existing SQLite data survives an inferred migration unchanged.
/// Call before publishing any container/context for the same URL.
enum DashLegacySchemaBridge {
    typealias SQLite = DashLegacyStoreSQLite
    enum Phase { case afterSnapshot, afterMigration, beforeInstall, writeLocked, afterCommit }
    struct Hooks {
        var visit: (Phase, URL) throws -> Void = { _, _ in }
        var availableCapacity: (URL) throws -> Int64 = DashLegacySchemaBridge.availableCapacity(at:)
    }
    struct Identity: Codable, Equatable {
        let versions: [String]
        let checksum: String
        let hashes: [String: Data]
    }
    private struct Journal: Codable {
        let formatVersion: Int
        let operation: UUID
        let source: Identity
        let destination: Identity
        let destinationData: SQLite.StoreEvidence?
    }

    static func open(configuration: ModelConfiguration, schema: Schema,
                     plan: any SchemaMigrationPlan.Type, hooks: Hooks = Hooks()) throws -> ModelContainer {
        let url = configuration.url
        func ordinary() throws -> ModelContainer {
            try ModelContainer(for: schema, migrationPlan: plan, configurations: [configuration])
        }
        guard !configuration.isStoredInMemoryOnly else { return try ordinary() }
        let root = backupDirectory(for: url)
        let marker = root.appendingPathComponent("active.json")
        if !FileManager.default.fileExists(atPath: url.path),
           !FileManager.default.fileExists(atPath: marker.path) { return try ordinary() }
        if !FileManager.default.fileExists(atPath: marker.path) {
            // Detection must not impose bridge-specific metadata or locking
            // requirements on stores handled by SwiftData's ordinary path.
            let needsMigration = (try? identity(at: url)).flatMap { try? needsBridge($0, plan: plan) } ?? false
            // Recheck after reading identity: a concurrent bridge publishes its
            // journal before the store can change to the destination schema.
            if !needsMigration && !FileManager.default.fileExists(atPath: marker.path) {
                let container = try ordinary()
                reclaimAfterSuccessfulOpen(at: url, root: root)
                return container
            }
        }
        let lock = try StoreLock(url: url)
        defer { lock.close() }
        // Recheck under the recovery lock. Our transactional installation never
        // removes the primary file, so absence may be an intentional external
        // reset. Neither resurrect an older backup nor create an empty store.
        guard FileManager.default.fileExists(atPath: url.path) else {
            throw SQLite.Failure.unsupported(
                "The original database is missing while migration recovery is pending. Recovery files remain at \(root.path). Restore the original database from a verified backup with the app closed, or contact support for deliberate recovery. Do not delete the journal or create an empty database.")
        }
        try SQLite.recoverRollbackJournal(at: url)
        try recoverIfNeeded(at: url, root: root)
        // Another opener may have finished migration before this lock was
        // acquired. Recovery errors above remain fatal; ordinary detection does not.
        guard let source = try? identity(at: url),
              (try? needsBridge(source, plan: plan)) == true else { return try ordinary() }
        guard configuration.allowsSave else { throw SQLite.Failure.unsupported("The store is read-only") }
        let permitted = Set(Schema(versionedSchema: DashSchemaV1.self).entities.map(\.name))
        let required: Set<String> = ["PersistentWallet", "PersistentAccount", "PersistentTransaction", "PersistentTxo"]
        let names = Set(source.hashes.keys)
        guard names.isSubset(of: permitted), required.isSubset(of: names) else {
            throw SQLite.Failure.unsupported("The old model contains unsupported or missing entities")
        }
        try rejectExternalStorage(at: url)
        // A killed attempt may leave copies before it ever published a journal.
        // Under the lock, no active marker means these directories are inactive.
        // Remove them before measuring capacity; never credit hypothetical space.
        reclaimInactiveAttempts(at: root, holding: lock)
        let requiredSpace = try requiredFreeSpace(at: url)
        let availableSpace = max(0, try hooks.availableCapacity(url.deletingLastPathComponent()))
        guard availableSpace >= requiredSpace else {
            throw SQLite.Failure.insufficientDiskSpace(required: requiredSpace, available: availableSpace)
        }
        let operation = UUID()
        let directory = root.appendingPathComponent(operation.uuidString, isDirectory: true)
        var directoryAttributes = try protectionAttributes(like: url)
        directoryAttributes[.posixPermissions] = 0o700
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true,
                                                attributes: directoryAttributes)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: false,
                                                attributes: directoryAttributes)
        var rootValues = URLResourceValues()
        rootValues.isExcludedFromBackup = true
        var excludedRoot = root
        try excludedRoot.setResourceValues(rootValues)
        let backup = directory.appendingPathComponent("original.store")
        let candidate = directory.appendingPathComponent("candidate.store")
        var committed = false
        defer {
            if !committed && !FileManager.default.fileExists(atPath: root.appendingPathComponent("active.json").path) {
                try? FileManager.default.removeItem(at: directory)
            }
        }
        let rawSource = try SQLite.rawDigest(url)
        try createProtectedFile(backup, like: url)
        try SQLite.copy(from: url, to: backup)
        guard try SQLite.rawDigest(url) == rawSource else { throw SQLite.Failure.sourceChanged }
        try SQLite.integrityCheck(backup)
        guard try identity(at: backup) == source else { throw SQLite.Failure.sourceChanged }
        try hooks.visit(.afterSnapshot, url)
        try createProtectedFile(candidate, like: url)
        try SQLite.copy(from: backup, to: candidate)
        // Never replace this with `schema` or the latest version. Users can skip
        // the V2 app; later releases must keep V2's frozen graph as this target.
        try autoreleasepool {
            let bridgeSchema = Schema(versionedSchema: DashSchemaV2.self)
            _ = try ModelContainer(for: bridgeSchema, configurations: [
                ModelConfiguration(schema: bridgeSchema, url: candidate, cloudKitDatabase: .none)
            ])
        }
        try SQLite.checkpoint(candidate)
        try hooks.visit(.afterMigration, candidate)
        try rejectExternalStorage(at: candidate)
        try SQLite.integrityCheck(candidate)
        try SQLite.validatePreservation(from: backup, to: candidate)
        // The normal plan must accept the fixed V2 result. In a future release
        // it may continue through additional explicitly registered stages here.
        try autoreleasepool {
            _ = try ModelContainer(for: schema, migrationPlan: plan, configurations: [
                ModelConfiguration(schema: schema, url: candidate, cloudKitDatabase: .none)
            ])
        }
        try SQLite.checkpoint(candidate)
        try rejectExternalStorage(at: candidate)
        try SQLite.integrityCheck(candidate)
        let destination = try identity(at: candidate)
        let journal = Journal(formatVersion: 2, operation: operation, source: source, destination: destination,
                              destinationData: try SQLite.evidence(at: candidate))
        // Publish the marker only after both copy filenames and their data are
        // durable. The marker lives in the parent directory, synced separately.
        for fileURL in [backup, candidate] {
            let file = try FileHandle(forWritingTo: fileURL)
            defer { try? file.close() }
            try file.synchronize()
        }
        try synchronizeDirectory(directory)
        try writeJournal(journal, at: root)
        try hooks.visit(.beforeInstall, url)
        try SQLite.copy(from: candidate, to: url) {
            // backup_step(0) owns SQLite's write lock, closing the race between
            // comparison and commit. Main/WAL bytes must still be the snapshot's
            // bytes; unrelated or concurrent writes cause a safe retry.
            guard try SQLite.rawDigest(url) == rawSource else { throw SQLite.Failure.sourceChanged }
            try hooks.visit(.writeLocked, url)
        }
        committed = true
        try hooks.visit(.afterCommit, url)
        let container = try ordinary()
        // This container has not escaped to the app, so no application writes
        // are permitted yet. Clearing must succeed before returning it: a stale
        // marker would incorrectly compare later writes with migration evidence.
        try clearJournal(at: root)
        try? FileManager.default.removeItem(at: candidate)
        return container
    }

    static func backupDirectory(for url: URL) -> URL {
        url.deletingLastPathComponent().appendingPathComponent(url.lastPathComponent + ".legacy-v2-backups", isDirectory: true)
    }

    /// Conservative estimate, not a reservation: two complete copies plus
    /// inferred-migration and promotion journals, with room for new columns and
    /// indexes. Include WAL bytes because the backup incorporates committed WAL.
    static func requiredFreeSpace(at url: URL) throws -> Int64 {
        var sourceBytes: Int64 = 0
        for suffix in ["", "-wal"] {
            let path = url.path + suffix
            if suffix.isEmpty || FileManager.default.fileExists(atPath: path) {
                let attributes = try FileManager.default.attributesOfItem(atPath: path)
                guard let size = (attributes[.size] as? NSNumber)?.int64Value, size >= 0 else {
                    throw SQLite.Failure.database("Cannot estimate migration storage requirements")
                }
                let sum = sourceBytes.addingReportingOverflow(size)
                guard !sum.overflow else { return Int64.max }
                sourceBytes = sum.partialValue
            }
        }
        let copiesAndJournals = sourceBytes.multipliedReportingOverflow(by: 4)
        let margin = max(64 * 1024 * 1024, sourceBytes / 2)
        let total = copiesAndJournals.partialValue.addingReportingOverflow(margin)
        return copiesAndJournals.overflow || total.overflow ? Int64.max : total.partialValue
    }

    private static func availableCapacity(at directory: URL) throws -> Int64 {
        try resolveAvailableCapacity(importantUsage: {
            try directory.resourceValues(forKeys: [.volumeAvailableCapacityForImportantUsageKey])
                .volumeAvailableCapacityForImportantUsage
        }, fileSystem: {
            let attributes = try FileManager.default.attributesOfFileSystem(forPath: directory.path)
            guard let capacity = (attributes[.systemFreeSize] as? NSNumber)?.int64Value else {
                throw SQLite.Failure.database("Cannot determine available storage; check device storage and retry")
            }
            return capacity
        })
    }

    /// Some macOS volumes report zero/negative or no ImportantUsage capacity
    /// despite free filesystem space. Confirm those results with a real free-byte
    /// query; never replace them with estimated savings or a test-only allowance.
    static func resolveAvailableCapacity(
        importantUsage: () throws -> Int64?, fileSystem: () throws -> Int64
    ) throws -> Int64 {
        if let capacity = try? importantUsage(), capacity > 0 { return capacity }
        return max(0, try fileSystem())
    }

    private static func needsBridge(_ source: Identity, plan: any SchemaMigrationPlan.Type) throws -> Bool {
        guard source.versions == ["1.0.0"] else { return false }
        for registered in plan.schemas where version(registered.versionIdentifier) == "1.0.0" {
            if source == (try identity(for: registered)) { return false }
        }
        return true
    }

    /// Cleanup is optional after an ordinary open. Never make known/current
    /// stores fail because a legacy opener holds the lock, and do not create a
    /// lock file unless there are actual attempt directories to reclaim.
    private static func reclaimAfterSuccessfulOpen(at url: URL, root: URL) {
        guard !attemptDirectories(at: root).isEmpty,
              let lock = try? StoreLock(url: url) else { return }
        defer { lock.close() }
        reclaimInactiveAttempts(at: root, holding: lock)
    }

    /// Requires exclusive ownership for both checking the marker and deleting
    /// copies. Call before a legacy attempt starts, or after a later ordinary
    /// open; the successful migration/recovery launch retains its own backup.
    private static func reclaimInactiveAttempts(at root: URL, holding _: StoreLock) {
        guard !FileManager.default.fileExists(atPath: root.appendingPathComponent("active.json").path) else { return }
        for directory in attemptDirectories(at: root) {
            // Failure only affects disk usage. Preflight measures the actual
            // remaining free capacity after these attempts, not estimated savings.
            try? FileManager.default.removeItem(at: directory)
        }
    }

    private static func attemptDirectories(at root: URL) -> [URL] {
        guard let entries = try? FileManager.default.contentsOfDirectory(
            at: root, includingPropertiesForKeys: [.isDirectoryKey, .isSymbolicLinkKey]) else { return [] }
        return entries.filter { entry in
            guard UUID(uuidString: entry.lastPathComponent) != nil,
                  let attributes = try? entry.resourceValues(forKeys: [.isDirectoryKey, .isSymbolicLinkKey]) else { return false }
            return attributes.isDirectory == true && attributes.isSymbolicLink != true
        }
    }

    private static func recoverIfNeeded(at url: URL, root: URL) throws {
        let marker = root.appendingPathComponent("active.json")
        guard FileManager.default.fileExists(atPath: marker.path) else { return }
        let journal = try JSONDecoder().decode(Journal.self, from: Data(contentsOf: marker))
        guard [1, 2].contains(journal.formatVersion) else {
            throw SQLite.Failure.unsupported("Unknown migration recovery format")
        }
        let directory = root.appendingPathComponent(journal.operation.uuidString, isDirectory: true)
        // SQLite commits or rolls back the entire page replacement, including
        // WAL recovery. We never restore a backup over potentially newer writes.
        let current = try identity(at: url)
        if current == journal.destination {
            try SQLite.integrityCheck(url)
            if journal.formatVersion == 2 {
                guard let expected = journal.destinationData,
                      try SQLite.evidence(at: url) == expected else {
                    throw SQLite.Failure.unsupported("Installed migration data differs from the validated candidate")
                }
            } else {
                // Older journals require their candidate as the data evidence.
                // Schema identity and SQLite integrity alone cannot prove preservation.
                let candidate = directory.appendingPathComponent("candidate.store")
                guard try identity(at: candidate) == journal.destination else {
                    throw SQLite.Failure.unsupported("Validated migration candidate is missing or has changed")
                }
                try SQLite.validatePreservation(from: candidate, to: url)
            }
        } else if current != journal.source {
            throw SQLite.Failure.unsupported("Store changed while migration recovery was pending")
        } else {
            // The original remains authoritative even if scratch copies were
            // removed. Validate it, clear the attempt, then take fresh copies.
            try SQLite.integrityCheck(url)
        }
        try clearJournal(at: root)
        if current == journal.source {
            // Promotion never committed; the original is still authoritative.
            try? FileManager.default.removeItem(at: directory)
        } else {
            try? FileManager.default.removeItem(at: directory.appendingPathComponent("candidate.store"))
        }
    }

    private static func identity(at url: URL) throws -> Identity {
        let metadata = try NSPersistentStoreCoordinator.metadataForPersistentStore(type: .sqlite, at: url)
        guard let versions = metadata[NSStoreModelVersionIdentifiersKey] as? [String],
              let checksum = metadata["NSStoreModelVersionChecksumKey"] as? String,
              let hashes = metadata[NSStoreModelVersionHashesKey] as? [String: Data],
              !versions.isEmpty, !checksum.isEmpty, !hashes.isEmpty else {
            throw SQLite.Failure.unsupported("Missing Core Data schema metadata")
        }
        return Identity(versions: versions, checksum: checksum, hashes: hashes)
    }

    private static func identity(for type: any VersionedSchema.Type) throws -> Identity {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let url = directory.appendingPathComponent("schema.store")
        try autoreleasepool {
            let schema = Schema(versionedSchema: type)
            _ = try ModelContainer(for: schema, configurations: [
                ModelConfiguration(schema: schema, url: url, cloudKitDatabase: .none)
            ])
        }
        return try identity(at: url)
    }

    private static func rejectExternalStorage(at url: URL) throws {
        let parent = url.deletingLastPathComponent()
        let names = Set([url.lastPathComponent, url.deletingPathExtension().lastPathComponent])
        for name in names {
            for companion in [".\(name)_SUPPORT", "\(name)_SUPPORT", "\(name).support", ".\(name).support"] {
                if FileManager.default.fileExists(atPath: parent.appendingPathComponent(companion).path) {
                    throw SQLite.Failure.unsupported("External binary storage requires a separate migration")
                }
            }
        }
    }

    private static func protectionAttributes(like original: URL) throws -> [FileAttributeKey: Any] {
        let attributes = try FileManager.default.attributesOfItem(atPath: original.path)
        var retained: [FileAttributeKey: Any] = [.posixPermissions: attributes[.posixPermissions] ?? 0o600]
        if let protection = attributes[.protectionKey] { retained[.protectionKey] = protection }
        return retained
    }
    private static func createProtectedFile(_ url: URL, like original: URL) throws {
        guard FileManager.default.createFile(atPath: url.path, contents: Data(),
                                             attributes: try protectionAttributes(like: original)) else {
            throw SQLite.Failure.database("Cannot create protected migration copy")
        }
    }

    private static func writeJournal(_ journal: Journal, at root: URL) throws {
        let url = root.appendingPathComponent("active.json")
        try JSONEncoder().encode(journal).write(to: url, options: .atomic)
        let file = try FileHandle(forWritingTo: url)
        defer { try? file.close() }
        try file.synchronize()
        try synchronizeDirectory(root)
    }
    private static func clearJournal(at root: URL) throws {
        try FileManager.default.removeItem(at: root.appendingPathComponent("active.json"))
        try synchronizeDirectory(root)
    }
    private static func synchronizeDirectory(_ url: URL) throws {
        let descriptor = Darwin.open(url.path, O_RDONLY)
        guard descriptor >= 0 else { throw SQLite.Failure.database("Cannot open migration directory") }
        defer { Darwin.close(descriptor) }
        guard fsync(descriptor) == 0 else { throw SQLite.Failure.database("Cannot persist migration journal") }
    }
    private static func version(_ value: Schema.Version) -> String {
        "\(value.major).\(value.minor).\(value.patch)"
    }

    private final class StoreLock {
        private var descriptor: Int32
        init(url: URL) throws {
            descriptor = Darwin.open(url.path + ".legacy-v2.lock", O_CREAT | O_RDWR | O_NOFOLLOW, 0o600)
            guard descriptor >= 0 else { throw SQLite.Failure.database("Cannot open database migration lock") }
            guard flock(descriptor, LOCK_EX | LOCK_NB) == 0 else {
                Darwin.close(descriptor)
                descriptor = -1
                throw SQLite.Failure.database("Another opener is using this database; retry after it finishes")
            }
        }
        func close() {
            if descriptor >= 0 { flock(descriptor, LOCK_UN); Darwin.close(descriptor); descriptor = -1 }
        }
        deinit { close() }
    }
}
