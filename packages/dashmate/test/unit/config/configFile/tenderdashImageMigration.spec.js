import fs from 'fs';
import path from 'path';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import getConfigFileMigrationsFactory from '../../../../configs/getConfigFileMigrationsFactory.js';
import migrateConfigFileFactory from '../../../../src/config/configFile/migrateConfigFileFactory.js';
import { PACKAGE_ROOT_DIR } from '../../../../src/constants.js';

describe('Tenderdash image migration', () => {
  const { version } = JSON.parse(fs.readFileSync(path.join(PACKAGE_ROOT_DIR, 'package.json'), 'utf8'));
  const base = getBaseConfigFactory()();
  const defaults = new Map(['base', 'mainnet', 'testnet'].map((name) => [name, base]));
  const getMigrations = getConfigFileMigrationsFactory(null, defaults);
  const migrateConfigFile = migrateConfigFileFactory(getMigrations);

  it('should use the floating 1.8 tag for new configurations', () => {
    expect(base.get('platform.drive.tenderdash.docker.image')).to.equal('dashpay/tenderdash:1.8');
  });

  for (const fromVersion of ['4.2.0-dev.1', '4.2.0-beta.3']) {
    it(`should migrate pinned images from config format ${fromVersion}`, () => {
      const configFileData = {
        configFormatVersion: fromVersion,
        configs: {
          mainnet: { platform: { drive: { tenderdash: { docker: { image: 'dashpay/tenderdash:1.8.0' } } } } },
          testnet: { platform: { drive: { tenderdash: { docker: { image: 'dashpay/tenderdash:1.7' } } } } },
          withoutPlatform: {},
          withoutDocker: { platform: { drive: { tenderdash: {} } } },
        },
      };

      const migrated = migrateConfigFile(configFileData, fromVersion, version);

      for (const name of ['mainnet', 'testnet']) {
        expect(migrated.configs[name].platform.drive.tenderdash.docker.image)
          .to.equal('dashpay/tenderdash:1.8');
      }
      expect(migrated.configs.withoutPlatform).to.deep.equal({});
      expect(migrated.configs.withoutDocker).to.deep.equal({
        platform: { drive: { tenderdash: {} } },
      });
      expect(migrated.configFormatVersion).to.equal('4.2.0');
      expect(migrateConfigFile(migrated, migrated.configFormatVersion, version)).to.equal(migrated);
    });
  }
});
