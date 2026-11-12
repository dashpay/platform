import waitForDKGWindowPass, {
  CHECK_INTERVAL_MS,
  MAX_FAILED_POLLS,
  MAX_UNREADABLE_POLLS,
} from '../../../../src/core/quorum/waitForDKGWindowPass.js';
import { SAFE_STOP_CONFIRMATION_DELAY_MS } from '../../../../src/core/quorum/checkMasternodeSafeToStop.js';
import getCoreRpcMock from '../../../../src/test/mock/getCoreRpcMock.js';

const CORE_23 = 230110;
const CORE_24_0 = 240000;
const CORE_24_1 = 240100;

// Long enough for at least one more poll, confirmation included. Ticks of
// this length land between polls, so state a test changes afterwards is
// read by the next poll.
const NEXT_POLL_MS = CHECK_INTERVAL_MS + SAFE_STOP_CONFIRMATION_DELAY_MS + 1;

/**
 * A Core RPC client that, while `core.down` is set, rejects every call the
 * way a restarting Core does.
 */
function getRestartableCoreRpcMock(sinon, core) {
  const running = getCoreRpcMock(sinon, core);
  const guard = (method) => sinon.stub().callsFake(async (...args) => {
    if (core.down) {
      throw new Error('connect ECONNREFUSED');
    }

    return running[method](...args);
  });

  return {
    getNetworkInfo: guard('getNetworkInfo'),
    getBlockchainInfo: guard('getBlockchainInfo'),
    getBlockHash: guard('getBlockHash'),
    quorum: guard('quorum'),
  };
}

function track(promise) {
  const state = { resolved: false, error: undefined };
  promise.then(() => { state.resolved = true; }, (e) => { state.error = e; });
  return state;
}

