import fs from 'fs';
import HomeDir from '../../../src/config/HomeDir.js';
import ConfigFile from '../../../src/config/configFile/ConfigFile.js';
import ConfigFileJsonRepository from '../../../src/config/configFile/ConfigFileJsonRepository.js';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import startGroupNodesTaskFactory from '../../../src/listr/tasks/startGroupNodesTaskFactory.js';
import { NETWORK_LOCAL } from '../../../src/constants.js';

describe('startGroupNodesTaskFactory', () => {
  let homeDir;
  let configFileRepository;
  let dockerCompose;
  let startGroupNodesTask;
  let writeConfigTemplates;
  let renderedConfig;
  let renderedWhileLocked;

  function createMinerConfig(address) {
    const config = getBaseConfigFactory(homeDir)();

    config.set('network', NETWORK_LOCAL);
    config.set('group', 'local');
    config.set('core.miner.enable', true);
    config.set('core.miner.address', address);

    return config;
  }

  function seedConfigFile(config) {
    const configFile = new ConfigFile(
      [config],
      '4.1.0',
      'abcdef12',
      config.getName(),
      'local',
    );

    fs.writeFileSync(
      homeDir.joinPath('config.json'),
      `${JSON.stringify(configFile.toObject(), undefined, 2)}\n`,
      'utf8',
    );
  }

  beforeEach(function beforeEach() {
    homeDir = HomeDir.createTemp();
    configFileRepository = new ConfigFileJsonRepository(
      (data) => data,
      homeDir,
      () => null,
    );
    dockerCompose = {
      execCommand: this.sinon.stub().resolves(),
    };
    renderedConfig = null;
    renderedWhileLocked = false;
    writeConfigTemplates = this.sinon.stub().callsFake((config) => {
      renderedConfig = config;
      renderedWhileLocked = fs.existsSync(homeDir.joinPath('.config.json.lock'));
    });

    startGroupNodesTask = startGroupNodesTaskFactory(
      dockerCompose,
      this.sinon.stub().resolves(),
      this.sinon.stub().resolves(),
      this.sinon.stub().returns({}),
      {},
      this.sinon.stub().resolves(),
      this.sinon.stub().resolves(),
      this.sinon.stub().resolves(),
      this.sinon.stub().resolves('127.0.0.1'),
      configFileRepository,
      writeConfigTemplates,
    );
  });

  afterEach(() => {
    homeDir.remove();
  });

  it('should persist a generated miner address instead of relying on command finalization', async () => {
    const minerConfig = createMinerConfig(null);
    seedConfigFile(minerConfig);

    await startGroupNodesTask([minerConfig]).run({ waitForReadiness: false });

    const persistedAddress = configFileRepository.read()
      .getConfig(minerConfig.getName())
      .get('core.miner.address');

    expect(persistedAddress).to.be.a('string').and.not.empty();
    expect(minerConfig.get('core.miner.address')).to.equal(persistedAddress);
    expect(dockerCompose.execCommand.firstCall.args[2][2]).to.include(persistedAddress);
    expect(writeConfigTemplates).to.have.been.calledOnce();
    expect(renderedConfig.get('core.miner.address')).to.equal(persistedAddress);
    expect(renderedWhileLocked).to.be.true();
  });

  it('should use an address set after the command loaded its config', async () => {
    const minerConfig = createMinerConfig(null);
    const freshConfig = createMinerConfig('yM6vM3D8R3g7v2YVnN9VqQqxv3ZQGQy2jS');
    seedConfigFile(freshConfig);

    await startGroupNodesTask([minerConfig]).run({ waitForReadiness: false });

    expect(minerConfig.get('core.miner.address'))
      .to.equal('yM6vM3D8R3g7v2YVnN9VqQqxv3ZQGQy2jS');
    expect(dockerCompose.execCommand.firstCall.args[2][2])
      .to.include('yM6vM3D8R3g7v2YVnN9VqQqxv3ZQGQy2jS');
    expect(writeConfigTemplates).to.have.been.calledOnce();
    expect(renderedConfig.get('core.miner.address'))
      .to.equal('yM6vM3D8R3g7v2YVnN9VqQqxv3ZQGQy2jS');
    expect(renderedWhileLocked).to.be.true();
  });
});
