# Repository Guidelines

Guidance for AI coding agents (Claude Code, Codex, etc.) working in this repository.

## Project Structure & Module Organization
- Monorepo using Yarn workspaces and a Rust Cargo workspace.
- Source packages live in `packages/*` (JS/TS and Rust crates). Examples: `packages/js-dash-sdk`, `packages/rs-drive`, `packages/rs-dpp`.
- End-to-end tests and helpers: `packages/platform-test-suite`.
- Docs in `docs/`, scripts in `scripts/`, Docker config at repo root, local fixtures in `db/`.

## Architecture

### Technology Stack
- **Rust**: Core platform components (Drive, DAPI server, DPP implementation)
- **JavaScript/TypeScript**: Client SDKs, developer tools, test suite
- **WebAssembly**: Bridge between Rust and JavaScript implementations
- **gRPC**: Service communication protocol
- **Docker**: Local development environment

### Key Components
- **Drive** (`packages/rs-drive`): Platform's decentralized storage component, implementing a replicated state machine for storing and retrieving application data.
- **DAPI** (`packages/dapi`): Decentralized API server that provides a unified interface for interacting with the Dash network and Platform.
- **DPP** (`packages/rs-dpp`, `packages/wasm-dpp`): Dash Platform Protocol implementation that defines data structures and validation rules.
- **SDK** (`packages/js-evo-sdk`, `packages/rs-sdk`): Client libraries providing high-level interfaces for building applications on Dash Platform. `js-evo-sdk` is the recommended JavaScript SDK, built on the Rust/WASM layer with proof verification support. `packages/js-dash-sdk` is a legacy SDK used internally by the platform test suite and is not recommended for new projects. See [packages/rs-sdk/README.md](packages/rs-sdk/README.md) for the Rust SDK checklist for implementing new `Fetch`/`FetchMany` queries.
- **WASM SDK** (`packages/wasm-sdk`): WebAssembly bindings for browser-based applications. See [AI_REFERENCE.md](packages/wasm-sdk/AI_REFERENCE.md) for comprehensive API documentation.
- **Dashmate** (`packages/dashmate`): Node management tool for setting up and managing Dash Platform nodes.

### Data Contracts
Platform uses data contracts to define application data schemas:
- `dpns-contract`: Dash Platform Naming Service
- `dashpay-contract`: Social payments functionality
- `masternode-reward-shares-contract`: Masternode reward distribution
- `withdrawals-contract`: Platform credit withdrawals

### Development Workflow
1. **Monorepo Structure**: Uses Yarn workspaces to manage multiple packages
2. **Cross-language Integration**: WASM bindings connect Rust and JavaScript code
3. **Local Development**: Docker Compose environment managed by dashmate
4. **Testing**: Comprehensive test suites at unit, integration, and e2e levels
5. **WASM SDK Development**:
   - Build with `./build.sh` in `packages/wasm-sdk`
   - Test with web interface at `index.html`
   - Keep docs in sync: `python3 generate_docs.py`

### Important Patterns
- **Platform Versioning**: Uses `rs-platform-version` for protocol versioning
- **Serialization**: Custom serialization with `rs-platform-serialization`
- **Value Handling**: `rs-platform-value` for cross-language data representation
- **Proof Verification**: `rs-drive-proof-verifier` for cryptographic proofs
- **State Transitions**: Documents and data contracts use state transitions for updates

## Build, Test, and Development Commands

### Setup and Development
 Set up the agent environment: `bash scripts/setup-ai-agent-environment.sh`
- Initial setup (installs deps, builds, and configures): `yarn setup`
- Start local development environment: `yarn start`
- Stop local environment: `yarn stop`
- Restart services: `yarn restart`
- Rebuild after changes: `yarn build`
- Complete reset of data and builds: `yarn reset` (or `yarn run dashmate group reset --hard`, cautiously)
- Access dashmate CLI: `yarn dashmate`
- Test net config: `yarn configure:tests:network`

### Testing
- Run full test suite (requires running node from `yarn start`): `yarn test`
- Test specific suites: `yarn test:suite` (platform test suite), `yarn test:dapi` (DAPI components), `yarn test:sdk` (JavaScript SDK), `yarn test:dpp` (Dash Platform Protocol), `yarn test:drive` (Drive storage layer), `yarn test:wallet-lib` (wallet library), `yarn test:dapi-client` (DAPI client)
- Test specific workspace: `yarn workspace <package_name> test`

### Rust Development
- Run Rust tests for a specific package: `cargo test -p <package_name>`
- Run all Rust tests: `cargo test --workspace`
- Check Rust code: `cargo check --workspace`
- Run clippy linter: `cargo clippy --workspace`
- Format Rust code: `cargo fmt --all`

### Other Commands
- Run linters: `yarn lint`

