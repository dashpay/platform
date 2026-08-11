import fs from 'fs';
import path from 'path';
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
    // Pointed at a config that is NOT already the default, so deleting the save
    // would fail this rather than passing on the starting state.
    it('should save the new default without touching the config file loaded at startup', async () => {
      await new ConfigCreateCommand().runWithDependencies(
        { config: 'node1', from: 'base' },
        flags,
        loadedConfigFile,
        configFileRepository,
        noTemplates,
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

    it('should leave the service directory alone when the removal cannot be saved', async () => {
      const failingRepository = {
        update: () => {
          throw new Error('write failed');
        },
      };
      let thrownError;

      try {
        await new ConfigRemoveCommand().runWithDependencies(
          { config: 'node1' },
          flags,
          loadedConfigFile,
          { has: () => false },
          homeDir,
          failingRepository,
        );
      } catch (e) {
        thrownError = e;
      }

      expect(thrownError).to.be.an('error');
      expect(thrownError.message).to.equal('write failed');
      expect(fs.existsSync(homeDir.joinPath('node1'))).to.be.true();
      expect(reread().isConfigExists('node1')).to.be.true();
    });

    // The removal is durable before the files are gone, so a delete that fails
    // cannot be retried - `config remove` would report the config is not there.
    // What must not survive is a directory under a name that is now free to
    // re-create, because the next config of that name would inherit the
    // previous node's TLS private key.
    it('should not leave the removed service directory under a re-creatable name', async () => {
      const keyPath = homeDir.joinPath('node1', 'platform', 'gateway', 'ssl', 'private.key');

      fs.mkdirSync(path.dirname(keyPath), { recursive: true });
      fs.writeFileSync(keyPath, 'previous-node-private-key');

      const repositoryFailingToDelete = new ConfigFileJsonRepository(
        (data) => data,
        homeDir,
        () => null,
      );
      const { rmSync } = fs;

      // Stand in for a delete that cannot complete - a permission denied, or a
      // file the platform will not unlink.
      fs.rmSync = (target, options) => {
        if (String(target).includes('node1')) {
          throw new Error('directory is busy');
        }

        return rmSync(target, options);
      };

      try {
        await expect(new ConfigRemoveCommand().runWithDependencies(
          { config: 'node1' },
          flags,
          loadedConfigFile,
          { has: () => false },
          homeDir,
          repositoryFailingToDelete,
        )).to.be.rejectedWith('directory is busy');
      } finally {
        fs.rmSync = rmSync;
      }

      expect(reread().isConfigExists('node1')).to.be.false();
      expect(
        fs.existsSync(homeDir.joinPath('node1')),
        'nothing may remain under the name that is now free to re-create',
      ).to.be.false();
    });
  });
});
