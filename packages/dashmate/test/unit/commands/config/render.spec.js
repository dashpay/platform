import HomeDir from '../../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import ConfigRenderCommand from '../../../../src/commands/config/render.js';

describe('Config render command', () => {
  let config;
  let mockRenderServiceTemplates;
  let mockWriteServiceConfigs;

  beforeEach(async function it() {
    const getBaseConfig = getBaseConfigFactory(HomeDir.createTemp());

    mockWriteServiceConfigs = this.sinon.stub();
    mockRenderServiceTemplates = this.sinon.stub();

    config = getBaseConfig();
  });

  it('should call render and write', async () => {
    const command = new ConfigRenderCommand();

    await command.runWithDependencies(
      {},
      {},
      config,
      mockRenderServiceTemplates,
      mockWriteServiceConfigs,
    );

    expect(mockRenderServiceTemplates).to.have.been.calledOnceWithExactly(config);
    expect(mockWriteServiceConfigs).to.have.been.calledOnceWith(config.getName());
  });
});
