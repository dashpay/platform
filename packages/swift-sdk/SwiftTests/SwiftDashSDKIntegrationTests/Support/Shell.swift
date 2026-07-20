import Foundation

/// Subprocess wrapper used by the dashmate driver. Captures
/// stdout / stderr and the exit code.
enum Shell {
    struct Result {
        let exitCode: Int32
        let stdout: String
        let stderr: String

        var ok: Bool { exitCode == 0 }
    }

    enum Error: Swift.Error, CustomStringConvertible {
        case nonZeroExit(command: String, exitCode: Int32, stdout: String, stderr: String)
        case spawnFailed(command: String, underlying: Swift.Error)

        var description: String {
            switch self {
            case let .nonZeroExit(command, code, stdout, stderr):
                // yarn often surfaces failure context on stdout; include
                // both streams so the captured data isn't discarded.
                return """
                Shell command failed (exit \(code)): \(command)
                stdout: \(stdout)
                stderr: \(stderr)
                """
            case let .spawnFailed(command, err):
                return "Failed to spawn `\(command)`: \(err)"
            }
        }
    }

    /// Run `command args...` in `cwd`, inheriting the parent
    /// environment.
    static func run(
        _ command: String,
        _ arguments: [String] = [],
        cwd: URL? = nil,
        timeout: TimeInterval? = nil
    ) throws -> Result {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: command)
        process.arguments = arguments
        if let cwd { process.currentDirectoryURL = cwd }
        process.environment = ProcessInfo.processInfo.environment

        let stdoutPipe = Pipe()
        let stderrPipe = Pipe()
        process.standardOutput = stdoutPipe
        process.standardError = stderrPipe

        do {
            try process.run()
        } catch {
            throw Error.spawnFailed(
                command: "\(command) \(arguments.joined(separator: " "))",
                underlying: error
            )
        }

        if let timeout {
            let deadline = Date().addingTimeInterval(timeout)
            while process.isRunning && Date() < deadline {
                Thread.sleep(forTimeInterval: 0.05)
            }
            if process.isRunning {
                process.terminate()
                Thread.sleep(forTimeInterval: 0.5)
                if process.isRunning { kill(process.processIdentifier, SIGKILL) }
            }
        }

        process.waitUntilExit()

        let stdoutData = stdoutPipe.fileHandleForReading.readDataToEndOfFile()
        let stderrData = stderrPipe.fileHandleForReading.readDataToEndOfFile()

        return Result(
            exitCode: process.terminationStatus,
            stdout: String(data: stdoutData, encoding: .utf8) ?? "",
            stderr: String(data: stderrData, encoding: .utf8) ?? ""
        )
    }

    /// Like `run` but throws on non-zero exit.
    @discardableResult
    static func runChecked(
        _ command: String,
        _ arguments: [String] = [],
        cwd: URL? = nil,
        timeout: TimeInterval? = nil
    ) throws -> Result {
        let result = try run(command, arguments, cwd: cwd, timeout: timeout)
        guard result.ok else {
            throw Error.nonZeroExit(
                command: "\(command) \(arguments.joined(separator: " "))",
                exitCode: result.exitCode,
                stdout: result.stdout,
                stderr: result.stderr
            )
        }
        return result
    }
}
