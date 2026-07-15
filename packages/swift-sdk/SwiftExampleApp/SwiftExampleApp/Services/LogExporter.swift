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
/// (see `LoggingPreferences`). This exporter stages the launch's own
/// session (passed in by the caller — never inferred from timestamp
/// order, which a clock rollback or stale future-dated directory
/// could subvert) plus the newest few before it, adds a
/// `summary.txt` of build/device context, then zips the staging
/// directory. Nothing is uploaded — the caller hands the returned
/// file to a share sheet and the user decides where it goes.
struct LogExporter {
    /// Current run's session + up to two before it. After a crash
    /// the run that crashed is usually the session immediately
    /// before the current one, so this window covers "it just
    /// crashed, send logs" without any .ips parsing.
    static let maxSessions = 3

    /// Older sessions stop being added once the archive's raw input
    /// would pass this. The first selected session (the current one
    /// when known) is always included even if it alone is bigger.
    static let maxTotalBytes: UInt64 = 15 * 1024 * 1024

    struct SessionCandidate: Equatable {
        let url: URL
        let bytes: UInt64
    }

    /// Blocking (file I/O + compression) — call off the main actor.
    ///
    /// - Parameters:
    ///   - network: display string for `summary.txt`, captured by
    ///     the caller on the main actor.
    ///   - appVersion: ditto.
    ///   - currentSession: `LoggingPreferences.currentSessionDirectory`,
    ///     captured by the caller on the main actor. `nil` when file
    ///     logging didn't install this launch — the export then falls
    ///     back to newest-on-disk ordering and says so in the summary.
    static func export(
        network: String,
        appVersion: String,
        currentSession: URL?
    ) throws -> URL {
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

        let onDisk = entries.filter {
            (try? $0.resourceValues(forKeys: [.isDirectoryKey]))?.isDirectory == true
        }

        let currentOnDisk = currentSession.flatMap { session in
            onDisk.first { $0.lastPathComponent == session.lastPathComponent }
        }
        let current = currentOnDisk.map {
            SessionCandidate(url: $0, bytes: directorySize(of: $0))
        }
        let others = onDisk
            .filter { $0.lastPathComponent != currentOnDisk?.lastPathComponent }
            .map { SessionCandidate(url: $0, bytes: directorySize(of: $0)) }

        let selected = selectSessions(current: current, others: others)
        guard !selected.isEmpty else {
            throw LogExportError.noLogsFound
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
            currentIsKnown: current != nil,
            totalSessionsOnDevice: onDisk.count
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

    /// Pure selection policy, split out for unit testing.
    ///
    /// The current session (when known) is always first and always
    /// included, no matter how its timestamp sorts against the rest
    /// — it's the authoritative record of this run, not a candidate.
    /// Older sessions then fill the remaining slots newest-first
    /// until either cap is hit. The walk stops at the first session
    /// that would breach the byte cap rather than skipping past it:
    /// a gap in the middle of "the last three runs" would be more
    /// confusing than a shorter archive.
    static func selectSessions(
        current: SessionCandidate?,
        others: [SessionCandidate],
        maxSessions: Int = LogExporter.maxSessions,
        maxTotalBytes: UInt64 = LogExporter.maxTotalBytes
    ) -> [SessionCandidate] {
        var selected: [SessionCandidate] = []
        var totalBytes: UInt64 = 0

        if let current {
            selected.append(current)
            totalBytes = current.bytes
        }

        // Fixed-width UTC stamps: lexicographic == chronological.
        let newestFirst = others.sorted {
            $0.url.lastPathComponent > $1.url.lastPathComponent
        }
        for candidate in newestFirst {
            guard selected.count < maxSessions else { break }
            if !selected.isEmpty && totalBytes + candidate.bytes > maxTotalBytes { break }
            selected.append(candidate)
            totalBytes += candidate.bytes
        }
        return selected
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
        selected: [SessionCandidate],
        currentIsKnown: Bool,
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
            "OS: \(ProcessInfo.processInfo.operatingSystemVersionString)",
            "Hardware: \(model) (\(environment))",
            "Network: \(network)",
            "",
            "Each session directory's build_info.txt records the exact",
            "platform-wallet git commit that run was built from.",
            "",
            "Sessions included (one directory per app launch):",
        ]
        for (index, session) in selected.enumerated() {
            let role: String
            switch (index, currentIsKnown) {
            case (0, true): role = "current session"
            case (0, false):
                role = "newest session on disk (current session unknown — "
                    + "file logging was not active this launch)"
            case (1, _): role = "previous session — after a crash, the crashed run is usually this one"
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
