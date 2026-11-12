import checkMasternodeSafeToStop from '../../../../src/core/quorum/checkMasternodeSafeToStop.js';

describe('checkMasternodeSafeToStop', () => {
  let rpcClient;

  beforeEach(function beforeEach() {
    rpcClient = { quorum: this.sinon.stub(), getBlockCount: this.sinon.stub() };
  });

  it('should allow a non-member immediately using one membership response', async () => {
    rpcClient.quorum.withArgs('dkginfo').resolves({
      result: {
        active_dkgs: 0,
        next_dkg: 1,
        upcoming_dkgs: [{ blocksUntilStart: 1, known: true, isMember: false }],
      },
    });
    expect(await checkMasternodeSafeToStop(rpcClient)).to.equal(true);
    expect(rpcClient.quorum).to.have.been.calledOnceWith('dkginfo');
    expect(rpcClient.getBlockCount).to.not.have.been.called();
  });

  it('should block a positive active_dkgs without inspecting dkgstatus', async () => {
    rpcClient.quorum.withArgs('dkginfo').resolves({
      result: {
        active_dkgs: 1,
        next_dkg: 24,
        upcoming_dkgs: [],
      },
    });
    expect(await checkMasternodeSafeToStop(rpcClient)).to.equal(false);
    expect(rpcClient.quorum).to.have.been.calledOnceWith('dkginfo');
    expect(rpcClient.getBlockCount).to.not.have.been.called();
  });

  it('should block a member of an imminent DKG', async () => {
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

  it('should retain the imminent-DKG guard without upcoming_dkgs', async () => {
    rpcClient.quorum.withArgs('dkginfo').resolves({ result: { active_dkgs: 0, next_dkg: 1 } });
    expect(await checkMasternodeSafeToStop(rpcClient)).to.equal(false);
    expect(rpcClient.quorum).to.have.been.calledOnceWith('dkginfo');
  });

  it('should allow a legacy response with no active or imminent DKG', async () => {
    rpcClient.quorum.withArgs('dkginfo').resolves({ result: { active_dkgs: 0, next_dkg: 24 } });
    expect(await checkMasternodeSafeToStop(rpcClient)).to.equal(true);
    expect(rpcClient.quorum).to.have.been.calledOnceWith('dkginfo');
  });

  it('should inspect active sessions for older Core', async () => {
    rpcClient.quorum.withArgs('dkginfo').resolves({ result: { active_dkgs: 1, next_dkg: 24 } });
    rpcClient.quorum.withArgs('dkgstatus').resolves({
      result: { session: [{ llmqType: 'llmq_60_75', status: { quorumHeight: 1000 } }] },
    });
    rpcClient.getBlockCount.resolves({ result: 1001 });
    expect(await checkMasternodeSafeToStop(rpcClient)).to.equal(false);
    expect(rpcClient.quorum.withArgs('dkgstatus')).to.have.been.calledOnce();
    expect(rpcClient.getBlockCount).to.have.been.calledOnce();
  });
});
