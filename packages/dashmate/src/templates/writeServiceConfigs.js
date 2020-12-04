const fs = require('fs');

/**
 * Write service config files
 * @param {Object} configFiles
 * @param {String} homedirPath
 * @param {String} configName
 * @returns {Promise<void>}
 */
async function writeServiceConfigs(configFiles, homedirPath, configName) {
  const configdirPath = `${homedirPath}/${configName}/`;
  try {
    fs.mkdirSync(configdirPath);
  } catch (e) {
    // do nothing
  }

  for (const configFile of configFiles) {
    const filePath = configdirPath + configFile.replace('.template', '');
    fs.writeFileSync(filePath, configFiles[configFile], 'utf8');
  }
}

module.exports = writeServiceConfigs;
