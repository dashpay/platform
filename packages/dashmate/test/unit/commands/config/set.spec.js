import fs from 'fs';
import HomeDir from '../../../../src/config/HomeDir.js';
import ConfigSetCommand from '../../../../src/commands/config/set.js';
import ConfigFile from '../../../../src/config/configFile/ConfigFile.js';
import ConfigFileJsonRepository from '../../../../src/config/configFile/ConfigFileJsonRepository.js';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';

describe('Config set command', () => {
  const flags = {};

  let homeDir;
  let config;
  let configFileRepository;
  let writeConfigTemplates;
  let command;

  /**
   * The command now reads, changes and saves in one locked step rather than
   * mutating the object it was handed, so these assert what actually reached
   * disk. Reading it back is the only thing that proves the value was saved.
   *
   * @param {string} path
   * @return {*}
   */
  function persisted(path) {
    return new ConfigFileJsonRepository((data) => data, homeDir, () => null)
      .read()
      .getConfig('base')
      .get(path);
  }

  beforeEach(() => {
    homeDir = HomeDir.createTemp();

    config = getBaseConfigFactory(homeDir)();

    const configFile = new ConfigFile([config], '4.1.0', 'abcdef12', 'base', null);

    fs.writeFileSync(
      homeDir.joinPath('config.json'),
      `${JSON.stringify(configFile.toObject(), undefined, 2)}\n`,
      'utf8',
    );

    configFileRepository = new ConfigFileJsonRepository((data) => data, homeDir, () => null);
    writeConfigTemplates = () => {};
    command = new ConfigSetCommand();
  });

  afterEach(() => {
    homeDir.remove();
  });

  /**
   * @param {string} option
   * @param {*} value
   */
  function run(option, value) {
    return command.runWithDependencies(
      { option, value },
      flags,
      config,
      configFileRepository,
      writeConfigTemplates,
    );
  }

  describe('#platform', () => {
    it('should allow setting strings', async () => {
      writeConfigTemplates = (renderedConfig) => {
        expect(persisted('core.docker.image'))
          .to.equal(renderedConfig.get('core.docker.image'));
      };

      await run('core.docker.image', 'fake_image');

      expect(persisted('core.docker.image')).to.equal('fake_image');
    });

    it('should allow setting null', async () => {
      await run('description', null);

      expect(persisted('description')).to.equal(null);

      await run('description', 'null');

      expect(persisted('description')).to.equal(null);
    });

    it('should allow setting numbers', async () => {
      await run('platform.drive.abci.validatorSet.quorum.llmqType', 107);

      expect(persisted('platform.drive.abci.validatorSet.quorum.llmqType')).to.equal(107);

      await run('platform.drive.abci.validatorSet.quorum.llmqType', '107');

      expect(persisted('platform.drive.abci.validatorSet.quorum.llmqType')).to.equal(107);
    });

    it('should allow setting booleans', async () => {
      await run('dashmate.helper.api.enable', 'true');

      expect(persisted('dashmate.helper.api.enable')).to.equal(true);

      await run('dashmate.helper.api.enable', true);

      expect(persisted('dashmate.helper.api.enable')).to.equal(true);
    });

    it('should allow setting array of values', async () => {
      await run('core.rpc.allowIps', '["1337", "36484"]');

      expect(persisted('core.rpc.allowIps')).to.deep.equal(['1337', '36484']);
    });

    it('should allow replacing part of the json', async () => {
      await run('docker.network', '{"subnet":"127.0.0.1/24"}');

      expect(persisted('docker.network.subnet')).to.equal('127.0.0.1/24');
    });

    it('should throw on unknown path', async () => {
      try {
        await run('fakePath', 'fake');

        expect.fail('should throw error');
      } catch (e) {
        expect(e.name).to.equal('InvalidOptionPathError');
      }
    });

    it('should throw if invalid json is passed', async () => {
      try {
        await run('core.rpc.allowIps', 'fake_image');

        expect.fail('should throw error');
      } catch (e) {
        expect(e.name).to.equal('InvalidOptionError');
      }
    });

    it('should throw on type mismatch', async () => {
      try {
        await run('dashmate.helper.api.enable', 120);

        expect.fail('should throw error');
      } catch (e) {
        expect(e.name).to.equal('InvalidOptionError');
      }
    });

    // A rejected set must leave the file exactly as it was, not half-applied.
    it('should not change the file when the value is rejected', async () => {
      const before = fs.readFileSync(homeDir.joinPath('config.json'), 'utf8');

      await run('dashmate.helper.api.enable', 120).catch(() => {});

      expect(fs.readFileSync(homeDir.joinPath('config.json'), 'utf8')).to.equal(before);
    });
  });
});
