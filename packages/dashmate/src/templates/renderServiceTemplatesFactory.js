const fs = require('fs');
const path = require('path');
const dots = require('dot');
const glob = require('glob');
const _ = require('lodash');

/**
 * @return {renderServiceTemplates}
 */
function renderServiceTemplatesFactory() {
  /**
   * Render templates for services
   *
   * @typedef {renderServiceTemplates}
   * @param {Config} config
   *
   * @param options
   * @return {Object<string,string>}
   */
  function renderServiceTemplates(config, options = { reindex: false }) {
    dots.templateSettings.strip = false;

    const templatesPath = path.join(__dirname, '..', '..', 'templates');

    const templatePaths = glob
      .sync(`${templatesPath}/**/*.dot`)
      // Do not render platform templates if it's not configured
      .filter((templatePath) => (
        !templatePath.includes('templates/platform') || config.get('platform.enable')
      ));

    const configFiles = {};
    for (const templatePath of templatePaths) {
      const templateString = fs.readFileSync(templatePath, 'utf-8');
      const template = dots.template(templateString);

      const configPath = templatePath
        .substring(templatesPath.length + 1)
        .replace('.dot', '');

      configFiles[configPath] = template(_.merge(config.options, options));
    }

    return configFiles;
  }

  return renderServiceTemplates;
}

module.exports = renderServiceTemplatesFactory;
