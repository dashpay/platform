import fs from 'fs';
import path from 'path';

/**
 * @param {HomeDir} homeDir
 * @return {writeServiceConfigs}
 */
export default function writeServiceConfigsFactory(homeDir) {
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
    const configDir = homeDir.joinPath(configName);

    for (const filePath of Object.keys(configFiles)) {
      const absoluteFilePath = path.join(configDir, filePath);
      const absoluteFileDir = path.dirname(absoluteFilePath);

      // Ensure dir
      fs.mkdirSync(absoluteFileDir, { recursive: true });

      // Write specified config files
      fs.writeFileSync(absoluteFilePath, configFiles[filePath], 'utf8');
    }
  }

  return writeServiceConfigs;
}