## Coding Style & Naming Conventions
- Editor config: 2-space indent (4 for `*.rs`), LF, UTF‑8, final newline (`.editorconfig`).
- JS/TS: ESLint (Airbnb/TypeScript rules via package configs). Use camelCase for variables/functions, PascalCase for classes; prefer kebab-case filenames within JS packages.
- Rust: Follow rustfmt defaults; keep code clippy-clean. Modules `snake_case`, types `PascalCase`, constants `SCREAMING_SNAKE_CASE`.

## Testing Guidelines
- Unit/integration tests live alongside each package (e.g., `packages/<name>/tests`). E2E lives in `packages/platform-test-suite`.
- Name tests descriptively, starting with "should …".
- Unit/integration tests should not perform network calls; mock dependencies.
- Run targeted suites during development (examples above) and full `yarn test`/`cargo test --workspace` in CI.

## Commit & Pull Request Guidelines
- Conventional Commits for titles and commits: `<type>(scope): <description>` (e.g., `feat(sdk): add identity fetch`). Use `!` for breaking changes. Types are specified in `.github/workflows/pr.yml`.
- Keep PRs focused, link issues, include tests, and fill the PR template (`.github/PULL_REQUEST_TEMPLATE.md`).
- Branching: bugfixes and new features to the current `vX-dev` branch.

## Specs are working artifacts — never commit them
A design doc / spec / plan written to drive a change (the problem, chosen approach, alternatives rejected, failure modes, test plan) exists to guide the work, not to ship. It goes stale the moment the code lands.
- Keep it in the working tree but **never stage or commit it**. Enforce this by discipline, not git plumbing: stage with explicit paths (`git add <file> …`), never a blind `git add -A`/`git add .`. Do NOT edit `.git/info/exclude`, add a `.gitignore` entry, or use any other git trick to hide it — just don't add it.
- Delete it once the task ships. If a branch already committed one, `git rm --cached <path>` before merge; the file stays on disk for the life of the task.
- Only commit documentation the project keeps long-term: user/developer guides, architecture references, runbooks, API docs. A fact worth keeping after merge goes in a code comment, a test name, the commit message, or the PR description — not a design doc.

## Agent-Specific Instructions

**Always use the `swift-rust-ffi-engineer` agent for:**
- Any Swift/Rust FFI integration work
- Swift wrapper implementations over FFI functions
- Debugging Swift/FFI type compatibility issues
- iOS SDK and SwiftExampleApp development
- Memory management across Swift/Rust boundaries
- Refactoring Swift code to properly wrap FFI functions

## iOS Development

iOS/FFI artifacts live in `packages/rs-sdk-ffi` (Rust FFI layer) and the Swift app in `packages/swift-sdk`.

See [packages/swift-sdk/BUILD_GUIDE_FOR_AI.md](packages/swift-sdk/BUILD_GUIDE_FOR_AI.md) for detailed instructions on building the iOS components. For SwiftExampleApp-specific guidance including token querying and data models, see [packages/swift-sdk/SwiftExampleApp/CLAUDE.md](packages/swift-sdk/SwiftExampleApp/CLAUDE.md).

iOS Simulator MCP server: see [packages/swift-sdk/IOS_SIMULATOR_MCP.md](packages/swift-sdk/IOS_SIMULATOR_MCP.md) for Codex config, tools, and usage. Default output dir set via `IOS_SIMULATOR_MCP_DEFAULT_OUTPUT_DIR`.

Quick build commands:
```bash
# Build unified iOS framework (includes Core + Platform)
cd packages/swift-sdk
./build_ios.sh --target sim                  # release profile (the default)
./build_ios.sh --target sim --profile dev    # local iteration ONLY — debug
                                             # assertions abort the host app

# Build SwiftExampleApp
cd packages/swift-sdk
xcodebuild -project SwiftExampleApp/SwiftExampleApp.xcodeproj \
  -scheme SwiftExampleApp \
  -sdk iphonesimulator \
  -destination 'platform=iOS Simulator,name=iPhone 16,arch=arm64' \
  -quiet clean build
```

### iOS Architecture

**Unified SDK**: The iOS SDK combines both Core (SPV wallet) and Platform (identity/documents) functionality:
- Core SDK functions: `dash_core_sdk_*` prefix
- Platform SDK functions: `dash_sdk_*` prefix
- Unified SDK functions: `dash_unified_sdk_*` prefix

**SwiftExampleApp**: Demonstrates integration of both layers:
- Uses SwiftUI for UI and SwiftData for persistence
- `UnifiedAppState` coordinates Core and Platform features
- `WalletService` manages SPV wallet operations
- `PlatformService` handles identity and document operations

**Common iOS Build Issues**:
- Missing xcframework: Create symlink or update Package.swift
- Type visibility: Make DPP types public in Swift
- C header issues: Use pointers for opaque FFI types
- After merges: Always clean and rebuild from scratch

## Security & Configuration Tips
- Do not commit secrets; prefer local env setup via `scripts/configure_dotenv.sh`.
- When resetting local data, use `yarn reset` or `yarn run dashmate group reset --hard` cautiously.
