const fs = require('fs');
const path = require('path');

/**
 * @param {string} homeDirPath
 * @return {writeServiceConfigs}
 */
function writeServiceConfigsFactory(homeDirPath) {
  /**
   * Write service config files
   *
   * @typedef {writeServiceConfigs}
   * @param {string} configName
   * @param {Object} configFiles
   *
   * @return {void}
   */
  function writeServiceConfigs(configName, configFiles) {
    for (const configPath of Object.keys(configFiles)) {
      const absoluteConfigPath = path.join(homeDirPath, configName, configPath);

      const absoluteConfigDir = path.dirname(absoluteConfigPath);

      fs.mkdirSync(absoluteConfigDir, { recursive: true });

      fs.writeFileSync(absoluteConfigPath, configFiles[configPath], 'utf8');
    }
  }

  return writeServiceConfigs;
}

module.exports = writeServiceConfigsFactory;
