import { Listr } from 'listr2';
import GroupRestartCommand from '../../../../src/commands/group/restart.js';
import getConfigMock from '../../../../src/test/mock/getConfigMock.js';

describe('Group restart command', () => {
  let configGroup;
  let dockerCompose;
  let stopNodeTask;
  let startGroupNodesTask;
  let buildServicesTask;
  let command;

  beforeEach(function it() {
    configGroup = [getConfigMock(this.sinon), getConfigMock(this.sinon)];
    configGroup.forEach((config, index) => {
      config.get.withArgs('group').returns('local');
      config.getName.returns(`local_${index}`);
    });

    dockerCompose = {
      pullMissingImages: this.sinon.stub().resolves([]),
    };

    stopNodeTask = this.sinon.stub().resolves();
    startGroupNodesTask = this.sinon.stub().resolves();
    buildServicesTask = this.sinon.stub().resolves();

    command = new GroupRestartCommand();
  });

  /**
   * @return {Promise<void>}
   */
  function run() {
    return command.runWithDependencies(
      {},
      { verbose: false, safe: false },
      dockerCompose,
      stopNodeTask,
      startGroupNodesTask,
      buildServicesTask,
      configGroup,
    );
  }

  /**
   * Ask one node of the group to build Drive from local sources, the way a
   * development group is configured
   *
   * @param {Config} config
   */
  function buildDriveFromSources(config) {
    config.get.withArgs('platform.enable').returns(true);
    config.get.withArgs('platform.drive.abci.docker.build.enabled').returns(true);
  }

  it('should not stop any node when a required image can not be pulled', async () => {
    dockerCompose.pullMissingImages
      .withArgs(configGroup[1])
      .rejects(new Error('Failed to pull image dashpay/drive:4: no space left on device'));

    const error = await run().then(() => null, (e) => e);

    expect(error, 'restart must fail instead of stopping the group').to.not.equal(null);

    // The command hides the reason behind MuteOneLineError for the CLI output
    expect(error.getError().message).to.include('no space left on device');

    expect(stopNodeTask).to.have.not.been.called();
    expect(startGroupNodesTask).to.have.not.been.called();
  });

  it('should make sure images of every node are present before stopping the first one', async () => {
    await run();

    expect(dockerCompose.pullMissingImages).to.have.been.calledTwice();
    expect(dockerCompose.pullMissingImages).to.have.been.calledBefore(stopNodeTask);
    expect(stopNodeTask).to.have.been.calledTwice();
    expect(startGroupNodesTask).to.have.been.calledOnce();
  });

  // An image built from local sources is in no registry, so pulling cannot
  // confirm it. Leaving the build to the group start runs it after every node
  // has been stopped, which is the outage this command exists to avoid.
  it('should build local images before stopping the first node', async () => {
    const [buildConfig] = configGroup;

    buildDriveFromSources(buildConfig);

    await run();

    expect(buildServicesTask).to.have.been.calledOnceWith(buildConfig);
    expect(buildServicesTask).to.have.been.calledBefore(stopNodeTask);
  });

  it('should not stop any node when a local image can not be built', async () => {
    buildDriveFromSources(configGroup[0]);

    buildServicesTask.rejects(new Error('failed to solve: process did not complete'));

    const error = await run().then(() => null, (e) => e);

    expect(error, 'restart must fail instead of stopping the group').to.not.equal(null);
    expect(error.getError().message).to.include('failed to solve');

    expect(stopNodeTask).to.have.not.been.called();
    expect(startGroupNodesTask).to.have.not.been.called();
  });

  // The group start builds the same images itself, and a second build would
  // make every restart of a development group pay for the whole build twice
  it('should tell the group start the images are already built', async function it() {
    buildDriveFromSources(configGroup[0]);

    let observedSkipBuildServices;

    startGroupNodesTask = this.sinon.stub().callsFake(() => new Listr([{
      task: (ctx) => {
        observedSkipBuildServices = ctx.skipBuildServices;
      },
    }]));

    await run();

    expect(observedSkipBuildServices).to.be.true();
  });

  it('should not build anything for a group that uses released images', async () => {
    await run();

    expect(buildServicesTask).to.have.not.been.called();
  });
});
