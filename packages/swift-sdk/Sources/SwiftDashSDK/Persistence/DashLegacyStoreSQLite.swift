import CryptoKit
import Foundation
import SQLite3

/// SQLite operations used only before a legacy store is opened by SwiftData.
/// The backup API copies committed WAL content and promotes a candidate in one
/// SQLite transaction; it never renames or deletes journal sidecars.
enum DashLegacyStoreSQLite {
    enum Failure: Error, LocalizedError {
        case database(String)
        case unsupported(String)
        case sourceChanged

        var errorDescription: String? {
            switch self {
            case .database(let reason): return "Legacy database migration failed: \(reason)"
            case .unsupported(let reason): return "Legacy database migration is not safe: \(reason)"
            case .sourceChanged: return "The database changed during migration. Close other users of the store and retry."
            }
        }
    }

    final class Connection {
        let handle: OpaquePointer
        init(_ url: URL, writable: Bool, create: Bool = false) throws {
            var result: OpaquePointer?
            let flags = writable ? SQLITE_OPEN_READWRITE | (create ? SQLITE_OPEN_CREATE : 0) : SQLITE_OPEN_READONLY
            let status = sqlite3_open_v2(url.path, &result, flags | SQLITE_OPEN_FULLMUTEX, nil)
            guard status == SQLITE_OK, let result else {
                let reason = result.map { String(cString: sqlite3_errmsg($0)) } ?? "Cannot open SQLite store"
                sqlite3_close(result)
                throw Failure.database(reason)
            }
            handle = result
            sqlite3_busy_timeout(handle, 0)
        }
        deinit { sqlite3_close(handle) }
        func execute(_ sql: String) throws {
            guard sqlite3_exec(handle, sql, nil, nil, nil) == SQLITE_OK else {
                throw Failure.database(String(cString: sqlite3_errmsg(handle)))
            }
        }
        func query(_ sql: String, _ row: (OpaquePointer) throws -> Void) throws {
            var statement: OpaquePointer?
            guard sqlite3_prepare_v2(handle, sql, -1, &statement, nil) == SQLITE_OK, let statement else {
                throw Failure.database(String(cString: sqlite3_errmsg(handle)))
            }
            defer { sqlite3_finalize(statement) }
            var status = sqlite3_step(statement)
            while status == SQLITE_ROW {
                try row(statement)
                status = sqlite3_step(statement)
            }
            guard status == SQLITE_DONE else { throw Failure.database(String(cString: sqlite3_errmsg(handle))) }
        }
    }

    static func rawDigest(_ url: URL) throws -> [String: String] {
        var result: [String: String] = [:]
        for suffix in ["", "-wal"] {
            let path = URL(fileURLWithPath: url.path + suffix)
            guard FileManager.default.fileExists(atPath: path.path) else { continue }
            let file = try FileHandle(forReadingFrom: path)
            defer { try? file.close() }
            var digest = SHA256()
            while let chunk = try file.read(upToCount: 1_048_576), !chunk.isEmpty { digest.update(data: chunk) }
            result[suffix] = hex(digest.finalize())
        }
        guard result[""] != nil else { throw Failure.database("Store file is missing") }
        return result
    }

    static func copy(from source: URL, to destination: URL,
                     lockedDestinationCheck: (() throws -> Void)? = nil) throws {
        let input = try Connection(source, writable: false)
        let output = try Connection(destination, writable: true, create: lockedDestinationCheck == nil)
        // C backup handles borrow both Swift connection owners, including on errors.
        try withExtendedLifetime((input, output)) {
            guard let backup = sqlite3_backup_init(output.handle, "main", input.handle, "main") else {
                throw Failure.database(String(cString: sqlite3_errmsg(output.handle)))
            }
            var finished = false
            defer { if !finished { sqlite3_backup_finish(backup) } }
            // Zero pages still acquires the destination write lock. No destination
            // connection APIs may run until backup_finish; raw file reads are safe.
            guard sqlite3_backup_step(backup, 0) == SQLITE_OK else {
                throw Failure.database("Database is busy; cannot acquire migration write lock")
            }
            try lockedDestinationCheck?()
            guard sqlite3_backup_step(backup, -1) == SQLITE_DONE else {
                throw Failure.database("SQLite could not complete the transactional database copy")
            }
            let status = sqlite3_backup_finish(backup)
            finished = true
            guard status == SQLITE_OK else { throw Failure.database("SQLite could not commit the database copy") }
        }
    }

