import fs from 'fs';
import path from 'path';
import writeFileAtomic from 'write-file-atomic';
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
        homeDir,
      );

      expect(reread().isConfigExists('node1')).to.be.true();
      expect(loadedConfigFile.isConfigExists('node1')).to.be.false();
      expect(loadedConfigFile.isChanged()).to.be.false();
    });

    // `config.json` resolves to the repository's own file, not to a service
    // directory. The cleanup-retry path reaches names no config is listed
    // under, so without a guard this deletes every configuration and reports
    // success.
    it('should refuse to remove a name that resolves to the config file itself', async () => {
      const before = fs.readFileSync(homeDir.joinPath('config.json'), 'utf8');

      for (const name of ['config.json', 'CONFIG.JSON', 'config.json.']) {
        // eslint-disable-next-line no-await-in-loop
        await expect(new ConfigRemoveCommand().runWithDependencies(
          { config: name },
          flags,
          loadedConfigFile,
          { has: () => false },
          homeDir,
          configFileRepository,
        ), name).to.be.rejectedWith('reserves for its own files');
      }

      expect(fs.existsSync(homeDir.joinPath('config.json'))).to.be.true();
      expect(fs.readFileSync(homeDir.joinPath('config.json'), 'utf8')).to.equal(before);
    });

    it('should reject a service directory that is not owned by config.json', async () => {
      const keyPath = homeDir.joinPath('node1', 'platform', 'gateway', 'ssl', 'private.key');

      fs.mkdirSync(path.dirname(keyPath), { recursive: true });
      fs.writeFileSync(keyPath, 'interrupted-create-private-key');

      await expect(new ConfigCreateCommand().runWithDependencies(
        { config: 'node1', from: 'base' },
        flags,
        loadedConfigFile,
        configFileRepository,
        noTemplates,
        homeDir,
      )).to.be.rejectedWith('dashmate config remove node1');

      expect(reread().isConfigExists('node1')).to.be.false();
      expect(fs.readFileSync(keyPath, 'utf8')).to.equal('interrupted-create-private-key');

      await new ConfigRemoveCommand().runWithDependencies(
        { config: 'node1' },
        flags,
        loadedConfigFile,
        { has: () => false },
        homeDir,
        configFileRepository,
      );

      expect(fs.existsSync(homeDir.joinPath('node1'))).to.be.false();

      await new ConfigCreateCommand().runWithDependencies(
        { config: 'node1', from: 'base' },
        flags,
        loadedConfigFile,
        configFileRepository,
        noTemplates,
        homeDir,
      );

      expect(reread().isConfigExists('node1')).to.be.true();
    });
  });

  describe('config default', () => {
    // Pointed at a config that is NOT already the default, so deleting the save
    // would fail this rather than passing on the starting state.
    it('should save the new default without touching the config file loaded at startup', async () => {
      await new ConfigCreateCommand().runWithDependencies(
        { config: 'node1', from: 'base' },
        flags,
        loadedConfigFile,
        configFileRepository,
        noTemplates,
        homeDir,
      );

      expect(reread().getDefaultConfigName()).to.equal('base');

      await new ConfigDefaultCommand().runWithDependencies(
        { config: 'node1' },
        flags,
        loadedConfigFile,
        configFileRepository,
      );

      expect(reread().getDefaultConfigName()).to.equal('node1');
      expect(loadedConfigFile.getDefaultConfigName()).to.equal('base');
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
        homeDir,
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

    it('should keep private files live until the removal is durable', async function it() {
      const keyPath = homeDir.joinPath('node1', 'platform', 'gateway', 'ssl', 'private.key');

      fs.mkdirSync(path.dirname(keyPath), { recursive: true });
      fs.writeFileSync(keyPath, 'previous-node-private-key');

      this.sinon.stub(writeFileAtomic, 'sync').callsFake(() => {
        expect(fs.readFileSync(keyPath, 'utf8')).to.equal('previous-node-private-key');

        throw new Error('write failed');
      });

      await expect(new ConfigRemoveCommand().runWithDependencies(
        { config: 'node1' },
        flags,
        loadedConfigFile,
        { has: () => false },
        homeDir,
        configFileRepository,
      )).to.be.rejectedWith('write failed');

      expect(reread().isConfigExists('node1')).to.be.true();
      expect(fs.readFileSync(keyPath, 'utf8')).to.equal('previous-node-private-key');
    });

    it('should retry cleanup after the config is already absent', async function it() {
      const keyPath = homeDir.joinPath('node1', 'platform', 'gateway', 'ssl', 'private.key');

      fs.mkdirSync(path.dirname(keyPath), { recursive: true });
      fs.writeFileSync(keyPath, 'previous-node-private-key');

      const rmSync = this.sinon.stub(fs, 'rmSync');
      rmSync.onFirstCall().throws(new Error('directory is busy'));
      rmSync.callThrough();

      await expect(new ConfigRemoveCommand().runWithDependencies(
        { config: 'node1' },
        flags,
        loadedConfigFile,
        { has: () => false },
        homeDir,
        configFileRepository,
      )).to.be.rejectedWith('directory is busy');

      expect(reread().isConfigExists('node1')).to.be.false();
      expect(fs.existsSync(homeDir.joinPath('node1'))).to.be.true();

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
  });
});
