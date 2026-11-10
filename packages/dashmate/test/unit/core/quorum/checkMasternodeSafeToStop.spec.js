import checkMasternodeSafeToStop, {
  readCoreVersion,
  SAFE_STOP_CONFIRMATION_DELAY_MS,
} from '../../../../src/core/quorum/checkMasternodeSafeToStop.js';
import { MIN_BLOCKS_BEFORE_DKG } from '../../../../src/constants.js';
import getCoreRpcMock from '../../../../src/test/mock/getCoreRpcMock.js';

const CORE_23 = 230110;
const CORE_24_0_PATCH = 240099;
const CORE_24_1 = 240100;

describe('checkMasternodeSafeToStop', () => {
  let core;
  let rpcClient;
  let clock;

  beforeEach(function beforeEach() {
    clock = this.sinon.useFakeTimers();
    core = {
      version: CORE_23,
      height: 1012,
      dkgInfo: { active_dkgs: 0, next_dkg: 12 },
      dkgStatus: { session: [] },
    };
    rpcClient = getCoreRpcMock(this.sinon, core);
  });

  /**
   * Start a check and run the fake clock until the first check has finished
   * and, on Core older than 24.1, is waiting to be confirmed.
   */
  async function startCheck() {
    const state = {};
    checkMasternodeSafeToStop(rpcClient, core.version).then((result) => { state.result = result; });
    await clock.tickAsync(0);

    return state;
  }

  describe('readCoreVersion', () => {
    it('should return the numeric version Core reports', async () => {
      expect(await readCoreVersion(rpcClient)).to.equal(CORE_23);
    });

    it('should refuse to guess when Core reports no integer version', async () => {
      // Which rule applies depends on the version; reading an older one for
      // Core 24.1 would ignore its session count.
      core.version = '23.1.10';

      await expect(readCoreVersion(rpcClient)).to.be.rejectedWith('Core did not report its version');
    });
  });

  it('should read the chain tip before and after the DKG state', async () => {
    core.version = CORE_24_1;

    await startCheck();

    const [tipBefore, tipAfter] = rpcClient.getBlockchainInfo.getCalls();
    const dkgInfo = rpcClient.quorum.withArgs('dkginfo').getCall(0);
    const dkgStatus = rpcClient.quorum.withArgs('dkgstatus').getCall(0);

    expect(tipBefore.calledBefore(dkgInfo)).to.be.true();
    expect(tipBefore.calledBefore(dkgStatus)).to.be.true();
    expect(dkgInfo.calledBefore(tipAfter)).to.be.true();
    expect(dkgStatus.calledBefore(tipAfter)).to.be.true();
  });

  it('should report unsettled when a block connects during the reads', async () => {
    core.version = CORE_24_1;
    core.beforeDkgInfo = () => { core.height += 1; };

    const { result } = await startCheck();

    expect(result.status).to.equal('unsettled');
  });

  it('should report unsettled when Core changes its session records between the reads', async () => {
    // Core older than 24.1 counts the records dkgstatus lists; it erases a
    // record when the next session of its type starts.
    core.dkgInfo = { active_dkgs: 1, next_dkg: 12 };

    const { result } = await startCheck();

    expect(result.status).to.equal('unsettled');
  });

  it('should report blocked at once without waiting for confirmation', async () => {
    core.dkgInfo = { active_dkgs: 0, next_dkg: MIN_BLOCKS_BEFORE_DKG };

    const { result } = await startCheck();

    expect(result.status).to.equal('blocked');
    expect(result.height).to.equal(1012);
    expect(result.blockers[0].reason).to.equal('imminent');
    expect(rpcClient.getBlockchainInfo).to.have.been.calledTwice();
  });

  it('should confirm a safe result after the confirmation delay on Core older than 24.1', async () => {
    const state = await startCheck();
    await clock.tickAsync(SAFE_STOP_CONFIRMATION_DELAY_MS - 1);

    expect(state.result).to.be.undefined();
    expect(rpcClient.quorum.withArgs('dkginfo')).to.have.been.calledOnce();

    await clock.tickAsync(1);

    expect(state.result.status).to.equal('safe');
    expect(rpcClient.quorum.withArgs('dkginfo')).to.have.been.calledTwice();
  });

  it('should confirm a safe result on a Core 24.0 patch release', async () => {
    core.version = CORE_24_0_PATCH;

    const state = await startCheck();
    await clock.tickAsync(SAFE_STOP_CONFIRMATION_DELAY_MS);

    expect(state.result.status).to.equal('safe');
    expect(rpcClient.quorum.withArgs('dkginfo')).to.have.been.calledTwice();
  });

  it('should start the confirmation delay once the first check has finished', async () => {
    // Every block the first check saw must be old enough for Core to have
    // recorded its sessions.
    const dkgStatusDelayMs = 10000;
    core.dkgStatusDelayMs = dkgStatusDelayMs;

    await startCheck();
    await clock.tickAsync(dkgStatusDelayMs + SAFE_STOP_CONFIRMATION_DELAY_MS - 1);

    expect(rpcClient.quorum.withArgs('dkginfo')).to.have.been.calledOnce();

    await clock.tickAsync(1);

    expect(rpcClient.quorum.withArgs('dkginfo')).to.have.been.calledTwice();
  });

  it('should return the confirming check when it finds a session', async () => {
    const state = await startCheck();

    core.dkgInfo = { active_dkgs: 1, next_dkg: 12 };
    core.dkgStatus = {
      session: [{ llmqType: 'llmq_test_platform', quorumIndex: 0, status: { quorumHeight: 1012 } }],
    };
    await clock.tickAsync(SAFE_STOP_CONFIRMATION_DELAY_MS);

    expect(state.result.status).to.equal('blocked');
    expect(state.result.blockers.find(({ reason }) => reason === 'session'))
      .to.include({ from: 1012, until: 1022 });
  });

  it('should accept up to MIN_BLOCKS_BEFORE_DKG new blocks during the confirmation', async () => {
    const state = await startCheck();
    core.height += MIN_BLOCKS_BEFORE_DKG;
    await clock.tickAsync(SAFE_STOP_CONFIRMATION_DELAY_MS);

    expect(state.result.status).to.equal('safe');
  });

  it('should report unsettled when more blocks arrive during the confirmation than the first check covered', async () => {
    // The first check only ruled out DKGs of this node within
    // MIN_BLOCKS_BEFORE_DKG blocks of its tip.
    const state = await startCheck();
    core.height += MIN_BLOCKS_BEFORE_DKG + 1;
    await clock.tickAsync(SAFE_STOP_CONFIRMATION_DELAY_MS);

    expect(state.result.status).to.equal('unsettled');
  });

  it('should report unsettled when the confirming check finds another block at the same height', async () => {
    // A reorg replaced the first check's tip, so that block may be new.
    const state = await startCheck();
    core.hash = 'reorganized';
    await clock.tickAsync(SAFE_STOP_CONFIRMATION_DELAY_MS);

    expect(state.result.status).to.equal('unsettled');
  });

  it('should report unsettled when the confirming check finds a lower tip', async () => {
    const state = await startCheck();
    core.height -= 1;
    await clock.tickAsync(SAFE_STOP_CONFIRMATION_DELAY_MS);

    expect(state.result.status).to.equal('unsettled');
  });

  it('should not wait for confirmation on Core 24.1', async () => {
    core.version = CORE_24_1;

    const { result } = await startCheck();

    expect(result.status).to.equal('safe');
    expect(rpcClient.quorum.withArgs('dkginfo')).to.have.been.calledOnce();
  });

  it('should not hold Core in initial block download to sessions, without confirmation', async () => {
    // Core runs no DKG session until it has synced, so session records and
    // unrecorded sessions do not apply; announced DKGs still do.
    core.isSyncing = true;
    core.dkgInfo = { active_dkgs: 1, next_dkg: 12 };
    core.dkgStatus = {
      session: [{ llmqType: 'llmq_test_platform', quorumIndex: 0, status: { quorumHeight: 1010 } }],
      quorumConnections: [{ llmqType: 'llmq_test_platform', quorumIndex: 0, pQuorumBaseBlockIndex: 1010 }],
    };

    const { result } = await startCheck();

    expect(result).to.include({ status: 'safe', isSyncing: true });
    expect(rpcClient.quorum.withArgs('dkginfo')).to.have.been.calledOnce();
  });

  it('should report unsettled when a reorg replaced the first check\'s tip and the chain grew', async () => {
    // The replacing branch can put this node in a DKG starting on it that
    // Core has not recorded yet.
    const state = await startCheck();
    const firstTip = core.height;
    core.height += 2;
    core.hashAt = (height) => `other-branch-${height}`;
    await clock.tickAsync(SAFE_STOP_CONFIRMATION_DELAY_MS);

    expect(state.result.status).to.equal('unsettled');
    expect(rpcClient.getBlockHash).to.have.been.calledWith(firstTip);
  });

  it('should treat Core in initial block download as safe even while blocks keep connecting', async () => {
    core.isSyncing = true;
    core.beforeDkgInfo = () => { core.height += 1; };

    const { result } = await startCheck();

    expect(result).to.include({ status: 'safe', isSyncing: true });
  });

  it('should pass on what it knows about DKGs this node is not part of', async () => {
    // A DKG starting at the tip blocks unless announced without this node.
    core.dkgInfo = { active_dkgs: 0, next_dkg: 24 };
    core.dkgStatus = {
      session: [],
      quorumConnections: [{ llmqType: 'llmq_100_67', quorumIndex: 0, pQuorumBaseBlockIndex: 1012 }],
    };

    const blocked = await startCheck();
    expect(blocked.result.status).to.equal('blocked');

    const state = {};
    checkMasternodeSafeToStop(rpcClient, core.version, {
      knownNonMemberDkgs: new Set(['llmq_100_67:0:1012']),
    }).then((result) => { state.result = result; });
    await clock.tickAsync(SAFE_STOP_CONFIRMATION_DELAY_MS);

    expect(state.result.status).to.equal('safe');
  });

  it('should report a replaced chain when the previous check\'s tip is gone', async () => {
    // Anything learned on the replaced branch, such as which DKGs leave this
    // node out, may not hold on the new one.
    core.version = CORE_24_1;
    const state = {};
    checkMasternodeSafeToStop(rpcClient, core.version, {
      ancestor: { height: 1010, hash: 'replaced-1010' },
    }).then((result) => { state.result = result; });
    await clock.tickAsync(0);

    expect(state.result).to.include({ status: 'unsettled', isReorg: true });
  });

  it('should not report a replaced chain when the previous check\'s tip is still there', async () => {
    core.version = CORE_24_1;
    const state = {};
    checkMasternodeSafeToStop(rpcClient, core.version, {
      ancestor: { height: 1010, hash: 'hash-1010' },
    }).then((result) => { state.result = result; });
    await clock.tickAsync(0);

    expect(state.result.status).to.equal('safe');
    expect(state.result.isReorg).to.not.equal(true);
  });

  it('should still block a DKG this node is in while Core syncs', async () => {
    // A Core only a little behind leaves initial block download in time for
    // the DKGs it announces.
    core.isSyncing = true;
    core.version = CORE_24_0_PATCH;
    core.dkgInfo = {
      active_dkgs: 0,
      next_dkg: 2,
      upcoming_dkgs: [{
        llmqType: 106, quorumIndex: 0, quorumHeight: 1014, blocksUntilStart: 2, known: true, isMember: true,
      }],
    };

    const { result } = await startCheck();

    expect(result.status).to.equal('blocked');
    expect(result.blockers.map(({ reason }) => reason)).to.deep.equal(['upcoming']);
  });

  it('should keep the first check\'s DKGs when Core restarts into initial block download during confirmation', async () => {
    // The first check sees a DKG of this node 8 blocks away; the restarted
    // Core no longer reports it.
    core.dkgInfo = {
      active_dkgs: 0,
      next_dkg: 8,
      upcoming_dkgs: [{
        llmqType: 106, quorumIndex: 0, quorumHeight: 1020, blocksUntilStart: 8, known: true, isMember: true,
      }],
    };
    core.version = CORE_24_0_PATCH;

    const state = await startCheck();
    core.isSyncing = true;
    core.height = 1015;
    core.dkgInfo = { active_dkgs: 0, next_dkg: 5 };
    await clock.tickAsync(SAFE_STOP_CONFIRMATION_DELAY_MS);

    expect(state.result.status).to.equal('blocked');
  });
});
