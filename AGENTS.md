# Repository Guidelines

## Project Structure & Module Organization
- Monorepo using Yarn workspaces and a Rust Cargo workspace.
- Source packages live in `packages/*` (JS/TS and Rust crates). Examples: `packages/js-dash-sdk`, `packages/rs-drive`, `packages/rs-dpp`.
- End-to-end tests and helpers: `packages/platform-test-suite`.
- Docs in `docs/`, scripts in `scripts/`, Docker config at repo root, local fixtures in `db/`.

## Build, Test, and Development Commands
- Dependencies: `scripts/install-dependencies-ubuntu.sh`
- Setup: `yarn setup` (install, build, configure).
- Dev network: `yarn start` (start), `yarn stop`, `yarn restart`; dashmate CLI: `yarn dashmate`.
- Build all: `yarn build`.
- Lint all: `yarn lint`.
- JS/TS tests: `yarn test` or filtered suites (e.g., `yarn test:suite`, `yarn test:dapi`, `yarn workspace @dashevo/platform-test-suite test`).
- Rust tests: `cargo test --workspace` or `cargo test -p <crate>`.
- Rust checks: `cargo clippy --workspace`, format with `cargo fmt --all`.
- Test net config: `yarn configure:tests:network` (see `scripts/`).

## Coding Style & Naming Conventions
- Editor config: 2-space indent (4 for `*.rs`), LF, UTF‑8, final newline (`.editorconfig`).
- JS/TS: ESLint (Airbnb/TypeScript rules via package configs). Use camelCase for variables/functions, PascalCase for classes; prefer kebab-case filenames within JS packages.
- Rust: Follow rustfmt defaults; keep code clippy-clean. Modules `snake_case`, types `PascalCase`, constants `SCREAMING_SNAKE_CASE`.

## Testing Guidelines
- Unit/integration tests live alongside each package (e.g., `packages/<name>/tests`). E2E lives in `packages/platform-test-suite`.
- Name tests descriptively, starting with “should …”.
- Unit/integration tests should not perform network calls; mock dependencies.
- Run targeted suites during development (examples above) and full `yarn test`/`cargo test --workspace` in CI.

## Commit & Pull Request Guidelines
- Conventional Commits for titles and commits: `<type>(scope): <description>` (e.g., `feat(sdk): add identity fetch`). Use `!` for breaking changes.
- Keep PRs focused, link issues, include tests, and fill the PR template (`.github/PULL_REQUEST_TEMPLATE.md`).
- Branching: bugfixes and new features to the current `vX-dev` branch.

## Agent-Specific Instructions
- Use the `swift-rust-ffi-engineer` agent for all Swift/Rust FFI work, Swift wrappers, iOS SDK and SwiftExampleApp tasks, and Swift↔Rust type/memory debugging.

## Security & Configuration Tips
- Do not commit secrets; prefer local env setup via `scripts/configure_dotenv.sh`.
- When resetting local data, use `yarn reset` or `yarn run dashmate group reset --hard` cautiously.

## iOS Notes
- iOS/FFI artifacts: `packages/rs-sdk-ffi` and Swift app in `packages/swift-sdk`.
- Example: build iOS framework
  - `cd packages/rs-sdk-ffi && ./build_ios.sh`
 - iOS Simulator MCP server: see `packages/swift-sdk/IOS_SIMULATOR_MCP.md` for Codex config, tools, and usage. Default output dir set via `IOS_SIMULATOR_MCP_DEFAULT_OUTPUT_DIR`.
