import path from 'path';
import DefaultConfigs from '../../../../src/config/DefaultConfigs.js';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import getLocalConfigFactory from '../../../../configs/defaults/getLocalConfigFactory.js';
import getTestnetConfigFactory from '../../../../configs/defaults/getTestnetConfigFactory.js';
import getMainnetConfigFactory from '../../../../configs/defaults/getMainnetConfigFactory.js';
import getConfigFileMigrationsFactory from '../../../../configs/getConfigFileMigrationsFactory.js';
import migrateConfigFileFactory from '../../../../src/config/configFile/migrateConfigFileFactory.js';

// Regression coverage for dashpay/platform#3889 and its rework.
//
// A node already on 3.0.1 skips the 3.0.0 migration (semver.gt filter)
// that used to re-sync Drive ABCI and rs-dapi images. The intervening
// 3.0.1 / 3.0.2 / 3.1.0 migrations only touched Core / Gateway / Tenderdash
// `.docker.image` fields. The result: a `dashmate update 3.0.x → 4.0.0-rc.x`
// kept the protocol-11 images (`dashpay/drive:3`, `dashpay/rs-dapi:3`)
// and the node crash-looped after protocol 12 activation.
//
// The original fix keyed the migration at 4.0.0-rc.2 — the same version
// dashmate already shipped — so any node already stamped
// configFormatVersion: "4.0.0-rc.2" short-circuited in migrateConfigFile
// (fromVersion === toVersion) and never ran the migration. The rework
// re-keys it at 4.0.0-rc.3 (the next release) and adds a stale-shipped-
// image predicate to preserve customised images.
describe('migration 4.0.0-rc.3: re-sync Drive ABCI & rs-dapi images (#3889)', () => {
  const STALE_DRIVE = 'dashpay/drive:3';
  const STALE_DRIVE_DEV = 'dashpay/drive:3-dev';
  const STALE_RS_DAPI = 'dashpay/rs-dapi:3';
  const STALE_RS_DAPI_DEV = 'dashpay/rs-dapi:3-dev';
  const CUSTOM_DRIVE = 'private-registry.internal/drive:patched-v3';
  const CUSTOM_RS_DAPI = 'my-org/rs-dapi:fork-3';

  let migrate;
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
    const defaults = new DefaultConfigs([
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

  function buildConfig({
    network, group, driveImage, rsDapiImage,
  }) {
    return {
      group,
      network,
      platform: {
        enable: true,
        drive: {
          abci: {
            docker: { image: driveImage },
          },
          tenderdash: {
            docker: { image: 'dashpay/tenderdash:1.5' },
          },
        },
        dapi: {
          rsDapi: {
            docker: { image: rsDapiImage },
          },
        },
      },
    };
  }

  // 3.0.x and 3.1.0 are real-world entry points: a node could be on 3.0.1
  // (the version that originally skipped the 3.0.0 image re-sync — the
  // direct #3889 repro) or on 3.1.0 (skipping it for the same reason).
  // Either way the new migration must converge them — and crucially, the
  // loop runs every migration with key > fromVersion, so this fires even
  // when toVersion is still 4.0.0-rc.2 (the version currently in
  // packages/dashmate/package.json).
  [
    { fromVersion: '3.0.1', toVersion: '4.0.0-rc.2' },
    { fromVersion: '3.1.0', toVersion: '4.0.0-rc.2' },
    { fromVersion: '3.0.1', toVersion: '4.0.0-rc.3' },
    { fromVersion: '3.1.0', toVersion: '4.0.0-rc.3' },
  ].forEach(({ fromVersion, toVersion }) => {
    it(`re-syncs stale drive.abci and dapi.rsDapi images on ${fromVersion} → ${toVersion}`, () => {
      const rawConfigFile = {
        configFormatVersion: fromVersion,
        defaultConfigName: null,
        defaultGroupName: null,
        configs: {
          testnet: buildConfig({
            network: 'testnet',
            group: 'testnet',
            driveImage: STALE_DRIVE,
            rsDapiImage: STALE_RS_DAPI,
          }),
          mainnet: buildConfig({
            network: 'mainnet',
            group: 'mainnet',
            driveImage: STALE_DRIVE,
            rsDapiImage: STALE_RS_DAPI,
          }),
        },
      };

      const migrated = migrate(rawConfigFile, fromVersion, toVersion);

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

      expect(migrated.configFormatVersion).to.equal(toVersion);
    });
  });

  it('re-syncs the dev-series stale tags too (dashpay/drive:3-dev, dashpay/rs-dapi:3-dev)', () => {
    const rawConfigFile = {
      configFormatVersion: '3.1.0',
      defaultConfigName: null,
      defaultGroupName: null,
      configs: {
        testnet: buildConfig({
          network: 'testnet',
          group: 'testnet',
          driveImage: STALE_DRIVE_DEV,
          rsDapiImage: STALE_RS_DAPI_DEV,
        }),
      },
    };

    const migrated = migrate(rawConfigFile, '3.1.0', '4.0.0-rc.3');

    expect(migrated.configs.testnet.platform.drive.abci.docker.image)
      .to.equal(expectedDriveImage);
    expect(migrated.configs.testnet.platform.dapi.rsDapi.docker.image)
      .to.equal(expectedRsDapiImage);
  });

  // The exact case the rework targets: a node that already upgraded to
  // 4.0.0-rc.2 binary kept the stale dashpay/drive:3 / dashpay/rs-dapi:3
  // tags and got configFormatVersion: "4.0.0-rc.2" stamped on disk.
  // After a chore(release) bumps packages/dashmate/package.json to
  // 4.0.0-rc.3, the next dashmate load enters migrateConfigFile with
  // fromVersion=4.0.0-rc.2 / toVersion=4.0.0-rc.3, and the new
  // migration finally runs (the original 4.0.0-rc.2 key would have
  // been short-circuited by semver.gt('4.0.0-rc.2', '4.0.0-rc.2')).
  it('re-syncs already-stamped 4.0.0-rc.2 configs on the rc.3 release bump', () => {
    const rawConfigFile = {
      configFormatVersion: '4.0.0-rc.2',
      defaultConfigName: null,
      defaultGroupName: null,
      configs: {
        mainnet: buildConfig({
          network: 'mainnet',
          group: 'mainnet',
          driveImage: STALE_DRIVE,
          rsDapiImage: STALE_RS_DAPI,
        }),
      },
    };

    const migrated = migrate(rawConfigFile, '4.0.0-rc.2', '4.0.0-rc.3');

    expect(migrated.configs.mainnet.platform.drive.abci.docker.image)
      .to.equal(expectedDriveImage);
    expect(migrated.configs.mainnet.platform.dapi.rsDapi.docker.image)
      .to.equal(expectedRsDapiImage);
    expect(migrated.configFormatVersion).to.equal('4.0.0-rc.3');
  });

  // Documents the bound of what migration alone can fix: as long as
  // packages/dashmate/package.json sits at 4.0.0-rc.2, a config already
  // stamped 4.0.0-rc.2 short-circuits in migrateConfigFile before any
  // entry can run. This is the structural limitation that motivated
  // re-keying the migration at the *next* release rather than reusing
  // 4.0.0-rc.2 — and why the full rollout depends on a follow-up
  // chore(release) PR bumping the package version (same flow as the
  // 3.0.2 Envoy CVE: #3794 added the migration, #3796 cut the release).
  it('cannot reach a 4.0.0-rc.2 stale config while toVersion is still 4.0.0-rc.2 (release-bump prerequisite)', () => {
    const rawConfigFile = {
      configFormatVersion: '4.0.0-rc.2',
      defaultConfigName: null,
      defaultGroupName: null,
      configs: {
        mainnet: buildConfig({
          network: 'mainnet',
          group: 'mainnet',
          driveImage: STALE_DRIVE,
          rsDapiImage: STALE_RS_DAPI,
        }),
      },
    };

    const migrated = migrate(rawConfigFile, '4.0.0-rc.2', '4.0.0-rc.2');

    // No migration runs — config returned untouched.
    expect(migrated.configs.mainnet.platform.drive.abci.docker.image).to.equal(STALE_DRIVE);
    expect(migrated.configs.mainnet.platform.dapi.rsDapi.docker.image).to.equal(STALE_RS_DAPI);
    expect(migrated.configFormatVersion).to.equal('4.0.0-rc.2');
  });

  // Custom-image preservation. The original 4.0.0-rc.2 migration
  // unconditionally overwrote every drive/rsDapi image with the
  // current default, clobbering private forks, vendor-patched builds,
  // and `:latest` pins. The rework matches only the stale shipped
  // defaults — same style as the 3.0.2 Envoy CVE migration that
  // rewrote only `dashpay/envoy:1.30.*`.
  it('preserves custom Drive / rs-dapi images', () => {
    const rawConfigFile = {
      configFormatVersion: '3.1.0',
      defaultConfigName: null,
      defaultGroupName: null,
      configs: {
        mainnet: buildConfig({
          network: 'mainnet',
          group: 'mainnet',
          driveImage: CUSTOM_DRIVE,
          rsDapiImage: CUSTOM_RS_DAPI,
        }),
      },
    };

    const migrated = migrate(rawConfigFile, '3.1.0', '4.0.0-rc.3');

    expect(migrated.configs.mainnet.platform.drive.abci.docker.image).to.equal(CUSTOM_DRIVE);
    expect(migrated.configs.mainnet.platform.dapi.rsDapi.docker.image).to.equal(CUSTOM_RS_DAPI);
  });

  // Mixed config: one config has stale dashmate defaults (gets rewritten),
  // a sibling has a custom image (preserved). Exercises the same predicate
  // pattern as the 3.0.2 Envoy CVE migration end-to-end.
  it('only rewrites the stale config when siblings have custom images', () => {
    const rawConfigFile = {
      configFormatVersion: '3.1.0',
      defaultConfigName: null,
      defaultGroupName: null,
      configs: {
        testnet: buildConfig({
          network: 'testnet',
          group: 'testnet',
          driveImage: STALE_DRIVE,
          rsDapiImage: STALE_RS_DAPI,
        }),
        mainnet: buildConfig({
          network: 'mainnet',
          group: 'mainnet',
          driveImage: CUSTOM_DRIVE,
          rsDapiImage: CUSTOM_RS_DAPI,
        }),
      },
    };

    const migrated = migrate(rawConfigFile, '3.1.0', '4.0.0-rc.3');

    expect(migrated.configs.testnet.platform.drive.abci.docker.image)
      .to.equal(expectedDriveImage);
    expect(migrated.configs.testnet.platform.dapi.rsDapi.docker.image)
      .to.equal(expectedRsDapiImage);

    expect(migrated.configs.mainnet.platform.drive.abci.docker.image).to.equal(CUSTOM_DRIVE);
    expect(migrated.configs.mainnet.platform.dapi.rsDapi.docker.image).to.equal(CUSTOM_RS_DAPI);
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

    const migrated = migrate(rawConfigFile, '3.1.0', '4.0.0-rc.3');

    expect(migrated.configs.mainnet.platform.drive.abci.docker.image)
      .to.equal(expectedDriveImage);
    expect(migrated.configs.mainnet.platform.dapi.rsDapi).to.equal(undefined);
  });
});
