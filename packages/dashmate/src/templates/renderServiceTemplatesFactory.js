const fs = require('fs');
const dots = require('dot');
const path = require('path');

/**
 * @return {renderServiceTemplates}
 */
function renderServiceTemplatesFactory(writeServiceConfigs) {
  /**
   * Render templates for services
   *
   * @typedef {renderServiceTemplates}
   * @param {Config} config
   * @param {string} homeDirPath
   *
   * @return {Promise<void>}
   */
  async function renderServiceTemplates(config, homeDirPath) {
    const files = fs.readdirSync(path.join(__dirname, 'templates'));

    dots.templateSettings.strip = false;
    const configFiles = {};
    for (const file of files) {
      const fileContents = fs.readFileSync(path.join(__dirname, 'templates', file), 'utf-8');
      const fileTemplate = dots.template(fileContents);
      if (
        file === 'genesis.json.template'
        && Object.keys(config.options.platform.drive.tendermint.genesis).length === 0
      ) {
        continue;
      }

      configFiles[file] = fileTemplate(config.options);
    }

    writeServiceConfigs(configFiles, homeDirPath, config.name);
  }

  return renderServiceTemplates;
}

module.exports = renderServiceTemplatesFactory;
