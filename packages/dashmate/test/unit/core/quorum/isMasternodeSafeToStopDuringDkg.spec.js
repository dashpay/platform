import isMasternodeSafeToStopDuringDkg, {
  DKG_MINING_WINDOW_START_BY_LLMQ_TYPE,
} from '../../../../src/core/quorum/isMasternodeSafeToStopDuringDkg.js';
import { MIN_BLOCKS_BEFORE_DKG } from '../../../../src/constants.js';

// A value of next_dkg that is safely above MIN_BLOCKS_BEFORE_DKG so the
// imminent-DKG guard never trips in tests that focus on the per-session
// active-window logic.
const NEXT_DKG_NOT_IMMINENT = MIN_BLOCKS_BEFORE_DKG + 5;
const PLATFORM_WINDOW = DKG_MINING_WINDOW_START_BY_LLMQ_TYPE.llmq_test_platform; // 10
const LARGE_QUORUM_WINDOW = DKG_MINING_WINDOW_START_BY_LLMQ_TYPE.llmq_400_60; // 20

describe('isMasternodeSafeToStopDuringDkg', () => {
  describe('imminent DKG guard from dkginfo.next_dkg', () => {
    it('blocks when next_dkg <= MIN_BLOCKS_BEFORE_DKG even when active_dkgs is 0', () => {
      const dkgInfo = { active_dkgs: 0, next_dkg: MIN_BLOCKS_BEFORE_DKG };

      expect(isMasternodeSafeToStopDuringDkg(dkgInfo, { session: [] }, 1000))
        .to.equal(false);
    });

    it('allows the stop when active_dkgs is 0 and next_dkg is past the imminent threshold', () => {
      const dkgInfo = { active_dkgs: 0, next_dkg: MIN_BLOCKS_BEFORE_DKG + 1 };

      expect(isMasternodeSafeToStopDuringDkg(dkgInfo, { session: [] }, 1000))
        .to.equal(true);
    });
  });

  describe('per-session active window from dkgstatus + getblockcount', () => {
    it('blocks a platform session at offset 0 (quorumHeight == currentHeight)', () => {
      const dkgInfo = { active_dkgs: 1, next_dkg: NEXT_DKG_NOT_IMMINENT };
      const dkgStatus = {
        session: [
          { llmqType: 'llmq_test_platform', status: { quorumHeight: 1000 } },
        ],
      };

      expect(isMasternodeSafeToStopDuringDkg(dkgInfo, dkgStatus, 1000)).to.equal(false);
    });

    it('blocks a platform session at the last block of its active window (offset == window - 1)', () => {
      const dkgInfo = { active_dkgs: 1, next_dkg: NEXT_DKG_NOT_IMMINENT };
      const dkgStatus = {
        session: [
          { llmqType: 'llmq_test_platform', status: { quorumHeight: 1000 } },
        ],
      };

      expect(isMasternodeSafeToStopDuringDkg(
        dkgInfo,
        dkgStatus,
        1000 + (PLATFORM_WINDOW - 1),
      )).to.equal(false);
    });

    it('allows the stop once a platform session reaches offset == window (window closed) even with active_dkgs > 0', () => {
      // The regression case this PR cares about: aggregate active_dkgs
      // is still > 0 (a non-platform LLMQ in its window elsewhere on
      // the node, say), but the platform session at offset 10 is past
      // its active phase and should not block.
      const dkgInfo = { active_dkgs: 1, next_dkg: NEXT_DKG_NOT_IMMINENT };
      const dkgStatus = {
        session: [
          { llmqType: 'llmq_test_platform', status: { quorumHeight: 1000 } },
        ],
      };

      expect(isMasternodeSafeToStopDuringDkg(dkgInfo, dkgStatus, 1000 + PLATFORM_WINDOW))
        .to.equal(true);
    });

    it('blocks a non-platform long-window session (llmq_400_60, offset 15) that is still in its 20-block window', () => {
      const dkgInfo = { active_dkgs: 1, next_dkg: NEXT_DKG_NOT_IMMINENT };
      const dkgStatus = {
        session: [
          { llmqType: 'llmq_400_60', status: { quorumHeight: 1000 } },
        ],
      };

      expect(isMasternodeSafeToStopDuringDkg(dkgInfo, dkgStatus, 1015)).to.equal(false);
      // Sanity: confirm 15 is inside the 20-block llmq_400_60 window.
      expect(LARGE_QUORUM_WINDOW).to.be.greaterThan(15);
    });

    it('allows the stop when every session is past its window even with active_dkgs > 0', () => {
      const dkgInfo = { active_dkgs: 2, next_dkg: NEXT_DKG_NOT_IMMINENT };
      const dkgStatus = {
        session: [
          { llmqType: 'llmq_test_platform', status: { quorumHeight: 1000 } },
          { llmqType: 'llmq_400_60', status: { quorumHeight: 900 } },
        ],
      };

      expect(isMasternodeSafeToStopDuringDkg(dkgInfo, dkgStatus, 1020)).to.equal(true);
    });

    it('blocks when any single session is still in its active window, even if others are stale', () => {
      const dkgInfo = { active_dkgs: 2, next_dkg: NEXT_DKG_NOT_IMMINENT };
      const dkgStatus = {
        session: [
          // Stale platform session (offset 50).
          { llmqType: 'llmq_test_platform', status: { quorumHeight: 950 } },
          // Active llmq_400_60 session (offset 5, window 20).
          { llmqType: 'llmq_400_60', status: { quorumHeight: 995 } },
        ],
      };

      expect(isMasternodeSafeToStopDuringDkg(dkgInfo, dkgStatus, 1000)).to.equal(false);
    });

    it('blocks a negative offset (quorumHeight ahead of currentHeight) as inconsistent status', () => {
      // Core should only report locally tracked sessions whose quorum
      // height has already arrived. If the chain tip is behind the
      // reported quorum height while active_dkgs > 0, fail safe.
      const dkgInfo = { active_dkgs: 1, next_dkg: NEXT_DKG_NOT_IMMINENT };
      const dkgStatus = {
        session: [
          { llmqType: 'llmq_test_platform', status: { quorumHeight: 1005 } },
        ],
      };

      expect(isMasternodeSafeToStopDuringDkg(dkgInfo, dkgStatus, 1000)).to.equal(false);
    });
  });

  describe('fail-safe on malformed dkgInfo', () => {
    it('blocks when dkgInfo is undefined', () => {
      expect(isMasternodeSafeToStopDuringDkg(undefined, { session: [] }, 1000))
        .to.equal(false);
    });

    it('blocks when dkgInfo is null', () => {
      expect(isMasternodeSafeToStopDuringDkg(null, { session: [] }, 1000))
        .to.equal(false);
    });

    it('blocks when next_dkg is missing', () => {
      const dkgInfo = { active_dkgs: 0 };

      expect(isMasternodeSafeToStopDuringDkg(dkgInfo, { session: [] }, 1000))
        .to.equal(false);
    });

    it('blocks when next_dkg is non-numeric', () => {
      const dkgInfo = { active_dkgs: 0, next_dkg: '24' };

      expect(isMasternodeSafeToStopDuringDkg(dkgInfo, { session: [] }, 1000))
        .to.equal(false);
    });

    it('blocks when next_dkg is NaN', () => {
      const dkgInfo = { active_dkgs: 0, next_dkg: NaN };

      expect(isMasternodeSafeToStopDuringDkg(dkgInfo, { session: [] }, 1000))
        .to.equal(false);
    });

    it('blocks when next_dkg is negative', () => {
      const dkgInfo = { active_dkgs: 0, next_dkg: -1 };

      expect(isMasternodeSafeToStopDuringDkg(dkgInfo, { session: [] }, 1000))
        .to.equal(false);
    });

    it('blocks when active_dkgs is missing', () => {
      const dkgInfo = { next_dkg: NEXT_DKG_NOT_IMMINENT };

      expect(isMasternodeSafeToStopDuringDkg(dkgInfo, { session: [] }, 1000))
        .to.equal(false);
    });

    it('blocks when active_dkgs is non-numeric', () => {
      const dkgInfo = { active_dkgs: '1', next_dkg: NEXT_DKG_NOT_IMMINENT };

      expect(isMasternodeSafeToStopDuringDkg(dkgInfo, { session: [] }, 1000))
        .to.equal(false);
    });

    it('blocks when active_dkgs is NaN', () => {
      const dkgInfo = { active_dkgs: NaN, next_dkg: NEXT_DKG_NOT_IMMINENT };

      expect(isMasternodeSafeToStopDuringDkg(dkgInfo, { session: [] }, 1000))
        .to.equal(false);
    });

    it('blocks when active_dkgs is negative', () => {
      const dkgInfo = { active_dkgs: -1, next_dkg: NEXT_DKG_NOT_IMMINENT };

      expect(isMasternodeSafeToStopDuringDkg(dkgInfo, { session: [] }, 1000))
        .to.equal(false);
    });
  });

  describe('fail-safe on malformed inputs while active_dkgs > 0', () => {
    it('blocks when a session has an unknown llmqType', () => {
      const dkgInfo = { active_dkgs: 1, next_dkg: NEXT_DKG_NOT_IMMINENT };
      const dkgStatus = {
        session: [
          { llmqType: 'llmq_future_unknown', status: { quorumHeight: 1000 } },
        ],
      };

      expect(isMasternodeSafeToStopDuringDkg(dkgInfo, dkgStatus, 1000)).to.equal(false);
    });

    it('blocks when a session is missing quorumHeight', () => {
      const dkgInfo = { active_dkgs: 1, next_dkg: NEXT_DKG_NOT_IMMINENT };
      const dkgStatus = {
        session: [
          { llmqType: 'llmq_test_platform', status: {} },
        ],
      };

      expect(isMasternodeSafeToStopDuringDkg(dkgInfo, dkgStatus, 1000)).to.equal(false);
    });

    it('blocks when a session is missing status entirely', () => {
      const dkgInfo = { active_dkgs: 1, next_dkg: NEXT_DKG_NOT_IMMINENT };
      const dkgStatus = {
        session: [
          { llmqType: 'llmq_test_platform' },
        ],
      };

      expect(isMasternodeSafeToStopDuringDkg(dkgInfo, dkgStatus, 1000)).to.equal(false);
    });

    it('blocks when dkgStatus is omitted while active_dkgs > 0', () => {
      const dkgInfo = { active_dkgs: 1, next_dkg: NEXT_DKG_NOT_IMMINENT };

      expect(isMasternodeSafeToStopDuringDkg(dkgInfo, undefined, 1000)).to.equal(false);
    });

    it('blocks when dkgStatus.session is not an array while active_dkgs > 0', () => {
      const dkgInfo = { active_dkgs: 1, next_dkg: NEXT_DKG_NOT_IMMINENT };

      expect(isMasternodeSafeToStopDuringDkg(dkgInfo, {}, 1000)).to.equal(false);
    });

    it('blocks when dkgStatus.session is empty while active_dkgs > 0', () => {
      const dkgInfo = { active_dkgs: 1, next_dkg: NEXT_DKG_NOT_IMMINENT };

      expect(isMasternodeSafeToStopDuringDkg(dkgInfo, { session: [] }, 1000)).to.equal(false);
    });

    it('blocks when currentHeight is omitted while active_dkgs > 0', () => {
      const dkgInfo = { active_dkgs: 1, next_dkg: NEXT_DKG_NOT_IMMINENT };
      const dkgStatus = {
        session: [
          { llmqType: 'llmq_test_platform', status: { quorumHeight: 1000 } },
        ],
      };

      expect(isMasternodeSafeToStopDuringDkg(dkgInfo, dkgStatus)).to.equal(false);
    });
  });

  describe('window table sanity vs Dash Core src/llmq/params.h', () => {
    it('exposes the expected dkgMiningWindowStart values', () => {
      expect(DKG_MINING_WINDOW_START_BY_LLMQ_TYPE.llmq_test_platform).to.equal(10);
      expect(DKG_MINING_WINDOW_START_BY_LLMQ_TYPE.llmq_test_dip0024).to.equal(12);
      expect(DKG_MINING_WINDOW_START_BY_LLMQ_TYPE.llmq_400_60).to.equal(20);
      expect(DKG_MINING_WINDOW_START_BY_LLMQ_TYPE.llmq_400_85).to.equal(20);
      expect(DKG_MINING_WINDOW_START_BY_LLMQ_TYPE.llmq_60_75).to.equal(42);
    });
  });
});
