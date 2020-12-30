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

    // Don't create blank tenderdash config objects for tenderdash init
    const skipEmpty = {
      genesis: 'genesis',
      node_key: 'nodeKey',
      priv_validator_key: 'validatorKey',
    };

    const emptyConfigsMask = Object.keys(skipEmpty).filter((key) => {
      const option = config.get(`platform.drive.tenderdash.${skipEmpty[key]}`);

      return Object.values(option).length === 0;
    }).join('|');

    const templatePaths = glob.sync(`${templatesPath}/**/!(${emptyConfigsMask}).*.template`);

    const configFiles = {};
    for (const templatePath of templatePaths) {
      const templateString = fs.readFileSync(templatePath, 'utf-8');
      const template = dots.template(templateString);

      const configPath = templatePath
        .substring(templatesPath.length + 1)
        .replace('.template', '');

      configFiles[configPath] = template(config.options);
    }

    return configFiles;
  }

  return renderServiceTemplates;
}

module.exports = renderServiceTemplatesFactory;
