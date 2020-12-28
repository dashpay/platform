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
    // Drop all files from configs directory
    const configDir = path.join(homeDirPath, configName);
    fs.rmdirSync(configDir, { recursive: true });

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
