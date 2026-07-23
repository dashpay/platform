import waitForDKGWindowPass from '../../../../src/core/quorum/waitForDKGWindowPass.js';

const CHECK_INTERVAL_MS = 10000;

describe('waitForDKGWindowPass', () => {
  let rpcClient;

  beforeEach(function beforeEach() {
    rpcClient = {
      quorum: this.sinon.stub(),
      getBlockCount: this.sinon.stub(),
    };
  });

  it('resolves immediately when there are no active sessions and no imminent cycle', async () => {
    rpcClient.quorum.withArgs('dkginfo').resolves({ result: { active_dkgs: 0, next_dkg: 24 } });

    await waitForDKGWindowPass(rpcClient);

    expect(rpcClient.quorum).to.have.been.calledOnceWith('dkginfo');
    expect(rpcClient.getBlockCount).to.not.have.been.called();
  });

  it('waits through an active platform session and resolves once the window has passed', async function it() {
    // Block height advances from 1005 (offset 5, in window) to 1010
    // (offset 10, window closed) between polls; active_dkgs stays > 0
    // to model Dash Core's lingering aggregate counter.
    const clock = this.sinon.useFakeTimers();

    rpcClient.quorum.withArgs('dkginfo').resolves({ result: { active_dkgs: 1, next_dkg: 20 } });
    rpcClient.quorum.withArgs('dkgstatus').resolves({
      result: {
        session: [
          { llmqType: 'llmq_test_platform', status: { quorumHeight: 1000 } },
        ],
      },
    });
    rpcClient.getBlockCount
      .onFirstCall().resolves({ result: 1005 })
      .onSecondCall().resolves({ result: 1010 });

    const promise = waitForDKGWindowPass(rpcClient);

    // Drain microtasks so the first iteration completes its three RPCs
    // and parks on `wait(CHECK_INTERVAL_MS)`.
    await clock.tickAsync(0);
    expect(rpcClient.getBlockCount).to.have.been.calledOnce();

    // Advance past the wait so the second iteration runs and returns.
    await clock.tickAsync(CHECK_INTERVAL_MS);
    await promise;

    expect(rpcClient.getBlockCount).to.have.been.calledTwice();
  });

  it('keeps waiting while next_dkg is imminent even when there are no sessions', async function it() {
    const clock = this.sinon.useFakeTimers();

    rpcClient.quorum.withArgs('dkginfo')
      .onFirstCall()
      .resolves({ result: { active_dkgs: 0, next_dkg: 3 } })
      .onSecondCall()
      .resolves({ result: { active_dkgs: 0, next_dkg: 24 } });

    const promise = waitForDKGWindowPass(rpcClient);

    await clock.tickAsync(0);
    expect(rpcClient.quorum.withArgs('dkginfo')).to.have.been.calledOnce();

    await clock.tickAsync(CHECK_INTERVAL_MS);
    await promise;

    expect(rpcClient.quorum.withArgs('dkginfo')).to.have.been.calledTwice();
  });

  it('fails safe (keeps waiting) when active_dkgs > 0 and a session has an unknown llmqType', async function it() {
    const clock = this.sinon.useFakeTimers();

    rpcClient.quorum.withArgs('dkginfo')
      .onFirstCall()
      .resolves({ result: { active_dkgs: 1, next_dkg: 20 } })
      .onSecondCall()
      .resolves({ result: { active_dkgs: 0, next_dkg: 20 } });
    rpcClient.quorum.withArgs('dkgstatus').resolves({
      result: { session: [{ llmqType: 'llmq_future_unknown', status: { quorumHeight: 1000 } }] },
    });
    rpcClient.getBlockCount.resolves({ result: 1000 });

    const promise = waitForDKGWindowPass(rpcClient);

    await clock.tickAsync(0);
    expect(rpcClient.quorum.withArgs('dkgstatus')).to.have.been.calledOnce();

    await clock.tickAsync(CHECK_INTERVAL_MS);
    await promise;

    expect(rpcClient.quorum.withArgs('dkgstatus')).to.have.been.calledOnce();
  });
});
