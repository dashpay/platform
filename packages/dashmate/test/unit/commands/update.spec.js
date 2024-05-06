import UpdateCommand from '../../../src/commands/update.js';
import HomeDir from '../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import updateNodeFactory from '../../../src/update/updateNodeFactory.js';
import getConfigMock from '../../../src/test/mock/getConfigMock.js';

describe('Update command', () => {
  let config;
  let mockServicesList;
  let mockGetServicesList;
  let mockDocker;
  let mockDockerStream;
  let mockDockerResponse;

  beforeEach(async function it() {
    config = getConfigMock(this.sinon);
  });

  beforeEach(async function it() {
    const getBaseConfig = getBaseConfigFactory(HomeDir.createTemp());

    config = getBaseConfig();

    mockGetServicesList = this.sinon.stub().callsFake(() => mockServicesList);
    mockDockerStream = {
      on: this.sinon.stub().callsFake((channel, cb) => (channel !== 'error'
        ? cb(Buffer.from(`${JSON.stringify(mockDockerResponse)}\r\n`)) : null)),
    };
    mockDocker = { pull: this.sinon.stub().callsFake((image, cb) => cb(false, mockDockerStream)) };
  });

  it('should just update', async () => {
    mockDockerResponse = { status: 'Status: Image is up to date for' };
    mockServicesList = [{ name: 'fake', image: 'fake', title: 'FAKE' }];

    const command = new UpdateCommand();

    const updateNode = updateNodeFactory(mockGetServicesList, mockDocker);

    await command.runWithDependencies({}, { format: 'json' }, mockDocker, config, updateNode);

    expect(mockGetServicesList).to.have.been.calledOnceWithExactly(config);
    expect(mockDocker.pull).to.have.been.calledOnceWith(mockServicesList[0].image);
  });

  it('should update other services if one of them fails', async function it() {
    const command = new UpdateCommand();
    mockDockerResponse = { status: 'Status: Image is up to date for' };
    mockServicesList = [{ name: 'fake', image: 'fake', title: 'FAKE' },
      { name: 'fake_docker_pull_error', image: 'fake_err_image', title: 'FAKE_ERROR' }];

    // test docker.pull returns error
    mockDocker = {
      pull: this.sinon.stub()
        .callsFake((image, cb) => (image === mockServicesList[1].image ? cb(new Error(), null)
          : cb(false, mockDockerStream))),
    };

    let updateNode = updateNodeFactory(mockGetServicesList, mockDocker);

    await command.runWithDependencies({}, { format: 'json' }, mockDocker, config, updateNode);

    expect(mockGetServicesList).to.have.been.calledOnceWithExactly(config);
    expect(mockDocker.pull.firstCall.firstArg).to.equal(mockServicesList[0].image);
    expect(mockDocker.pull.secondCall.firstArg).to.equal(mockServicesList[1].image);

    // test docker.pull stream returns error
    mockDocker = { pull: this.sinon.stub().callsFake((image, cb) => cb(false, mockDockerStream)) };
    mockDockerStream = {
      on: this.sinon.stub().callsFake((channel, cb) => (channel === 'error' ? cb(new Error()) : null)),
    };

    // reset
    mockGetServicesList = this.sinon.stub().callsFake(() => mockServicesList);
    mockDocker = { pull: this.sinon.stub().callsFake((image, cb) => cb(false, mockDockerStream)) };

    updateNode = updateNodeFactory(mockGetServicesList, mockDocker);

    await command.runWithDependencies({}, { format: 'json' }, mockDocker, config, updateNode);

    expect(mockGetServicesList).to.have.been.calledOnceWithExactly(config);
    expect(mockDocker.pull.firstCall.firstArg).to.equal(mockServicesList[0].image);
    expect(mockDocker.pull.secondCall.firstArg).to.equal(mockServicesList[1].image);
  });
});
