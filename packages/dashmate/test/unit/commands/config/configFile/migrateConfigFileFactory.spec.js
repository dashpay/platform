const createDIContainer = require('../../../../../src/createDIContainer');
const getConfigFileDataV0250 = require('../../../../../src/test/fixtures/getConfigFileDataV0250');
const packageJson = require('../../../../../package.json');
const HomeDir = require('../../../../../src/config/HomeDir');

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

  it('should migrate', async () => {
    const currentConfigFile = createConfigFile();
    const currentConfigFileData = currentConfigFile.toObject();

    const migratedConfigFileData = migrateConfigFile(
      mockConfigFileData,
      mockConfigFileData.configFormatVersion,
      packageJson.version,
    );

    for (const [name, config] of Object.entries(currentConfigFileData.configs)) {
      expect(config).to.be.deep.equal(migratedConfigFileData.configs[name]);
    }

    delete currentConfigFileData.configs;
    delete migratedConfigFileData.configs;

    expect(migratedConfigFileData).to.be.deep.equal(currentConfigFileData);
  });
});
