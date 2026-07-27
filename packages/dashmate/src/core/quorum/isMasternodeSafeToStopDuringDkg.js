import { MIN_BLOCKS_BEFORE_DKG } from '../../constants.js';

/**
 * `dkgMiningWindowStart` values from Dash Core `src/llmq/params.h`,
 * indexed by llmqType string as reported in `quorum dkgstatus`. The
 * number is how many blocks after a session's `quorumHeight` the
 * active DKG window lasts. Once `currentHeight - quorumHeight >=
 * window`, the session is past its active phase and a restart no
 * longer risks a PoSe penalty for that session.
 *
 * Keep in sync with `src/llmq/params.h` in Dash Core.
 */
export const DKG_MINING_WINDOW_START_BY_LLMQ_TYPE = {
  llmq_test: 10,
  llmq_test_instantsend: 10,
  llmq_test_v17: 10,
  llmq_test_dip0024: 12,
  llmq_test_platform: 10,
  llmq_devnet: 10,
  llmq_devnet_dip0024: 12,
  llmq_devnet_platform: 10,
  llmq_50_60: 10,
  llmq_60_75: 42,
  llmq_400_60: 20,
  llmq_400_85: 20,
  llmq_100_67: 10,
  llmq_25_67: 10,
};

function isValidDkgCounter(value) {
  return typeof value === 'number' && Number.isFinite(value) && value >= 0;
}

function hasValidDkgInfoShape(dkgInfo) {
  return !!dkgInfo
    && typeof dkgInfo === 'object'
    && isValidDkgCounter(dkgInfo.active_dkgs)
    && isValidDkgCounter(dkgInfo.next_dkg);
}

/**
 * @param {{ active_dkgs: number, next_dkg: number }} dkgInfo
 * @return {boolean}
 */
export function shouldInspectDkgStatusForSafeStop(dkgInfo) {
  if (!hasValidDkgInfoShape(dkgInfo)) {
    return false;
  }

  return dkgInfo.active_dkgs > 0 && dkgInfo.next_dkg > MIN_BLOCKS_BEFORE_DKG;
}

/**
 * Determine whether a masternode can be safely stopped without
 * risking a PoSe penalty from disrupting an in-progress or imminent
 * DKG session.
 *
 * Inputs come from three Core RPCs:
 *   - `quorum dkginfo`   → `{ active_dkgs, next_dkg }`
 *   - `quorum dkgstatus` → `{ session: [{ llmqType, status: { quorumHeight } }, ...] }`
 *   - `getblockcount`    → integer chain tip height
 *
 * Decision rules:
 *   1. `next_dkg <= MIN_BLOCKS_BEFORE_DKG` — a new cycle could begin
 *      before the restart completes. Unsafe regardless of sessions.
 *   2. `active_dkgs === 0` — no sessions tracked locally. Safe.
 *   3. `active_dkgs > 0` — `active_dkgs` in Core is
 *      `dkgdbgman.GetSessionCount()`, an aggregate counter spanning
 *      all LLMQs the node knows about and can linger past a session's
 *      true active window. Resolve the ambiguity per-session against
 *      `quorum dkgstatus` + the chain tip:
 *        - For each session, look up its llmqType's
 *          `dkgMiningWindowStart`. If the llmqType is unknown or
 *          `quorumHeight` is missing/malformed, fail safe (unsafe) —
 *          we cannot reason about a session we cannot identify.
 *        - A session is still active when
 *          `0 <= currentHeight - quorumHeight < dkgMiningWindowStart`.
 *          Any such session blocks the stop.
 *        - A negative offset is inconsistent with Core's tracked
 *          sessions and fails safe. Sessions whose offset is past the
 *          window are treated as stale and ignored.
 *      If every session is past its window, the stop is safe.
 *
 * @param {{ active_dkgs: number, next_dkg: number }} dkgInfo
 *   Result of `quorum dkginfo`.
 * @param {{ session?: Array<{ llmqType?: string, status?: { quorumHeight?: number } }> }} [dkgStatus]
 *   Result of `quorum dkgstatus`. Only consulted when
 *   `dkgInfo.active_dkgs > 0`.
 * @param {number} [currentHeight]
 *   Current block height from `getblockcount`. Only consulted when
 *   `dkgInfo.active_dkgs > 0`.
 * @return {boolean} `true` when the node can be safely stopped.
 */
export default function isMasternodeSafeToStopDuringDkg(
  dkgInfo,
  dkgStatus,
  currentHeight,
) {
  if (!hasValidDkgInfoShape(dkgInfo)) {
    return false;
  }

  const { active_dkgs: activeDkgs, next_dkg: nextDkg } = dkgInfo;

  if (nextDkg <= MIN_BLOCKS_BEFORE_DKG) {
    return false;
  }

  if (activeDkgs === 0) {
    return true;
  }

  if (!dkgStatus
    || !Array.isArray(dkgStatus.session)
    || dkgStatus.session.length === 0
    || typeof currentHeight !== 'number'
    || !Number.isFinite(currentHeight)) {
    return false;
  }

  for (const sessionEntry of dkgStatus.session) {
    const llmqType = sessionEntry && sessionEntry.llmqType;
    const quorumHeight = sessionEntry && sessionEntry.status
      && sessionEntry.status.quorumHeight;

    const windowLength = DKG_MINING_WINDOW_START_BY_LLMQ_TYPE[llmqType];

    if (windowLength === undefined
      || typeof quorumHeight !== 'number'
      || !Number.isFinite(quorumHeight)) {
      return false;
    }

    const offset = currentHeight - quorumHeight;
    if (offset < 0 || offset < windowLength) {
      return false;
    }
  }

  return true;
}
