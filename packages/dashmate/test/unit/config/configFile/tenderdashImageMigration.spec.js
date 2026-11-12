import fs from 'fs';
import path from 'path';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import getLocalConfigFactory from '../../../../configs/defaults/getLocalConfigFactory.js';
import getConfigFileMigrationsFactory from '../../../../configs/getConfigFileMigrationsFactory.js';
import InvalidOptionError from '../../../../src/config/errors/InvalidOptionError.js';
import getConfigFormatVersion from '../../../../src/config/configFile/getConfigFormatVersion.js';
import migrateConfigFileFactory from '../../../../src/config/configFile/migrateConfigFileFactory.js';
import { PACKAGE_ROOT_DIR } from '../../../../src/constants.js';

describe('Tenderdash image migration', () => {
  const { version } = JSON.parse(fs.readFileSync(path.join(PACKAGE_ROOT_DIR, 'package.json'), 'utf8'));
  const getBaseConfig = getBaseConfigFactory();
  const base = getBaseConfig();
  const local = getLocalConfigFactory(getBaseConfig)();
  const defaults = new Map([
    ...['base', 'mainnet', 'testnet'].map((name) => [name, base]),
    ['local', local],
  ]);
  const getMigrations = getConfigFileMigrationsFactory(null, defaults);
  const migrateConfigFile = migrateConfigFileFactory(getMigrations);

  it('should use the Tenderdash 2.0.0-dev.2 image for new configurations', () => {
    expect(base.get('platform.drive.tenderdash.docker.image')).to.equal('dashpay/tenderdash:2.0.0-dev.2');
  });

  for (const fromVersion of ['4.2.0-dev.1', '4.2.0-beta.3', '5.0.0-rc.3', '5.0.3']) {
    it(`should migrate pinned images from config format ${fromVersion}`, () => {
      const configFileData = {
        configFormatVersion: fromVersion,
        configs: {
          mainnet: { platform: { drive: { tenderdash: { docker: { image: 'dashpay/tenderdash:1.8' } } } } },
          testnet: { platform: { drive: { tenderdash: { docker: { image: 'dashpay/tenderdash:1.8.2' } } } } },
          withoutPlatform: {},
          withoutDocker: { platform: { drive: { tenderdash: {} } } },
        },
      };

      const migrated = migrateConfigFile(configFileData, fromVersion, version);

      for (const name of ['mainnet', 'testnet']) {
        expect(migrated.configs[name].platform.drive.tenderdash.docker.image)
          .to.equal('dashpay/tenderdash:2.0.0-dev.2');
      }
      expect(migrated.configs.withoutPlatform).to.deep.equal({});
      expect(migrated.configs.withoutDocker).to.deep.equal({
        platform: { drive: { tenderdash: {} } },
      });
      // Stamped with the format this build produces: the newest migration while it is ahead
      // of the package version, the package version once a release has passed it.
      expect(migrated.configFormatVersion)
        .to.equal(getConfigFormatVersion(getMigrations(), version));
      expect(migrateConfigFile(migrated, migrated.configFormatVersion, version)).to.equal(migrated);
    });
  }

  describe('P2P settings Tenderdash 2.0 refuses to start with', () => {
    function migrateP2p(configs) {
      return migrateConfigFile({ configFormatVersion: '5.0.3', configs }, '5.0.3', version).configs;
    }

    function p2pConfig(network, p2p) {
      return { network, platform: { drive: { tenderdash: { docker: {}, p2p } } } };
    }

    it('should reject a zero maxPacketMsgPayloadSize', () => {
      const config = getBaseConfig();

      expect(() => config.set('platform.drive.tenderdash.p2p.maxPacketMsgPayloadSize', 0))
        .to.throw(InvalidOptionError);
    });

    it('should reset a zero maxPacketMsgPayloadSize to the network default', () => {
      const configs = migrateP2p({
        mainnet: p2pConfig('mainnet', { maxPacketMsgPayloadSize: 0 }),
        local: p2pConfig('local', { maxPacketMsgPayloadSize: 0 }),
        custom: p2pConfig('testnet', { maxPacketMsgPayloadSize: 4096 }),
      });

      expect(configs.mainnet.platform.drive.tenderdash.p2p.maxPacketMsgPayloadSize).to.equal(10240);
      expect(configs.local.platform.drive.tenderdash.p2p.maxPacketMsgPayloadSize).to.equal(1024);
      expect(configs.custom.platform.drive.tenderdash.p2p.maxPacketMsgPayloadSize).to.equal(4096);
    });

    it('should cap maxOutgoingConnections at maxConnections', () => {
      const configs = migrateP2p({
        above: p2pConfig('mainnet', { maxConnections: 20, maxOutgoingConnections: 30 }),
        equal: p2pConfig('mainnet', { maxConnections: 30, maxOutgoingConnections: 30 }),
        below: p2pConfig('mainnet', { maxConnections: 64, maxOutgoingConnections: 30 }),
      });

      expect(configs.above.platform.drive.tenderdash.p2p.maxOutgoingConnections).to.equal(20);
      expect(configs.equal.platform.drive.tenderdash.p2p.maxOutgoingConnections).to.equal(30);
      expect(configs.below.platform.drive.tenderdash.p2p.maxOutgoingConnections).to.equal(30);
    });
  });
});
