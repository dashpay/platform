const fs = require('fs');
const path = require('path');
const dots = require('dot');
const glob = require('glob');

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
   * @return {Object<string,string>}
   */
  function renderServiceTemplates(config) {
    dots.templateSettings.strip = false;

    const templatesPath = path.join(__dirname, '..', '..', 'templates');

    const templatePaths = glob
      .sync(`${templatesPath}/**/*.dot`)
      .filter((templatePath) => {
        // Do not render platform templates if it's not configured
        if (templatePath.includes('templates/platform') && !config.has('platform')) {
          return false;
        }

        // Don't create blank tenderdash configs
        if (templatePath.includes('templates/platform/drive/tenderdash')) {
          const skipEmpty = {
            genesis: 'genesis',
            node_key: 'nodeKey',
          };

          for (const [configName, optionName] of Object.entries(skipEmpty)) {
            const option = config.get(`platform.drive.tenderdash.${optionName}`);

            if (templatePath.includes(configName) && Object.values(option).length === 0) {
              return false;
            }
          }
        }

        return true;
      });

    const configFiles = {};
    for (const templatePath of templatePaths) {
      const templateString = fs.readFileSync(templatePath, 'utf-8');
      const template = dots.template(templateString);

      const configPath = templatePath
        .substring(templatesPath.length + 1)
        .replace('.dot', '');

      configFiles[configPath] = template(config.options);
    }

    return configFiles;
  }

  return renderServiceTemplates;
}

module.exports = renderServiceTemplatesFactory;
