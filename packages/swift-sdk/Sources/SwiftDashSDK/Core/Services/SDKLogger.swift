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

    @discardableResult
    @MainActor
    public static func configure() -> LoggingPreset {
        let preset = loadPreset()
        let enableSwiftVerbose: Bool

        if let sessionRoot = launchLogPaths(),
           SDK.enableFileLogging(level: .info, sessionRoot: sessionRoot.path) {
            currentSessionDirectory = sessionRoot
            pruneOldSessions(keeping: sessionRoot)
        } else {
            SDK.enableLogging(level: .info)
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

    /// Delete session directories beyond `maxRetainedSessions`,
    /// oldest first. The current session is always kept regardless
    /// of where its stamp sorts. Runs detached — deleting a few
    /// multi-megabyte directories has no business on the main
    /// actor during launch.
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
            for stale in sessions.dropFirst(maxRetainedSessions)
            where stale.lastPathComponent != currentName {
                try? fm.removeItem(at: stale)
            }
        }
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
