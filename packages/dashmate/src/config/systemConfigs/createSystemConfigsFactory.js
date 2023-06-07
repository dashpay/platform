const Config = require('../Config');

const ConfigFile = require('../configFile/ConfigFile');

const packageJson = require('../../../package.json');
const getShortHash = require('../../util/getShortHash');
const { HOME_DIR_PATH } = require('../../constants');

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
    const projectId = getShortHash(HOME_DIR_PATH);

    const configFile = new ConfigFile(
      [],
      packageJson.version,
      projectId,
      null,
      null,
    );

    const configs = Object.entries(systemConfigs).map(([name, options]) => (
      new Config(name, options)
    ));

    configFile.setConfigs(configs);

    return configFile;
  }

  return createSystemConfigs;
}

module.exports = createSystemConfigsFactory;
