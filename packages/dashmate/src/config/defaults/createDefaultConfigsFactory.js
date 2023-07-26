const ConfigFile = require('../configFile/ConfigFile');

const packageJson = require('../../../package.json');
const getShortHash = require('../../util/getShortHash');

/**
 * @param {DefaultConfigs} defaultConfigs
 * @param {HomeDir} homeDir
 * @return {createDefaultConfigs}
 */
function createDefaultConfigsFactory(defaultConfigs, homeDir) {
  /**
   * @typedef {createDefaultConfigs}
   * @returns {ConfigFile}
   */
  function createDefaultConfigs() {
    const projectId = getShortHash(homeDir.getPath());

    return new ConfigFile(
      defaultConfigs.getAll(),
      packageJson.version,
      projectId,
      null,
      null,
    );
  }

  return createDefaultConfigs;
}

module.exports = createDefaultConfigsFactory;
