# QA004 – Reclaim an Unclaimed Invitation

**Objective:** Validate the inviter-side reclaim flow: an unclaimed invitation voucher can be recovered as Platform **credits** into either an existing identity (top-up) or a new identity (register), the local row transitions correctly, an already-consumed voucher surfaces a neutral message, and the minimum-amount guard blocks vouchers too small to fund an identity.

## Preconditions

- App launched in the simulator; Core SPV sync **running and caught up** to the testnet tip (Sync tab → `Start`; invitation create/reclaim need an InstantSend lock on the funding tx). Confirm headers/filters read `N/N` (equal), not `0/—`.
- A funded testnet wallet with spendable Dash (a reclaim create burns the voucher amount plus fees; budget ≥ 0.02 DASH to run every step).
- At least one existing Platform identity on the current network (the top-up target). Its credit balance is read from SwiftData `ZPERSISTENTIDENTITY.ZBALANCE`.
- DashPay tab selected (`DashPay` tab-bar item), a DashPay profile present (`dashpay.identityPicker`).

## Steps

1. **Create a reclaimable invitation.** Open create-invitation: DashPay tab → paperplane (`dashpay.openSentInvitations`) → the **+** button (`dashpay.invitations.create`) on the Sent Invitations screen → `CreateInvitationSheet`. Leave the amount at its default (`0.005` DASH) and submit. Wait for the InstantSend lock to confirm, then screenshot `AI_QA/output/QA004_created.png`. Read `ZPERSISTENTINVITATION` via `sqlite3` and record the new row's `ZOUTPOINTHEX`, `ZAMOUNTDUFFS` (`500000`), and `ZSTATUSRAW` (`0` = Created).
2. **Open Sent Invitations.** Tap `dashpay.openSentInvitations` (paperplane). `ios-simulator__ui_describe_all` → `AI_QA/output/QA004_sent_list.json` and confirm `dashpay.invitations.list` shows the new row with amount `0.00500000 DASH` and a `Created` badge.
3. **Reclaim → top-up (existing identity).** Swipe the `Created` row left to reveal `dashpay.invitations.reclaim`; tap it. In the sheet, confirm the body copy reads "…as identity credits — not spendable Dash." Leave the target on **Existing identity** (`dashpay.invite.reclaim.target`), pick the target via `dashpay.invite.reclaim.identityPicker`, and tap `dashpay.invite.reclaim.submit`. Wait for the Platform state transition to confirm. Screenshot `AI_QA/output/QA004_reclaim_topup.png`.
4. **Verify the top-up.** Re-read SwiftData: the target identity's `ZBALANCE` rose by ~the voucher value (in credits; 1 duff ≈ 1000 credits), and the reclaimed row's `ZSTATUSRAW` is now `2` (Reclaimed). Cross-check the outpoint reads **consumed** on-chain (platform-explorer).
5. **Reclaim → register (new identity).** Repeat steps 1–3 with a **second** invitation, but in the sheet switch the target to **New identity** (right segment of `dashpay.invite.reclaim.target`) — the identity picker is replaced by "A brand-new identity funded by this voucher." Submit. After it confirms, confirm a new funded identity appears in the Identities tab and the row's `ZSTATUSRAW` is `2` (Reclaimed). Screenshot `AI_QA/output/QA004_reclaim_register.png`.
6. **Already-consumed handling.** Take a row whose voucher is already consumed on-chain (either the register-arm race, or force-quit the app, set that row's `ZSTATUSRAW` back to `0` via `sqlite3` while the app is terminated, relaunch, and reclaim it again). Tap `dashpay.invitations.reclaim` → submit. `ios-simulator__ui_describe_all` → `AI_QA/output/QA004_already_consumed.json`.
7. **Minimum-amount guard.** Open `CreateInvitationSheet` (paperplane → **+** `dashpay.invitations.create`), set the amount below the floor (e.g. `0.001`), and confirm the create button is disabled with the sub-minimum hint. Screenshot `AI_QA/output/QA004_min_guard.png`.

## Expected Results

- **Step 1** create succeeds only because the default is `0.005` DASH; the persisted row is `Created` (`ZSTATUSRAW=0`) with `ZAMOUNTDUFFS=500000` and a 36-byte `ZRAWOUTPOINT`.
- **Step 3–4 (top-up)** the sheet states value returns as credits, never L1 Dash; on success the target identity's credit balance rises by ~the voucher value and the row badge flips to **Reclaimed** (`ZSTATUSRAW=2`). No L1 Dash is returned (the on-chain amount was an `OP_RETURN` burn at create time).
- **Step 5 (register)** a brand-new funded identity lands in Identities and the row flips to **Reclaimed** (`ZSTATUSRAW=2`); no contact request is sent (a reclaim carries no DashPay enc/dec pair).
- **Step 6 (already-consumed)** the second consume is deterministically rejected (`IdentityAssetLockTransactionOutPointAlreadyConsumedError`, Display "…already completely used"); the sheet shows the **neutral** message "This invitation was already claimed." — the claimant is **not** named — and flips the row to **Claimed** (`ZSTATUSRAW=1`). No funds are lost.
- **Step 7 (min guard)** creating below the minimum is blocked in-UI with "Minimum 0.003 DASH — a smaller voucher can't fund identity registration."; the Rust layer independently rejects a sub-minimum amount (`MIN_INVITATION_DUFFS`), so the floor holds even if the UI guard is bypassed.

Fail the QA if: a reclaim returns L1 Dash instead of credits; a failed or non-consumed reclaim flips the row status (a non-consumed error must leave it `Created`); the already-consumed path names the claimant or shows a raw error instead of the neutral message; or an invitation below `0.003` DASH can be created.
