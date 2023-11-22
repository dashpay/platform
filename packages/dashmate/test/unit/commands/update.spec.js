import UpdateCommand from "../../../src/commands/update.js";
import HomeDir from "../../../src/config/HomeDir.js";
import getBaseConfigFactory from "../../../configs/defaults/getBaseConfigFactory.js";
import updateNodeFactory from "../../../src/update/updateNodeFactory.js";
import getConfigMock from "../../../src/test/mock/getConfigMock.js";

describe('Update command', () => {
  let config;

  beforeEach(async function it() {
    config = getConfigMock(this.sinon);
  });

  beforeEach(async () => {
    const getBaseConfig = getBaseConfigFactory(HomeDir.createTemp());

    config = getBaseConfig();
  });

  describe('Update dashmate command', () => {
    it('should just update', async function it () {
      const command = new UpdateCommand();

      const mockGetServicesList = this.sinon.stub();
      const mockDocker = this.sinon.stub()

      const updateNode = updateNodeFactory(mockGetServicesList, mockDocker)

      await command.runWithDependencies({}, {format: 'json'}, mockDocker, config, updateNode);
    });
  });
});
