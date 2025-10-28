# QA003 – Clear Sync Data Resets Progress

**Objective:** Ensure that clearing SPV storage returns all progress metrics to the baseline while preserving wallet listings.

## Preconditions

- App on Wallets screen with at least one wallet.
- Sync may be running or idle.

## Steps

1. Capture baseline state via `ios-simulator__ui_describe_all` (`QA003_before.json`).
2. Tap the red `Clear` button (`ios-simulator__ui_tap`).
3. Wait 3 seconds for the operation to finish.
4. Dump accessibility again to `QA003_after.json`.
5. Take a screenshot `QA003_after.png`.

## Expected Results

- Headers card shows `0% complete` or the configured checkpoint baseline percentage (should never exceed 1%). Heights reset to baseline values (e.g., `1,100,000/2,248,000`).
- Filter Headers card progress displays `0% complete`; numerator equals baseline.
- Filters card detail reads `Compact Filters: 0%`; numerator equals baseline.
- Blocks hit counter resets to `0`.
- Wallet list remains intact (wallet titled `New` still present).
- Start button turns blue (`Start` label) indicating sync is idle.

If any metric remains above the baseline, the clear routine is regressive – mark QA003 failed and attach artifacts.
