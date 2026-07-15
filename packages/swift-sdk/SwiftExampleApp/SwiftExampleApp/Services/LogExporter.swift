import Foundation
import SwiftDashSDK

enum LogExportError: LocalizedError {
    case noLogsFound
    case zipFailed(String)

    var errorDescription: String? {
        switch self {
        case .noLogsFound:
            return "No SDK log sessions found on this device. "
                + "Logs are written from the next launch after installing "
                + "a build with file logging enabled."
        case .zipFailed(let reason):
            return "Could not create the log archive: \(reason)"
        }
    }
}

/// Bundles recent SwiftDashSDK log sessions into a shareable zip.
///
/// Each app launch writes one session directory of per-crate
/// `run.log` files under `Library/Logs/SwiftDashSDK/<timestamp>/`
/// (see `LoggingPreferences`). This exporter stages the newest few
/// sessions plus a `summary.txt` of build/device context, then zips
/// the staging directory. Nothing is uploaded — the caller hands the
/// returned file to a share sheet and the user decides where it goes.
struct LogExporter {
    /// Newest session (current run) + up to two before it. After a
    /// crash the run that crashed is usually the session immediately
    /// before the current one, so this window covers "it just
    /// crashed, send logs" without any .ips parsing.
    static let maxSessions = 3

    /// Older sessions stop being added once the archive's raw input
    /// would pass this. The newest session is always included even
    /// if it alone is bigger.
    static let maxTotalBytes: UInt64 = 15 * 1024 * 1024

    struct SelectedSession {
        let url: URL
        let bytes: UInt64
    }

    /// Blocking (file I/O + compression) — call off the main actor.
    /// `network` and `appVersion` are display strings captured by the
    /// caller on the main actor; they only feed `summary.txt`.
    static func export(network: String, appVersion: String) throws -> URL {
        let fm = FileManager.default

        guard let root = LoggingPreferences.logsRootDirectory,
              let entries = try? fm.contentsOfDirectory(
                  at: root,
                  includingPropertiesForKeys: [.isDirectoryKey],
                  options: [.skipsHiddenFiles]
              )
        else {
            throw LogExportError.noLogsFound
        }

        // Fixed-width UTC stamps: lexicographic == chronological.
        let sessions = entries
            .filter { (try? $0.resourceValues(forKeys: [.isDirectoryKey]))?.isDirectory == true }
            .sorted { $0.lastPathComponent > $1.lastPathComponent }

        guard !sessions.isEmpty else {
            throw LogExportError.noLogsFound
        }

        var selected: [SelectedSession] = []
        var totalBytes: UInt64 = 0
        for session in sessions.prefix(maxSessions) {
            let bytes = directorySize(of: session)
            if !selected.isEmpty && totalBytes + bytes > maxTotalBytes { break }
            selected.append(SelectedSession(url: session, bytes: bytes))
            totalBytes += bytes
        }

        let stamp = selected[0].url.lastPathComponent
        let archiveName = "SwiftDashSDK-logs-\(stamp)"
        let scratch = fm.temporaryDirectory
            .appendingPathComponent("LogExport-\(UUID().uuidString)", isDirectory: true)
        let staging = scratch.appendingPathComponent(archiveName, isDirectory: true)
        defer { try? fm.removeItem(at: scratch) }

        try fm.createDirectory(at: staging, withIntermediateDirectories: true)
        for session in selected {
            try fm.copyItem(
                at: session.url,
                to: staging.appendingPathComponent(session.url.lastPathComponent, isDirectory: true)
            )
        }

        let summary = summaryText(
            network: network,
            appVersion: appVersion,
            selected: selected,
            totalSessionsOnDevice: sessions.count
        )
        try summary.write(
            to: staging.appendingPathComponent("summary.txt"),
            atomically: true,
            encoding: .utf8
        )

        let zipURL = fm.temporaryDirectory.appendingPathComponent("\(archiveName).zip")
        if fm.fileExists(atPath: zipURL.path) {
            try? fm.removeItem(at: zipURL)
        }
        try zipDirectory(at: staging, to: zipURL)
        return zipURL
    }

    /// Zip without third-party dependencies: a coordinated read with
    /// `.forUploading` makes the system produce a zip of a directory
    /// in a temporary location that is only valid inside the
    /// accessor, so the copy to `destination` happens in the block.
    private static func zipDirectory(at source: URL, to destination: URL) throws {
        var coordinatorError: NSError?
        var copyError: Error?
        NSFileCoordinator().coordinate(
            readingItemAt: source,
            options: .forUploading,
            error: &coordinatorError
        ) { zippedURL in
            do {
                try FileManager.default.copyItem(at: zippedURL, to: destination)
            } catch {
                copyError = error
            }
        }
        if let coordinatorError {
            throw LogExportError.zipFailed(coordinatorError.localizedDescription)
        }
        if let copyError {
            throw LogExportError.zipFailed(copyError.localizedDescription)
        }
    }

    private static func directorySize(of directory: URL) -> UInt64 {
        guard let enumerator = FileManager.default.enumerator(
            at: directory,
            includingPropertiesForKeys: [.totalFileAllocatedSizeKey, .fileSizeKey]
        ) else { return 0 }

        var total: UInt64 = 0
        for case let file as URL in enumerator {
            let values = try? file.resourceValues(
                forKeys: [.totalFileAllocatedSizeKey, .fileSizeKey]
            )
            total += UInt64(values?.totalFileAllocatedSize ?? values?.fileSize ?? 0)
        }
        return total
    }

    private static func summaryText(
        network: String,
        appVersion: String,
        selected: [SelectedSession],
        totalSessionsOnDevice: Int
    ) -> String {
        let iso = ISO8601DateFormatter()
        let sizeFormatter = ByteCountFormatter()
        sizeFormatter.countStyle = .file

        var machine = utsname()
        uname(&machine)
        let model = withUnsafeBytes(of: &machine.machine) { raw in
            String(decoding: raw.prefix(while: { $0 != 0 }), as: UTF8.self)
        }
        #if targetEnvironment(simulator)
        let environment = "simulator"
        #else
        let environment = "device"
        #endif

        var lines: [String] = [
            "SwiftDashSDK diagnostic log export",
            "Generated: \(iso.string(from: Date()))",
            "",
            "App version: \(appVersion)",
            "Git commit: \(AppVersion.gitCommit)",
            "OS: \(ProcessInfo.processInfo.operatingSystemVersionString)",
            "Hardware: \(model) (\(environment))",
            "Network: \(network)",
            "",
            "Sessions included (newest first, one directory per app launch):",
        ]
        for (index, session) in selected.enumerated() {
            let role: String
            switch index {
            case 0: role = "current session"
            case 1: role = "previous session — after a crash, the crashed run is usually this one"
            default: role = "older session"
            }
            lines.append(
                "  \(session.url.lastPathComponent)  "
                    + "[\(sizeFormatter.string(fromByteCount: Int64(session.bytes)))] — \(role)"
            )
        }
        lines.append("")
        lines.append(
            "Included \(selected.count) of \(totalSessionsOnDevice) session(s) on this "
                + "\(environment) (limit: \(maxSessions) sessions / "
                + "\(sizeFormatter.string(fromByteCount: Int64(maxTotalBytes))) raw)."
        )
        return lines.joined(separator: "\n") + "\n"
    }
}
