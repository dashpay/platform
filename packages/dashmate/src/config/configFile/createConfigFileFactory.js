const ConfigFile = require('./ConfigFile');

const packageJson = require('../../../package.json');
const getShortHash = require('../../util/getShortHash');

/**
 * @param {DefaultConfigs} defaultConfigs
 * @param {HomeDir} homeDir
 * @return {createConfigFile}
 */
function createConfigFileFactory(defaultConfigs, homeDir) {
  /**
   * @typedef {function} createConfigFile
   * @returns {ConfigFile}
   */
  function createConfigFile() {
    const projectId = getShortHash(homeDir.getPath());

    return new ConfigFile(
      defaultConfigs.getAll(),
      packageJson.version,
      projectId,
      null,
      null,
    );
  }

  return createConfigFile;
}

module.exports = createConfigFileFactory;
