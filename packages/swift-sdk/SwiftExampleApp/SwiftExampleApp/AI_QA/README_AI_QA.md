# AI QA Playbooks

This folder documents interactive QA scenarios that can be executed by an agent via the MCP simulator tooling. Each playbook outlines:

- **Objective** – what behaviour we want to validate.
- **Preconditions** – data or simulator state required before starting.
- **Steps** – deterministic actions the agent must perform using simulator APIs (tap, type, wait, screenshot, etc.).
- **Expected Results** – UI state and telemetry the agent should verify after the steps.
- **Artifacts** – screenshots or logs to capture for posterity.

## Running These Playbooks

1. Boot the iOS simulator MCP server from `packages/swift-sdk/IOS_SIMULATOR_MCP.md`.
2. Launch the `SwiftExampleApp` build that you just produced (`xcodebuild ... build`). Keep the simulator in the foreground.
3. Execute each playbook in order. Every step references explicit MCP commands such as `ios-simulator__ui_tap`, `ios-simulator__ui_type`, `ios-simulator__screenshot`, and `ios-simulator__ui_describe_all` so the agent can translate them to tool invocations without additional guidance.
4. Compare the observed state with the expectations. If a mismatch occurs, attach the collected artifacts and open an issue.

## Available Playbooks

- [QA001 – Wallet Sync Bars After Fresh Start](QA001_wallet_sync_progress.md)
- [QA002 – Resume Sync After Pause](QA002_resume_sync.md)
- [QA003 – Clear Sync Data Resets Progress](QA003_clear_sync_resets.md)

Add more scenarios as regressions are discovered or new flows require coverage. Follow the structure used in the existing playbooks.
