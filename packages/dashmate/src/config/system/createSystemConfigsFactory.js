const ConfigFile = require('../configFile/ConfigFile');

const packageJson = require('../../../package.json');
const getShortHash = require('../../util/getShortHash');

/**
 * @param {SystemConfigs} systemConfigs
 * @param {HomeDir} homeDir
 * @return {createSystemConfigs}
 */
function createSystemConfigsFactory(systemConfigs, homeDir) {
  /**
   * @typedef {createSystemConfigs}
   * @returns {ConfigFile}
   */
  function createSystemConfigs() {
    const projectId = getShortHash(homeDir.getPath());

    return new ConfigFile(
      systemConfigs.getAll(),
      packageJson.version,
      projectId,
      null,
      null,
    );
  }

  return createSystemConfigs;
}

module.exports = createSystemConfigsFactory;
