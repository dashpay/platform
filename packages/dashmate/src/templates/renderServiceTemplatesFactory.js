import dots from 'dot';
import * as glob from 'glob';
import { TEMPLATES_DIR } from '../constants.js';

/**
 * @return {renderServiceTemplates}
 */
export function renderServiceTemplatesFactory(renderTemplate) {
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

    const templatePaths = glob.sync(`${TEMPLATES_DIR}/**/*.dot`, {
      ignore: {
        // Ignore manual rendered templates
        ignored: (p) => p.name.startsWith('_'),
      },
    });

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
