import { MIN_BLOCKS_BEFORE_DKG } from '../../constants.js';

/**
 * First Dash Core release whose `quorum dkginfo` derives `active_dkgs` from
 * this masternode's membership, counting a session from its start block
 * (Core 24.1.0). Older Core counts the DKG worker's local session records,
 * which appear only after the worker has started a session and linger long
 * after it ends. Core reports `version` as 10000 * major + 100 * minor + patch.
 */
export const MEMBERSHIP_ACTIVE_DKGS_CORE_VERSION = 240100;

/**
 * DKG parameters from Dash Core `src/llmq/params.h`. `quorum dkgstatus` names
 * types by `name`, `quorum dkginfo` `upcoming_dkgs` by numeric `id`.
 *
 * Keep in sync with `src/llmq/params.h` in Dash Core.
 */
export const LLMQ_TYPES = [
  {
    id: 1, name: 'llmq_50_60', dkgInterval: 24, dkgPhaseBlocks: 2,
  },
  {
    id: 2, name: 'llmq_400_60', dkgInterval: 288, dkgPhaseBlocks: 4,
  },
  {
    id: 3, name: 'llmq_400_85', dkgInterval: 576, dkgPhaseBlocks: 4,
  },
  {
    id: 4, name: 'llmq_100_67', dkgInterval: 24, dkgPhaseBlocks: 2,
  },
  {
    id: 5, name: 'llmq_60_75', dkgInterval: 288, dkgPhaseBlocks: 2,
  },
  {
    id: 6, name: 'llmq_25_67', dkgInterval: 24, dkgPhaseBlocks: 2,
  },
  {
    id: 100, name: 'llmq_test', dkgInterval: 24, dkgPhaseBlocks: 2,
  },
  {
    id: 101, name: 'llmq_devnet', dkgInterval: 24, dkgPhaseBlocks: 2,
  },
  {
    id: 102, name: 'llmq_test_v17', dkgInterval: 24, dkgPhaseBlocks: 2,
  },
  {
    id: 103, name: 'llmq_test_dip0024', dkgInterval: 24, dkgPhaseBlocks: 2,
  },
  {
    id: 104, name: 'llmq_test_instantsend', dkgInterval: 24, dkgPhaseBlocks: 2,
  },
  {
    id: 105, name: 'llmq_devnet_dip0024', dkgInterval: 48, dkgPhaseBlocks: 2,
  },
  {
    id: 106, name: 'llmq_test_platform', dkgInterval: 24, dkgPhaseBlocks: 2,
  },
  {
    id: 107, name: 'llmq_devnet_platform', dkgInterval: 24, dkgPhaseBlocks: 2,
  },
];

/**
 * Blocks from a session's start to the end of its Commit phase, Core's
 * `QuorumPhase::Commit * dkgPhaseBlocks`. By then a member has sent every
 * message that decides whether the final commitment counts it as valid,
 * which is what PoSe penalizes; finalizing and mining the commitment does
 * not need it. Each rotated quorum index starts on its own block and is
 * measured from its own `quorumHeight`, not from the cycle's shared mining
 * window.
 *
 * @param {{ dkgPhaseBlocks: number }} llmq
 * @return {number}
 */
export function getActiveDkgBlocks(llmq) {
  return 5 * llmq.dkgPhaseBlocks;
}

/**
 * @param {number|null} coreVersion
 * @return {boolean}
 */
export function hasMembershipActiveDkgs(coreVersion) {
  return Number.isInteger(coreVersion) && coreVersion >= MEMBERSHIP_ACTIVE_DKGS_CORE_VERSION;
}

/**
 * @typedef {Object} DkgStopBlocker
 * @property {string} reason `session`, `new-round`, `unrecorded`, `upcoming`,
 *   `imminent`, `active-count` or `unreadable`
 * @property {string|number|null} [llmqType] LLMQ name, or Core's numeric id
 *   when dashmate does not know the type
 * @property {number} [quorumIndex]
 * @property {boolean} [membershipKnown] for `upcoming`: `false` when Core
 *   could not determine whether this node is a member
 * @property {number} [count] for `active-count`
 * @property {string} [source] for `unreadable`: which Core data was unreadable
 * @property {number|null} from first blocked height; `null` blocks from now
 * @property {number|null} until first height no longer blocked; `null` when
 *   the end is unknown
 */

/**
 * @param {DkgStopBlocker} blocker
 * @param {number} height
 * @return {boolean}
 */
function isBlockedAt(blocker, height) {
  return (blocker.from === null || blocker.from <= height)
    && (blocker.until === null || height < blocker.until);
}

/**
 * @param {DkgStopBlocker[]} blockers
 * @param {number} height
 * @return {DkgStopBlocker[]}
 */
export function getBlockersAt(blockers, height) {
  return blockers.filter((blocker) => isBlockedAt(blocker, height));
}

function isCount(value) {
  return Number.isInteger(value) && value >= 0;
}

function hasValidDkgInfoShape(dkgInfo) {
  return !!dkgInfo
    && typeof dkgInfo === 'object'
    && isCount(dkgInfo.active_dkgs)
    && isCount(dkgInfo.next_dkg);
}

