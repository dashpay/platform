import getDkgStopBlockers, {
  getAnnouncedNonMemberDkgs,
  getBlockersAt,
  hasMembershipActiveDkgs,
  LLMQ_TYPES,
} from '../../../../src/core/quorum/getDkgStopBlockers.js';
import { MIN_BLOCKS_BEFORE_DKG } from '../../../../src/constants.js';

const CORE_23 = 230110;
const CORE_24_0 = 240000;
const CORE_24_1 = 240100;
const HEIGHT = 1000;

// A value of next_dkg that is safely above MIN_BLOCKS_BEFORE_DKG so the
// imminent-DKG guard never trips in tests that focus on session records.
const NEXT_DKG_NOT_IMMINENT = MIN_BLOCKS_BEFORE_DKG + 5;
const PLATFORM_ACTIVE_BLOCKS = 10; // llmq_test_platform: 5 phases * 2 blocks
const PLATFORM_INTERVAL = 24;

// Core lists the quorum connections in every `quorum dkgstatus` answer.
function withConnections(dkgStatus) {
  return dkgStatus && typeof dkgStatus === 'object'
    ? { quorumConnections: [], ...dkgStatus }
    : dkgStatus;
}

function blockersAt({
  dkgInfo,
  dkgStatus = { session: [] },
  height = HEIGHT,
  coreVersion = CORE_23,
}) {
  return getBlockersAt(getDkgStopBlockers({
    dkgInfo, dkgStatus: withConnections(dkgStatus), height, coreVersion,
  }), height);
}

function record(llmqType, quorumHeight, quorumIndex = 0) {
  return { llmqType, quorumIndex, status: { quorumHeight } };
}

// Shape of a `quorum dkginfo` upcoming_dkgs entry once membership is known.
function upcomingDkg(blocksUntilStart, isMember, llmqType = 106) {
  return {
    llmqType,
    quorumIndex: 0,
    quorumHeight: HEIGHT + blocksUntilStart,
    blocksUntilStart,
    known: true,
    isMember,
    workBlockHeight: HEIGHT + blocksUntilStart - 8,
    workBlockHash: '00'.repeat(32),
  };
}

function unknownUpcomingDkg(blocksUntilStart) {
  return {
    llmqType: 103,
    quorumIndex: 0,
    quorumHeight: HEIGHT + blocksUntilStart,
    blocksUntilStart,
    known: false,
    reason: 'rotated quorum snapshots are not available yet',
  };
}

