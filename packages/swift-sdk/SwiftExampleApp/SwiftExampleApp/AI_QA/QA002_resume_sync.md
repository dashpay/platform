# QA002 – Pause/Resume Sync Loop

**Objective:** Validate that repeated pause/start cycles keep the sync pipeline progressing, never regress reported heights, and always restart syncing after each resume.

## Preconditions

- App launched in the simulator and sitting on the Wallets screen.
- A wallet exists. If the landing view shows a `Create Wallet` button, tap it and complete the default flow before proceeding.

## Steps

1. Tap the `Clear` button to reset sync state. Confirm headers drop back to their initial baseline (`1,100,000` range) via `ios-simulator__ui_describe_all` → `AI_QA/output/QA002_after_clear.json`.
2. Tap `Start Sync`. Wait 5 seconds for headers to begin advancing, then capture baseline metrics to `AI_QA/output/QA002_cycle0.json` and screenshot `AI_QA/output/QA002_cycle0.png`.
3. Run three pause/resume cycles:
   - For each cycle *n* (1…3):
     1. Tap `Pause`.
     2. Wait 2 seconds; dump accessibility to `AI_QA/output/QA002_cycle{n}_paused.json` and ensure the `Syncing:` banner disappears and `isSyncing` is false.
     3. Tap `Start`.
     4. Wait 5 seconds; dump accessibility to `AI_QA/output/QA002_cycle{n}_resumed.json` and take screenshot `AI_QA/output/QA002_cycle{n}_resumed.png`.
4. After the third cycle, let sync run an additional 10 seconds, then take a final accessibility dump `AI_QA/output/QA002_final.json` to confirm progress continues upward.

## Expected Results

- Step 1 clearing resets filter and filter-header cards to 0% while keeping the wallet present (no prompts to create/import).
- Step 2 baseline shows headers increasing from the starting window and the banner reading `Syncing: Headers` or any downstream phase.
- Every paused snapshot reports unchanged heights/percentages relative to the immediately preceding running state; no UI cards animate backward while paused.
- Every resumed snapshot shows `isSyncing` true, the banner re-appearing with the correct stage label, and at least one of the progress heights increasing versus the baseline (`QA002_cycle0.json`).
- Final snapshot confirms cumulative progress strictly increases across the three cycles (no regression in headers, filter headers, or filters heights).

Fail the QA if any cycle fails to restart sync, percentages regress, or the banner stage conflicts with the highest-progress card.
