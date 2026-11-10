import { Listr } from 'listr2';
import stopNodeTaskFactory from '../../../../src/listr/tasks/stopNodeTaskFactory.js';
import { SAFE_STOP_CONFIRMATION_DELAY_MS } from '../../../../src/core/quorum/checkMasternodeSafeToStop.js';
import getConfigMock from '../../../../src/test/mock/getConfigMock.js';
import getCoreRpcMock from '../../../../src/test/mock/getCoreRpcMock.js';

const CORE_24_0 = 240000;
const CORE_24_1 = 240100;

describe('stopNodeTaskFactory', () => {
  let clock;
  let core;
  let rpcClient;
  let config;
  let dockerCompose;
  let stopNodeTask;

  beforeEach(function beforeEach() {
    clock = this.sinon.useFakeTimers();

    core = {
      version: CORE_24_1,
      height: 1012,
      dkgInfo: { active_dkgs: 0, next_dkg: 12, upcoming_dkgs: [] },
      dkgStatus: { session: [] },
    };

    config = getConfigMock(this.sinon);
    config.get.withArgs('core.masternode.enable').returns(true);
    config.getName.returns('mainnet');

    dockerCompose = {
      isNodeRunning: this.sinon.stub().resolves(true),
      stop: this.sinon.stub().resolves(),
    };

    stopNodeTask = stopNodeTaskFactory(
      dockerCompose,
      () => {
        rpcClient = getCoreRpcMock(this.sinon, core);
        return rpcClient;
      },
      async () => '127.0.0.1',
      () => [],
    );
  });

  /**
   * Run a plain stop, advancing the fake clock past any confirmation delays.
   */
  async function stop() {
    const state = {};
    new Listr([{ task: () => stopNodeTask(config) }], { renderer: 'silent' })
      .run({ isForce: false, isSafe: false, platformOnly: false })
      .then(() => { state.stopped = true; }, (e) => { state.error = e; });

    await clock.tickAsync(SAFE_STOP_CONFIRMATION_DELAY_MS * 4);

    return state;
  }

  it('should stop a masternode that no DKG includes', async () => {
    const { stopped } = await stop();

    expect(stopped).to.be.true();
    expect(dockerCompose.stop).to.have.been.calledOnce();
  });

  it('should refuse to stop a masternode during its DKG session', async () => {
    core.dkgInfo = { active_dkgs: 1, next_dkg: 12, upcoming_dkgs: [] };

    const { error } = await stop();

    expect(error.message).to.include('may result in a PoSe ban');
    expect(dockerCompose.stop).to.not.have.been.called();
  });

  it('should refuse a stop on Core 24.0 when a session appears during the confirmation', async () => {
    // At a DKG start block Core 24.0 reports the session only once its DKG
    // worker has recorded it.
    core.version = CORE_24_0;
    core.afterDkgStatus = () => {
      core.dkgInfo = { active_dkgs: 1, next_dkg: 24, upcoming_dkgs: [] };
      core.dkgStatus = {
        session: [{ llmqType: 'llmq_100_67', quorumIndex: 0, status: { quorumHeight: 1012 } }],
      };
      core.afterDkgStatus = undefined;
    };

    const { error } = await stop();

    expect(error.message).to.include('may result in a PoSe ban');
    expect(rpcClient.quorum.withArgs('dkginfo')).to.have.been.calledTwice();
    expect(dockerCompose.stop).to.not.have.been.called();
  });

  it('should refuse to stop when Core reports DKG data dashmate cannot read', async () => {
    // --safe would wait for this forever, so it is not offered.
    core.dkgInfo = { active_dkgs: 1, next_dkg: 12, upcoming_dkgs: [] };
    core.dkgStatus = {
      session: [{ llmqType: 'llmq_future_unknown', quorumIndex: 0, status: { quorumHeight: 1000 } }],
    };

    const { error } = await stop();

    expect(error.message).to.include('dashmate cannot read Core\'s DKG state');
    expect(error.message).to.not.include('--safe');
    expect(dockerCompose.stop).to.not.have.been.called();
  });

  it('should check again when a block connects during a check', async () => {
    core.beforeDkgInfo = () => {
      core.height += 1;
      core.beforeDkgInfo = undefined;
    };

    const { stopped } = await stop();

    expect(stopped).to.be.true();
    expect(rpcClient.quorum.withArgs('dkginfo')).to.have.been.calledTwice();
  });

  it('should give up after three checks while the chain keeps advancing', async () => {
    // Core catching up connects a block during every check.
    core.beforeDkgInfo = () => { core.height += 1; };

    const { error } = await stop();

    expect(error.message).to.include('The chain kept advancing');
    expect(rpcClient.quorum.withArgs('dkginfo')).to.have.been.calledThrice();
    expect(dockerCompose.stop).to.not.have.been.called();
  });

  it('should not wait for DKGs once the local network\'s miner has stopped', async function it() {
    // Group commands stop a local network's miner first. Without it no block
    // comes, so a DKG wait could never end, and with every masternode of the
    // network going down together their DKG produces no commitment to
    // penalize them in.
    const minerConfig = getConfigMock(this.sinon);
    minerConfig.get.withArgs('core.masternode.enable').returns(false);
    minerConfig.get.withArgs('core.miner.enable').returns(true);
    minerConfig.get.withArgs('network').returns('local');
    minerConfig.get.withArgs('group').returns('local');
    minerConfig.getName.returns('local_seed');
    config.get.withArgs('network').returns('local');
    config.get.withArgs('group').returns('local');
    core.dkgInfo = { active_dkgs: 1, next_dkg: 3, upcoming_dkgs: [] };

    const state = {};
    new Listr([
      { task: () => stopNodeTask(minerConfig) },
      { task: () => stopNodeTask(config) },
    ], { renderer: 'silent' })
      .run({ isForce: false, isSafe: true, platformOnly: false })
      .then(() => { state.stopped = true; }, (e) => { state.error = e; });
    await clock.tickAsync(1);

    expect(state.stopped).to.be.true();
    expect(dockerCompose.stop).to.have.been.calledTwice();
  });

  it('should refuse a plain stop at a DKG start that Core has not recorded yet', async () => {
    core.version = CORE_24_0;
    core.dkgInfo = { active_dkgs: 0, next_dkg: 24, upcoming_dkgs: [] };
    core.dkgStatus = {
      session: [],
      quorumConnections: [{ llmqType: 'llmq_100_67', quorumIndex: 0, pQuorumBaseBlockIndex: 1012 }],
    };

    const { error } = await stop();

    expect(error.message).to.include('may result in a PoSe ban');
    expect(dockerCompose.stop).to.not.have.been.called();
  });

  it('should still check a masternode of another network after a local miner stopped', async function it() {
    // A group may hold configs of several networks; stopping a local miner
    // does not stop blocks on any other.
    const minerConfig = getConfigMock(this.sinon);
    minerConfig.get.withArgs('core.masternode.enable').returns(false);
    minerConfig.get.withArgs('core.miner.enable').returns(true);
    minerConfig.get.withArgs('network').returns('local');
    minerConfig.get.withArgs('group').returns('mixed');
    minerConfig.getName.returns('local_seed');
    config.get.withArgs('group').returns('mixed');
    core.dkgInfo = { active_dkgs: 1, next_dkg: 3, upcoming_dkgs: [] };

    const state = {};
    new Listr([
      { task: () => stopNodeTask(minerConfig) },
      { task: () => stopNodeTask(config) },
    ], { renderer: 'silent' })
      .run({ isForce: false, isSafe: false, platformOnly: false })
      .then(() => { state.stopped = true; }, (e) => { state.error = e; });
    await clock.tickAsync(1);

    expect(state.error).to.be.an('error');
    expect(dockerCompose.stop).to.have.been.calledOnce();
  });
});
