import fs from 'fs';
import path from 'path';
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

  it('should move a stock 4.x platform image onto the current base image', async () => {
    // An operator running a released 4.x sits past every migration that re-pins
    // the drive and rs-dapi images from the base config, so without a migration
    // keyed above their version they keep running the images of the major they
    // installed even though the base config now derives a different tag.
    const { version } = JSON.parse(fs.readFileSync(path.join(PACKAGE_ROOT_DIR, 'package.json'), 'utf8'));

    const defaultConfigFileData = createConfigFile().toObject();
    const [firstConfigName] = Object.keys(defaultConfigFileData.configs);
    const expectedDriveImage = defaultConfigFileData
      .configs[firstConfigName].platform.drive.abci.docker.image;
    const expectedRsDapiImage = defaultConfigFileData
      .configs[firstConfigName].platform.dapi.rsDapi.docker.image;

    const staleConfigFileData = createConfigFile().toObject();
    staleConfigFileData.configFormatVersion = '4.0.0';
    for (const options of Object.values(staleConfigFileData.configs)) {
      options.platform.drive.abci.docker.image = 'dashpay/drive:4';
      options.platform.dapi.rsDapi.docker.image = 'dashpay/rs-dapi:4';
    }

    const migratedConfigFileData = migrateConfigFile(
      staleConfigFileData,
      staleConfigFileData.configFormatVersion,
      version,
    );

    for (const [name, options] of Object.entries(migratedConfigFileData.configs)) {
      expect(options.platform.drive.abci.docker.image).to.equal(
        expectedDriveImage,
        `drive image not moved onto the base image for ${name}`,
      );
      expect(options.platform.dapi.rsDapi.docker.image).to.equal(
        expectedRsDapiImage,
        `rs-dapi image not moved onto the base image for ${name}`,
      );
    }
  });

  it('should keep a custom platform image chosen by the operator', async () => {
    // Re-pinning images must never overwrite an image the operator selected
    // themselves - a private fork, a vendor-patched build or a tag pinned to an
    // exact version. Only the stock Dash tags are safe to move.
    const { version } = JSON.parse(fs.readFileSync(path.join(PACKAGE_ROOT_DIR, 'package.json'), 'utf8'));

    const customDriveImage = 'registry.example.com/drive:4-patched';
    const customRsDapiImage = 'registry.example.com/rs-dapi:4-patched';

    const customConfigFileData = createConfigFile().toObject();
    customConfigFileData.configFormatVersion = '4.0.0';
    for (const options of Object.values(customConfigFileData.configs)) {
      options.platform.drive.abci.docker.image = customDriveImage;
      options.platform.dapi.rsDapi.docker.image = customRsDapiImage;
    }

    const migratedConfigFileData = migrateConfigFile(
      customConfigFileData,
      customConfigFileData.configFormatVersion,
      version,
    );

    for (const [name, options] of Object.entries(migratedConfigFileData.configs)) {
      expect(options.platform.drive.abci.docker.image).to.equal(
        customDriveImage,
        `custom drive image overwritten for ${name}`,
      );
      expect(options.platform.dapi.rsDapi.docker.image).to.equal(
        customRsDapiImage,
        `custom rs-dapi image overwritten for ${name}`,
      );
    }
  });
});
