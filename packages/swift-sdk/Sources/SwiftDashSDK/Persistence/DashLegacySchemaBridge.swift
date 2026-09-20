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
        guard FileManager.default.fileExists(atPath: url.path) else {
            if FileManager.default.fileExists(atPath: backupDirectory(for: url).appendingPathComponent("active.json").path) {
                throw SQLite.Failure.unsupported("Original database is missing while migration recovery is pending")
            }
            return try ordinary()
        }
        let root = backupDirectory(for: url)
        let marker = root.appendingPathComponent("active.json")
        if !FileManager.default.fileExists(atPath: marker.path) {
            // Detection must not impose bridge-specific metadata or locking
            // requirements on stores handled by SwiftData's ordinary path.
            let needsMigration = (try? identity(at: url)).flatMap { try? needsBridge($0, plan: plan) } ?? false
            // Recheck after reading identity: a concurrent bridge publishes its
            // journal before the store can change to the destination schema.
            if !needsMigration && !FileManager.default.fileExists(atPath: marker.path) {
                let container = try ordinary()
                reclaimCompletedBackups(at: root)
                return container
            }
        }
        let lock = try StoreLock(url: url)
        defer { lock.close() }
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
        // Clear before returning a live container. Recovery must never replace
        // a successfully opened store after the app has started writing to it.
        try clearJournal(at: root)
        try? FileManager.default.removeItem(at: candidate)
        return container
    }

    static func backupDirectory(for url: URL) -> URL {
        url.deletingLastPathComponent().appendingPathComponent(url.lastPathComponent + ".legacy-v2-backups", isDirectory: true)
    }

    private static func needsBridge(_ source: Identity, plan: any SchemaMigrationPlan.Type) throws -> Bool {
        guard source.versions == ["1.0.0"] else { return false }
        for registered in plan.schemas where version(registered.versionIdentifier) == "1.0.0" {
            if source == (try identity(for: registered)) { return false }
        }
        return true
    }

    /// A backup survives the migration/recovery launch. Reclaim it only after
    /// a later ordinary open succeeds and no migration journal is pending.
    /// Cleanup failure affects disk usage, never the ability to open the wallet.
    private static func reclaimCompletedBackups(at root: URL) {
        let marker = root.appendingPathComponent("active.json")
        guard !FileManager.default.fileExists(atPath: marker.path),
              let entries = try? FileManager.default.contentsOfDirectory(
                at: root, includingPropertiesForKeys: [.isDirectoryKey, .isSymbolicLinkKey]) else { return }
        for entry in entries where UUID(uuidString: entry.lastPathComponent) != nil {
            guard let attributes = try? entry.resourceValues(forKeys: [.isDirectoryKey, .isSymbolicLinkKey]),
                  attributes.isDirectory == true, attributes.isSymbolicLink != true else { continue }
            try? FileManager.default.removeItem(at: entry)
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
                throw SQLite.Failure.database("Another process is opening this database; retry after it finishes")
            }
        }
        func close() {
            if descriptor >= 0 { flock(descriptor, LOCK_UN); Darwin.close(descriptor); descriptor = -1 }
        }
        deinit { close() }
    }
}
