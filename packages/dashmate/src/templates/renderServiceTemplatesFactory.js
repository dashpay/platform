import * as glob from 'glob';
import { TEMPLATES_DIR } from '../constants.js';

/**
 * @param {renderTemplate} renderTemplate
 * @param {ensureTenderdashNodeKey} ensureTenderdashNodeKey
 * @return {renderServiceTemplates}
 */
export default function renderServiceTemplatesFactory(renderTemplate, ensureTenderdashNodeKey) {
  /**
   * Render templates for services
   *
   * @typedef {renderServiceTemplates}
   * @param {Config} config
   *
   * @return {Object<string,string>}
   */
  function renderServiceTemplates(config) {
    // node_key.json interpolates platform.drive.tenderdash.node.{id,key}
    // literally, so a null key must be filled in before rendering or
    // tenderdash panics at startup on the string "null".
    ensureTenderdashNodeKey(config);

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
