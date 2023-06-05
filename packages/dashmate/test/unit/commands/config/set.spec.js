const ConfigSetCommand = require('../../../../src/commands/config/set');
const Config = require("../../../../src/config/Config");
const baseConfig = require("../../../../configs/system/base");

describe('Set Command Status', () => {
  let config

  beforeEach(async function it() {
    config = new Config('config', baseConfig)
  });

  describe('#platform', () => {
    it('should just work', async () => {
      const command = new ConfigSetCommand();

      const flags = {}

      await command.runWithDependencies({option: 'core.docker.image', value: 'fake_image'}, flags, config);
    });

    it('should throw on unknown path', async () => {
      const command = new ConfigSetCommand();

      const flags = {}

      // invalid path
      try {
        await command.runWithDependencies({option: 'fakePath', value: 'fake'}, flags, config);

        expect.fail('should throw error');
      } catch (e) {
        expect(e.name).to.equal('InvalidOptionPathError');
      }
    });

    it('should allow setting array of values', async () => {
      const command = new ConfigSetCommand();

      const flags = {}

      // invalid json
      try {
        await command.runWithDependencies({option: 'core.rpc.allowIps', value: 'fake_image'}, flags, config);

        expect.fail('should throw error');
      } catch (e) {
        expect(e.name).to.equal('SyntaxError');
      }

      // type mismatch
      try {
        await command.runWithDependencies({option: 'core.rpc.allowIps', value: [1337, 36484]}, flags, config);

        expect.fail('should throw error');
      } catch (e) {
        console.error(e)
        expect(e.name).to.equal('SyntaxError');
      }

      await command.runWithDependencies({option: 'core.rpc.allowIps', value: '["1337", "36484"]'}, flags, config);
    });

    it.only('should allow replacing part of the json', async () => {
      const command = new ConfigSetCommand();

      const flags = {}

      await command.runWithDependencies({option: 'docker', value: '{"network":{"subnet":"127.0.0.1/24"}}'}, flags, config);
    });
  });
});
