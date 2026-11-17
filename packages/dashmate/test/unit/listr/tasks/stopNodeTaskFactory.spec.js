import { Listr, ListrEventType } from 'listr2';
import stopNodeTaskFactory from '../../../../src/listr/tasks/stopNodeTaskFactory.js';
import { SAFE_STOP_CONFIRMATION_DELAY_MS } from '../../../../src/core/quorum/checkMasternodeSafeToStop.js';
import { CHECK_INTERVAL_MS } from '../../../../src/core/quorum/waitForDKGWindowPass.js';
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
    config.get.withArgs('network').returns('mainnet');
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
   * Run the stop task as a command would, collecting what the DKG wait
   * prints. `ctx` holds the flags of the running command.
   */
  function run(ctx) {
    const state = { outputs: [] };

    new Listr([{
      task: () => {
        const listr = stopNodeTask(config);
        listr.tasks
          .find(({ title }) => title === 'Wait for DKG window to pass')
          .subscribe(({ type, data }) => {
            if (type === ListrEventType.DATA) {
              state.outputs.push(data);
            }
          });

        return listr;
      },
    }], { renderer: 'silent' })
      .run({ platformOnly: false, ...ctx })
      .then(() => { state.stopped = true; }, (e) => { state.error = e; });

    return state;
  }

  /**
   * Run a plain stop, advancing the fake clock past any confirmation delays.
   */
  async function stop(ctx = { isForce: false, isSafe: false }) {
    const state = run(ctx);

    await clock.tickAsync(SAFE_STOP_CONFIRMATION_DELAY_MS * 4);

    return state;
  }

  it('should stop a masternode that no DKG includes', async () => {
    const { stopped } = await stop();

    expect(stopped).to.be.true();
    expect(dockerCompose.stop).to.have.been.calledOnce();
  });

  it('should refuse to stop a masternode during its DKG session and say until when', async () => {
    core.dkgInfo = { active_dkgs: 1, next_dkg: 12, upcoming_dkgs: [] };
    core.dkgStatus = {
      session: [{ llmqType: 'llmq_100_67', quorumIndex: 0, status: { quorumHeight: 1008 } }],
    };

    const { error } = await stop();

    expect(error.message.split('\n')).to.deep.equal([
      'Stopping now risks a PoSe penalty for this masternode:',
      'llmq_100_67 DKG at block 1008 runs until block 1018',
      'Earliest safe stop: block 1018, in 6+ blocks (~15m)',
      'Retry then, use --safe to wait and stop automatically,',
      'or --force to stop now and accept the PoSe risk',
    ]);
    expect(dockerCompose.stop).to.not.have.been.called();
  });

  it('should only suggest the flags the running command has', async () => {
    // group restart has --safe but no --force; core reindex the reverse.
    core.dkgInfo = { active_dkgs: 0, next_dkg: 3, upcoming_dkgs: [] };
    core.version = 230110;
    delete core.dkgInfo.upcoming_dkgs;

    const { error: groupRestart } = await stop({ isSafe: false });
    const { error: reindex } = await stop({ isForce: false });

    expect(groupRestart.message).to.include('--safe').and.not.include('--force');
    expect(reindex.message).to.include('--force').and.not.include('--safe');
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

    expect(error.message).to.include('risks a PoSe penalty');
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

    expect(error.message).to.include('dashmate cannot read Core\'s DKG data');
    expect(error.message).to.include('llmq_future_unknown');
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

    expect(error.message).to.include('Core\'s DKG state kept changing');
    expect(rpcClient.quorum.withArgs('dkginfo')).to.have.been.calledThrice();
    expect(dockerCompose.stop).to.not.have.been.called();
  });

  it('should show what a --safe stop waits for and estimate how long', async () => {
    core.dkgInfo = { active_dkgs: 0, next_dkg: 12, upcoming_dkgs: [] };
    core.dkgStatus = {
      session: [{ llmqType: 'llmq_100_67', quorumIndex: 0, status: { quorumHeight: 1008 } }],
    };

    const state = run({ isForce: false, isSafe: true });
    await clock.tickAsync(CHECK_INTERVAL_MS * 3 + 1);

    // Mainnet blocks are 2.5 minutes apart.
    expect(state.outputs).to.deep.equal([[
      'At block 1012, waiting for:',
      'llmq_100_67 DKG at block 1008 runs until block 1018',
      'Earliest safe stop: block 1018, in 6+ blocks (~15m)',
    ].join('\n')]);

    core.height = 1013;
    await clock.tickAsync(CHECK_INTERVAL_MS * 3);

    // Only the block that connected changed what there is to say.
    expect(state.outputs).to.have.lengthOf(2);
    expect(state.outputs[1]).to.include('in 5+ blocks (~13m)');

    core.height = 1018;
    await clock.tickAsync(CHECK_INTERVAL_MS);

    expect(state.stopped).to.be.true();
    expect(dockerCompose.stop).to.have.been.calledOnce();
  });

  it('should estimate in blocks only on a local network', async () => {
    config.get.withArgs('network').returns('local');
    core.dkgStatus = {
      session: [{ llmqType: 'llmq_test_platform', quorumIndex: 0, status: { quorumHeight: 1008 } }],
    };

    const state = run({ isForce: false, isSafe: true });
    await clock.tickAsync(0);

    expect(state.outputs[0]).to.include('Earliest safe stop: block 1018, in 6+ blocks')
      .and.not.include('~');
  });

  it('should estimate with the block spacing of each network', async () => {
    core.dkgStatus = {
      session: [{ llmqType: 'llmq_test_platform', quorumIndex: 0, status: { quorumHeight: 1008 } }],
    };

    config.get.withArgs('network').returns('testnet');
    const testnet = run({ isForce: false, isSafe: true });
    await clock.tickAsync(1);

    config.get.withArgs('network').returns('devnet');
    config.get.withArgs('core.devnet.powTargetSpacing').returns(20);
    const devnet = run({ isForce: false, isSafe: true });
    await clock.tickAsync(1);

    expect(testnet.outputs[0]).to.match(/in 6\+ blocks \(~15m\)$/);
    expect(devnet.outputs[0]).to.match(/in 6\+ blocks \(~2m\)$/);
  });

  it('should give up a --safe stop on DKG data dashmate cannot read, offering only the flags it has', async () => {
    core.dkgInfo = { active_dkgs: 0, next_dkg: 12, upcoming_dkgs: {} };

    const safeStop = run({ isForce: false, isSafe: true });
    const groupRestart = run({ isSafe: true });
    await clock.tickAsync(CHECK_INTERVAL_MS * 4);

    expect(safeStop.error.message.split('\n')).to.deep.equal([
      'dashmate cannot read Core\'s DKG data:',
      'quorum dkginfo upcoming_dkgs',
      'Update dashmate, or use --force to stop now',
    ]);
    expect(groupRestart.error.message.split('\n').at(-1)).to.equal('Update dashmate');
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

    expect(error.message).to.include('risks a PoSe penalty');
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