    static func recoverRollbackJournal(at url: URL) throws {
        guard FileManager.default.fileExists(atPath: url.path + "-journal") else { return }
        // A process killed during SQLite promotion can leave a hot rollback
        // journal. A writable SQLite read recovers it before metadata inspection.
        let connection = try Connection(url, writable: true)
        try connection.query("PRAGMA schema_version") { _ in }
    }

    static func integrityCheck(_ url: URL) throws {
        let connection = try Connection(url, writable: false)
        var answers: [String] = []
        try connection.query("PRAGMA quick_check") { answers.append(string($0, 0)) }
        guard answers == ["ok"] else { throw Failure.unsupported("SQLite integrity check failed") }
    }

    static func checkpoint(_ url: URL) throws {
        let connection = try Connection(url, writable: true)
        guard sqlite3_wal_checkpoint_v2(connection.handle, nil, SQLITE_CHECKPOINT_TRUNCATE, nil, nil) == SQLITE_OK else {
            throw Failure.database("Cannot close the migrated WAL")
        }
        var modes: [String] = []
        try connection.query("PRAGMA journal_mode=DELETE") { modes.append(string($0, 0).lowercased()) }
        guard modes == ["delete"] else {
            throw Failure.database("Migrated store did not leave WAL mode")
        }
    }

    struct Column: Equatable {
        let actual: String
        let normalized: String
        let declaredType: String
    }
    struct Table {
        let actual: String
        let columns: [String: Column]
    }
    struct Layout {
        let entities: [Int64: String]
        let tables: [String: Table]
    }

    struct TableEvidence: Codable, Equatable {
        let columns: [String: String]
        let rowsDigest: String
    }
    struct StoreEvidence: Codable, Equatable {
        let entities: [String]
        let tables: [String: TableEvidence]
    }

    /// Durable evidence of the validated final candidate, independent of
    /// SQLite page layout, WAL state and disposable migration copy files.
    static func evidence(at url: URL) throws -> StoreEvidence {
        let connection = try Connection(url, writable: false)
        let layout = try layout(connection)
        var tables: [String: TableEvidence] = [:]
        for (name, table) in layout.tables {
            tables[name] = TableEvidence(
                columns: table.columns.mapValues(\.declaredType),
                rowsDigest: try rowsDigest(connection, table: table,
                                           names: table.columns.keys.sorted(), entities: layout.entities))
        }
        return StoreEvidence(entities: layout.entities.values.sorted(), tables: tables)
    }

    /// Compare every original application column and typed cell, including
    /// relationship foreign keys/join rows. Extra destination columns/tables
    /// are allowed; removals, type conversions and changed values are not.
    static func validatePreservation(from source: URL, to destination: URL) throws {
        let old = try Connection(source, writable: false)
        let new = try Connection(destination, writable: false)
        let before = try layout(old)
        let after = try layout(new)
        guard Set(before.entities.values).isSubset(of: Set(after.entities.values)) else {
            throw Failure.unsupported("Migration removed an entity")
        }
        for (name, table) in before.tables {
            guard let next = after.tables[name] else { throw Failure.unsupported("Migration removed table \(name)") }
            let names = table.columns.keys.sorted()
            for name in names {
                guard let nextColumn = next.columns[name], nextColumn.declaredType == table.columns[name]?.declaredType else {
                    throw Failure.unsupported("Migration removed or converted a column in \(table.actual)")
                }
            }
            let oldDigest = try rowsDigest(old, table: table, names: names, entities: before.entities)
            let newDigest = try rowsDigest(new, table: next, names: names, entities: after.entities)
            guard oldDigest == newDigest else { throw Failure.unsupported("Migration changed existing data in \(name)") }
        }
    }

