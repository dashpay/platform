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

  // Regression for CVE-2026-47774 / GHSA-22m2-hvr2-xqc8 (HTTP/2 DoS in Envoy).
  // Configs created on 3.0.0..4.0.0-beta.2 pin the EOL, vulnerable
  // `dashpay/envoy:1.30.2-impr.1`, and no migration above 3.0.0 touched the
  // gateway image before the `4.0.0-beta.3` migration. This pins that the
  // bump actually reaches that cohort — without the migration the image stays
  // on the vulnerable 1.30 line and this fails.
  it('should bump the vulnerable Platform Gateway (Envoy) image for configs from 4.0.0-beta.2', () => {
    const base = container.resolve('defaultConfigs').get('base');
    const patchedImage = base.get('platform.gateway.docker.image');

    // The base default itself must be on a patched line (>= 1.35), never the
    // vulnerable 1.30.x line — pins the security intent of the fix.
    expect(patchedImage).to.match(/^dashpay\/envoy:1\.(3[5-9]|[4-9]\d)\./);

    const rawConfigFile = {
      configFormatVersion: '4.0.0-beta.2',
      configs: {
        testnet: {
          platform: {
            gateway: {
              docker: {
                image: 'dashpay/envoy:1.30.2-impr.1',
              },
            },
          },
        },
      },
    };

    const migrated = migrateConfigFile(rawConfigFile, '4.0.0-beta.2', '4.0.0-beta.3');

    const migratedImage = migrated.configs.testnet.platform.gateway.docker.image;
    expect(migratedImage).to.equal(patchedImage);
    expect(migratedImage).to.not.equal('dashpay/envoy:1.30.2-impr.1');
  });
});
