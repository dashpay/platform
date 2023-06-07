const Config = require('../Config');

const ConfigFile = require('../configFile/ConfigFile');

const packageJson = require('../../../package.json');

/**
 * @param {Object} systemConfigs
 * @return {createSystemConfigs}
 */
function createSystemConfigsFactory(systemConfigs) {
  /**
   * @typedef {createSystemConfigs}
   * @returns {ConfigFile}
   */
  function createSystemConfigs() {
    const configFile = new ConfigFile(
      [],
      packageJson.version,
      null,
      null,
    );

    const configs = Object.entries(systemConfigs).map(([name, options]) => (
      new Config(name, configFile, options)
    ));

    configFile.setConfigs(configs);

    return configFile;
  }

  return createSystemConfigs;
}

module.exports = createSystemConfigsFactory;