function isValidUpcomingDkg(dkg) {
  return !!dkg
    && typeof dkg === 'object'
    && isCount(dkg.blocksUntilStart)
    && typeof dkg.known === 'boolean'
    && (!dkg.known || typeof dkg.isMember === 'boolean');
}

function unreadable(source) {
  return {
    reason: 'unreadable', source, from: null, until: null,
  };
}

/**
 * Without membership data (Core < 24, or Core does not know this node's
 * proTxHash), any DKG of any type may include this node, so the restart
 * margin before the next one blocks with no known end.
 */
function getImminentDkgBlocker(dkgInfo, height) {
  return {
    reason: 'imminent',
    from: height + dkgInfo.next_dkg - MIN_BLOCKS_BEFORE_DKG,
    until: null,
  };
}

/**
 * Core lists a DKG in `upcoming_dkgs` from its work block, 8 blocks before
 * the cycle starts, so every DKG starting within `MIN_BLOCKS_BEFORE_DKG`
 * is listed. A DKG this node is a member of, or whose membership Core could
 * not determine, blocks from the restart margin before its start until its
 * Commit phase ends.
 */
function getUpcomingDkgBlockers(upcomingDkgs, height) {
  if (!Array.isArray(upcomingDkgs)) {
    return [unreadable('quorum dkginfo upcoming_dkgs')];
  }

  return upcomingDkgs.flatMap((dkg) => {
    if (!isValidUpcomingDkg(dkg)) {
      return [unreadable('quorum dkginfo upcoming_dkgs')];
    }

    if (dkg.known && !dkg.isMember) {
      return [];
    }

    const llmq = LLMQ_TYPES.find(({ id }) => id === dkg.llmqType);
    const start = height + dkg.blocksUntilStart;

    return [{
      reason: 'upcoming',
      llmqType: llmq ? llmq.name : dkg.llmqType,
      quorumIndex: dkg.quorumIndex,
      membershipKnown: dkg.known,
      from: start - MIN_BLOCKS_BEFORE_DKG,
      until: llmq ? start + getActiveDkgBlocks(llmq) : null,
    }];
  });
}

/**
 * The DKG worker keeps a record for the latest session of each LLMQ type
 * and quorum index this node was a member of. A record inside its active
 * window blocks. A record at least one interval old, but less than one
 * interval plus the active window, means the next session of that type and
 * index has started and the worker, which runs shortly after the block
 * connects, has not yet replaced or removed the record. This node may be a
 * member again, so the record blocks until that session's Commit phase ends.
 */
function getSessionRecordBlockers(dkgStatus, height) {
  const sessions = dkgStatus && dkgStatus.session;

  if (!Array.isArray(sessions)) {
    return [unreadable('quorum dkgstatus')];
  }

  return sessions.flatMap((entry) => {
    const llmq = entry && LLMQ_TYPES.find(({ name }) => name === entry.llmqType);
    const quorumHeight = entry && entry.status && entry.status.quorumHeight;

    if (!llmq || !isCount(quorumHeight) || quorumHeight > height) {
      return [unreadable(`quorum dkgstatus session ${entry && entry.llmqType}`)];
    }

    const activeBlocks = getActiveDkgBlocks(llmq);
    const offset = height - quorumHeight;
    const session = { llmqType: llmq.name, quorumIndex: entry.quorumIndex };

    if (offset < activeBlocks) {
      return [{
        ...session,
        reason: 'session',
        from: quorumHeight,
        until: quorumHeight + activeBlocks,
      }];
    }

    const nextQuorumHeight = quorumHeight + llmq.dkgInterval;
    if (height >= nextQuorumHeight && height < nextQuorumHeight + activeBlocks) {
      return [{
        ...session,
        reason: 'new-round',
        from: nextQuorumHeight,
        until: nextQuorumHeight + activeBlocks,
      }];
    }

    return [];
  });
}

/**
 * Key naming a DKG session: its quorum and its start block.
 *
 * @param {string} llmqType
 * @param {number} quorumIndex
 * @param {number} start
 * @return {string}
 */
function getDkgKey(llmqType, quorumIndex, start) {
  return `${llmqType}:${quorumIndex}:${start}`;
}

/**
 * The DKGs Core announces this node is not a member of, as
 * `llmqType:quorumIndex:start` keys. Once such a DKG starts Core no longer
 * announces it, so a caller polling Core can remember these to tell a
 * session it is not part of from one Core has not recorded yet.
 *
 * @param {Object} dkgInfo Result of `quorum dkginfo`.
 * @param {number} height Chain tip `dkgInfo` describes.
 * @return {Set<string>}
 */
