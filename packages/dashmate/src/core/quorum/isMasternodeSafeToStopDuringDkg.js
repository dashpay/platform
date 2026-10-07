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
 * Malformed entries always block; member and unknown-membership entries
 * block when they start within the restart margin.
 */
function isUpcomingDkgBlockingStop(dkg) {
  if (!dkg
    || typeof dkg !== 'object'
    || !isValidDkgCounter(dkg.blocksUntilStart)
    || typeof dkg.known !== 'boolean'
    || (dkg.known && typeof dkg.isMember !== 'boolean')) {
    return true;
  }

  return dkg.blocksUntilStart <= MIN_BLOCKS_BEFORE_DKG && (!dkg.known || dkg.isMember);
}

/**
 * @param {Object} dkgInfo
 * @return {boolean}
 */
export function shouldInspectDkgStatusForSafeStop(dkgInfo) {
  if (!hasValidDkgInfoShape(dkgInfo)) {
    return false;
  }

  return dkgInfo.upcoming_dkgs === undefined
    && dkgInfo.active_dkgs > 0
    && dkgInfo.next_dkg > MIN_BLOCKS_BEFORE_DKG;
}

/**
 * With upcoming_dkgs, stopping is safe when no DKG counts toward active_dkgs
 * and no member or unknown-membership DKG starts within the restart margin.
 *
 * Without upcoming_dkgs (older Core, or no proTxHash), use the legacy next_dkg
 * guard and resolve tracked sessions against dkgstatus and getblockcount.
 * Unknown quorum types or malformed status fail safe; sessions past
 * dkgMiningWindowStart are ignored.
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

  const { upcoming_dkgs: upcomingDkgs } = dkgInfo;

  // Relies on active_dkgs counting member and unknown-membership sessions
  // (dashpay/dash#7812).
  if (upcomingDkgs !== undefined) {
    return dkgInfo.active_dkgs === 0
      && Array.isArray(upcomingDkgs)
      && !upcomingDkgs.some(isUpcomingDkgBlockingStop);
  }

  if (dkgInfo.next_dkg <= MIN_BLOCKS_BEFORE_DKG) {
    return false;
  }

  if (dkgInfo.active_dkgs === 0) {
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
