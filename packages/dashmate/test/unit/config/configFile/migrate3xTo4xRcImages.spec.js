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
// 3.0.1 / 3.0.2 / 3.1.0 migrations only touched Core / Gateway / Tenderdash
// `.docker.image` fields (3.1.0 does add `buildArgs` on the drive/rsDapi
// `docker.build` blocks, but never their image tags). The result: a
// `dashmate update 3.0.x → 4.0.0-rc.x` kept the protocol-11 images and
// the node crash-looped after protocol 12 activation.
//
// The new `4.0.0-rc.2` migration must re-sync those two image fields
// from the current default config.
describe('migration 4.0.0-rc.2: re-sync Drive ABCI & rs-dapi images (#3889)', () => {
  const STALE_DRIVE = 'dashpay/drive:3';
  const STALE_RS_DAPI = 'dashpay/rs-dapi:3';

  let migrate;
  let defaults;
  let expectedDriveImage;
  let expectedRsDapiImage;

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

    expectedDriveImage = defaults.get('base').get('platform.drive.abci.docker.image');
    expectedRsDapiImage = defaults.get('base').get('platform.dapi.rsDapi.docker.image');
  });

  // Minimal post-3.0.x config shape carrying the stale protocol-11
  // image tags that dashmate 3.0.x shipped. Earlier migrations expect
  // a much richer object, so we set fromVersion >= 3.0.1 to make the
  // semver.gt filter run only the new step.
  function buildStaleConfig({ network, group }) {
    return {
      group,
      network,
      platform: {
        enable: true,
        drive: {
          abci: {
            docker: { image: STALE_DRIVE },
          },
          tenderdash: {
            docker: { image: 'dashpay/tenderdash:1.5' },
          },
        },
        dapi: {
          rsDapi: {
            docker: { image: STALE_RS_DAPI },
          },
        },
      },
    };
  }

  // Both fromVersion variants are real-world entry points: a node could
  // be on 3.0.1 (the version that originally skipped the 3.0.0 image
  // re-sync — the direct #3889 repro) or on 3.1.0 (skipping it for the
  // same reason). Either way the new migration must converge them.
  ['3.0.1', '3.1.0'].forEach((fromVersion) => {
    it(`re-syncs drive.abci and dapi.rsDapi images on ${fromVersion} → 4.0.0-rc.2`, () => {
      const rawConfigFile = {
        configFormatVersion: fromVersion,
        defaultConfigName: null,
        defaultGroupName: null,
        configs: {
          testnet: buildStaleConfig({ network: 'testnet', group: 'testnet' }),
          mainnet: buildStaleConfig({ network: 'mainnet', group: 'mainnet' }),
        },
      };

      const migrated = migrate(rawConfigFile, fromVersion, '4.0.0-rc.2');

      ['testnet', 'mainnet'].forEach((name) => {
        const { docker: driveDocker } = migrated.configs[name].platform.drive.abci;
        const { docker: rsDapiDocker } = migrated.configs[name].platform.dapi.rsDapi;

        expect(driveDocker.image).to.equal(expectedDriveImage);
        expect(rsDapiDocker.image).to.equal(expectedRsDapiImage);

        // Pin against the specific regression so the test stays
        // meaningful even if a future default happens to match
        // the stale value again.
        expect(driveDocker.image).to.not.equal(STALE_DRIVE);
        expect(rsDapiDocker.image).to.not.equal(STALE_RS_DAPI);
      });

      expect(migrated.configFormatVersion).to.equal('4.0.0-rc.2');
    });
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
                docker: { image: STALE_DRIVE },
              },
              tenderdash: {
                docker: { image: 'dashpay/tenderdash:1.5' },
              },
            },
            dapi: {},
          },
        },
      },
    };

    const migrated = migrate(rawConfigFile, '3.1.0', '4.0.0-rc.2');

    expect(migrated.configs.mainnet.platform.drive.abci.docker.image)
      .to.equal(expectedDriveImage);
    expect(migrated.configs.mainnet.platform.dapi.rsDapi).to.equal(undefined);
  });
});
