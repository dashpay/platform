import fs from 'fs';
import path from 'path';
import Config from '../../../../src/config/Config.js';
import HomeDir from '../../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import getConfigFileMigrationsFactory from '../../../../configs/getConfigFileMigrationsFactory.js';
import migrateConfigFileFactory from '../../../../src/config/configFile/migrateConfigFileFactory.js';
import renderTemplateFactory from '../../../../src/templates/renderTemplateFactory.js';
import renderServiceTemplatesFactory from '../../../../src/templates/renderServiceTemplatesFactory.js';
import { PACKAGE_ROOT_DIR } from '../../../../src/constants.js';

describe('Core image migration', () => {
  const { version } = JSON.parse(fs.readFileSync(path.join(PACKAGE_ROOT_DIR, 'package.json'), 'utf8'));
  const base = getBaseConfigFactory()();
  const defaults = new Map(['base', 'mainnet', 'testnet', 'local'].map((name) => [name, base]));
  const getMigrations = getConfigFileMigrationsFactory(null, defaults);
  const migrateConfigFile = migrateConfigFileFactory(getMigrations);

  /**
   * A config file 5.0.3 wrote, every config holding the given Core image
   *
   * @param {Object<string, {network: string, group: string|null}>} configs
   * @param {string} image
   * @return {Object}
   */
  function getConfigFileData(configs, image) {
    return {
      configFormatVersion: '5.0.3',
      configs: Object.fromEntries(Object.entries(configs).map(([name, { network, group }]) => {
        const options = base.getStoredOptions();
        options.network = network;
        options.group = group;
        options.core.docker.image = image;

        return [name, options];
      })),
    };
  }

  const publicConfigs = {
    base: { network: 'mainnet', group: null },
    mainnet: { network: 'mainnet', group: null },
    testnet: { network: 'testnet', group: null },
    evonode: { network: 'mainnet', group: null },
    testnetEvonode: { network: 'testnet', group: null },
    local: { network: 'local', group: null },
  };

  const devConfigs = {
    local_seed: { network: 'local', group: 'local' },
    local_1: { network: 'local', group: 'local' },
    myLocal: { network: 'local', group: null },
    devnetNode: { network: 'devnet', group: null },
  };

  it('should run Core v24 on new configurations', () => {
    expect(base.get('core.docker.image')).to.equal('dashpay/dashd:24');
  });

  it('should move mainnet, testnet and the local preset from the stock Core 23 image to 24', () => {
    const migrated = migrateConfigFile(
      getConfigFileData(publicConfigs, 'dashpay/dashd:23'),
      '5.0.3',
      version,
    );

    for (const name of Object.keys(publicConfigs)) {
      expect(migrated.configs[name].core.docker.image).to.equal(
        'dashpay/dashd:24',
        `${name} was not moved to Core v24`,
      );
      expect(() => new Config(name, migrated.configs[name]), `${name} does not load`).to.not.throw();
    }
  });

  it('should leave local network and devnet nodes on Core 23', () => {
    // Core v24 drops the sporks a v23 node stored, so these networks move as a whole.
    const migrated = migrateConfigFile(
      getConfigFileData(devConfigs, 'dashpay/dashd:23'),
      '5.0.3',
      version,
    );

    for (const name of Object.keys(devConfigs)) {
      expect(migrated.configs[name].core.docker.image).to.equal(
        'dashpay/dashd:23',
        `${name} was moved off Core 23`,
      );
    }
  });

  it('should keep a Core image the operator chose', () => {
    const operatorImages = [
      'dashpay/dashd:24.0.0',
      'dashpay/dashd:24.0.0-rc.7',
      'dashpay/dashd:23.1.10',
      'dashpay/dashd:23.1',
      'dashpay/dashd:22',
      'dashpay/dashd:latest',
      'dashpay/dashd:develop',
      'dashpay/dashd:23@sha256:37071aec15848ef7b5d53e3531197a9e2fad45752ff23ce8f3f447c07b3590c5',
      'docker.io/dashpay/dashd:23',
      'registry.example.com/dashd:23',
      'dashd:local',
    ];

    for (const image of operatorImages) {
      const migrated = migrateConfigFile(getConfigFileData(publicConfigs, image), '5.0.3', version);

      for (const name of Object.keys(publicConfigs)) {
        expect(migrated.configs[name].core.docker.image).to.equal(
          image,
          `operator image ${image} was overwritten for ${name}`,
        );
      }
    }
  });

  it('should move a mainnet node set up before 3.0.1 to Core 24', () => {
    // 3.0.1 stored the floating 23 tag for every config, so older nodes pass through it.
    const configFileData = getConfigFileData({ mainnet: publicConfigs.mainnet }, 'dashpay/dashd:20.1.1');
    configFileData.configFormatVersion = '3.0.0';

    const migrated = migrateConfigFile(configFileData, '3.0.0', version);

    expect(migrated.configs.mainnet.core.docker.image).to.equal('dashpay/dashd:24');
  });

  it('should change nothing when applied again', () => {
    const migrate = getMigrations()['5.0.4'];
    const migrated = migrate(getConfigFileData({ ...publicConfigs, ...devConfigs }, 'dashpay/dashd:23'));
    const snapshot = JSON.parse(JSON.stringify(migrated));

    expect(migrate(migrated)).to.deep.equal(snapshot);
  });

  it('should leave a config without Core docker options alone', () => {
    const configFileData = {
      configFormatVersion: '5.0.3',
      configs: { partial: { network: 'mainnet', core: {} } },
    };

    const migrated = migrateConfigFile(configFileData, '5.0.3', version);

    expect(migrated.configs.partial).to.deep.equal({ network: 'mainnet', core: {} });
  });

  describe('generated Core configuration', () => {
    let config;
    let renderServiceTemplates;

    beforeEach(() => {
      config = getBaseConfigFactory(HomeDir.createTemp())();
      config.set('core.masternode.enable', true);
      config.set('core.masternode.operator.privateKey', '1'.repeat(64));
      renderServiceTemplates = renderServiceTemplatesFactory(renderTemplateFactory());
    });

    ['mainnet', 'testnet'].forEach((network) => {
      it(`should satisfy what a Core v24 masternode on ${network} requires to start`, () => {
        config.set('network', network);

        const dashConf = renderServiceTemplates(config)['core/dash.conf'];

        expect(dashConf).to.match(/^masternodeblsprivkey=1+$/m);
        // Core v24 refuses to start a masternode with -blocksonly or without -peerblockfilters.
        expect(dashConf).to.match(/^blockfilterindex=basic$/m);
        expect(dashConf).to.match(/^peerblockfilters=1$/m);
        expect(dashConf).to.not.match(/^blocksonly=1$/m);
        // Drive reads masternodes' `service` from protx listdiff.
        expect(dashConf).to.match(/^deprecatedrpc=service$/m);

        const [, whitelist] = dashConf.match(/^rpcwhitelist=drive_consensus:(.*)$/m);
        expect(whitelist.split(',')).to.include.members(['getspecialtxes', 'getdeploymentinfo']);
      });
    });
  });
});
