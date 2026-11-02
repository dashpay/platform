import { Listr } from 'listr2';
import restartNodeTaskFactory from '../../../../src/listr/tasks/restartNodeTaskFactory.js';
import getConfigMock from '../../../../src/test/mock/getConfigMock.js';

describe('restartNodeTask', () => {
  let config;
  let dockerCompose;
  let startNodeTask;
  let stopNodeTask;
  let buildServicesTask;
  let getConfigProfiles;
  let restartNodeTask;

  beforeEach(function it() {
    config = getConfigMock(this.sinon);
    config.get.withArgs('dashmate.helper.docker.build.enabled').returns(false);
    config.get.withArgs('platform.drive.abci.docker.build.enabled').returns(false);
    config.get.withArgs('platform.dapi.rsDapi.docker.build.enabled').returns(false);

    dockerCompose = {
      pullMissingImages: this.sinon.stub().resolves([]),
    };

    getConfigProfiles = this.sinon.stub().returns(['core', 'platform', 'platform-dapi-rs']);

    startNodeTask = this.sinon.stub().returns(new Listr([{ task: () => {} }]));
    stopNodeTask = this.sinon.stub().returns(new Listr([{ task: () => {} }]));
    buildServicesTask = this.sinon.stub().returns(new Listr([{ task: () => {} }]));

    restartNodeTask = restartNodeTaskFactory(
      startNodeTask,
      stopNodeTask,
      buildServicesTask,
      dockerCompose,
      getConfigProfiles,
    );
  });

  it('should not stop running services when a required image can not be pulled', async () => {
    dockerCompose.pullMissingImages.rejects(
      new Error('Failed to pull image dashpay/drive:4: no space left on device'),
    );

    await expect(restartNodeTask(config).run({}))
      .to.be.rejectedWith('no space left on device');

    expect(stopNodeTask).to.have.not.been.called();
    expect(startNodeTask).to.have.not.been.called();
  });

  it('should make sure all images are present before stopping the node', async () => {
    await restartNodeTask(config).run({});

    expect(dockerCompose.pullMissingImages).to.have.been.calledOnce();
    expect(dockerCompose.pullMissingImages.firstCall.args[0]).to.equal(config);
    expect(dockerCompose.pullMissingImages.firstCall.args[1].profiles).to.deep.equal([]);
    expect(dockerCompose.pullMissingImages).to.have.been.calledBefore(stopNodeTask);
    expect(stopNodeTask).to.have.been.calledOnceWithExactly(config);
    expect(startNodeTask).to.have.been.calledOnceWithExactly(config);
  });

  it('should not pull images of services a platform only restart leaves alone', async () => {
    await restartNodeTask(config).run({ platformOnly: true });

    expect(dockerCompose.pullMissingImages.firstCall.args[1].profiles)
      .to.deep.equal(['platform', 'platform-dapi-rs']);
  });

  it('should report pull progress while the node is still running', async () => {
    dockerCompose.pullMissingImages.callsFake(async (pullConfig, { onProgress }) => {
      onProgress('dashpay/drive:4: Downloading [====>  ] 12MB/45MB');

      return [];
    });

    const tasks = restartNodeTask(config);

    await tasks.run({});

    const [pullTask] = tasks.tasks.filter((task) => task.title === 'Pull missing images');

    expect(pullTask.output).to.equal('dashpay/drive:4: Downloading [====>  ] 12MB/45MB');
  });
});
