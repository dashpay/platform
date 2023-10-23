const _ = require('lodash');
const Ajv = require('ajv');
const addFormats = require('ajv-formats');

const createDIContainer = require('../../../../../src/createDIContainer');
const mockConfig = require('./mock_config.json');
const configJsonSchema = require('../../../../../src/config/configJsonSchema');
const packageJson = require('../../../../../package.json');

describe('MigrateConfigFileCommand', () => {
  let config;
  let container;
  let ajv;

  beforeEach(async () => {
    ajv = new Ajv();
    addFormats(ajv, { mode: 'fast', formats: ['ipv4'] });

    container = await createDIContainer();
    config = _.cloneDeep(mockConfig);
  });

  it('should gracefully migrate', async () => {
    const migrateConfigFile = container.resolve('migrateConfigFile');
    const getBaseConfig = container.resolve('getBaseConfig');
    const getMainnetConfig = container.resolve('getMainnetConfig');
    const getTestnetConfig = container.resolve('getTestnetConfig');

    const migratedConfigFileData = migrateConfigFile(
      config,
      config.configFormatVersion,
      packageJson.version,
    );

    // check config is valid after migration
    for (const configName of Object.keys(migratedConfigFileData.configs)) {
      const myConfig = migratedConfigFileData.configs[configName];
      const isValid = ajv.validate(configJsonSchema, myConfig);

      expect(isValid).to.equal(true);

      let targetConfig;

      switch (myConfig.network) {
        case 'local':
          targetConfig = getBaseConfig();
          break;
        case 'testnet':
          targetConfig = configName === 'base' ? getBaseConfig() : getTestnetConfig();
          break;
        case 'mainnet':
          targetConfig = getMainnetConfig();
          break;
        default:
          throw new Error('Unknown network type');
      }

      // check image version
      expect(myConfig.core.docker.image)
        .to.equal(targetConfig.options.core.docker.image);
      expect(myConfig.platform.drive.abci.docker.image)
        .to.equal(targetConfig.options.platform.drive.abci.docker.image);
      expect(myConfig.platform.drive.tenderdash.docker.image)
        .to.equal(targetConfig.options.platform.drive.tenderdash.docker.image);

      // check genesis
      if (configName !== 'base') {
        expect(myConfig.platform.drive.tenderdash.genesis.chain_id)
          .to.equal(targetConfig.options.platform.drive.tenderdash.genesis.chain_id);
        expect(myConfig.platform.drive.tenderdash.genesis.genesis_time)
          .to.equal(targetConfig.options.platform.drive.tenderdash.genesis.genesis_time);

        expect(myConfig.platform.drive.tenderdash.genesis.initial_core_chain_locked_height)
          .to.equal(targetConfig.options
            .platform.drive.tenderdash.genesis.initial_core_chain_locked_height);
      }
    }
  });
});
