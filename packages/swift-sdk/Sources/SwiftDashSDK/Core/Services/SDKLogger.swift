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

    @discardableResult
    @MainActor
    public static func configure() -> LoggingPreset {
        let preset = loadPreset()
        let enableSwiftVerbose: Bool

        // File logging is gated to the iOS Simulator
        #if targetEnvironment(simulator)
        if let sessionRoot = launchLogPaths(),
           SDK.enableFileLogging(level: .info, sessionRoot: sessionRoot) {
        } else {
            SDK.enableLogging(level: .info)
        }
        #else
        SDK.enableLogging(level: .info)
        #endif

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

    private static func launchLogPaths() -> String? {
        guard
            let libraryURL = FileManager.default
                .urls(for: .libraryDirectory, in: .userDomainMask).first
        else {
            return nil
        }

        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = TimeZone(secondsFromGMT: 0)
        formatter.dateFormat = "yyyy-MM-dd'T'HH-mm-ss'Z'"

        return libraryURL
            .appendingPathComponent("Logs", isDirectory: true)
            .appendingPathComponent("SwiftDashSDK", isDirectory: true)
            .appendingPathComponent(formatter.string(from: Date()), isDirectory: true)
            .path
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
