import fs from 'fs';
import path from 'path';
import HomeDir from '../../../../src/config/HomeDir.js';
import { PACKAGE_ROOT_DIR } from '../../../../src/constants.js';
import createDIContainer from '../../../../src/createDIContainer.js';
import migrateConfigFileFactory from '../../../../src/config/configFile/migrateConfigFileFactory.js';
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

  it('should not apply migrations newer than the target format version', () => {
    let futureMigrationCalled = false;
    const migrateToTarget = migrateConfigFileFactory(() => ({
      '4.1.0-beta.3': (configFile) => {
        futureMigrationCalled = true;
        return configFile;
      },
    }));
    const configFileData = {
      configFormatVersion: '4.0.0',
      configs: {},
    };

    const migratedConfigFileData = migrateToTarget(
      configFileData,
      configFileData.configFormatVersion,
      '4.1.0-beta.2',
    );

    expect(futureMigrationCalled).to.equal(false);
    expect(migratedConfigFileData.configFormatVersion).to.equal('4.1.0-beta.2');
  });

  it('should repair stable image pins after a config was stamped with beta.2', async () => {
    const defaultConfigFileData = createConfigFile().toObject();
    const [firstConfigName] = Object.keys(defaultConfigFileData.configs);
    const expectedDriveImage = defaultConfigFileData
      .configs[firstConfigName].platform.drive.abci.docker.image;
    const expectedRsDapiImage = defaultConfigFileData
      .configs[firstConfigName].platform.dapi.rsDapi.docker.image;

    const stableConfigFileData = createConfigFile().toObject();
    stableConfigFileData.configFormatVersion = '4.1.0-beta.2';
    for (const options of Object.values(stableConfigFileData.configs)) {
      options.platform.drive.abci.docker.image = 'dashpay/drive:4';
      options.platform.dapi.rsDapi.docker.image = 'dashpay/rs-dapi:4';
    }

    const migratedConfigFileData = migrateConfigFile(
      stableConfigFileData,
      stableConfigFileData.configFormatVersion,
      '4.1.0-beta.3',
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

  it('should preserve custom image pins while repairing beta.2 configs', async () => {
    const customConfigFileData = createConfigFile().toObject();
    customConfigFileData.configFormatVersion = '4.1.0-beta.2';
    for (const options of Object.values(customConfigFileData.configs)) {
      options.platform.drive.abci.docker.image = 'registry.example/drive@sha256:abc123';
      options.platform.dapi.rsDapi.docker.image = 'registry.example/rs-dapi:custom';
    }

    const migratedConfigFileData = migrateConfigFile(
      customConfigFileData,
      customConfigFileData.configFormatVersion,
      '4.1.0-beta.3',
    );

    for (const options of Object.values(migratedConfigFileData.configs)) {
      expect(options.platform.drive.abci.docker.image)
        .to.equal('registry.example/drive@sha256:abc123');
      expect(options.platform.dapi.rsDapi.docker.image)
        .to.equal('registry.example/rs-dapi:custom');
    }
  });
});
