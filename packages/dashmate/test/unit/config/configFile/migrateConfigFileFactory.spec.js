import fs from 'fs';
import path from 'path';
import semver from 'semver';
import HomeDir from '../../../../src/config/HomeDir.js';
import { PACKAGE_ROOT_DIR } from '../../../../src/constants.js';
import createDIContainer from '../../../../src/createDIContainer.js';
import getConfigFileDataV0250 from '../../../../src/test/fixtures/getConfigFileDataV0250.js';

describe('migrateConfigFileFactory', () => {
  let mockConfigFileData;
  let container;
  let createConfigFile;
  let migrateConfigFile;

  beforeEach(async () => {
    container = await createDIContainer();
    migrateConfigFile = container.resolve('migrateConfigFile');
    createConfigFile = container.resolve('createConfigFile');

    const homeDir = container.resolve('homeDir');
    homeDir.change(new HomeDir('/Users/dashmate/.dashmate', true));

    mockConfigFileData = getConfigFileDataV0250();
  });

  it('should migrate v0.25.0 config file to the latest one', async () => {
    const currentConfigFile = createConfigFile();
    const currentConfigFileData = currentConfigFile.toObject();
    const { version } = JSON.parse(fs.readFileSync(path.join(PACKAGE_ROOT_DIR, 'package.json'), 'utf8'));

    const migratedConfigFileData = migrateConfigFile(
      mockConfigFileData,
      mockConfigFileData.configFormatVersion,
      version,
    );

    for (const [name, defaultConfig] of Object.entries(currentConfigFileData.configs)) {
      expect(defaultConfig).to.be.deep.equal(
        migratedConfigFileData.configs[name],
        `Migrated and default ${name} config do not match`,
      );
    }

    delete currentConfigFileData.configs;
    delete migratedConfigFileData.configs;

    expect(migratedConfigFileData).to.be.deep.equal(currentConfigFileData);
  });

  it('should refresh the version-derived platform images when upgrading from a version that has no migration of its own', async () => {
    // The drive and rs-dapi image tags are derived from the package major
    // version. An operator upgrading from a recent version (e.g. a prerelease
    // of the same major) sits past the legacy 0.25.x migrations that refresh
    // images from the base config, so a per-release migration must re-pin them
    // or they stay stuck on the old/prerelease tag.
    //
    // 4.0.0-rc.2 is deliberately not a migration key: the table walk below can
    // only start from versions that are keys, so this covers the case it
    // structurally cannot reach.
    const { version } = JSON.parse(fs.readFileSync(path.join(PACKAGE_ROOT_DIR, 'package.json'), 'utf8'));

    const defaultConfigFileData = createConfigFile().toObject();
    const [firstConfigName] = Object.keys(defaultConfigFileData.configs);
    const expectedDriveImage = defaultConfigFileData
      .configs[firstConfigName].platform.drive.abci.docker.image;
    const expectedRsDapiImage = defaultConfigFileData
      .configs[firstConfigName].platform.dapi.rsDapi.docker.image;

    const staleConfigFileData = createConfigFile().toObject();
    staleConfigFileData.configFormatVersion = '4.0.0-rc.2';
    for (const options of Object.values(staleConfigFileData.configs)) {
      options.platform.drive.abci.docker.image = 'dashpay/drive:4-rc';
      options.platform.dapi.rsDapi.docker.image = 'dashpay/rs-dapi:4-rc';
    }

    const migratedConfigFileData = migrateConfigFile(
      staleConfigFileData,
      staleConfigFileData.configFormatVersion,
      version,
    );

    for (const [name, options] of Object.entries(migratedConfigFileData.configs)) {
      expect(options.platform.drive.abci.docker.image).to.equal(
        expectedDriveImage,
        `drive image not refreshed for ${name}`,
      );
      expect(options.platform.dapi.rsDapi.docker.image).to.equal(
        expectedRsDapiImage,
        `rs-dapi image not refreshed for ${name}`,
      );
    }
  });

  it('should refresh version-derived images from every config version that has a migration', async () => {
    // The drive and rs-dapi image tags are derived from the package version, so
    // every release that changes the major or the prerelease identifier changes
    // them. Config files store resolved values, so operators only pick the new
    // tags up when a migration re-pins them; a release that forgets leaves them
    // running the images of the line they installed.
    //
    // Rather than a test per migration, walk the migration table itself: every
    // config version an operator can be sitting on must end up on the images a
    // fresh install produces today. New migrations are covered without touching
    // this test. A failure here means the upcoming release needs its own
    // migration re-pinning these images from the base config.
    //
    // The migration at this key and every older one rewrite the config shape of
    // their own era and throw against a config built from the current defaults
    // (this one reads core.log.file.categories, which current configs no longer
    // have). They stay covered by 'should migrate v0.25.0 config file to the
    // latest one', which carries a real historical config all the way up.
    const OLDEST_MIGRATABLE_FROM_VERSION = '1.3.0-dev.3';

    const { version } = JSON.parse(fs.readFileSync(path.join(PACKAGE_ROOT_DIR, 'package.json'), 'utf8'));
    const getConfigFileMigrations = container.resolve('getConfigFileMigrations');

    const defaultConfigFileData = createConfigFile().toObject();
    const [firstConfigName] = Object.keys(defaultConfigFileData.configs);
    const expectedDriveImage = defaultConfigFileData
      .configs[firstConfigName].platform.drive.abci.docker.image;
    const expectedRsDapiImage = defaultConfigFileData
      .configs[firstConfigName].platform.dapi.rsDapi.docker.image;

    const migrationVersions = Object.keys(getConfigFileMigrations());
    const releases = migrationVersions
      .filter((migrationVersion) => semver.gte(migrationVersion, OLDEST_MIGRATABLE_FROM_VERSION))
      .sort(semver.compare);

    // The newest migration is the one a release is most likely to forget, so
    // guard against a floor that silently drops it along with everything else.
    expect(releases).to.include(
      migrationVersions.sort(semver.compare).at(-1),
      'the newest migration is not being checked',
    );

    // Collect every mismatch instead of stopping at the first, so one stale
    // release does not hide the others.
    const stale = [];

    for (const release of releases) {
      // The tag that release's base config produced, derived the same way
      // getBaseConfigFactory derives it.
      const prereleaseTag = semver.prerelease(release) === null ? '' : `-${semver.prerelease(release)[0]}`;
      const releaseImageVersion = `${semver.major(release)}${prereleaseTag}`;

      const configFileData = createConfigFile().toObject();
      configFileData.configFormatVersion = release;
      for (const options of Object.values(configFileData.configs)) {
        options.platform.drive.abci.docker.image = `dashpay/drive:${releaseImageVersion}`;
        options.platform.dapi.rsDapi.docker.image = `dashpay/rs-dapi:${releaseImageVersion}`;
      }

      const migratedConfigFileData = migrateConfigFile(configFileData, release, version);

      for (const [name, options] of Object.entries(migratedConfigFileData.configs)) {
        const { image: driveImage } = options.platform.drive.abci.docker;
        if (driveImage !== expectedDriveImage) {
          stale.push(`${release} -> ${name}: drive ${driveImage}, expected ${expectedDriveImage}`);
        }

        const { image: rsDapiImage } = options.platform.dapi.rsDapi.docker;
        if (rsDapiImage !== expectedRsDapiImage) {
          stale.push(`${release} -> ${name}: rs-dapi ${rsDapiImage}, expected ${expectedRsDapiImage}`);
        }
      }
    }

    expect(stale).to.deep.equal(
      [],
      'images left on a stale tag; add a migration keyed at the upcoming release that re-pins them from the base config',
    );
  });

  it('should move only the stock version-derived tags and leave operator images alone', async () => {
    // The re-pin is guarded so it never overwrites an image the operator chose.
    // Both halves of that guard need pinning: the stock tags a release publishes
    // must move, and anything else must survive untouched. Every operator image
    // below except the private-registry one sits in the dashpay namespace, so
    // the suffix is what has to reject them - a foreign registry would pass on
    // the namespace alone and never exercise it.
    //
    // 4.0.0 is the only starting point where this migration acts alone. Below
    // it the unconditional re-pin in the '4.0.0' migration overwrites every
    // image regardless, operator-chosen ones included.
    const FROM_VERSION = '4.0.0';
    // Kept in step with the identifiers the migration itself lists.
    const STOCK_PRERELEASE_IDS = ['alpha', 'beta', 'dev', 'hotfix', 'pr', 'rc'];

    const { version } = JSON.parse(fs.readFileSync(path.join(PACKAGE_ROOT_DIR, 'package.json'), 'utf8'));

    const defaultConfigFileData = createConfigFile().toObject();
    const [firstConfigName] = Object.keys(defaultConfigFileData.configs);
    const expectedDriveImage = defaultConfigFileData
      .configs[firstConfigName].platform.drive.abci.docker.image;
    const expectedRsDapiImage = defaultConfigFileData
      .configs[firstConfigName].platform.dapi.rsDapi.docker.image;

    // Both images are named explicitly rather than derived from one another, so
    // a case added later cannot seed the rs-dapi slot with a drive image and
    // pass without testing anything.
    const migrateImages = (driveImage, rsDapiImage) => {
      const configFileData = createConfigFile().toObject();
      configFileData.configFormatVersion = FROM_VERSION;
      for (const options of Object.values(configFileData.configs)) {
        options.platform.drive.abci.docker.image = driveImage;
        options.platform.dapi.rsDapi.docker.image = rsDapiImage;
      }

      const migrated = migrateConfigFile(configFileData, FROM_VERSION, version);

      return migrated.configs[firstConfigName].platform;
    };

    const stockTags = ['4', ...STOCK_PRERELEASE_IDS.map((id) => `4-${id}`)];

    for (const tag of stockTags) {
      const platform = migrateImages(`dashpay/drive:${tag}`, `dashpay/rs-dapi:${tag}`);

      expect(platform.drive.abci.docker.image).to.equal(
        expectedDriveImage,
        `stock drive image dashpay/drive:${tag} was not moved`,
      );
      expect(platform.dapi.rsDapi.docker.image).to.equal(
        expectedRsDapiImage,
        `stock rs-dapi image dashpay/rs-dapi:${tag} was not moved`,
      );
    }

    const operatorImages = [
      // vendor-patched build under the stock namespace
      ['dashpay/drive:4-patched', 'dashpay/rs-dapi:4-patched'],
      // locally built image, operator's own tag
      ['dashpay/drive:4-local', 'dashpay/rs-dapi:4-local'],
      // pinned to an exact version
      ['dashpay/drive:4.0.1', 'dashpay/rs-dapi:4.0.1'],
      // floating tag the operator opted into
      ['dashpay/drive:latest', 'dashpay/rs-dapi:latest'],
      // private registry
      ['registry.example.com/drive:4-rc', 'registry.example.com/rs-dapi:4-rc'],
    ];

    for (const [driveImage, rsDapiImage] of operatorImages) {
      const platform = migrateImages(driveImage, rsDapiImage);

      expect(platform.drive.abci.docker.image).to.equal(
        driveImage,
        `operator drive image ${driveImage} was overwritten`,
      );
      expect(platform.dapi.rsDapi.docker.image).to.equal(
        rsDapiImage,
        `operator rs-dapi image ${rsDapiImage} was overwritten`,
      );
    }
  });
});
