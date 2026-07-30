import fs from 'fs';
import HomeDir from '../../../../src/config/HomeDir.js';
import ConfigFile from '../../../../src/config/configFile/ConfigFile.js';
import ConfigFileJsonRepository from '../../../../src/config/configFile/ConfigFileJsonRepository.js';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import ConfigCreateCommand from '../../../../src/commands/config/create.js';
import ConfigDefaultCommand from '../../../../src/commands/config/default.js';
import ConfigRemoveCommand from '../../../../src/commands/config/remove.js';

/**
 * These commands change the config file as one locked step rather than mutating
 * the config file they were handed at startup. Two properties matter and are
 * asserted for each: the change reaches disk, and the object loaded at startup
 * is left alone - if a command mutated that instead, `BaseCommand.finally()`
 * would save it afterwards and write a snapshot from before the command ran.
 */
describe('Config mutating commands', () => {
  const flags = {};
  const noTemplates = () => {};

  let homeDir;
  let loadedConfigFile;
  let configFileRepository;

  function reread() {
    return new ConfigFileJsonRepository((data) => data, homeDir, () => null).read();
  }

  beforeEach(() => {
    homeDir = HomeDir.createTemp();

    const configFile = new ConfigFile(
      [getBaseConfigFactory(homeDir)()],
      '4.1.0',
      'abcdef12',
      'base',
      null,
    );

    fs.writeFileSync(
      homeDir.joinPath('config.json'),
      `${JSON.stringify(configFile.toObject(), undefined, 2)}\n`,
      'utf8',
    );

    configFileRepository = new ConfigFileJsonRepository((data) => data, homeDir, () => null);
    loadedConfigFile = configFileRepository.read();
  });

  afterEach(() => {
    homeDir.remove();
  });

  describe('config create', () => {
    it('should save the new config without touching the config file loaded at startup', async () => {
      await new ConfigCreateCommand().runWithDependencies(
        { config: 'node1', from: 'base' },
        flags,
        loadedConfigFile,
        configFileRepository,
        noTemplates,
      );

      expect(reread().isConfigExists('node1')).to.be.true();
      expect(loadedConfigFile.isConfigExists('node1')).to.be.false();
      expect(loadedConfigFile.isChanged()).to.be.false();
    });
  });

  describe('config default', () => {
    it('should save the new default without touching the config file loaded at startup', async () => {
      await new ConfigDefaultCommand().runWithDependencies(
        { config: 'base' },
        flags,
        loadedConfigFile,
        configFileRepository,
      );

      expect(reread().getDefaultConfigName()).to.equal('base');
      expect(loadedConfigFile.isChanged()).to.be.false();
    });
  });

  describe('config remove', () => {
    beforeEach(async () => {
      await new ConfigCreateCommand().runWithDependencies(
        { config: 'node1', from: 'base' },
        flags,
        loadedConfigFile,
        configFileRepository,
        noTemplates,
      );

      fs.mkdirSync(homeDir.joinPath('node1'), { recursive: true });
    });

    it('should save the removal and then delete the service directory', async () => {
      await new ConfigRemoveCommand().runWithDependencies(
        { config: 'node1' },
        flags,
        loadedConfigFile,
        { has: () => false },
        homeDir,
        configFileRepository,
      );

      expect(reread().isConfigExists('node1')).to.be.false();
      expect(fs.existsSync(homeDir.joinPath('node1'))).to.be.false();
    });

    // The service files are what a node actually runs from. Deleting them before
    // the removal is saved would leave a config listed in config.json with
    // nothing on disk behind it.
    it('should leave the service directory alone when the removal cannot be saved', async () => {
      await new ConfigRemoveCommand().runWithDependencies(
        { config: 'does-not-exist' },
        flags,
        loadedConfigFile,
        { has: () => false },
        homeDir,
        configFileRepository,
      ).catch(() => {});

      expect(fs.existsSync(homeDir.joinPath('node1'))).to.be.true();
      expect(reread().isConfigExists('node1')).to.be.true();
    });
  });
});
