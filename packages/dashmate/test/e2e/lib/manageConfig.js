const ConfigFileJsonRepository = require("../../../src/config/configFile/ConfigFileJsonRepository");
const migrateConfigFile = require("../../../src/config/configFile/migrateConfigFile");


/**
 * Return configuration file based on provided config name
 *
 * @param {string} configName
 * @return {Config[]}
 */

async function getConfig(configName) {
  const configFileRepository = new ConfigFileJsonRepository(migrateConfigFile);
  const configFile = await configFileRepository.read();

  switch(configName) {
    case 'local':
      return configFile.getGroupConfigs(configName);
    case 'testnet':  // Fallthrough
    case 'mainnet':
      return configFile.getConfig(configName);
  }
}

async function isConfigExist(configName) {
  const configFileRepository = new ConfigFileJsonRepository(migrateConfigFile);
  const configFile = await configFileRepository.read();

  switch(configName) {
    case 'local':
      return configFile.isGroupExists(configName);
    case 'testnet':  // Fallthrough
    case 'mainnet':
      return configFile.isConfigExists(configName);
  }
}

module.exports = {
  getConfig,
  isConfigExist
}