describe('getDkgStopBlockers', () => {
  describe('imminent DKG guard from dkginfo.next_dkg (no membership data)', () => {
    it('should block when next_dkg <= MIN_BLOCKS_BEFORE_DKG even with no sessions', () => {
      const blockers = blockersAt({ dkgInfo: { active_dkgs: 0, next_dkg: MIN_BLOCKS_BEFORE_DKG } });

      expect(blockers).to.have.lengthOf(1);
      expect(blockers[0]).to.include({ reason: 'imminent', until: null });
    });

    it('should allow the stop when next_dkg is past the restart margin', () => {
      expect(blockersAt({ dkgInfo: { active_dkgs: 0, next_dkg: MIN_BLOCKS_BEFORE_DKG + 1 } }))
        .to.be.empty();
    });

    it('should report when the next DKG starts blocking', () => {
      const [blocker] = getDkgStopBlockers({
        dkgInfo: { active_dkgs: 0, next_dkg: 20 },
        dkgStatus: withConnections({ session: [] }),
        height: HEIGHT,
        coreVersion: CORE_23,
      });

      expect(blocker).to.include({
        reason: 'imminent', from: HEIGHT + 20 - MIN_BLOCKS_BEFORE_DKG, until: null,
      });
    });

    it('should ignore active_dkgs on Core older than 24.1, which counts lingering records', () => {
      expect(blockersAt({ dkgInfo: { active_dkgs: 3, next_dkg: NEXT_DKG_NOT_IMMINENT } }))
        .to.be.empty();
    });
  });

  describe('membership from dkginfo.upcoming_dkgs', () => {
    it('should allow the stop when an imminent DKG does not include this node', () => {
      const dkgInfo = {
        active_dkgs: 0,
        next_dkg: 1,
        upcoming_dkgs: [upcomingDkg(1, false), upcomingDkg(MIN_BLOCKS_BEFORE_DKG, false)],
      };

      expect(blockersAt({ dkgInfo, coreVersion: CORE_24_0 })).to.be.empty();
    });

    it('should allow the stop when next_dkg is imminent but no upcoming DKG is listed', () => {
      // next_dkg reports 1 for the whole rotation signing window, even
      // after the last rotated index has started.
      const dkgInfo = { active_dkgs: 0, next_dkg: 1, upcoming_dkgs: [] };

      expect(blockersAt({ dkgInfo, coreVersion: CORE_24_0 })).to.be.empty();
    });

    it('should block when this node is a member of an imminent DKG until its Commit phase ends', () => {
      const dkgInfo = {
        active_dkgs: 0,
        next_dkg: 1,
        upcoming_dkgs: [upcomingDkg(1, false), upcomingDkg(MIN_BLOCKS_BEFORE_DKG, true)],
      };
      const start = HEIGHT + MIN_BLOCKS_BEFORE_DKG;

      const blockers = blockersAt({ dkgInfo, coreVersion: CORE_24_0 });

      expect(blockers).to.have.lengthOf(1);
      expect(blockers[0]).to.include({
        reason: 'upcoming',
        llmqType: 'llmq_test_platform',
        membershipKnown: true,
        from: start - MIN_BLOCKS_BEFORE_DKG,
        until: start + PLATFORM_ACTIVE_BLOCKS,
      });
    });

    it('should not block yet when this node is a member of a DKG past the restart margin', () => {
      const dkgInfo = {
        active_dkgs: 0,
        next_dkg: MIN_BLOCKS_BEFORE_DKG + 1,
        upcoming_dkgs: [upcomingDkg(MIN_BLOCKS_BEFORE_DKG + 1, true)],
      };

      const blockers = getDkgStopBlockers({
        dkgInfo, dkgStatus: withConnections({ session: [] }), height: HEIGHT, coreVersion: CORE_24_0,
      });

      expect(getBlockersAt(blockers, HEIGHT)).to.be.empty();
      expect(getBlockersAt(blockers, HEIGHT + 1)).to.have.lengthOf(1);
    });

    it('should block when membership of an imminent DKG is unknown', () => {
      const dkgInfo = {
        active_dkgs: 0,
        next_dkg: 1,
        upcoming_dkgs: [unknownUpcomingDkg(1)],
      };

      const blockers = blockersAt({ dkgInfo, coreVersion: CORE_24_0 });

      expect(blockers).to.have.lengthOf(1);
      expect(blockers[0]).to.include({ reason: 'upcoming', membershipKnown: false });
    });

    it('should allow the stop when membership is unknown only for a DKG past the restart margin', () => {
      const dkgInfo = {
        active_dkgs: 0,
        next_dkg: MIN_BLOCKS_BEFORE_DKG + 1,
        upcoming_dkgs: [unknownUpcomingDkg(MIN_BLOCKS_BEFORE_DKG + 1)],
      };

      expect(blockersAt({ dkgInfo, coreVersion: CORE_24_0 })).to.be.empty();
    });

    it('should block a member DKG of a type dashmate does not know, with no known end', () => {
      const dkgInfo = {
        active_dkgs: 0,
        next_dkg: 1,
        upcoming_dkgs: [upcomingDkg(1, true, 250)],
      };

      const blockers = blockersAt({ dkgInfo, coreVersion: CORE_24_0 });

      expect(blockers).to.have.lengthOf(1);
      expect(blockers[0]).to.include({ reason: 'upcoming', llmqType: 250, until: null });
    });

    describe('fail-safe on malformed upcoming_dkgs', () => {
      const malformed = {
        'upcoming_dkgs is an object': {},
        'upcoming_dkgs is null': null,
        'an entry is null': [null],
        'an entry is missing known': [{ blocksUntilStart: 1, isMember: false }],
        'a known entry is missing isMember': [{ blocksUntilStart: 1, known: true }],
        'an entry is missing blocksUntilStart': [{ known: true, isMember: false }],
        'an entry has a negative blocksUntilStart': [
          { blocksUntilStart: -1, known: true, isMember: false },
        ],
        'an entry outside the margin is malformed': [
          { blocksUntilStart: MIN_BLOCKS_BEFORE_DKG + 1, known: true },
        ],
      };

      Object.entries(malformed).forEach(([description, upcomingDkgs]) => {
        it(`should block when ${description}`, () => {
          const dkgInfo = {
            active_dkgs: 0,
            next_dkg: NEXT_DKG_NOT_IMMINENT,
            upcoming_dkgs: upcomingDkgs,
          };

          const blockers = blockersAt({ dkgInfo, coreVersion: CORE_24_0 });

          expect(blockers).to.have.lengthOf(1);
          expect(blockers[0]).to.include({ reason: 'unreadable', until: null });
        });
      });
    });
  });

  describe('session records from dkgstatus', () => {
    const dkgInfo = { active_dkgs: 1, next_dkg: NEXT_DKG_NOT_IMMINENT };

    it('should block a session at its start block', () => {
      const blockers = blockersAt({
        dkgInfo,
        dkgStatus: { session: [record('llmq_test_platform', HEIGHT)] },
      });

      expect(blockers).to.have.lengthOf(1);
      expect(blockers[0]).to.include({
        reason: 'session',
        llmqType: 'llmq_test_platform',
        from: HEIGHT,
        until: HEIGHT + PLATFORM_ACTIVE_BLOCKS,
      });
    });

    it('should block a session at the last block of its Commit phase', () => {
      const dkgStatus = { session: [record('llmq_test_platform', HEIGHT - PLATFORM_ACTIVE_BLOCKS + 1)] };

      expect(blockersAt({ dkgInfo, dkgStatus })).to.have.lengthOf(1);
    });

    it('should allow the stop once a session has finished its Commit phase', () => {
      const dkgStatus = { session: [record('llmq_test_platform', HEIGHT - PLATFORM_ACTIVE_BLOCKS)] };

      expect(blockersAt({ dkgInfo, dkgStatus })).to.be.empty();
    });

    it('should block a 400-member session for its 20 blocks', () => {
      expect(blockersAt({ dkgInfo, dkgStatus: { session: [record('llmq_400_60', HEIGHT - 19)] } }))
        .to.have.lengthOf(1);
      expect(blockersAt({ dkgInfo, dkgStatus: { session: [record('llmq_400_60', HEIGHT - 20)] } }))
        .to.be.empty();
    });

    it('should measure a rotated session from its own quorum index start', () => {
      // llmq_60_75 index 20 starts at cycle base + 20, not at the cycle's
      // shared mining window (cycle base + 42).
      const quorumHeight = 288000 + 20;

      expect(blockersAt({
        dkgInfo,
        height: quorumHeight + 9,
        dkgStatus: { session: [record('llmq_60_75', quorumHeight, 20)] },
      })).to.have.lengthOf(1);
      expect(blockersAt({
        dkgInfo,
        height: quorumHeight + 10,
        dkgStatus: { session: [record('llmq_60_75', quorumHeight, 20)] },
      })).to.be.empty();
    });

    it('should allow the stop when every session has finished', () => {
      const dkgStatus = {
        session: [
          record('llmq_test_platform', HEIGHT - 20),
          record('llmq_400_60', HEIGHT - 100),
        ],
      };

      expect(blockersAt({ dkgInfo: { active_dkgs: 2, next_dkg: NEXT_DKG_NOT_IMMINENT }, dkgStatus }))
        .to.be.empty();
    });

    it('should block when any single session is still active, even if others have finished', () => {
      const dkgStatus = {
        session: [
          record('llmq_test_platform', HEIGHT - 50),
          record('llmq_400_60', HEIGHT - 5),
        ],
      };

      const blockers = blockersAt({ dkgInfo: { active_dkgs: 2, next_dkg: NEXT_DKG_NOT_IMMINENT }, dkgStatus });

      expect(blockers).to.have.lengthOf(1);
      expect(blockers[0]).to.include({ llmqType: 'llmq_400_60' });
    });

    it('should not need session records when Core reports none', () => {
      expect(blockersAt({ dkgInfo: { active_dkgs: 0, next_dkg: NEXT_DKG_NOT_IMMINENT } }))
        .to.be.empty();
    });

    describe('a record from the previous round at the next round', () => {
      // Core replaces or removes the record shortly after the next round's
      // start block connects; until then this node may be a member again.
      const previousRound = HEIGHT - PLATFORM_INTERVAL;

      it('should block from the next round start block', () => {
        const blockers = blockersAt({
          dkgInfo,
          dkgStatus: { session: [record('llmq_test_platform', previousRound)] },
        });

        expect(blockers).to.have.lengthOf(1);
        expect(blockers[0]).to.include({
          reason: 'new-round', from: HEIGHT, until: HEIGHT + PLATFORM_ACTIVE_BLOCKS,
        });
      });

      it('should not block just before the next round starts', () => {
        expect(blockersAt({
          dkgInfo,
          dkgStatus: { session: [record('llmq_test_platform', previousRound + 1)] },
        })).to.be.empty();
      });

      it('should stop blocking once the next round has finished its Commit phase', () => {
        expect(blockersAt({
          dkgInfo,
          dkgStatus: { session: [record('llmq_test_platform', previousRound - PLATFORM_ACTIVE_BLOCKS)] },
        })).to.be.empty();
      });
    });
  });

  describe('active_dkgs on Core 24.1+', () => {
    it('should block while active_dkgs is positive even if no upcoming DKG blocks', () => {
      const dkgInfo = {
        active_dkgs: 1,
        next_dkg: NEXT_DKG_NOT_IMMINENT,
        upcoming_dkgs: [upcomingDkg(1, false)],
      };

      const blockers = blockersAt({ dkgInfo, coreVersion: CORE_24_1 });

      expect(blockers).to.have.lengthOf(1);
      expect(blockers[0]).to.include({ reason: 'active-count', count: 1, until: null });
    });

    it('should block a recorded session after Core stopped counting it', () => {
      // A ProUpServTx mined mid-session clears Core's proTxHash, so it
      // reports active_dkgs 0 and no upcoming_dkgs while the session runs.
      const blockers = blockersAt({
        dkgInfo: { active_dkgs: 0, next_dkg: NEXT_DKG_NOT_IMMINENT },
        dkgStatus: { session: [record('llmq_test_platform', HEIGHT - 3)] },
        coreVersion: CORE_24_1,
      });

      expect(blockers).to.have.lengthOf(1);
      expect(blockers[0]).to.include({ reason: 'session' });
    });

    it('should block an unrecorded session even when a later member DKG is announced', () => {
      // At a start block Core 24.1 already counts the session, before its
      // DKG worker has recorded it; a DKG announced for later does not
      // account for it.
      const blockers = blockersAt({
        dkgInfo: {
          active_dkgs: 1,
          next_dkg: NEXT_DKG_NOT_IMMINENT,
          upcoming_dkgs: [upcomingDkg(10, true)],
        },
        coreVersion: CORE_24_1,
      });

      expect(blockers.map(({ reason }) => reason)).to.deep.equal(['active-count']);
    });

    it('should use Core\'s count from Core 24.1.0 only', () => {
      expect(hasMembershipActiveDkgs(240000)).to.be.false();
      expect(hasMembershipActiveDkgs(240099)).to.be.false();
      expect(hasMembershipActiveDkgs(240100)).to.be.true();
      expect(hasMembershipActiveDkgs(250000)).to.be.true();
      expect(hasMembershipActiveDkgs('240100')).to.be.false();
      expect(hasMembershipActiveDkgs(null)).to.be.false();
    });
  });

  describe('fail-safe on malformed dkginfo', () => {
    const malformed = {
      'dkgInfo is undefined': undefined,
      'dkgInfo is null': null,
      'next_dkg is missing': { active_dkgs: 0 },
      'next_dkg is non-numeric': { active_dkgs: 0, next_dkg: '24' },
      'next_dkg is NaN': { active_dkgs: 0, next_dkg: NaN },
      'next_dkg is negative': { active_dkgs: 0, next_dkg: -1 },
      'active_dkgs is missing': { next_dkg: NEXT_DKG_NOT_IMMINENT },
      'active_dkgs is non-numeric': { active_dkgs: '1', next_dkg: NEXT_DKG_NOT_IMMINENT },
      'active_dkgs is NaN': { active_dkgs: NaN, next_dkg: NEXT_DKG_NOT_IMMINENT },
      'active_dkgs is negative': { active_dkgs: -1, next_dkg: NEXT_DKG_NOT_IMMINENT },
    };

    Object.entries(malformed).forEach(([description, dkgInfo]) => {
      it(`should block when ${description}`, () => {
        const blockers = blockersAt({ dkgInfo });

        expect(blockers).to.have.lengthOf(1);
        expect(blockers[0]).to.include({ reason: 'unreadable', source: 'quorum dkginfo' });
      });
    });

    it('should block when the block height is not a height', () => {
      const blockers = getDkgStopBlockers({
        dkgInfo: { active_dkgs: 0, next_dkg: NEXT_DKG_NOT_IMMINENT },
        dkgStatus: { session: [] },
        height: undefined,
        coreVersion: CORE_23,
      });

      expect(blockers).to.have.lengthOf(1);
      expect(blockers[0]).to.include({ reason: 'unreadable', from: null, until: null });
    });
  });

  describe('fail-safe on malformed dkgstatus', () => {
    const dkgInfo = { active_dkgs: 1, next_dkg: NEXT_DKG_NOT_IMMINENT };
    const malformed = {
      'a session has an unknown llmqType': { session: [record('llmq_future_unknown', HEIGHT)] },
      'a session is missing quorumHeight': { session: [{ llmqType: 'llmq_test_platform', status: {} }] },
      'a session has a NaN quorumHeight': { session: [record('llmq_test_platform', NaN)] },
      'a session is missing status entirely': { session: [{ llmqType: 'llmq_test_platform' }] },
      'a session starts above the block height': { session: [record('llmq_test_platform', HEIGHT + 5)] },
      'a session entry is null': { session: [null] },
      'dkgStatus is omitted': undefined,
      'dkgStatus.session is not an array': {},
    };

    Object.entries(malformed).forEach(([description, dkgStatus]) => {
      it(`should block when ${description}`, () => {
        const blockers = getBlockersAt(getDkgStopBlockers({
          dkgInfo, dkgStatus: withConnections(dkgStatus), height: HEIGHT, coreVersion: CORE_23,
        }), HEIGHT);

        expect(blockers).to.not.be.empty();
        blockers.forEach((blocker) => {
          expect(blocker).to.include({ reason: 'unreadable', until: null });
        });
      });
    });
  });

  describe('LLMQ table vs Dash Core src/llmq/params.h', () => {
    it('should match every LLMQ type Core defines', () => {
      expect(LLMQ_TYPES.map(({
        id, name, dkgInterval, dkgPhaseBlocks,
      }) => [id, name, dkgInterval, dkgPhaseBlocks])).to.deep.equal([
        [1, 'llmq_50_60', 24, 2],
        [2, 'llmq_400_60', 288, 4],
        [3, 'llmq_400_85', 576, 4],
        [4, 'llmq_100_67', 24, 2],
        [5, 'llmq_60_75', 288, 2],
        [6, 'llmq_25_67', 24, 2],
        [100, 'llmq_test', 24, 2],
        [101, 'llmq_devnet', 24, 2],
        [102, 'llmq_test_v17', 24, 2],
        [103, 'llmq_test_dip0024', 24, 2],
        [104, 'llmq_test_instantsend', 24, 2],
        [105, 'llmq_devnet_dip0024', 48, 2],
        [106, 'llmq_test_platform', 24, 2],
        [107, 'llmq_devnet_platform', 24, 2],
      ]);
    });

    // [type, DKG interval, blocks from a session's start to the end of its
    // Commit phase, quorum index], as Dash Core schedules them.
    [
      ['llmq_60_75', 288, 10, 20],
      ['llmq_400_60', 288, 20, 0],
      ['llmq_400_85', 576, 20, 0],
      ['llmq_100_67', 24, 10, 0],
      ['llmq_50_60', 24, 10, 0],
      ['llmq_25_67', 24, 10, 0],
    ].forEach(([llmqType, interval, activeBlocks, quorumIndex]) => {
      it(`should block a ${llmqType} member for ${activeBlocks} blocks of each session, every ${interval} blocks`, () => {
        const quorumHeight = 288 * 24 * 10 + quorumIndex;
        const isBlockedAtOffset = (offset) => blockersAt({
          dkgInfo: { active_dkgs: 1, next_dkg: NEXT_DKG_NOT_IMMINENT },
          height: quorumHeight + offset,
          dkgStatus: { session: [record(llmqType, quorumHeight, quorumIndex)] },
        }).length > 0;

        expect(isBlockedAtOffset(activeBlocks - 1), 'last block of the session').to.be.true();
        expect(isBlockedAtOffset(activeBlocks), 'after the Commit phase').to.be.false();
        expect(isBlockedAtOffset(interval - 1), 'before the next session').to.be.false();
        expect(isBlockedAtOffset(interval), 'next session start').to.be.true();
        expect(isBlockedAtOffset(interval + activeBlocks - 1), 'next session end').to.be.true();
        expect(isBlockedAtOffset(interval + activeBlocks), 'after the next session').to.be.false();
      });
    });
  });

  describe('a DKG that starts at the tip, before Core records it (Core older than 24.1)', () => {
    // `quorum dkgstatus` lists every enabled quorum with the start of its
    // current session. At a start block Core's DKG worker records the session
    // only after it has computed the members, and nothing says whether this
    // node is one of them until then.
    const startsHere = {
      llmqType: 'llmq_100_67', quorumIndex: 0, pQuorumBaseBlockIndex: HEIGHT,
    };
    const dkgInfo = { active_dkgs: 0, next_dkg: 24 };

    function blockersWith(dkgStatus, options = {}) {
      return getBlockersAt(getDkgStopBlockers({
        dkgInfo, dkgStatus, height: HEIGHT, coreVersion: CORE_23, ...options,
      }), HEIGHT);
    }

    it('should block the start block while Core has no record of the session', () => {
      const blockers = blockersWith({ session: [], quorumConnections: [startsHere] });

      expect(blockers).to.have.lengthOf(1);
      expect(blockers[0]).to.include({
        reason: 'starting', llmqType: 'llmq_100_67', quorumIndex: 0, from: HEIGHT, until: HEIGHT + 1,
      });
    });

    it('should leave the session to its record once Core has one', () => {
      const blockers = blockersWith({
        session: [record('llmq_100_67', HEIGHT)],
        quorumConnections: [startsHere],
      });

      expect(blockers.map(({ reason }) => reason)).to.deep.equal(['session']);
    });

    it('should not block a session that started before the tip', () => {
      expect(blockersWith({
        session: [],
        quorumConnections: [{ ...startsHere, pQuorumBaseBlockIndex: HEIGHT - 1 }],
      })).to.be.empty();
    });

    it('should not block a DKG this node was announced to be no member of', () => {
      expect(blockersWith(
        { session: [], quorumConnections: [startsHere] },
        { knownNonMemberDkgs: new Set([`llmq_100_67:0:${HEIGHT}`]) },
      )).to.be.empty();
    });

    it('should rely on Core\'s count on Core 24.1, which includes such a session', () => {
      expect(blockersWith(
        { session: [], quorumConnections: [startsHere] },
        { coreVersion: CORE_24_1 },
      )).to.be.empty();
    });

    it('should block when quorum connections are missing', () => {
      const blockers = blockersWith({ session: [] });

      expect(blockers).to.have.lengthOf(1);
      expect(blockers[0]).to.include({ reason: 'unreadable', source: 'quorum dkgstatus quorumConnections' });
    });
  });

  describe('getAnnouncedNonMemberDkgs', () => {
    it('should name the announced DKGs this node is known to be no member of', () => {
      expect([...getAnnouncedNonMemberDkgs({
        upcoming_dkgs: [
          upcomingDkg(2, false, 4),
          upcomingDkg(3, true, 4),
          unknownUpcomingDkg(4),
          upcomingDkg(5, false, 250),
        ],
      }, HEIGHT)]).to.deep.equal([`llmq_100_67:0:${HEIGHT + 2}`]);
    });

    it('should name none without membership data', () => {
      expect([...getAnnouncedNonMemberDkgs({ active_dkgs: 0, next_dkg: 3 }, HEIGHT)]).to.be.empty();
    });
  });
});
