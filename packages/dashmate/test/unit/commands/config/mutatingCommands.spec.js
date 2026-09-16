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
      const writeConfigTemplates = (config) => {
        expect(reread().isConfigExists(config.getName())).to.be.true();
      };

      await new ConfigCreateCommand().runWithDependencies(
        { config: 'node1', from: 'base' },
        flags,
        loadedConfigFile,
        configFileRepository,
        writeConfigTemplates,
        homeDir,
      );

      expect(reread().isConfigExists('node1')).to.be.true();
      expect(loadedConfigFile.isConfigExists('node1')).to.be.false();
      expect(loadedConfigFile.isChanged()).to.be.false();
    });

    it('should reject absent reserved names without deleting the config file', async () => {
      const before = fs.readFileSync(homeDir.joinPath('config.json'), 'utf8');

      await Promise.all(['config.json', 'CONFIG.JSON', 'config.json.'].map(async (name) => {
        await expect(new ConfigRemoveCommand().runWithDependencies(
          { config: name },
          flags,
          loadedConfigFile,
          { has: () => false },
          homeDir,
          configFileRepository,
        )).to.be.rejectedWith(`Config with name '${name}' is not present`);
      }));

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
      )).to.be.rejectedWith(`Inspect '${homeDir.joinPath('node1')}' and move or delete it manually`);

      expect(reread().isConfigExists('node1')).to.be.false();
      expect(fs.readFileSync(keyPath, 'utf8')).to.equal('interrupted-create-private-key');

      fs.rmSync(homeDir.joinPath('node1'), { recursive: true });

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

    it('should reject a portable-name alias after a saved create fails to render', async () => {
      await expect(new ConfigCreateCommand().runWithDependencies(
        { config: 'node1', from: 'base' },
        flags,
        loadedConfigFile,
        configFileRepository,
        () => {
          throw new Error('template write failed');
        },
        homeDir,
      )).to.be.rejectedWith('template write failed');

      expect(reread().isConfigExists('node1')).to.be.true();
      expect(fs.existsSync(homeDir.joinPath('node1'))).to.be.false();

      await expect(new ConfigCreateCommand().runWithDependencies(
        { config: 'NODE1', from: 'base' },
        flags,
        loadedConfigFile,
        configFileRepository,
        noTemplates,
        homeDir,
      )).to.be.rejectedWith("Config with name 'NODE1' already present");

      expect(reread().isConfigExists('NODE1')).to.be.false();
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

    it('should preserve an orphan after removal until the operator clears it', async function it() {
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

      await expect(new ConfigRemoveCommand().runWithDependencies(
        { config: 'node1' },
        flags,
        loadedConfigFile,
        { has: () => false },
        homeDir,
        configFileRepository,
      )).to.be.rejectedWith("Config with name 'node1' is not present");

      expect(reread().isConfigExists('node1')).to.be.false();
      expect(fs.readFileSync(keyPath, 'utf8')).to.equal('previous-node-private-key');
    });

    it('should not remove a differently-cased listed config or its private files', async () => {
      const keyPath = homeDir.joinPath('node1', 'platform', 'gateway', 'ssl', 'private.key');

      fs.mkdirSync(path.dirname(keyPath), { recursive: true });
      fs.writeFileSync(keyPath, 'listed-node-private-key');

      await expect(new ConfigRemoveCommand().runWithDependencies(
        { config: 'NODE1' },
        flags,
        loadedConfigFile,
        { has: () => false },
        homeDir,
        configFileRepository,
      )).to.be.rejectedWith("Config with name 'NODE1' is not present");

      expect(reread().isConfigExists('node1')).to.be.true();
      expect(fs.readFileSync(keyPath, 'utf8')).to.equal('listed-node-private-key');
    });

    it('should preserve service files used by a surviving portable-name alias', async () => {
      const configFilePath = homeDir.joinPath('config.json');
      const data = JSON.parse(fs.readFileSync(configFilePath, 'utf8'));
      const keyPath = homeDir.joinPath('node1', 'platform', 'gateway', 'ssl', 'private.key');

      data.configs.NODE1 = data.configs.node1;
      fs.writeFileSync(configFilePath, `${JSON.stringify(data, undefined, 2)}\n`, 'utf8');
      fs.mkdirSync(path.dirname(keyPath), { recursive: true });
      fs.writeFileSync(keyPath, 'shared-node-private-key');

      await new ConfigRemoveCommand().runWithDependencies(
        { config: 'node1' },
        flags,
        loadedConfigFile,
        { has: () => false },
        homeDir,
        configFileRepository,
      );

      expect(reread().isConfigExists('node1')).to.be.false();
      expect(reread().isConfigExists('NODE1')).to.be.true();
      expect(fs.readFileSync(keyPath, 'utf8')).to.equal('shared-node-private-key');
    });

    it('should not delete config.json when it appears immediately before the locked read', async function it() {
      const configFilePath = homeDir.joinPath('config.json');
      const configFileJson = fs.readFileSync(configFilePath, 'utf8');
      const update = configFileRepository.update.bind(configFileRepository);

      fs.rmSync(configFilePath);
      this.sinon.stub(configFileRepository, 'update').callsFake((...args) => {
        fs.writeFileSync(configFilePath, configFileJson, 'utf8');

        return update(...args);
      });

      await expect(new ConfigRemoveCommand().runWithDependencies(
        { config: 'config.json' },
        flags,
        loadedConfigFile,
        { has: () => false },
        homeDir,
        configFileRepository,
      )).to.be.rejectedWith("Config with name 'config.json' is not present");

      expect(fs.existsSync(configFilePath)).to.be.true();
      expect(reread().isConfigExists('node1')).to.be.true();
    });

    it('should remove a listed legacy reserved name without deleting repository files', async () => {
      const configFilePath = homeDir.joinPath('config.json');
      const data = JSON.parse(fs.readFileSync(configFilePath, 'utf8'));

      data.configs['config.json'] = data.configs.node1;
      fs.writeFileSync(configFilePath, `${JSON.stringify(data, undefined, 2)}\n`, 'utf8');

      await new ConfigRemoveCommand().runWithDependencies(
        { config: 'config.json' },
        flags,
        loadedConfigFile,
        { has: () => false },
        homeDir,
        configFileRepository,
      );

      expect(fs.existsSync(configFilePath)).to.be.true();
      expect(reread().isConfigExists('config.json')).to.be.false();
      expect(reread().isConfigExists('node1')).to.be.true();
    });
  });
});
