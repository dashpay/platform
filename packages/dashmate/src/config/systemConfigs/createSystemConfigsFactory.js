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
    const configs = Object.entries(systemConfigs).map(([name, options]) => (
      new Config(name, options)
    ));

    return new ConfigFile(
      configs,
      packageJson.version,
      'base',
      null,
    );
  }

  return createSystemConfigs;
}

module.exports = createSystemConfigsFactory;
