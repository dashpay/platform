import Foundation

// MARK: - Logging Preferences

public enum LoggingPreset: String {
    case low
    case medium
    case high

    var priority: Int {
        switch self {
        case .low: return 0
        case .medium: return 1
        case .high: return 2
        }
    }

    func allows(_ threshold: LoggingPreset) -> Bool {
        priority >= threshold.priority
    }
}

public enum LoggingPreferences {
    private static let defaultsKey = "SwiftSDKLogLevel"

    /// Sessions beyond this count are deleted at startup, oldest
    /// first. File logging runs on devices too (beta diagnostics),
    /// so growth must stay bounded without anyone thinking about it.
    private static let maxRetainedSessions = 20

    /// Total-bytes quota across retained sessions. The Rust side
    /// appends without rotation, so individual sessions can be
    /// arbitrarily large — a count cap alone doesn't bound disk
    /// use. Sessions are kept newest-first until the quota is hit;
    /// the current session is always kept (its in-flight growth
    /// can only be bounded by rotation on the Rust side).
    private static let maxRetainedBytes: UInt64 = 100 * 1024 * 1024

    /// Root under which each launch creates one timestamped session
    /// directory of per-crate `run.log` files. Exposed so diagnostics
    /// features (log export) can enumerate sessions without
    /// re-deriving the layout.
    public static var logsRootDirectory: URL? {
        FileManager.default
            .urls(for: .libraryDirectory, in: .userDomainMask).first?
            .appendingPathComponent("Logs", isDirectory: true)
            .appendingPathComponent("SwiftDashSDK", isDirectory: true)
    }

    /// Session directory of the current launch, set once `configure()`
    /// installs the file subscriber. `nil` when file logging is off
    /// (subscriber already installed, or the path wasn't writable).
    @MainActor
    public private(set) static var currentSessionDirectory: URL?

    /// The tracing subscriber is process-global and first-init-wins,
    /// so the install must run at most once per process. Without
    /// this guard, every `configure()` call after the first (e.g.
    /// a bootstrap retry) would make the Rust initializer lay out a
    /// fresh session directory of empty log files before `try_init`
    /// discovers the existing subscriber and bails — leaving decoy
    /// "newest" sessions that logging never writes to.
    @MainActor
    private static var didInstallLogging = false

    @discardableResult
    @MainActor
    public static func configure() -> LoggingPreset {
        let preset = loadPreset()
        let enableSwiftVerbose: Bool

        if !didInstallLogging {
            didInstallLogging = true
            if let sessionRoot = launchLogPaths(),
               SDK.enableFileLogging(level: .info, sessionRoot: sessionRoot.path) {
                currentSessionDirectory = sessionRoot
                pruneOldSessions(keeping: sessionRoot)
            } else {
                SDK.enableLogging(level: .info)
            }
        }

        switch preset {
        case .high:
            enableSwiftVerbose = true
        case .medium:
            enableSwiftVerbose = false
        case .low:
            enableSwiftVerbose = false
        }

        setenv("SPV_SWIFT_LOG", enableSwiftVerbose ? "1" : "0", 1)

        return preset
    }

    private static func launchLogPaths() -> URL? {
        guard let root = logsRootDirectory else { return nil }

        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = TimeZone(secondsFromGMT: 0)
        formatter.dateFormat = "yyyy-MM-dd'T'HH-mm-ss'Z'"

        return root
            .appendingPathComponent(formatter.string(from: Date()), isDirectory: true)
    }

    /// Delete session directories past either retention bound —
    /// `maxRetainedSessions` count or `maxRetainedBytes` total —
    /// walking newest-first so what survives is always the most
    /// recent history. The current session is always kept
    /// regardless of where its stamp sorts or how big it is.
    /// Runs detached — deleting multi-megabyte directories has no
    /// business on the main actor during launch.
    private static func pruneOldSessions(keeping current: URL) {
        guard let root = logsRootDirectory else { return }
        Task.detached(priority: .utility) {
            let fm = FileManager.default
            guard let entries = try? fm.contentsOfDirectory(
                at: root,
                includingPropertiesForKeys: [.isDirectoryKey],
                options: [.skipsHiddenFiles]
            ) else { return }

            // Session stamps are fixed-width UTC ISO timestamps, so
            // lexicographic order is chronological order.
            let sessions = entries
                .filter { (try? $0.resourceValues(forKeys: [.isDirectoryKey]))?.isDirectory == true }
                .sorted { $0.lastPathComponent > $1.lastPathComponent }

            let currentName = current.lastPathComponent
            var keptCount = 0
            var keptBytes: UInt64 = 0
            for session in sessions {
                if session.lastPathComponent == currentName {
                    // Counted against the byte quota so a huge
                    // just-finished-syncing session pushes old
                    // history out, but never deleted itself.
                    keptCount += 1
                    keptBytes += directoryBytes(of: session)
                    continue
                }
                let bytes = directoryBytes(of: session)
                if keptCount >= maxRetainedSessions
                    || keptBytes + bytes > maxRetainedBytes {
                    try? fm.removeItem(at: session)
                } else {
                    keptCount += 1
                    keptBytes += bytes
                }
            }
        }
    }

    private static func directoryBytes(of directory: URL) -> UInt64 {
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

    public static var preset: LoggingPreset { loadPreset() }

    public static var shouldEmitDefaultLogs: Bool { preset == .high }

    public static func allows(_ threshold: LoggingPreset) -> Bool {
        preset.allows(threshold)
    }

    private static func loadPreset() -> LoggingPreset {
        if let stored = UserDefaults.standard.string(forKey: defaultsKey)?.lowercased(),
           let preset = LoggingPreset(rawValue: stored) {
            return preset
        }
        return .low
    }
}

public enum SDKLogger {
    public static func log(_ message: String, minimumLevel level: LoggingPreset = .medium) {
        guard LoggingPreferences.allows(level) else { return }
        // Mirror to NSLog (unified logging) in addition to stdout so
        // `xcrun simctl spawn booted log stream` and Console.app see
        // the message even when no Xcode debugger is attached. The
        // `print` path is preserved because the dev loop still wants
        // stdout for in-Xcode use; NSLog goes to os_log.
        NSLog("%@", message)
        Swift.print(message)
    }

    public static func error(_ message: String) {
        // Route through both `NSLog` (unified log — Console.app, device
        // console, Xcode debug area without depending on stdout
        // capture) and `Swift.print` (stdout — preserves the existing
        // dev-loop behaviour where `print` output is what's visible).
        // Errors are rare; double-emit is fine and makes them harder
        // to miss when something does go wrong.
        NSLog("%@", message)
        Swift.print(message)
    }
}

// Package-internal override of `print` that honors the logging preference.
// Kept non-public so it only applies within SwiftDashSDK.
func print(_ items: Any..., separator: String = " ", terminator: String = "\n") {
    let output = items.map { String(describing: $0) }.joined(separator: separator)
    let lowercased = output.lowercased()
    let shouldAlwaysPrint = output.contains("❌") || output.contains("⚠️") || lowercased.contains("error")

    guard LoggingPreferences.shouldEmitDefaultLogs || shouldAlwaysPrint else { return }
    Swift.print(output, terminator: terminator)
}
