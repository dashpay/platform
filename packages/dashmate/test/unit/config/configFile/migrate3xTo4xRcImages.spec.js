import path from 'path';
import DefaultConfigs from '../../../../src/config/DefaultConfigs.js';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import getLocalConfigFactory from '../../../../configs/defaults/getLocalConfigFactory.js';
import getTestnetConfigFactory from '../../../../configs/defaults/getTestnetConfigFactory.js';
import getMainnetConfigFactory from '../../../../configs/defaults/getMainnetConfigFactory.js';
import getConfigFileMigrationsFactory from '../../../../configs/getConfigFileMigrationsFactory.js';
import migrateConfigFileFactory from '../../../../src/config/configFile/migrateConfigFileFactory.js';

// Regression coverage for dashpay/platform#3889.
//
// A node already on 3.0.1 skips the 3.0.0 migration (semver.gt filter)
// that used to re-sync Drive ABCI and rs-dapi images. The intervening
// 3.0.2 / 3.1.0 migrations only touched Gateway / Tenderdash, so a
// `dashmate update 3.0.x → 4.0.0-rc.x` kept the old protocol-11
// images and the node crash-looped after protocol 12 activation.
//
// The new `4.0.0-rc.2` migration must re-sync those two image fields
// from the current default config.
describe('migration 4.0.0-rc.2: re-sync Drive ABCI & rs-dapi images (#3889)', () => {
  let migrate;
  let defaults;

  beforeEach(() => {
    // Construct the migration directly (no DI). The full DI container
    // transitively imports `@dashevo/dapi-client`, which can fail to
    // load when `@dashevo/wasm-dpp` hasn't been built; the migration
    // itself only needs the default-config factories.
    const homeDirStub = {
      joinPath: (...segments) => path.join('/tmp/dashmate-spec', ...segments),
    };
    const getBaseConfig = getBaseConfigFactory();
    defaults = new DefaultConfigs([
      getBaseConfig,
      getLocalConfigFactory(getBaseConfig),
      getTestnetConfigFactory(homeDirStub, getBaseConfig),
      getMainnetConfigFactory(homeDirStub, getBaseConfig),
    ]);
    const getMigrations = getConfigFileMigrationsFactory(homeDirStub, defaults);
    migrate = migrateConfigFileFactory(getMigrations);
  });

  // Minimal post-3.0.x config shape carrying the stale protocol-11
  // image tags that dashmate 3.0.x shipped. Earlier migrations expect
  // a much richer object, so we set fromVersion >= 3.1.0 to make the
  // semver.gt filter run only the new step.
  function buildStaleConfig({ network, group, customImages = {} }) {
    return {
      group,
      network,
      platform: {
        enable: true,
        drive: {
          abci: {
            docker: {
              image: customImages.drive ?? 'dashpay/drive:3',
            },
          },
        },
        dapi: {
          rsDapi: {
            docker: {
              image: customImages.rsDapi ?? 'dashpay/rs-dapi:3',
            },
          },
        },
      },
    };
  }

  it('re-syncs drive.abci and dapi.rsDapi images to current defaults', () => {
    const rawConfigFile = {
      configFormatVersion: '3.1.0',
      defaultConfigName: null,
      defaultGroupName: null,
      configs: {
        testnet: buildStaleConfig({ network: 'testnet', group: 'testnet' }),
        mainnet: buildStaleConfig({ network: 'mainnet', group: 'mainnet' }),
      },
    };

    const expectedDriveImage = defaults.get('base').get('platform.drive.abci.docker.image');
    const expectedRsDapiImage = defaults.get('base').get('platform.dapi.rsDapi.docker.image');

    const migrated = migrate(rawConfigFile, '3.1.0', '4.0.0-rc.2');

    expect(migrated.configs.testnet.platform.drive.abci.docker.image)
      .to.equal(expectedDriveImage);
    expect(migrated.configs.testnet.platform.dapi.rsDapi.docker.image)
      .to.equal(expectedRsDapiImage);
    expect(migrated.configs.mainnet.platform.drive.abci.docker.image)
      .to.equal(expectedDriveImage);
    expect(migrated.configs.mainnet.platform.dapi.rsDapi.docker.image)
      .to.equal(expectedRsDapiImage);
    expect(migrated.configFormatVersion).to.equal('4.0.0-rc.2');
  });

  it('runs for 3.0.1 fromVersion (the version skipped by the 3.0.0 migration)', () => {
    // Direct repro of #3889: nodes that were already on 3.0.1 must
    // still get the image re-sync, even though 3.0.0 itself is gated
    // out by `semver.gt('3.0.0', '3.0.1') === false`.
    const rawConfigFile = {
      configFormatVersion: '3.0.1',
      defaultConfigName: null,
      defaultGroupName: null,
      configs: {
        testnet: buildStaleConfig({ network: 'testnet', group: 'testnet' }),
      },
    };

    const expectedDriveImage = defaults.get('base').get('platform.drive.abci.docker.image');
    const expectedRsDapiImage = defaults.get('base').get('platform.dapi.rsDapi.docker.image');

    const migrated = migrate(rawConfigFile, '3.0.1', '4.0.0-rc.2');

    expect(migrated.configs.testnet.platform.drive.abci.docker.image)
      .to.equal(expectedDriveImage);
    expect(migrated.configs.testnet.platform.dapi.rsDapi.docker.image)
      .to.equal(expectedRsDapiImage);
  });

  it('no-ops on configs without platform.dapi.rsDapi', () => {
    const rawConfigFile = {
      configFormatVersion: '3.1.0',
      defaultConfigName: null,
      defaultGroupName: null,
      configs: {
        mainnet: {
          group: 'mainnet',
          network: 'mainnet',
          platform: {
            enable: true,
            drive: {
              abci: {
                docker: { image: 'dashpay/drive:3' },
              },
            },
            dapi: {},
          },
        },
      },
    };

    const expectedDriveImage = defaults.get('base').get('platform.drive.abci.docker.image');

    const migrated = migrate(rawConfigFile, '3.1.0', '4.0.0-rc.2');

    expect(migrated.configs.mainnet.platform.drive.abci.docker.image)
      .to.equal(expectedDriveImage);
    expect(migrated.configs.mainnet.platform.dapi.rsDapi).to.equal(undefined);
  });
});
