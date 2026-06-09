# SwiftExampleApp UI Tests

XCUITest suite that drives the SwiftExampleApp through real user flows on the iOS Simulator.

## Tests in the suite

- `SwiftExampleAppUITests.testCreateGeneratedWalletFlow` — generates a fresh wallet end-to-end (local-only).
- `WalletPersistenceTests.testWalletPersistsAcrossRelaunch` — wallet survives an app relaunch.
- `WalletPersistenceTests.testWalletDeletionCleanupSurvivesRelaunch` — deleted wallet stays gone after relaunch.
- `CreditTransferTest.testImportWalletAndDiscoverIdentity` — imports a known testnet mnemonic, runs DIP-9 identity discovery, asserts the registered identity has a non-zero credit balance. **Self-skips without `UI_TEST_MNEMONIC`.**

The first three are local-only and hermetic; the last hits public testnet DAPI.

## Running locally

### From Xcode

Pick the `SwiftExampleApp` scheme, run `Product → Test`. To pass the testnet mnemonic, edit the Test scheme's Environment Variables and add `UI_TEST_MNEMONIC` with your 12-word phrase. No `TEST_RUNNER_` prefix needed when set via the scheme — Xcode forwards it directly.

### From the command line

```bash
rm -rf /tmp/ui-tests.xcresult
TEST_RUNNER_UI_TEST_MNEMONIC="your 12 word phrase" \
xcodebuild test \
  -project packages/swift-sdk/SwiftExampleApp/SwiftExampleApp.xcodeproj \
  -scheme SwiftExampleApp \
  -destination 'platform=iOS Simulator,name=iPhone 17,arch=arm64' \
  -resultBundlePath /tmp/ui-tests.xcresult
```

**The `TEST_RUNNER_` prefix is mandatory on the command line.** `xcodebuild` strips it before forwarding the env var to the XCUITest runner process; without the prefix, `ProcessInfo.processInfo.environment["UI_TEST_MNEMONIC"]` returns nil and the test self-skips.

To target a single test, append `-only-testing:SwiftExampleAppUITests/<TestClass>/<testMethod>`.

## Simulator state hygiene

The suite expects a clean simulator. Two pre-existing-state failure modes you'll hit if you skip the wipe:

1. **SwiftData migration crash.** Old persistent stores from previous app builds may have schema rows incompatible with the current model (e.g. mandatory fields added). Symptom: app crashes on launch with `SwiftDataError._Error.loadIssueModelContainer`; the test times out at "Expected root tab bar".
2. **Orphan-mnemonic recovery prompt.** The iOS Keychain persists across app uninstalls — uninstalling alone won't clear it. If a previous run left mnemonics behind and the SwiftData store is fresh, the app pops a "Recover Wallet?" alert on launch. `failIfRecoveryPromptVisible` catches this loudly, but you can't proceed without resolving it.

Recommended reset before a fresh session:

```bash
udid=$(xcrun simctl list devices booted | awk '/iPhone/ {print $NF}' | tr -d '()' | head -1)
xcrun simctl shutdown "$udid"
xcrun simctl erase "$udid"
xcrun simctl boot "$udid"
xcrun simctl bootstatus "$udid" -b
```

`simctl erase` is the only way to clear leftover Keychain entries.

## CI

[`.github/workflows/swift-example-app-ui-smoke.yml`](../../../../.github/workflows/swift-example-app-ui-smoke.yml) runs all four tests on a self-hosted macOS ARM64 runner:

- **Manually** via `workflow_dispatch` (the "Run workflow" button on the workflow page).
- **Nightly** at 23:00 UTC.

The `UI_TEST_MNEMONIC` GitHub Actions secret must be set for `testImportWalletAndDiscoverIdentity` to actually exercise discovery; otherwise it self-skips. Fork PRs never receive secrets, so the discovery test always self-skips on forks (intentional).

The cron only fires when the self-hosted Mac is online — there's no GitHub-hosted macOS fallback. If two runs collide (e.g. a manual dispatch during the cron), the workflow's `concurrency:` block cancels the older one.
