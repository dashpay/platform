import GroupRestartCommand from '../../../../src/commands/group/restart.js';
import getConfigMock from '../../../../src/test/mock/getConfigMock.js';

describe('Group restart command', () => {
  let configGroup;
  let dockerCompose;
  let stopNodeTask;
  let startGroupNodesTask;
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
      configGroup,
    );
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
});
