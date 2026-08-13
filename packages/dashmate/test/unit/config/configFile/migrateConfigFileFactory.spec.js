import fs from 'fs';
import path from 'path';
import { STOCK_PRERELEASE_IDS } from '../../../../src/config/stockImages.js';
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

  it('should unset a stock version-derived tag and leave an operator image alone', async () => {
    // The migration records intent rather than re-pinning a value: a tag a
    // release published becomes unset, meaning "use the line this build ships",
    // while an image the operator chose stays exactly as they set it.
    //
    // This is the last time a stock tag is recognised by shape, so both halves
    // need pinning - a widened pattern would overwrite operator images, and a
    // narrowed one would strand operators on a tag that no longer moves.
    const FROM_VERSION = '4.0.0';

    // Spelled out rather than only imported, so adding an identifier to the
    // shared list without deciding it belongs here fails instead of silently
    // widening what the migration unsets.
    const expectedPrereleaseIds = ['alpha', 'beta', 'dev', 'hotfix', 'pr', 'rc'];
    expect([...STOCK_PRERELEASE_IDS].sort()).to.deep.equal(
      expectedPrereleaseIds,
      'the published prerelease identifiers changed; confirm the new one should unset operators',
    );

    const { version } = JSON.parse(fs.readFileSync(path.join(PACKAGE_ROOT_DIR, 'package.json'), 'utf8'));
    const [firstConfigName] = Object.keys(createConfigFile().toObject().configs);

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

    const stockTags = ['4', ...expectedPrereleaseIds.map((id) => `4-${id}`)];

    for (const tag of stockTags) {
      const platform = migrateImages(`dashpay/drive:${tag}`, `dashpay/rs-dapi:${tag}`);

      expect(platform.drive.abci.docker.image).to.equal(
        null,
        `stock drive image dashpay/drive:${tag} was not unset`,
      );
      expect(platform.dapi.rsDapi.docker.image).to.equal(
        null,
        `stock rs-dapi image dashpay/rs-dapi:${tag} was not unset`,
      );
    }

    const operatorImages = [
      // vendor-patched build under the stock namespace
      ['dashpay/drive:4-patched', 'dashpay/rs-dapi:4-patched'],
      // a major no release had published when this migration shipped, so it can
      // only be something the operator built themselves
      ['dashpay/drive:5', 'dashpay/rs-dapi:5'],
      ['dashpay/drive:999', 'dashpay/rs-dapi:999'],
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

  it('should keep an operator image that predates the 4.0.0 re-pin', async () => {
    // Every config older than 4.0.0 crosses the unconditional re-pin in that
    // migration, so it is the first place operator intent can be respected. It
    // used to overwrite unconditionally, which destroyed a custom image before
    // any later migration could tell it apart from a stale default.
    const FROM_VERSION = '3.1.0';
    const customDriveImage = 'registry.example.com/security-patched-drive:stable';
    const customRsDapiImage = 'registry.example.com/security-patched-rs-dapi:stable';

    const { version } = JSON.parse(fs.readFileSync(path.join(PACKAGE_ROOT_DIR, 'package.json'), 'utf8'));

    const configFileData = createConfigFile().toObject();
    configFileData.configFormatVersion = FROM_VERSION;
    for (const options of Object.values(configFileData.configs)) {
      options.platform.drive.abci.docker.image = customDriveImage;
      options.platform.dapi.rsDapi.docker.image = customRsDapiImage;
    }

    const migrated = migrateConfigFile(configFileData, FROM_VERSION, version);

    for (const [name, options] of Object.entries(migrated.configs)) {
      expect(options.platform.drive.abci.docker.image).to.equal(
        customDriveImage,
        `operator drive image was overwritten for ${name}`,
      );
      expect(options.platform.dapi.rsDapi.docker.image).to.equal(
        customRsDapiImage,
        `operator rs-dapi image was overwritten for ${name}`,
      );
    }
  });

  it('should unset a stock tag carried across majors', async () => {
    // A config from before 4.0.0 carries a tag of its own era. Those still have
    // to be recognised as published defaults, or an operator who never chose an
    // image is stranded on a tag nothing moves any more.
    const FROM_VERSION = '3.1.0';

    const { version } = JSON.parse(fs.readFileSync(path.join(PACKAGE_ROOT_DIR, 'package.json'), 'utf8'));

    // v1.0.0 and v1.0.1 derived major.minor tags; the derivation changed to the
    // major alone in v1.0.2, and the 0.x line used major.minor throughout.
    for (const tag of ['3', '2', '1-dev', '1.0', '1.0-rc', '0.25', '0.24']) {
      const configFileData = createConfigFile().toObject();
      configFileData.configFormatVersion = FROM_VERSION;
      for (const options of Object.values(configFileData.configs)) {
        options.platform.drive.abci.docker.image = `dashpay/drive:${tag}`;
        options.platform.dapi.rsDapi.docker.image = `dashpay/rs-dapi:${tag}`;
      }

      const migrated = migrateConfigFile(configFileData, FROM_VERSION, version);

      for (const [name, options] of Object.entries(migrated.configs)) {
        expect(options.platform.drive.abci.docker.image).to.equal(
          null,
          `stock drive image dashpay/drive:${tag} was not unset for ${name}`,
        );
      }
    }
  });

  it('should carry an operator image from the oldest migratable config to the newest', async () => {
    // The strongest form of the guarantee: a config old enough to cross every
    // migration in the table still arrives with the operator's image intact.
    // These re-pins used to overwrite unconditionally, so an operator running
    // their own build lost it at the first one they crossed - far earlier than
    // any guard could see it.
    const customDriveImage = 'registry.example.com/security-patched-drive:stable';
    const { version } = JSON.parse(fs.readFileSync(path.join(PACKAGE_ROOT_DIR, 'package.json'), 'utf8'));

    const oldConfigFileData = getConfigFileDataV0250();
    for (const options of Object.values(oldConfigFileData.configs)) {
      options.platform.drive.abci.docker.image = customDriveImage;
    }

    const migrated = migrateConfigFile(
      oldConfigFileData,
      oldConfigFileData.configFormatVersion,
      version,
    );

    for (const [name, options] of Object.entries(migrated.configs)) {
      expect(options.platform.drive.abci.docker.image).to.equal(
        customDriveImage,
        `operator drive image was overwritten for ${name}`,
      );
    }
  });

  // Upgrading Dashmate has to leave a mark on the config file even when nothing
  // needed migrating, because that recorded version is the only thing that tells
  // the next command every config is stale. `ConfigFileJsonRepository.read()`
  // marks all configs changed when the version moved, and that is what re-renders
  // service templates after an upgrade. Stamping the version only when a
  // migration happened to apply would leave nodes running templates rendered by
  // an older Dashmate, with nothing to signal it.
  it('should record the new version after an upgrade even when no migration applies', () => {
    const configFileData = getConfigFileDataV0250();

    // Both versions sit above every migration key, so nothing can match and the
    // recorded version is the only thing the upgrade can change.
    const installedVersion = '98.0.0';
    const upgradedVersion = '99.0.0';

    configFileData.configFormatVersion = installedVersion;

    const migrated = migrateConfigFile(
      configFileData,
      configFileData.configFormatVersion,
      upgradedVersion,
    );

    expect(migrated.configFormatVersion).to.equal(upgradedVersion);

    // And migrating again from there is a no-op, so the mark does not repeat.
    const again = migrateConfigFile(
      migrated,
      migrated.configFormatVersion,
      upgradedVersion,
    );

    expect(again.configFormatVersion).to.equal(upgradedVersion);
  });
});
