import prettyMs from 'pretty-ms';
import { MIN_BLOCKS_BEFORE_DKG } from '../../constants.js';
import { SAFE_STOP_CONFIRMATION_DELAY_MS } from './checkMasternodeSafeToStop.js';
import {
  getBlockersAt,
  getEarliestSafeHeight,
  withoutRecordedSessionCount,
} from './getDkgStopBlockers.js';

// Lines for mainnet quorum types stay within 64 characters up to 8-digit
// heights: the default listr renderer cuts every line at the terminal width,
// less the indentation of the task it belongs to.

function describeQuorum({ llmqType, quorumIndex }) {
  const name = typeof llmqType === 'number' ? `LLMQ type ${llmqType}` : llmqType;

  return Number.isInteger(quorumIndex) && quorumIndex > 0 ? `${name} #${quorumIndex}` : name;
}

function getStart(blocker) {
  return blocker.reason === 'upcoming' ? blocker.from + MIN_BLOCKS_BEFORE_DKG : blocker.from;
}

function describeRunningDkg(blocker) {
  const end = blocker.until === null ? 'is running' : `runs until block ${blocker.until}`;

  return `${describeQuorum(blocker)} DKG at block ${getStart(blocker)} ${end}`;
}

/**
 * @param {DkgStopBlocker} blocker
 * @param {number} height
 * @return {string[]}
 */
function describeBlocker(blocker, height) {
  switch (blocker.reason) {
    case 'session':
    case 'new-round':
      return [describeRunningDkg(blocker)];
    case 'upcoming':
      if (getStart(blocker) <= height) {
        return [describeRunningDkg(blocker)];
      }

      return [`${describeQuorum(blocker)} DKG starts at block ${getStart(blocker)} `
        + `(${blocker.membershipKnown ? 'member' : 'membership unknown'})`];
    case 'imminent':
      return [`A DKG starts at block ${blocker.from + MIN_BLOCKS_BEFORE_DKG} (membership not reported)`];
    case 'active-count':
      return [`Core reports ${blocker.count} DKG session${blocker.count === 1 ? '' : 's'} `
        + 'in progress for this node'];
    default:
      return ['dashmate cannot read Core\'s DKG data:', blocker.source];
  }
}

/**
 * A DKG the wait remembered from its announcement, once Core shows a record
 * of the same session.
 */
function isShownAsRecord(blocker, blockers) {
  return blocker.reason === 'upcoming'
    && blockers.some((other) => (other.reason === 'session' || other.reason === 'new-round')
      && other.llmqType === blocker.llmqType
      && other.quorumIndex === blocker.quorumIndex
      && other.from === getStart(blocker));
}

/**
 * @param {number} seconds
 * @return {string}
 */
function formatDuration(seconds) {
  const minutes = Math.max(1, Math.round(seconds / 60));

  return prettyMs(minutes * 60 * 1000, { unitCount: 2 });
}

/**
 * @param {number|null} safeHeight
 * @param {number} height
 * @param {number|null} blockTimeSeconds
 * @return {string}
 */
function describeEarliestSafeStop(safeHeight, height, blockTimeSeconds) {
  if (safeHeight === null) {
    return 'Earliest safe stop: unknown';
  }

  const blocks = safeHeight - height;
  const time = blockTimeSeconds ? ` (~${formatDuration(blocks * blockTimeSeconds)})` : '';

  return `Earliest safe stop: block ${safeHeight}, in ${blocks}+ blocks${time}`;
}

/**
 * What blocks stopping until the earliest safe height, or, when that is
 * unknown, what blocks it now; then the earliest safe height.
 *
 * @param {number} height
 * @param {DkgStopBlocker[]} blockers
 * @param {number|null} blockTimeSeconds
 * @return {string[]}
 */
function describeBlockers(height, blockers, blockTimeSeconds) {
  const safeHeight = getEarliestSafeHeight(blockers, height);
  const relevant = withoutRecordedSessionCount(blockers, height);

  const listed = safeHeight === null
    ? getBlockersAt(relevant, height)
    : relevant.filter(({ from, until }) => (from === null || from < safeHeight)
      && (until === null || until > height));

  return [
    ...listed
      .filter((blocker) => !isShownAsRecord(blocker, listed))
      .flatMap((blocker) => describeBlocker(blocker, height)),
    describeEarliestSafeStop(safeHeight, height, blockTimeSeconds),
  ];
}

/**
 * Progress of `waitForDKGWindowPass` for the operator.
 *
 * @param {Object} status as passed to its `progressCallback`
 * @param {Object} options
 * @param {number|null} options.blockTimeSeconds target block spacing, or
 *   `null` to estimate in blocks only
 * @return {string}
 */
export default function formatDkgStopStatus(status, { blockTimeSeconds }) {
  switch (status.type) {
    case 'blocked':
      return [
        `At block ${status.height}, waiting for:`,
        ...describeBlockers(status.height, status.blockers, blockTimeSeconds),
      ].join('\n');
    case 'confirming':
      return `Block ${status.height} looks safe; checking again `
        + `in ${SAFE_STOP_CONFIRMATION_DELAY_MS / 1000}s`;
    case 'unsettled':
      return 'Core\'s DKG state changed while reading it; checking again';
    default:
      return `Core RPC failed (${status.failedPolls} of ${status.maxFailedPolls}), retrying:\n`
        + `${status.error.message}`;
  }
}

/**
 * Why a plain stop refuses to stop the masternode, and what the operator can
 * do. Only the flags the running command has are suggested.
 *
 * @param {SafeStopCheck} check a check that is not `safe`
 * @param {Object} options
 * @param {number|null} options.blockTimeSeconds
 * @param {boolean} options.canWait the command has `--safe`
 * @param {boolean} options.canForce the command has `--force`
 * @return {string}
 */
export function formatDkgStopRefusal(check, { blockTimeSeconds, canWait, canForce }) {
  const unreadable = check.status === 'blocked'
    && getBlockersAt(check.blockers, check.height).find(({ reason }) => reason === 'unreadable');

  // Unreadable data does not clear on its own, so --safe is not offered.
  if (unreadable) {
    return [
      ...describeBlocker(unreadable, check.height),
      canForce ? 'Update dashmate, or use --force to stop now' : 'Update dashmate',
    ].join('\n');
  }

  const lines = check.status === 'blocked'
    ? [
      'Stopping now risks a PoSe penalty for this masternode:',
      ...describeBlockers(check.height, check.blockers, blockTimeSeconds),
    ]
    : ['Core\'s DKG state kept changing while reading it', '(is Core still syncing?)'];

  const retry = check.status === 'blocked' && getEarliestSafeHeight(check.blockers, check.height) !== null
    ? 'Retry then'
    : 'Retry later';

  if (canWait && canForce) {
    lines.push(`${retry}, use --safe to wait and stop automatically,`);
    lines.push('or --force to stop now and accept the PoSe risk');
  } else if (canWait) {
    lines.push(`${retry}, or use --safe to wait and stop automatically`);
  } else if (canForce) {
    lines.push(`${retry}, or use --force to stop now and accept the PoSe risk`);
  } else {
    lines.push(retry);
  }

  return lines.join('\n');
}