describe('waitForDKGWindowPass', () => {
  let clock;

  beforeEach(function beforeEach() {
    clock = this.sinon.useFakeTimers();
  });

  async function expectStillPolling(waiting, rpcClient) {
    const polls = rpcClient.quorum.withArgs('dkginfo').callCount;

    await clock.tickAsync(NEXT_POLL_MS);

    expect(waiting.error).to.be.undefined();
    expect(waiting.resolved).to.be.false();
    expect(rpcClient.quorum.withArgs('dkginfo').callCount).to.be.greaterThan(polls);
  }

  it('should resolve at once on Core 24.1 when no DKG includes this node', async function it() {
    const rpcClient = getCoreRpcMock(this.sinon, {
      version: CORE_24_1,
      height: 1000,
      dkgInfo: {
        active_dkgs: 0,
        next_dkg: 1,
        upcoming_dkgs: [{
          llmqType: 106, quorumHeight: 1001, blocksUntilStart: 1, known: true, isMember: false,
        }],
      },
      dkgStatus: { session: [] },
    });

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(0);

    expect(waiting.resolved).to.be.true();
    expect(rpcClient.quorum.withArgs('dkginfo')).to.have.been.calledOnce();
  });

  it('should resolve once a safe result is confirmed on Core 24.0', async function it() {
    const rpcClient = getCoreRpcMock(this.sinon, {
      version: CORE_24_0,
      height: 1000,
      dkgInfo: {
        active_dkgs: 0,
        next_dkg: 1,
        upcoming_dkgs: [{
          llmqType: 106, quorumHeight: 1001, blocksUntilStart: 1, known: true, isMember: false,
        }],
      },
      dkgStatus: { session: [] },
    });

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(SAFE_STOP_CONFIRMATION_DELAY_MS - 1);

    expect(waiting.resolved).to.be.false();

    await clock.tickAsync(1);

    expect(waiting.resolved).to.be.true();
    expect(rpcClient.quorum.withArgs('dkginfo')).to.have.been.calledTwice();
  });

  it('should keep waiting while this node is a member of an imminent DKG', async function it() {
    const core = {
      version: CORE_24_1,
      height: 1000,
      dkgInfo: {
        active_dkgs: 0,
        next_dkg: 1,
        upcoming_dkgs: [{
          llmqType: 106, quorumHeight: 1001, blocksUntilStart: 1, known: true, isMember: true,
        }],
      },
      dkgStatus: { session: [] },
    };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await expectStillPolling(waiting, rpcClient);

    // The session started at 1001 and its Commit phase has ended.
    core.height = 1011;
    core.dkgInfo = { active_dkgs: 0, next_dkg: 13, upcoming_dkgs: [] };
    core.dkgStatus = {
      session: [{ llmqType: 'llmq_test_platform', quorumIndex: 0, status: { quorumHeight: 1001 } }],
    };
    await clock.tickAsync(NEXT_POLL_MS);

    expect(waiting.resolved).to.be.true();
  });

  it('should keep waiting on Core 24.1 while active_dkgs is positive', async function it() {
    const core = {
      version: CORE_24_1,
      height: 1000,
      dkgInfo: { active_dkgs: 1, next_dkg: 1, upcoming_dkgs: [] },
      dkgStatus: { session: [] },
    };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await expectStillPolling(waiting, rpcClient);

    core.dkgInfo = { active_dkgs: 0, next_dkg: 1, upcoming_dkgs: [] };
    await clock.tickAsync(NEXT_POLL_MS);

    expect(waiting.resolved).to.be.true();
  });

  it('should wait through an active session and resolve once its Commit phase has ended', async function it() {
    const core = {
      version: CORE_23,
      height: 1005,
      dkgInfo: { active_dkgs: 1, next_dkg: 19 },
      dkgStatus: {
        session: [{ llmqType: 'llmq_test_platform', quorumIndex: 0, status: { quorumHeight: 1000 } }],
      },
    };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await expectStillPolling(waiting, rpcClient);

    core.height = 1010;
    core.dkgInfo = { active_dkgs: 1, next_dkg: 14 };
    await clock.tickAsync(NEXT_POLL_MS);

    expect(waiting.resolved).to.be.true();
  });

  it('should keep waiting while a DKG is imminent and Core reports no membership', async function it() {
    const core = {
      version: CORE_23,
      height: 1000,
      dkgInfo: { active_dkgs: 0, next_dkg: 3 },
      dkgStatus: { session: [] },
    };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await expectStillPolling(waiting, rpcClient);

    // The DKG passed without including this node.
    core.height = 1013;
    core.dkgInfo = { active_dkgs: 0, next_dkg: 14 };
    await clock.tickAsync(NEXT_POLL_MS);

    expect(waiting.resolved).to.be.true();
  });

  it('should give up when Core keeps reporting DKG data dashmate cannot read', async function it() {
    // An unknown quorum type does not go away while dashmate and Core stay
    // as they are, so waiting for it would never end.
    const rpcClient = getCoreRpcMock(this.sinon, {
      version: CORE_23,
      height: 1000,
      dkgInfo: { active_dkgs: 1, next_dkg: 20 },
      dkgStatus: {
        session: [{ llmqType: 'llmq_future_unknown', quorumIndex: 0, status: { quorumHeight: 900 } }],
      },
    });

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(CHECK_INTERVAL_MS * 10);

    expect(waiting.error.message).to.include('dashmate cannot read Core\'s DKG data')
      .and.include('llmq_future_unknown');
    expect(waiting.error.blocker).to.include({ reason: 'unreadable' });
    expect(rpcClient.quorum.withArgs('dkginfo')).to.have.callCount(MAX_UNREADABLE_POLLS);
  });

  it('should count only unreadable polls in a row, starting again after a readable one', async function it() {
    const unreadable = { active_dkgs: 0, next_dkg: 20, upcoming_dkgs: {} };
    const blocked = { active_dkgs: 1, next_dkg: 20, upcoming_dkgs: [] };
    const core = {
      version: CORE_24_1, height: 1000, dkgInfo: unreadable, dkgStatus: { session: [] },
    };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(CHECK_INTERVAL_MS * (MAX_UNREADABLE_POLLS - 1) - 1);
    core.dkgInfo = blocked;
    await clock.tickAsync(CHECK_INTERVAL_MS);
    core.dkgInfo = unreadable;
    await clock.tickAsync(CHECK_INTERVAL_MS * (MAX_UNREADABLE_POLLS - 1));

    expect(waiting.error).to.be.undefined();
    expect(waiting.resolved).to.be.false();
    expect(rpcClient.quorum.withArgs('dkginfo')).to.have.callCount(MAX_UNREADABLE_POLLS * 2 - 1);
  });

  it('should keep waiting through a single read dashmate cannot make sense of', async function it() {
    const core = {
      version: CORE_24_1,
      height: 1000,
      dkgInfo: { active_dkgs: 0, next_dkg: 20, upcoming_dkgs: {} },
      dkgStatus: { session: [] },
    };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(1);

    expect(waiting.resolved).to.be.false();

    core.dkgInfo = { active_dkgs: 0, next_dkg: 20, upcoming_dkgs: [] };
    await clock.tickAsync(NEXT_POLL_MS);

    expect(waiting.resolved).to.be.true();
  });

  it('should report what it waits for, a DKG it saw announced included', async function it() {
    const progressCallback = this.sinon.stub();
    const start = 24 * 5000;
    const core = {
      version: CORE_24_0,
      height: start - 1,
      dkgInfo: {
        active_dkgs: 0,
        next_dkg: 1,
        upcoming_dkgs: [{
          llmqType: 4, quorumIndex: 0, quorumHeight: start, blocksUntilStart: 1, known: true, isMember: true,
        }],
      },
      dkgStatus: { session: [] },
    };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    waitForDKGWindowPass(rpcClient, { progressCallback });
    await clock.tickAsync(1);

    core.height = start;
    core.dkgInfo = { active_dkgs: 0, next_dkg: 24, upcoming_dkgs: [] };
    await clock.tickAsync(NEXT_POLL_MS);

    const announced = {
      reason: 'upcoming', llmqType: 'llmq_100_67', from: start - 6, until: start + 10,
    };
    expect(progressCallback.firstCall.args[0]).to.deep.include({ type: 'blocked', height: start - 1 });
    expect(progressCallback.firstCall.args[0].blockers).to.have.lengthOf(1);
    expect(progressCallback.firstCall.args[0].blockers[0]).to.include(announced);
    expect(progressCallback.lastCall.args[0]).to.deep.include({ type: 'blocked', height: start });
    expect(progressCallback.lastCall.args[0].blockers).to.have.lengthOf(1);
    expect(progressCallback.lastCall.args[0].blockers[0]).to.include(announced);
    // The remembered DKG blocks the start block whatever a confirmation finds.
    expect(progressCallback).to.not.have.been.calledWithMatch({ type: 'confirming' });
  });

  it('should keep a DKG it saw announced alongside Core\'s record of it', async function it() {
    const progressCallback = this.sinon.stub();
    const start = 24 * 5000;
    const core = {
      version: CORE_24_0,
      height: start - 1,
      dkgInfo: {
        active_dkgs: 0,
        next_dkg: 1,
        upcoming_dkgs: [{
          llmqType: 4, quorumIndex: 0, quorumHeight: start, blocksUntilStart: 1, known: true, isMember: true,
        }],
      },
      dkgStatus: { session: [] },
    };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    waitForDKGWindowPass(rpcClient, { progressCallback });
    await clock.tickAsync(1);

    core.height = start + 2;
    core.dkgInfo = { active_dkgs: 1, next_dkg: 22, upcoming_dkgs: [] };
    core.dkgStatus = {
      session: [{ llmqType: 'llmq_100_67', quorumIndex: 0, status: { quorumHeight: start } }],
    };
    await clock.tickAsync(NEXT_POLL_MS);

    // The formatter lists the two as one DKG.
    expect(progressCallback.lastCall.args[0].blockers.map(({ reason }) => reason).sort())
      .to.deep.equal(['session', 'upcoming']);
  });

  it('should report when Core\'s state changed while reading it', async function it() {
    const progressCallback = this.sinon.stub();
    const core = {
      version: CORE_24_1,
      height: 1000,
      dkgInfo: { active_dkgs: 1, next_dkg: 20, upcoming_dkgs: [] },
      dkgStatus: { session: [] },
    };
    core.beforeDkgInfo = () => { core.height += 1; };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    waitForDKGWindowPass(rpcClient, { progressCallback });
    await clock.tickAsync(1);

    expect(progressCallback).to.have.been.calledOnceWith({ type: 'unsettled', height: null });
  });

  it('should ride out Core restarting and judge by the version it comes back as', async function it() {
    // Core 24.0 needs 15 s to confirm a safe result; Core 24.1 does not.
    const progressCallback = this.sinon.stub();
    const core = {
      version: CORE_24_0,
      height: 1005,
      dkgInfo: { active_dkgs: 1, next_dkg: 19, upcoming_dkgs: [] },
      dkgStatus: {
        session: [{ llmqType: 'llmq_test_platform', quorumIndex: 0, status: { quorumHeight: 1000 } }],
      },
    };
    const rpcClient = getRestartableCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient, { progressCallback }));
    await clock.tickAsync(1);

    core.down = true;
    await clock.tickAsync(CHECK_INTERVAL_MS * 2);

    expect(waiting.error).to.be.undefined();
    expect(progressCallback).to.have.been.calledWithMatch({
      type: 'rpc-error', failedPolls: 2, maxFailedPolls: MAX_FAILED_POLLS,
    });

    core.down = false;
    core.version = CORE_24_1;
    core.height = 1010;
    core.dkgInfo = { active_dkgs: 0, next_dkg: 14, upcoming_dkgs: [] };
    await clock.tickAsync(CHECK_INTERVAL_MS + 1);

    expect(waiting.resolved).to.be.true();
  });

  it('should count only failed RPC calls in a row, starting again after one that works', async function it() {
    const core = {
      version: CORE_24_1,
      height: 1000,
      dkgInfo: { active_dkgs: 1, next_dkg: 20, upcoming_dkgs: [] },
      dkgStatus: { session: [] },
      down: true,
    };
    const rpcClient = getRestartableCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(CHECK_INTERVAL_MS * (MAX_FAILED_POLLS - 1) - 1);
    core.down = false;
    await clock.tickAsync(CHECK_INTERVAL_MS);
    core.down = true;
    await clock.tickAsync(CHECK_INTERVAL_MS * (MAX_FAILED_POLLS - 1));

    expect(waiting.error).to.be.undefined();
    expect(waiting.resolved).to.be.false();
  });

  it('should give up, leaving the node running, after too many failed RPC calls in a row', async function it() {
    const core = {
      version: CORE_24_1,
      height: 1000,
      dkgInfo: { active_dkgs: 0, next_dkg: 20, upcoming_dkgs: [] },
      dkgStatus: { session: [] },
      down: true,
    };
    const rpcClient = getRestartableCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(CHECK_INTERVAL_MS * (MAX_FAILED_POLLS + 1));

    expect(waiting.error.message).to.equal(`Core did not answer ${MAX_FAILED_POLLS} checks in a row, `
      + 'so the node was not stopped:\nconnect ECONNREFUSED');
    expect(rpcClient.getNetworkInfo).to.have.callCount(MAX_FAILED_POLLS);
  });

  it('should stop waiting on Core 24.0 when active_dkgs counts only finished sessions', async function it() {
    // Core 24.0 reports upcoming_dkgs but counts lingering debug records in
    // active_dkgs, so a masternode that was in any recent DKG never sees 0.
    const height = 288000 + 12;

    const rpcClient = getCoreRpcMock(this.sinon, {
      version: CORE_24_0,
      height,
      dkgInfo: {
        active_dkgs: 2,
        next_dkg: 1,
        upcoming_dkgs: [{
          llmqType: 5,
          quorumIndex: 20,
          quorumHeight: 288000 + 20,
          blocksUntilStart: 8,
          known: true,
          isMember: false,
        }, {
          llmqType: 5,
          quorumIndex: 14,
          quorumHeight: 288000 + 14,
          blocksUntilStart: 2,
          known: true,
          isMember: false,
        }],
      },
      // This cycle's index 25 has not started, so last cycle's record stays;
      // this hour's llmq_100_67 session ended two blocks ago.
      dkgStatus: {
        session: [
          { llmqType: 'llmq_60_75', quorumIndex: 25, status: { quorumHeight: 288000 - 288 + 25 } },
          { llmqType: 'llmq_100_67', quorumIndex: 0, status: { quorumHeight: 288000 } },
        ],
      },
    });

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(60000);

    expect(waiting.resolved).to.be.true();
  });

  it('should keep waiting at a rotation cycle start on Core 24.0 until this node\'s index has passed', async function it() {
    // Last cycle this node was in llmq_60_75 index 5. The new cycle's index 5
    // has started and Core has not replaced that record yet; Core announces
    // only the indices still to come, none of which include this node.
    const cycleBase = 288000;
    const core = {
      version: CORE_24_0,
      height: cycleBase + 5,
      dkgInfo: {
        active_dkgs: 1,
        next_dkg: 1,
        upcoming_dkgs: [6, 7, 8].map((quorumIndex) => ({
          llmqType: 5,
          quorumIndex,
          quorumHeight: cycleBase + quorumIndex,
          blocksUntilStart: quorumIndex - 5,
          known: true,
          isMember: false,
        })),
      },
      dkgStatus: {
        session: [{ llmqType: 'llmq_60_75', quorumIndex: 5, status: { quorumHeight: cycleBase - 288 + 5 } }],
      },
    };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await expectStillPolling(waiting, rpcClient);

    // Ten blocks later Core has removed the record: this node is not in the
    // new index 5.
    core.height = cycleBase + 15;
    core.dkgInfo = { active_dkgs: 0, next_dkg: 1, upcoming_dkgs: [] };
    core.dkgStatus = { session: [] };
    await clock.tickAsync(NEXT_POLL_MS);

    expect(waiting.resolved).to.be.true();
  });

  it('should keep waiting through an announced DKG while Core has not recorded its session', async function it() {
    // At the start block Core erases last round's record, computes the new
    // session's members and only then records it. On a busy node that can
    // outlast the confirmation delay.
    const start = 24 * 5000;
    const core = {
      version: CORE_24_0,
      height: start - 1,
      dkgInfo: {
        active_dkgs: 0,
        next_dkg: 1,
        upcoming_dkgs: [{
          llmqType: 4, quorumIndex: 0, quorumHeight: start, blocksUntilStart: 1, known: true, isMember: true,
        }],
      },
      dkgStatus: { session: [] },
    };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(1);

    core.height = start;
    core.dkgInfo = { active_dkgs: 0, next_dkg: 24, upcoming_dkgs: [] };
    await expectStillPolling(waiting, rpcClient);
    await expectStillPolling(waiting, rpcClient);

    // The session has passed its Commit phase.
    core.height = start + 10;
    core.dkgInfo = { active_dkgs: 0, next_dkg: 14, upcoming_dkgs: [] };
    await clock.tickAsync(NEXT_POLL_MS);

    expect(waiting.resolved).to.be.true();
  });

  it('should end a rotated DKG session once its own Commit phase is over', async function it() {
    // llmq_60_75 index 31 starts at cycle base + 31; ten blocks later this
    // member has sent everything its DKG needs, although the cycle's shared
    // mining window only opens at cycle base + 42.
    const quorumHeight = 288000 + 31;

    const rpcClient = getCoreRpcMock(this.sinon, {
      version: CORE_23,
      height: quorumHeight + 10,
      dkgInfo: { active_dkgs: 1, next_dkg: 7 },
      dkgStatus: {
        session: [{ llmqType: 'llmq_60_75', quorumIndex: 31, status: { quorumHeight } }],
      },
    });

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(60000);

    expect(waiting.resolved).to.be.true();
  });

  it('should keep waiting at a DKG start block while Core still shows the previous round', async function it() {
    // The new llmq_100_67 round starts at `height`. Core has not replaced
    // last round's record yet, so it looks 24 blocks old, past its window.
    const height = 24 * 5000;
    const core = {
      version: CORE_23,
      height,
      dkgInfo: { active_dkgs: 1, next_dkg: 24 },
      dkgStatus: {
        session: [{ llmqType: 'llmq_100_67', quorumIndex: 0, status: { quorumHeight: height - 24 } }],
      },
    };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(60000);
    await expectStillPolling(waiting, rpcClient);

    // Core records the new session; it ends ten blocks after it started.
    core.dkgStatus = {
      session: [{ llmqType: 'llmq_100_67', quorumIndex: 0, status: { quorumHeight: height } }],
    };
    core.height = height + 10;
    core.dkgInfo = { active_dkgs: 1, next_dkg: 14 };
    await clock.tickAsync(60000);

    expect(waiting.resolved).to.be.true();
  });

  it('should not stop at a DKG start that Core has not recorded yet', async function it() {
    // At the start block Core answers before its DKG worker has created the
    // session record; a moment later the record appears.
    const height = 24 * 5000;
    const core = {
      version: CORE_23,
      height,
      dkgInfo: { active_dkgs: 0, next_dkg: 24 },
      dkgStatus: { session: [] },
    };
    core.afterDkgStatus = () => {
      core.dkgInfo = { active_dkgs: 1, next_dkg: 24 };
      core.dkgStatus = {
        session: [{ llmqType: 'llmq_100_67', quorumIndex: 0, status: { quorumHeight: height } }],
      };
      core.afterDkgStatus = undefined;
    };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(60000);

    await expectStillPolling(waiting, rpcClient);
  });

  it('should not trust DKG reads that saw a newer tip than the height read', async function it() {
    // The first check is safe: the member DKG is still 7 blocks away. Before
    // the confirming check reads Core's DKG state, the chain reaches that
    // DKG's start block, which Core has not recorded yet. The confirming check
    // also sees more new blocks than the first covered; checkMasternodeSafeToStop
    // tests the torn read on its own.
    const height = 24 * 5000 - 7;
    const memberDkgStart = height + 7;
    const core = {
      version: CORE_24_0,
      height,
      dkgInfo: {
        active_dkgs: 0,
        next_dkg: 7,
        upcoming_dkgs: [{
          llmqType: 4,
          quorumIndex: 0,
          quorumHeight: memberDkgStart,
          blocksUntilStart: 7,
          known: true,
          isMember: true,
        }],
      },
      dkgStatus: { session: [] },
    };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(0);

    // Six blocks arrive while the first verdict waits to be confirmed.
    core.height = height + 6;
    core.beforeDkgInfo = () => {
      core.height = memberDkgStart;
      core.dkgInfo = { active_dkgs: 0, next_dkg: 24, upcoming_dkgs: [] };
      core.beforeDkgInfo = undefined;
    };
    await clock.tickAsync(20000);

    // Core has now recorded the session.
    core.dkgInfo = { active_dkgs: 1, next_dkg: 24, upcoming_dkgs: [] };
    core.dkgStatus = {
      session: [{ llmqType: 'llmq_100_67', quorumIndex: 0, status: { quorumHeight: memberDkgStart } }],
    };
    await clock.tickAsync(60000);

    await expectStillPolling(waiting, rpcClient);
  });

  it('should keep waiting on Core 24.1 while a session runs after the proTxHash was cleared', async function it() {
    // A ProUpServTx mined mid-session clears Core's proTxHash: dkginfo then
    // reports no active DKG and no upcoming_dkgs, but the session continues.
    const height = 24 * 5000 + 3;
    const core = {
      version: CORE_24_1,
      height,
      dkgInfo: { active_dkgs: 0, next_dkg: 21 },
      dkgStatus: {
        session: [{ llmqType: 'llmq_100_67', quorumIndex: 0, status: { quorumHeight: height - 3 } }],
      },
    };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(60000);
    await expectStillPolling(waiting, rpcClient);

    // The session has passed its Commit phase.
    core.height = height + 7;
    core.dkgInfo = { active_dkgs: 0, next_dkg: 14 };
    await clock.tickAsync(NEXT_POLL_MS);

    expect(waiting.resolved).to.be.true();
  });

  it('should not stop at a DKG start reached in Core\'s record gap, however long it lasts', async function it() {
    // Waiting starts right at the start block: no earlier poll saw the DKG
    // announced, and Core takes longer than the confirmation delay to record
    // the session, or never does because this node is not a member.
    const start = 24 * 5000;
    const core = {
      version: CORE_24_0,
      height: start,
      dkgInfo: { active_dkgs: 0, next_dkg: 24, upcoming_dkgs: [] },
      dkgStatus: {
        session: [],
        quorumConnections: [{ llmqType: 'llmq_100_67', quorumIndex: 0, pQuorumBaseBlockIndex: start }],
      },
    };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(60000);
    await expectStillPolling(waiting, rpcClient);

    // Without a record, nothing shows this node is not a member, so the
    // session blocks until its Commit phase ends.
    core.height = start + 1;
    core.dkgInfo = { active_dkgs: 0, next_dkg: 23, upcoming_dkgs: [] };
    await expectStillPolling(waiting, rpcClient);

    core.height = start + 10;
    core.dkgInfo = { active_dkgs: 0, next_dkg: 14, upcoming_dkgs: [] };
    await clock.tickAsync(NEXT_POLL_MS);

    expect(waiting.resolved).to.be.true();
  });

  it('should not stop in a session Core has not recorded when waiting starts after its start block', async function it() {
    const start = 24 * 5000;
    const rpcClient = getCoreRpcMock(this.sinon, {
      version: CORE_24_0,
      height: start + 1,
      dkgInfo: { active_dkgs: 0, next_dkg: 23, upcoming_dkgs: [] },
      dkgStatus: {
        session: [],
        quorumConnections: [{ llmqType: 'llmq_100_67', quorumIndex: 0, pQuorumBaseBlockIndex: start }],
      },
    });

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(60000);

    await expectStillPolling(waiting, rpcClient);
  });

  it('should not hold a DKG start it saw announced without this node', async function it() {
    const start = 24 * 5000;
    const core = {
      version: CORE_24_0,
      height: start - 1,
      dkgInfo: {
        active_dkgs: 0,
        next_dkg: 1,
        upcoming_dkgs: [{
          llmqType: 4, quorumIndex: 0, quorumHeight: start, blocksUntilStart: 1, known: true, isMember: false,
        }],
      },
      dkgStatus: { session: [] },
    };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(1);

    // Interrupt the first confirmation, so the wait checks again at the start.
    core.height = start;
    core.dkgInfo = { active_dkgs: 0, next_dkg: 24, upcoming_dkgs: [] };
    core.dkgStatus = {
      session: [],
      quorumConnections: [{ llmqType: 'llmq_100_67', quorumIndex: 0, pQuorumBaseBlockIndex: start }],
    };
    core.beforeDkgInfo = () => {
      core.beforeDkgInfo = undefined;
      core.height = start + 7;
    };
    await clock.tickAsync(SAFE_STOP_CONFIRMATION_DELAY_MS);
    core.height = start;
    await clock.tickAsync(NEXT_POLL_MS);

    expect(waiting.resolved).to.be.true();
  });

  it('should keep blocking a DKG it saw announced after Core restarts and forgets the session', async function it() {
    // Core keeps its session records in memory. Restarted mid-session it
    // reports no session, no announcement and no count for the running DKG.
    const start = 24 * 5000;
    const core = {
      version: CORE_24_0,
      height: start - 1,
      dkgInfo: {
        active_dkgs: 0,
        next_dkg: 1,
        upcoming_dkgs: [{
          llmqType: 4, quorumIndex: 0, quorumHeight: start, blocksUntilStart: 1, known: true, isMember: true,
        }],
      },
      dkgStatus: { session: [] },
    };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(1);

    core.height = start + 2;
    core.dkgInfo = { active_dkgs: 1, next_dkg: 22, upcoming_dkgs: [] };
    core.dkgStatus = {
      session: [{ llmqType: 'llmq_100_67', quorumIndex: 0, status: { quorumHeight: start } }],
    };
    await clock.tickAsync(NEXT_POLL_MS);

    core.height = start + 3;
    core.dkgInfo = { active_dkgs: 0, next_dkg: 21, upcoming_dkgs: [] };
    core.dkgStatus = { session: [] };
    await expectStillPolling(waiting, rpcClient);
  });

  it('should treat a Core version it cannot read as a failed check', async function it() {
    // Whether Core's session count applies depends on the version.
    const progressCallback = this.sinon.stub();
    const core = {
      version: undefined,
      height: 1000,
      dkgInfo: { active_dkgs: 0, next_dkg: 20, upcoming_dkgs: [] },
      dkgStatus: { session: [] },
    };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient, { progressCallback }));
    await clock.tickAsync(1);

    expect(waiting.resolved).to.be.false();
    expect(progressCallback).to.have.been.calledWithMatch({ type: 'rpc-error', failedPolls: 1 });
    expect(rpcClient.quorum).to.not.have.been.called();

    core.version = CORE_24_1;
    await clock.tickAsync(CHECK_INTERVAL_MS);

    expect(waiting.resolved).to.be.true();
  });

  it('should count only unreadable polls in a row, starting again after an interrupted one', async function it() {
    const core = {
      version: CORE_24_1,
      height: 1000,
      dkgInfo: { active_dkgs: 0, next_dkg: 20, upcoming_dkgs: {} },
      dkgStatus: { session: [] },
      down: false,
    };
    const rpcClient = getRestartableCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(CHECK_INTERVAL_MS * (MAX_UNREADABLE_POLLS - 1) - 1);
    core.down = true;
    await clock.tickAsync(CHECK_INTERVAL_MS);
    core.down = false;
    await clock.tickAsync(CHECK_INTERVAL_MS * (MAX_UNREADABLE_POLLS - 1));

    expect(waiting.error).to.be.undefined();
    expect(waiting.resolved).to.be.false();
  });

  it('should forget DKGs announced without this node when a reorg replaces their chain', async function it() {
    // On the replacing branch the work block differs, so this node may be a
    // member after all.
    const start = 24 * 5000;
    const core = {
      version: CORE_24_0,
      height: start - 1,
      dkgInfo: {
        active_dkgs: 0,
        next_dkg: 1,
        upcoming_dkgs: [{
          llmqType: 4, quorumIndex: 0, quorumHeight: start, blocksUntilStart: 1, known: true, isMember: false,
        }],
      },
      dkgStatus: { session: [] },
    };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(1);

    // While the first result waits to be confirmed, a reorg replaces the tip
    // and the replacing branch reaches the DKG's start.
    core.hashAt = (height) => `other-branch-${height}`;
    core.height = start;
    core.dkgInfo = { active_dkgs: 0, next_dkg: 24, upcoming_dkgs: [] };
    core.dkgStatus = {
      session: [],
      quorumConnections: [{ llmqType: 'llmq_100_67', quorumIndex: 0, pQuorumBaseBlockIndex: start }],
    };
    await clock.tickAsync(NEXT_POLL_MS * 2);

    await expectStillPolling(waiting, rpcClient);
  });

  it('should not hold a session it saw announced without this node after its start block', async function it() {
    const start = 24 * 5000;
    const core = {
      version: CORE_24_0,
      height: start - 1,
      dkgInfo: {
        active_dkgs: 1,
        next_dkg: 1,
        upcoming_dkgs: [{
          llmqType: 4, quorumIndex: 0, quorumHeight: start, blocksUntilStart: 1, known: true, isMember: false,
        }],
      },
      dkgStatus: {
        // Another session keeps the node busy until the announced one has started.
        session: [{ llmqType: 'llmq_400_60', quorumIndex: 0, status: { quorumHeight: start - 12 } }],
      },
    };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(1);

    const connections = [{ llmqType: 'llmq_100_67', quorumIndex: 0, pQuorumBaseBlockIndex: start }];
    core.height = start + 1;
    core.dkgInfo = { active_dkgs: 1, next_dkg: 23, upcoming_dkgs: [] };
    core.dkgStatus = { ...core.dkgStatus, quorumConnections: connections };
    await clock.tickAsync(CHECK_INTERVAL_MS);

    // The other session has ended; the announced one is still running.
    core.height = start + 8;
    core.dkgInfo = { active_dkgs: 1, next_dkg: 16, upcoming_dkgs: [] };
    await clock.tickAsync(NEXT_POLL_MS * 2);

    expect(waiting.resolved).to.be.true();
  });

  it('should keep what a check announced when Core restarts during its confirmation', async function it() {
    // The first check announces a member DKG for later and is confirmed 15 s
    // on; Core restarts in between and comes back without the announcement.
    const start = 24 * 5000;
    const core = {
      version: CORE_24_0,
      height: start - 7,
      dkgInfo: {
        active_dkgs: 0,
        next_dkg: 7,
        upcoming_dkgs: [{
          llmqType: 4, quorumIndex: 0, quorumHeight: start, blocksUntilStart: 7, known: true, isMember: true,
        }],
      },
      dkgStatus: { session: [] },
    };
    const rpcClient = getRestartableCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(1);

    core.down = true;
    await clock.tickAsync(SAFE_STOP_CONFIRMATION_DELAY_MS);

    // Back after the DKG started, as Core 24.1 that has not worked out its
    // proTxHash yet: nothing it reports shows the DKG.
    core.down = false;
    core.version = CORE_24_1;
    core.height = start + 2;
    core.dkgInfo = { active_dkgs: 0, next_dkg: 22 };
    await clock.tickAsync(NEXT_POLL_MS);

    await expectStillPolling(waiting, rpcClient);
  });

  it('should keep blocking a DKG it saw announced when Core restarts into initial block download', async function it() {
    // A restarted Core reports initial block download, yet it may finish
    // syncing in time for the DKG this node is a member of.
    const start = 24 * 5000;
    const core = {
      version: CORE_24_0,
      height: start - 2,
      dkgInfo: {
        active_dkgs: 0,
        next_dkg: 2,
        upcoming_dkgs: [{
          llmqType: 4, quorumIndex: 0, quorumHeight: start, blocksUntilStart: 2, known: true, isMember: true,
        }],
      },
      dkgStatus: { session: [] },
    };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(1);

    core.isSyncing = true;
    core.dkgInfo = { active_dkgs: 0, next_dkg: 2, upcoming_dkgs: [] };
    await expectStillPolling(waiting, rpcClient);

    // Once the DKG's Commit phase has ended, a syncing Core may stop.
    core.height = start + 10;
    await clock.tickAsync(NEXT_POLL_MS);

    expect(waiting.resolved).to.be.true();
  });

  it('should keep what a check announced when its confirming read is discarded', async function it() {
    // The first check announces a member DKG for later; a block connects
    // during the read confirming it, so that read is discarded.
    const start = 24 * 5000;
    const core = {
      version: CORE_24_0,
      height: start - 8,
      dkgInfo: {
        active_dkgs: 0,
        next_dkg: 8,
        upcoming_dkgs: [{
          llmqType: 4, quorumIndex: 0, quorumHeight: start, blocksUntilStart: 8, known: true, isMember: true,
        }],
      },
      dkgStatus: { session: [] },
    };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(1);

    core.beforeDkgInfo = () => {
      core.height += 1;
      core.beforeDkgInfo = undefined;
    };
    await clock.tickAsync(SAFE_STOP_CONFIRMATION_DELAY_MS);

    // After the DKG started, Core reports nothing that shows it, as Core 24.1
    // that has not worked out its proTxHash yet.
    core.version = CORE_24_1;
    core.height = start + 2;
    core.dkgInfo = { active_dkgs: 0, next_dkg: 22 };
    await clock.tickAsync(CHECK_INTERVAL_MS);

    await expectStillPolling(waiting, rpcClient);
  });

  it('should keep what a check announced when Core changes its session records during the confirming read', async function it() {
    // The first check announces a member DKG for later; the read confirming
    // it finds Core 24.0 counting a session record dkgstatus does not list.
    const start = 24 * 5000;
    const core = {
      version: CORE_24_0,
      height: start - 8,
      dkgInfo: {
        active_dkgs: 0,
        next_dkg: 8,
        upcoming_dkgs: [{
          llmqType: 4, quorumIndex: 0, quorumHeight: start, blocksUntilStart: 8, known: true, isMember: true,
        }],
      },
      dkgStatus: { session: [] },
    };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(1);

    core.dkgInfo = { ...core.dkgInfo, active_dkgs: 1 };
    await clock.tickAsync(SAFE_STOP_CONFIRMATION_DELAY_MS);

    // After the DKG started, Core reports nothing that shows it, as Core 24.1
    // that has not worked out its proTxHash yet.
    core.version = CORE_24_1;
    core.height = start + 2;
    core.dkgInfo = { active_dkgs: 0, next_dkg: 22 };
    await clock.tickAsync(CHECK_INTERVAL_MS);

    await expectStillPolling(waiting, rpcClient);
  });

  it('should judge each poll by the Core version answering it', async function it() {
    // Core is replaced between polls, with no failed call to tell.
    const start = 24 * 5000;
    const core = {
      version: CORE_24_1,
      height: start + 1,
      dkgInfo: { active_dkgs: 1, next_dkg: 23, upcoming_dkgs: [] },
      dkgStatus: { session: [] },
    };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(1);

    // Core 24.0 counts records, and has none yet for the running session.
    core.version = CORE_24_0;
    core.dkgInfo = { active_dkgs: 0, next_dkg: 23, upcoming_dkgs: [] };
    core.dkgStatus = {
      session: [],
      quorumConnections: [{ llmqType: 'llmq_100_67', quorumIndex: 0, pQuorumBaseBlockIndex: start }],
    };

    await expectStillPolling(waiting, rpcClient);
  });

  it('should keep blocking a running session after Core restarts and forgets it', async function it() {
    // Restarted Core keeps no session record, and Core 24.1 without its
    // proTxHash yet reports no count or announcement for the session.
    const start = 24 * 5000;
    const core = {
      version: CORE_24_1,
      height: start + 3,
      dkgInfo: { active_dkgs: 1, next_dkg: 21 },
      dkgStatus: {
        session: [{ llmqType: 'llmq_100_67', quorumIndex: 0, status: { quorumHeight: start } }],
      },
    };
    const rpcClient = getCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(1);

    core.height = start + 4;
    core.dkgInfo = { active_dkgs: 0, next_dkg: 20 };
    core.dkgStatus = { session: [] };
    await expectStillPolling(waiting, rpcClient);

    core.height = start + 10;
    core.dkgInfo = { active_dkgs: 0, next_dkg: 14 };
    await clock.tickAsync(NEXT_POLL_MS);

    expect(waiting.resolved).to.be.true();
  });

  it('should keep blocking a running session through failed calls and a Core restart', async function it() {
    // Waiting starts after the session's announcement is gone; a failed call
    // and a restart later, Core reports nothing of the session.
    const start = 24 * 5000;
    const core = {
      version: CORE_24_1,
      height: start + 3,
      dkgInfo: { active_dkgs: 1, next_dkg: 21 },
      dkgStatus: {
        session: [{ llmqType: 'llmq_100_67', quorumIndex: 0, status: { quorumHeight: start } }],
      },
    };
    const rpcClient = getRestartableCoreRpcMock(this.sinon, core);

    const waiting = track(waitForDKGWindowPass(rpcClient));
    await clock.tickAsync(1);

    core.down = true;
    await clock.tickAsync(CHECK_INTERVAL_MS);

    core.down = false;
    core.height = start + 4;
    core.dkgInfo = { active_dkgs: 0, next_dkg: 20 };
    core.dkgStatus = { session: [] };
    await expectStillPolling(waiting, rpcClient);

    core.height = start + 10;
    core.dkgInfo = { active_dkgs: 0, next_dkg: 14 };
    await clock.tickAsync(NEXT_POLL_MS);

    expect(waiting.resolved).to.be.true();
  });
});
