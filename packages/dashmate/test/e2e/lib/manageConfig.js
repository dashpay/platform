const ConfigFileJsonRepository = require('../../../src/config/configFile/ConfigFileJsonRepository');
const migrateConfigFile = require('../../../src/config/configFile/migrateConfigFile');

/**
 * Return configuration file based on provided config name
 * @param {string} configName
 * @return {Promise<Config[]>}
 */
async function getConfig(configName) {
  let config;
  const configFileRepository = new ConfigFileJsonRepository(migrateConfigFile);
  const configFile = await configFileRepository.read();

  switch (configName) {
    case 'local':
      config = configFile.getGroupConfigs(configName);
      break;
    case 'testnet': // Fallthrough
    case 'mainnet':
      config = configFile.getConfig(configName);
      break;
    default:
      throw new Error('Wrong config name.');
  }
  return config;
}

/**
 * Check if config exist
 * @param {string} configName
 * @return {Promise<boolean>}
 */
async function isConfigExist(configName) {
  let bool;
  const configFileRepository = new ConfigFileJsonRepository(migrateConfigFile);
  const configFile = await configFileRepository.read();

  switch (configName) {
    case 'local':
      bool = configFile.isGroupExists(configName);
      break;
    case 'testnet': // Fallthrough
    case 'mainnet':
      bool = configFile.isConfigExists(configName);
      break;
    default:
      throw new Error('Wrong config name.');
  }
  return bool;
}

module.exports = {
  getConfig,
  isConfigExist,
};
