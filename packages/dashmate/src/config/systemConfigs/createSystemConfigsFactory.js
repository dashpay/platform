const Config = require('../Config');

const ConfigCollection = require('../ConfigCollection');

/**
 * @param {Object} systemConfigs
 * @return {createSystemConfigs}
 */
function createSystemConfigsFactory(systemConfigs) {
  /**
   * @typedef {createSystemConfigs}
   * @returns {ConfigCollection}
   */
  function createSystemConfigs() {
    const configs = Object.entries(systemConfigs).map(([name, options]) => (
      new Config(name, options)
    ));

    return new ConfigCollection(configs, 'base');
  }

  return createSystemConfigs;
}

module.exports = createSystemConfigsFactory;