    private static func layout(_ connection: Connection) throws -> Layout {
        var entities: [Int64: String] = [:]
        try connection.query("SELECT Z_ENT, Z_NAME FROM Z_PRIMARYKEY") {
            entities[sqlite3_column_int64($0, 0)] = string($0, 1).uppercased()
        }
        guard !entities.isEmpty else { throw Failure.unsupported("Missing Core Data entity map") }
        // Core Data metadata and persistent-history bookkeeping are not model
        // rows. SwiftData may rebuild them while inferring a lightweight map.
        let internalTables: Set<String> = ["Z_METADATA", "Z_MODELCACHE", "Z_PRIMARYKEY", "ACHANGE", "ATRANSACTION", "ATRANSACTIONSTRING"]
        var tableNames: [String] = []
        try connection.query("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'") {
            let name = string($0, 0)
            if !internalTables.contains(name) { tableNames.append(name) }
        }
        var tables: [String: Table] = [:]
        for tableName in tableNames {
            let normalized = try normalize(tableName, entities: entities)
            guard tables[normalized] == nil else { throw Failure.unsupported("Ambiguous Core Data table name") }
            var columns: [String: Column] = [:]
            try connection.query("PRAGMA table_info(\(quote(tableName)))") {
                let name = string($0, 1)
                // Optimistic-lock revision changes are not application values.
                guard name != "Z_OPT" else { return }
                let key = try normalize(name, entities: entities)
                guard columns[key] == nil else { throw Failure.unsupported("Ambiguous Core Data column name") }
                columns[key] = Column(actual: name, normalized: key, declaredType: string($0, 2).uppercased())
            }
            tables[normalized] = Table(actual: tableName, columns: columns)
        }
        return Layout(entities: entities, tables: tables)
    }

    private static func normalize(_ name: String, entities: [Int64: String]) throws -> String {
        guard name.hasPrefix("Z_") else { return name }
        let suffix = name.dropFirst(2)
        let digits = suffix.prefix(while: { $0.isNumber })
        guard !digits.isEmpty else { return name }
        guard let number = Int64(digits), let entity = entities[number] else {
            throw Failure.unsupported("Unknown entity ordinal in relationship table")
        }
        return "Z_" + entity + "_" + suffix.dropFirst(digits.count)
    }

    private static func rowsDigest(_ connection: Connection, table: Table, names: [String],
                                   entities: [Int64: String]) throws -> String {
        let columns = names.compactMap { table.columns[$0] }
        let selected = columns.map { quote($0.actual) }.joined(separator: ",")
        let ordering = table.columns["Z_PK"].map { quote($0.actual) } ?? selected
        var digest = SHA256()
        var count: UInt64 = 0
        try connection.query("SELECT \(selected) FROM \(quote(table.actual)) ORDER BY \(ordering)") { row in
            count += 1
            for (offset, column) in columns.enumerated() {
                let index = Int32(offset)
                let type = sqlite3_column_type(row, index)
                digest.update(data: Data([UInt8(type)]))
                var data = Data()
                switch type {
                case SQLITE_INTEGER:
                    let value = sqlite3_column_int64(row, index)
                    if column.actual == "Z_ENT" {
                        guard let entity = entities[value] else { throw Failure.unsupported("Unknown row entity ordinal") }
                        data = Data(entity.utf8)
                    } else {
                        var bits = value.bigEndian
                        data = withUnsafeBytes(of: &bits) { Data($0) }
                    }
                case SQLITE_FLOAT:
                    var bits = sqlite3_column_double(row, index).bitPattern.bigEndian
                    data = withUnsafeBytes(of: &bits) { Data($0) }
                case SQLITE_TEXT, SQLITE_BLOB:
                    let size = Int(sqlite3_column_bytes(row, index))
                    let bytes = type == SQLITE_TEXT ? sqlite3_column_text(row, index).map(UnsafeRawPointer.init) : sqlite3_column_blob(row, index)
                    if size > 0, let bytes { data = Data(bytes: bytes, count: size) }
                case SQLITE_NULL: break
                default: throw Failure.unsupported("Unknown SQLite value type")
                }
                var length = UInt64(data.count).bigEndian
                digest.update(data: withUnsafeBytes(of: &length) { Data($0) })
                digest.update(data: data)
            }
        }
        var rows = count.bigEndian
        digest.update(data: withUnsafeBytes(of: &rows) { Data($0) })
        return hex(digest.finalize())
    }

    private static func quote(_ name: String) -> String { "\"" + name.replacingOccurrences(of: "\"", with: "\"\"") + "\"" }
    private static func string(_ statement: OpaquePointer, _ column: Int32) -> String {
        guard let text = sqlite3_column_text(statement, column) else { return "" }
        return String(decoding: UnsafeBufferPointer(start: text, count: Int(sqlite3_column_bytes(statement, column))), as: UTF8.self)
    }
    private static func hex<D: Sequence>(_ data: D) -> String where D.Element == UInt8 {
        data.map { String(format: "%02x", $0) }.joined()
    }
}
