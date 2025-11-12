# QA001 – Wallet Sync Bars After Fresh Start

**Objective:** Validate that starting a sync from a clean state shows correct progress distribution: headers begin at the previously known baseline, filter headers & filters remain at 0%, and the banner announces the headers phase.

## Preconditions

- iOS simulator booted (e.g., iPhone 16 Pro iOS 18.6) with the latest `SwiftExampleApp` installed from the current workspace build.
- MCP simulator server running so agent commands are honoured.
- App launched to the Wallets tab showing an existing wallet (default fixture `New`). If no wallet exists, run `ios-simulator__ui_tap` on the plus button and follow the create-wallet flow before starting.

## Steps

1. Ensure Wallets screen is visible. If not, tap the `Wallets` tab bar item (`ios-simulator__ui_tap` at approx. `{x:60, y:1060}`).
2. Tap the `Clear` button in the Sync Status card (`ios-simulator__ui_tap` around the red button). Confirm any prompts if presented (currently none).
3. Wait 2 seconds for the clear routine to finish (`sleep 2`).
4. Tap the `Start` button (`ios-simulator__ui_tap` on the blue button). This should initiate syncing.
5. Monitor headers:
   - Poll `ios-simulator__ui_describe_all` every 10 seconds (maximum 60 seconds total).
   - As soon as the Headers card reports `> 0% complete`, capture `AI_QA/output/QA001_headers.png` (screenshot) and `QA001_headers.json` (describe dump).
6. Monitor filter headers:
   - Continue polling every 10 seconds until the banner reads `Syncing: Filter Headers` **or** the Filter Headers card shows `> 0% complete`.
   - Capture `QA001_filter_headers.png`/`QA001_filter_headers.json` on the first frame of this phase.
7. Optional masternode phase:
   - If the app is running in non-trusted mode and surfaces a separate “Masternode List” stage (card progress `> 0%` or banner change), capture `QA001_masternodes.png`/`QA001_masternodes.json`. Skip this step if the stage never appears.
8. Monitor filters:
   - Continue polling until the Filters card shows `> 0% complete` (banner `Syncing: Filters`).
   - Capture `QA001_filters.png`/`QA001_filters.json` at the first indication of filter progress.
9. Completion:
   - Poll until Filters reach `100%` or the banner reports `Syncing: Complete`.
   - Capture `QA001_complete.png`/`QA001_complete.json` as the terminal state.
10. Store all artifacts under `AI_QA/output/` and note timestamps for later comparison.

## Expected Results

For each captured phase:

- **Headers (`QA001_headers.*`)** – Headers card shows `0% < progress < 100%`; Filter Headers and Filters remain at `0%`. `Blocks hit: 0`. Height is `<baseline>/<tip>` where `baseline` matches the stored start height (e.g., `1,100,000/…`).
- **Filter Headers (`QA001_filter_headers.*`)** – Headers locked at `100%`; Filter Headers progressing (`0% < value < 100%`); Filters still `0%`.
- **Masternode (optional)** – Only present when non-trusted masternode sync is enabled. Progress should advance monotonically without affecting Filter Headers/Filters percentages.
- **Filters (`QA001_filters.*`)** – Filters card shows `0% < n < 100%`; Filter Headers remain fixed at their terminal value; Headers stay at `100%`.
- **Complete (`QA001_complete.*`)** – All cards display `100%`; banner reads `Syncing: Complete`.
- **Chain tip sanity** – For every capture with a height fraction `current/tip`, the denominator `tip` must stay within ±100,000 blocks of the expected tip `1332564 + 576 × daysSince(2025-09-24)`. Compute the expected tip via:
  ```sh
  python3 - <<'PY'
from datetime import date
today = date.today()
anchor = date(2025, 9, 24)
days = (today - anchor).days
expected_tip = 1332564 + 576 * max(days, 0)
print(expected_tip)
PY
  ```
  Flag the QA run as failed if any denominator falls outside this tolerance.

If any expectation fails, mark QA001 as failed and attach the artifacts plus notes about deviations.
