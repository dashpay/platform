import * as glob from 'glob';
import { TEMPLATES_DIR } from '../constants.js';
import ensureTenderdashNodeKey from '../tenderdash/ensureTenderdashNodeKey.js';

/**
 * @param {renderTemplate} renderTemplate
 * @return {renderServiceTemplates}
 */
export default function renderServiceTemplatesFactory(renderTemplate) {
  /**
   * Render templates for services
   *
   * @typedef {renderServiceTemplates}
   * @param {Config} config
   * @param {Object} [scope] - which service files to render, all by default
   * @param {string[]} [scope.skipDirectories] - top-level directories of the
   *  templates whose files are not rendered
   *
   * @return {Object<string,string>}
   */
  function renderServiceTemplates(config, { skipDirectories = [] } = {}) {
    // Prevent node_key.json from containing the literal string "null".
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

      if (skipDirectories.includes(configPath.split('/')[0])) {
        continue;
      }

      configFiles[configPath] = renderTemplate(templatePath, config.options);
    }

    return configFiles;
  }

  return renderServiceTemplates;
}
