import HomeDir from '../../../../src/config/HomeDir.js';
import ConfigSetCommand from '../../../../src/commands/config/set.js';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';

describe('Config set command', () => {
  const flags = {};

  let config;

  beforeEach(async () => {
    const getBaseConfig = getBaseConfigFactory(HomeDir.createTemp());

    config = getBaseConfig();
  });

  describe('#platform', () => {
    it('should allow setting strings', async () => {
      const command = new ConfigSetCommand();

      await command.runWithDependencies({
        option: 'core.docker.image', value: 'fake_image',
      }, flags, config);
    });

    it('should allow setting null', async () => {
      const command = new ConfigSetCommand();

      await command.runWithDependencies({
        option: 'description', value: null,
      }, flags, config);

      expect(config.get('description')).to.equal(null);

      await command.runWithDependencies({
        option: 'description', value: 'null',
      }, flags, config);

      expect(config.get('description')).to.equal(null);
    });

    it('should allow setting numbers', async () => {
      const command = new ConfigSetCommand();

      await command.runWithDependencies({
        option: 'platform.drive.abci.validatorSet.quorum.llmqType',
        value: 107,
      }, flags, config);

      expect(config.get('platform.drive.abci.validatorSet.quorum.llmqType')).to.equal(107);

      await command.runWithDependencies({
        option: 'platform.drive.abci.validatorSet.quorum.llmqType',
        value: '107',
      }, flags, config);

      expect(config.get('platform.drive.abci.validatorSet.quorum.llmqType')).to.equal(107);
    });

    it('should allow setting booleans', async () => {
      const command = new ConfigSetCommand();

      await command.runWithDependencies({
        option: 'dashmate.helper.api.enable', value: 'true',
      }, flags, config);

      expect(config.get('dashmate.helper.api.enable')).to.equal(true);

      await command.runWithDependencies({
        option: 'dashmate.helper.api.enable', value: true,
      }, flags, config);

      expect(config.get('dashmate.helper.api.enable')).to.equal(true);
    });

    it('should allow setting array of values', async () => {
      const command = new ConfigSetCommand();

      await command.runWithDependencies({
        option: 'core.rpc.allowIps', value: '["1337", "36484"]',
      }, flags, config);

      expect(config.get('core.rpc.allowIps')).to.deep.equal(['1337', '36484']);
    });

    it('should allow replacing part of the json', async () => {
      const command = new ConfigSetCommand();

      await command.runWithDependencies({
        option: 'docker.network',
        value: '{"subnet":"127.0.0.1/24"}',
      }, flags, config);
    });

    it('should throw on unknown path', async () => {
      const command = new ConfigSetCommand();

      // invalid path
      try {
        await command.runWithDependencies({
          option: 'fakePath', value: 'fake',
        }, flags, config);

        expect.fail('should throw error');
      } catch (e) {
        expect(e.name).to.equal('InvalidOptionPathError');
      }
    });

    it('should throw if invalid json is passed', async () => {
      const command = new ConfigSetCommand();

      // invalid json
      try {
        await command.runWithDependencies({
          option: 'core.rpc.allowIps', value: 'fake_image',
        }, flags, config);

        expect.fail('should throw error');
      } catch (e) {
        expect(e.name).to.equal('InvalidOptionError');
      }
    });

    it('should throw on type mismatch', async () => {
      const command = new ConfigSetCommand();

      // invalid json
      try {
        await command.runWithDependencies({
          option: 'dashmate.helper.api.enable', value: 120,
        }, flags, config);

        expect.fail('should throw error');
      } catch (e) {
        expect(e.name).to.equal('InvalidOptionError');
      }
    });
  });
});