export function getAnnouncedNonMemberDkgs(dkgInfo, height) {
  const upcomingDkgs = dkgInfo && dkgInfo.upcoming_dkgs;

  if (!Array.isArray(upcomingDkgs) || !isCount(height)) {
    return new Set();
  }

  return new Set(upcomingDkgs
    .filter((dkg) => isValidUpcomingDkg(dkg) && dkg.known && !dkg.isMember)
    .map((dkg) => [LLMQ_TYPES.find(({ id }) => id === dkg.llmqType), dkg])
    .filter(([llmq]) => llmq)
    .map(([llmq, dkg]) => getDkgKey(llmq.name, dkg.quorumIndex, height + dkg.blocksUntilStart)));
}

/**
 * At a session's start block Core older than 24.1 erases the previous round's
 * record, computes the new session's members and only then records the
 * session if this node is one of them; until then nothing tells whether it
 * is, however long that takes. `quorum dkgstatus` lists every enabled quorum
 * with the start of its current session, so a current session without a
 * record blocks until its Commit phase ends, unless Core announced it
 * without this node.
 */
function getUnrecordedSessionBlockers(dkgStatus, height, knownNonMemberDkgs) {
  const connections = dkgStatus && dkgStatus.quorumConnections;

  if (!Array.isArray(connections)) {
    return [unreadable('quorum dkgstatus quorumConnections')];
  }

  const sessions = Array.isArray(dkgStatus.session) ? dkgStatus.session : [];
  const isRecorded = ({ llmqType, quorumIndex, pQuorumBaseBlockIndex: start }) => sessions
    .some((entry) => entry
      && entry.llmqType === llmqType
      && entry.quorumIndex === quorumIndex
      && entry.status
      && entry.status.quorumHeight === start);

  return connections
    // A quorum whose current session has not started yet has no start block.
    .filter((quorum) => quorum && isCount(quorum.pQuorumBaseBlockIndex)
      && quorum.pQuorumBaseBlockIndex <= height)
    .flatMap((quorum) => {
      const { llmqType, quorumIndex, pQuorumBaseBlockIndex: start } = quorum;
      const llmq = LLMQ_TYPES.find(({ name }) => name === llmqType);

      if (!llmq) {
        return [unreadable(`quorum dkgstatus quorumConnections ${llmqType}`)];
      }

      const until = start + getActiveDkgBlocks(llmq);

      if (height >= until || isRecorded(quorum)
        || knownNonMemberDkgs.has(getDkgKey(llmqType, quorumIndex, start))) {
        return [];
      }

      return [{
        reason: 'unrecorded', llmqType, quorumIndex, from: start, until,
      }];
    });
}

/**
 * Everything that makes stopping this masternode risk a PoSe penalty, as
 * height ranges. Stopping is safe at a height no blocker covers (see
 * {@link getBlockersAt}). Some blockers start above the current height; they
 * do not block yet and only help estimate when stopping becomes safe.
 *
 * All inputs must come from the same chain tip, `height`. Core older than
 * {@link MEMBERSHIP_ACTIVE_DKGS_CORE_VERSION} reports a session only after
 * its DKG worker has recorded it, after the start block connects; until then
 * the session blocks unless Core announced it without this node. Callers
 * must also confirm a safe result a while later, once Core has had time to
 * replace records of previous rounds.
 *
 * Unreadable Core data blocks with no known end.
 *
 * @param {Object} snapshot
 * @param {Object} snapshot.dkgInfo Result of `quorum dkginfo`.
 * @param {Object} snapshot.dkgStatus Result of `quorum dkgstatus`.
 * @param {number} snapshot.height Chain tip both results describe.
 * @param {number} snapshot.coreVersion Core's numeric `version`.
 * @param {Set<string>} [snapshot.knownNonMemberDkgs] DKGs Core announced
 *   without this node, from {@link getAnnouncedNonMemberDkgs}
 * @return {DkgStopBlocker[]}
 */
export default function getDkgStopBlockers({
  dkgInfo,
  dkgStatus,
  height,
  coreVersion,
  knownNonMemberDkgs = new Set(),
}) {
  if (!isCount(height)) {
    return [unreadable('block height')];
  }

  if (!hasValidDkgInfoShape(dkgInfo)) {
    return [unreadable('quorum dkginfo')];
  }

  const blockers = [
    ...(dkgInfo.upcoming_dkgs === undefined
      ? [getImminentDkgBlocker(dkgInfo, height)]
      : getUpcomingDkgBlockers(dkgInfo.upcoming_dkgs, height)),
    // Read on every Core version: Core 24.1 stops counting a running session
    // in active_dkgs when a mid-session ProUpServTx clears its proTxHash, but
    // the session keeps going and its record stays.
    ...getSessionRecordBlockers(dkgStatus, height),
  ];

  if (!hasMembershipActiveDkgs(coreVersion)) {
    blockers.push(...getUnrecordedSessionBlockers(dkgStatus, height, knownNonMemberDkgs));
  }

  // Core 24.1+ counts every session this node is a member of, or whose
  // membership is unknown, from its start block through its Commit phase.
  if (hasMembershipActiveDkgs(coreVersion) && dkgInfo.active_dkgs > 0) {
    blockers.push({
      reason: 'active-count',
      count: dkgInfo.active_dkgs,
      from: null,
      until: null,
    });
  }

  return blockers;
}
