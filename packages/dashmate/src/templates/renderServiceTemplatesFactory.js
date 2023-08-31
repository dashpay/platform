const dots = require('dot');
const glob = require('glob');
const { TEMPLATES_DIR } = require('../constants');

/**
 * @return {renderServiceTemplates}
 */
function renderServiceTemplatesFactory(renderTemplate) {
  /**
   * Render templates for services
   *
   * @typedef {renderServiceTemplates}
   * @param {Config} config
   *
   * @return {Object<string,string>}
   */
  function renderServiceTemplates(config) {
    dots.templateSettings.strip = false;

    const templatePaths = glob.sync(`${TEMPLATES_DIR}/**/*.dot`);

    const configFiles = {};
    for (const templatePath of templatePaths) {
      const configPath = templatePath
        .substring(TEMPLATES_DIR.length + 1)
        .replace('.dot', '');

      configFiles[configPath] = renderTemplate(templatePath, config.options);
    }

    return configFiles;
  }

  return renderServiceTemplates;
}

module.exports = renderServiceTemplatesFactory;
