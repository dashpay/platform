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
 * Unknown membership and malformed entries block the stop. Upcoming
 * sessions only matter within the restart margin; active_dkg_sessions already
 * contains only sessions inside their active window.
 */
function isDkgBlockingStop(dkg, blocksField, maxBlocks = Infinity) {
  if (!dkg
    || typeof dkg !== 'object'
    || !isValidDkgCounter(dkg[blocksField])
    || typeof dkg.known !== 'boolean'
    || (dkg.known && typeof dkg.isMember !== 'boolean')) {
    return true;
  }

  return dkg[blocksField] <= maxBlocks && (!dkg.known || dkg.isMember);
}

function isDkgMembershipBlockingStop(dkgInfo) {
  const { active_dkg_sessions: activeDkgSessions, upcoming_dkgs: upcomingDkgs } = dkgInfo;

  // Older Core, and current Core without a proTxHash, omit membership lists.
  // Keep the legacy guard when active_dkg_sessions is unavailable.
  if (activeDkgSessions === undefined) {
    return dkgInfo.next_dkg <= MIN_BLOCKS_BEFORE_DKG;
  }

  if (!Array.isArray(activeDkgSessions) || !Array.isArray(upcomingDkgs)) {
    return true;
  }

  return dkgInfo.active_dkgs > 0
    || activeDkgSessions.some((dkg) => isDkgBlockingStop(dkg, 'blocksSinceStart'))
    || upcomingDkgs.some((dkg) => isDkgBlockingStop(dkg, 'blocksUntilStart', MIN_BLOCKS_BEFORE_DKG));
}

/**
 * @param {Object} dkgInfo
 * @return {boolean}
 */
export function shouldInspectDkgStatusForSafeStop(dkgInfo) {
  if (!hasValidDkgInfoShape(dkgInfo)) {
    return false;
  }

  return dkgInfo.active_dkg_sessions === undefined
    && dkgInfo.active_dkgs > 0
    && dkgInfo.next_dkg > MIN_BLOCKS_BEFORE_DKG;
}

/**
 * Core's active_dkg_sessions and upcoming_dkgs report membership independently
 * of asynchronous local session tracking. Any current member or unknown
 * session blocks stopping, as does an upcoming one within the restart margin.
 *
 * Without active_dkg_sessions, use the legacy next_dkg guard and resolve tracked
 * sessions against dkgstatus and getblockcount. Unknown quorum types or
 * malformed status fail safe; sessions past dkgMiningWindowStart are ignored.
 *
 * @param {Object} dkgInfo Result of quorum dkginfo.
 * @param {Object} [dkgStatus] Result of quorum dkgstatus for older Core.
 * @param {number} [currentHeight] Chain tip height for older Core.
 * @return {boolean}
 */
export default function isMasternodeSafeToStopDuringDkg(
  dkgInfo,
  dkgStatus,
  currentHeight,
) {
  if (!hasValidDkgInfoShape(dkgInfo)) {
    return false;
  }

  if (isDkgMembershipBlockingStop(dkgInfo)) {
    return false;
  }

  if (dkgInfo.active_dkg_sessions !== undefined || dkgInfo.active_dkgs === 0) {
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
