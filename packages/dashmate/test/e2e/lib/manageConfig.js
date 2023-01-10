const ConfigFileJsonRepository = require("../../../src/config/configFile/ConfigFileJsonRepository");
const migrateConfigFile = require("../../../src/config/configFile/migrateConfigFile");


/**
 * Return configuration file based on provided config name
 *
 * @param {string} configName
 * @return {Config[]}
 */

async function getGroupConfig(configName) {
  const configFileRepository = new ConfigFileJsonRepository(migrateConfigFile);
  const configFile = await configFileRepository.read();
  return configFile.getGroupConfigs(configName);
}

async function getConfig(configName) {
  const configFileRepository = new ConfigFileJsonRepository(migrateConfigFile);
  const configFile = await configFileRepository.read();
  return configFile.getConfig(configName);
}

async function isGroupConfigExist(configName) {
  const configFileRepository = new ConfigFileJsonRepository(migrateConfigFile);
  const configFile = await configFileRepository.read();
  return configFile.isGroupExists(configName);
}

async function isConfigExist(configName) {
  const configFileRepository = new ConfigFileJsonRepository(migrateConfigFile);
  const configFile = await configFileRepository.read();
  return configFile.isConfigExists(configName);
}

module.exports = {
  getGroupConfig,
  getConfig,
  isGroupConfigExist,
  isConfigExist
}
