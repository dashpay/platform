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

  it('should refresh the version-derived platform images when upgrading from a recent version', async () => {
    // The drive and rs-dapi image tags are derived from the package major
    // version. An operator upgrading from a recent version (e.g. a prerelease
    // of the same major) sits past the legacy 0.25.x migrations that refresh
    // images from the base config, so a per-release migration must re-pin them
    // or they stay stuck on the old/prerelease tag.
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

  it('should refresh version-derived images for every release an operator can upgrade from', async () => {
    // The drive and rs-dapi image tags are derived from the package version, so
    // every release that changes the major or the prerelease identifier changes
    // them. Config files store resolved values, so operators only pick the new
    // tags up when a migration re-pins them; a release that forgets leaves them
    // running the images of the line they installed.
    //
    // Rather than a test per migration, walk the migration table itself: every
    // release an operator can be sitting on must end up on the images a fresh
    // install produces today. New migrations are covered without touching this
    // test.
    //
    // Migrations older than this key rewrite the config shape of their own era
    // and cannot run against a config built from the current defaults. That
    // range is covered by the v0.25.0 fixture test above, which carries a real
    // historical config all the way to the current one.
    const OLDEST_CURRENT_SHAPED_RELEASE = '1.3.0-dev.3';

    const { version } = JSON.parse(fs.readFileSync(path.join(PACKAGE_ROOT_DIR, 'package.json'), 'utf8'));
    const getConfigFileMigrations = container.resolve('getConfigFileMigrations');

    const defaultConfigFileData = createConfigFile().toObject();
    const [firstConfigName] = Object.keys(defaultConfigFileData.configs);
    const expectedDriveImage = defaultConfigFileData
      .configs[firstConfigName].platform.drive.abci.docker.image;
    const expectedRsDapiImage = defaultConfigFileData
      .configs[firstConfigName].platform.dapi.rsDapi.docker.image;

    const releases = Object.keys(getConfigFileMigrations())
      .filter((migrationVersion) => semver.gte(migrationVersion, OLDEST_CURRENT_SHAPED_RELEASE));

    expect(releases).to.have.lengthOf.at.least(1, 'no releases selected to check');

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
        expect(options.platform.drive.abci.docker.image).to.equal(
          expectedDriveImage,
          `drive image not refreshed for ${name} upgrading from ${release}`,
        );
        expect(options.platform.dapi.rsDapi.docker.image).to.equal(
          expectedRsDapiImage,
          `rs-dapi image not refreshed for ${name} upgrading from ${release}`,
        );
      }
    }
  });
});
