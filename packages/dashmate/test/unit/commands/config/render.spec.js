import fs from 'fs';
import HomeDir from '../../../../src/config/HomeDir.js';
import ConfigFile from '../../../../src/config/configFile/ConfigFile.js';
import ConfigFileJsonRepository from '../../../../src/config/configFile/ConfigFileJsonRepository.js';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import ConfigRenderCommand from '../../../../src/commands/config/render.js';
import writeServiceConfigsFactory from '../../../../src/templates/writeServiceConfigsFactory.js';

describe('Config render command', () => {
  let config;
  let homeDir;
  let mockRenderServiceTemplates;
  let mockWriteServiceConfigs;

  beforeEach(async function it() {
    homeDir = HomeDir.createTemp();
    const getBaseConfig = getBaseConfigFactory(homeDir);

    mockWriteServiceConfigs = this.sinon.stub();
    mockRenderServiceTemplates = this.sinon.stub();

    config = getBaseConfig();
  });

  afterEach(() => {
    homeDir.remove();
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

  it('should repair stale service files from the saved config', async () => {
    const configFile = new ConfigFile(
      [config],
      '4.1.0',
      'abcdef12',
      'base',
      null,
    );
    const configFilePath = homeDir.joinPath('config.json');
    const renderedPath = homeDir.joinPath('base', 'rendered.txt');

    fs.writeFileSync(
      configFilePath,
      `${JSON.stringify(configFile.toObject(), undefined, 2)}\n`,
      'utf8',
    );
    fs.mkdirSync(homeDir.joinPath('base'), { recursive: true });
    fs.writeFileSync(renderedPath, 'stale value', 'utf8');

    const repository = new ConfigFileJsonRepository(
      (data) => data,
      homeDir,
      () => configFile,
    );

    expect(() => repository.update((freshConfigFile) => {
      freshConfigFile.getConfig('base').set('description', 'saved value');
    }, {
      onSaved: () => {
        throw new Error('template write failed');
      },
    })).to.throw('template write failed');

    expect(fs.readFileSync(renderedPath, 'utf8')).to.equal('stale value');

    const savedConfig = repository.read().getConfig('base');
    const renderServiceTemplates = (renderedConfig) => ({
      'rendered.txt': renderedConfig.get('description'),
    });

    await new ConfigRenderCommand().runWithDependencies(
      {},
      {},
      savedConfig,
      renderServiceTemplates,
      writeServiceConfigsFactory(homeDir),
    );

    expect(fs.readFileSync(renderedPath, 'utf8')).to.equal('saved value');
  });
});
