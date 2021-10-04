const fs = require('fs');
const path = require('path');
const rimraf = require('rimraf');

const { HOME_DIR_PATH } = require('../constants');

/**
 * @return {writeServiceConfigs}
 */
function writeServiceConfigsFactory() {
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
    // Drop all files from configs directory
    const configDir = path.join(HOME_DIR_PATH, configName);
    rimraf.sync(configDir);

    for (const filePath of Object.keys(configFiles)) {
      const absoluteFilePath = path.join(configDir, filePath);
      const absoluteFileDir = path.dirname(absoluteFilePath);

      // Recreate it
      fs.mkdirSync(absoluteFileDir, { recursive: true });

      // Write specified config files
      fs.writeFileSync(absoluteFilePath, configFiles[filePath], 'utf8');
    }
  }

  return writeServiceConfigs;
}

module.exports = writeServiceConfigsFactory;
