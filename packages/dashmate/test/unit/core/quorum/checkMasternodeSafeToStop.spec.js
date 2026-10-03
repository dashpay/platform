import checkMasternodeSafeToStop, {
  SAFE_STOP_CONFIRMATION_DELAY_MS,
} from '../../../../src/core/quorum/checkMasternodeSafeToStop.js';

describe('checkMasternodeSafeToStop', () => {
  let rpcClient;

  beforeEach(function beforeEach() {
    rpcClient = {
      quorum: this.sinon.stub(),
      getBlockCount: this.sinon.stub(),
    };
  });

  it('should return true without confirmation when Core reports no membership data', async () => {
    rpcClient.quorum.withArgs('dkginfo').resolves({ result: { active_dkgs: 0, next_dkg: 24 } });

    expect(await checkMasternodeSafeToStop(rpcClient)).to.equal(true);
    expect(rpcClient.quorum).to.have.been.calledOnceWith('dkginfo');
  });

  it('should return false without confirmation when the first check is unsafe', async () => {
    rpcClient.quorum.withArgs('dkginfo').resolves({
      result: {
        active_dkgs: 0,
        next_dkg: 1,
        upcoming_dkgs: [{ blocksUntilStart: 1, known: true, isMember: true }],
      },
    });

    expect(await checkMasternodeSafeToStop(rpcClient)).to.equal(false);
    expect(rpcClient.quorum).to.have.been.calledOnceWith('dkginfo');
  });

  it('should confirm a safe verdict when membership data cleared an imminent DKG', async function it() {
    const clock = this.sinon.useFakeTimers();

    rpcClient.quorum.withArgs('dkginfo').resolves({
      result: {
        active_dkgs: 0,
        next_dkg: 1,
        upcoming_dkgs: [{ blocksUntilStart: 1, known: true, isMember: false }],
      },
    });

    const promise = checkMasternodeSafeToStop(rpcClient);

    await clock.tickAsync(0);
    expect(rpcClient.quorum.withArgs('dkginfo')).to.have.been.calledOnce();

    await clock.tickAsync(SAFE_STOP_CONFIRMATION_DELAY_MS);

    expect(await promise).to.equal(true);
    expect(rpcClient.quorum.withArgs('dkginfo')).to.have.been.calledTwice();
    expect(rpcClient.getBlockCount).to.not.have.been.called();
  });

  it('should return false when confirmation reveals a member session that started at the tip', async function it() {
    // A rotated session starting at the tip block is not listed in
    // upcoming_dkgs, and active_dkgs only counts it once Core has
    // initialized the session shortly after the block is connected.
    const clock = this.sinon.useFakeTimers();

    rpcClient.quorum.withArgs('dkginfo')
      .onFirstCall()
      .resolves({ result: { active_dkgs: 0, next_dkg: 1, upcoming_dkgs: [] } })
      .onSecondCall()
      .resolves({ result: { active_dkgs: 1, next_dkg: 1, upcoming_dkgs: [] } });
    rpcClient.quorum.withArgs('dkgstatus').resolves({
      result: {
        session: [
          { llmqType: 'llmq_60_75', status: { quorumHeight: 1000 } },
        ],
      },
    });
    rpcClient.getBlockCount.resolves({ result: 1000 });

    const promise = checkMasternodeSafeToStop(rpcClient);

    await clock.tickAsync(SAFE_STOP_CONFIRMATION_DELAY_MS);

    expect(await promise).to.equal(false);
    expect(rpcClient.quorum.withArgs('dkgstatus')).to.have.been.calledOnce();
  });
});
