import getBaseConfigFactory from '../../../../../../configs/defaults/getBaseConfigFactory.js';
import Config from '../../../../../../src/config/Config.js';
import ConfigFile from '../../../../../../src/config/configFile/ConfigFile.js';
import getLocalGroupConfigs from '../../../../../../src/listr/tasks/setup/local/getLocalGroupConfigs.js';

describe('getLocalGroupConfigs', () => {
  function createConfigFile(nodeImages) {
    const preset = getBaseConfigFactory()();
    const configs = [new Config('local', { ...preset.getStoredOptions(), network: 'local' })];

    for (const [name, image] of Object.entries(nodeImages)) {
      const config = new Config(name, preset.getStoredOptions());
      config.set('core.docker.image', image);
      configs.push(config);
    }

    return new ConfigFile(configs, '5.0.4', 'abcdef12', null, null);
  }

  it('should create every node from the local preset for a new network', () => {
    const configFile = createConfigFile({});

    const configs = getLocalGroupConfigs(configFile, ['local_1', 'local_2', 'local_seed']);

    expect(configs.map((config) => config.getName())).to.deep.equal(['local_1', 'local_2', 'local_seed']);
    for (const config of configs) {
      expect(config.get('core.docker.image')).to.equal('dashpay/dashd:24');
    }
  });

  it('should run an added node on the Core image of the existing network', () => {
    const configFile = createConfigFile({
      local_1: 'dashpay/dashd:23',
      local_seed: 'dashpay/dashd:23',
    });

    const configs = getLocalGroupConfigs(configFile, ['local_1', 'local_2', 'local_seed']);

    expect(configs.map((config) => config.get('core.docker.image'))).to.deep.equal([
      'dashpay/dashd:23',
      'dashpay/dashd:23',
      'dashpay/dashd:23',
    ]);
    expect(configs[0]).to.equal(configFile.getConfig('local_1'));
    expect(configFile.getConfig('local_2').get('core.docker.image')).to.equal('dashpay/dashd:23');
  });
});
