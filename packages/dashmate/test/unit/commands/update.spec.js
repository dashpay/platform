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
});
